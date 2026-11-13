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
