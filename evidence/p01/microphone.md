# P01.2 microphone evidence

**Purpose:** Records the 60-second microphone capture through the Swift bridge for P01.2. **Audience:** Reviewers of P01. **Status:** Recorded 2026-11-14.

| Field             | Value                                                                                                                                                                                                                         |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Code              | `d1ef9de` (branch `p03a-common-engine`)                                                                                                                                                                                       |
| Host              | macOS 27.0.1 (build 26A434), arm64, MacBook Pro bench                                                                                                                                                                         |
| Toolchain         | rustc 1.100.0, Xcode 27.2 / Swift 6.4.2, ffprobe 8.1.2                                                                                                                                                                        |
| Device            | Default input "Microfone (MacBook Pro)", 1 input channel, 48 kHz                                                                                                                                                              |
| Microphone access | Already granted to the terminal app that ran the probe (TCC attributes the request to the responsible app). No prompt appeared; `--request-access` was not needed. Grant, denial, and revocation in a signed build are P01.6. |
| Command           | `cargo run --locked -q -p recorder-core --example macos-microphone-probe -- --output evidence/p01/microphone-60s.wav --seconds 60`                                                                                            |
| Recorded          | 2026-11-14 17:23:55Z to 17:24:56Z (UTC, wall clock around the command)                                                                                                                                                        |
| Artifact          | `evidence/p01/microphone-60s.wav` (git-ignored), 11,520,058 bytes                                                                                                                                                             |
| SHA-256           | `2136aa156ec94fae57bfe52e7da6eef7cdeca627fa6e6acf9f96379452a975e3`                                                                                                                                                            |

## Probe summary

Raw output in [microphone-probe-summary.json](microphone-probe-summary.json).

```json
{
  "probe": "macos-microphone",
  "output": "evidence/p01/microphone-60s.wav",
  "requestedSeconds": 60,
  "elapsedSeconds": 60.066,
  "format": {
    "sampleRateHz": 48000,
    "channelCount": 1,
    "sampleFormat": "f32",
    "maxBlockFrames": 512
  },
  "callbacks": 5625,
  "blockFrames": { "min": 512, "max": 512 },
  "blocksWritten": 5625,
  "framesWritten": 2880000,
  "writtenSeconds": 60.0,
  "queueDrops": { "blocks": 0, "frames": 0 },
  "discontinuityBlocks": 0,
  "bridgeDiscontinuities": 0,
  "sampleTimeGaps": 0,
  "maxSampleTimeGapFrames": 0.0,
  "invalidHostTimes": 0,
  "invalidSampleTimes": 0,
  "firstHostTimeNs": 174168285198250,
  "hostSpanNs": 59989348000,
  "hostMeasuredRateHz": 47999.988,
  "peakDbfs": -25.31,
  "rmsDbfs": -40.89,
  "nonFiniteSamples": 0
}
```

## ffprobe

`ffprobe -v error -print_format json -show_format -show_streams evidence/p01/microphone-60s.wav`, relevant fields; full output in [microphone-ffprobe.json](microphone-ffprobe.json):

```json
{
  "codec_name": "pcm_f32le",
  "codec_tag": "0x0003",
  "sample_fmt": "flt",
  "sample_rate": "48000",
  "channels": 1,
  "bits_per_sample": 32,
  "time_base": "1/48000",
  "duration_ts": 2880000,
  "duration": "60.000000",
  "format_name": "wav",
  "size": "11520058"
}
```

## Observations for P03b and later steps

- AUHAL delivered a constant 512 frames per callback (about 10.7 ms); `kAudioUnitProperty_MaximumFramesPerSlice` reported 512.
- Host time and sample time were valid on every block. Sample time advanced by exactly the block size each time: no gaps, no bridge discontinuities, no queue drops.
- The host clock implies 47,999.988 Hz over 60 s, -0.25 ppm against the nominal rate. That is within the precision of one block-start timestamp and is not yet a drift estimate.
- The command spent 66 ms more than the audio length (60.066 s for 60.000 s), which covers start latency and teardown together; P01.4 measures the start offset separately.

## Result and limits

- Accepted for the P01.2 gate: decodable 60 s WAV, the format matches the negotiated one, duration 60.000 s, no drops.
- The signal is real capture, not silence: peak -25.31 dBFS, RMS -40.89 dBFS. A 2-second check of the room before the run measured peak -28.27 dBFS and RMS -40.23 dBFS, so this recording shows room sound rather than clear speech. Repeat with speech if a later step needs a speech fixture.
- Hardware XCTest: `HEED_HARDWARE_TESTS=1 swift test --filter CoreAudioLiveTests` passed. It ran three prepare/start/stop cycles of 1 s each; no callback ran after `stop` returned.
- The probe was renamed `macos-capture-probe` in P01.3; `--source microphone` reproduces this command.
