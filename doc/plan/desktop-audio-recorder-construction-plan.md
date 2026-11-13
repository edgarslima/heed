# Desktop Audio Recorder Construction Plan

**Purpose:** Delivery sequence, technical tests, and cross-tool execution workflow. **Audience:** Project owner, engineers, and AI agents. **Status:** Active, revision 1.2. Last reviewed: 2026-11-13.

**Execution order.** Prove capture and scope on macOS and Windows, build the engine and the application in a continuous flow, validate both executables, and implement Linux. The plan uses a repository, a set of specifications, and small tasks that Codex, Claude Code, or Cursor can resume from.

Revision 1.2 · 11/13/2026. [Build specification](../specs/build-specification-desktop-audio-recorder.md) · [Design system](../specs/design-system-desktop-audio-recorder.md) · [Software architecture](../specs/software-architecture-desktop-audio-recorder.md) · [Coding practices](../standard/coding-practices-desktop-audio-recorder.md) · [Work breakdown](desktop-audio-recorder-work-breakdown.md) · [P00 bootstrap plan](desktop-audio-recorder-p00-bootstrap.md) · [OKF policy](../okf/README.md)

## 0. Repository-alignment preflight

Before construction begins, align the implementation to the repository rules already in place:

- Keep the canonical project knowledge under `doc/` and do not introduce parallel knowledge stores outside it.
- Use `doc/README.md` and `doc/okf/README.md` as the canonical navigation and policy sources.
- Use `AGENTS.md`, `CLAUDE.md`, `.cursorrules`, and `.cursor/rules/*.mdc` only as lightweight entry points, not as the primary documentation system.
- Write all repository-facing text, scripts, comments, diagnostics, and code in American English.
- Use AI Memory as a startup hook and error-recovery layer: recover the active objective, decisions, blockers, and last checkpoint before resuming work or retrying failed operations.
- Keep `doc/status.md` as the project execution state file when it exists; otherwise create it in `doc/` and keep a small, machine-readable checkpoint of the last verified state.
- Preserve the file naming pattern already adopted in the repo: `doc/plan/*.md`, `doc/specs/*.md`, `doc/standard/*.md`, `doc/environment/*.sh`, with English names and no legacy Portuguese aliases.

This preflight is required because the repository already enforces the OKF model and the English-only rule. The construction plan below assumes those rules are active before the first implementation task is started.

## 1. Prepare the project

