# Project Status — Desktop Audio Recorder

**Purpose:** Execution checkpoint with the current task, last verified state, and next step. **Audience:** Engineers and AI agents (Codex, Claude Code, Cursor) starting or resuming work. **Status:** Active. Last reviewed: 2026-11-17.

Related documents: [Construction plan](plan/desktop-audio-recorder-construction-plan.md) · [Coding practices](standard/coding-practices-desktop-audio-recorder.md) · [Agent directives](okf/agent-directives.md)

Each entry follows the checkpoint template in section 4 of the construction plan. Record facts and evidence only; do not paste chat history or user audio data.

## Next task

P03a and P01.1–P01.4 are ready for review on `p03a-common-engine`. Next: P01.5 (application scope with Safari and Chrome tones) with the owner present, then P01.6 (permissions in a signed development build; an Apple Development identity is on the bench). P03b (timeline and resampling) can start from the P01.2–P01.4 observations.

## Repository

| Item | Value |
| --- | --- |
| Remote | `origin` → https://github.com/edgarslima/heed (public) |
| Default branch | `main` — stable; initial commit `c3d3bef` |
| Integration branch | `dev` — created from `main` |
| Working branches | `initial_dev` — P00 bootstrap (merged into `dev` at `aa47d72`); `p03a-common-engine` — P03a |
| CI host | GitHub Actions, `.github/workflows/ci.yml`; jobs on `macos-15`, `windows-2025`, `ubuntu-24.04` |
| History rewrite | On 2026-11-14 every commit message was rewritten to drop the assistant co-author line (owner request); trees, authors, and dates are unchanged. Commit hashes in this file use the rewritten history; linked CI runs ran on the pre-rewrite commits with identical trees. |

## Host environment

Observed on 2026-11-13 on the macOS bench.

| Component | Observed version |
| --- | --- |
| macOS | 27.0.1, arm64 |
| Xcode / Swift | Xcode 27.2 / Swift 6.4.2 |
| Rust | rustc 1.100.0, cargo 1.100.0, targets `aarch64-apple-darwin` and `x86_64-apple-darwin` |
| Tauri CLI | `@tauri-apps/cli` 2.12.4 pinned in `package.json` (a cargo `tauri-cli` 2.12.4 is also installed but not used) |
| Coverage | `cargo-llvm-cov` 0.9.1; `llvm-tools-preview` installed through `rust-toolchain.toml` |
| Node / npm | Node 24.21.0 / npm 11.19.0 |
| ffprobe | 8.1.2 |
| Windows 11 x64 bench | Not recorded |
| Ubuntu 24.04 LTS bench | Not recorded |

Environment gaps:

- `~/.cargo/bin` is not on the default shell `PATH`; run `source "$HOME/.cargo/env"` or add it to the shell profile.

## Open decisions

