# P01.6 permissions evidence

**Purpose:** Records the permission runs against the signed probe bundle for P01.6 and why they are inconclusive. **Audience:** Reviewers of P01 and implementers of P07.2 and P09.3. **Status:** Recorded 2026-11-17; inconclusive.

| Field              | Value                                                                                                                                                                                                        |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Host               | macOS 27.0.1 (build 26A434), arm64, MacBook Pro bench                                                                                                                                                        |
| Bundle             | `HeedProbe.app`, `pro.edgarlima.heed.probe`, built by `scripts/build-macos-probe-app.sh` around `macos-capture-probe`; Apple Development identity, Hardened Runtime, `com.apple.security.device.audio-input` |
| Usage descriptions | `NSMicrophoneUsageDescription`: "Heed records your microphone when you start a recording." `NSAudioCaptureUsageDescription`: "Heed records the audio your Mac plays when you start a recording."             |
| Launch             | `open -n -W [--stdout <file>] [--stderr <file>] -a <path>/HeedProbe.app --args …`, so the bundle is its own TCC client                                                                                       |
| Reset              | Only `tccutil reset Microphone pro.edgarlima.heed.probe` and `tccutil reset AudioCapture pro.edgarlima.heed.probe`; the terminal's grants were never touched                                                 |
| Devices            | The default devices changed during the session: built-in microphone and speakers, then AirPods Pro (Bluetooth, 24 kHz input), then a Logi USB Headset (48 kHz)                                               |

## Observations

All times are UTC on 2026-11-17.

1. **Attribution.** With `open -n`, with and without stdio redirection, `responsibility_get_pid_responsible_for_pid` returned the probe's own PID (parent `launchd`), not the terminal's. The bundle, not iTerm, is the TCC client.
2. **First grant (17:16).** After a reset, `--request-access` waited about 5 minutes until the owner clicked Allow on the microphone prompt. The capture then ran. Its zeros came from the AirPods input, which the terminal also read as zeros, not from the grant. A later run on the Logi headset captured signal (peak -8.74 dBFS).
3. **System audio grant (17:24).** The owner saw and allowed a system-audio prompt. The capture held the tone (peak -39.06 dBFS).
4. **Revocation (17:25–17:29).** The owner switched HeedProbe off under Microphone and under Screen & System Audio Recording, and confirmed both lists showed it off. New launches still captured with signal at 17:25 and 17:28:50, from both the microphone and the system tap, and the microphone status query still reported granted. One microphone run at 17:28:33 returned exact zeros; its cause is unexplained.
5. **Reset (18:45–18:48).** A run started the moment `tccutil` returned still saw the old grant. With a 2 s wait, the status query reported not determined and the probe stopped with its access error.
6. **Batched session (18:56).** The harness waited until the status query reported not determined, then ran: microphone with `--request-access`, system audio twice, a second reset, and the same again. Every run captured with signal (microphone peaks -33 to -26 dBFS, system -39 dBFS), and the owner confirmed no prompt appeared. `requestAccess` returned granted without a prompt for a bundle whose status had just been reported as not determined.

7. **Fresh bundle IDs (19:49–19:50).** To rule out a stale decision, `scripts/build-macos-probe-app.sh` built two bundles TCC had never seen, `pro.edgarlima.heed.probe-deny` ("HeedProbe Deny") and `pro.edgarlima.heed.probe-revoke` ("HeedProbe Revoke"), with the built-in microphone and speakers as defaults. For each, the status query first reported not determined. Then `--request-access` returned granted within about 2 s, the microphone captured signal (peaks -28 to -6 dBFS), and the system tap captured the tone (-39 dBFS), on first launch and on relaunch. The owner confirmed no prompt appeared for either bundle. `profiles` reports no configuration profiles and no MDM enrollment. Both bundles' entries were reset afterwards.

## Result

Inconclusive. The bundle gets its own TCC identity, and the first grant prompts behaved as expected. After that, on this bench, bundles signed with the Apple Development identity got microphone and system-audio access with no prompt, even under bundle IDs TCC had never seen. Revocation in System Settings did not stop new launches within 4 minutes. No profile or MDM explains it, and without read access to the TCC database the bench cannot show which rule granted access. Denial was never exercised: no "Don't Allow" decision reached a probe.

Next: repeat grant, denial, and revocation with the real signed app bundle in P07.2 and in the installer check of P09.3, ideally in a fresh macOS user account or VM with no earlier decisions, and record the status query and an all-zero check after each relaunch.
