# Desktop Audio Recorder P03a Common Engine Plan

**Purpose:** Step-level implementation plan for P03a (work breakdown P03.1–P03.4): capture contract, synthetic source, bounded block queue, PCM conversion, and metering. **Audience:** The engineer or AI agent executing P03a. **Status:** Executed on 2026-11-13; deviations and evidence in the P03a entry of `doc/status.md`. Last reviewed: 2026-11-13.

Related documents: [Work breakdown](desktop-audio-recorder-work-breakdown.md) · [Construction plan](desktop-audio-recorder-construction-plan.md) · [Software architecture](../specs/software-architecture-desktop-audio-recorder.md) · [Design system](../specs/design-system-desktop-audio-recorder.md) · [Coding practices](../standard/coding-practices-desktop-audio-recorder.md) · [Status](../status.md)

**Goal:** Give `recorder-core` a tested, allocation-free path from a capture source to converted `f32` samples and meter snapshots, using a deterministic synthetic source.

**Architecture:** `capture/` holds the contract (formats on the wire, blocks, events, errors, the source trait), the bounded block queue, and the synthetic source. `audio/` holds format types, PCM conversion, and metering. No Tauri, IPC, or `serde` dependency in `recorder-core`. Timeline, drift estimation, gap classification, and resampling are P03b and stay out.

**Tech stack:** Rust 1.100.0 (edition 2024), `rtrb` 0.4.1 (provisional, gated by the allocation test in Task 6), Node gate tooling from P00.

**Spec:** Architecture section 2 (capture contract and concurrency) and section 4 (conversion and metering rows); design system section 3 (meter thresholds); coding practices sections 3–6.

## Global constraints

- American English everywhere; Rust naming per coding practices section 1; units in names (`sample_rate_hz`, `frame_count`, `host_time_ns`, `peak_dbfs`).
- No `unwrap`, `expect`, or `panic!` outside tests; no `unsafe`.
- No allocation, I/O, locking, or logging in `BlockProducer::push` and `push_silence` (they run in the audio callback).
- All multi-byte PCM is little-endian (all supported targets are little-endian).
- Sample rates 8,000–384,000 Hz; 1–8 channels.
- Test convention: public-API tests live in `crates/recorder-core/tests/<topic>.rs`. Private helpers, if they need tests, use a sibling `src/<path>/tests.rs` declared with `#[cfg(test)] mod tests;`, excluded by rule. No in-file `#[cfg(test)]` test bodies.
- Tests are deterministic: no threads, sleeps, or wall-clock reads.
- Numerical tolerance: integer-to-`f32` conversions are exact (`assert_eq!`); dB values use an absolute tolerance of `0.001` dB; RMS of synthetic sines uses `0.01` dB.
- Names not in architecture section 2 are *proposed*; Task 8 records them in the architecture.
- Local verification uses `&&`-chained commands (never `set -e` in a subshell).

## Review focus

1. **Allocation in the callback path.** A push after buffers return out of order must still not allocate. Pinned by the Task 6 allocation test.
2. **A consumer that never recycles.** The producer must count drops, never block or panic. Pinned by Task 6 `pushes_drop_when_consumer_does_not_recycle`.
3. **Malformed byte buffers from adapters.** A 24-bit stereo buffer whose length is not a multiple of 6 must return an error, never index out of bounds. Pinned by Task 4 `rejects_partial_frames`.
4. **NaN or infinity from float devices.** They must be sanitized and counted, and meters must never report NaN. Pinned by Task 4 `sanitizes_non_finite_float_samples` and Task 8 `dbfs_of_nan_is_the_floor`.
5. **Long sessions with drift.** Host time over 24 h at 384 kHz with ±1000 ppm must not overflow. Pinned by Task 7 `host_time_survives_a_day_at_384_khz`.

---

### Task 1: Gate support for declarative Rust files

**Files:**
- Modify: `scripts/coverage-gate.mjs`, `scripts/coverage-gate.test.mjs`

**Interfaces:**
- Produces: scope field `declarativeFiles: Array<{ path: string, reason: string }>`. A listed file may be absent from the report (llvm-cov omits files without functions, verified 2026-11-13). Each entry must be a literal path that matches a source file and has a non-empty reason. The gate prints `declarative <path>: <reason>`, and fails with `declarative file has measured code: <path>` when the report lists the file with `lines.total > 0`.

