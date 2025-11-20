//! Error handling integration tests.

use term_test::{Result, TermTestError, TuiTestHarness};

#[test]
fn test_invalid_terminal_dimensions() {
    let result = TuiTestHarness::new(0, 24);
    assert!(result.is_err());

    let result = TuiTestHarness::new(80, 0);
    assert!(result.is_err());

    let result = TuiTestHarness::new(0, 0);
    assert!(result.is_err());
}

#[test]
fn test_invalid_resize_dimensions() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let result = harness.resize(0, 24);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TermTestError::InvalidDimensions { .. }
    ));

    let result = harness.resize(80, 0);
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_wait_exit_without_process() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let result = harness.wait_exit();
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TermTestError::NoProcessRunning
    ));

    Ok(())
}

#[test]
fn test_spawn_invalid_command() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;
    let mut cmd = portable_pty::CommandBuilder::new("this-command-definitely-does-not-exist-123456");

    let result = harness.spawn(cmd);
    // Should fail with spawn error
    assert!(result.is_err());

    Ok(())
}

#[test]
fn test_timeout_error_contains_context() -> Result<()> {
    let err = TermTestError::Timeout { timeout_ms: 5000 };
    let msg = err.to_string();

    assert!(msg.contains("5000"));
    assert!(msg.to_lowercase().contains("timeout"));

    Ok(())
}

#[test]
fn test_spawn_failed_error_message() {
    let err = TermTestError::SpawnFailed("test error message".to_string());
    let msg = err.to_string();

    assert!(msg.contains("test error message"));
    assert!(msg.contains("Failed to spawn"));
}

#[test]
fn test_process_already_running_error() {
    let err = TermTestError::ProcessAlreadyRunning;
    let msg = err.to_string();

    assert!(msg.to_lowercase().contains("already"));
    assert!(msg.to_lowercase().contains("running"));
}

#[test]
fn test_error_conversions() {
    // Test I/O error conversion
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let term_err: TermTestError = io_err.into();
    assert!(matches!(term_err, TermTestError::Io(_)));

    // Test anyhow error conversion
    let anyhow_err = anyhow::anyhow!("test error");
    let term_err: TermTestError = anyhow_err.into();
    assert!(matches!(term_err, TermTestError::Pty(_)));
}
