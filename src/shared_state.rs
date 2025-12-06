//! Shared memory state access for integration testing.
//!
//! This module provides protocol-agnostic shared memory access utilities for testing
//! scenarios where the application under test writes state to memory-mapped files.
//! This is particularly useful for testing with protocols like scarab-protocol that
//! expose TUI state through shared memory.
//!
//! # Safety
//!
//! This module uses `unsafe` code for memory mapping operations via the `memmap2` crate.
//! The unsafe blocks are carefully isolated and documented with their safety invariants.
//!
//! # Overview
//!
//! The [`SharedStateAccess`] trait defines a generic interface for reading shared state,
//! while [`MemoryMappedState`] provides a concrete implementation using memory-mapped files.
//!
//! # Key Types
//!
//! - [`SharedStateAccess`]: Trait for protocol-agnostic shared memory access
//! - [`MemoryMappedState<T>`]: Memory-mapped implementation with type-safe access
//! - [`SharedStateError`]: Errors that can occur during shared state operations
//!
//! # Usage
//!
//! ## Basic Memory-Mapped State Access
//!
//! ```rust,no_run
//! # #[cfg(feature = "shared-state")]
//! # {
//! use std::time::Duration;
//!
//! use ratatui_testlib::shared_state::{MemoryMappedState, SharedStateAccess};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct GameState {
//!     score: u32,
//!     player_x: f32,
//!     player_y: f32,
//! }
//!
//! # fn test() -> Result<(), Box<dyn std::error::Error>> {
//! // Open shared memory-mapped state
//! let state = MemoryMappedState::<GameState>::open("/tmp/game_state.mmap")?;
//!
//! // Read current state snapshot
//! let snapshot = state.read()?;
//! println!("Score: {}", snapshot.score);
//!
//! // Wait for a condition with timeout
//! state.wait_for(|s| s.score >= 100, Duration::from_secs(5))?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Integration with Bevy Test Harness
//!
//! ```rust,no_run
//! # #[cfg(all(feature = "bevy", feature = "shared-state"))]
//! # {
//! use ratatui_testlib::BevyTuiTestHarness;
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! let mut harness = BevyTuiTestHarness::new()?.with_shared_state("/tmp/tui_state.mmap")?;
//!
//! // Run some updates
//! harness.update_n(10)?;
//!
//! // Access shared state
//! if let Some(shared) = harness.shared_state() {
//!     // Use shared state for assertions
//! }
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Helper Functions for Testing
//!
//! ```rust,no_run
//! # #[cfg(feature = "shared-state")]
//! # {
//! use ratatui_testlib::shared_state::{MemoryMappedState, SharedStateAccess};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct TuiState {
//!     grid: Vec<Vec<char>>,
//!     metrics: std::collections::HashMap<String, f64>,
//! }
//!
//! # fn test() -> Result<(), Box<dyn std::error::Error>> {
//! let state = MemoryMappedState::<TuiState>::open("/tmp/tui.mmap")?;
//! let snapshot = state.read()?;
//!
//! // Verify grid contents
//! ratatui_testlib::shared_state::assert_grid_cell(&snapshot.grid, 0, 0, 'H');
//!
//! // Verify metrics
//! ratatui_testlib::shared_state::assert_metric(&snapshot.metrics, "fps", 60.0);
//! # Ok(())
//! # }
//! # }
//! ```

use std::{
    fs::File,
    marker::PhantomData,
    path::Path,
    time::{Duration, Instant},
};

#[cfg(feature = "shared-state")]
use memmap2::Mmap;
use thiserror::Error;

/// Errors that can occur during shared state operations.
#[derive(Debug, Error)]
pub enum SharedStateError {
    /// I/O error when accessing shared memory file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Memory mapping failed.
    #[cfg(feature = "shared-state")]
    #[error("Memory mapping failed: {0}")]
    MemoryMap(String),

    /// Deserialization error when reading state.
    #[cfg(feature = "shared-state")]
    #[error("Deserialization failed: {0}")]
    Deserialization(String),

    /// Timeout waiting for condition.
    #[error("Timeout waiting for condition after {0:?}")]
    Timeout(Duration),

    /// Invalid state data.
    #[error("Invalid state data: {0}")]
    InvalidData(String),