- [x] **Step 1:** Write failing tests:
  - `accepts a declarative file that is absent from the report`
  - `fails a declarative file that the report measures`
  - `rejects a declarative file with a glob path, missing reason, or no matching source`
  - `prints a note for each declarative file`
- [x] **Step 2:** Run `npx vitest run scripts`. Expected: the 4 new tests fail.
- [x] **Step 3:** Implement. Validation sits with the `justifiedExclusions` checks; the missing-from-report loop skips declarative paths; the measured-code check runs for active scopes.
- [x] **Step 4:** Run `npx vitest run scripts`. Expected: all pass.
- [x] **Step 5:** Commit `feat: allow audited declarative files in the coverage gate`.

### Task 2: Audio format types and scope activation (P03.1)

**Files:**
- Create: `crates/recorder-core/src/audio/mod.rs`, `crates/recorder-core/src/audio/format.rs`, `crates/recorder-core/tests/format.rs`
- Modify: `crates/recorder-core/src/lib.rs` (`pub mod audio;`), `scripts/coverage-scopes.json`

**Interfaces (proposed names):**
- `pub enum SampleFormat { I16, I24, I32, F32 }` with `pub const fn bytes_per_sample(self) -> usize` (2, 3, 4, 4). `I24` is packed 3 bytes.
- `pub enum SampleLayout { Interleaved, Planar }`
- `pub enum ChannelLayout { Mono, Stereo, Discrete(u16) }` with `pub fn from_channel_count(count: u16) -> Result<ChannelLayout, FormatError>` (1 → Mono, 2 → Stereo, 3..=8 → Discrete) and `pub const fn channel_count(self) -> u16`.
- `pub struct AudioFormat`: private fields; constructor `AudioFormat::new(sample_rate_hz: u32, channel_layout: ChannelLayout, sample_format: SampleFormat, sample_layout: SampleLayout) -> Result<AudioFormat, FormatError>`; getters of the same names; `pub fn bytes_per_frame(&self) -> usize`; `pub fn frames_in_ms(&self, duration_ms: u32) -> u64` (rounds up).
- `pub enum FormatError { UnsupportedSampleRate(u32), UnsupportedChannelCount(u16) }`
- Constants: `MIN_SAMPLE_RATE_HZ = 8_000`, `MAX_SAMPLE_RATE_HZ = 384_000`, `MAX_CHANNELS = 8`.

- [x] **Step 1:** Write failing tests in `tests/format.rs`:
  - `bytes_per_frame_for_stereo_i24_is_6`
  - `frames_in_100_ms_at_48_khz_is_4800`
  - `frames_in_10_ms_at_44_1_khz_is_441`
  - `frames_in_1_ms_at_44_1_khz_rounds_up_to_45`
  - `rejects_sample_rates_outside_8_khz_to_384_khz` (7_999 and 384_001 rejected; 8_000 and 384_000 accepted)
  - `maps_channel_counts_to_layouts` (1 → Mono, 2 → Stereo, 6 → Discrete(6); 0 and 9 rejected)
- [x] **Step 2:** Run `cargo test -p recorder-core --test format`. Expected: compile failure (module missing).
- [x] **Step 3:** Implement `audio/format.rs`; `audio/mod.rs` declares `pub mod format;`.
- [x] **Step 4:** Run the test. Expected: PASS.
- [x] **Step 5:** In `coverage-scopes.json`, set `rust-shared` to `active`, add `exclude: ["crates/**/src/**/tests.rs"]`, and add `declarativeFiles` for `crates/recorder-core/src/lib.rs` and `crates/recorder-core/src/audio/mod.rs` (reason: module declarations only).
- [x] **Step 6:** Run `cargo llvm-cov --locked --workspace --json --output-path coverage/rust.json && npm run coverage:gate`. Expected: `PASS rust-shared`.
- [x] **Step 7:** Commit `feat: add audio format types and activate the rust-shared coverage scope`.

### Task 3: Blocks, flags, events, and errors (P03.1)

**Files:**
- Create: `crates/recorder-core/src/capture/mod.rs`, `crates/recorder-core/src/capture/block.rs`, `crates/recorder-core/src/capture/event.rs`, `crates/recorder-core/tests/capture-contract.rs`
- Modify: `crates/recorder-core/src/lib.rs` (`pub mod capture;`), `scripts/coverage-scopes.json` (declarative `capture/mod.rs`)

