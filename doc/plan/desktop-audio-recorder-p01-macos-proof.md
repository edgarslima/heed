# Desktop Audio Recorder P01 macOS Proof Plan

**Purpose:** Sequence the macOS capture proof and its evidence. **Audience:** Engineers and the project owner. **Status:** In progress. Last reviewed: 2026-11-14.

Related documents: [Construction plan](desktop-audio-recorder-construction-plan.md) · [Work breakdown](desktop-audio-recorder-work-breakdown.md) · [Architecture](../specs/software-architecture-desktop-audio-recorder.md) · [Coding practices](../standard/coding-practices-desktop-audio-recorder.md) · [Status](../status.md)

## Sequence and gates

1. **P01.1 — Bridge skeleton.** Define the C header, opaque handle and explicit error codes. Compile Swift from Cargo on macOS; exercise the ABI from Rust and the lifecycle with XCTest. Run the production Swift line-coverage gate from `xccov` on macOS. The skeleton must return an error for discovery or capture until a real source is implemented.
2. **P01.2 — Microphone.** Implement HAL/AUHAL capture and a probe that writes a 60-second WAV. Verify the observed format and duration with `ffprobe`, recording the host, permissions, and timestamps.
3. **P01.3 — System audio.** Implement the global process tap and aggregate device. Capture 60 seconds while confirming playback remains audible; verify reverse-order cleanup leaves no aggregate device behind.
4. **P01.4 — Concurrent capture.** Capture microphone and system audio simultaneously and record the start offset and host-time observations for P03b.
5. **P01.5 — Application scope.** Map process objects and browser helper processes; measure isolation with two distinct tones in Safari and Chrome, recording OS and browser versions.
6. **P01.6 — Permissions.** Test grant, denial, and revocation in a signed development build. Record usage descriptions, entitlements, and observed behavior.

The owner attends permission and playback checks. Record commands, environment, results, and evidence paths in `doc/status.md`; do not claim native capture from synthetic tests. Review P03a and integrate the relevant branch before considering the combined work ready for merge. Before adding Rust platform-only adapter files, extend the coverage inventory to group those files by OS as noted in the P03a checkpoint.