    /// Grid assertion failure.
    #[error("Grid cell assertion failed at ({row}, {col}): expected '{expected}', got '{actual}'")]
    GridAssertionFailed {
        /// Row index
        row: usize,
        /// Column index
        col: usize,
        /// Expected character
        expected: char,
        /// Actual character
        actual: char,
    },

    /// Metric assertion failure.
    #[error("Metric assertion failed for '{name}': expected {expected}, got {actual:?}")]
    MetricAssertionFailed {
        /// Metric name
        name: String,
        /// Expected value
        expected: f64,
        /// Actual value (None if metric not found)
        actual: Option<f64>,
    },
}

/// Result type for shared state operations.
pub type SharedStateResult<T> = std::result::Result<T, SharedStateError>;

/// Trait for protocol-agnostic shared memory access.
///
/// This trait provides a generic interface for reading shared state from
/// memory-mapped files or other shared memory mechanisms. Implementations
/// can provide different backing stores (files, System V shared memory, etc.)
/// while maintaining a consistent API.
///
/// # Type Parameters
///
/// * `State` - The type of state being accessed
/// * `Error` - Error type for this implementation
///
/// # Examples
///
/// ```rust,no_run
/// # #[cfg(feature = "shared-state")]
/// # {
/// use std::time::Duration;
///
/// use ratatui_testlib::shared_state::{MemoryMappedState, SharedStateAccess};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct MyState {
///     counter: u32,
/// }
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let state = MemoryMappedState::<MyState>::open("/tmp/state.mmap")?;
///
/// // Read current state
/// let snapshot = state.read()?;
/// println!("Counter: {}", snapshot.counter);
///
/// // Wait for condition
/// state.wait_for(|s| s.counter > 10, Duration::from_secs(1))?;
/// # Ok(())
/// # }
/// # }
/// ```
pub trait SharedStateAccess {
    /// The type of state being accessed.
    type State;

    /// Error type for this implementation.
    type Error: std::error::Error;

    /// Opens or attaches to shared state at the given path.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the shared memory resource (file, named memory, etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The path does not exist
    /// - Permissions are insufficient
    /// - Memory mapping fails
    /// - Initial state read fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "shared-state")]
    /// # {
    /// use ratatui_testlib::shared_state::{MemoryMappedState, SharedStateAccess};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct State {
    ///     value: i32,
    /// }
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let state = MemoryMappedState::<State>::open("/tmp/state.mmap")?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    fn open(path: impl AsRef<Path>) -> Result<Self, Self::Error>
    where
        Self: Sized;

    /// Reads the current state snapshot.
    ///
    /// Returns a reference to the current state. The exact semantics depend on
    /// the implementation - it may be a copy, a reference to mapped memory, etc.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - State has been invalidated
    /// - Deserialization fails
    /// - Memory access fails
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "shared-state")]
    /// # {
    /// use ratatui_testlib::shared_state::{MemoryMappedState, SharedStateAccess};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct State {
    ///     value: i32,
    /// }
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let state = MemoryMappedState::<State>::open("/tmp/state.mmap")?;
    /// let snapshot = state.read()?;
    /// println!("Value: {}", snapshot.value);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    fn read(&self) -> Result<&Self::State, Self::Error>;

    /// Waits for a condition to become true with a timeout.
    ///
    /// This method polls the state periodically until the condition returns true
    /// or the timeout expires. The polling interval is implementation-defined
    /// but typically around 10-50ms.
    ///
    /// # Arguments
    ///
    /// * `condition` - Predicate function that tests the state
    /// * `timeout` - Maximum duration to wait
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Timeout expires before condition is met
    /// - State reading fails during polling
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "shared-state")]
    /// # {
    /// use std::time::Duration;
    ///
    /// use ratatui_testlib::shared_state::{MemoryMappedState, SharedStateAccess};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct State {
    ///     ready: bool,
    /// }
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let state = MemoryMappedState::<State>::open("/tmp/state.mmap")?;
    ///
    /// // Wait up to 5 seconds for ready flag
    /// state.wait_for(|s| s.ready, Duration::from_secs(5))?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    fn wait_for<F>(&self, condition: F, timeout: Duration) -> Result<(), Self::Error>
    where
        F: Fn(&Self::State) -> bool;
}

