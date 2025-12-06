//! Example demonstrating shared state access for testing.
//!
//! This example shows how to use the shared-state feature to access
//! memory-mapped shared state during testing. This is particularly useful
//! when testing applications that expose state via protocols like scarab-protocol.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example shared_state_test --features bevy,shared-state
//! ```

use std::{collections::HashMap, fs::File, io::Write, time::Duration};

use ratatui_testlib::shared_state::{
    assert_grid_cell, assert_metric, snapshot_grid, MemoryMappedState, SharedStateAccess,
};
use serde::{Deserialize, Serialize};

/// Example state structure that might be exposed by a TUI application
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TuiState {
    /// The terminal grid contents
    grid: Vec<Vec<char>>,
    /// Performance metrics
    metrics: HashMap<String, f64>,
    /// Application status
    status: String,
}

impl TuiState {
    fn new() -> Self {
        Self {
            grid: vec![vec!['H', 'e', 'l', 'l', 'o'], vec!['W', 'o', 'r', 'l', 'd']],
            metrics: {
                let mut m = HashMap::new();
                m.insert("fps".to_string(), 60.0);
                m.insert("frame_time_ms".to_string(), 16.666);
                m
            },
            status: "Ready".to_string(),
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Shared State Testing Example ===\n");

    // Create a temporary file for the shared state
    let temp_path = "/tmp/ratatui_testlib_example_state.mmap";

    println!("1. Creating example shared state...");
    let state = TuiState::new();
    let encoded = bincode::serialize(&state)?;

    // Write state to file
    let mut file = File::create(temp_path)?;
    file.write_all(&encoded)?;
    file.sync_all()?;
    drop(file);

    println!("   Created shared state at: {}", temp_path);
    println!("   State size: {} bytes\n", encoded.len());

    // Now open the shared state for reading (this is what tests would do)
    println!("2. Opening shared state via memory mapping...");
    let mmap_state = MemoryMappedState::<TuiState>::open(temp_path)?;
    println!("   Memory mapping successful\n");

    // Read the current state
    println!("3. Reading current state snapshot...");
    let snapshot = mmap_state.read()?;
    println!("   Status: {}", snapshot.status);
    println!("   Grid dimensions: {}x{}", snapshot.grid.len(), snapshot.grid[0].len());
    println!("   Metrics count: {}\n", snapshot.metrics.len());

    // Demonstrate helper functions
    println!("4. Using helper functions for assertions...");

    // Test grid cell assertion
    println!("   - Verifying grid cell (0, 0) = 'H'...");
    assert_grid_cell(&snapshot.grid, 0, 0, 'H')?;
    println!("     ✓ Grid cell assertion passed");

    // Test metric assertion
    println!("   - Verifying metric 'fps' = 60.0...");
    assert_metric(&snapshot.metrics, "fps", 60.0)?;
    println!("     ✓ Metric assertion passed");

    // Test grid snapshot
    println!("   - Creating grid snapshot...");
    let grid_snapshot = snapshot_grid(&snapshot.grid);
    println!("     Grid snapshot:");
    for line in grid_snapshot.lines() {
        println!("       > {}", line);
    }

    println!("\n5. Demonstrating wait_for with timeout...");
    println!("   - Waiting for status 'Ready' (should succeed immediately)...");

    let result = mmap_state.wait_for(|s| s.status == "Ready", Duration::from_secs(1));

    match result {
        Ok(()) => println!("     ✓ Condition met successfully"),
        Err(e) => println!("     ✗ Wait failed: {}", e),
    }

    // Clean up
    println!("\n6. Cleaning up...");
    std::fs::remove_file(temp_path)?;
    println!("   ✓ Temporary file removed");

    println!("\n=== Example Complete ===");
    println!("\nThis example demonstrated:");
    println!("  • Creating and memory-mapping shared state");
    println!("  • Reading state snapshots");
    println!("  • Using assertion helpers (grid cells, metrics)");
    println!("  • Capturing grid snapshots for comparison");
    println!("  • Waiting for conditions with timeouts");
    println!("\nUse these patterns in your integration tests with BevyTuiTestHarness!");

    Ok(())
}
