//! P01 probe: captures the default microphone, the system audio, the audio of chosen
//! processes, or the microphone and system audio at once through the Swift bridge, and
//! writes one IEEE-float WAV per source plus a JSON summary of the observed formats and
//! timestamps.
//!
//! Usage: `cargo run -p recorder-core --example macos-capture-probe -- --output <path.wav> \
//!   [--source microphone|system|both|application] [--pids <pid,...>] [--seconds <n>] \
//!   [--tones <hz,...>] [--summary <path.json>] [--request-access]`, or `-- --list-processes`.
//!
//! With `--source both`, the files are `<stem>-microphone.wav` and `<stem>-system.wav`.
//! `--tones` reports each tone's level over the channel mean with a Goertzel filter.
//!
//! A validation tool, not shipped code. Blocks travel from the audio callback through the
//! P03a block queue to a writer thread, as the engine will receive them.

#[cfg(target_os = "macos")]
fn main() -> std::process::ExitCode {
    probe::run()
}

#[cfg(not(target_os = "macos"))]
fn main() -> std::process::ExitCode {
    eprintln!("the capture probe runs on macOS only");
    std::process::ExitCode::from(2)
}

#[cfg(target_os = "macos")]
mod probe {
    use std::ffi::c_void;
    use std::fs::File;
    use std::io::{BufWriter, Seek, SeekFrom, Write};
    use std::path::{Path, PathBuf};
    use std::process::ExitCode;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    use recorder_core::audio::format::{AudioFormat, ChannelLayout, SampleFormat, SampleLayout};
    use recorder_core::audio::meter::amplitude_to_dbfs;
    use recorder_core::capture::block::{BlockFlags, BlockHeader, SourceId};
    use recorder_core::capture::queue::{
        BlockConsumer, BlockProducer, PushOutcome, QueueSizing, block_queue,
    };

    const RECORDER_OK: i32 = 0;
    const RECORDER_SOURCE_MICROPHONE: u32 = 1;
    const RECORDER_SOURCE_SYSTEM: u32 = 2;
    const RECORDER_SOURCE_APPLICATION: u32 = 3;
    const RECORDER_PROCESS_RUNNING_OUTPUT: u32 = 1 << 0;
    const RECORDER_PROCESS_RUNNING_INPUT: u32 = 1 << 1;
    const RECORDER_SAMPLE_F32: u32 = 1;
    const RECORDER_ACCESS_GRANTED: u32 = 2;
    const BLOCK_HOST_TIME_VALID: u32 = 1 << 0;
    const BLOCK_SAMPLE_TIME_VALID: u32 = 1 << 1;
    const BLOCK_DISCONTINUITY: u32 = 1 << 2;
    /// Smallest block assumed when sizing the queue to about one second.
    const MIN_BLOCK_FRAMES: u32 = 64;
    const WAV_HEADER_BYTES: u64 = 58;

    #[repr(C)]
    #[derive(Default)]
    struct RecorderFormat {
        sample_rate_hz: u32,
        channel_count: u32,
        sample_format: u32,
        max_block_frames: u32,
    }

    #[repr(C)]
    struct RecorderBlock {
        samples: *const f32,
        host_time_ns: u64,
        sample_time: f64,
        frame_count: u32,
        flags: u32,
    }

    #[repr(C)]
    struct RecorderProcess {
        pid: i32,
        flags: u32,
        bundle_id: [u8; 256],
    }

    type BlockCallback = unsafe extern "C" fn(*mut c_void, *const RecorderBlock);

