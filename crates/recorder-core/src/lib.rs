//! Audio capture, processing, and storage engine for the Heed desktop recorder.
//!
//! This crate has no dependency on Tauri, the window, or IPC. The Tauri app in
//! `src-tauri` composes it; capture adapters, DSP, storage, and the session
//! state machine live here so they stay testable without a window.
