//! Basic usage example of ratatui_testlib.
//!
//! This example demonstrates the core functionality of the ratatui_testlib library:
//! - Creating a PTY-based test harness
//! - Spawning processes in the pseudo-terminal
//! - Capturing and inspecting screen output
//! - Tracking cursor position
//! - Waiting for specific text to appear
//! - Using the builder pattern for custom configuration
//!
//! # Running This Example
//!
//! ```bash
//! cargo run --example basic_test
//! ```
//!
//! # Expected Output
//!
//! This example demonstrates several common testing patterns:
//! 1. Simple command output capture (echo)
//! 2. Multi-line output with formatting (printf)
//! 3. Waiting for text conditions
//! 4. Cursor position tracking
//! 5. Builder pattern configuration
//!
//! Each section shows the captured screen contents and cursor position.

use std::time::Duration;

use portable_pty::CommandBuilder;
use ratatui_testlib::{Result, TuiTestHarness};

fn main() -> Result<()> {
    println!("=== Basic ratatui_testlib Example ===\n");

    // Example 1: Simple echo command
    // This demonstrates the most basic usage: spawning a command and capturing output
    example_1_simple_echo()?;

    // Example 2: Multi-line output with formatting
    // Shows how to handle escape sequences and multiple lines
    example_2_multiline_output()?;

    // Example 3: Waiting for text conditions
    // Demonstrates the wait_for_text helper for async-like waiting
    example_3_wait_for_text()?;

    // Example 4: Builder pattern configuration
    // Shows advanced configuration with custom timeouts
    example_4_builder_pattern()?;

    // Example 5: Cursor position tracking
    // Demonstrates tracking cursor movements via escape sequences
    example_5_cursor_tracking()?;

    println!("\n=== All Examples Completed Successfully ===");

    Ok(())
}

/// Example 1: Simple echo command
///
/// Demonstrates:
/// - Creating a basic test harness
/// - Spawning a simple command
/// - Capturing screen output
/// - Checking process exit status
fn example_1_simple_echo() -> Result<()> {
    println!("--- Example 1: Simple Echo Command ---");

    // Create a test harness with standard 80x24 dimensions
    let mut harness = TuiTestHarness::new(80, 24)?;
    println!("Created terminal: 80 columns x 24 rows");

    // Spawn a simple echo command
    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("Hello from ratatui_testlib!");

    harness.spawn(cmd)?;
    println!("Spawned: echo 'Hello from ratatui_testlib!'");

    // Give the command time to execute and output
    std::thread::sleep(Duration::from_millis(100));

    // Update screen state from PTY output
    harness.update_state()?;

    // Get and display screen contents
    let contents = harness.screen_contents();
    println!("\nScreen contents:");
    println!("┌{:─<80}┐", "");
    for line in contents.lines().take(3) {
        println!("│{:<80}│", line);
    }
    println!("└{:─<80}┘", "");

    // Check cursor position
    let (row, col) = harness.cursor_position();
    println!("Cursor position: row={}, col={}", row, col);

    // Verify the text appears on screen
    assert!(
        contents.contains("Hello from ratatui_testlib!"),
        "Expected text not found in output"
    );

    // Note: wait_exit() can hang with very short-lived processes
    // In real tests, use is_running() to check if process has exited
    std::thread::sleep(Duration::from_millis(100));

    if !harness.is_running() {
        println!("Process has exited");
    }

    println!();
    Ok(())
}