    #[link(name = "RecorderBridge", kind = "static")]
    unsafe extern "C" {
        fn recorder_microphone_access(request: u32, access: *mut u32) -> i32;
        fn recorder_list_processes(
            processes: *mut RecorderProcess,
            capacity: u32,
            count: *mut u32,
        ) -> i32;
        fn recorder_create(handle: *mut *mut c_void) -> i32;
        fn recorder_set_processes(handle: *mut c_void, pids: *const i32, count: u32) -> i32;
        fn recorder_prepare(
            handle: *mut c_void,
            source_kind: u32,
            format: *mut RecorderFormat,
        ) -> i32;
        fn recorder_start(
            handle: *mut c_void,
            callback: Option<BlockCallback>,
            context: *mut c_void,
        ) -> i32;
        fn recorder_stop(handle: *mut c_void) -> i32;
        fn recorder_destroy(handle: *mut c_void) -> i32;
        fn recorder_last_os_status(handle: *mut c_void, status: *mut i32) -> i32;
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Source {
        Microphone,
        System,
        Application,
    }

    impl Source {
        fn name(self) -> &'static str {
            match self {
                Self::Microphone => "microphone",
                Self::System => "system",
                Self::Application => "application",
            }
        }

        fn kind(self) -> u32 {
            match self {
                Self::Microphone => RECORDER_SOURCE_MICROPHONE,
                Self::System => RECORDER_SOURCE_SYSTEM,
                Self::Application => RECORDER_SOURCE_APPLICATION,
            }
        }
    }

    struct Options {
        output: PathBuf,
        sources: Vec<Source>,
        pids: Vec<i32>,
        tones_hz: Vec<f64>,
        summary: Option<PathBuf>,
        seconds: u64,
        request_access: bool,
    }

    /// One prepared source: its bridge handle, callback state, and writer thread.
    struct Capture {
        source: Source,
        output: PathBuf,
        handle: *mut c_void,
        native: RecorderFormat,
        state: *mut CallbackState,
        done: Arc<AtomicBool>,
        writer: thread::JoinHandle<std::io::Result<WriterResult>>,
        start_result: i32,
        stop_result: i32,
    }

    struct Report {
        source: Source,
        output: PathBuf,
        native: RecorderFormat,
        state: Box<CallbackState>,
        written: WriterResult,
    }

    struct CallbackState {
        producer: BlockProducer,
        format: AudioFormat,
        channel_count: usize,
        sequence: u64,
        next_frame: u64,
        first_sample_time: Option<f64>,
        expected_sample_time: Option<f64>,
        sample_time_gaps: u64,
        max_sample_time_gap_frames: f64,
        bridge_discontinuities: u64,
        invalid_host_times: u64,
        invalid_sample_times: u64,
        first_host_time_ns: Option<u64>,
        last_host_time_ns: u64,
        last_block_start_frame: u64,
        min_block_frames: u32,
        max_block_frames: u32,
    }

    /// Goertzel filter for one tone over the per-frame channel mean.
    struct Goertzel {
        tone_hz: f64,
        coefficient: f64,
        previous: f64,
        before_previous: f64,
        frames: u64,
    }

    impl Goertzel {
        fn new(tone_hz: f64, sample_rate_hz: u32) -> Self {
            Self {
                tone_hz,
                coefficient: 2.0
                    * (std::f64::consts::TAU * tone_hz / f64::from(sample_rate_hz)).cos(),
                previous: 0.0,
                before_previous: 0.0,
                frames: 0,
            }
        }

        fn push(&mut self, sample: f64) {
            let current = sample + self.coefficient * self.previous - self.before_previous;
            self.before_previous = self.previous;
            self.previous = current;
            self.frames += 1;
        }