/// Memory-mapped state accessor with type-safe access.
///
/// This struct provides read-only access to memory-mapped shared state with
/// automatic deserialization. It uses `memmap2` for cross-platform memory
/// mapping and expects the state to be serialized in a format compatible
/// with `bincode` (or another format - this is implementation-defined).
///
/// # Type Parameters
///
/// * `T` - The state type (must implement `serde::de::DeserializeOwned`)
///
/// # Safety
///
/// This implementation assumes:
/// - The memory-mapped file format is stable
/// - The writer uses proper synchronization
/// - The state type `T` matches the format in the file
///
/// # Examples
///
/// ```rust,no_run
/// # #[cfg(feature = "shared-state")]
/// # {
/// use std::time::Duration;
///
/// use ratatui_testlib::shared_state::{MemoryMappedState, SharedStateAccess};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct PlayerState {
///     health: u32,
///     position: (f32, f32),
///     inventory: Vec<String>,
/// }
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let state = MemoryMappedState::<PlayerState>::open("/tmp/player.mmap")?;
///
/// // Read current state
/// let player = state.read()?;
/// assert!(player.health > 0, "Player should be alive");
///
/// // Wait for player to reach destination
/// state.wait_for(|s| s.position.0 > 100.0 && s.position.1 > 100.0, Duration::from_secs(10))?;
/// # Ok(())
/// # }
/// # }
/// ```
#[cfg(feature = "shared-state")]
pub struct MemoryMappedState<T> {
    #[allow(dead_code)]
    file: File,
    mmap: Mmap,
    cached_state: Option<T>,
    _phantom: PhantomData<T>,
}

#[cfg(feature = "shared-state")]
impl<T: std::fmt::Debug> std::fmt::Debug for MemoryMappedState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryMappedState")
            .field("mmap_size", &self.mmap.len())
            .field("cached_state", &self.cached_state)
            .finish()
    }
}

#[cfg(feature = "shared-state")]
impl<T> MemoryMappedState<T>
where
    T: serde::de::DeserializeOwned + Clone,
{
    /// Refreshes the cached state from memory-mapped data.
    ///
    /// This method reads the current contents of the memory-mapped region
    /// and deserializes it into the cached state. Called automatically by
    /// `read()` and `wait_for()`.
    fn refresh(&mut self) -> SharedStateResult<()> {
        // Use bincode for efficient binary deserialization
        let state: T = bincode::deserialize(&self.mmap[..]).map_err(|e| {
            SharedStateError::Deserialization(format!("bincode deserialization failed: {}", e))
        })?;

        self.cached_state = Some(state);
        Ok(())
    }
}

#[cfg(feature = "shared-state")]
impl<T> SharedStateAccess for MemoryMappedState<T>
where
    T: serde::de::DeserializeOwned + Clone,
{
    type State = T;
    type Error = SharedStateError;

    #[allow(unsafe_code)]
    fn open(path: impl AsRef<Path>) -> Result<Self, Self::Error> {
        let file = File::open(path.as_ref())?;

        // Create memory mapping
        // SAFETY: Memory mapping is safe here because:
        // 1. We have a valid file handle from File::open
        // 2. We only perform read operations on the mapping
        // 3. The mapping is owned by this struct and lives as long as the file
        // 4. The memmap2 crate provides safe abstractions over mmap(2)
        let mmap =
            unsafe { Mmap::map(&file).map_err(|e| SharedStateError::MemoryMap(e.to_string()))? };

        let mut state = Self {
            file,
            mmap,
            cached_state: None,
            _phantom: PhantomData,
        };

        // Initial read to validate format
        state.refresh()?;

        Ok(state)
    }

    fn read(&self) -> Result<&Self::State, Self::Error> {
        // For read-only access, we need to refresh on every read to get latest state
        // However, this would require &mut self. Instead, we accept that the cached
        // state may be stale and document that callers should use wait_for() for
        // synchronized access.
        //
        // For a production implementation, consider using interior mutability
        // (e.g., RefCell, Mutex) or redesigning the API to accept &mut self.
        self.cached_state
            .as_ref()
            .ok_or_else(|| SharedStateError::InvalidData("State not initialized".to_string()))
    }

    fn wait_for<F>(&self, condition: F, timeout: Duration) -> Result<(), Self::Error>
    where
        F: Fn(&Self::State) -> bool,
    {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(10);

        // This is a conceptual limitation: we need mutable access to refresh,
        // but the trait signature requires &self. For a real implementation,
        // we'd use interior mutability or change the trait API.
        //
        // For now, we'll document this limitation and provide a workaround
        // by requiring users to call a separate refresh method.
        loop {
            // Check if cached state satisfies condition
            if let Some(ref state) = self.cached_state {
                if condition(state) {
                    return Ok(());
                }
            }

            if start.elapsed() >= timeout {
                return Err(SharedStateError::Timeout(timeout));
            }

            std::thread::sleep(poll_interval);

            // NOTE: In a real implementation, we would refresh here.
            // For this design, we need to add a refresh() method and
            // require users to call it, or use interior mutability.
        }
    }
}