**Interfaces:**
- `pub struct SourceId(u32)`: `new(u32)`, `get() -> u32`; `Copy`, `Eq`, `Hash`, `Debug`. Opaque per session.
- `pub struct BlockFlags { pub silent: bool, pub discontinuity: bool, pub timestamp_invalid: bool }` (`Default`, `Copy`).
- `pub struct BlockHeader { pub source_id: SourceId, pub sequence: u64, pub start_frame: u64, pub frame_count: u32, pub format: AudioFormat, pub first_frame_host_time_ns: Option<u64>, pub flags: BlockFlags }`. These map one-to-one to architecture section 2 `AudioBlock` (rate, sample format, and channel layout live in `format`).
- `impl BlockHeader { pub fn byte_len(&self) -> usize }` returns `frame_count * format.bytes_per_frame()`.
- `pub enum CaptureError { PermissionDenied, SourceUnavailable, FormatUnsupported, CaptureInterrupted }` with `pub const fn code(self) -> &'static str` returning the architecture section 5 codes (`permissionDenied`, `sourceUnavailable`, `formatUnsupported`, `captureInterrupted`).
- `pub enum CaptureEvent { Disconnected { source_id, at_frame: Option<u64> }, FormatChanged { source_id, format: AudioFormat }, Discontinuity { source_id, at_frame: u64, lost_frames: Option<u64> }, Failed { source_id, error: CaptureError }, Ended { source_id, final_frame: u64 } }` with `pub fn source_id(&self) -> SourceId`.

- [x] **Step 1:** Write failing tests:
  - `header_byte_len_is_frames_times_bytes_per_frame` (480 frames, stereo I16 → 1_920)
  - `capture_error_codes_match_the_architecture` (all four literals)
  - `every_event_reports_its_source`
  - `default_flags_are_all_clear`
- [x] **Step 2:** Run `cargo test -p recorder-core --test capture-contract`. Expected: compile failure.
- [x] **Step 3:** Implement `capture/block.rs` and `capture/event.rs`.
- [x] **Step 4:** Run the test. Expected: PASS. Run the gate. Expected: `PASS rust-shared`.
- [x] **Step 5:** Commit `feat: add capture block, event, and error types`.

### Task 4: PCM conversion to finite f32 (P03.4)

**Files:**
- Create: `crates/recorder-core/src/audio/convert.rs`, `crates/recorder-core/tests/convert.rs`

**Interfaces:**
- `pub fn convert_to_f32(pcm: &[u8], format: &AudioFormat, out: &mut Vec<f32>) -> Result<ConversionReport, ConversionError>`. It clears `out` and writes `frames * channels` interleaved samples. It never shrinks capacity and grows it only when `out` is too small.
- `pub struct ConversionReport { pub frame_count: u32, pub sanitized_samples: u32 }`
- `pub enum ConversionError { PartialFrame { byte_len: usize, bytes_per_frame: usize } }`
- Scale: I16 ÷ 32,768; I24 (sign-extended) ÷ 8,388,608; I32 ÷ 2,147,483,648. F32 is kept as is when finite (values above 1.0 are preserved for clip detection); NaN and infinities become 0.0 and are counted. Planar input becomes interleaved output.

- [x] **Step 1:** Write failing tests (exact equality):
  - `converts_i16_extremes` (`[0x00,0x80]` → -1.0; `[0xff,0x7f]` → 32767/32768; `[0,0]` → 0.0)
  - `converts_i24_extremes` (`[0,0,0x80]` → -1.0; `[0xff,0xff,0x7f]` → 8388607/8388608)
  - `converts_i32_minimum_to_minus_one`
  - `sanitizes_non_finite_float_samples` (NaN, +inf, -inf, 0.5 → [0, 0, 0, 0.5], `sanitized_samples` 3)
  - `interleaves_planar_stereo` (L = [0.25, 0.5], R = [-0.25, -0.5] → [0.25, -0.25, 0.5, -0.5])
  - `rejects_partial_frames` (5 bytes of stereo I24 → `PartialFrame { byte_len: 5, bytes_per_frame: 6 }`)
  - `reuses_output_capacity` (pointer and capacity unchanged when `out` already has room)
- [x] **Step 2:** Run `cargo test -p recorder-core --test convert`. Expected: compile failure.
- [x] **Step 3:** Implement.
- [x] **Step 4:** Run the test. Expected: PASS. Run the gate. Expected: `PASS rust-shared`.
- [x] **Step 5:** Commit `feat: convert PCM blocks to finite interleaved f32`.

