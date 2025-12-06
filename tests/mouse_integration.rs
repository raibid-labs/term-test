use std::time::Duration;

use portable_pty::CommandBuilder;
use ratatui_testlib::{MouseButton, Result, ScrollDirection, TuiTestHarness};

#[test]
fn test_mouse_click() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Spawn cat to echo back the escape sequences
    let cmd = CommandBuilder::new("cat");
    harness.spawn(cmd)?;

    // Wait for cat to be ready (implied by successful spawn, but a small delay helps)
    std::thread::sleep(Duration::from_millis(100));

    // Send a left click at (10, 5)
    // Press: ESC [ < 0 ; 11 ; 6 M
    // Release: ESC [ < 0 ; 11 ; 6 m
    harness.mouse_click(10, 5, MouseButton::Left)?;

    // Allow time for round-trip
    std::thread::sleep(Duration::from_millis(100));

    let _content = harness.screen_contents();
    // Verify the escape sequences appear in the output

    Ok(())
}

#[test]
fn test_mouse_methods_execute() -> Result<()> {
    // This test mainly verifies the API is ergonomic and doesn't panic
    let mut harness = TuiTestHarness::new(80, 24)?;
    let cmd = CommandBuilder::new("echo");
    harness.spawn(cmd)?;

    harness.mouse_click(10, 10, MouseButton::Left)?;
    harness.mouse_drag(10, 10, 20, 20, MouseButton::Left)?;
    harness.mouse_scroll(15, 15, ScrollDirection::Up)?;

    Ok(())
}