/// Example 2: Multi-line output with formatting
///
/// Demonstrates:
/// - Handling multi-line output
/// - Parsing escape sequences
/// - Line-by-line inspection
fn example_2_multiline_output() -> Result<()> {
    println!("--- Example 2: Multi-line Output ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use printf to generate multi-line formatted output
    let mut cmd = CommandBuilder::new("printf");
    cmd.arg("Line 1: First\\nLine 2: Second\\nLine 3: Third");

    harness.spawn(cmd)?;
    println!("Spawned: printf with 3 lines");

    std::thread::sleep(Duration::from_millis(100));
    harness.update_state()?;

    let contents = harness.screen_contents();
    println!("\nScreen contents:");
    println!("┌{:─<80}┐", "");
    for (i, line) in contents.lines().take(5).enumerate() {
        println!("│{:2} {:<77}│", i, line);
    }
    println!("└{:─<80}┘", "");

    // Verify each line appears correctly
    assert!(contents.contains("Line 1: First"), "First line not found");
    assert!(contents.contains("Line 2: Second"), "Second line not found");
    assert!(contents.contains("Line 3: Third"), "Third line not found");

    println!("All three lines captured successfully");

    println!();
    Ok(())
}

/// Example 3: Waiting for text conditions
///
/// Demonstrates:
/// - wait_for_text() helper method
/// - Polling for expected output
/// - Timeout handling
fn example_3_wait_for_text() -> Result<()> {
    println!("--- Example 3: Waiting for Text ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Spawn a command that produces output after a delay
    // We use sh -c to run a compound command
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("sleep 0.2 && echo 'Ready!'");

    harness.spawn(cmd)?;
    println!("Spawned: delayed echo command");

    // Wait for the expected text to appear (polls automatically)
    println!("Waiting for 'Ready!' to appear...");
    harness.wait_for_text("Ready!")?;

    println!("✓ Text 'Ready!' appeared on screen");

    let contents = harness.screen_contents();
    println!("\nFinal screen contents:");
    println!("┌{:─<80}┐", "");
    for line in contents.lines().take(3) {
        println!("│{:<80}│", line);
    }
    println!("└{:─<80}┘", "");

    println!();
    Ok(())
}

/// Example 4: Builder pattern configuration
///
/// Demonstrates:
/// - Using the builder pattern for configuration
/// - Setting custom timeouts
/// - Setting custom poll intervals
/// - Setting custom buffer sizes
fn example_4_builder_pattern() -> Result<()> {
    println!("--- Example 4: Builder Pattern Configuration ---");

    // Create a harness with custom settings using the builder
    let mut harness = TuiTestHarness::builder()
        .with_size(100, 30)                              // Larger terminal
        .with_timeout(Duration::from_secs(10))           // Longer timeout
        .with_poll_interval(Duration::from_millis(50))   // Faster polling
        .with_buffer_size(8192)                          // Larger buffer
        .build()?;

    println!("Created terminal with custom configuration:");
    println!("  Size: 100x30");
    println!("  Timeout: 10 seconds");
    println!("  Poll interval: 50ms");
    println!("  Buffer size: 8KB");

    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("Custom configuration test");

    harness.spawn(cmd)?;
    std::thread::sleep(Duration::from_millis(100));
    harness.update_state()?;

    let (width, height) = harness.state().size();
    println!("\nVerified terminal size: {}x{}", width, height);
    assert_eq!(width, 100);
    assert_eq!(height, 30);

    let contents = harness.screen_contents();
    assert!(contents.contains("Custom configuration test"));
    println!("✓ Custom terminal configuration working correctly");

    println!();
    Ok(())
}

/// Example 5: Cursor position tracking
///
/// Demonstrates:
/// - Tracking cursor movements
/// - Using escape sequences
/// - Cursor position validation
fn example_5_cursor_tracking() -> Result<()> {
    println!("--- Example 5: Cursor Position Tracking ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use a command that moves the cursor explicitly
    // ESC[5;10H moves cursor to row 5, column 10 (1-based)
    // Note: We use printf to send escape sequences
    let mut cmd = CommandBuilder::new("printf");
    cmd.arg("\\033[5;10HX"); // Move to (5,10) and print X

    harness.spawn(cmd)?;
    println!("Spawned: printf with cursor movement escape sequence");
    println!("Sent: ESC[5;10H (move to row 5, col 10)");

    std::thread::sleep(Duration::from_millis(100));
    harness.update_state()?;

    // Get cursor position (ratatui_testlib uses 0-based indexing)
    let (row, col) = harness.cursor_position();
    println!("\nCursor position after movement:");
    println!("  Row: {} (0-based)", row);
    println!("  Col: {} (0-based)", col);

    // The cursor should be near row 4 (5-1), col 10 (after printing X)
    println!("\nNote: Escape sequences use 1-based indexing,");
    println!("      but ratatui_testlib returns 0-based positions.");

    // Verify the character was placed at the cursor position
    let contents = harness.screen_contents();
    println!("\nScreen contents (showing rows 3-6):");
    println!("┌{:─<80}┐", "");
    for (i, line) in contents.lines().enumerate().skip(3).take(4) {
        println!("│{:2} {:<77}│", i, line);
    }
    println!("└{:─<80}┘", "");

    println!("✓ Cursor tracking demonstration complete");

    println!();
    Ok(())
}
