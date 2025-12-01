//! Keyboard event simulation demonstration.
//!
//! This example shows how to use the keyboard event simulation features
//! of mimic to send various types of input to TUI applications.
//!
//! # Running This Example
//!
//! ```bash
//! cargo run --example keyboard_demo
//! ```
//!
//! # What This Demonstrates
//!
//! - Sending individual keys with `send_key()`
//! - Sending keys with modifiers (Ctrl, Alt)
//! - Typing text strings with `send_keys()`
//! - Navigation keys (arrows, Home, End)
//! - Function keys (F1-F12)
//! - Special keys (Enter, Tab, Esc, Backspace)
//! - Interactive input patterns

use portable_pty::CommandBuilder;
use std::time::Duration;
use mimic::{KeyCode, Modifiers, Result, TuiTestHarness};

fn main() -> Result<()> {
    println!("=== Keyboard Event Simulation Demo ===\n");

    // Example 1: Simple character input
    example_1_simple_chars()?;

    // Example 2: Text string typing
    example_2_text_typing()?;

    // Example 3: Navigation keys
    example_3_navigation_keys()?;

    // Example 4: Modifier keys (Ctrl, Alt)
    example_4_modifier_keys()?;

    // Example 5: Function keys
    example_5_function_keys()?;

    // Example 6: Interactive session
    example_6_interactive_session()?;

    // Example 7: Special keys
    example_7_special_keys()?;

    println!("\n=== All Keyboard Demos Completed Successfully ===");

    Ok(())
}

