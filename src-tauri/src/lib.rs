//! Tauri application shell for the Heed desktop recorder.
//!
//! `configure` composes the app (commands, plugins, state) on any runtime so the
//! composition is testable on the mock runtime. `main.rs` only runs it on the real
//! window runtime.

use tauri::{Builder, Runtime};

/// Adds the application's commands, plugins, and state to a Tauri builder.
pub fn configure<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder
}

/// Maps the result of running the app to the process exit code.
///
/// A failure prints the stable code `appStartFailed` and the error to stderr.
pub fn exit_code(result: Result<(), tauri::Error>) -> std::process::ExitCode {
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("appStartFailed: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
