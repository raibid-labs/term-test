//! Basic integration tests for term-test.

use term_test::{Result, TuiTestHarness};

#[test]
fn test_create_harness() -> Result<()> {
    let harness = TuiTestHarness::new(80, 24)?;
    assert_eq!(harness.state().size(), (80, 24));
    Ok(())
}

#[test]
fn test_invalid_dimensions() {
    let result = TuiTestHarness::new(0, 24);
    assert!(result.is_err());

    let result = TuiTestHarness::new(80, 0);
    assert!(result.is_err());
}

#[test]
fn test_screen_state() -> Result<()> {
    let harness = TuiTestHarness::new(80, 24)?;
    let state = harness.state();

    // Initially empty
    assert!(state.contents().is_empty() || state.contents().trim().is_empty());

    // Cursor at origin
    let (row, col) = state.cursor_position();
    assert_eq!(row, 0);
    assert_eq!(col, 0);

    Ok(())
}

#[test]
fn test_timeout_configuration() -> Result<()> {
    use std::time::Duration;

    let harness = TuiTestHarness::new(80, 24)?
        .with_timeout(Duration::from_secs(10))
        .with_poll_interval(Duration::from_millis(5));

    // Just verify it doesn't panic
    Ok(())
}

#[test]
fn test_screen_contents_empty() -> Result<()> {
    let harness = TuiTestHarness::new(80, 24)?;
    let contents = harness.screen_contents();

    // Empty screen should be all spaces or empty
    assert!(contents.is_empty() || contents.trim().is_empty());
    Ok(())
}

#[test]
fn test_cursor_position_initial() -> Result<()> {
    let harness = TuiTestHarness::new(80, 24)?;
    let (row, col) = harness.cursor_position();

    // Initial cursor should be at origin
    assert_eq!(row, 0);
    assert_eq!(col, 0);
    Ok(())
}

#[test]
fn test_resize() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    harness.resize(100, 30)?;

    let state = harness.state();
    assert_eq!(state.size(), (100, 30));
    Ok(())
}

#[test]
fn test_resize_to_invalid_dimensions() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let result = harness.resize(0, 24);
    assert!(result.is_err());

    let result = harness.resize(80, 0);
    assert!(result.is_err());

    Ok(())
}