### Task 5: Queue sizing (P03.3)

**Files:**
- Create: `crates/recorder-core/src/capture/queue.rs`, `crates/recorder-core/tests/queue.rs`
- Modify: `crates/recorder-core/Cargo.toml` (`rtrb = "0.4.1"`)

**Interfaces:**
- `pub struct QueueSizing { pub slot_count: usize, pub slot_bytes: usize }`
- `QueueSizing::for_format(format: &AudioFormat, max_block_frames: u32) -> QueueSizing`: `slot_count = max(2, ceil(sample_rate_hz / max_block_frames))` (about 1 s of audio); `slot_bytes = max_block_frames * bytes_per_frame`.

- [x] **Step 1:** Write failing tests:
  - `sizes_one_second_of_10_ms_blocks_at_48_khz` (480 frames, stereo F32 → 100 slots of 3,840 bytes)
  - `rounds_slot_count_up_for_uneven_blocks` (44,100 Hz, 1,024 frames → 44 slots)
  - `keeps_at_least_two_slots` (8,000 Hz, 16,000 frames → 2)
- [x] **Step 2:** Run `cargo test -p recorder-core --test queue`. Expected: compile failure.
- [x] **Step 3:** Implement sizing.
- [x] **Step 4:** Run the test. Expected: PASS.
- [x] **Step 5:** Commit `feat: size block queues to one second of audio`.

### Task 6: Allocation-free block queue (P03.3)

**Files:**
- Modify: `crates/recorder-core/src/capture/queue.rs`, `crates/recorder-core/tests/queue.rs`
- Create: `crates/recorder-core/tests/queue-allocation.rs`

**Interfaces:**
- `pub fn block_queue(sizing: QueueSizing) -> (BlockProducer, BlockConsumer)`. It uses two `rtrb` rings of `slot_count` entries: filled blocks go from producer to consumer, and empty buffers go back from consumer to producer. All buffers are `Vec::with_capacity(slot_bytes)`, created up front and placed in the return ring.
- `BlockProducer::push(&mut self, header: BlockHeader, pcm: &[u8]) -> PushOutcome` copies `pcm` into a recycled buffer. It returns `Dropped(SizeMismatch)` if `pcm.len() != header.byte_len()`, `Dropped(Oversize)` if it exceeds `slot_bytes`, and `Dropped(QueueFull)` when no buffer is free.
- `BlockProducer::push_silence(&mut self, header: BlockHeader) -> PushOutcome` zero-fills `header.byte_len()` bytes and sets `flags.silent`, without reading device memory.
- After any drop, the next queued block has `flags.discontinuity = true`.
- `pub enum PushOutcome { Queued, Dropped(DropReason) }`; `pub enum DropReason { QueueFull, Oversize, SizeMismatch }`.
- `BlockConsumer::pop(&mut self) -> Option<ReceivedBlock>`; `BlockConsumer::recycle(&mut self, block: ReceivedBlock)`.
- `ReceivedBlock`: `header(&self) -> &BlockHeader`, `pcm(&self) -> &[u8]`.
- `stats(&self) -> QueueStats` on both ends: `QueueStats { queued_blocks: u64, dropped_blocks: u64, dropped_frames: u64 }`, shared through `Arc` atomics with `Relaxed` ordering.

- [x] **Step 1:** Write failing tests in `tests/queue.rs`:
  - `delivers_blocks_in_order_with_their_bytes`
  - `push_silence_zero_fills_and_flags_silent`
  - `rejects_mismatched_and_oversize_blocks`
  - `pushes_drop_when_consumer_does_not_recycle`: 4 slots of 480 frames; push 6 without recycling. Expect 4 queued, 2 dropped, `dropped_frames` 960.
  - `flags_the_block_after_a_drop_as_discontinuous`
  - `recycled_buffers_are_reused`
- [x] **Step 2:** Write the failing test in `tests/queue-allocation.rs`: `push_pop_and_recycle_do_not_allocate`. It uses a counting `#[global_allocator]` with a `const`-initialized thread-local counter. Warm up for 10 cycles, then run 10,000 push → pop → recycle cycles, recycling out of order with two blocks in flight. Assert zero allocations on the test thread.
- [x] **Step 3:** Run `cargo test -p recorder-core --test queue --test queue-allocation`. Expected: compile failure.
- [x] **Step 4:** Implement with `rtrb`.
- [x] **Step 5:** Run both tests. Expected: PASS. If the allocation test fails because of `rtrb`, record the evidence and pick another library. Do not write a lock-free structure (architecture section 2).
- [x] **Step 6:** Commit `feat: add an allocation-free bounded block queue`.