1. Create or confirm the application repository and keep the canonical documentation under `doc/` instead of `docs/`. Keep the evidence directory for validation outside the user artifact area. Version small synthetic fixtures and authorized test text.
2. Generate the Tauri 2 skeleton in TypeScript with Vite and npm. Fix the Node LTS, Rust, and dependency versions selected at the beginning. Register `Cargo.lock` and `package-lock.json`. Configure the minimum window permissions and confirm that the bundle uses local assets. Keep the resulting project root aligned with the repository conventions and the repository-specific conventions described in the OKF docs.
3. Prepare a Mac Apple Silicon machine, a physical Windows 11 x64 machine, and an Ubuntu 24.04 LTS x64 machine with PipeWire/WirePlumber. Use the MacBook Pro M5 as the macOS bench. Record OS versions, browser versions, SDKs, WebView versions, and audio routes. If a target machine is not available, document the exact missing dependency and the validation that remains blocked.
4. Establish a `doc/status.md` file with the next task, base commit, status by platform, and observed results. Keep `AGENTS.md` short. It points to the documents, repo commands, and scope rules.
5. Keep `CLAUDE.md` with `@AGENTS.md` or an equivalent pointer and tool-specific guidance if necessary. In Cursor, use the root `AGENTS.md` or a project rule in `.cursor/rules/` that references the same documents. The conversation from each tool can supplement context. Recoverable state stays in the repository. [Codex — AGENTS.md](https://developers.openai.com/codex/guides/agents-md) · [Claude Code — memory and imports](https://code.claude.com/docs/en/memory) · [Cursor — rules](https://cursor.com/docs/rules)

Notes before starting implementation:
- The project should not introduce a second documentation hierarchy such as `docs/` or a parallel internal playbook that competes with `doc/`.
- The repository already favors English-named files and documents; the build setup must follow that convention from the start.
- The repo requires coding standards to be authoritative; do not proceed with implementation before those standards are in the working context and available to the tool that will build the app.

| Environment | Development prerequisites |
| --- | --- |
| Windows | MSVC Build Tools, Windows SDK, Rust MSVC, Node LTS, WebView2, and packaging tools |
| macOS | Xcode/SDK, command-line tools, Rust, and Node LTS. Required for a signed and notarized distributable package |
| Linux | C/Rust toolchain, Node LTS, WebKitGTK headers, and PipeWire/WirePlumber active in the test environment |

Confirm requirements and versions before fixing the CI image. [Tauri — prerequisites](https://v2.tauri.app/start/prerequisites/) · [Tauri — distribution](https://v2.tauri.app/distribute/)

## 2. Implementation sequence

| Task | Depends on | Deliverable and required evidence |
| --- | --- | --- |
| **P00 · Bootstrap** | — | Tauri skeleton, `recorder-core` crate, CI lint/build, and basic packaged window. Register commands actually executed. |
| **P01 · macOS proof** | P00 | Run microphone and global system capture, validate simultaneity, preserve playback, confirm permission flow and cleanup of aggregated devices. Measure 60 s per mode and record format and timestamps. Test Safari and Chrome with concurrent audio to map helper processes. |
| **P02 · Windows proof** | P00 | Run WASAPI input and loopback capture, record silence and audio, switch endpoints, run process loopback, and measure second-app exclusion. Record formats, flags, and browser behavior. |
| **P03 · Common engine** | P00 (P03a), P01 (P03b), P02 (P03.7) | Block contract, synthetic source, bounded queue, timeline, DSP, and meter. Test format, rollover, invalid timestamps, simulated drift, overflow, and silence classification. |
| **P04 · WAV and recovery** | P03 | Writer per source, manifest, rotation, checkpoint, and recovery. Inject failure between writes and renames. Prove the completed master remains intact and partial output is detected. |
| **P05 · Transcription copy** | P04 | PCM 16/16 mono export, downmix, gain handling, sync, and peak limits. Test silence, phase inversion, low voice, pre-clipping, two-source sessions, and per-segment boundaries. |
| **P06 · Desktop interface** | P00, architecture IPC contract | Implement tokens, layout, focus, selection, folder, meters, states, and messages. Use UI fixtures while native integration is under development. Verify screenshots in light and dark modes at 560×600 and 420×480. |
| **P07 · Session integration** | P03, P04, P05, P06 | Connect backends, engine, IPC, and UI. Validate double-open, previous session state, idempotent stop, close flow, recovery, and copy failure handling. Test without network after packaging. |
| **P08 · Application filter** | P01, P02, P07 | Consolidate process resolution and scope. Validate by browser/OS. Keep the unavailable mode when no reliable isolation evidence exists. |
| **P09 · Release macOS and Windows** | P07, P08 | Measure performance and quality on the physical machines. Generate signed NSIS and notarized DMG packages. Test clean installation, permissions, and offline workflow. |
| **P10 · Linux** | P07 | Integrate PipeWire, prove input and monitor capture, test hotplug and Wayland sessions, and generate a `.deb` package. Validate the **Application** mode separately after proving scope and playback continuity. |
| **P11 · Linux release** | P10 | Clean validation on the approved target. Record versions, package dependencies, and matrix limits. Run the applicable AC-01 to AC-10 criteria. |

P03 is split into P03a, P03b, and P03.7 as defined in [work breakdown section 6](desktop-audio-recorder-work-breakdown.md#6-approved-deviation-split-p03): the engine is built with the macOS bench and synthetic sources, and P03.7 re-validates it against the Windows observations from P02 before P08. P01 and P02 can advance in separate branches. P06 can move forward with fixtures once the minimal IPC fields are stable. The integration owner resolves contract questions before merging concurrent work. A single agent edits each task at a time.

### Checkpoints by delivery

- **Native proof, P01–P02.** Decodable test files and a source matrix observed on hardware. The record includes behavior when the app is silent and when the audio route changes. If a filter fails, isolate the cause and keep the general mode functional.
- **Vertical slice, P03–P07.** Open the dev executable, select a folder, test the level, record, stop, and open the files. Reopen the app and confirm preferences. Confirm that the UI, engine, and file observe the same session.
- **Priority MVP, P08–P09.** Applicable criteria pass on Windows and Mac. Evidence files bind commit, environment, browser version, output, logs, and test files. Signing and notarization are release steps with credentials owned by the project owner.
- **Linux, P10–P11.** Release for the validated distro, architecture, and session combination. Application-mode capture may have a distinct scope depending on the environment. Record the published matrix.

## 3. Technical tests

| Group | Procedure | Evidence and criterion |
| --- | --- | --- |
| File | Capture a reference tone and voice. Inspect sample rate, channels, codec, and bit depth with `ffprobe`. Check frame count against duration and PCM size. | Header, frequency analysis, and sample comparison. Master at 48 kHz / 24 bits. Copy at 16 kHz / 16 bits mono. |
| Isolation | Use distinct frequency tones in two apps and, separately, in two browser tabs. Check the spectrum with the device and version recorded. | **System** mode includes the target signal in the chosen scope. **Application** mode includes the target browser signal and excludes the competing app. Initial bench criterion: component isolated at least 40 dB below the target signal after measuring background noise. |
| Clocks | Inject pulses in the playback path and microphone with a physical bench. Measure difference at the start, middle, and end. Use a simulated clock for regression. | Residual drift within 20 ms/h after subtracting fixed bench latency. Report gaps and events. |
| Long session | Capture two sources for 2 hours with meters. Record CPU, p95, memory, disk usage, lost frames, and rotation moment. | CPU average under 5% of total machine capacity, counting app and WebView. Zero internal overflow and zero unreported gaps. Confirm segment continuity. |
| Interruption | Disconnect USB/Bluetooth, deny access, switch route, suspend, kill the process during writing, and simulate disk space exhaustion on a test volume. | Result consistent with RF-13 and RF-14. Target recovery within 5 s after process termination on healthy local disk, measured separately from electrical failures. Keep completed files and demonstrate recovery. |
| Speech | Use a PT-BR corpus and an American English corpus, each with reference text. Score each language separately. Run at least four groups per language: clean speech, low speech, noisy speech, and a remote session with two sources. Run the same transcriber and parameters on the master, unadjusted copy, and adjusted copy. | Report WER by language, by group, and in total, plus clipping and auditory review. Initial criterion: treated copy deteriorates by no more than 2 percentage points per group versus untreated copy, subject to calibration in the pilot. Preserve fixture and scoring method. |
| UX and access | Navigate by keyboard, test light/dark mode, text scaling, dual meters, long folder paths, errors, recovery, and close states. | Screenshots, keyboard script, and accessibility verification on the approved platforms. |
| Distribution | Install the packages on clean machines. Confirm signing or notarization when applicable and test offline after installation. | SHA-256 of packages, version, commit, OS, and workflow result. |

Use synthetic fixtures for each data class and failure case. Hardware measurements and permission checks require a physical or equivalent validated bench. CI synthetic tests validate algorithms and integration; the matrix records the results actually tested on the target OS.

### Verification commands

The quality gate commands (format, lint, typecheck, tests, coverage, and build) are defined once, in [coding practices section 6](../standard/coding-practices-desktop-audio-recorder.md#6-coverage-policy-strictly-above-80). P00 creates the scripts and records versions. Run at the repo root with the platform toolchain. In addition, the app runs and packages with:

```sh
npm run tauri dev
npm run tauri build
```

Run native builds and tests in separate jobs on Windows, macOS, and Linux when the corresponding backend is included. Repeat capture tests on the installed artifact because signing and packaging can change permission behavior. Release secrets stay in the authorized release configuration.

Example of a bench file inspection.

```sh
ffprobe -v error -show_entries stream=codec_name,sample_rate,channels,bits_per_sample,duration -of json path/to/file.wav
```

## 4. Execution between Codex, Claude Code, and Cursor

The responsible engineer chooses the tool for the task. A practical split is Codex for spikes, integration, and validation on the host; Claude Code for implementing or reviewing an isolated backend; and Cursor for iterating the UI and inspecting local code. Any tool can assume another task using the same checkpoint.

### Task start prompt

```text
Read AGENTS.md, doc/status.md, and the sections relevant to task PXX
in the desktop recorder documents. Inspect git status and git log -1.
Confirm the available platform and which validations depend on hardware.
Implement PXX in the smallest coherent set of files. Use synthetic audio
only in tests. Run the available checks and record command, result,
environment, pending work, and next step in doc/status.md.
When you find a mismatch between the spec and observed behavior,
record the evidence and correct the issue in scope or flag the open decision.
```

### Resume prompt for another tool

```text
Resume PXX from doc/status.md. Read the diff and compare it to the last
recorded commit. Preserve in-progress work. Run first the smallest check
that confirms the latest change. Continue from the next step recorded and
update the checkpoint with real evidence. If the current host cannot run
the native test, leave the command and required environment for the next run.
```

### Review prompt before integration

```text
Review the diff for PXX against the RF/AC requirements cited in doc/status.md.
Inspect capture, concurrency, close flow, and produced file boundaries when the
change affects those points. Run the checks that can refute the completion claim.
List reproducible findings with file, condition, and result, and mark hardware
validation separately as pending.
```

### Checkpoint template in `doc/status.md`

```markdown
## PXX — title

- Status — to do, in progress, ready for review, or validated.
- Branch and base commit — real identifiers.
- Requirements covered — applicable RF and AC items.
- Changes — files and decisions with implementation impact.
- Evidence — command, environment, result, and artifact.
- Pending — tests or decisions, required environment, and reason.
- Next step — first concrete action when resuming.
```

Use one branch or worktree per concurrent task. Before switching tools, keep changes in a work commit or the same worktree with an identified diff. Switching tools does not require repeating already-observed tests. Re-run the smallest local check and the validations affected by the new diff. Checkpoints record facts without reproducing chat history or user audio data.

## 5. Execution risks and response

| Risk | Detection signal | Action |
| --- | --- | --- |
| Safari or Chrome emits audio via a helper process | Application filter misses part of the audio or includes another program | Inspect process sessions and objects in the spike. Approve the combination only after reproducing isolation. |
| Capture fails only in the packaged app | Debug mode works and the release returns permission denial or silence | Validate Info.plist, entitlement, signing identity, and app permissions in the installed application. |
| Drift between input and output | Pulses separate over time during the session | Fix clock mapping and resampling ratio before releasing dual mode. |
| Checkpoint or write degrades callback | The queue grows, frames are lost, or p95 CPU rises | Measure write time and synchronization. Split the writer worker if the single worker is the bottleneck observed. |
| Bluetooth changes profile | Rate, channels, or endpoint changes | Apply source reconnection policy and preserve the partial segment when necessary. |
| Distro or policy changes the route | PipeWire reconnects to another node or redirects playback | Fix the target for the session and validate the WirePlumber configuration in the package. |

The version is ready when the applicable criteria are executed and approved on the target machines, the artifacts are traceable to the code, and the source matrix reflects the observed result.
