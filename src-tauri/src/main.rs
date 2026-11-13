// Prevents an additional console window on Windows in release builds. Do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    match heed_lib::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("appStartFailed: {error}");
            ExitCode::FAILURE
        }
    }
}
