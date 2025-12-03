# Shared State Testing Guide

This document describes the shared state testing feature in `ratatui-testlib`, which enables memory-mapped shared state access for integration testing.

## Overview

The `shared-state` feature provides a protocol-agnostic way to access shared memory state during testing. This is particularly useful when testing applications that expose their internal state via memory-mapped files, such as those using the [scarab-protocol](https://github.com/raibid-labs/scarab-protocol).

## Key Components

### 1. `SharedStateAccess` Trait

A generic interface for reading shared state:

```rust
pub trait SharedStateAccess {
    type State;
    type Error;

    fn open(path: impl AsRef<Path>) -> Result<Self, Self::Error>;
    fn read(&self) -> Result<&Self::State, Self::Error>;
    fn wait_for<F>(&self, condition: F, timeout: Duration) -> Result<(), Self::Error>
    where F: Fn(&Self::State) -> bool;
}
```

### 2. `MemoryMappedState<T>`

A concrete implementation using the `memmap2` crate:

```rust
use ratatui_testlib::shared_state::{MemoryMappedState, SharedStateAccess};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppState {
    counter: u32,
    status: String,
}

let state = MemoryMappedState::<AppState>::open("/tmp/app_state.mmap")?;
let snapshot = state.read()?;
```

### 3. Helper Functions

Convenience functions for common test patterns:

- **`assert_grid_cell(grid, row, col, expected)`**: Verify grid cell contents
- **`assert_metric(metrics, name, expected)`**: Verify metric values
- **`snapshot_grid(grid)`**: Capture grid for comparison

## Integration with Bevy Harnesses

### BevyTuiTestHarness

```rust
use ratatui_testlib::BevyTuiTestHarness;
use ratatui_testlib::shared_state::{MemoryMappedState, SharedStateAccess};

let harness = BevyTuiTestHarness::new()?
    .with_shared_state("/tmp/tui_state.mmap")?;

// Run updates
harness.update_n(10)?;

// Access shared state
if let Some(path) = harness.shared_state_path() {
    let state = MemoryMappedState::<AppState>::open(path)?;
    let app_state = state.read()?;
    assert_eq!(app_state.status, "Ready");
}
```

### HybridBevyHarness

```rust
use ratatui_testlib::HybridBevyHarness;

let harness = HybridBevyHarness::new()?
    .with_shared_state("/tmp/client_state.mmap")?;

harness.tick()?;

if let Some(path) = harness.shared_state_path() {
    // Access shared state
}
```

## Usage Examples

### Basic State Access

```rust
use ratatui_testlib::shared_state::{MemoryMappedState, SharedStateAccess};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GameState {
    score: u32,
    level: u32,
    player_health: f32,
}

// Open shared state
let state = MemoryMappedState::<GameState>::open("/tmp/game.mmap")?;

// Read current state
let game = state.read()?;
assert!(game.player_health > 0.0);

// Wait for condition with timeout
state.wait_for(
    |s| s.score >= 100,
    Duration::from_secs(5)
)?;
```

### Grid Verification

```rust
use ratatui_testlib::shared_state::{assert_grid_cell, snapshot_grid};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TuiState {
    grid: Vec<Vec<char>>,
}

let state = MemoryMappedState::<TuiState>::open("/tmp/tui.mmap")?;
let tui = state.read()?;

// Verify specific cells
assert_grid_cell(&tui.grid, 0, 0, 'H')?;
assert_grid_cell(&tui.grid, 0, 1, 'e')?;

// Capture grid for snapshot testing
let snapshot = snapshot_grid(&tui.grid);
insta::assert_snapshot!(snapshot);
```

### Metrics Validation

```rust
use ratatui_testlib::shared_state::assert_metric;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PerfState {
    metrics: HashMap<String, f64>,
}

let state = MemoryMappedState::<PerfState>::open("/tmp/perf.mmap")?;
let perf = state.read()?;

// Verify metrics (uses epsilon for floating point comparison)
assert_metric(&perf.metrics, "fps", 60.0)?;
assert_metric(&perf.metrics, "frame_time_ms", 16.666)?;
```

## Architecture

### Memory Safety

The implementation uses `unsafe` code for memory mapping via `memmap2`, but isolates it carefully:

1. Memory mapping is performed only in the `open()` method
2. Safety invariants are documented
3. Read-only access prevents data races
4. Mapping lifetime is tied to the struct

### Error Handling

The module defines `SharedStateError` for:
- I/O errors (file not found, permissions)
- Memory mapping failures
- Deserialization errors
- Timeout errors
- Assertion failures (grid cells, metrics)

These errors automatically convert to `TermTestError` when using the `?` operator.

### Cross-Platform Support

Uses `memmap2` for cross-platform memory mapping on:
- Linux (mmap)
- macOS (mmap)
- Windows (CreateFileMapping/MapViewOfFile)

## Serialization Format

The default implementation uses `bincode` for efficient binary serialization. The state type must implement:
- `serde::de::DeserializeOwned`
- `Clone` (for caching)

Example:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MyState {
    // fields...
}
```

## Performance Considerations

1. **Caching**: The current state is cached after reading. For frequently-updated state, consider implementing a refresh mechanism.

2. **Polling**: `wait_for()` polls every 10ms by default. This is suitable for most testing scenarios.

3. **Memory Mapping Overhead**: Memory mapping has minimal overhead once established, but initial mapping takes time.

4. **Serialization**: `bincode` is fast but requires matching versions. For cross-version compatibility, consider JSON or other self-describing formats.

## Testing Best Practices

### 1. Use Descriptive State Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestScenarioState {
    phase: String,          // "init", "running", "complete"
    items_processed: u32,
    errors: Vec<String>,
}
```

### 2. Set Appropriate Timeouts

```rust
// Quick check (100ms)
state.wait_for(|s| s.phase == "init", Duration::from_millis(100))?;

// Longer operation (5s)
state.wait_for(|s| s.items_processed >= 1000, Duration::from_secs(5))?;
```

### 3. Combine with Screen State

```rust
let mut harness = BevyTuiTestHarness::new()?
    .with_shared_state("/tmp/state.mmap")?;

// Check both terminal output and shared state
assert!(harness.state().contains("Processing..."));

if let Some(path) = harness.shared_state_path() {
    let state = MemoryMappedState::<AppState>::open(path)?;
    let app = state.read()?;
    assert!(app.progress > 0.0);
}
```

### 4. Clean Up Temporary Files

```rust
let temp_path = "/tmp/test_state.mmap";

// ... test code ...

// Clean up
std::fs::remove_file(temp_path).ok();
```

## Limitations and Future Work

### Current Limitations

1. **Read-Only**: The trait is designed for read-only access. Write support would require careful synchronization.

2. **Cached State**: State is cached after initial read. For real-time updates, you'd need to track file modifications.

3. **Format Dependency**: Using `bincode` means both writer and reader must use the same version.

4. **No Incremental Updates**: Full deserialization on each read (no delta updates).

### Potential Enhancements

1. **Interior Mutability**: Use `RefCell` or `Mutex` to allow refreshing without `&mut self`

2. **File Watching**: Monitor file changes and auto-refresh state

3. **Format Flexibility**: Support multiple serialization formats (JSON, MessagePack, etc.)

4. **Incremental Access**: Support reading specific fields without full deserialization

5. **Write Support**: Add safe write APIs with proper locking

## Example Applications

This feature is designed for testing applications like:

1. **TUI with Daemon Architecture**: Where a daemon process exposes state via shared memory
2. **Client-Server TUIs**: Testing client state while server runs in PTY
3. **Performance Monitoring**: Validating metrics exposed via shared memory
4. **State Persistence**: Testing state save/restore mechanisms

## Related Features

- **`bevy` feature**: Integrates with Bevy ECS testing harnesses
- **`snapshot-insta` feature**: Combine shared state with snapshot testing
- **`headless` feature**: Run in CI/CD without display server

## Resources

- [Module Documentation](../src/shared_state.rs)
- [Example Code](../examples/shared_state_test.rs)
- [Integration Tests](../tests/shared_state_integration.rs)
- [memmap2 crate](https://docs.rs/memmap2)
- [bincode crate](https://docs.rs/bincode)