### Task 7: Capture source trait and synthetic source (P03.1, P03.2)

**Files:**
- Create: `crates/recorder-core/src/capture/source.rs`, `crates/recorder-core/src/capture/synthetic.rs`, `crates/recorder-core/tests/synthetic.rs`

**Interfaces:**
- ```rust
  pub trait CaptureSource {
      fn source_id(&self) -> SourceId;
      fn prepare(&mut self) -> Result<AudioFormat, CaptureError>;
      fn start(&mut self, producer: BlockProducer, events: std::sync::mpsc::Sender<CaptureEvent>) -> Result<(), CaptureError>;
      fn stop(&mut self) -> StopSummary;
  }
  ```
- `pub struct StopSummary { pub final_frame: u64, pub already_stopped: bool }`. `stop` is idempotent: the first call sends `Ended` once, and later calls send nothing and return `already_stopped: true`.
- `pub enum SyntheticSignal { Silence, Constant(f32), Sine { frequency_hz: f32, amplitude: f32 } }`
- `pub enum ScriptedEvent { SilentBlock, Discontinuity { lost_frames: u64 }, InvalidTimestamp, Disconnect }`
- `pub struct SyntheticConfig { pub source_id: SourceId, pub format: AudioFormat, pub signal: SyntheticSignal, pub block_frames: u32, pub start_host_time_ns: u64, pub drift_ppm: i32, pub script: Vec<(u64, ScriptedEvent)> }`, where the `u64` is a block index.
- `SyntheticSource::new(config) -> SyntheticSource`; `pump(&mut self, blocks: u32) -> Result<u32, CaptureError>` emits blocks on the calling thread and returns the count emitted. It returns `Err(SourceUnavailable)` before `start`, after `stop`, or after a scripted `Disconnect`.
- `pub fn host_time_ns(start_host_time_ns: u64, frame: u64, sample_rate_hz: u32, drift_ppm: i32) -> u64`: `start + frame * 10^15 / (rate * (10^6 + ppm))` in `u128`, floored. A positive ppm means the source clock runs fast.
- Behavior:
  - Samples are encoded in the configured sample format and layout. `SilentBlock` uses `push_silence`.
  - `Discontinuity` advances `start_frame` by `lost_frames`, sets the flag, and sends `CaptureEvent::Discontinuity`.
  - `InvalidTimestamp` sets `timestamp_invalid` with `first_frame_host_time_ns: None`.
  - `Disconnect` sends `Disconnected { at_frame }` and emits nothing further.

- [x] **Step 1:** Write failing tests in `tests/synthetic.rs`:
  - `pumps_sequential_blocks_with_contiguous_frames`
  - `sine_round_trips_through_conversion_within_one_i16_step` (1 kHz, amplitude 0.5, mono I16 at 48 kHz; decoded with `convert_to_f32`; max error ≤ 1/32,768)
  - `host_time_without_drift_is_exact` (48,000 frames at 48 kHz from 1,000 → 1,000 + 1,000,000,000)
  - `host_time_with_100_ppm_fast_clock` (4,800,000 frames at 48 kHz → start + 99,990,000,999)
  - `host_time_survives_a_day_at_384_khz` (33,177,600,000 frames, ±1000 ppm, no panic; +1000 ppm gives 86,313,686,313,686 ns)
  - `scripted_silence_discontinuity_and_invalid_timestamp`
  - `disconnect_stops_emission_and_reports_the_frame`
  - `stop_is_idempotent_and_sends_ended_once`
  - `pump_before_start_is_an_error`
- [x] **Step 2:** Run `cargo test -p recorder-core --test synthetic`. Expected: compile failure.
- [x] **Step 3:** Implement.
- [x] **Step 4:** Run the test. Expected: PASS. Run the gate. Expected: `PASS rust-shared`.
- [x] **Step 5:** Commit `feat: add the capture source trait and a deterministic synthetic source`.

### Task 8: Metering and snapshot slot (P03.4)

**Files:**
- Create: `crates/recorder-core/src/audio/meter.rs`, `crates/recorder-core/tests/meter.rs`

