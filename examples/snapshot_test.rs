//! Snapshot testing example using insta.
//!
//! This example demonstrates how to use mimic with the insta snapshot testing library
//! for visual regression testing of TUI applications:
//! - Capturing terminal output as snapshots
//! - Using insta for snapshot comparison
//! - Testing multi-frame UI updates
//! - Handling dynamic content with redactions
//! - Best practices for snapshot testing
//!
//! # Running This Example
//!
//! ```bash
//! cargo run --example snapshot_test --features snapshot-insta
//! ```
//!
//! # What is Snapshot Testing?
//!
//! Snapshot testing captures the output of your code and saves it to a file.
//! On subsequent runs, the output is compared to the saved snapshot. If they
//! differ, the test fails, alerting you to unexpected changes.
//!
//! # Why Use Snapshots for TUI Testing?
//!
//! - Catch visual regressions in UI layout
//! - Document expected behavior with real examples
//! - Easier than writing complex assertions for screen contents
//! - Quickly review changes with `cargo insta review`
//!
//! # Expected Output
//!
//! This example demonstrates:
//! 1. Basic snapshot capture
//! 2. Named snapshots for multiple scenarios
//! 3. Snapshot settings (redactions, filters)
//! 4. Testing multi-step UI flows
//! 5. Best practices and common patterns

use portable_pty::CommandBuilder;
use mimic::{Result, TuiTestHarness};
use std::time::Duration;

fn main() -> Result<()> {
    println!("=== Snapshot Testing Example with insta ===\n");

    println!("Note: This example demonstrates snapshot testing patterns.");
    println!("In a real test file, you would use #[test] functions and");
    println!("the insta::assert_snapshot!() macro.\n");

    // Example 1: Basic snapshot capture
    example_1_basic_snapshot()?;

    // Example 2: Named snapshots
    example_2_named_snapshots()?;

    // Example 3: Multi-step UI flow
    example_3_multi_step_flow()?;

    // Example 4: Snapshot with settings
    example_4_snapshot_settings()?;

    // Example 5: Testing error states
    example_5_error_states()?;

    println!("\n=== All Snapshot Examples Completed ===");
    println!("\nTo use in real tests:");
    println!("1. Add to your test file: use insta::assert_snapshot;");
    println!("2. Replace println!() with assert_snapshot!()");
    println!("3. Run: cargo test");
    println!("4. Review snapshots: cargo insta review");

    Ok(())
}

