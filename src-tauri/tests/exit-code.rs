//! Verifies how the app's run result maps to the process exit code.

use heed_lib::exit_code;
use std::process::ExitCode;

#[test]
fn successful_run_exits_with_success() {
    assert_eq!(exit_code(Ok(())), ExitCode::SUCCESS);
}

#[test]
fn failed_run_exits_with_failure() {
    assert_eq!(
        exit_code(Err(tauri::Error::FailedToReceiveMessage)),
        ExitCode::FAILURE
    );
}
