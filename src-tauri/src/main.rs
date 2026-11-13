// Prevents an additional console window on Windows in release builds. Do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Entry point: runs the configured app on the real window runtime. Excluded from the
//! coverage gate because it needs a window event loop; see scripts/coverage-scopes.json.

use std::process::ExitCode;

fn main() -> ExitCode {
    let result = heed_lib::configure(tauri::Builder::default()).run(tauri::generate_context!());
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("appStartFailed: {error}");
            ExitCode::FAILURE
        }
    }
}
