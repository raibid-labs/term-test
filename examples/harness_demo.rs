//! Demonstration of TuiTestHarness features
//!
//! This example shows how to use the harness for testing TUI applications.

use mimic::{Result, TuiTestHarness};
use std::time::Duration;

fn main() -> Result<()> {
    println!("=== TuiTestHarness Demo ===\n");

    // Example 1: Basic usage with default settings
    println!("1. Creating harness with default settings...");
    let harness = TuiTestHarness::new(80, 24)?;
    println!("   Created 80x24 terminal");
    println!("   Timeout: {:?}", Duration::from_secs(5));
    println!("   Poll interval: {:?}\n", Duration::from_millis(100));

    // Example 2: Using the builder pattern
    println!("2. Creating harness with builder pattern...");
    let harness = TuiTestHarness::builder()
        .with_size(100, 30)
        .with_timeout(Duration::from_secs(10))
        .with_poll_interval(Duration::from_millis(50))
        .with_buffer_size(8192)
        .build()?;
    let (width, height) = harness.state().size();
    println!("   Created {}x{} terminal with custom settings\n", width, height);

    // Example 3: Cursor position tracking
    println!("3. Cursor position tracking...");
    let (row, col) = harness.cursor_position();
    println!("   Initial cursor at row: {}, col: {}", row, col);

    // Alternative method name
    let (row, col) = harness.get_cursor_position();
    println!("   (Using get_cursor_position): row: {}, col: {}\n", row, col);

    // Example 4: Manual state manipulation
    println!("4. Direct state manipulation...");
    let mut harness = TuiTestHarness::new(80, 24)?;
    harness.state_mut().feed(b"Test data on screen");
    let contents = harness.screen_contents();
    println!("   Fed data: 'Test data on screen'");
    println!("   Contains 'Test': {}\n", contents.contains("Test"));

    // Example 5: Escape sequences and cursor movement
    println!("5. Escape sequences and cursor movement...");
    harness.state_mut().feed(b"\x1b[5;10H"); // Move to row 5, col 10
    let (row, col) = harness.cursor_position();
    println!("   After escape sequence \\x1b[5;10H");
    println!("   Cursor at row: {}, col: {}\n", row, col);

    // Example 6: Resizing
    println!("6. Resizing terminal...");
    harness.resize(120, 40)?;
    let (width, height) = harness.state().size();
    println!("   Resized to {}x{}\n", width, height);

    // Example 7: Available methods summary
    println!("7. Available wait_for methods:");
    println!("   - wait_for(condition): Wait for a custom condition");
    println!("   - wait_for_with_context(condition, desc): Wait with error context");
    println!("   - wait_for_text(text): Wait for specific text");
    println!("   Note: These require a running process to avoid blocking\n");

    // Example 8: Builder pattern details
    println!("8. Builder pattern configuration:");
    let harness = TuiTestHarness::builder()
        .with_size(80, 24)
        .with_timeout(Duration::from_secs(5))
        .with_poll_interval(Duration::from_millis(100))
        .with_buffer_size(4096)
        .build()?;
    println!("   All settings configured via builder");
    println!("   Size: {}x{}", harness.state().size().0, harness.state().size().1);

    println!("\n=== Demo Complete ===");
    println!("\nKey features demonstrated:");
    println!("- Builder pattern for configuration");
    println!("- Cursor position tracking");
    println!("- Direct state manipulation");
    println!("- Terminal resizing");
    println!("- Escape sequence processing");

    Ok(())
}
