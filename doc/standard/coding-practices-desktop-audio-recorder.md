# Coding Practices — Desktop Audio Recorder

**Purpose:** Implementation, testing, coverage, and review criteria for all code. **Audience:** Engineers and AI agents writing or reviewing code, scripts, and tests. **Status:** Active. Last reviewed: 2026-11-13.

**Contribution rule.** All code must be written in **American English**, including comments, tests, and scripts. Production coverage must be **strictly above 80%**, with automatic verification by language and platform. A result of 80.00% fails.

Applicable to the desktop app **Tauri 2 + TypeScript + Rust**, including the macOS Swift bridge and the audio adapters for Windows, macOS, and Linux. This document defines implementation and review criteria. The commands below are intended for the application repository.

[Build specification](../specs/build-specification-desktop-audio-recorder.md) · [Software architecture](../specs/software-architecture-desktop-audio-recorder.md) · [Construction plan](../plan/desktop-audio-recorder-construction-plan.md)

## 1. English everywhere in code

| Surface | Required standard |
| --- | --- |
| Identifiers | English in variables, functions, types, modules, classes, components, flags, and test names. |
| Comments and technical documentation | English in comments, Rustdoc, JSDoc, security annotations, examples, and `TODO` / `FIXME` markers. Explain the reason behind a decision or a guarantee required by the code. |
| Executable repository | English in code file names, scripts, CI jobs, project-created configuration keys, and diagnostic messages. |
| Contracts | English in IPC commands, serialized fields, error codes, event names, and CLI arguments. Respect externally defined names from the OS, SDK, protocol, or library. |
| Tests | English in test names, scenario descriptions, and failure messages. Speech audio and reference transcriptions enter as test data. |
| Interface | All user-facing strings, messages, and file names are in American English. Keep them in a single string catalog keyed by stable English identifiers, separate from internal codes. |

Use `snake_case` for Rust functions, modules, and fields, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for constants. In TypeScript, use `camelCase` for functions and variables and `PascalCase` for components and types. In Swift, use `lowerCamelCase` for functions and properties and `UpperCamelCase` for types. Prefix C ABI symbols with `recorder_`. For IPC contracts, follow the names and serialization defined by the architecture, such as `start_recording` and `sourceId`.

Use names that express unit and scope. Examples: `sample_rate_hz`, `frame_count`, `elapsed_ms`, `peak_dbfs`, `source_id`. Human review validates language and clarity. Automatic checks can detect evident deviations using an allowed technical-term catalog. Diagnostic code uses stable codes such as `sourceUnavailable`, mapped to user-facing messages by the interface string catalog.

## 2. Structure and contracts

- Respect the existing architecture. The Rust engine is independent of the Tauri window. The interface controls presentation and interaction. Adapters implement native discovery, lifecycle, and capture. The Swift bridge exposes only the required macOS resources.
- Put session, file, resampling, and recovery decisions in the owner layer. Reuse session and source types. Introduce an abstraction only when it resolves a real boundary, such as separating the native API from a deterministic test.
- Validate IPC inputs, source IDs, audio formats, file paths, and buffer sizes at the boundary. Map failures to typed errors and present the user with the actionable next step. Keep internal code and cause separate from the user-facing message.
- Preserve compatibility with previously recorded preferences and manifests. Version schema changes and test reading older files, missing optional fields, and corrupted data. Document contract changes with input and output examples.
- Pin toolchain and dependency versions, keep lockfiles, and justify new dependencies by use case, platform support, and package impact. The release build should include only the resources and capabilities actually used.

## 3. Language-specific patterns

### Rust and native integration

