//! Integration tests for golden file functionality.

use std::{fs, path::PathBuf};

use ratatui_testlib::{GoldenFile, GoldenMetadata, Result, ScreenState, TuiTestHarness};
use tempfile::TempDir;

/// Setup a temporary directory for golden files in tests.
fn setup_test_golden_dir() -> TempDir {
    let temp = TempDir::new().expect("Failed to create temp dir");
    std::env::set_var("GOLDEN_DIR", temp.path());
    temp
}

/// Cleanup environment variables after tests.
fn cleanup_env() {
    std::env::remove_var("GOLDEN_DIR");
    std::env::remove_var("UPDATE_GOLDENS");
}

#[test]
fn test_golden_metadata_creation() {
    let meta = GoldenMetadata::new("test_golden", 80, 24);

    assert_eq!(meta.test_name, "test_golden");
    assert_eq!(meta.width, 80);
    assert_eq!(meta.height, 24);
    assert!(!meta.timestamp.is_empty());
}

#[test]
fn test_golden_metadata_serialization_roundtrip() {
    let meta = GoldenMetadata::new("test_roundtrip", 120, 40);
    let header = meta.to_header();

    assert!(header.contains("--- GOLDEN FILE ---"));
    assert!(header.contains("test: test_roundtrip"));
    assert!(header.contains("size: 120x40"));
    assert!(header.contains("timestamp:"));

    let parsed = GoldenMetadata::from_header(&header).expect("Failed to parse header");
    assert_eq!(parsed.test_name, meta.test_name);
    assert_eq!(parsed.width, meta.width);
    assert_eq!(parsed.height, meta.height);
}

#[test]
fn test_golden_file_from_screen_state() {
    let _temp = setup_test_golden_dir();
    let mut state = ScreenState::new(80, 24);
    state.feed(b"Hello, World!");

    let golden = GoldenFile::from_screen_state("test_from_state", &state);

    assert_eq!(golden.metadata.test_name, "test_from_state");
    assert_eq!(golden.metadata.width, 80);
    assert_eq!(golden.metadata.height, 24);
    assert!(golden.content.contains("Hello, World!"));

    cleanup_env();
}

#[test]
fn test_golden_file_serialization_roundtrip() {
    let _temp = setup_test_golden_dir();
    let mut state = ScreenState::new(80, 24);
    state.feed(b"Test content\nLine 2\nLine 3");

    let golden = GoldenFile::from_screen_state("test_serialize", &state);
    let serialized = golden.to_string();

    assert!(serialized.contains("--- GOLDEN FILE ---"));
    assert!(serialized.contains("--- CONTENT ---"));
    assert!(serialized.contains("test: test_serialize"));
    assert!(serialized.contains("size: 80x24"));

    let deserialized = GoldenFile::from_string(&serialized).expect("Failed to deserialize");
    assert_eq!(deserialized.metadata.test_name, golden.metadata.test_name);
    assert_eq!(deserialized.metadata.width, golden.metadata.width);
    assert_eq!(deserialized.metadata.height, golden.metadata.height);
    assert_eq!(deserialized.content, golden.content);

    cleanup_env();
}

#[test]
fn test_save_and_load_golden_file() {
    let temp = setup_test_golden_dir();
    let mut state = ScreenState::new(80, 24);
    state.feed(b"Saved content");

    let golden = GoldenFile::from_screen_state("test_save_load", &state);
    let path = golden
        .save("test_save_load")
        .expect("Failed to save golden");

    assert!(path.exists());
    assert!(path.to_str().unwrap().contains("test_save_load.golden.txt"));

    let loaded = GoldenFile::load("test_save_load").expect("Failed to load golden");
    assert_eq!(loaded.metadata.test_name, "test_save_load");
    assert!(loaded.content.contains("Saved content"));

    drop(temp);
    cleanup_env();
}

#[test]
fn test_golden_comparison_success() {
    let _temp = setup_test_golden_dir();
    let mut state1 = ScreenState::new(80, 24);
    state1.feed(b"Identical content");

    let golden = GoldenFile::from_screen_state("test_compare_success", &state1);
    golden.save("test_compare_success").unwrap();

    let mut state2 = ScreenState::new(80, 24);
    state2.feed(b"Identical content");

    let result = golden.compare(&state2);
    assert!(result.is_ok(), "Golden comparison should succeed");

    cleanup_env();
}

