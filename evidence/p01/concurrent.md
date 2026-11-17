# P01.4 concurrent capture evidence

**Purpose:** Records the 60-second simultaneous microphone and system-audio capture, with start offset and clock observations for P03b. **Audience:** Reviewers of P01 and implementers of P03b. **Status:** Recorded 2026-11-17.

| Field     | Value                                                                                                                                                                                                                                                                                                 |
| --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Code      | `61370a7` (branch `p03a-common-engine`)                                                                                                                                                                                                                                                               |
| Host      | macOS 27.0.1 (build 26A434), arm64, MacBook Pro bench                                                                                                                                                                                                                                                 |
| Devices   | Default input "Microfone (MacBook Pro)", 1 channel, 48 kHz; default output "Alto-falantes (MacBook Pro)", 2 channels, 48 kHz                                                                                                                                                                          |
| Signal    | The 1 kHz tone from [system-audio.md](system-audio.md), playing from 1 s before the capture until after it; the microphone also hears it from the speakers                                                                                                                                            |
| Access    | Microphone and System Audio Recording granted to iTerm                                                                                                                                                                                                                                                |
| Command   | `cargo run --locked -q -p recorder-core --example macos-capture-probe -- --source both --output evidence/p01/concurrent-60s.wav --seconds 60`                                                                                                                                                         |
| Recorded  | 2026-11-17 16:47:22Z to 16:48:23Z (UTC, wall clock around the command)                                                                                                                                                                                                                                |
| Order     | Both handles are prepared first; then the microphone starts, then the system source                                                                                                                                                                                                                   |
| Artifacts | `evidence/p01/concurrent-60s-microphone.wav`, 11,520,058 bytes, SHA-256 `73bba799a446207a6cba16750617e2c7b467c61e3b373e7deb3b05b603fd4150`; `evidence/p01/concurrent-60s-system.wav`, 23,044,154 bytes, SHA-256 `df9eabe6fd006874bc4d97afa0a0a73c7af94ca316ba1d88f1a337611d1d4be9` (both git-ignored) |

Raw output in [concurrent-probe-summary.json](concurrent-probe-summary.json); `ffprobe` output in [concurrent-microphone-ffprobe.json](concurrent-microphone-ffprobe.json) and [concurrent-system-ffprobe.json](concurrent-system-ffprobe.json).

| Source     | Format                                | Frames    | Seconds | Drops / gaps / discontinuities | First host time (ns) | Last block host time (ns) | Host-measured rate | Peak / RMS           |
| ---------- | ------------------------------------- | --------- | ------- | ------------------------------ | -------------------- | ------------------------- | ------------------ | -------------------- |
| Microphone | 48 kHz, 1 ch, `f32`, 512-frame blocks | 2,880,000 | 60.000  | 0 / 0 / 0                      | 430,168,827,066,291  | 430,228,816,394,583       | 48,000.004 Hz      | -10.97 / -33.08 dBFS |
| System     | 48 kHz, 2 ch, `f32`, 512-frame blocks | 2,880,512 | 60.011  | 0 / 0 / 0                      | 430,168,845,748,125  | 430,228,845,743,083       | 48,000.004 Hz      | -38.90 / -42.08 dBFS |

`ffprobe` agrees: `pcm_f32le`, 48 kHz; microphone 1 channel, `duration_ts` 2880000 (60.000000 s); system 2 channels, `duration_ts` 2880512 (60.010667 s).

## Observations for P03b

- **Start offset.** The system source's first frame is 18.68 ms (18,681,834 ns of host time) after the microphone's first frame. Both run on 512-frame cycles of 10.67 ms, so the offset is under two cycles. The engine must align on first-frame host time, not on the order of start calls.
- **Clocks.** Both sources measured 48,000.004 Hz against the host clock over 60 s, a relative drift of 0.000 ppm. On this bench both devices share the built-in clock. A USB or Bluetooth device would not, so P03b still needs the drift estimator; this run gives it a zero-drift baseline.
- **Frame counts.** The system file is 512 frames longer because its last block ended later before stop. The common timeline is cut to one limit at stop, as the architecture requires.
- **Idle system output.** See [system-audio.md](system-audio.md): when nothing plays at start, the system source delivers no frames until playback begins.