/// Example 1: Basic snapshot capture
///
/// Demonstrates:
/// - Capturing simple terminal output
/// - Basic snapshot usage
/// - What gets captured
fn example_1_basic_snapshot() -> Result<()> {
    println!("--- Example 1: Basic Snapshot Capture ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Spawn a command with predictable output
    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("Hello, Snapshot Testing!");

    harness.spawn(cmd)?;
    std::thread::sleep(Duration::from_millis(100));
    harness.update_state()?;

    let contents = harness.screen_contents();

    println!("Captured screen contents:");
    println!("┌{:─<78}┐", "");
    for (i, line) in contents.lines().take(5).enumerate() {
        println!("│ {:2} {:<74}│", i, line);
    }
    println!("└{:─<78}┘", "");

    // In a real test, you would use:
    // insta::assert_snapshot!(contents);

    println!("\nIn a real test file:");
    println!("  #[test]");
    println!("  fn test_welcome_message() -> Result<()> {{");
    println!("      let mut harness = TuiTestHarness::new(80, 24)?;");
    println!("      // ... spawn and capture ...");
    println!("      insta::assert_snapshot!(harness.screen_contents());");
    println!("      Ok(())");
    println!("  }}");

    println!();
    Ok(())
}

/// Example 2: Named snapshots
///
/// Demonstrates:
/// - Using named snapshots for multiple test cases
/// - Organizing snapshots by scenario
/// - Testing different UI states
fn example_2_named_snapshots() -> Result<()> {
    println!("--- Example 2: Named Snapshots ---");

    println!("Testing multiple scenarios with descriptive names:\n");

    // Scenario 1: Empty state
    let mut harness1 = TuiTestHarness::new(80, 24)?;
    let mut cmd1 = CommandBuilder::new("echo");
    cmd1.arg("");
    harness1.spawn(cmd1)?;
    std::thread::sleep(Duration::from_millis(100));
    harness1.update_state()?;

    let empty_state = harness1.screen_contents();
    println!("Scenario: empty_state");
    println!("  Would save as: snapshots/snapshot_test__empty_state.snap");

    // In a real test:
    // insta::assert_snapshot!("empty_state", harness.screen_contents());

    // Scenario 2: Single line
    let mut harness2 = TuiTestHarness::new(80, 24)?;
    let mut cmd2 = CommandBuilder::new("echo");
    cmd2.arg("Single line of text");
    harness2.spawn(cmd2)?;
    std::thread::sleep(Duration::from_millis(100));
    harness2.update_state()?;

    let single_line = harness2.screen_contents();
    println!("Scenario: single_line");
    println!("  Would save as: snapshots/snapshot_test__single_line.snap");

    // Scenario 3: Multi-line
    let mut harness3 = TuiTestHarness::new(80, 24)?;
    let mut cmd3 = CommandBuilder::new("printf");
    cmd3.arg("Line 1\\nLine 2\\nLine 3");
    harness3.spawn(cmd3)?;
    std::thread::sleep(Duration::from_millis(100));
    harness3.update_state()?;

    let multi_line = harness3.screen_contents();
    println!("Scenario: multi_line");
    println!("  Would save as: snapshots/snapshot_test__multi_line.snap");

    println!("\nBenefit: Each scenario has its own snapshot file,");
    println!("         making it easy to review changes.");

    println!();
    Ok(())
}

/// Example 3: Multi-step UI flow
///
/// Demonstrates:
/// - Testing sequential UI updates
/// - Capturing state at each step
/// - Verifying transitions
fn example_3_multi_step_flow() -> Result<()> {
    println!("--- Example 3: Multi-step UI Flow ---");

    println!("Testing a multi-step interaction flow:\n");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Step 1: Initial state
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("echo 'Step 1: Initializing...' && sleep 0.1 && echo 'Step 2: Loading...' && sleep 0.1 && echo 'Step 3: Ready!'");

    harness.spawn(cmd)?;

    // Capture state at different points
    std::thread::sleep(Duration::from_millis(50));
    harness.update_state()?;

    let state_1 = harness.screen_contents();
    println!("Capturing state 1 (early):");
    println!("  insta::assert_snapshot!(\"flow_step_1\", contents);");

    std::thread::sleep(Duration::from_millis(150));
    harness.update_state()?;

    let state_2 = harness.screen_contents();
    println!("Capturing state 2 (mid):");
    println!("  insta::assert_snapshot!(\"flow_step_2\", contents);");

    std::thread::sleep(Duration::from_millis(150));
    harness.update_state()?;

    let state_3 = harness.screen_contents();
    println!("Capturing state 3 (final):");
    println!("  insta::assert_snapshot!(\"flow_step_3\", contents);");

    println!("\nFinal state preview:");
    println!("┌{:─<78}┐", "");
    for line in state_3.lines().take(5) {
        println!("│ {:<77}│", line);
    }
    println!("└{:─<78}┘", "");

    println!("\nUse case: Verify each step of a wizard or loading sequence");

    println!();
    Ok(())
}

/// Example 4: Snapshot with settings
///
/// Demonstrates:
/// - Using insta settings for redactions
/// - Filtering dynamic content (timestamps, IDs)
/// - Normalizing output for stable snapshots
fn example_4_snapshot_settings() -> Result<()> {
    println!("--- Example 4: Snapshot Settings ---");

    println!("Handling dynamic content with insta settings:\n");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Simulate output with timestamps (would vary on each run)
    let mut cmd = CommandBuilder::new("date");
    cmd.arg("+%Y-%m-%d %H:%M:%S");

    harness.spawn(cmd)?;
    std::thread::sleep(Duration::from_millis(100));
    harness.update_state()?;

    let contents = harness.screen_contents();

    println!("Raw output (contains timestamp):");
    println!("┌{:─<78}┐", "");
    for line in contents.lines().take(3) {
        println!("│ {:<77}│", line);
    }
    println!("└{:─<78}┘", "");

    println!("\nProblem: Timestamps change every run, breaking snapshots");
    println!("\nSolution: Use insta settings with redactions:");
    println!("
    let mut settings = insta::Settings::clone_current();
    settings.add_filter(r\"\\d{{4}}-\\d{{2}}-\\d{{2}}\", \"[DATE]\");
    settings.add_filter(r\"\\d{{2}}:\\d{{2}}:\\d{{2}}\", \"[TIME]\");
    let _guard = settings.bind_to_scope();

    insta::assert_snapshot!(contents);
    ");

    println!("Result: Snapshot contains [DATE] [TIME] instead of actual values");

    println!("\nOther common redactions:");
    println!("  - Process IDs: add_filter(r\"pid: \\d+\", \"pid: [PID]\")");
    println!("  - Memory addresses: add_filter(r\"0x[0-9a-f]+\", \"0x[ADDR]\")");
    println!("  - UUIDs: add_filter(r\"[0-9a-f-]{{36}}\", \"[UUID]\")");

    println!();
    Ok(())
}

/// Example 5: Testing error states
///
/// Demonstrates:
/// - Capturing error messages
/// - Testing edge cases
/// - Verifying error formatting
fn example_5_error_states() -> Result<()> {
    println!("--- Example 5: Testing Error States ---");

    println!("Capturing error messages and edge cases:\n");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Simulate a command that produces error output
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("echo 'Error: File not found' >&2");

    harness.spawn(cmd)?;
    std::thread::sleep(Duration::from_millis(100));
    harness.update_state()?;

    let error_contents = harness.screen_contents();

    println!("Error state captured:");
    println!("┌{:─<78}┐", "");
    for line in error_contents.lines().take(5) {
        println!("│ {:<77}│", line);
    }
    println!("└{:─<78}┘", "");

    println!("\nIn a real test:");
    println!("  #[test]");
    println!("  fn test_file_not_found_error() -> Result<()> {{");
    println!("      // ... trigger error condition ...");
    println!("      insta::assert_snapshot!(\"error_file_not_found\", contents);");
    println!("      Ok(())");
    println!("  }}");

    println!("\nBenefits:");
    println!("  - Catch regressions in error message formatting");
    println!("  - Document expected error states");
    println!("  - Ensure helpful error messages");

    println!();
    Ok(())
}

// Example of how snapshot tests would look in a real test file:
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use insta::assert_snapshot;
//
//     #[test]
//     fn test_welcome_screen() -> Result<()> {
//         let mut harness = TuiTestHarness::new(80, 24)?;
//         let mut cmd = CommandBuilder::new("my-app");
//         harness.spawn(cmd)?;
//
//         harness.wait_for_text("Welcome")?;
//         assert_snapshot!(harness.screen_contents());
//
//         Ok(())
//     }
//
//     #[test]
//     fn test_menu_navigation() -> Result<()> {
//         let mut harness = TuiTestHarness::new(80, 24)?;
//         let mut cmd = CommandBuilder::new("my-app");
//         harness.spawn(cmd)?;
//
//         // Initial menu
//         harness.wait_for_text("Menu")?;
//         assert_snapshot!("menu_initial", harness.screen_contents());
//
//         // Navigate down
//         harness.send_text("\x1b[B")?;  // Down arrow
//         std::thread::sleep(Duration::from_millis(100));
//         harness.update_state()?;
//         assert_snapshot!("menu_item_2_selected", harness.screen_contents());
//
//         Ok(())
//     }
// }