#[test]
fn test_golden_comparison_failure() {
    let _temp = setup_test_golden_dir();
    let mut state1 = ScreenState::new(80, 24);
    state1.feed(b"Original content");

    let golden = GoldenFile::from_screen_state("test_compare_fail", &state1);
    golden.save("test_compare_fail").unwrap();

    let mut state2 = ScreenState::new(80, 24);
    state2.feed(b"Different content");

    let result = golden.compare(&state2);
    assert!(result.is_err(), "Golden comparison should fail");

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains("Golden file mismatch"), "Error should mention mismatch");
    assert!(err_msg.contains("test_compare_fail"), "Error should mention test name");
    assert!(err_msg.contains("---"), "Error should contain diff markers");

    cleanup_env();
}

#[test]
fn test_harness_save_golden() -> Result<()> {
    let _temp = setup_test_golden_dir();
    let mut harness = TuiTestHarness::new(80, 24)?;
    harness.state_mut().feed(b"Harness test content");

    let path = harness.save_golden("test_harness_save")?;
    assert!(path.exists());

    let loaded = GoldenFile::load("test_harness_save")?;
    assert!(loaded.content.contains("Harness test content"));

    cleanup_env();
    Ok(())
}

#[test]
fn test_harness_assert_matches_golden() -> Result<()> {
    let _temp = setup_test_golden_dir();
    let mut harness = TuiTestHarness::new(80, 24)?;
    harness.state_mut().feed(b"Golden harness test");

    // Save golden
    harness.save_golden("test_harness_match")?;

    // Create new harness with same content
    let mut harness2 = TuiTestHarness::new(80, 24)?;
    harness2.state_mut().feed(b"Golden harness test");

    // Should match
    harness2.assert_matches_golden("test_harness_match")?;

    cleanup_env();
    Ok(())
}

#[test]
fn test_harness_assert_matches_golden_failure() -> Result<()> {
    let _temp = setup_test_golden_dir();
    let mut harness = TuiTestHarness::new(80, 24)?;
    harness.state_mut().feed(b"Original golden content");

    harness.save_golden("test_harness_mismatch")?;

    let mut harness2 = TuiTestHarness::new(80, 24)?;
    harness2.state_mut().feed(b"Different content");

    let result = harness2.assert_matches_golden("test_harness_mismatch");
    assert!(result.is_err(), "Should fail on mismatch");

    cleanup_env();
    Ok(())
}

#[test]
fn test_harness_update_golden() -> Result<()> {
    let _temp = setup_test_golden_dir();
    let mut harness = TuiTestHarness::new(80, 24)?;
    harness.state_mut().feed(b"Initial content");

    harness.save_golden("test_harness_update")?;

    // Update with new content
    let mut harness2 = TuiTestHarness::new(80, 24)?;
    harness2.state_mut().feed(b"Updated content");
    harness2.update_golden("test_harness_update")?;

    // Load and verify update
    let loaded = GoldenFile::load("test_harness_update")?;
    assert!(loaded.content.contains("Updated content"));
    assert!(!loaded.content.contains("Initial content"));

    cleanup_env();
    Ok(())
}

#[test]
fn test_update_goldens_env_var() {
    let _temp = setup_test_golden_dir();

    std::env::remove_var("UPDATE_GOLDENS");
    assert!(!ratatui_testlib::golden::should_update_goldens());

    std::env::set_var("UPDATE_GOLDENS", "1");
    assert!(ratatui_testlib::golden::should_update_goldens());

    std::env::set_var("UPDATE_GOLDENS", "0");
    assert!(!ratatui_testlib::golden::should_update_goldens());

    std::env::set_var("UPDATE_GOLDENS", "yes");
    assert!(!ratatui_testlib::golden::should_update_goldens());

    cleanup_env();
}

#[test]
fn test_assert_matches_golden_with_update_env() -> Result<()> {
    let _temp = setup_test_golden_dir();
    let mut harness = TuiTestHarness::new(80, 24)?;
    harness.state_mut().feed(b"Original");

    harness.save_golden("test_update_env")?;

    // Enable update mode
    std::env::set_var("UPDATE_GOLDENS", "1");

    let mut harness2 = TuiTestHarness::new(80, 24)?;
    harness2.state_mut().feed(b"Updated via env");

    // Should update instead of compare
    harness2.assert_matches_golden("test_update_env")?;

    // Disable update mode
    std::env::remove_var("UPDATE_GOLDENS");

    // Verify it was updated
    let loaded = GoldenFile::load("test_update_env")?;
    assert!(loaded.content.contains("Updated via env"));

    cleanup_env();
    Ok(())
}

