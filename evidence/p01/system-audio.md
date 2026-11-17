# P01.3 system audio evidence

**Purpose:** Records the 60-second system-audio capture through the global tap and private aggregate device for P01.3. **Audience:** Reviewers of P01. **Status:** Recorded 2026-11-16.

| Field                         | Value                                                                                                                                                                                      |
| ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Code                          | `61370a7` (branch `p03a-common-engine`)                                                                                                                                                    |
| Host                          | macOS 27.0.1 (build 26A434), arm64, MacBook Pro bench                                                                                                                                      |
| Toolchain                     | rustc 1.100.0, Xcode 27.2 / Swift 6.4.2, ffprobe 8.1.2, ffmpeg (Homebrew) for the tone                                                                                                     |
| Output device                 | Default output "Alto-falantes (MacBook Pro)", 2 channels, 48 kHz; it clocks the aggregate device                                                                                           |
| Signal                        | `ffmpeg -f lavfi -i "sine=frequency=1000:sample_rate=48000:duration=75" -af "volume=-18dB" -ac 2 tone-1khz.wav`, played with `afplay` from 1 s before the capture until after it           |
| System Audio Recording access | Granted to iTerm, the terminal that ran the probe. A denied or missing grant produces exact zeros; the probe reports `allZero`. Grant, denial, and revocation in a signed build are P01.6. |
| Command                       | `cargo run --locked -q -p recorder-core --example macos-capture-probe -- --source system --output evidence/p01/system-audio-60s.wav --seconds 60`                                          |
| Recorded                      | 2026-11-16 12:05:35Z to 12:06:35Z (UTC, wall clock around the command)                                                                                                                     |
| Playback check                | The owner confirmed the tone stayed audible from the speakers through the whole capture.                                                                                                   |
| Artifact                      | `evidence/p01/system-audio-60s.wav` (git-ignored), 23,031,866 bytes                                                                                                                        |
| SHA-256                       | `c16a3fc443e9c59856fb1baf6a34cd286d3dabf6a7803e495a89897027f83f61`                                                                                                                         |

## Probe summary

Raw output in [system-audio-probe-summary.json](system-audio-probe-summary.json): 48 kHz, 2 channels, `f32`, 512-frame blocks; 5,623 callbacks; 2,878,976 frames (59.979 s); no queue drops, discontinuities, sample-time gaps, or invalid timestamps; host-measured rate 47,999.996 Hz; peak -38.90 dBFS, RMS -42.08 dBFS, `allZero` false.

## ffprobe

Full output in [system-audio-ffprobe.json](system-audio-ffprobe.json): `pcm_f32le`, `sample_rate` 48000, `channels` 2, `duration_ts` 2878976, `duration` 59.978667.

## Checks

- **Signal.** Peak minus RMS is 3.18 dB, close to the 3.01 dB of a pure sine. Zero crossings on the left channel give 1000.3 Hz. Left and right are identical over the first second, as expected from the stereo mixdown tap.
- **Duration.** 21 ms (two blocks) short of 60 s: the probe's timer starts when `recorder_start` returns, and the aggregate delivers its first cycle after that.
- **Cleanup.** `HEED_HARDWARE_TESTS=1 swift test --filter CoreAudioLiveTests` passed. It ran three prepare/start/stop cycles of 1 s each with its own tone. After each stop, no callback ran, and within 1 s neither the aggregate UID (`kAudioHardwarePropertyTranslateUIDToDevice`) nor the tap UUID (`kAudioHardwarePropertyTranslateUIDToTap`) resolved. The same held after a prepare that failed once the tap and aggregate existed. Both objects are private, so this check must run inside the process that created them.

## Observations for P03b and later steps

- The captured level is about 21 dB below the tone file at the bench's volume setting. The level that reaches the tap depends on the system output volume; record the volume in later level tests.
- **Idle output.** The IOProc does not run until something plays. A 20 s capture with the tone playing only from about 5 s to 12 s delivered 15.6 s of frames: none before playback began, then continuous frames (silence after playback stopped) until stop, with no sample-time gaps. The system source's first frame can therefore arrive seconds after start. P03b must place it on the timeline by host time and fill the leading interval with silence, not treat it as a loss.