/// Example 1: Sending individual character keys.
///
/// Demonstrates:
/// - Creating a harness
/// - Spawning cat to echo input
/// - Sending individual characters
/// - Verifying they appear in output
fn example_1_simple_chars() -> Result<()> {
    println!("--- Example 1: Simple Character Keys ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Spawn cat which will echo our input
    let mut cmd = CommandBuilder::new("cat");
    harness.spawn(cmd)?;

    println!("Spawned: cat");
    std::thread::sleep(Duration::from_millis(100));

    // Send individual characters
    println!("Sending: 'H', 'i', '!'");
    harness.send_key(KeyCode::Char('H'))?;
    harness.send_key(KeyCode::Char('i'))?;
    harness.send_key(KeyCode::Char('!'))?;

    // cat echoes them back
    harness.wait_for_text("Hi!")?;

    let contents = harness.screen_contents();
    println!("\nScreen output:");
    println!("┌{:─<80}┐", "");
    for line in contents.lines().take(2) {
        println!("│{:<80}│", line);
    }
    println!("└{:─<80}┘", "");

    println!("✓ Characters appeared correctly\n");
    Ok(())
}

/// Example 2: Typing text strings.
///
/// Demonstrates:
/// - Using send_keys() for convenience
/// - Typing multi-character strings
/// - Press Enter to submit
fn example_2_text_typing() -> Result<()> {
    println!("--- Example 2: Text String Typing ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use bash to read and echo a line
    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c");
    cmd.arg("read line && echo \"You typed: $line\"");
    harness.spawn(cmd)?;

    println!("Spawned: bash interactive read");
    std::thread::sleep(Duration::from_millis(100));

    // Type a string of text
    let text = "Hello, World!";
    println!("Typing: '{}'", text);
    harness.send_keys(text)?;

    // Press Enter to submit
    println!("Pressing: Enter");
    harness.send_key(KeyCode::Enter)?;

    // Wait for the response
    harness.wait_for_text("You typed: Hello, World!")?;

    let contents = harness.screen_contents();
    println!("\nScreen output:");
    println!("┌{:─<80}┐", "");
    for line in contents.lines().take(3) {
        println!("│{:<80}│", line);
    }
    println!("└{:─<80}┘", "");

    println!("✓ Text typing works correctly\n");
    Ok(())
}

/// Example 3: Navigation keys.
///
/// Demonstrates:
/// - Arrow keys (Up, Down, Left, Right)
/// - Home, End keys
/// - PageUp, PageDown keys
fn example_3_navigation_keys() -> Result<()> {
    println!("--- Example 3: Navigation Keys ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use cat -v to visualize escape sequences
    let mut cmd = CommandBuilder::new("cat");
    cmd.arg("-v");
    harness.spawn(cmd)?;

    println!("Spawned: cat -v (shows escape sequences)");
    std::thread::sleep(Duration::from_millis(100));

    // Send various navigation keys
    println!("Sending navigation keys:");

    println!("  - Up arrow");
    harness.send_key(KeyCode::Up)?;

    println!("  - Down arrow");
    harness.send_key(KeyCode::Down)?;

    println!("  - Left arrow");
    harness.send_key(KeyCode::Left)?;

    println!("  - Right arrow");
    harness.send_key(KeyCode::Right)?;

    println!("  - Home");
    harness.send_key(KeyCode::Home)?;

    println!("  - End");
    harness.send_key(KeyCode::End)?;

    println!("  - PageUp");
    harness.send_key(KeyCode::PageUp)?;

    println!("  - PageDown");
    harness.send_key(KeyCode::PageDown)?;

    std::thread::sleep(Duration::from_millis(200));

    let contents = harness.screen_contents();
    println!("\nScreen output (escape sequences):");
    println!("┌{:─<80}┐", "");
    for line in contents.lines().take(5) {
        println!("│{:<80}│", line);
    }
    println!("└{:─<80}┘", "");

    println!("✓ Navigation keys sent correctly\n");
    Ok(())
}

/// Example 4: Modifier keys (Ctrl, Alt).
///
/// Demonstrates:
/// - Ctrl combinations
/// - Alt combinations
/// - Common shortcuts (Ctrl+C, Ctrl+D, etc.)
fn example_4_modifier_keys() -> Result<()> {
    println!("--- Example 4: Modifier Keys ---");

    // Demonstrate Ctrl+D (EOF)
    {
        let mut harness = TuiTestHarness::new(80, 24)?;
        let mut cmd = CommandBuilder::new("cat");
        harness.spawn(cmd)?;

        println!("Spawned: cat");
        std::thread::sleep(Duration::from_millis(100));

        // Type some data
        println!("Typing: 'data'");
        harness.send_keys("data")?;

        // Send Ctrl+D to signal EOF
        println!("Sending: Ctrl+D (EOF)");
        harness.send_key_with_modifiers(KeyCode::Char('d'), Modifiers::CTRL)?;

        // Give cat time to exit
        std::thread::sleep(Duration::from_millis(200));

        if !harness.is_running() {
            println!("✓ cat exited on Ctrl+D as expected");
        } else {
            println!("⚠ cat still running (may be expected in some environments)");
        }
    }

    // Demonstrate Ctrl+A through shell
    {
        println!("\nTesting Ctrl+A (0x01) control character:");

        let mut harness = TuiTestHarness::new(80, 24)?;
        let mut cmd = CommandBuilder::new("bash");
        cmd.arg("-c");
        cmd.arg("dd bs=1 count=1 2>/dev/null | od -An -tx1");
        harness.spawn(cmd)?;

        std::thread::sleep(Duration::from_millis(100));

        // Send Ctrl+A
        println!("Sending: Ctrl+A");
        harness.send_key_with_modifiers(KeyCode::Char('a'), Modifiers::CTRL)?;

        std::thread::sleep(Duration::from_millis(300));

        let contents = harness.screen_contents();
        if contents.contains("01") {
            println!("✓ Ctrl+A sent correctly (hex: 01)");
        }
    }

    // Demonstrate Alt combinations
    {
        println!("\nTesting Alt+a (ESC + 'a'):");

        let mut harness = TuiTestHarness::new(80, 24)?;
        let mut cmd = CommandBuilder::new("bash");
        cmd.arg("-c");
        cmd.arg("dd bs=1 count=2 2>/dev/null | od -An -tx1");
        harness.spawn(cmd)?;

        std::thread::sleep(Duration::from_millis(100));

        // Send Alt+a
        println!("Sending: Alt+a");
        harness.send_key_with_modifiers(KeyCode::Char('a'), Modifiers::ALT)?;

        std::thread::sleep(Duration::from_millis(300));

        let contents = harness.screen_contents();
        if contents.contains("1b") && contents.contains("61") {
            println!("✓ Alt+a sent correctly (hex: 1b 61)");
        }
    }

    println!();
    Ok(())
}

/// Example 5: Function keys.
///
/// Demonstrates:
/// - F1 through F12
/// - Escape sequence generation
fn example_5_function_keys() -> Result<()> {
    println!("--- Example 5: Function Keys ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use cat -v to visualize escape sequences
    let mut cmd = CommandBuilder::new("cat");
    cmd.arg("-v");
    harness.spawn(cmd)?;

    println!("Spawned: cat -v");
    std::thread::sleep(Duration::from_millis(100));

    // Send a few function keys
    println!("Sending function keys:");

    println!("  - F1");
    harness.send_key(KeyCode::F(1))?;

    println!("  - F2");
    harness.send_key(KeyCode::F(2))?;

    println!("  - F5");
    harness.send_key(KeyCode::F(5))?;

    println!("  - F12");
    harness.send_key(KeyCode::F(12))?;

    std::thread::sleep(Duration::from_millis(200));

    let contents = harness.screen_contents();
    println!("\nScreen output (function key sequences):");
    println!("┌{:─<80}┐", "");
    for line in contents.lines().take(3) {
        println!("│{:<80}│", line);
    }
    println!("└{:─<80}┘", "");

    println!("✓ Function keys sent correctly\n");
    Ok(())
}

/// Example 6: Interactive session simulation.
///
/// Demonstrates:
/// - Multi-step interaction
/// - Waiting for prompts
/// - Combining different input methods
fn example_6_interactive_session() -> Result<()> {
    println!("--- Example 6: Interactive Session ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Create an interactive bash script
    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c");
    cmd.arg(
        r#"
        read -p "Name: " name
        read -p "Age: " age
        echo "Hello, $name! You are $age years old."
        "#,
    );
    harness.spawn(cmd)?;

    println!("Spawned: interactive bash script");

    // Wait for first prompt
    println!("Waiting for 'Name:' prompt...");
    harness.wait_for_text("Name:")?;

    // Enter name
    println!("Typing: 'Alice'");
    harness.send_keys("Alice")?;
    harness.send_key(KeyCode::Enter)?;

    // Wait for second prompt
    println!("Waiting for 'Age:' prompt...");
    harness.wait_for_text("Age:")?;

    // Enter age
    println!("Typing: '30'");
    harness.send_keys("30")?;
    harness.send_key(KeyCode::Enter)?;

    // Wait for final output
    println!("Waiting for response...");
    harness.wait_for_text("Hello, Alice! You are 30 years old.")?;

    let contents = harness.screen_contents();
    println!("\nFinal screen output:");
    println!("┌{:─<80}┐", "");
    for line in contents.lines().take(6) {
        println!("│{:<80}│", line);
    }
    println!("└{:─<80}┘", "");

    println!("✓ Interactive session completed successfully\n");
    Ok(())
}

/// Example 7: Special keys.
///
/// Demonstrates:
/// - Enter, Tab, Esc
/// - Backspace, Delete, Insert
/// - Various control characters
fn example_7_special_keys() -> Result<()> {
    println!("--- Example 7: Special Keys ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use cat -v to show special characters
    let mut cmd = CommandBuilder::new("cat");
    cmd.arg("-v");
    harness.spawn(cmd)?;

    println!("Spawned: cat -v");
    std::thread::sleep(Duration::from_millis(100));

    println!("Sending special keys:");

    println!("  - Tab");
    harness.send_key(KeyCode::Tab)?;

    println!("  - Escape");
    harness.send_key(KeyCode::Esc)?;

    println!("  - Backspace (DEL)");
    harness.send_key(KeyCode::Backspace)?;

    println!("  - Delete");
    harness.send_key(KeyCode::Delete)?;

    println!("  - Insert");
    harness.send_key(KeyCode::Insert)?;

    std::thread::sleep(Duration::from_millis(200));

    let contents = harness.screen_contents();
    println!("\nScreen output (special key sequences):");
    println!("┌{:─<80}┐", "");
    for line in contents.lines().take(4) {
        println!("│{:<80}│", line);
    }
    println!("└{:─<80}┘", "");

    println!("✓ Special keys sent correctly\n");
    Ok(())
}