- Apply `rustfmt` and Clippy to all workspace targets that compile on the job OS. Prefer enums for domain states and failures, `Result` for recoverable operations, and types that express units and invariants. Avoid `unwrap`, `expect`, and `panic!` in execution paths reached by user, disk, device, or permission failures.
- Keep `unsafe` at FFI and native API boundaries. Enable `unsafe_op_in_unsafe_fn` as an error. Each `unsafe` block should include an English `SAFETY:` comment with concrete preconditions for validity, size, alignment, ownership, thread, and duration as applicable. Review validates the preconditions at the call site. [Rust — unsafe operations in unsafe functions](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-op-in-unsafe-fn.html)
- Prefer RAII for buffers, handles, and OS resources. Release in reverse creation order. Keep the FFI context alive until the last callback. Define ownership for each pointer, free it with the correct allocator, and prevent exception or panic propagation across the C ABI.
- Preserve the required thread or context for the backend. On Windows, validate COM lifecycle and WASAPI flags. On macOS, test teardown for IOProc, aggregate device, and tap. On Linux, return PipeWire buffers and validate offset, size, and stride before reading. [Microsoft — GetBuffer](https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudiocaptureclient-getbuffer) · [Apple — Core Audio taps](https://developer.apple.com/documentation/coreaudio/capturing-system-audio-with-core-audio-taps) · [PipeWire — streams](https://docs.pipewire.org/page_streams.html)
- Use fixed-size C types and opaque handles in the Swift bridge. Keep the header as the ABI contract. Check nullable inputs, buffer capacity, and API availability. Use Swift errors or explicit codes at the boundary, with tests validating translation to Rust.

### TypeScript and Tauri

- Use `strict: true`, formatter, and lint rules against explicit `any` and ignored promises. Treat external data as `unknown` at the boundary and validate before use. Use discriminated unions for states and errors when that distinction changes behavior.
- Centralize the IPC bridge in a typed module. Test compatibility with Rust Serde types using common fixtures. Event subscriptions are explicitly discarded and ignore stale session revisions. Handle rejected commands and loading states. [TypeScript — strict](https://www.typescriptlang.org/tsconfig/strict.html) · [Tauri — calling Rust](https://v2.tauri.app/develop/calling-rust/)
- Prefer semantic controls and design-system tokens. Decouple meter updates from session state. Use the native snapshot for levels and envelope and limit visual updates to the frequency defined by the architecture.
- Restrict window permissions to the commands and destinations the app actually needs. The UI calls specific commands and never gains arbitrary filesystem access. Validate the file and session scope in Rust. Routine logs use codes and counts and keep audio, tokens, and full paths out of the log. [Tauri — capabilities](https://v2.tauri.app/security/capabilities/)

## 4. Capture, concurrency, and integrity rules

| Area | Implementation and review rule |
| --- | --- |
| Audio callback | Use preallocated buffers and a bounded queue. Keep writing, IPC, synchronous logging, blocking waits, and per-block allocation outside the callback. Test overflow and discard with frame counts. |
| Clocks | Use a monotonic clock and frame indices. Convert units at explicit boundaries. Preserve the distinction between legitimate silence, measurable gaps, and interruption by an unknown extension. |
| Session | Serialize transitions, invalidate old events by session ID or generation, and make stop idempotent. Undo partial preparation for two sources. Link reconnection to the same source. |
| Writing | Use a temporary path, checkpoint, header close, and rename in the same directory. Verify write results and synchronization. Preserve completed masters and test interruption between finalization steps. |
| DSP | Limit memory usage by session duration. Declare channel mapping and peak policy. Handle invalid samples, clipping, resampling, and phase cancellation. Apply export gain only to the copy. |
| Errors | Monitor failures per source, log the observable cause, and include a partial result when recoverable. Test the failure and cleanup in the same scenario. |

## 5. Testing strategy

| Level | Proposed tool | Minimum cases |
| --- | --- | --- |
| Rust unit | `cargo test` | State machine, session IDs, clock, PCM conversion, dBFS, resampling, rotation, frame calculation, manifest, and error handling. |
| Rust integration | Synthetic source, controlled clock, temporary folder | Full dual recording, silence, overflow, device removal, write error, repeated stop, rotation, and recovery after each commit step. |
| TypeScript unit and integration | Vitest with DOM test environment | IPC types and adapter, preferences, states, messages, selectors, and errors. Mock the IPC boundary and test observable UI behavior. |
| macOS bridge | XCTest and coverage for the Swift target | ABI, invalid parameters, errors, ownership, teardown, and simulated lifecycle. Complement with real tap and microphone tests. |
| Packaged desktop | Automation compatible with Tauri or a staged script | Open, select folder, test level, record, stop, open result, and recover session. Run on approved Windows, macOS, and Linux targets. |
| Hardware and performance | Bench by OS, known signals, and `ffprobe` | Permissions in the final installer, microphone and loopback, app isolation, USB/Bluetooth route changes, suspension, sync, and a two-hour session. |

Use small PCM fixtures generated with fixed parameters. Test async operations with synchronization signals, injected clock, and time limits to avoid arbitrary waits. Verify output and observable effects such as duration, status, lost frames, partial file, and released handles. Use numerical comparison with documented DSP tolerance. Add regression tests for corrected behavior. Automated coverage and hardware validation are independent criteria. [Rust — test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html) · [Tauri — tests](https://v2.tauri.app/develop/tests/) · [Tauri — WebDriver](https://v2.tauri.app/develop/tests/webdriver/)

### Required scenarios before releasing a platform

1. **Capture and session.** Open sources, interrupt during start, reject simultaneous start, disconnect the device, test silence, and finalize a session with two sources.
2. **File and recovery.** Write failure, disk full, folder removed, process interrupted between renames, and recovery of a WAV with an incomplete frame. Validate header, frame count, and preservation of existing masters.
3. **Audio quality.** Known tones, soft speech, saturation, inverted phase, resampling 48 kHz → 16 kHz, and timing under simulated drift. Validate alignment and isolation on hardware as well.
4. **Interface and permissions.** Permission denied, device unavailable, persisted preference, long path, keyboard navigation, and offline flow in the installed binary.

## 6. Coverage policy strictly above 80%

| Active scope | Blocking metric | Collection |
| --- | --- | --- |
| Shared Rust and Tauri commands | **Lines > 80%** in each scope, reported by OS | `cargo-llvm-cov` with `llvm-tools-preview` |
| Rust OS adapter | **Lines > 80%** separately on Windows, macOS, and Linux from each adapter integration | Native job report grouped by adapter files |
| TypeScript production | **Lines > 80%** and **branches > 80%** across the frontend | Vitest with V8 provider and explicit `coverage.include` for production code |
| macOS Swift bridge | **Lines > 80%** on the production target | XCTest instrumentation and Xcode/`xccov` coverage report |

The policy applies to code written by the team and shipped in the app, including native wrappers, error handling, and production files without direct tests. CI also publishes additional metrics when available, such as functions or regions. Do not sum reports across different languages or OSes to compensate for a low result. Planned but nonexistent components appear as pending in the matrix. An implemented scope must appear in its job report for merge.

### Gate rules

1. Use the **integer counts** from the report and approve only when `covered * 100 > total * 80`. With 100 lines, 80 covered fails and 81 passes. With 1,000 lines, 801 passes. Rounded values are for display only.
2. Fail if the report is missing, invalid, empty, or if an active scope has zero measured items. Match the versioned production source inventory to the report. An active executable file missing from the report requires correction to test or collection. A declarative source without executable lines may have zero items if instrumentation confirms that.
3. Report Rust modules by path and OS. Files under another `cfg(target_os)` belong to the respective native run. Use the Swift summary for the bridge target and the Vitest summary for the frontend. Archive machine reports and a readable summary in CI.
4. Exclude third-party code, generated sources, tests, and fixtures from the production denominator. Any other exclusion requires a precise path, justification, review, and alternative testing when applicable. Missing hardware, `unsafe`, FFI, or low coverage by themselves do not justify exclusion. Use controlled fakes to test adapter behavior and real tests to confirm integration.
5. Require these checks in merge and release builds. An execution with skipped tests must explicitly state which scenarios remain pending and why. A native validation pending for a platform blocks the release for that platform. [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) · [Vitest coverage configuration](https://vitest.dev/config/coverage) · [Apple — test results and coverage](https://developer.apple.com/documentation/xcode/running-tests-and-interpreting-results)

### CI reference commands

On bootstrap, pin versions and create scripts with names in English. `npm run coverage:gate` applies the gate rules above to the collected reports. In CI, run `npm ci` and `npm run build` before the Cargo steps, because the Tauri crate embeds the built frontend at compile time. The coverage script should read the reports and apply the exact comparison above. Example Rust and frontend collection, assuming the `coverage/` directory is prepared and the `test:coverage` script is configured.

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo llvm-cov --locked --workspace --json --output-path coverage/rust.json
npm ci
npm run lint
npm run typecheck
npm run test:coverage
npm run coverage:gate
npm run build
```

In Vitest, configure `coverage.include` for the production set, `json-summary` and `text` reports, and a dedicated frontend directory. On macOS, enable coverage in the test scheme that exercises the bridge and generate the report from the `.xcresult`. Validate the gate with fixtures that pass, fail at exactly 80%, fail without a report, and detect a missing production file.

## 7. Review and completion checklist

- [ ] Identifiers, comments, tests, diagnostics, and scripts in the change are in American English. User-facing strings come from the interface string catalog.
- [ ] The change respects app boundaries and contracts for IPC, files, permissions, and states. Older-version data remains readable when the contract is affected.
- [ ] The tests cover the requested behavior, primary failure path, and resource cleanup. Capture and recording changes include the relevant integrity scenarios.
- [ ] Formatting, lint, typecheck, tests, and build pass for the applicable targets. Each implemented scope has automated coverage strictly above 80% on the required metrics.
- [ ] Hardware and installer tests have evidence when the change affects capture, permissions, performance, or packaging. Record OS, command or script, result, and artifact.
- [ ] The project checkpoint records what was executed, what still depends on another host, and the next step. Codex, Claude Code, and Cursor use the same criteria.