#[test]
fn test_custom_golden_dir() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let custom_path = temp.path().join("my_custom_goldens");
    fs::create_dir(&custom_path).expect("Failed to create custom dir");

    std::env::set_var("GOLDEN_DIR", &custom_path);

    let golden_dir = ratatui_testlib::golden::get_golden_dir();
    assert_eq!(golden_dir, custom_path);

    cleanup_env();
}

#[test]
fn test_golden_with_ansi_sequences() -> Result<()> {
    let _temp = setup_test_golden_dir();
    let mut state = ScreenState::new(80, 24);

    // Feed ANSI color sequences
    state.feed(b"\x1b[31mRed text\x1b[0m");
    state.feed(b"\x1b[32mGreen text\x1b[0m");

    let golden = GoldenFile::from_screen_state("test_ansi", &state);
    golden.save("test_ansi")?;

    let loaded = GoldenFile::load("test_ansi")?;

    // The content should preserve the text (ANSI codes are processed)
    assert!(loaded.content.contains("Red text"));
    assert!(loaded.content.contains("Green text"));

    cleanup_env();
    Ok(())
}

#[test]
fn test_golden_with_multiline_content() -> Result<()> {
    let _temp = setup_test_golden_dir();
    let mut state = ScreenState::new(80, 24);

    state.feed(b"Line 1\r\n");
    state.feed(b"Line 2\r\n");
    state.feed(b"Line 3\r\n");

    let golden = GoldenFile::from_screen_state("test_multiline", &state);
    golden.save("test_multiline")?;

    let loaded = GoldenFile::load("test_multiline")?;
    let content = loaded.content;

    // Should contain all lines
    assert!(content.contains("Line 1"), "Should contain Line 1");
    assert!(content.contains("Line 2"), "Should contain Line 2");
    assert!(content.contains("Line 3"), "Should contain Line 3");

    cleanup_env();
    Ok(())
}

#[test]
fn test_diff_generation_shows_changes() {
    let _temp = setup_test_golden_dir();
    let expected = "Line 1\nLine 2\nLine 3\n";
    let actual = "Line 1\nLine 2 modified\nLine 3\n";

    let diff = ratatui_testlib::golden::generate_diff(expected, actual);

    // Diff should show both files
    assert!(diff.contains("--- expected"));
    assert!(diff.contains("+++ actual"));

    // Should show the change
    assert!(diff.contains("-") || diff.contains("+"));

    cleanup_env();
}

#[test]
fn test_golden_file_missing_error() {
    let _temp = setup_test_golden_dir();

    let result = GoldenFile::load("nonexistent_golden");
    assert!(result.is_err(), "Loading nonexistent golden should fail");

    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(err_msg.contains("Failed to read golden file") || err_msg.contains("I/O error"));

    cleanup_env();
}

#[test]
fn test_golden_file_invalid_format() {
    let _temp = setup_test_golden_dir();

    let invalid_content = "This is not a valid golden file format";
    let result = GoldenFile::from_string(invalid_content);

    assert!(result.is_err(), "Parsing invalid golden should fail");

    cleanup_env();
}

#[test]
fn test_golden_preserves_terminal_dimensions() -> Result<()> {
    let _temp = setup_test_golden_dir();

    // Test with different terminal sizes
    let sizes = [(80, 24), (120, 40), (100, 30)];

    for (width, height) in sizes {
        let mut state = ScreenState::new(width, height);
        state.feed(b"Test content");

        let test_name = format!("test_size_{}x{}", width, height);
        let golden = GoldenFile::from_screen_state(&test_name, &state);
        golden.save(&test_name)?;

        let loaded = GoldenFile::load(&test_name)?;
        assert_eq!(loaded.metadata.width, width);
        assert_eq!(loaded.metadata.height, height);
    }

    cleanup_env();
    Ok(())
}

#[test]
fn test_golden_empty_content() -> Result<()> {
    let _temp = setup_test_golden_dir();
    let state = ScreenState::new(80, 24);

    let golden = GoldenFile::from_screen_state("test_empty", &state);
    golden.save("test_empty")?;

    let loaded = GoldenFile::load("test_empty")?;
    assert_eq!(loaded.metadata.width, 80);
    assert_eq!(loaded.metadata.height, 24);

    cleanup_env();
    Ok(())
}
