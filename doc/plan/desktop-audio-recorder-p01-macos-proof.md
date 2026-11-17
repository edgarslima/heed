# Desktop Audio Recorder P01 macOS Proof Plan

**Purpose:** Sequence the macOS capture proof and its evidence. **Audience:** Engineers and the project owner. **Status:** In progress. Last reviewed: 2026-11-17.

Related documents: [Construction plan](desktop-audio-recorder-construction-plan.md) · [Work breakdown](desktop-audio-recorder-work-breakdown.md) · [Architecture](../specs/software-architecture-desktop-audio-recorder.md) · [Coding practices](../standard/coding-practices-desktop-audio-recorder.md) · [Status](../status.md)

## Sequence and gates

1. **P01.1 — Bridge skeleton.** Define the C header, opaque handle and explicit error codes. Compile Swift from Cargo on macOS; exercise the ABI from Rust and the lifecycle with XCTest. Run the production Swift line-coverage gate from `xccov` on macOS. The skeleton must return an error for discovery or capture until a real source is implemented.
2. **P01.2 — Microphone.** Implement HAL/AUHAL capture and a probe that writes a 60-second WAV. Verify the observed format and duration with `ffprobe`, recording the host, permissions, and timestamps.
3. **P01.3 — System audio.** Implement the global process tap and aggregate device. Capture 60 seconds while confirming playback remains audible; verify reverse-order cleanup leaves no aggregate device behind.
4. **P01.4 — Concurrent capture.** Capture microphone and system audio simultaneously and record the start offset and host-time observations for P03b.
5. **P01.5 — Application scope.** Map process objects and browser helper processes; measure isolation with two distinct tones in Safari and Chrome, recording OS and browser versions.
6. **P01.6 — Permissions.** Test grant, denial, and revocation in a signed development build. Record usage descriptions, entitlements, and observed behavior.

The owner attends permission and playback checks. Record commands, environment, results, and evidence paths in `doc/status.md`; do not claim native capture from synthetic tests. Review P03a and integrate the relevant branch before considering the combined work ready for merge. Before adding Rust platform-only adapter files, extend the coverage inventory to group those files by OS as noted in the P03a checkpoint.

## P01.2 step plan — Microphone

Executed on 2026-11-14; results and deviations are in the P01.2 entry of `doc/status.md`.

**Goal:** Capture the default input device through AUHAL in the Swift bridge, deliver borrowed `f32` blocks across the C ABI into the P03a block queue, and write a 60-second WAV from a probe on the Mac.

**Decisions:**

- **ABI.** `recorder_prepare(handle, source_kind, format_out)` opens the source and reports the negotiated format; `recorder_start(handle, callback, context)` registers a C callback that receives a `recorder_block` (borrowed interleaved `f32` samples, frame count, host time in nanoseconds, sample time, validity flags) on the audio thread. The pointer is valid only during the call. `recorder_stop` returns only after the device stopped calling back, so the context can be freed afterwards. `recorder_last_os_status` exposes the last Core Audio `OSStatus` for diagnostics. New codes: `RECORDER_PERMISSION_DENIED` and `RECORDER_NATIVE_ERROR`. `recorder_microphone_access` reports or requests TCC access from a control thread.
- **Format.** The client format keeps the device's sample rate and channel count and asks AUHAL for packed interleaved `Float32`. AUHAL does not resample; P03b owns rate changes.
- **Seam.** `MicrophoneCapture` owns state, buffers, and callback forwarding and calls Core Audio only through the `InputUnitDriver` protocol. XCTest drives it with a fake driver for lifecycle, invalid states, render errors, and teardown. The real `AUHALDriver` is a thin pass-through tested on the Mac: opening the default device needs no TCC grant; capture tests that start the device run only when `HEED_HARDWARE_TESTS=1`.
- **Probe.** `crates/recorder-core/examples/macos-microphone-probe.rs` (renamed `macos-capture-probe.rs` in P01.3) is a validation tool, not shipped code, so it sits outside the `crates/**/src/**` coverage scope. Its callback pushes into `BlockProducer`; a consumer thread writes an IEEE-float WAV and the probe prints a JSON summary (format, frames, blocks, drops, first host time, sample-time gaps, peak dBFS). No production Rust file is added, so the per-OS Rust inventory stays a prerequisite of the first Rust adapter file.
- **Build.** `build.rs` compiles every Swift file under `Sources/RecorderBridge` and reruns when any of them or the header changes.

