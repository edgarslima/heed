// Prevents an additional console window on Windows in release builds. Do not remove.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Entry point: runs the configured app on the real window runtime. Excluded from the
//! coverage gate because it needs a window event loop; see scripts/coverage-scopes.json.

fn main() -> std::process::ExitCode {
    heed_lib::exit_code(
        heed_lib::configure(tauri::Builder::default()).run(tauri::generate_context!()),
    )
}
