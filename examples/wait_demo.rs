//! Demonstration of wait condition patterns in mimic.
//!
//! This example shows various wait patterns for testing TUI applications:
//! - Waiting for text to appear
//! - Waiting for cursor movements
//! - Using custom predicates
//! - Handling timeouts gracefully
//! - Chaining multiple wait operations
//!
//! Run with: cargo run --example wait_demo

use portable_pty::CommandBuilder;
use std::time::Duration;
use mimic::{Result, TermTestError, TuiTestHarness};

fn main() -> Result<()> {
    println!("=== Term-Test Wait Conditions Demo ===\n");

    demo_basic_text_wait()?;
    demo_multiline_wait()?;
    demo_cursor_wait()?;
    demo_custom_predicates()?;
    demo_timeout_handling()?;
    demo_sequential_waits()?;
    demo_builder_pattern()?;

    println!("\n=== All demos completed successfully! ===");
    Ok(())
}

/// Demonstrates basic text waiting
fn demo_basic_text_wait() -> Result<()> {
    println!("1. Basic Text Wait");
    println!("   Spawning 'echo Hello World' and waiting for output...");

    let mut harness = TuiTestHarness::new(80, 24)?
        .with_timeout(Duration::from_secs(2))
        .with_poll_interval(Duration::from_millis(50));

    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("Hello World");
    harness.spawn(cmd)?;

    // Wait for text to appear
    harness.wait_for_text("Hello")?;
    println!("   ✓ Found 'Hello'");

    harness.wait_for_text("World")?;
    println!("   ✓ Found 'World'");

    let contents = harness.screen_contents();
    println!("   Screen contents: {}", contents.trim());
    println!();

    Ok(())
}

/// Demonstrates waiting for multiline output
fn demo_multiline_wait() -> Result<()> {
    println!("2. Multiline Output Wait");
    println!("   Spawning command that produces multiple lines...");

    let mut harness = TuiTestHarness::new(80, 24)?
        .with_timeout(Duration::from_secs(3))
        .with_poll_interval(Duration::from_millis(50));

    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("echo 'Line 1: Starting'; echo 'Line 2: Processing'; echo 'Line 3: Complete'");
    harness.spawn(cmd)?;

    // Wait for each line in sequence
    harness.wait_for_text("Line 1")?;
    println!("   ✓ Line 1 appeared");

    harness.wait_for_text("Line 2")?;
    println!("   ✓ Line 2 appeared");

    harness.wait_for_text("Line 3")?;
    println!("   ✓ Line 3 appeared");

    // Verify all content is present
    let contents = harness.screen_contents();
    assert!(contents.contains("Starting"));
    assert!(contents.contains("Processing"));
    assert!(contents.contains("Complete"));
    println!("   ✓ All lines verified");
    println!();

    Ok(())
}

/// Demonstrates cursor position waiting
fn demo_cursor_wait() -> Result<()> {
    println!("3. Cursor Position Wait");
    println!("   Feeding cursor movement sequences and waiting...");

    let mut harness = TuiTestHarness::new(80, 24)?
        .with_timeout(Duration::from_secs(1));

    // Feed escape sequence to move cursor to row 10, col 20 (1-based)
    harness.state_mut().feed(b"\x1b[10;20H");
    harness.update_state()?;

    // Wait for cursor to reach position (0-based coordinates)
    harness.wait_for_cursor((9, 19))?;
    println!("   ✓ Cursor reached position (9, 19)");

    // Verify cursor position
    let (row, col) = harness.cursor_position();
    println!("   Current cursor: row={}, col={}", row, col);
    println!();

    Ok(())
}

/// Demonstrates custom predicates for complex conditions
fn demo_custom_predicates() -> Result<()> {
    println!("4. Custom Predicates");
    println!("   Using custom predicates for complex conditions...");

    let mut harness = TuiTestHarness::new(80, 24)?
        .with_timeout(Duration::from_secs(2));

    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("Progress: 75% [Status: OK]");
    harness.spawn(cmd)?;

    // Wait for both "Progress" and a percentage
    harness.wait_for(|state| {
        let contents = state.contents();
        contents.contains("Progress") && contents.contains("%")
    })?;
    println!("   ✓ Found progress indicator");

    // Wait for status indicator
    harness.wait_for(|state| {
        state.contents().contains("Status: OK")
    })?;
    println!("   ✓ Found status indicator");

    // Count digits in output
    let digit_count = harness.screen_contents()
        .chars()
        .filter(|c| c.is_numeric())
        .count();
    println!("   Found {} digits in output", digit_count);
    println!();

    Ok(())
}