The full list, with what each blocks and its default, is in [work breakdown section 7](plan/desktop-audio-recorder-work-breakdown.md#7-decisions-before-construction).

1. **Windows 11 x64 and Ubuntu 24.04 benches.** Not available yet; P02, P03.7, P08, P09, P10, and P11 stay blocked until they exist.
2. **Signing credentials.** Needed at P09.2 only.

## Resolved decisions

Resolved by the project owner on 2026-11-13.

1. **Interface language.** The interface ships in American English only, with strings in a single catalog keyed by stable English identifiers. Updated the coding practices.
2. **Private architecture reference.** Removed the private `chatgpt.com` link from the software architecture.
3. **Assistant file priority.** Canonical documents in `doc/` always win over tool entry points. `doc/README.md` now lists the entry points per tool and the conflict order; `doc/okf/structure.md` and `doc/okf/agent-directives.md` match it.
4. **File naming.** All file names, including scripts, use lowercase words separated by hyphens, except names fixed by a tool. Renamed the setup script to `doc/environment/setup-environment.sh`.
5. **Product name and identifier.** Product `Heed`, app identifier `pro.edgarlima.heed`, window title "Heed". Local desktop project; no DNS reservation. Updated the design system header.
6. **Bench strategy.** Mac only for now; follow the Mac-only path in work breakdown section 5.
7. **P03 split.** Approved: P03a and P03b build the engine on the Mac; P03.7 re-validates it against Windows observations before P08. Updated the construction plan dependency table.
8. **Speech corpus.** WER is measured on both PT-BR and American English corpora, scored separately. Updated the build specification (AC-08) and the plan's speech test.
9. **Git remote and CI host.** Public GitHub repository `edgarslima/heed`; CI runs on GitHub Actions. Branches: `main` (stable), `dev` (integration), `initial_dev` (P00 work).
10. **Coverage exclusion of `src-tauri/src/main.rs`.** Approved by the project owner on 2026-11-13: the window entry point only calls the tested `configure` and `exit_code`.

## Preflight — Repository alignment

- Status — validated.
- Branch and base commit — `main` at `c3d3bef` (initial commit).
- Requirements covered — construction plan section 0 (repository-alignment preflight).
- Changes — created `doc/status.md`; added OKF metadata (purpose, audience, status, review date) to every document; fixed the broken build-specification link in the construction plan; removed duplicated gain sentences in architecture section 4; reduced `AGENTS.md`, `CLAUDE.md`, `.cursorrules`, and `.cursor/rules/okf.mdc` to pointers; `CLAUDE.md` imports `AGENTS.md` and the agent directives; extended the setup script with coverage tooling, `ffprobe`, and the keg-only Node `PATH` fix.
- Evidence — relative-link check over `doc/**/*.md` reports no broken links; `bash -n doc/environment/setup-environment.sh` passes.
- Pending — running the updated setup script (P00 Task 4 installs the missing coverage tooling).
- Next step — P00 on `initial_dev`.

## Planning — Work breakdown

- Status — validated by the owner (decisions recorded above).
- Branch and base commit — `main` at `c3d3bef` (initial commit).
- Requirements covered — construction plan P00–P11; RF-01 to RF-15 and AC-01 to AC-10 traced in the work breakdown.
- Changes — added `doc/plan/desktop-audio-recorder-work-breakdown.md` and `doc/plan/desktop-audio-recorder-p00-bootstrap.md`; the construction plan section 3 now points to coding practices section 6 for the gate commands.
- Evidence — link and metadata checks pass; every RF and AC ID appears in the traceability table.
- Pending — Windows and Ubuntu benches.
- Next step — execute P00.

## P00 — Bootstrap

- Status — ready for review.
- Branch and base commit — `initial_dev`, base `38937ca` (from `dev`).
- Requirements covered — construction plan P00; work breakdown P00.1–P00.7.
- Changes:
  - Toolchain pins (`rust-toolchain.toml` 1.100.0, `.nvmrc` 24, `.npmrc` exact versions), Cargo workspace with `crates/recorder-core` (no code yet) and `src-tauri`, workspace lints (`unsafe_op_in_unsafe_fn` denied; `unwrap`, `expect`, `panic` rejected by clippy outside tests).
  - Tauri 2 skeleton without demo code: product `Heed`, identifier `pro.edgarlima.heed`, window 560 × 600, minimum 420 × 480, CSP `default-src 'self'`, capability `core:default` only.
  - Frontend tooling: strict `tsconfig`, ESLint `strictTypeChecked` with `no-explicit-any` and `no-floating-promises`, Prettier, Vitest with jsdom and V8 coverage; `mountApp` app shell.
  - `src-tauri/src/lib.rs` exposes `configure` and `exit_code`, tested on the Tauri mock runtime; `main.rs` is a one-line window entry point.
  - Coverage gate `scripts/coverage-gate.mjs` with scopes in `scripts/coverage-scopes.json`; integer rule `covered * 100 > total * 80`. It also validates the scope schema, requires literal paths and reasons for justified exclusions, prints every exclusion with its measured counts, fails on coverage pragmas in production sources, and fails a pending scope whose report already measures code.
  - CI workflow per OS; `.gitattributes` forces LF; Windows test binaries embed the app manifest (`src-tauri/build.rs`).
- Evidence:
  - CI run [43501415439](https://github.com/edgarslima/heed/actions/runs/43501415439) on `f8225c5`: success on macOS 15, Windows 2025, Ubuntu 24.04. Gate: frontend lines 6/6 and branches 2/2, rust-tauri lines 9/9 (`main.rs` excluded with 0/6 shown), rust-shared and swift-bridge pending. Gate tests: 21; total frontend and script tests: 24.
  - Local bench: the same command sequence passes; `tauri dev` window "Heed" 560 × 600, clamps to 420 × 480.
  - Packaged app: [evidence/p00/packaging.md](../evidence/p00/packaging.md).
- Pending:
  - Packaged-window check on Windows and Linux (no benches yet).
  - Merge `initial_dev` into `dev` after review.
- Deviations from the P00 plan (each decided and verified during execution):
  - Task 2: dropped template dependencies not used in P00 (`@tauri-apps/api`, `@tauri-apps/plugin-opener`, `serde`, `serde_json`), demo assets, `README.md`, and `.vscode/`; moved `[profile.release]` to the workspace root; `withGlobalTauri` is `false`.
  - Task 3: `mountApp` was created in Task 3 after its failing test instead of in Task 2; Prettier ignores `doc/`.
  - Task 4: instead of `run() -> Result` in `lib.rs`, the app composition is `configure<R: Runtime>(Builder<R>) -> Builder<R>` and the exit mapping is `exit_code(Result<(), tauri::Error>) -> ExitCode`, both tested; `main.rs` only calls them. The mobile entry point attribute was removed (desktop only).
  - Task 5: test reports are built from literals in temporary directories instead of committed fixture files; the scope schema gained `exclude` (rule-based exclusions such as tests) and `justifiedExclusions` (reviewed exceptions with a reason). The gate was hardened after the final review (see Changes).
  - Task 6: the CI build order rationale was corrected (only release builds embed `frontendDist`); CI creates `coverage/` before `cargo llvm-cov`, pins `cargo-llvm-cov` 0.9.1, and Windows test binaries embed the app manifest. `.gitattributes` forces LF.
  - Task 7: the "networking disabled" check was replaced by `lsof` showing no sockets for the app process; the full offline check stays in P07.5 and P09.3.
- Owner actions:
  - Enable branch protection on `main` and `dev` with the `verify` CI checks required, so coding practices section 6 rule 5 is enforced at merge.
- Next step — review P00, merge into `dev`, then write the P01 or P03a step plan.

## P03a — Common engine

- Status — ready for review.
- Branch and base commit — `p03a-common-engine`, base `aa47d72` (`dev`).
- Requirements covered — work breakdown P03.1–P03.4; architecture section 2 (capture contract, queue) and section 4 (conversion, metering); RF-06 and AC-06 groundwork.
- Changes:
  - `recorder-core`: audio formats, block header and flags, capture events and errors, the `CaptureSource` trait, an allocation-free `rtrb` block queue, PCM conversion to finite `f32`, 100 ms metering with a latest-value snapshot slot, and a deterministic synthetic source with scripted events and clock drift.
  - Coverage gate: `declarativeFiles` for files llvm-cov omits (module declarations, a trait without bodies), each with a reason and a check that they stay unmeasured; `rust-shared` scope active.
  - Architecture section 2 records the implemented contract.
- Evidence:
  - CI run [43508152103](https://github.com/edgarslima/heed/actions/runs/43508152103) on `3d084fc` (after the review fixes): success on macOS 15, Windows 2025, Ubuntu 24.04; gate `rust-shared` lines 447/456.
  - 61 Rust tests and 28 frontend and script tests pass locally. The allocation test reports 0 allocations over 20,000 queue operations; a mutation check (an injected allocation) made it fail with 20,000.
- Deviations from the P03a plan:
  - Integration test files allow `clippy::panic`, `unwrap_used`, and `expect_used` with a stated reason; clippy's test exemptions do not cover helpers in `tests/`.
  - `rtrb` was added in Task 6 instead of Task 5; `capture/source.rs` is a declarative file.
  - `SnapshotSlot::publish` skips a snapshot under lock contention instead of holding it pending; the next window follows within 100 ms.
  - Extra tests beyond the plan cover format getters, all sample encoders, planar rendering, and multi-window calls.
  - Final review fixes: `QueueSizing::for_format` takes minimum and maximum block sizes so small blocks still get about 1 s of headroom; blocks carry their queue identity and `recycle` rejects foreign blocks; the synthetic source no longer sends `Discontinuity` events from its callback path and `CaptureSource::start` documents the event channel as non-real-time; the meter envelope starts from the window's own samples.
- Pending:
  - Deferred minor findings: `amplitude_to_dbfs` returns infinity for infinite input (inputs are sanitized by `convert_to_f32`); the I32 conversion doc says `[-1.0, 1.0)` but `i32::MAX` rounds to 1.0; a skipped snapshot under lock contention can lose a clipping indication; an unrecycled block shrinks its queue without a counter; no compile-time `Send` assertion for `BlockProducer`; declarative files are not checked for function bodies or scope membership; the `tests.rs` exclusion is not checked against `#[cfg(test)]`.
  - Before P01 adapters: the coverage gate needs a per-OS scope or inventory, because platform-only adapter files are absent from other OSes' reports.
  - P03b needs P01 observations; P03.7 needs the Windows bench.
- Next step — review P03a and choose how to integrate it into `dev`; then P01 with the owner present.

## P01.1 — macOS bridge skeleton

- Status — ready for review on `p03a-common-engine` at `5f3a08b`; not merged.
- Requirements covered — P01.1 and architecture section 3 macOS C ABI groundwork. No capture or device discovery is implemented.
- Changes — added a Swift bridge, opaque C handle, explicit result codes, Cargo compilation on macOS, Rust ABI smoke test, Swift XCTest target, and a P01 plan. The coverage gate reads `xccov` reports and requires the Swift scope on macOS; CI has a macOS XCTest and report step. Discovery and preparation return source unavailable, and start returns invalid state. Review on 2026-11-14: the Swift package now declares macOS 14.2 like `project.yml` and `build.rs`; `.gitignore` and `.prettierignore` skip `native/macos/.build/` (Swift package output that broke `npm run lint`) and `.xcresult` bundles.
- Evidence — macOS arm64 with Xcode 27.2: Swift package XCTest passed 2 tests; Xcode XCTest passed 2 tests outside the sandbox, and `xccov` reports 27/27 production lines. The full local gate (`npm run build`, `cargo fmt --all -- --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo llvm-cov --locked --workspace --json --output-path coverage/rust.json`, `npm run lint`, `npm run typecheck`, `npm run test:coverage`, and `npm run coverage:gate`) passed: Swift 27/27, shared Rust 447/456, Tauri 9/9, frontend 6/6 lines and 2/2 branches; 62 Rust tests passed. After adding a platform-skip regression, lint, coverage tests (32 frontend/script tests), and the coverage gate passed again. A C11 header syntax check passed. Re-run on 2026-11-14 after the review fixes, with `swift test` output present: the same sequence plus `xcodebuild … test` and `xccov` passed — Swift 27/27, shared Rust 447/456, Tauri 9/9, frontend 6/6 lines and 2/2 branches, 32 frontend and script tests.
- CI — run [43619725671](https://github.com/edgarslima/heed/actions/runs/43619725671) on `5f3a08b`: success on macOS 15, Windows 2025, Ubuntu 24.04. macOS gate: Swift 27/27, shared Rust 447/456, Tauri 9/9, frontend 6/6 and 2/2; Windows and Ubuntu skip the Swift scope as not applicable.
- Pending — review and merge with P03a.
- Next step — P01.2.

## P01.2 — Microphone capture

- Status — ready for review on `p03a-common-engine` at `d1ef9de`; not merged.
- Requirements covered — work breakdown P01.2; architecture section 3 macOS items 1–3 for the microphone (AUHAL, device callback, host time, reverse-order teardown).
- Changes:
  - C ABI: `recorder_microphone_access`, `recorder_prepare(handle, source_kind, format)`, `recorder_start(handle, callback, context)`, `recorder_last_os_status`, the `recorder_format` and `recorder_block` structs with static size checks, block flags, and the codes `RECORDER_PERMISSION_DENIED` and `RECORDER_NATIVE_ERROR`. `recorder_list` counts the default input device; `recorder_destroy` stops a running handle.
  - Swift: `MicrophoneCapture` opens the default input through AUHAL with packed interleaved `Float32` at the device rate, preallocates the render buffer, marks the next block discontinuous after an oversized or failed render, and tears down with stop, uninitialize, and dispose before `stop` returns. Prepare requires granted access because macOS records silence without it. Core Audio calls go through the `CoreAudioCalls` seam; XCTest drives a fake for lifecycle, errors, and teardown.
  - Rust: `build.rs` compiles every Swift source; the ABI test checks the struct layouts and argument validation; `examples/macos-microphone-probe.rs` pushes callback blocks into the P03a queue and writes an IEEE-float WAV from a writer thread.
- Evidence:
  - [evidence/p01/microphone.md](../evidence/p01/microphone.md): a 60.000 s WAV, `pcm_f32le`, 48 kHz mono, 2,880,000 frames, 512-frame blocks, no drops or sample-time gaps, host clock 47,999.988 Hz; `ffprobe` JSON and SHA-256 recorded.
  - The opt-in hardware XCTest (`HEED_HARDWARE_TESTS=1 swift test --filter CoreAudioLiveTests`) passed three 1-second capture cycles, and no callback ran after `stop` returned.
  - Full local gate on macOS arm64 (the P01.1 sequence plus `xcodebuild … test` and `xccov`): Swift 313/321, shared Rust 447/456, Tauri 9/9, frontend 6/6 lines and 2/2 branches; 63 Rust tests and 32 frontend and script tests passed. The Swift package ran 19 tests; the hardware test is skipped unless enabled.
- Deviations from the P01.2 step plan:
  - Microphone access was already granted to the terminal, so no prompt fired. Grant, denial, and revocation stay in P01.6.
  - The recording shows room sound (peak -25.31 dBFS), not clear speech; the acceptance check compared it with a room check at peak -28.27 dBFS.
- Skipped scenarios — the hardware XCTest is skipped on CI because runners have no granted microphone; the default run never starts input, so it cannot raise a prompt. Run the hardware test with `swift test`, which inherits the terminal's microphone grant: with `TEST_RUNNER_HEED_HARDWARE_TESTS=1`, the variable reaches the runner that `xcodebuild` launches, but that runner has its own TCC identity without access, so the test skips.
- CI — run [43621168784](https://github.com/edgarslima/heed/actions/runs/43621168784) on `0cb7463` (covers the `d1ef9de` code): success on macOS 15, Windows 2025, Ubuntu 24.04. macOS gate: Swift 313/321, shared Rust 447/456, Tauri 9/9, frontend 6/6 and 2/2; Windows and Ubuntu skip the Swift scope and build the probe's non-macOS stub. The default live XCTest ran on the runner without hanging on a prompt.
- Pending — review and merge with P03a and P01.1.
- Next step — P01.3 with the owner present.

## P01.3 — System audio

- Status — ready for review on `p03a-common-engine` at `61370a7`; not merged.
- Requirements covered — work breakdown P01.3; architecture section 3 macOS items 2–3 for **System** (global tap, aggregate device, IOProc, reverse-order teardown).
- Changes:
  - C ABI: `RECORDER_SOURCE_SYSTEM = 2` on the existing `recorder_prepare`, `recorder_start`, `recorder_stop`, and block callback. A handle switches source only when idle.
  - Swift: `SystemCapture` creates a private, unmuted `CATapDescription(stereoGlobalTapButExcludeProcesses: [])`, a private aggregate device whose main sub-device is the default output device, with the tap in its tap list, drift compensation, and tap auto-start, and an IOProc. It reports the aggregate's nominal rate and `kAudioDevicePropertyBufferFrameSize`. Interleaved input passes through borrowed; planar input is interleaved into a preallocated buffer; an unexpected buffer shape or an oversized cycle marks the next block discontinuous. Teardown stops the device and destroys the IOProc, aggregate, and tap in that order, also when prepare fails partway. Calls go through `SystemAudioCalls`; XCTest drives `FakeSystemAudio`. Microphone and system blocks share one `RecorderBlock` initializer, and both captures conform to `CaptureSession`.
  - The opt-in hardware test plays its own 440 Hz tone, captures three 1 s cycles, and checks that no callback runs after stop, that the capture is not all zeros, and that the tap and aggregate UIDs stop resolving after stop and after a failed prepare.
  - The probe is renamed `macos-capture-probe` and takes `--source microphone|system|both`. The Xcode project was regenerated with `xcodegen`.
- Evidence:
  - [evidence/p01/system-audio.md](../evidence/p01/system-audio.md): a 59.979 s WAV, `pcm_f32le`, 48 kHz stereo, 512-frame blocks, no drops, gaps, or discontinuities, host clock 47,999.996 Hz, a 1 kHz tone at peak -38.90 dBFS. The owner confirmed playback stayed audible during the capture.
  - `HEED_HARDWARE_TESTS=1 swift test --filter CoreAudioLiveTests` passed the microphone and system hardware tests on 2026-11-16.
  - Full local gate on macOS arm64 on 2026-11-17 (`npm run build`, `cargo fmt --all -- --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`, `cargo llvm-cov --locked --workspace --json --output-path coverage/rust.json`, `npm run lint`, `npm run typecheck`, `npm run test:coverage`, `swift test`, `xcodebuild … test`, `xccov`, `npm run coverage:gate`): Swift 529/569, shared Rust 447/456, Tauri 9/9, frontend 6/6 lines and 2/2 branches; 63 Rust tests and 32 frontend and script tests passed; the Swift package ran 30 tests with the 2 hardware tests skipped.
- Deviations from the P01.3 step plan:
  - The IOProc does not run while nothing plays, so the hardware test plays its own tone instead of relying on ambient audio.
  - The HAL drops a destroyed aggregate from UID translation shortly after the destroy call returns (under 100 ms observed); the hardware test polls for up to 1 s.
  - The 60 s capture is 21 ms (two blocks) short because the first cycle arrives after `recorder_start` returns.
- Skipped scenarios — the hardware tests are skipped on CI: runners have no System Audio Recording grant, and the default run never creates a tap. Grant, denial, and revocation stay in P01.6. Route changes of the default output device during capture are not handled yet.
- CI — pending.
- Next step — P01.4.

## P01.4 — Concurrent capture

- Status — ready for review on `p03a-common-engine` at `61370a7`; not merged.
- Requirements covered — work breakdown P01.4; architecture section 4 (start and alignment) groundwork for P03b.
- Changes — `macos-capture-probe --source both` prepares one handle per source, starts the microphone and then the system source, writes `<stem>-microphone.wav` and `<stem>-system.wav`, and reports `startOffsetNs` (system first-frame host time minus the microphone's) and `relativeDriftPpm`.
- Evidence — [evidence/p01/concurrent.md](../evidence/p01/concurrent.md): 60 s of both sources with the 1 kHz tone playing; microphone 2,880,000 frames, system 2,880,512 frames; no drops, gaps, or discontinuities in either; start offset 18.68 ms; both clocks 48,000.004 Hz against the host clock, relative drift 0.000 ppm.
- Observations for P03b — align on first-frame host time; the system source delivers nothing until playback begins, so its leading interval must be filled with silence on the timeline, not reported as a loss; the built-in input and output share a clock on this bench, so drift estimation still needs a USB or Bluetooth test.
- CI — pending.
- Next step — P01.5 with the owner present.
