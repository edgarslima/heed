//! Verifies the app composition on the Tauri mock runtime, without a window.

use heed_lib::configure;
use tauri::test::{mock_builder, mock_context, noop_assets};

#[test]
fn configured_app_builds_on_the_mock_runtime() {
    let app = configure(mock_builder()).build(mock_context(noop_assets()));

    assert!(app.is_ok());
}