        /// Sine amplitude at the tone frequency, in dBFS.
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            reason = "frame counts stay far below 2^52 in a probe run; the level fits f32"
        )]
        fn level_dbfs(&self) -> f32 {
            let power = self.previous * self.previous + self.before_previous * self.before_previous
                - self.coefficient * self.previous * self.before_previous;
            let amplitude = if self.frames == 0 {
                0.0
            } else {
                2.0 * power.max(0.0).sqrt() / self.frames as f64
            };
            amplitude_to_dbfs(amplitude as f32)
        }
    }

    #[derive(Default)]
    struct WriterResult {
        tones: Vec<Goertzel>,
        frames: u64,
        blocks: u64,
        discontinuity_blocks: u64,
        peak: f32,
        energy: f64,
        non_finite: u64,
        dropped_blocks: u64,
        dropped_frames: u64,
    }

    pub fn run() -> ExitCode {
        if std::env::args().nth(1).as_deref() == Some("--list-processes") {
            return match list_processes() {
                Ok(listing) => {
                    println!("{listing}");
                    ExitCode::SUCCESS
                }
                Err(message) => {
                    eprintln!("probe failed: {message}");
                    ExitCode::FAILURE
                }
            };
        }
        let options = match parse_options() {
            Ok(options) => options,
            Err(message) => {
                eprintln!("{message}");
                return ExitCode::from(2);
            }
        };
        match capture(&options) {
            Ok(summary) => {
                println!("{summary}");
                // A bundle launched with `open` has no terminal, so it can write the summary to a file.
                if let Some(path) = &options.summary
                    && let Err(error) = std::fs::write(path, format!("{summary}\n"))
                {
                    eprintln!("probe failed: cannot write the summary: {error}");
                    return ExitCode::FAILURE;
                }
                ExitCode::SUCCESS
            }
            Err(message) => {
                eprintln!("probe failed: {message}");
                ExitCode::FAILURE
            }
        }
    }

    fn parse_options() -> Result<Options, String> {
        let usage = "usage: --output <path.wav> [--source microphone|system|both|application] \
                     [--pids <pid,...>] [--seconds <n>] [--tones <hz,...>] [--summary <path.json>] \
                     [--request-access], \
                     or --list-processes";
        let mut output = None;
        let mut sources = vec![Source::Microphone];
        let mut pids = Vec::new();
        let mut tones_hz = Vec::new();
        let mut summary = None;
        let mut seconds = 60;
        let mut request_access = false;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--output" => output = args.next().map(PathBuf::from),
                "--source" => {
                    sources = match args.next().as_deref() {
                        Some("microphone") => vec![Source::Microphone],
                        Some("system") => vec![Source::System],
                        Some("both") => vec![Source::Microphone, Source::System],
                        Some("application") => vec![Source::Application],
                        _ => {
                            return Err(
                                "--source needs microphone, system, both, or application".into()
                            );
                        }
                    };
                }
                "--pids" => {
                    pids = parse_list(args.next(), |pid: &i32| *pid > 0)
                        .ok_or("--pids needs positive process IDs separated by commas")?;
                }
                "--tones" => {
                    tones_hz = parse_list(args.next(), |hz: &f64| (1.0..=20_000.0).contains(hz))
                        .ok_or(
                            "--tones needs frequencies from 1 to 20000 Hz separated by commas",
                        )?;
                }
                "--seconds" => {
                    seconds = args
                        .next()
                        .and_then(|value| value.parse().ok())
                        .filter(|value| (1..=3_600).contains(value))
                        .ok_or("--seconds needs a whole number from 1 to 3600")?;
                }
                "--summary" => summary = args.next().map(PathBuf::from),
                "--request-access" => request_access = true,
                other => return Err(format!("unknown argument: {other}")),
            }
        }
        if sources.contains(&Source::Application) == pids.is_empty() {
            return Err("--pids goes with --source application, and only with it".into());
        }
        Ok(Options {
            output: output.ok_or(usage)?,
            sources,
            pids,
            tones_hz,
            summary,
            seconds,
            request_access,
        })
    }

    fn parse_list<T: std::str::FromStr>(
        value: Option<String>,
        valid: impl Fn(&T) -> bool,
    ) -> Option<Vec<T>> {
        let items: Vec<T> = value?
            .split(',')
            .map(|item| item.trim().parse().ok().filter(&valid))
            .collect::<Option<_>>()?;
        (!items.is_empty()).then_some(items)
    }

    /// Lists the processes Core Audio knows as a JSON array.
    fn list_processes() -> Result<String, String> {
        let mut count = 0;
        // SAFETY: a zero capacity with a null array only reads the count into a live u32.
        let result = unsafe { recorder_list_processes(std::ptr::null_mut(), 0, &raw mut count) };
        if result != RECORDER_OK {
            return Err(format!("recorder_list_processes returned {result}"));
        }
        // Room for processes that start between the two calls.
        let capacity = count + 16;
        let mut processes: Vec<RecorderProcess> = (0..capacity)
            .map(|_| RecorderProcess {
                pid: 0,
                flags: 0,
                bundle_id: [0; 256],
            })
            .collect();
        // SAFETY: `processes` holds `capacity` writable entries and `count` is a live u32.
        let result =
            unsafe { recorder_list_processes(processes.as_mut_ptr(), capacity, &raw mut count) };
        if result != RECORDER_OK {
            return Err(format!("recorder_list_processes returned {result}"));
        }
        processes.truncate(count.min(capacity) as usize);
        let entries: Vec<String> = processes
            .iter()
            .map(|process| {
                let end = process
                    .bundle_id
                    .iter()
                    .position(|&byte| byte == 0)
                    .unwrap_or(256);
                format!(
                    "{{\"pid\":{},\"bundleId\":\"{}\",\"runningOutput\":{},\"runningInput\":{}}}",
                    process.pid,
                    json_escape(&String::from_utf8_lossy(&process.bundle_id[..end])),
                    process.flags & RECORDER_PROCESS_RUNNING_OUTPUT != 0,
                    process.flags & RECORDER_PROCESS_RUNNING_INPUT != 0,
                )
            })
            .collect();
        Ok(format!("[{}]", entries.join(",")))
    }

    fn json_escape(text: &str) -> String {
        text.replace('\\', "\\\\").replace('"', "\\\"")
    }

    fn capture(options: &Options) -> Result<String, String> {
        if options.sources.contains(&Source::Microphone) {
            check_microphone_access(options.request_access)?;
        }
        let mut captures = Vec::new();
        for &source in &options.sources {
            let output = output_path(&options.output, source, options.sources.len() > 1);
            match open(source, output, options) {
                Ok(capture) => captures.push(capture),
                Err(message) => {
                    for capture in captures {
                        let _ = finish(capture);
                    }
                    return Err(message);
                }
            }
        }

        // Every source is prepared before the first starts, so the start offset excludes setup.
        let started = Instant::now();
        for capture in &mut captures {
            // SAFETY: `capture.state` stays allocated until `finish` frees it after the handle
            // is destroyed, which the bridge guarantees is after the last callback; only the
            // audio thread touches it in between.
            capture.start_result =
                unsafe { recorder_start(capture.handle, Some(on_block), capture.state.cast()) };
        }
        if captures
            .iter()
            .all(|capture| capture.start_result == RECORDER_OK)
        {
            thread::sleep(Duration::from_secs(options.seconds));
        }
        for capture in &mut captures {
            // SAFETY: `handle` is live; stop is idempotent and safe when start failed.
            capture.stop_result = unsafe { recorder_stop(capture.handle) };
        }
        let elapsed = started.elapsed();

        let mut reports = Vec::new();
        let mut failure = None;
        for capture in captures {
            match finish(capture) {
                Ok(report) => reports.push(report),
                Err(message) => {
                    failure.get_or_insert(message);
                }
            }
        }
        if let Some(message) = failure {
            return Err(message);
        }
        if reports
            .iter()
            .any(|report| report.source == Source::System && report.written.peak <= 0.0)
        {
            eprintln!(
                "warning: the system capture is all zeros; check that this terminal has \
                 System Audio Recording access and that audio was playing"
            );
        }
        Ok(summary(options, &reports, elapsed))
    }

    fn check_microphone_access(request: bool) -> Result<(), String> {
        let mut access = 0;
        // SAFETY: `access` is a live u32; the call blocks on this thread until it returns.
        let result = unsafe { recorder_microphone_access(u32::from(request), &raw mut access) };
        if result != RECORDER_OK || access != RECORDER_ACCESS_GRANTED {
            return Err(format!(
                "microphone access is not granted (result {result}, access {access}); \
                 rerun with --request-access or allow it in System Settings"
            ));
        }
        Ok(())
    }

    fn output_path(output: &Path, source: Source, suffixed: bool) -> PathBuf {
        if !suffixed {
            return output.to_path_buf();
        }
        let stem = output.file_stem().unwrap_or_default().to_string_lossy();
        let extension = output.extension().map_or(String::new(), |extension| {
            format!(".{}", extension.to_string_lossy())
        });
        output.with_file_name(format!("{stem}-{}{extension}", source.name()))
    }

    fn open(source: Source, output: PathBuf, options: &Options) -> Result<Capture, String> {
        let mut handle = std::ptr::null_mut();
        // SAFETY: `handle` is a live pointer slot; `finish` or the error path below destroys
        // the handle exactly once.
        let result = unsafe { recorder_create(&raw mut handle) };
        if result != RECORDER_OK {
            return Err(format!("recorder_create returned {result}"));
        }
        open_with(source, output, handle, options).inspect_err(|_| {
            // SAFETY: `handle` came from `recorder_create`, was never started, and is not
            // used after this call.
            unsafe { recorder_destroy(handle) };
        })
    }

    fn open_with(
        source: Source,
        output: PathBuf,
        handle: *mut c_void,
        options: &Options,
    ) -> Result<Capture, String> {
        let name = source.name();
        if source == Source::Application {
            let count = u32::try_from(options.pids.len()).map_err(|_| "too many PIDs")?;
            // SAFETY: `handle` is live and `pids` holds `count` values; the bridge copies them.
            let result = unsafe { recorder_set_processes(handle, options.pids.as_ptr(), count) };
            if result != RECORDER_OK {
                return Err(format!("{name}: recorder_set_processes returned {result}"));
            }
        }
        let mut native = RecorderFormat::default();
        // SAFETY: `handle` is live and `native` is a writable `recorder_format`.
        let result = unsafe { recorder_prepare(handle, source.kind(), &raw mut native) };
        if result != RECORDER_OK {
            return Err(format!(
                "{name}: recorder_prepare returned {result} (OSStatus {})",
                last_os_status(handle)
            ));
        }
        if native.sample_format != RECORDER_SAMPLE_F32 {
            return Err(format!(
                "{name}: unexpected sample format {}",
                native.sample_format
            ));
        }
        let channel_count = u16::try_from(native.channel_count).map_err(|_| "channel count")?;
        let layout = ChannelLayout::from_channel_count(channel_count)
            .map_err(|error| format!("{error:?}"))?;
        let format = AudioFormat::new(
            native.sample_rate_hz,
            layout,
            SampleFormat::F32,
            SampleLayout::Interleaved,
        )
        .map_err(|error| format!("{error:?}"))?;
        let sizing = QueueSizing::for_format(
            &format,
            MIN_BLOCK_FRAMES.min(native.max_block_frames),
            native.max_block_frames,
        );
        let (producer, consumer) = block_queue(sizing);

        let mut file = File::create(&output).map_err(|error| error.to_string())?;
        write_wav_header(&mut file, &format, 0).map_err(|error| error.to_string())?;
        let done = Arc::new(AtomicBool::new(false));
        let tones = options
            .tones_hz
            .iter()
            .map(|&tone_hz| Goertzel::new(tone_hz, native.sample_rate_hz))
            .collect();
        let writer = {
            let done = Arc::clone(&done);
            thread::spawn(move || write_blocks(consumer, file, format, tones, &done))
        };
        let state = Box::into_raw(Box::new(CallbackState {
            producer,
            format,
            channel_count: usize::from(channel_count),
            sequence: 0,
            next_frame: 0,
            first_sample_time: None,
            expected_sample_time: None,
            sample_time_gaps: 0,
            max_sample_time_gap_frames: 0.0,
            bridge_discontinuities: 0,
            invalid_host_times: 0,
            invalid_sample_times: 0,
            first_host_time_ns: None,
            last_host_time_ns: 0,
            last_block_start_frame: 0,
            min_block_frames: u32::MAX,
            max_block_frames: 0,
        }));
        Ok(Capture {
            source,
            output,
            handle,
            native,
            state,
            done,
            writer,
            start_result: RECORDER_OK,
            stop_result: RECORDER_OK,
        })
    }

    /// Destroys the handle, then reclaims the callback state and joins the writer.
    fn finish(capture: Capture) -> Result<Report, String> {
        let name = capture.source.name();
        let os_status = last_os_status(capture.handle);
        // SAFETY: `handle` came from `recorder_create` and is not used after this call.
        // Destroy stops the handle first, so no callback runs once it returns.
        let destroyed = unsafe { recorder_destroy(capture.handle) };
        // SAFETY: no callback runs after the destroy above, so this thread owns `state` again
        // and frees it exactly once.
        let state = unsafe { Box::from_raw(capture.state) };
        capture.done.store(true, Ordering::Release);
        let written = capture
            .writer
            .join()
            .map_err(|_| format!("{name}: the writer thread panicked"))?
            .map_err(|error| format!("{name}: writing the WAV failed: {error}"))?;
        if capture.start_result != RECORDER_OK {
            return Err(format!(
                "{name}: recorder_start returned {} (OSStatus {os_status})",
                capture.start_result
            ));
        }
        if capture.stop_result != RECORDER_OK {
            return Err(format!(
                "{name}: recorder_stop returned {} (OSStatus {os_status})",
                capture.stop_result
            ));
        }
        if destroyed != RECORDER_OK {
            return Err(format!("{name}: recorder_destroy returned {destroyed}"));
        }
        Ok(Report {
            source: capture.source,
            output: capture.output,
            native: capture.native,
            state,
            written,
        })
    }

    fn last_os_status(handle: *mut c_void) -> i32 {
        let mut status = 0;
        // SAFETY: `handle` is live and `status` is a writable i32.
        unsafe { recorder_last_os_status(handle, &raw mut status) };
        status
    }
    unsafe extern "C" fn on_block(context: *mut c_void, block: *const RecorderBlock) {
        // SAFETY: `context` is the `CallbackState` passed to `recorder_start`, used only on
        // this thread until stop returns; `block` is valid for the duration of this call.
        let (state, block) = unsafe { (&mut *context.cast::<CallbackState>(), &*block) };
        let frames = block.frame_count;
        state.min_block_frames = state.min_block_frames.min(frames);
        state.max_block_frames = state.max_block_frames.max(frames);
        if block.flags & BLOCK_DISCONTINUITY != 0 {
            state.bridge_discontinuities += 1;
        }
        if block.flags & BLOCK_SAMPLE_TIME_VALID == 0 {
            state.invalid_sample_times += 1;
        } else {
            let first = *state.first_sample_time.get_or_insert(block.sample_time);
            if let Some(expected) = state.expected_sample_time {
                let gap = block.sample_time - expected;
                if gap.abs() >= 1.0 {
                    state.sample_time_gaps += 1;
                    state.max_sample_time_gap_frames =
                        state.max_sample_time_gap_frames.max(gap.abs());
                }
            }
            state.expected_sample_time = Some(block.sample_time + f64::from(frames));
            // Frame positions follow the device sample clock, so a gap stays a gap.
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "non-negative frame offset"
            )]
            let position = (block.sample_time - first).max(0.0) as u64;
            state.next_frame = state.next_frame.max(position);
        }
        let host_time_valid = block.flags & BLOCK_HOST_TIME_VALID != 0;
        if host_time_valid {
            state.first_host_time_ns.get_or_insert(block.host_time_ns);
            state.last_host_time_ns = block.host_time_ns;
            state.last_block_start_frame = state.next_frame;
        } else {
            state.invalid_host_times += 1;
        }
        if block.samples.is_null() {
            return;
        }
        let byte_len = frames as usize * state.channel_count * size_of::<f32>();
        // SAFETY: the bridge guarantees `samples` holds `frame_count * channel_count`
        // initialized f32 values for this call; f32 has no invalid byte patterns.
        let pcm = unsafe { std::slice::from_raw_parts(block.samples.cast::<u8>(), byte_len) };
        let header = BlockHeader {
            source_id: SourceId::new(1),
            sequence: state.sequence,
            start_frame: state.next_frame,
            frame_count: frames,
            format: state.format,
            first_frame_host_time_ns: host_time_valid.then_some(block.host_time_ns),
            flags: BlockFlags {
                silent: false,
                discontinuity: block.flags & BLOCK_DISCONTINUITY != 0,
                timestamp_invalid: !host_time_valid,
            },
        };
        let _: PushOutcome = state.producer.push(header, pcm);
        state.sequence += 1;
        state.next_frame += u64::from(frames);
    }

    fn write_blocks(
        mut consumer: BlockConsumer,
        mut file: File,
        format: AudioFormat,
        tones: Vec<Goertzel>,
        done: &AtomicBool,
    ) -> std::io::Result<WriterResult> {
        let mut result = WriterResult {
            tones,
            ..WriterResult::default()
        };
        let channels = usize::from(format.channel_layout().channel_count());
        let mut output = BufWriter::new(&mut file);
        loop {
            // Read the flag before draining so blocks queued before stop are never missed.
            let finished = done.load(Ordering::Acquire);
            let mut drained = false;
            while let Some(block) = consumer.pop() {
                drained = true;
                result.blocks += 1;
                result.frames += u64::from(block.header().frame_count);
                if block.header().flags.discontinuity {
                    result.discontinuity_blocks += 1;
                }
                for frame in block.pcm().as_chunks::<4>().0.chunks(channels) {
                    let mut sum = 0.0;
                    for bytes in frame {
                        let sample = f32::from_le_bytes(*bytes);
                        if sample.is_finite() {
                            result.peak = result.peak.max(sample.abs());
                            result.energy += f64::from(sample) * f64::from(sample);
                            sum += f64::from(sample);
                        } else {
                            result.non_finite += 1;
                        }
                    }
                    #[allow(clippy::cast_precision_loss, reason = "at most 8 channels")]
                    let mean = sum / channels as f64;
                    for tone in &mut result.tones {
                        tone.push(mean);
                    }
                }
                output.write_all(block.pcm())?;
                consumer.recycle(block);
            }
            if finished {
                break;
            }
            if !drained {
                thread::sleep(Duration::from_millis(10));
            }
        }
        output.flush()?;
        drop(output);
        file.seek(SeekFrom::Start(0))?;
        write_wav_header(&mut file, &format, result.frames)?;
        file.sync_all()?;
        result.dropped_blocks = consumer.stats().dropped_blocks;
        result.dropped_frames = consumer.stats().dropped_frames;
        Ok(result)
    }

    /// Writes a 58-byte WAVE_FORMAT_IEEE_FLOAT header: RIFF, `fmt ` (18 bytes), `fact`, `data`.
    fn write_wav_header(
        output: &mut impl Write,
        format: &AudioFormat,
        frames: u64,
    ) -> std::io::Result<()> {
        let too_long = || std::io::Error::other("the recording exceeds the WAV size limit");
        let channels = format.channel_layout().channel_count();
        let block_align = u32::from(channels) * 4;
        let data_bytes = u32::try_from(frames * u64::from(block_align)).map_err(|_| too_long())?;
        let riff_bytes = data_bytes
            .checked_add(u32::try_from(WAV_HEADER_BYTES - 8).map_err(|_| too_long())?)
            .ok_or_else(too_long)?;
        let frame_count = u32::try_from(frames).map_err(|_| too_long())?;
        let mut header = Vec::with_capacity(58);
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&riff_bytes.to_le_bytes());
        header.extend_from_slice(b"WAVEfmt ");
        header.extend_from_slice(&18_u32.to_le_bytes());
        header.extend_from_slice(&3_u16.to_le_bytes());
        header.extend_from_slice(&channels.to_le_bytes());
        header.extend_from_slice(&format.sample_rate_hz().to_le_bytes());
        header.extend_from_slice(&(format.sample_rate_hz() * block_align).to_le_bytes());
        header.extend_from_slice(
            &u16::try_from(block_align)
                .map_err(|_| too_long())?
                .to_le_bytes(),
        );
        header.extend_from_slice(&32_u16.to_le_bytes());
        header.extend_from_slice(&0_u16.to_le_bytes());
        header.extend_from_slice(b"fact");
        header.extend_from_slice(&4_u32.to_le_bytes());
        header.extend_from_slice(&frame_count.to_le_bytes());
        header.extend_from_slice(b"data");
        header.extend_from_slice(&data_bytes.to_le_bytes());
        output.write_all(&header)
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "frame and nanosecond counts stay far below 2^52 in a probe run; RMS fits f32"
    )]
    fn summary(options: &Options, reports: &[Report], elapsed: Duration) -> String {
        let sources: Vec<String> = reports.iter().map(source_summary).collect();
        let concurrent = match reports {
            [microphone, system] => concurrent_summary(microphone, system),
            _ => String::new(),
        };
        format!(
            "{{\"probe\":\"macos-capture\",\"requestedSeconds\":{},\"elapsedSeconds\":{:.3},\
             \"sources\":[{}]{concurrent}}}",
            options.seconds,
            elapsed.as_secs_f64(),
            sources.join(","),
        )
    }

    /// Frames per second of host time between the first and the last timestamped block.
    #[allow(
        clippy::cast_precision_loss,
        reason = "frame and nanosecond counts stay far below 2^52 in a probe run"
    )]
    fn host_measured_rate_hz(state: &CallbackState) -> f64 {
        let span_ns = state
            .first_host_time_ns
            .map_or(0, |first| state.last_host_time_ns.saturating_sub(first));
        if span_ns == 0 {
            0.0
        } else {
            state.last_block_start_frame as f64 * 1e9 / span_ns as f64
        }
    }

    /// Start offset and relative clock drift between the two sources, for P03b.
    fn concurrent_summary(microphone: &Report, system: &Report) -> String {
        let offset_ns = match (
            microphone.state.first_host_time_ns,
            system.state.first_host_time_ns,
        ) {
            (Some(microphone), Some(system)) => {
                (i128::from(system) - i128::from(microphone)).to_string()
            }
            _ => "null".into(),
        };
        let ratio = |report: &Report| {
            host_measured_rate_hz(&report.state) / f64::from(report.native.sample_rate_hz)
        };
        format!(
            ",\"concurrent\":{{\"startOffsetNs\":{offset_ns},\"relativeDriftPpm\":{:.3}}}",
            (ratio(system) - ratio(microphone)) * 1e6
        )
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "frame counts stay far below 2^52 in a probe run; RMS fits f32"
    )]
    fn source_summary(report: &Report) -> String {
        let Report {
            source,
            output,
            native,
            state,
            written,
        } = report;
        let sample_count = written.frames * u64::from(native.channel_count);
        let rms = if sample_count == 0 {
            0.0
        } else {
            (written.energy / sample_count as f64).sqrt()
        };
        let host_span_ns = state
            .first_host_time_ns
            .map_or(0, |first| state.last_host_time_ns.saturating_sub(first));
        format!(
            concat!(
                "{{\"source\":\"{}\",\"output\":\"{}\",\"format\":{{\"sampleRateHz\":{},",
                "\"channelCount\":{},\"sampleFormat\":\"f32\",\"maxBlockFrames\":{}}},\"callbacks\":{},",
                "\"blockFrames\":{{\"min\":{},\"max\":{}}},\"blocksWritten\":{},\"framesWritten\":{},",
                "\"writtenSeconds\":{:.3},\"queueDrops\":{{\"blocks\":{},\"frames\":{}}},",
                "\"discontinuityBlocks\":{},\"bridgeDiscontinuities\":{},\"sampleTimeGaps\":{},",
                "\"maxSampleTimeGapFrames\":{:.1},\"invalidHostTimes\":{},\"invalidSampleTimes\":{},",
                "\"firstHostTimeNs\":{},\"lastHostTimeNs\":{},\"hostSpanNs\":{},",
                "\"hostMeasuredRateHz\":{:.3},\"peakDbfs\":{:.2},\"rmsDbfs\":{:.2},",
                "\"allZero\":{},\"nonFiniteSamples\":{},\"toneDbfs\":{{{}}}}}"
            ),
            source.name(),
            json_escape(&output.display().to_string()),
            native.sample_rate_hz,
            native.channel_count,
            native.max_block_frames,
            state.sequence,
            if state.sequence == 0 {
                0
            } else {
                state.min_block_frames
            },
            state.max_block_frames,
            written.blocks,
            written.frames,
            written.frames as f64 / f64::from(native.sample_rate_hz),
            written.dropped_blocks,
            written.dropped_frames,
            written.discontinuity_blocks,
            state.bridge_discontinuities,
            state.sample_time_gaps,
            state.max_sample_time_gap_frames,
            state.invalid_host_times,
            state.invalid_sample_times,
            state.first_host_time_ns.unwrap_or(0),
            state.last_host_time_ns,
            host_span_ns,
            host_measured_rate_hz(state),
            amplitude_to_dbfs(written.peak),
            amplitude_to_dbfs(rms as f32),
            written.peak <= 0.0,
            written.non_finite,
            written
                .tones
                .iter()
                .map(|tone| format!("\"{}\":{:.2}", tone.tone_hz, tone.level_dbfs()))
                .collect::<Vec<_>>()
                .join(","),
        )
    }
}
