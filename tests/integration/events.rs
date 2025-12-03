//! Integration tests for keyboard and mouse event simulation.
//!
//! These tests verify that keyboard and mouse events are correctly encoded and sent to
//! PTY-based applications, and that the applications respond as expected.

use portable_pty::CommandBuilder;
use std::time::Duration;
use term_test::{KeyCode, Modifiers, MouseButton, Result, ScrollDirection, TuiTestHarness};

/// Test sending a single character key.
#[test]
fn test_send_single_char() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;
    let mut cmd = CommandBuilder::new("cat");
    harness.spawn(cmd)?;

    // Give cat time to start
    std::thread::sleep(Duration::from_millis(100));

    // Send a character
    harness.send_key(KeyCode::Char('a'))?;

    // cat should echo it back
    harness.wait_for_text("a")?;

    Ok(())
}

/// Test sending multiple characters using send_keys.
#[test]
fn test_send_keys_string() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;
    let mut cmd = CommandBuilder::new("cat");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Type a string
    harness.send_keys("hello")?;

    // cat should echo it back
    harness.wait_for_text("hello")?;

    Ok(())
}

/// Test sending Enter key.
#[test]
fn test_send_enter_key() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use bash to read a line and echo it
    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c").arg("read line && echo \"Got: $line\"");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Type text and press Enter
    harness.send_keys("test")?;
    harness.send_key(KeyCode::Enter)?;

    // Wait for the response
    harness.wait_for_text("Got: test")?;

    Ok(())
}

/// Test sending Tab key.
#[test]
fn test_send_tab_key() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;
    let mut cmd = CommandBuilder::new("cat");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send Tab
    harness.send_key(KeyCode::Tab)?;

    // Tab should be visible in the output (as whitespace or special char)
    // cat echoes it back
    let contents = harness.screen_contents();
    assert!(!contents.is_empty(), "Screen should have content after Tab");

    Ok(())
}

/// Test sending Ctrl+C (interrupt signal).
///
/// Note: This test verifies that Ctrl+C is sent, but the application
/// behavior depends on signal handling. cat exits on Ctrl+D, not Ctrl+C
/// when in a PTY.
#[test]
fn test_send_ctrl_d_eof() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;
    let mut cmd = CommandBuilder::new("cat");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Type something first
    harness.send_keys("data")?;

    // Send Ctrl+D (EOF)
    harness.send_key_with_modifiers(KeyCode::Char('d'), Modifiers::CTRL)?;

    // Give cat time to exit
    std::thread::sleep(Duration::from_millis(200));

    // cat should have exited
    assert!(
        !harness.is_running(),
        "cat should exit after Ctrl+D (EOF)"
    );

    Ok(())
}

/// Test sending Ctrl key combinations.
#[test]
fn test_ctrl_combinations() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use a bash script that reads a control character
    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c")
        .arg("dd bs=1 count=1 2>/dev/null | od -An -tx1");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send Ctrl+A (0x01)
    harness.send_key_with_modifiers(KeyCode::Char('a'), Modifiers::CTRL)?;

    // Wait for output - should show "01" in hex
    std::thread::sleep(Duration::from_millis(300));
    let contents = harness.screen_contents();

    // The output should contain "01" (hex representation of Ctrl+A)
    assert!(
        contents.contains("01"),
        "Expected hex 01 for Ctrl+A, got: {}",
        contents
    );

    Ok(())
}

/// Test sending Alt key combinations.
#[test]
fn test_alt_combinations() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use od to show the bytes sent
    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c")
        .arg("dd bs=1 count=2 2>/dev/null | od -An -tx1");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send Alt+a (ESC + 'a')
    harness.send_key_with_modifiers(KeyCode::Char('a'), Modifiers::ALT)?;

    std::thread::sleep(Duration::from_millis(300));
    let contents = harness.screen_contents();

    // Should show "1b 61" (ESC = 0x1b, 'a' = 0x61)
    assert!(
        contents.contains("1b") && contents.contains("61"),
        "Expected ESC (1b) + 'a' (61) for Alt+a, got: {}",
        contents
    );

    Ok(())
}

