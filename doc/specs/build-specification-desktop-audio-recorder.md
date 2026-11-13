# Build Specification — Desktop Audio Recorder

**Purpose:** Product scope, functional requirements, file contract, and acceptance criteria. **Audience:** Project owner, engineers, QA, and AI agents. **Status:** Proposed, revision 1.0. Last reviewed: 2026-11-13.

| Field | Definition |
| --- | --- |
| Product | Local desktop audio recorder |
| Revision | 1.0 · proposed specification · 11/13/2026 |
| Priority | Windows 11 x64 and macOS 14.2+ Apple Silicon |
| Planned expansion | Linux x86-64 with PipeWire and WirePlumber |
| Stack | Tauri 2, TypeScript, Rust, native capture APIs |

The application opens in its own window. Interface features travel in the installer. Capture, recording, and export work offline after installation. Each platform has its own executable and package. The first delivery produces local high-quality files and a transcription copy. The user controls the start of capture and the output directory. [Tauri architecture](https://v2.tauri.app/concept/architecture/)

Related documents: [Design system](design-system-desktop-audio-recorder.md) · [Software architecture](software-architecture-desktop-audio-recorder.md) · [Construction plan](../plan/desktop-audio-recorder-construction-plan.md).

## 1. Scope and behavior by platform

| Interface mode | Windows 11 | macOS 14.2+ | Linux with PipeWire |
| --- | --- | --- | --- |
| **Microphone** | Selected physical input or default | Selected physical input or default | Selected input node or default |
| **System** | Mixer output of the selected endpoint or default | Global system audio tap without muting playback | Monitor of the selected sink or default |
| **Application** | Process loopback via WASAPI | Audio associated with the application via Core Audio taps | Identifiable application streams after validation |
| **Microphone + System** | Two independent sources | Two independent sources | Two independent sources |
| **Microphone + Application** | Two sources, subject to app support | Two sources, subject to app support | After app capture validation |

**RF-01 · Selection.** Choose exactly one mode, one microphone when applicable, and one playback source when applicable. “System default” is a preference resolved at the start for a concrete device. The app shows the active device or “System audio” in macOS system mode. The application mode shows the program name. PIDs and ephemeral node IDs are used only for the current session. The app persists only stable IDs when available.

**RF-02 · Browsers.** Capture Safari on macOS and Chrome, Edge, or Firefox on platforms where they are used via **System** mode. In **Application** mode, expose validated browsers and versions. Selecting Chrome may include multiple Chrome tabs and windows. The scope test includes audio from another app, two tabs, and browser restarts. Selecting a single tab remains a future enhancement. Safari isolation depends on identifying helper processes. [Microsoft — endpoint capture](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording) · [Microsoft — process loopback](https://learn.microsoft.com/en-us/samples/microsoft/windows-classic-samples/applicationloopbackaudio-sample/) · [Apple — taps](https://developer.apple.com/documentation/coreaudio/capturing-system-audio-with-core-audio-taps) · [PipeWire — loopback](https://docs.pipewire.org/page_module_loopback.html) · [Chrome — tabCapture](https://developer.chrome.com/docs/extensions/reference/api/tabCapture)

**RF-03 · Explicit scope.** If a source becomes unavailable, the app informs the user of the observable reason and keeps the selection visible. Offer **System** as an explicit choice when the application filter is unavailable. Invalid selection blocks starting until corrected. On Windows and Linux, **System** covers a single output per session. Protected media, exclusive output, hidden routes, and OS policy may restrict capture. Silence alone does not identify the root cause. [Microsoft — loopback restrictions](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording)

**Initial version limits.** One active recording per app instance, one window, and a maximum of two simultaneous streams. Multi-app selection, editing, pause, integrated transcription, file upload, audible monitoring by the recorder, and automatic gain control in capture are out of scope for this release.

## 2. Usage flow and functional requirements

| ID | Action | Behavior and acceptance criteria |
| --- | --- | --- |
| RF-04 | First use | Show **Microphone** and the default device if available. Prompt for a folder using the native selector. Opening the window leaves capture inactive. |
| RF-05 | Choose folder | Persist the selected folder locally. Canceling the selector keeps the previous selection. Reopening restores the folder and mode selection. If the folder disappears or loses access, show the previous destination and enable **Change folder**. |
| RF-06 | Test level | Capture only the selected sources after explicit action. Update the meter per source. **Stop test** frees the devices. Changing source or starting recording also stops the preview. |
| RF-07 | Record | Resolve sources, verify permissions, validate destination, and create temporary files. A dual-source session only starts if both sources open. If the second fails, release the first, report the failure, and allow a retry. |
| RF-08 | Active recording | Show a monotonic duration, meters, waveform, and **Stop**. Block changes to source, destination, and profile. Allow legitimate silence. Record observable delivery failures in the manifest. |
| RF-09 | Stop | End capture, drain queues, close WAV files, and confirm files in the same directory. A repeated stop command receives the current state. The user sees **Recording saved** after the masters are finalized. |
| RF-10 | Generate copy | Enabled by default. After saving the masters, produce the transcription WAV. Show **Preparing copy** during processing. If it fails, keep the masters and offer **Generate copy again**. |
| RF-11 | Show result | Show duration and session folder with **Open folder**. If gaps are present, include **Partial recording** and specify the source and duration of the interruption when known. |
| RF-12 | Close window | In preview, release capture and close. During recording, offer **Continue recording** and **Stop and close**. After the second choice, finalize the masters and the requested copy before closing. During finalization or export, inform the user that the window will close when the operation ends. |
| RF-13 | Recover | When opening or selecting a known folder, identify interrupted sessions and offer recovery. Preserve existing files. If possible, rebuild WAV files from complete frames and mark the result as recovered. |
| RF-14 | Handle disconnection | If the same source returns within 3 seconds, resume and show a measurable gap when applicable. If the source does not return or the format becomes incompatible, end recording and preserve the available segment. Never silently expand the scope of the selection. |
| RF-15 | Preferences | Persist folder, mode, stable selection by mode, and export option in versioned local configuration. Accept new fields and apply defaults when an optional value is missing. A configuration failure should indicate a new selection without modifying recordings. |

The 3-second value is an initial product limit. During the session, “default device” remains bound to the device resolved at the start. When only the default changes and the previous device remains active, the session continues with the same source. Machine suspension ends the session as partial when the backend can signal or detect it upon return. The application starts a new session only after finishing the previous one, including automatic export when enabled.

## 3. States and messages

| State | Main actions | Normal transition |
| --- | --- | --- |
| **Idle** | Source, folder, **Test level**, **Record** | Testing or Starting |
| **Testing** | **Stop test**, **Record**, change source | Idle or Starting |
| **Starting** | Wait, cancel start with **Stop** if requested | Recording, Idle, or Error |
| **Recording** | **Stop** or request close | Finalizing |
| **Finalizing** | Wait | Exporting, Saved, or Error |
| **Exporting** | **Open folder** for already-finalized masters | Saved or Saved with copy failure |
| **Saved** | **Open folder**, **Record**, regenerate copy if applicable | Starting or Exporting |
| **Error** | Fix cause, open partial result or recover if available | Idle or Saved |
| **Recovery available** | **Recover recording** or **Later** | Saved, Error, or Idle |

The recovery alert appears over **Idle**. The controller serializes start, stop, and close. A delayed response from an older session does not change the active session. Before recording, an invalid or inaccessible folder becomes a contextual action such as **Change folder** or **Open settings** when supported.

| Condition | Response to user |
| --- | --- |
| Capture open and below activity threshold | “No signal detected” after 3 seconds of sustained low level. Keep recording and timer active. |
| Permission denied or restricted | Identify microphone or system audio and guide the user to the relevant setting. |
| Device or app disappeared | Show the interrupted source and retry status. |
| Disk full or unavailable | End capture, save the recoverable segment when the destination allows it, and indicate the path. |
| Transcription copy failed | “Recording saved. The transcription copy failed.” Offer a retry. |
| Recording recovered | Show “Recovered partial recording” with the folder and available duration. |

## 4. File contract and quality

| Artifact | Contract |
| --- | --- |
| Session folder | `YYYY-MM-DD_HH-mm-ss_<suffix>`, created only within the selected destination |
| Master | `microphone.wav`, `system.wav`, or `application.wav`. WAV PCM 24-bit, 48 kHz, microphone mono by default, and system output stereo when available |
| Transcription copy | `transcription.wav`. WAV PCM 16-bit, 16 kHz, mono, generated after closing the masters. A dual-source session generates a combined copy |
| Long sessions | Rotate masters and copy at intervals up to 60 minutes, with incremental suffixes starting at `-002`, such as `system-002.wav` and `transcription-002.wav`. All masters rotate on the same sample timeline |
| Manifest | `manifest.json` versioned. Stores state, formats, segments, time position, gaps, and recovery. Sensitive local identities and audio remain on-device |

When the device reports a different rate, resample and record the source rate. Use DSP in `f32`. The master preserves the captured signal within the target profile, with the expected rate, channel, and bit-depth conversion. The app does not apply AGC, noise reduction, or dynamic compression to the master. The transcription copy uses fixed per-source gain after measuring audio, with an initial ceiling of +12 dB and a final sample peak of up to -3 dBFS. If speech estimation is insufficient, use a unity gain before adjusting the peak and emit an indication when low amplitude is relevant. Preserve the masters for separate transcription in simultaneous speech and to avoid cancellation in the downmix. Validate the gain policy with PT-BR and American English sample audio. [Google Cloud — audio best practices](https://docs.cloud.google.com/speech-to-text/docs/best-practices) · [Azure Speech — PCM input](https://learn.microsoft.com/en-us/azure/ai-services/speech-service/how-to-use-audio-input-streams)

Simultaneous writing budget for microphone mono and system stereo at 48 kHz/24 bits is approximately **432 kB/s**, or around **1,555.2 MB per hour**. The mono 16 kHz transcription copy adds approximately **115.2 MB/h**. Decimal values exclude headers. Initial precheck target: 256 MiB free and 64 MiB operating reserve. The precheck allows short sessions. The monitor during writing handles actual availability. The final volume may include partial audio when the OS interrupts write operations.

## 5. Acceptance criteria

| ID | Verifiable test | Acceptance |
| --- | --- | --- |
| AC-01 | Clean installation and offline use | Open from the OS menu, record, stop, and play the WAV. Restart and confirm the folder persists. |
| AC-02 | Sources and isolation | Test microphone, system, app, and dual modes available. With two identifiable signals, the system includes audio from the target endpoint or global scope, and the application excludes audio from the competing program. |
| AC-03 | Permissions and visibility | First use, denial, revocation, and return flow. Capture depends on explicit user action. Indicator and state match the actual capture. |
| AC-04 | Long-session integrity | Two hours under normal load. Zero lost frames from internal overflow, zero unreported gaps, valid files, correct rotation, and coherent duration. |
| AC-05 | Synchronization | Known pulses on both paths. Residual drift under 20 ms/h after subtracting fixed bench latency. |
| AC-06 | Performance | Average app + WebView CPU under 5% of total machine capacity during dual capture. Report p95, memory, machine config, and export separately. |
| AC-07 | Failure and recovery | Test process kill, disk full, folder removal, suspend, USB disconnect, and Bluetooth profile changes. Recovery target within 5 s after process kill on healthy local disk, measured separately from electrical faults. |
| AC-08 | Format and speech | Inspect header and samples. Compare WER on a PT-BR corpus and an American English corpus, scored separately, with the master, unadjusted copy, and adjusted copy. Approve the gain policy if it does not meaningfully worsen the test groups defined in the plan in either language. |
| AC-09 | Accessibility | Full flow by keyboard, visible focus, updated status messages, screen-reader compatibility, and layout without clipping at the design-system sizes. |
| AC-10 | Distribution | Validate the release package on the approved machines, with architecture, signature, and permission behavior recorded. |

Performance, synchronization, and recovery targets are proposed criteria. Validation must present measured values on each platform.
