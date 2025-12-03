//! Error types for ratatui_testlib.
//!
//! This module defines all error types that can occur during TUI testing operations.
//! The main error type [`TermTestError`] is an enum covering all possible failure modes,
//! and [`Result<T>`] is a type alias for convenience.
//!
//! # Examples
//!
//! ```rust
//! use ratatui_testlib::{Result, TermTestError};
//!
//! fn may_fail() -> Result<()> {
//!     Err(TermTestError::Timeout { timeout_ms: 5000 })
//! }
//!
//! match may_fail() {
//!     Ok(_) => println!("Success"),
//!     Err(TermTestError::Timeout { timeout_ms }) => {
//!         eprintln!("Timed out after {}ms", timeout_ms);
//!     }
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```

use std::io;
use thiserror::Error;

/// Result type alias for ratatui_testlib operations.
///
/// This is a convenience alias for `std::result::Result<T, TermTestError>`.
/// Most public APIs in this crate return this type.
///
/// # Examples
///
/// ```rust
/// use ratatui_testlib::{Result, TuiTestHarness};
///
/// fn create_harness() -> Result<TuiTestHarness> {
///     TuiTestHarness::new(80, 24)
/// }
/// ```
pub type Result<T> = std::result::Result<T, TermTestError>;

/// Errors that can occur during TUI testing.
///
/// This enum represents all possible error conditions in the ratatui_testlib library.
/// Each variant provides specific context about the failure.
///
/// # Variants
///
/// - [`TermTestError::Pty`]: Low-level PTY operation failures
/// - [`TermTestError::Io`]: Standard I/O errors (file, network, etc.)
/// - [`TermTestError::Timeout`]: Wait operations that exceed their deadline
/// - [`TermTestError::Parse`]: Terminal escape sequence parsing errors
/// - `SnapshotMismatch`: Snapshot testing failures (requires `snapshot-insta` feature)
/// - `SixelValidation`: Sixel graphics validation failures (requires `sixel` feature)
/// - [`TermTestError::SpawnFailed`]: Process spawning failures
/// - [`TermTestError::ProcessAlreadyRunning`]: Attempt to spawn when a process is already running
/// - [`TermTestError::NoProcessRunning`]: Attempt to interact with a non-existent process
/// - [`TermTestError::InvalidDimensions`]: Invalid terminal size parameters
/// - `Bevy`: Bevy ECS-related errors (requires `bevy` feature)
#[derive(Debug, Error)]
pub enum TermTestError {
    /// Error from PTY (pseudo-terminal) operations.
    ///
    /// This error occurs when low-level PTY operations fail, such as:
    /// - PTY allocation failures
    /// - PTY configuration errors
    /// - PTY system unavailability
    #[error("PTY error: {0}")]
    Pty(String),

    /// Standard I/O error.
    ///
    /// This wraps [`std::io::Error`] and occurs for file operations, network I/O,
    /// or other system-level I/O failures. Automatically converted via `From` trait.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Timeout waiting for a condition.
    ///
    /// This error is returned when a wait operation (like `TuiTestHarness::wait_for`)
    /// exceeds its configured timeout duration. The error includes the timeout value
    /// for debugging purposes.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{TuiTestHarness, TermTestError};
    /// use std::time::Duration;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?
    ///     .with_timeout(Duration::from_secs(1));
    ///
    /// match harness.wait_for_text("Never appears") {
    ///     Err(TermTestError::Timeout { timeout_ms }) => {
    ///         eprintln!("Timed out after {}ms", timeout_ms);
    ///     }
    ///     _ => {}
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[error("Timeout waiting for condition after {timeout_ms}ms")]
    Timeout {
        /// Timeout duration in milliseconds.
        timeout_ms: u64,
    },

    /// Error parsing terminal escape sequences.
    ///
    /// This occurs when the terminal emulator encounters malformed or unexpected
    /// escape sequences in the PTY output.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Snapshot comparison mismatch.
    ///
    /// This error is returned when using the `snapshot-insta` feature and a
    /// snapshot assertion fails. Requires the `snapshot-insta` feature flag.
    #[cfg(feature = "snapshot-insta")]
    #[error("Snapshot mismatch: {0}")]
    SnapshotMismatch(String),

