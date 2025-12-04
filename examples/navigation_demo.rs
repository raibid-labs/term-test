//! Navigation testing demonstration.
//!
//! This example demonstrates the navigation testing helpers for keyboard-driven
//! navigation systems including:
//! - Vimium-style hint mode
//! - Focus tracking and tab navigation
//! - Mode detection and transitions
//!
//! # Running This Example
//!
//! ```bash
//! cargo run --example navigation_demo
//! ```
//!
//! # What This Demonstrates
//!
//! - Detecting navigation modes from screen content
//! - Finding and activating hint labels
//! - Focus tracking and navigation
//! - Mode transitions

use std::time::Duration;

use portable_pty::CommandBuilder;
use ratatui_testlib::{
    navigation::{NavMode, NavigationTestExt},
    Result, TuiTestHarness,
};

fn main() -> Result<()> {
    println!("=== Navigation Testing Demo ===\n");

    // Example 1: Mode detection
    example_1_mode_detection()?;

    // Example 2: Hint label detection
    example_2_hint_detection()?;

    // Example 3: Focus navigation
    example_3_focus_navigation()?;

    println!("\n=== All Navigation Demos Completed Successfully ===");

    Ok(())
}

/// Example 1: Detecting navigation modes from screen content.
///
/// Demonstrates:
/// - Default mode detection (Normal)
/// - Detecting mode indicators in status lines
/// - Using current_mode() method
fn example_1_mode_detection() -> Result<()> {
    println!("--- Example 1: Mode Detection ---");

    // Create a simple script that displays mode indicators
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c");
    cmd.arg(
        r#"
        echo "Welcome to the app"
        echo ""
        echo "-- NORMAL --"
        sleep 0.5
        "#,
    );
    harness.spawn(cmd)?;

    // Wait for output to appear
    // We use wait_for_text instead of sleep ensures the state is updated
    harness.wait_for_text("-- NORMAL --")?;

    // Detect mode
    let mode = harness.current_mode();
    println!("Detected mode: {}", mode.as_str());

    let contents = harness.screen_contents();
    println!("\nScreen output:");
    println!("┌{:─<80}┐", "");
    for line in contents.lines().take(5) {
        println!("│{:<80}│", line);
    }
    println!("└{:─<80}┘", "");

    println!("✓ Mode detection works\n");
    Ok(())
}

/// Example 2: Detecting hint labels on screen.
///
/// Demonstrates:
/// - Finding hint labels like [a], [b], [aa]
/// - Extracting hint positions
/// - Detecting hint element types
fn example_2_hint_detection() -> Result<()> {
    println!("--- Example 2: Hint Label Detection ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Create a mock hint mode display
    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c");
    cmd.arg(
        r#"
        echo "Links:"
        echo "  [a] https://github.com"
        echo "  [b] https://rust-lang.org"
        echo "  [c] /path/to/file.txt"
        echo "  [d] user@example.com"
        sleep 1
        "#,
    );
    harness.spawn(cmd)?;

    // Wait for hints to appear
    harness.wait_for_text("[d]")?;

    // Find all hints
    let hints = harness.visible_hints();
    println!("Found {} hints:", hints.len());

    for hint in &hints {
        println!(
            "  - Hint '{}' at ({}, {}) - Type: {:?}",
            hint.label, hint.position.0, hint.position.1, hint.element_type
        );
    }

    let contents = harness.screen_contents();
    println!("\nScreen output:");
    println!("┌{:─<80}┐", "");
    for line in contents.lines().take(6) {
        println!("│{:<80}│", line);
    }
    println!("└{:─<80}┘", "");

    println!("✓ Found {} hint labels\n", hints.len());
    Ok(())
}

/// Example 3: Focus navigation with Tab/Shift+Tab.
///
/// Demonstrates:
/// - Using focus_next() and focus_prev()
/// - Detecting focused elements
/// - Tab navigation patterns
fn example_3_focus_navigation() -> Result<()> {
    println!("--- Example 3: Focus Navigation ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use cat to echo our input
    let cmd = CommandBuilder::new("cat");
    harness.spawn(cmd)?;

    println!("Spawned: cat");
    std::thread::sleep(Duration::from_millis(100));

    // Simulate tab navigation
    println!("Sending Tab key (focus_next)...");
    harness.focus_next()?;

    std::thread::sleep(Duration::from_millis(100));

    // Try to detect focused element
    if let Some(focus) = harness.focused_element() {
        println!(
            "Focused element: {} at ({}, {})",
            focus.element_type, focus.bounds.x, focus.bounds.y
        );
    } else {
        println!("No focused element detected (expected for cat)");
    }

    // Send Shift+Tab (focus_prev)
    println!("Sending Shift+Tab (focus_prev)...");
    harness.focus_prev()?;

    std::thread::sleep(Duration::from_millis(100));

    println!("✓ Focus navigation methods work\n");
    Ok(())
}

/// Example 4: Testing hint activation.
///
/// Demonstrates:
/// - Activating hints by label
/// - Simulating Vimium-style workflows
#[allow(dead_code)]
fn example_4_hint_activation() -> Result<()> {
    println!("--- Example 4: Hint Activation ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // This would require a real TUI app with hint mode support
    // For demonstration, we'll show the API usage

    // Simulate entering hint mode
    println!("Entering hint mode...");
    // harness.enter_hint_mode()?; // Would send 'f' key

    // Find hints
    let hints = harness.visible_hints();
    if let Some(hint) = hints.first() {
        println!("Activating hint: {}", hint.label);
        harness.activate_hint(&hint.label)?;
    }

    println!("✓ Hint activation API demonstrated\n");
    Ok(())
}

/// Example 5: Mode transitions.
///
/// Demonstrates:
/// - Entering different modes
/// - Waiting for mode transitions
/// - Exiting to normal mode
#[allow(dead_code)]
fn example_5_mode_transitions() -> Result<()> {
    println!("--- Example 5: Mode Transitions ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // This would require a modal TUI app
    println!("Testing mode transitions...");

    // Enter hint mode
    println!("Entering hint mode...");
    // harness.enter_hint_mode()?;

    // Wait for mode change
    // harness.wait_for_mode(NavMode::Hints, Duration::from_secs(1))?;

    // Exit to normal
    println!("Exiting to normal mode...");
    harness.exit_to_normal()?;

    // Verify we're back in normal mode
    let mode = harness.current_mode();
    assert_eq!(mode, NavMode::Normal);

    println!("✓ Mode transitions work\n");
    Ok(())
}

/// Example 6: Testing with real vim-style application.
///
/// Demonstrates:
/// - Testing a real modal application
/// - Combining navigation helpers
#[allow(dead_code)]
fn example_6_vim_style_app() -> Result<()> {
    println!("--- Example 6: Vim-Style Application ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Spawn a vim-like application (if available)
    // For this demo, we'll use a simple mock

    println!("Testing vim-style navigation...");

    // Start in normal mode
    assert_eq!(harness.current_mode(), NavMode::Normal);

    // Enter insert mode (usually 'i' key)
    // harness.send_key(KeyCode::Char('i'))?;
    // harness.wait_for_mode(NavMode::Insert, Duration::from_secs(1))?;

    // Type some text
    // harness.send_keys("Hello, World!")?;

    // Exit to normal mode
    harness.exit_to_normal()?;

    println!("✓ Vim-style navigation patterns demonstrated\n");
    Ok(())
}