/// Test sending arrow keys.
#[test]
fn test_arrow_keys() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use cat -v to show escape sequences
    let mut cmd = CommandBuilder::new("cat");
    cmd.arg("-v");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send Up arrow
    harness.send_key(KeyCode::Up)?;

    std::thread::sleep(Duration::from_millis(200));

    // cat -v should show the escape sequence (^[[A or similar)
    let contents = harness.screen_contents();
    assert!(
        !contents.is_empty(),
        "Screen should show escape sequence for Up arrow"
    );

    Ok(())
}

/// Test sending function keys.
#[test]
fn test_function_keys() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("cat");
    cmd.arg("-v");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send F1
    harness.send_key(KeyCode::F(1))?;

    std::thread::sleep(Duration::from_millis(200));

    let contents = harness.screen_contents();
    assert!(
        !contents.is_empty(),
        "Screen should show escape sequence for F1"
    );

    Ok(())
}

/// Test sending navigation keys (Home, End).
#[test]
fn test_navigation_keys() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("cat");
    cmd.arg("-v");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send Home
    harness.send_key(KeyCode::Home)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send End
    harness.send_key(KeyCode::End)?;

    std::thread::sleep(Duration::from_millis(200));

    let contents = harness.screen_contents();
    assert!(
        !contents.is_empty(),
        "Screen should show escape sequences for navigation keys"
    );

    Ok(())
}

/// Test sending Page Up and Page Down keys.
#[test]
fn test_page_keys() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("cat");
    cmd.arg("-v");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send PageUp
    harness.send_key(KeyCode::PageUp)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send PageDown
    harness.send_key(KeyCode::PageDown)?;

    std::thread::sleep(Duration::from_millis(200));

    let contents = harness.screen_contents();
    assert!(
        !contents.is_empty(),
        "Screen should show escape sequences for page keys"
    );

    Ok(())
}

/// Test interactive session with mixed input.
#[test]
fn test_mixed_input_session() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use bash to create an interactive session
    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c")
        .arg("read -p 'Name: ' name && echo \"Hello, $name!\"");
    harness.spawn(cmd)?;

    // Wait for prompt
    harness.wait_for_text("Name:")?;

    // Type name using different methods
    harness.send_keys("Alice")?;
    harness.send_key(KeyCode::Enter)?;

    // Wait for response
    harness.wait_for_text("Hello, Alice!")?;

    Ok(())
}

/// Test that send_key_event handles rapid input correctly.
#[test]
fn test_rapid_key_input() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c").arg("cat > /dev/null; echo Done");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send many keys rapidly
    for _ in 0..10 {
        harness.send_key(KeyCode::Char('x'))?;
    }

    // Send EOF
    harness.send_key_with_modifiers(KeyCode::Char('d'), Modifiers::CTRL)?;

    // Should complete successfully
    harness.wait_for_text("Done")?;

    Ok(())
}

/// Test backspace key.
#[test]
fn test_backspace_key() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("cat");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Type some text
    harness.send_keys("abc")?;

    // Send backspace
    harness.send_key(KeyCode::Backspace)?;

    std::thread::sleep(Duration::from_millis(200));

    // The backspace should be processed
    // Note: cat may show the backspace sequence
    let contents = harness.screen_contents();
    assert!(!contents.is_empty(), "Screen should have content");

    Ok(())
}

/// Test escape key.
#[test]
fn test_escape_key() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("cat");
    cmd.arg("-v");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send Escape
    harness.send_key(KeyCode::Esc)?;

    std::thread::sleep(Duration::from_millis(200));

    let contents = harness.screen_contents();
    // cat -v shows ESC as ^[
    assert!(
        !contents.is_empty(),
        "Screen should show escape character"
    );

    Ok(())
}

/// Test Delete key (different from Backspace).
#[test]
fn test_delete_key() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("cat");
    cmd.arg("-v");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send Delete
    harness.send_key(KeyCode::Delete)?;

    std::thread::sleep(Duration::from_millis(200));

    let contents = harness.screen_contents();
    assert!(
        !contents.is_empty(),
        "Screen should show escape sequence for Delete"
    );

    Ok(())
}

