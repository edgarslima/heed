# P01.5 application scope evidence

**Purpose:** Records which processes carry Safari's and Chrome's audio, application-scoped captures of each browser, isolation between them, and self-exclusion from the global tap. **Audience:** Reviewers of P01 and implementers of P08.1. **Status:** Recorded 2026-11-17.

| Field    | Value                                                                                                                                                                                                                                                                                                                                                              |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Code     | `68d006b` (branch `p03a-common-engine`)                                                                                                                                                                                                                                                                                                                            |
| Host     | macOS 27.0.1 (build 26A434), arm64, MacBook Pro bench; default output "Alto-falantes (MacBook Pro)", 48 kHz                                                                                                                                                                                                                                                        |
| Browsers | Safari 27.0.1; Google Chrome 157.0.8103.79                                                                                                                                                                                                                                                                                                                         |
| Signals  | `ffmpeg -f lavfi -i "sine=frequency=<hz>:sample_rate=48000:duration=300" -af "volume=-20dB" -ac 2 tone-<hz>.wav` (ffmpeg 8.1.2). 600 Hz opened in Safari and 1500 Hz in Chrome with `open -a Safari tone-600.wav` and `open -a "Google Chrome" tone-1500.wav`; both browsers played the local file without a click. The two tones are not harmonics of each other. |
| Access   | System Audio Recording granted to iTerm                                                                                                                                                                                                                                                                                                                            |
| Recorded | 2026-11-17 17:08:48Z to 17:09:49Z (UTC, wall clock around the three captures)                                                                                                                                                                                                                                                                                      |

## Process map

Recorded while both tones played: `cargo run --locked -q -p recorder-core --example macos-capture-probe -- --list-processes`, filtered to browser processes in [application-processes.json](application-processes.json), and `ps -o pid,ppid,comm`.

| PID   | Parent | Bundle ID                  | Running output | Executable                                                  |
| ----- | ------ | -------------------------- | -------------- | ----------------------------------------------------------- |
| 677   | 1      | `com.apple.Safari`         | no             | `Safari.app/Contents/MacOS/Safari`                          |
| 1818  | 1      | `com.apple.WebKit.GPU`     | **yes**        | `WebKit.framework/…/XPCServices/com.apple.WebKit.GPU.xpc/…` |
| 75840 | 1      | `com.apple.WebKit.GPU`     | no             | the same XPC service, a second instance                     |
| 97771 | 1      | `com.google.Chrome`        | no             | `Google Chrome.app/Contents/MacOS/Google Chrome`            |
| 74154 | 97771  | `com.google.Chrome.helper` | **yes**        | `Google Chrome Helper (Aperitif).app` (audio service)       |
| 74155 | 97771  | `com.google.Chrome.helper` | no             | `Google Chrome Helper (Aperitif).app`                       |

## Captures

Each capture ran for 20 s with `--tones 600,1500`; levels are Goertzel amplitudes over the channel mean. All three delivered 958,464 frames at 48 kHz stereo with no queue drops, discontinuities, or sample-time gaps. Raw output: [control](application-control-probe-summary.json), [Chrome](application-chrome-probe-summary.json), [Safari](application-safari-probe-summary.json).

| Capture          | Command arguments                               | 600 Hz (Safari) | 1500 Hz (Chrome) | Isolation                   |
| ---------------- | ----------------------------------------------- | --------------- | ---------------- | --------------------------- |
| Global control   | `--source system`                               | -41.09 dBFS     | -41.08 dBFS      | none expected: both present |
| Chrome processes | `--source application --pids 74154,74155,97771` | below -120 dBFS | -41.09 dBFS      | at least 79 dB              |
| Safari processes | `--source application --pids 1818,75840,677`    | -41.09 dBFS     | below -120 dBFS  | at least 79 dB              |

"Below -120 dBFS" is the probe's floor (`amplitude_to_dbfs`), so the isolation figures are lower bounds set by the measurement, not by the tap.

WAV files (git-ignored), SHA-256:

- `evidence/p01/application-control-20s.wav` — `6ad4ddde1edaee367b7993894bf095601ccad284ac3fe91c50c7b5a30fcd888f`
- `evidence/p01/application-chrome-20s.wav` — `f315dff601e39335b2a8fb8a19c6d55cca2bf858429c7f5a6f740271c22744bd`
- `evidence/p01/application-safari-20s.wav` — `14f8513c2dccbeffc63e936abef8c5bd8ffbb275e8f0ec52c5a25cd3cdaf8c5a`

## Self-exclusion

The global tap excludes the recorder's own process object. The hardware test `HEED_HARDWARE_TESTS=1 swift test --filter CoreAudioLiveTests` plays a 440 Hz tone inside the test process and captures 1 s globally: the capture held no nonzero sample (the test accepts either no blocks or only zeros, since the IOProc may not run). The same test's positive case plays its tone from an `afplay` child process and captures it.

## Findings for P08.1

- Neither browser's main process plays audio. Chrome plays through a helper whose parent is the browser process. Safari plays through `com.apple.WebKit.GPU`, an XPC service whose parent is `launchd`, so the parent link does not tie it to Safari, and other WebKit apps can share that service.
- A process gets a Core Audio process object only after it has used audio, so scope resolution must run after playback starts, or be refreshed when new processes appear.
- Selecting all processes with a browser's bundle prefix (`com.google.Chrome`, or `com.apple.Safari` plus `com.apple.WebKit.GPU`) isolated the two browsers on this bench. Telling Safari's WebKit audio apart from another WebKit app needs more than the bundle ID.
