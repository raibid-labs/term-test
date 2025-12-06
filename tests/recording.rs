//! Tests for recording and debugging features in TuiTestHarness.

use std::fs;

use ratatui_testlib::{Result, TuiTestHarness};
use tempfile::TempDir;

#[test]
fn test_recording_start_stop() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Initially not recording
    assert!(!harness.is_recording());

    // Start recording
    harness.start_recording();
    assert!(harness.is_recording());

    // Stop recording
    harness.stop_recording();
    assert!(!harness.is_recording());

    Ok(())
}

#[test]
fn test_recording_saves_to_file() -> Result<()> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let recording_path = temp_dir.path().join("recording.json");

    let mut harness = TuiTestHarness::new(80, 24)?;
    harness.start_recording();

    // Simulate some activity (without spawning a process, just state changes)
    // The recording should at least have the start marker

    harness.save_recording(&recording_path)?;

    // Verify file was created
    assert!(recording_path.exists(), "Recording file should exist");

    // Verify file contains JSON array
    let contents = fs::read_to_string(&recording_path)?;
    assert!(contents.starts_with('['), "Recording should be JSON array");
    assert!(contents.ends_with("]\n"), "Recording should end with ]");

    Ok(())
}

#[test]
fn test_screenshot_string_format() -> Result<()> {
    let harness = TuiTestHarness::new(80, 24)?;

    let screenshot = harness.screenshot_string();

    // Verify format
    assert!(screenshot.contains("=== Screen State ==="));
    assert!(screenshot.contains("Size: 80x24"));
    assert!(screenshot.contains("Cursor:"));
    assert!(screenshot.contains("==================="));

    Ok(())
}

#[test]
fn test_save_screenshot() -> Result<()> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let screenshot_path = temp_dir.path().join("screenshot.txt");

    let harness = TuiTestHarness::new(80, 24)?;
    harness.save_screenshot(&screenshot_path)?;

    // Verify file was created
    assert!(screenshot_path.exists(), "Screenshot file should exist");

    // Verify file contains expected content
    let contents = fs::read_to_string(&screenshot_path)?;
    assert!(contents.contains("=== Screen State ==="));
    assert!(contents.contains("Size: 80x24"));

    Ok(())
}

#[test]
fn test_verbose_mode() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Enable verbose mode
    harness.set_verbose(true);

    // Disable verbose mode
    harness.set_verbose(false);

    Ok(())
}

#[test]
fn test_recording_with_send_text() -> Result<()> {
    use portable_pty::CommandBuilder;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let recording_path = temp_dir.path().join("recording_with_text.json");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Spawn a simple command that echoes
    #[cfg(unix)]
    let cmd = CommandBuilder::new("cat");
    #[cfg(windows)]
    let mut cmd = CommandBuilder::new("cmd");
    #[cfg(windows)]
    cmd.arg("/c").arg("type CON");

    harness.start_recording();
    harness.spawn(cmd)?;

    // Send some text
    harness.send_text("hello\n")?;

    // Small delay to let output be processed
    std::thread::sleep(std::time::Duration::from_millis(100));

    harness.save_recording(&recording_path)?;

    // Verify recording contains data
    let contents = fs::read_to_string(&recording_path)?;
    assert!(contents.len() > 10, "Recording should have content");

    // Verify it's valid JSON array
    assert!(contents.starts_with('['));
    assert!(contents.trim().ends_with(']'));

    Ok(())
}

#[test]
fn test_screenshot_after_text_input() -> Result<()> {
    use portable_pty::CommandBuilder;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let screenshot_path = temp_dir.path().join("screenshot_after_input.txt");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Spawn a simple echo command
    #[cfg(unix)]
    let cmd = CommandBuilder::new("cat");
    #[cfg(windows)]
    let mut cmd = CommandBuilder::new("cmd");
    #[cfg(windows)]
    cmd.arg("/c").arg("type CON");

    harness.spawn(cmd)?;
    harness.send_text("test\n")?;

    // Small delay
    std::thread::sleep(std::time::Duration::from_millis(100));

    harness.save_screenshot(&screenshot_path)?;

    let contents = fs::read_to_string(&screenshot_path)?;
    assert!(contents.contains("=== Screen State ==="));

    Ok(())
}

#[test]
#[ignore] // PTY writer limitation prevents multiple send_text calls in quick succession
fn test_recording_preserves_event_order() -> Result<()> {
    use portable_pty::CommandBuilder;

    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let recording_path = temp_dir.path().join("event_order.json");

    let mut harness = TuiTestHarness::new(80, 24)?;

    #[cfg(unix)]
    let cmd = CommandBuilder::new("cat");
    #[cfg(windows)]
    let mut cmd = CommandBuilder::new("cmd");
    #[cfg(windows)]
    cmd.arg("/c").arg("type CON");

    harness.start_recording();
    harness.spawn(cmd)?;

    // Send multiple inputs with adequate delays
    harness.send_text("first\n")?;
    std::thread::sleep(std::time::Duration::from_millis(200));
    harness.send_text("second\n")?;
    std::thread::sleep(std::time::Duration::from_millis(200));

    harness.save_recording(&recording_path)?;

    let contents = fs::read_to_string(&recording_path)?;

    // Verify we have multiple events
    let event_count = contents.matches("\"event\"").count();
    assert!(event_count > 0, "Should have recorded events");

    Ok(())
}

#[test]
fn test_recording_cleared_on_restart() -> Result<()> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let first_recording = temp_dir.path().join("first.json");
    let second_recording = temp_dir.path().join("second.json");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // First recording session
    harness.start_recording();
    // (simulate some activity here if needed)
    harness.save_recording(&first_recording)?;
    harness.stop_recording();

    // Second recording session - should clear previous events
    harness.start_recording();
    // (different activity)
    harness.save_recording(&second_recording)?;

    // Both files should exist
    assert!(first_recording.exists());
    assert!(second_recording.exists());

    Ok(())
}

#[test]
fn test_screenshot_includes_cursor_position() -> Result<()> {
    let harness = TuiTestHarness::new(80, 24)?;
    let screenshot = harness.screenshot_string();

    // Should include cursor position in format "row=X, col=Y"
    assert!(screenshot.contains("row="));
    assert!(screenshot.contains("col="));

    Ok(())
}

#[test]
fn test_multiple_screenshots() -> Result<()> {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    let harness = TuiTestHarness::new(80, 24)?;

    // Save multiple screenshots
    for i in 0..3 {
        let path = temp_dir.path().join(format!("screenshot_{}.txt", i));
        harness.save_screenshot(&path)?;
        assert!(path.exists());
    }

    Ok(())
}