**Tasks:**

1. Extend the header and the Swift entry points with failing XCTest cases for the new arguments, states, and codes; then implement `MicrophoneCapture` against a fake driver.
2. Implement `AUHALDriver`, with a Mac test that opens and closes the default input device and an opt-in hardware test that captures one second.
3. Update `build.rs` and the Rust ABI smoke test; add the probe example.
4. With the owner present: grant microphone access, record 60 seconds of speech, and run `ffprobe -v error -print_format json -show_format -show_streams`. Accept when the duration is 60 s ± one block, the format matches the reported one, there are no drops, and the peak is well above silence. Store the probe summary, `ffprobe` JSON, host, permission state, and the WAV SHA-256 in `evidence/p01/microphone.md`; the WAV stays git-ignored.
5. Run the full local gate and CI; record the results in `doc/status.md`.

## P01.3 and P01.4 step plan — System audio and concurrent capture

Executed on 2026-11-16 and 2026-11-17; results and deviations are in the P01.3 and P01.4 entries of `doc/status.md`.

**Goal:** Capture everything the Mac plays through a global process tap and a private aggregate device, deliver it through the same block ABI as the microphone, and then run both sources at once to record their start offset and clocks for P03b.

**Decisions:**

- **ABI.** `RECORDER_SOURCE_SYSTEM = 2` reuses `recorder_prepare`, `recorder_start`, `recorder_stop`, and the block callback. One handle captures one source; concurrent capture uses two handles. A handle switches source only when idle.
- **Tap and aggregate.** `SystemCapture` creates a private `CATapDescription(stereoGlobalTapButExcludeProcesses: [])` with `muteBehavior = .unmuted`, then a private aggregate device whose main sub-device (the clock) is the default output device, with the tap in its tap list, drift compensation on, and tap auto-start. The recorder plays no audio, so it excludes no process. Prepare reports the aggregate's nominal rate, because the IOProc runs on that clock. Teardown runs in reverse order: stop the device, destroy the IOProc, the aggregate, then the tap.
- **Buffers.** The IOProc passes an interleaved input buffer through borrowed; planar input is interleaved into a buffer preallocated for `kAudioDevicePropertyBufferFrameSize` frames. An unexpected buffer shape or an oversized cycle marks the next block discontinuous, as for the microphone.
- **Permission.** No public API reports the System Audio Recording grant; without it the tap delivers exact zeros. The bridge does not check microphone access for this source. The probe reports an all-zero capture so the bench can tell a missing grant from a code fault. Grant, denial, and revocation stay in P01.6.
- **Seam.** Core Audio tap, aggregate, and IOProc calls go through `SystemAudioCalls`; XCTest drives a fake. The live calls run only in an opt-in hardware test (`HEED_HARDWARE_TESTS=1`), which also checks that the tap and aggregate UIDs no longer resolve after `stop`.
- **Probe.** `macos-microphone-probe` becomes `macos-capture-probe --source microphone|system|both`. With `both`, it prepares both handles, starts the microphone and then the system source, writes one WAV per source, and reports the start offset between their first host times and each source's host-measured rate.

**Tasks:**

1. Add the system source to the header and the handle; implement `SystemCapture` against a fake, with failing tests first for prepare, unwinding, delivery shapes, and teardown order.
2. Implement the live calls and the opt-in hardware test; regenerate the Xcode project with `xcodegen`.
3. Rename and extend the probe.
4. With the owner present: play a 1 kHz tone through the default output while capturing 60 s of system audio; record whether playback stayed audible, the probe summary, `ffprobe` JSON, and SHA-256 in `evidence/p01/system-audio.md`.
5. Capture both sources for 60 s with the same tone; record the start offset and host-clock observations in `evidence/p01/concurrent.md`.
6. Run the full local gate and CI; record the results in `doc/status.md`.