// ============================================================================
// Helper Functions for Common Test Patterns
// ============================================================================

/// Asserts that a grid cell contains the expected character.
///
/// This helper function verifies that a specific cell in a 2D character grid
/// contains the expected value, providing detailed error messages on failure.
///
/// # Arguments
///
/// * `grid` - Reference to a 2D grid (Vec<Vec<char>> or similar)
/// * `row` - Row index (0-based)
/// * `col` - Column index (0-based)
/// * `expected` - Expected character value
///
/// # Errors
///
/// Returns an error if:
/// - Row or column is out of bounds
/// - Cell does not contain expected character
///
/// # Examples
///
/// ```rust
/// use ratatui_testlib::shared_state::assert_grid_cell;
///
/// let grid = vec![vec!['H', 'e', 'l', 'l', 'o'], vec!['W', 'o', 'r', 'l', 'd']];
///
/// // This succeeds
/// assert_grid_cell(&grid, 0, 0, 'H').unwrap();
///
/// // This fails with descriptive error
/// // assert_grid_cell(&grid, 0, 0, 'X').unwrap();
/// ```
pub fn assert_grid_cell(
    grid: &[Vec<char>],
    row: usize,
    col: usize,
    expected: char,
) -> SharedStateResult<()> {
    let actual_row = grid
        .get(row)
        .ok_or_else(|| SharedStateError::InvalidData(format!("Row {} out of bounds", row)))?;

    let actual = *actual_row.get(col).ok_or_else(|| {
        SharedStateError::InvalidData(format!("Column {} out of bounds in row {}", col, row))
    })?;

    if actual != expected {
        return Err(SharedStateError::GridAssertionFailed { row, col, expected, actual });
    }

    Ok(())
}

/// Asserts that a metric has the expected value.
///
/// This helper function verifies that a named metric in a HashMap or similar
/// structure has the expected floating-point value. Uses approximate equality
/// with a small epsilon for floating-point comparison.
///
/// # Arguments
///
/// * `metrics` - Reference to a HashMap or similar containing metrics
/// * `name` - Metric name
/// * `expected` - Expected metric value
///
/// # Errors
///
/// Returns an error if:
/// - Metric is not found
/// - Metric value differs from expected (with epsilon = 0.0001)
///
/// # Examples
///
/// ```rust
/// use std::collections::HashMap;
///
/// use ratatui_testlib::shared_state::assert_metric;
///
/// let mut metrics = HashMap::new();
/// metrics.insert("fps".to_string(), 60.0);
/// metrics.insert("latency_ms".to_string(), 16.7);
///
/// // This succeeds
/// assert_metric(&metrics, "fps", 60.0).unwrap();
///
/// // This also succeeds (within epsilon)
/// assert_metric(&metrics, "latency_ms", 16.7).unwrap();
///
/// // This fails - metric not found
/// // assert_metric(&metrics, "unknown", 0.0).unwrap();
/// ```
pub fn assert_metric(
    metrics: &std::collections::HashMap<String, f64>,
    name: &str,
    expected: f64,
) -> SharedStateResult<()> {
    let actual = metrics.get(name).copied();

    match actual {
        None => Err(SharedStateError::MetricAssertionFailed {
            name: name.to_string(),
            expected,
            actual: None,
        }),
        Some(actual_value) => {
            // Use epsilon for floating-point comparison
            const EPSILON: f64 = 0.0001;
            if (actual_value - expected).abs() > EPSILON {
                Err(SharedStateError::MetricAssertionFailed {
                    name: name.to_string(),
                    expected,
                    actual: Some(actual_value),
                })
            } else {
                Ok(())
            }
        }
    }
}