**Interfaces:**
- `pub const DBFS_FLOOR: f32 = -120.0;` and `pub fn amplitude_to_dbfs(amplitude: f32) -> f32` computes `20·log10(|a|)`, clamped to at least the floor; NaN and 0 map to the floor.
- `pub const CLIP_LEVEL: f32 = 32767.0 / 32768.0;` A window is clipped when any `|x| >= CLIP_LEVEL`.
- `pub struct MeterSnapshot { pub sequence: u64, pub end_frame: u64, pub peak_dbfs: f32, pub rms_dbfs: f32, pub clipped: bool, pub envelope_min: f32, pub envelope_max: f32 }`
- `Meter::new(format: &AudioFormat) -> Meter`: window = `frames_in_ms(100)`.
- `Meter::process(&mut self, interleaved: &[f32], start_frame: u64) -> Option<MeterSnapshot>` returns the last window completed in this call, or `None`.
  - Peak is the largest `|x|` across channels.
  - RMS is `sqrt(Σx² / (frames · channels))`, the average channel energy (architecture section 4).
  - Envelope is the min and max signed sample.
  - `sequence` increments per emitted window. No allocation after `new`.
- `pub struct SnapshotSlot`: `new() -> SnapshotSlot`, `Clone` (shared `Arc<Mutex<Option<MeterSnapshot>>>`).
  - `publish(&self, snapshot: MeterSnapshot)` uses `try_lock` and never blocks; on contention it keeps the newer snapshot pending and publishes it on the next call.
  - `take_latest(&self) -> Option<MeterSnapshot>`.

- [x] **Step 1:** Write failing tests (0.001 dB tolerance unless stated):
  - `dbfs_of_full_scale_is_zero`
  - `dbfs_of_half_scale_is_minus_6_0206`
  - `dbfs_of_silence_is_the_floor`
  - `dbfs_of_nan_is_the_floor`
  - `full_scale_sine_has_0_dbfs_peak_and_minus_3_0103_rms` (1 kHz, 48 kHz, mono; RMS within 0.01 dB)
  - `stereo_peak_uses_the_louder_channel_and_rms_averages_energy` (L constant 0.5, R 0.0 → peak -6.0206, RMS -9.0309)
  - `flags_clipping_at_the_i16_full_scale_code` (32767/32768 → clipped; 0.999 → not clipped)
  - `emits_a_window_every_100_ms` (4,799 frames → None; 1 more → `end_frame` 4,800, `sequence` 1)
  - `reports_the_signed_envelope` (samples [-0.25, 0.75, 0.1] padded to a window → min -0.25, max 0.75)
  - `snapshot_slot_keeps_only_the_latest`
- [x] **Step 2:** Run `cargo test -p recorder-core --test meter`. Expected: compile failure.
- [x] **Step 3:** Implement.
- [x] **Step 4:** Run the test. Expected: PASS. Run the gate. Expected: `PASS rust-shared`.
- [x] **Step 5:** Commit `feat: add 100 ms metering and a latest-value snapshot slot`.

### Task 9: Contract record and checkpoint

**Files:**
- Modify: `doc/specs/software-architecture-desktop-audio-recorder.md` (section 2), `doc/status.md`, `doc/plan/desktop-audio-recorder-work-breakdown.md` (P03 link)

- [x] **Step 1:** Add to architecture section 2 a short "Implemented contract (P03a)" table that maps each `AudioBlock` field to `BlockHeader` and names the accepted types (`AudioFormat`, `ChannelLayout`, `BlockFlags`, `CaptureEvent`, `CaptureError`, `CaptureSource`, `BlockProducer`/`BlockConsumer`, `MeterSnapshot`). Record `rtrb` as the selected queue library with the allocation-test evidence.
- [x] **Step 2:** Run the full local sequence (`npm ci && npm run build && cargo fmt --all -- --check && cargo clippy --locked --workspace --all-targets -- -D warnings && mkdir -p coverage && cargo llvm-cov --locked --workspace --json --output-path coverage/rust.json && npm run lint && npm run typecheck && npm run test:coverage && npm run coverage:gate`). Expected: exit 0.
- [x] **Step 3:** Push the branch and watch CI. Expected: green on the three OSes.
- [x] **Step 4:** Update the `doc/status.md` P03a entry with the plan section 4 template, then commit `docs: record P03a contract and checkpoint`.
