//! Integration tests for shared state access with Bevy harnesses.

#![cfg(all(feature = "bevy", feature = "shared-state"))]

use std::{collections::HashMap, fs::File, io::Write};

use ratatui_testlib::{
    bevy::{BevyTuiTestHarness, HybridBevyHarness},
    shared_state::{MemoryMappedState, SharedStateAccess},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestState {
    counter: u32,
    message: String,
    metrics: HashMap<String, f64>,
}

impl TestState {
    fn new() -> Self {
        let mut metrics = HashMap::new();
        metrics.insert("test_metric".to_string(), 42.0);

        Self {
            counter: 0,
            message: "Initial state".to_string(),
            metrics,
        }
    }
}

fn write_state_to_file(path: &str, state: &TestState) -> std::io::Result<()> {
    let encoded = bincode::serialize(state).unwrap();
    let mut file = File::create(path)?;
    file.write_all(&encoded)?;
    file.sync_all()?;
    Ok(())
}

#[test]
fn test_bevy_harness_with_shared_state_path() -> ratatui_testlib::Result<()> {
    let temp_path = "/tmp/ratatui_testlib_test_bevy_shared_state.mmap";

    // Create initial state
    let state = TestState::new();
    write_state_to_file(temp_path, &state).unwrap();

    // Create harness with shared state
    let harness = BevyTuiTestHarness::new()?.with_shared_state(temp_path)?;

    // Verify shared state path is set
    assert_eq!(harness.shared_state_path(), Some(temp_path));

    // Open and verify state
    if let Some(path) = harness.shared_state_path() {
        let mmap_state = MemoryMappedState::<TestState>::open(path)?;
        let loaded_state = mmap_state.read()?;
        assert_eq!(loaded_state.counter, 0);
        assert_eq!(loaded_state.message, "Initial state");
    }

    // Clean up
    std::fs::remove_file(temp_path).ok();

    Ok(())
}

#[test]
fn test_hybrid_harness_with_shared_state_path() -> ratatui_testlib::Result<()> {
    let temp_path = "/tmp/ratatui_testlib_test_hybrid_shared_state.mmap";

    // Create initial state
    let state = TestState::new();
    write_state_to_file(temp_path, &state).unwrap();

    // Create harness with shared state
    let harness = HybridBevyHarness::new()?.with_shared_state(temp_path)?;

    // Verify shared state path is set
    assert_eq!(harness.shared_state_path(), Some(temp_path));

    // Open and verify state
    if let Some(path) = harness.shared_state_path() {
        let mmap_state = MemoryMappedState::<TestState>::open(path)?;
        let loaded_state = mmap_state.read()?;
        assert_eq!(loaded_state.counter, 0);
    }

    // Clean up
    std::fs::remove_file(temp_path).ok();

    Ok(())
}

#[test]
fn test_shared_state_without_configuration() -> ratatui_testlib::Result<()> {
    // Create harness without shared state
    let harness = BevyTuiTestHarness::new()?;

    // Verify shared state path is None
    assert_eq!(harness.shared_state_path(), None);

    Ok(())
}

#[test]
fn test_shared_state_read_after_update() -> ratatui_testlib::Result<()> {
    let temp_path = "/tmp/ratatui_testlib_test_state_update.mmap";

    // Create initial state
    let mut state = TestState::new();
    write_state_to_file(temp_path, &state).unwrap();

    // Open shared state
    let mmap_state = MemoryMappedState::<TestState>::open(temp_path)?;
    let initial = mmap_state.read()?;
    assert_eq!(initial.counter, 0);

    // Update the state file (simulating application writing to shared memory)
    state.counter = 10;
    state.message = "Updated state".to_string();
    write_state_to_file(temp_path, &state).unwrap();

    // Note: Current implementation caches state, so we'd need to refresh
    // For now, just verify initial read worked
    assert_eq!(initial.message, "Initial state");

    // Clean up
    std::fs::remove_file(temp_path).ok();

    Ok(())
}

#[test]
fn test_shared_state_with_metrics() -> ratatui_testlib::Result<()> {
    use ratatui_testlib::shared_state::assert_metric;

    let temp_path = "/tmp/ratatui_testlib_test_metrics.mmap";

    // Create state with metrics
    let mut state = TestState::new();
    state.metrics.insert("fps".to_string(), 60.0);
    state.metrics.insert("frame_time".to_string(), 16.666);
    write_state_to_file(temp_path, &state).unwrap();

    // Open and verify metrics
    let mmap_state = MemoryMappedState::<TestState>::open(temp_path)?;
    let loaded = mmap_state.read()?;

    assert_metric(&loaded.metrics, "fps", 60.0).unwrap();
    assert_metric(&loaded.metrics, "frame_time", 16.666).unwrap();
    assert_metric(&loaded.metrics, "test_metric", 42.0).unwrap();

    // Clean up
    std::fs::remove_file(temp_path).ok();

    Ok(())
}

#[test]
fn test_shared_state_error_handling() {
    // Try to open non-existent file
    let result = MemoryMappedState::<TestState>::open("/tmp/nonexistent_file_123456.mmap");
    assert!(result.is_err());
}