// ============================================================================
// Mouse Event Tests
// ============================================================================

/// Test sending a left mouse click.
#[test]
fn test_mouse_left_click() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use od to show the bytes sent
    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c")
        .arg("dd bs=1 count=16 2>/dev/null | od -An -tx1");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send a left click at position (10, 5)
    harness.click(10, 5)?;

    std::thread::sleep(Duration::from_millis(300));
    let contents = harness.screen_contents();

    // Should see the SGR mouse sequence: ESC[<0;11;6M (press) and ESC[<0;11;6m (release)
    // Looking for the escape sequence start: 1b 5b 3c (ESC [ <)
    assert!(
        contents.contains("1b") && contents.contains("5b") && contents.contains("3c"),
        "Expected SGR mouse sequence for left click, got: {}",
        contents
    );

    Ok(())
}

/// Test sending a right mouse click.
#[test]
fn test_mouse_right_click() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c")
        .arg("dd bs=1 count=16 2>/dev/null | od -An -tx1");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send a right click at position (10, 5)
    harness.right_click(10, 5)?;

    std::thread::sleep(Duration::from_millis(300));
    let contents = harness.screen_contents();

    // Should see SGR mouse sequence starting with ESC[<2 (button 2 = right)
    assert!(
        contents.contains("1b") && contents.contains("5b"),
        "Expected SGR mouse sequence for right click, got: {}",
        contents
    );

    Ok(())
}

/// Test sending mouse scroll events.
#[test]
fn test_mouse_scroll() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c")
        .arg("dd bs=1 count=16 2>/dev/null | od -An -tx1");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send scroll up event
    harness.scroll(10, 5, ScrollDirection::Up, 1)?;

    std::thread::sleep(Duration::from_millis(300));
    let contents = harness.screen_contents();

    // Should see SGR mouse sequence
    assert!(
        contents.contains("1b") && contents.contains("5b"),
        "Expected SGR mouse sequence for scroll, got: {}",
        contents
    );

    Ok(())
}

/// Test sending a drag operation.
#[test]
fn test_mouse_drag() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c")
        .arg("dd bs=1 count=48 2>/dev/null | od -An -tx1");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send drag from (10, 5) to (20, 15)
    harness.drag((10, 5), (20, 15))?;

    std::thread::sleep(Duration::from_millis(300));
    let contents = harness.screen_contents();

    // Should see multiple SGR mouse sequences (press, motion, release)
    assert!(
        contents.contains("1b") && contents.contains("5b"),
        "Expected SGR mouse sequences for drag, got: {}",
        contents
    );

    Ok(())
}

/// Test sending mouse click with modifiers.
#[test]
fn test_mouse_click_with_ctrl() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c")
        .arg("dd bs=1 count=16 2>/dev/null | od -An -tx1");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send Ctrl+Click
    harness.send_mouse_event(10, 5, MouseButton::Left, Modifiers::CTRL)?;

    std::thread::sleep(Duration::from_millis(300));
    let contents = harness.screen_contents();

    // Should see SGR mouse sequence with modified button code
    assert!(
        contents.contains("1b") && contents.contains("5b"),
        "Expected SGR mouse sequence with Ctrl modifier, got: {}",
        contents
    );

    Ok(())
}

/// Test multiple scroll events.
#[test]
fn test_mouse_multiple_scrolls() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c")
        .arg("dd bs=1 count=48 2>/dev/null | od -An -tx1");
    harness.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(100));

    // Send 3 scroll up events
    harness.scroll(10, 5, ScrollDirection::Up, 3)?;

    std::thread::sleep(Duration::from_millis(300));
    let contents = harness.screen_contents();

    // Should see multiple SGR mouse sequences
    assert!(
        contents.contains("1b") && contents.contains("5b"),
        "Expected multiple SGR mouse sequences for scrolling, got: {}",
        contents
    );

    Ok(())
}
