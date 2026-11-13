//! Tauri application shell for the Heed desktop recorder.

/// Builds and runs the Tauri application until the last window closes.
///
/// # Errors
///
/// Returns the Tauri error when the runtime fails to start or run.
pub fn run() -> Result<(), tauri::Error> {
    tauri::Builder::default().run(tauri::generate_context!())
}