/// Demonstrates timeout handling
fn demo_timeout_handling() -> Result<()> {
    println!("5. Timeout Handling");
    println!("   Demonstrating graceful timeout handling...");

    let mut harness = TuiTestHarness::new(80, 24)?
        .with_timeout(Duration::from_millis(500))
        .with_poll_interval(Duration::from_millis(50));

    let mut cmd = CommandBuilder::new("sleep");
    cmd.arg("5");
    harness.spawn(cmd)?;

    // This will timeout
    match harness.wait_for_text("text_that_never_appears") {
        Ok(_) => {
            println!("   ✗ Unexpected success");
        }
        Err(TermTestError::Timeout { timeout_ms }) => {
            println!("   ✓ Timeout occurred as expected ({}ms)", timeout_ms);
            println!("   ✓ Error message provides debugging context");
        }
        Err(e) => {
            println!("   ✗ Unexpected error: {}", e);
        }
    }

    // Demonstrate custom timeout override
    let mut harness2 = TuiTestHarness::new(80, 24)?;
    let mut cmd2 = CommandBuilder::new("echo");
    cmd2.arg("quick");
    harness2.spawn(cmd2)?;

    // Use shorter timeout for fast operation
    harness2.wait_for_text_timeout("quick", Duration::from_millis(800))?;
    println!("   ✓ Custom timeout worked");
    println!();

    Ok(())
}

/// Demonstrates sequential wait operations
fn demo_sequential_waits() -> Result<()> {
    println!("6. Sequential Wait Operations");
    println!("   Chaining multiple wait operations...");

    let mut harness = TuiTestHarness::new(80, 24)?
        .with_timeout(Duration::from_secs(3))
        .with_poll_interval(Duration::from_millis(50));

    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("echo 'Initializing...'; sleep 0.1; echo 'Loading...'; sleep 0.1; echo 'Ready!'");
    harness.spawn(cmd)?;

    // Sequential waits for application lifecycle
    println!("   Waiting for initialization...");
    harness.wait_for_text("Initializing")?;
    println!("   ✓ Initialized");

    println!("   Waiting for loading...");
    harness.wait_for_text("Loading")?;
    println!("   ✓ Loading started");

    println!("   Waiting for ready state...");
    harness.wait_for_text("Ready")?;
    println!("   ✓ Application ready");

    let contents = harness.screen_contents();
    println!("   Final screen:\n{}", indent_text(&contents.trim(), 5));
    println!();

    Ok(())
}

/// Demonstrates builder pattern with wait conditions
fn demo_builder_pattern() -> Result<()> {
    println!("7. Builder Pattern Configuration");
    println!("   Using builder pattern for custom configuration...");

    let mut harness = TuiTestHarness::builder()
        .with_size(100, 30)
        .with_timeout(Duration::from_secs(3))
        .with_poll_interval(Duration::from_millis(25))
        .with_buffer_size(8192)
        .build()?;

    println!("   ✓ Created harness with custom settings:");
    println!("     - Size: 100x30");
    println!("     - Timeout: 3 seconds");
    println!("     - Poll interval: 25ms");
    println!("     - Buffer size: 8KB");

    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("Custom configuration test");
    harness.spawn(cmd)?;

    harness.wait_for_text("configuration")?;
    println!("   ✓ Wait operation succeeded with custom settings");
    println!();

    Ok(())
}

/// Helper function to indent text
fn indent_text(text: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{}{}", indent, line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wait_patterns() -> Result<()> {
        // Run all demos as tests
        demo_basic_text_wait()?;
        demo_multiline_wait()?;
        demo_cursor_wait()?;
        demo_custom_predicates()?;
        demo_timeout_handling()?;
        demo_sequential_waits()?;
        demo_builder_pattern()?;
        Ok(())
    }
}