    /// Sixel validation failed.
    ///
    /// This error occurs when Sixel graphics validation fails, such as:
    /// - Sixel graphics outside expected bounds
    /// - Invalid Sixel sequence format
    /// - Sixel position validation failures
    ///
    /// Requires the `sixel` feature flag.
    #[cfg(feature = "sixel")]
    #[error("Sixel validation failed: {0}")]
    SixelValidation(String),

    /// Process spawn failed.
    ///
    /// This error occurs when attempting to spawn a process in the PTY fails,
    /// typically due to:
    /// - Command not found
    /// - Permission denied
    /// - Resource limits exceeded
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),

    /// Process already running.
    ///
    /// This error is returned when attempting to spawn a process while another
    /// process is still running in the PTY. Only one process can run at a time
    /// in a given `TestTerminal`.
    #[error("Process is already running")]
    ProcessAlreadyRunning,

    /// No process running.
    ///
    /// This error occurs when attempting to interact with a process (e.g., wait,
    /// kill) when no process is currently running in the PTY.
    #[error("No process is running")]
    NoProcessRunning,

    /// Invalid terminal dimensions.
    ///
    /// This error is returned when attempting to create or resize a terminal
    /// with invalid dimensions (e.g., zero width or height, or dimensions that
    /// exceed system limits).
    #[error("Invalid terminal dimensions: width={width}, height={height}")]
    InvalidDimensions {
        /// Terminal width in columns.
        width: u16,
        /// Terminal height in rows.
        height: u16,
    },

    /// Process has exited.
    ///
    /// This error is returned when attempting to read from or interact with a PTY
    /// whose child process has already terminated. This prevents infinite loops
    /// in wait operations when the process exits unexpectedly.
    #[error("Child process has exited")]
    ProcessExited,

    /// Bevy ECS-specific errors.
    ///
    /// This error occurs for Bevy-related failures when using the `bevy` feature,
    /// such as:
    /// - Entity query failures
    /// - System execution errors
    /// - Plugin initialization failures
    ///
    /// Requires the `bevy` feature flag.
    #[cfg(feature = "bevy")]
    #[error("Bevy error: {0}")]
    Bevy(String),
}

// Conversion from anyhow::Error (used by portable-pty)
impl From<anyhow::Error> for TermTestError {
    fn from(err: anyhow::Error) -> Self {
        TermTestError::Pty(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test error");
        let term_err: TermTestError = io_err.into();

        assert!(matches!(term_err, TermTestError::Io(_)));
        assert!(term_err.to_string().contains("test error"));
    }

    #[test]
    fn test_timeout_error_message() {
        let err = TermTestError::Timeout { timeout_ms: 5000 };
        let msg = err.to_string();

        assert!(msg.contains("5000"));
        assert!(msg.contains("Timeout"));
    }

    #[test]
    fn test_invalid_dimensions_error() {
        let err = TermTestError::InvalidDimensions {
            width: 0,
            height: 24,
        };
        let msg = err.to_string();

        assert!(msg.contains("Invalid"));
        assert!(msg.contains("width=0"));
        assert!(msg.contains("height=24"));
    }

    #[test]
    fn test_spawn_failed_error() {
        let err = TermTestError::SpawnFailed("command not found".to_string());
        let msg = err.to_string();

        assert!(msg.contains("Failed to spawn"));
        assert!(msg.contains("command not found"));
    }

    #[test]
    fn test_process_already_running_error() {
        let err = TermTestError::ProcessAlreadyRunning;
        let msg = err.to_string();

        assert!(msg.contains("already running"));
    }

    #[test]
    fn test_no_process_running_error() {
        let err = TermTestError::NoProcessRunning;
        let msg = err.to_string();

        assert!(msg.contains("No process"));
    }

    #[test]
    fn test_anyhow_error_conversion() {
        let anyhow_err = anyhow::anyhow!("test anyhow error");
        let term_err: TermTestError = anyhow_err.into();

        assert!(matches!(term_err, TermTestError::Pty(_)));
        assert!(term_err.to_string().contains("test anyhow error"));
    }

    #[test]
    fn test_process_exited_error() {
        let err = TermTestError::ProcessExited;
        let msg = err.to_string();

        assert!(msg.contains("exited"));
        assert!(msg.contains("Child process"));
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_sixel_validation_error() {
        let err = TermTestError::SixelValidation("out of bounds".to_string());
        let msg = err.to_string();

        assert!(msg.contains("Sixel"));
        assert!(msg.contains("out of bounds"));
    }
}