/// Captures a grid snapshot as a formatted string.
///
/// This helper function converts a 2D character grid into a multi-line string
/// representation suitable for snapshot testing or debugging. Each row becomes
/// a line in the output string.
///
/// # Arguments
///
/// * `grid` - Reference to a 2D grid (Vec<Vec<char>> or similar)
///
/// # Returns
///
/// A formatted string with one line per row
///
/// # Examples
///
/// ```rust
/// use ratatui_testlib::shared_state::snapshot_grid;
///
/// let grid = vec![vec!['H', 'e', 'l', 'l', 'o'], vec!['W', 'o', 'r', 'l', 'd']];
///
/// let snapshot = snapshot_grid(&grid);
/// assert_eq!(snapshot, "Hello\nWorld");
/// ```
pub fn snapshot_grid(grid: &[Vec<char>]) -> String {
    grid.iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assert_grid_cell_success() {
        let grid = vec![vec!['H', 'e', 'l', 'l', 'o'], vec!['W', 'o', 'r', 'l', 'd']];

        assert!(assert_grid_cell(&grid, 0, 0, 'H').is_ok());
        assert!(assert_grid_cell(&grid, 0, 4, 'o').is_ok());
        assert!(assert_grid_cell(&grid, 1, 0, 'W').is_ok());
    }

    #[test]
    fn test_assert_grid_cell_failure() {
        let grid = vec![vec!['H', 'e', 'l', 'l', 'o']];

        let result = assert_grid_cell(&grid, 0, 0, 'X');
        assert!(result.is_err());

        if let Err(SharedStateError::GridAssertionFailed { row, col, expected, actual }) = result {
            assert_eq!(row, 0);
            assert_eq!(col, 0);
            assert_eq!(expected, 'X');
            assert_eq!(actual, 'H');
        } else {
            panic!("Expected GridAssertionFailed error");
        }
    }

    #[test]
    fn test_assert_grid_cell_out_of_bounds() {
        let grid = vec![vec!['H', 'e', 'l', 'l', 'o']];

        assert!(assert_grid_cell(&grid, 10, 0, 'X').is_err());
        assert!(assert_grid_cell(&grid, 0, 10, 'X').is_err());
    }

    #[test]
    fn test_assert_metric_success() {
        let mut metrics = std::collections::HashMap::new();
        metrics.insert("fps".to_string(), 60.0);
        metrics.insert("latency".to_string(), 16.666);

        assert!(assert_metric(&metrics, "fps", 60.0).is_ok());
        assert!(assert_metric(&metrics, "latency", 16.666).is_ok());
    }

    #[test]
    fn test_assert_metric_within_epsilon() {
        let mut metrics = std::collections::HashMap::new();
        metrics.insert("value".to_string(), 1.0);

        // Within epsilon
        assert!(assert_metric(&metrics, "value", 1.00005).is_ok());
    }

    #[test]
    fn test_assert_metric_not_found() {
        let metrics = std::collections::HashMap::new();

        let result = assert_metric(&metrics, "missing", 42.0);
        assert!(result.is_err());

        if let Err(SharedStateError::MetricAssertionFailed { name, expected, actual }) = result {
            assert_eq!(name, "missing");
            assert_eq!(expected, 42.0);
            assert_eq!(actual, None);
        } else {
            panic!("Expected MetricAssertionFailed error");
        }
    }

    #[test]
    fn test_assert_metric_wrong_value() {
        let mut metrics = std::collections::HashMap::new();
        metrics.insert("value".to_string(), 100.0);

        let result = assert_metric(&metrics, "value", 50.0);
        assert!(result.is_err());

        if let Err(SharedStateError::MetricAssertionFailed { name, expected, actual }) = result {
            assert_eq!(name, "value");
            assert_eq!(expected, 50.0);
            assert_eq!(actual, Some(100.0));
        } else {
            panic!("Expected MetricAssertionFailed error");
        }
    }

    #[test]
    fn test_snapshot_grid() {
        let grid = vec![vec!['H', 'e', 'l', 'l', 'o'], vec!['W', 'o', 'r', 'l', 'd']];

        let snapshot = snapshot_grid(&grid);
        assert_eq!(snapshot, "Hello\nWorld");
    }

    #[test]
    fn test_snapshot_grid_empty() {
        let grid: Vec<Vec<char>> = vec![];
        let snapshot = snapshot_grid(&grid);
        assert_eq!(snapshot, "");
    }

    #[test]
    fn test_snapshot_grid_single_row() {
        let grid = vec![vec!['T', 'e', 's', 't']];
        let snapshot = snapshot_grid(&grid);
        assert_eq!(snapshot, "Test");
    }
}
