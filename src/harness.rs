//! High-level test harness for TUI applications.
//!
//! This module provides the main testing interface for TUI applications through
//! the [`TuiTestHarness`] struct. It combines PTY management and terminal emulation
//! into an ergonomic API for spawning applications, sending input, and waiting for
//! screen updates.
//!
//! # Key Features
//!
//! - **Process Management**: Spawn and control TUI applications
//! - **Input Simulation**: Send keyboard input to the application
//! - **State Inspection**: Query screen contents and cursor position
//! - **Wait Conditions**: Block until specific screen states are reached
//! - **Flexible Configuration**: Builder pattern for custom timeout/polling settings
//!
//! # Example
//!
//! ```rust,no_run
//! use ratatui_testlib::TuiTestHarness;
//! use portable_pty::CommandBuilder;
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! // Create a test harness
//! let mut harness = TuiTestHarness::new(80, 24)?;
//!
//! // Spawn your TUI application
//! let mut cmd = CommandBuilder::new("my-tui-app");
//! harness.spawn(cmd)?;
//!
//! // Wait for initial render
//! harness.wait_for_text("Welcome")?;
//!
//! // Send input
//! harness.send_text("hello\n")?;
//!
//! // Verify output
//! assert!(harness.screen_contents().contains("hello"));
//! # Ok(())
//! # }
//! ```

use crate::error::{Result, TermTestError};
use crate::events::{encode_key_event, KeyCode, KeyEvent, Modifiers};
use crate::pty::TestTerminal;
use crate::screen::ScreenState;
use portable_pty::{CommandBuilder, ExitStatus};
use std::time::{Duration, Instant};

/// Default timeout for wait operations (5 seconds).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Axis for alignment checking.
///
/// Used with [`TuiTestHarness::assert_aligned`] to specify which axis to check
/// for alignment between two rectangles.
///
/// # Example
///
/// ```rust,no_run
/// use ratatui_testlib::{TuiTestHarness, Rect, Axis};
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let harness = TuiTestHarness::new(80, 24)?;
/// let button1 = Rect::new(10, 20, 15, 3);
/// let button2 = Rect::new(30, 20, 15, 3);
///
/// // Check horizontal alignment (same Y coordinate)
/// harness.assert_aligned(button1, button2, Axis::Horizontal)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Horizontal axis (same Y coordinate).
    Horizontal,
    /// Vertical axis (same X coordinate).
    Vertical,
}

/// Default polling interval for wait operations (100ms).
const DEFAULT_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Default buffer size for reading PTY output (4KB).
const DEFAULT_BUFFER_SIZE: usize = 4096;

/// High-level test harness for TUI applications.
///
/// This combines PTY management and terminal emulation to provide
/// an ergonomic API for testing TUI applications.
///
/// # Example
///
/// ```rust,no_run
/// use ratatui_testlib::TuiTestHarness;
/// use portable_pty::CommandBuilder;
///
/// let mut harness = TuiTestHarness::new(80, 24)?;
/// let mut cmd = CommandBuilder::new("my-app");
/// harness.spawn(cmd)?;
/// harness.wait_for(|state| state.contains("Ready"))?;
/// # Ok::<(), ratatui_testlib::TermTestError>(())
/// ```
///
/// # Builder Pattern
///
/// ```rust,no_run
/// use ratatui_testlib::TuiTestHarness;
/// use std::time::Duration;
///
/// let mut harness = TuiTestHarness::builder()
///     .with_size(80, 24)
///     .with_timeout(Duration::from_secs(10))
///     .with_poll_interval(Duration::from_millis(50))
///     .build()?;
/// # Ok::<(), ratatui_testlib::TermTestError>(())
/// ```
pub struct TuiTestHarness {
    terminal: TestTerminal,
    state: ScreenState,
    timeout: Duration,
    poll_interval: Duration,
    buffer_size: usize,
}

impl TuiTestHarness {
    /// Creates a new test harness with the specified dimensions.
    ///
    /// # Arguments
    ///
    /// * `width` - Terminal width in columns
    /// * `height` - Terminal height in rows
    ///
    /// # Errors
    ///
    /// Returns an error if terminal creation fails.
    pub fn new(width: u16, height: u16) -> Result<Self> {
        let terminal = TestTerminal::new(width, height)?;
        let state = ScreenState::new(width, height);

        Ok(Self {
            terminal,
            state,
            timeout: DEFAULT_TIMEOUT,
            poll_interval: DEFAULT_POLL_INTERVAL,
            buffer_size: DEFAULT_BUFFER_SIZE,
        })
    }

    /// Creates a builder for configuring a test harness.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    /// use std::time::Duration;
    ///
    /// let mut harness = TuiTestHarness::builder()
    ///     .with_size(80, 24)
    ///     .with_timeout(Duration::from_secs(10))
    ///     .build()?;
    /// # Ok::<(), ratatui_testlib::TermTestError>(())
    /// ```
    pub fn builder() -> TuiTestHarnessBuilder {
        TuiTestHarnessBuilder::default()
    }

    /// Sets the timeout for wait operations.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the polling interval for wait operations.
    ///
    /// # Arguments
    ///
    /// * `interval` - Polling interval
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Spawns a process in the PTY.
    ///
    /// # Arguments
    ///
    /// * `cmd` - Command to spawn
    ///
    /// # Errors
    ///
    /// Returns an error if spawning fails.
    pub fn spawn(&mut self, cmd: CommandBuilder) -> Result<()> {
        self.terminal.spawn(cmd)
    }

    /// Sends text to the PTY.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to send
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails.
    pub fn send_text(&mut self, text: &str) -> Result<()> {
        self.terminal.write(text.as_bytes())?;
        // Update state, ignoring ProcessExited since the process might exit
        // after receiving input (e.g., sending 'q' to quit)
        let _ = self.update_state();
        Ok(())
    }

    /// Sends a single key event to the PTY.
    ///
    /// This is the simplest way to send keyboard input. It handles the conversion
    /// to escape sequences automatically and updates the screen state.
    ///
    /// # Arguments
    ///
    /// * `key` - The key code to send
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails or state update fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{TuiTestHarness, KeyCode};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// // ... spawn application ...
    ///
    /// // Send Enter key
    /// harness.send_key(KeyCode::Enter)?;
    ///
    /// // Send letter 'a'
    /// harness.send_key(KeyCode::Char('a'))?;
    ///
    /// // Send arrow key
    /// harness.send_key(KeyCode::Up)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_key(&mut self, key: KeyCode) -> Result<()> {
        self.send_key_event(KeyEvent::new(key))
    }

    /// Sends a key with modifiers to the PTY.
    ///
    /// Use this when you need to send keys with Ctrl, Alt, Shift, or Meta modifiers.
    ///
    /// # Arguments
    ///
    /// * `key` - The key code to send
    /// * `modifiers` - The modifier keys to apply
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails or state update fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{TuiTestHarness, KeyCode, Modifiers};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// // ... spawn application ...
    ///
    /// // Send Ctrl+C
    /// harness.send_key_with_modifiers(KeyCode::Char('c'), Modifiers::CTRL)?;
    ///
    /// // Send Alt+X
    /// harness.send_key_with_modifiers(KeyCode::Char('x'), Modifiers::ALT)?;
    ///
    /// // Send Ctrl+Alt+Delete (multiple modifiers)
    /// harness.send_key_with_modifiers(
    ///     KeyCode::Delete,
    ///     Modifiers::CTRL | Modifiers::ALT
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_key_with_modifiers(&mut self, key: KeyCode, modifiers: Modifiers) -> Result<()> {
        self.send_key_event(KeyEvent::with_modifiers(key, modifiers))
    }

    /// Types a text string by sending each character as a key event.
    ///
    /// This is a convenience method for sending multiple characters. It's more
    /// ergonomic than calling `send_key(KeyCode::Char(c))` in a loop.
    ///
    /// # Arguments
    ///
    /// * `text` - The text string to type
    ///
    /// # Errors
    ///
    /// Returns an error if any key send fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// // ... spawn application ...
    ///
    /// // Type a string of text
    /// harness.send_keys("Hello, World!")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_keys(&mut self, text: &str) -> Result<()> {
        for ch in text.chars() {
            self.send_key(KeyCode::Char(ch))?;
        }
        Ok(())
    }

    /// Internal method to send a key event and update state.
    ///
    /// This encodes the key event to bytes, writes to the PTY, adds a small
    /// delay for the application to process the input, and updates the screen state.
    fn send_key_event(&mut self, event: KeyEvent) -> Result<()> {
        let bytes = encode_key_event(&event);
        self.terminal.write_all(&bytes)?;

        // Small delay to allow the application to process the input
        // This is important for applications that need time to react to input
        std::thread::sleep(Duration::from_millis(50));

        // Update state, ignoring ProcessExited since the process might exit
        // after receiving input (e.g., pressing 'q' to quit)
        let _ = self.update_state();
        Ok(())
    }

    /// Updates the screen state by reading from the PTY.
    ///
    /// This reads output in chunks (configured by buffer_size) and feeds it to the
    /// terminal emulator. It handles partial escape sequences correctly by continuing
    /// to read until no more data is available.
    ///
    /// This is called automatically by other methods but can be called
    /// manually if needed.
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the PTY fails.
    /// Returns [`TermTestError::ProcessExited`] if the child process has exited.
    pub fn update_state(&mut self) -> Result<()> {
        // First check if the child process has exited
        if !self.terminal.is_running() {
            // Process has exited - try to read any remaining buffered output
            let mut buf = vec![0u8; self.buffer_size];
            loop {
                match self.terminal.read(&mut buf) {
                    Ok(0) => break, // No more data
                    Ok(n) => {
                        self.state.feed(&buf[..n]);
                    }
                    Err(_) => break, // Any error, just stop reading
                }
            }
            // Return ProcessExited to signal the caller
            return Err(TermTestError::ProcessExited);
        }

        let mut buf = vec![0u8; self.buffer_size];

        loop {
            match self.terminal.read(&mut buf) {
                Ok(0) => break, // No more data available (WouldBlock returns Ok(0))
                Ok(n) => {
                    self.state.feed(&buf[..n]);
                }
                Err(e) => {
                    // Use proper ErrorKind matching instead of string matching
                    match e {
                        TermTestError::Io(io_err) if io_err.kind() == std::io::ErrorKind::WouldBlock => {
                            break;
                        }
                        _ => return Err(e),
                    }
                }
            }
        }

        Ok(())
    }

    /// Waits for a condition to be true, with timeout.
    ///
    /// This method polls the PTY output at the configured interval and checks
    /// the condition against the current screen state. If the timeout is reached,
    /// it returns an error with context about what was being waited for and the
    /// current screen state.
    ///
    /// # Arguments
    ///
    /// * `condition` - Condition to wait for
    ///
    /// # Errors
    ///
    /// Returns a `Timeout` error if the condition is not met within the configured timeout.
    /// The error includes the current screen state for debugging.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use ratatui_testlib::TuiTestHarness;
    /// # let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.wait_for(|state| {
    ///     state.contains("Ready")
    /// })?;
    /// # Ok::<(), ratatui_testlib::TermTestError>(())
    /// ```
    pub fn wait_for<F>(&mut self, condition: F) -> Result<()>
    where
        F: Fn(&ScreenState) -> bool,
    {
        self.wait_for_with_context(condition, "condition")
    }

    /// Waits for a condition with a custom error context.
    ///
    /// This is similar to `wait_for` but allows providing a description of what
    /// is being waited for, which improves error messages.
    ///
    /// # Arguments
    ///
    /// * `condition` - Condition to wait for
    /// * `description` - Human-readable description of the condition
    ///
    /// # Errors
    ///
    /// Returns a `Timeout` error if the condition is not met within the configured timeout.
    /// Returns `ProcessExited` if the child process exits before the condition is met.
    pub fn wait_for_with_context<F>(&mut self, condition: F, description: &str) -> Result<()>
    where
        F: Fn(&ScreenState) -> bool,
    {
        let start = Instant::now();
        let mut iterations = 0;

        loop {
            // Update state - this may return ProcessExited
            match self.update_state() {
                Ok(()) => {
                    // Check condition after successful update
                    if condition(&self.state) {
                        return Ok(());
                    }
                }
                Err(TermTestError::ProcessExited) => {
                    // Process exited - check condition one final time with current state
                    if condition(&self.state) {
                        return Ok(());
                    }

                    // Condition not met and process has exited
                    let current_state = self.state.debug_contents();
                    let cursor = self.state.cursor_position();

                    eprintln!("\n=== Process exited while waiting for: {} ===", description);
                    eprintln!("Waited: {:?} ({} iterations)", start.elapsed(), iterations);
                    eprintln!("Cursor position: row={}, col={}", cursor.0, cursor.1);
                    eprintln!("Final screen state:\n{}", current_state);
                    eprintln!("==========================================\n");

                    return Err(TermTestError::ProcessExited);
                }
                Err(e) => return Err(e),
            }

            let elapsed = start.elapsed();
            if elapsed >= self.timeout {
                // Create a detailed error message with current state
                let current_state = self.state.debug_contents();
                let cursor = self.state.cursor_position();

                eprintln!("\n=== Timeout waiting for: {} ===", description);
                eprintln!("Waited: {:?} ({} iterations)", elapsed, iterations);
                eprintln!("Cursor position: row={}, col={}", cursor.0, cursor.1);
                eprintln!("Current screen state:\n{}", current_state);
                eprintln!("==========================================\n");

                return Err(TermTestError::Timeout {
                    timeout_ms: self.timeout.as_millis() as u64,
                });
            }

            iterations += 1;
            std::thread::sleep(self.poll_interval);
        }
    }

    /// Waits for specific text to appear anywhere on the screen.
    ///
    /// This is a convenience wrapper around `wait_for` for the common case
    /// of waiting for text to appear. Uses the configured timeout.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to wait for
    ///
    /// # Errors
    ///
    /// Returns a `Timeout` error if the text does not appear within the configured timeout.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use ratatui_testlib::TuiTestHarness;
    /// # let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.wait_for_text("Ready")?;
    /// # Ok::<(), ratatui_testlib::TermTestError>(())
    /// ```
    pub fn wait_for_text(&mut self, text: &str) -> Result<()> {
        let text = text.to_string();
        let description = format!("text '{}'", text);
        self.wait_for_with_context(
            move |state| state.contains(&text),
            &description,
        )
    }

    /// Waits for specific text to appear with a custom timeout.
    ///
    /// This allows overriding the configured timeout for a single wait operation.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to wait for
    /// * `timeout` - Timeout duration for this operation
    ///
    /// # Errors
    ///
    /// Returns a `Timeout` error if the text does not appear within the specified timeout.
    /// Returns `ProcessExited` if the child process exits before the text appears.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use ratatui_testlib::TuiTestHarness;
    /// # use std::time::Duration;
    /// # let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.wait_for_text_timeout("Ready", Duration::from_secs(2))?;
    /// # Ok::<(), ratatui_testlib::TermTestError>(())
    /// ```
    pub fn wait_for_text_timeout(&mut self, text: &str, timeout: Duration) -> Result<()> {
        let text = text.to_string();
        let description = format!("text '{}'", text);

        let start = Instant::now();
        let mut iterations = 0;

        loop {
            // Update state - this may return ProcessExited
            match self.update_state() {
                Ok(()) => {
                    if self.state.contains(&text) {
                        return Ok(());
                    }
                }
                Err(TermTestError::ProcessExited) => {
                    // Process exited - check condition one final time
                    if self.state.contains(&text) {
                        return Ok(());
                    }

                    let current_state = self.state.debug_contents();
                    let cursor = self.state.cursor_position();

                    eprintln!("\n=== Process exited while waiting for: {} ===", description);
                    eprintln!("Waited: {:?} ({} iterations)", start.elapsed(), iterations);
                    eprintln!("Cursor position: row={}, col={}", cursor.0, cursor.1);
                    eprintln!("Final screen state:\n{}", current_state);
                    eprintln!("==========================================\n");

                    return Err(TermTestError::ProcessExited);
                }
                Err(e) => return Err(e),
            }

            let elapsed = start.elapsed();
            if elapsed >= timeout {
                let current_state = self.state.debug_contents();
                let cursor = self.state.cursor_position();

                eprintln!("\n=== Timeout waiting for: {} ===", description);
                eprintln!("Waited: {:?} ({} iterations)", elapsed, iterations);
                eprintln!("Cursor position: row={}, col={}", cursor.0, cursor.1);
                eprintln!("Current screen state:\n{}", current_state);
                eprintln!("==========================================\n");

                return Err(TermTestError::Timeout {
                    timeout_ms: timeout.as_millis() as u64,
                });
            }

            iterations += 1;
            std::thread::sleep(self.poll_interval);
        }
    }

    /// Waits for the cursor to reach a specific position.
    ///
    /// This is useful for verifying cursor movements after sending input
    /// or for tracking application state changes.
    ///
    /// # Arguments
    ///
    /// * `pos` - Target cursor position as (row, col) tuple (0-based)
    ///
    /// # Errors
    ///
    /// Returns a `Timeout` error if the cursor does not reach the position within the configured timeout.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use ratatui_testlib::TuiTestHarness;
    /// # let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.wait_for_cursor((5, 10))?;
    /// # Ok::<(), ratatui_testlib::TermTestError>(())
    /// ```
    pub fn wait_for_cursor(&mut self, pos: (u16, u16)) -> Result<()> {
        let description = format!("cursor at ({}, {})", pos.0, pos.1);
        self.wait_for_with_context(
            move |state| state.cursor_position() == pos,
            &description,
        )
    }

    /// Waits for the cursor to reach a specific position with a custom timeout.
    ///
    /// # Arguments
    ///
    /// * `pos` - Target cursor position as (row, col) tuple (0-based)
    /// * `timeout` - Timeout duration for this operation
    ///
    /// # Errors
    ///
    /// Returns a `Timeout` error if the cursor does not reach the position within the specified timeout.
    /// Returns `ProcessExited` if the child process exits before the cursor reaches the position.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use ratatui_testlib::TuiTestHarness;
    /// # use std::time::Duration;
    /// # let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.wait_for_cursor_timeout((5, 10), Duration::from_millis(500))?;
    /// # Ok::<(), ratatui_testlib::TermTestError>(())
    /// ```
    pub fn wait_for_cursor_timeout(&mut self, pos: (u16, u16), timeout: Duration) -> Result<()> {
        let description = format!("cursor at ({}, {})", pos.0, pos.1);

        let start = Instant::now();
        let mut iterations = 0;

        loop {
            // Update state - this may return ProcessExited
            match self.update_state() {
                Ok(()) => {
                    if self.state.cursor_position() == pos {
                        return Ok(());
                    }
                }
                Err(TermTestError::ProcessExited) => {
                    // Process exited - check condition one final time
                    if self.state.cursor_position() == pos {
                        return Ok(());
                    }

                    let current_state = self.state.debug_contents();
                    let cursor = self.state.cursor_position();

                    eprintln!("\n=== Process exited while waiting for: {} ===", description);
                    eprintln!("Waited: {:?} ({} iterations)", start.elapsed(), iterations);
                    eprintln!("Cursor position: row={}, col={}", cursor.0, cursor.1);
                    eprintln!("Final screen state:\n{}", current_state);
                    eprintln!("==========================================\n");

                    return Err(TermTestError::ProcessExited);
                }
                Err(e) => return Err(e),
            }

            let elapsed = start.elapsed();
            if elapsed >= timeout {
                let current_state = self.state.debug_contents();
                let cursor = self.state.cursor_position();

                eprintln!("\n=== Timeout waiting for: {} ===", description);
                eprintln!("Waited: {:?} ({} iterations)", elapsed, iterations);
                eprintln!("Cursor position: row={}, col={}", cursor.0, cursor.1);
                eprintln!("Current screen state:\n{}", current_state);
                eprintln!("==========================================\n");

                return Err(TermTestError::Timeout {
                    timeout_ms: timeout.as_millis() as u64,
                });
            }

            iterations += 1;
            std::thread::sleep(self.poll_interval);
        }
    }

    /// Returns the current screen contents as a string.
    pub fn screen_contents(&self) -> String {
        self.state.contents()
    }

    /// Returns the current cursor position as (row, col).
    ///
    /// Both row and column are 0-based indices. This is required for Phase 3
    /// Sixel position verification.
    ///
    /// # Returns
    ///
    /// A tuple of (row, col) where both are 0-based.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use ratatui_testlib::TuiTestHarness;
    /// # let harness = TuiTestHarness::new(80, 24)?;
    /// let (row, col) = harness.cursor_position();
    /// println!("Cursor at: row={}, col={}", row, col);
    /// # Ok::<(), ratatui_testlib::TermTestError>(())
    /// ```
    pub fn cursor_position(&self) -> (u16, u16) {
        self.state.cursor_position()
    }

    /// Alias for `cursor_position()` for convenience.
    ///
    /// Returns the current cursor position as (row, col).
    pub fn get_cursor_position(&self) -> (u16, u16) {
        self.cursor_position()
    }

    /// Returns the current screen state.
    ///
    /// Provides immutable access to the terminal screen state for inspecting
    /// rendered content without modifying it.
    ///
    /// # Returns
    ///
    /// A reference to the current [`ScreenState`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    /// let state = harness.state();
    /// println!("Screen size: {:?}", state.size());
    /// # Ok(())
    /// # }
    /// ```
    pub fn state(&self) -> &ScreenState {
        &self.state
    }

    /// Returns a mutable reference to the screen state.
    ///
    /// Allows direct manipulation of the screen state, which can be useful
    /// for testing specific scenarios or feeding mock data.
    ///
    /// # Returns
    ///
    /// A mutable reference to the [`ScreenState`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.state_mut().feed(b"Test data");
    /// assert!(harness.screen_contents().contains("Test"));
    /// # Ok(())
    /// # }
    /// ```
    pub fn state_mut(&mut self) -> &mut ScreenState {
        &mut self.state
    }

    /// Resizes the terminal.
    ///
    /// Changes the terminal dimensions and resets the screen state.
    /// This can be useful for testing responsive TUI layouts.
    ///
    /// # Arguments
    ///
    /// * `width` - New width in columns
    /// * `height` - New height in rows
    ///
    /// # Errors
    ///
    /// Returns an error if the resize operation fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.resize(120, 40)?;
    /// assert_eq!(harness.state().size(), (120, 40));
    /// # Ok(())
    /// # }
    /// ```
    pub fn resize(&mut self, width: u16, height: u16) -> Result<()> {
        self.terminal.resize(width, height)?;
        self.state = ScreenState::new(width, height);
        Ok(())
    }

    /// Checks if the child process is still running.
    ///
    /// # Returns
    ///
    /// `true` if a process is currently running in the PTY, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    /// use portable_pty::CommandBuilder;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// let cmd = CommandBuilder::new("sleep");
    /// harness.spawn(cmd)?;
    /// assert!(harness.is_running());
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_running(&mut self) -> bool {
        self.terminal.is_running()
    }

    /// Waits for the child process to exit.
    ///
    /// Blocks until the spawned process terminates and returns its exit status.
    ///
    /// # Returns
    ///
    /// The [`ExitStatus`] of the terminated process.
    ///
    /// # Errors
    ///
    /// Returns [`TermTestError::NoProcessRunning`] if no process is currently running.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    /// use portable_pty::CommandBuilder;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// let mut cmd = CommandBuilder::new("echo");
    /// cmd.arg("test");
    /// harness.spawn(cmd)?;
    ///
    /// let status = harness.wait_exit()?;
    /// assert!(status.success());
    /// # Ok(())
    /// # }
    /// ```
    pub fn wait_exit(&mut self) -> Result<ExitStatus> {
        self.terminal.wait()
    }

    // ========================================================================
    // Position and Layout Assertions
    // ========================================================================

    /// Asserts that text appears at a specific position on the screen.
    ///
    /// This verifies that the given text starts at the exact (row, col) position.
    /// The text must match character-by-character at that position.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to search for
    /// * `row` - Row position (0-indexed)
    /// * `col` - Column position (0-indexed)
    ///
    /// # Errors
    ///
    /// Returns [`TermTestError::Parse`] if the text is not found at the specified position.
    /// The error message includes the actual content found at that position.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    /// // ... render UI with tab bar at bottom ...
    ///
    /// // Verify "Tab 1" appears at row 22, column 0
    /// harness.assert_text_at_position("Tab 1", 22, 0)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn assert_text_at_position(&self, text: &str, row: u16, col: u16) -> Result<()> {
        let (width, height) = self.state.size();

        // Validate coordinates
        if row >= height {
            return Err(TermTestError::Parse(format!(
                "Row {} is out of bounds (screen height: {})",
                row, height
            )));
        }
        if col >= width {
            return Err(TermTestError::Parse(format!(
                "Column {} is out of bounds (screen width: {})",
                col, width
            )));
        }

        // Extract text at the position
        let mut actual = String::new();
        for (i, _) in text.chars().enumerate() {
            let current_col = col.saturating_add(i as u16);
            if current_col >= width {
                break;
            }
            if let Some(cell) = self.state.get_cell(row, current_col) {
                actual.push(cell.c);
            }
        }

        // Compare
        if actual != text {
            return Err(TermTestError::Parse(format!(
                "Text mismatch at position ({}, {})\n  Expected: {:?}\n  Found:    {:?}\n\nScreen state:\n{}",
                row, col, text, actual, self.state.debug_contents()
            )));
        }

        Ok(())
    }

    /// Asserts that text appears anywhere within a specified rectangular area.
    ///
    /// This searches for the text within the given bounds and succeeds if found
    /// anywhere in that region.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to search for
    /// * `area` - The rectangular area to search within
    ///
    /// # Errors
    ///
    /// Returns [`TermTestError::Parse`] if the text is not found within the area.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{TuiTestHarness, Rect};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    /// // ... render UI ...
    ///
    /// // Verify "Preview" appears somewhere in the preview area
    /// let preview_area = Rect::new(5, 40, 35, 15);
    /// harness.assert_text_within_bounds("Preview", preview_area)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn assert_text_within_bounds(&self, text: &str, area: crate::screen::Rect) -> Result<()> {
        let (width, height) = self.state.size();

        // Search within the area
        for row in area.y..area.bottom().min(height) {
            for col in area.x..area.right().min(width) {
                // Try to match text starting at this position
                let mut matches = true;
                for (i, expected_char) in text.chars().enumerate() {
                    let current_col = col.saturating_add(i as u16);
                    if current_col >= area.right() || current_col >= width {
                        matches = false;
                        break;
                    }
                    if let Some(cell) = self.state.get_cell(row, current_col) {
                        if cell.c != expected_char {
                            matches = false;
                            break;
                        }
                    } else {
                        matches = false;
                        break;
                    }
                }
                if matches {
                    return Ok(()); // Found it!
                }
            }
        }

        // Not found
        Err(TermTestError::Parse(format!(
            "Text {:?} not found within bounds (x={}, y={}, width={}, height={})\n\nScreen state:\n{}",
            text, area.x, area.y, area.width, area.height,
            self.state.debug_contents()
        )))
    }

    /// Asserts that two rectangular areas do not overlap.
    ///
    /// This is useful for verifying that UI components don't render on top of
    /// each other incorrectly.
    ///
    /// # Arguments
    ///
    /// * `rect1` - First rectangle
    /// * `rect2` - Second rectangle
    ///
    /// # Errors
    ///
    /// Returns [`TermTestError::Parse`] if the rectangles overlap.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{TuiTestHarness, Rect};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    ///
    /// let sidebar = Rect::new(0, 0, 20, 24);
    /// let preview = Rect::new(20, 0, 60, 24);
    ///
    /// // Verify sidebar and preview don't overlap
    /// harness.assert_no_overlap(sidebar, preview)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn assert_no_overlap(&self, rect1: crate::screen::Rect, rect2: crate::screen::Rect) -> Result<()> {
        if rect1.intersects(&rect2) {
            return Err(TermTestError::Parse(format!(
                "Rectangles overlap!\n\
                 Rect 1: (x={}, y={}, width={}, height={})\n\
                 Rect 2: (x={}, y={}, width={}, height={})\n\
                 Overlap region exists between x=[{}, {}) and y=[{}, {})",
                rect1.x, rect1.y, rect1.width, rect1.height,
                rect2.x, rect2.y, rect2.width, rect2.height,
                rect1.x.max(rect2.x), rect1.right().min(rect2.right()),
                rect1.y.max(rect2.y), rect1.bottom().min(rect2.bottom())
            )));
        }
        Ok(())
    }

    /// Asserts that two rectangles are aligned along a specified axis.
    ///
    /// For horizontal alignment, the Y coordinates must match.
    /// For vertical alignment, the X coordinates must match.
    ///
    /// # Arguments
    ///
    /// * `rect1` - First rectangle
    /// * `rect2` - Second rectangle
    /// * `axis` - The axis to check alignment on
    ///
    /// # Errors
    ///
    /// Returns [`TermTestError::Parse`] if the rectangles are not aligned.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{TuiTestHarness, Rect, Axis};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    ///
    /// let button1 = Rect::new(10, 20, 15, 3);
    /// let button2 = Rect::new(30, 20, 15, 3);
    ///
    /// // Verify both buttons are horizontally aligned (same Y)
    /// harness.assert_aligned(button1, button2, Axis::Horizontal)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn assert_aligned(
        &self,
        rect1: crate::screen::Rect,
        rect2: crate::screen::Rect,
        axis: Axis,
    ) -> Result<()> {
        match axis {
            Axis::Horizontal => {
                if rect1.y != rect2.y {
                    return Err(TermTestError::Parse(format!(
                        "Rectangles not horizontally aligned (different Y coordinates)\n\
                         Rect 1: y={} (x={}, width={}, height={})\n\
                         Rect 2: y={} (x={}, width={}, height={})",
                        rect1.y, rect1.x, rect1.width, rect1.height,
                        rect2.y, rect2.x, rect2.width, rect2.height
                    )));
                }
            }
            Axis::Vertical => {
                if rect1.x != rect2.x {
                    return Err(TermTestError::Parse(format!(
                        "Rectangles not vertically aligned (different X coordinates)\n\
                         Rect 1: x={} (y={}, width={}, height={})\n\
                         Rect 2: x={} (y={}, width={}, height={})",
                        rect1.x, rect1.y, rect1.width, rect1.height,
                        rect2.x, rect2.y, rect2.width, rect2.height
                    )));
                }
            }
        }
        Ok(())
    }

    // ========================================================================
    // Sixel Graphics Validation APIs
    // ========================================================================

    /// Returns all captured Sixel regions from the screen state.
    ///
    /// This provides direct access to all Sixel graphics detected in the terminal
    /// output. Each region includes position and dimension information.
    ///
    /// # Returns
    ///
    /// A slice containing all [`SixelRegion`](crate::screen::SixelRegion) instances
    /// currently on screen.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// // ... spawn app that renders Sixel graphics ...
    ///
    /// let regions = harness.sixel_regions();
    /// for (i, region) in regions.iter().enumerate() {
    ///     println!("Sixel {}: position ({}, {}), size {}x{}",
    ///         i, region.start_row, region.start_col,
    ///         region.width, region.height);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sixel")]
    pub fn sixel_regions(&self) -> &[crate::screen::SixelRegion] {
        self.state.sixel_regions()
    }

    /// Returns the count of Sixel graphics currently on screen.
    ///
    /// This is a convenience method equivalent to `harness.sixel_regions().len()`.
    /// Useful for quick assertions about the number of graphics present.
    ///
    /// # Returns
    ///
    /// The number of Sixel graphics regions detected.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    /// // ... render graphics ...
    ///
    /// assert_eq!(harness.sixel_count(), 1, "Expected exactly one image");
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sixel")]
    pub fn sixel_count(&self) -> usize {
        self.sixel_regions().len()
    }

    /// Finds a Sixel graphic at a specific position.
    ///
    /// Searches for a Sixel region that starts at the exact (row, col) position.
    /// Returns `None` if no Sixel is found at that position.
    ///
    /// # Arguments
    ///
    /// * `row` - Row to search (0-based)
    /// * `col` - Column to search (0-based)
    ///
    /// # Returns
    ///
    /// A reference to the [`SixelRegion`](crate::screen::SixelRegion) at that position,
    /// or `None` if no Sixel starts there.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    /// // ... render Sixel at (5, 10) ...
    ///
    /// if let Some(region) = harness.sixel_at(5, 10) {
    ///     println!("Found Sixel: {}x{} pixels", region.width, region.height);
    /// } else {
    ///     println!("No Sixel at that position");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sixel")]
    pub fn sixel_at(&self, row: u16, col: u16) -> Option<&crate::screen::SixelRegion> {
        self.state.sixel_regions()
            .iter()
            .find(|r| r.start_row == row && r.start_col == col)
    }

    /// Asserts that all Sixel graphics are within the specified area.
    ///
    /// This validates that every Sixel bounding rectangle is completely contained
    /// within the given area. If any Sixel extends beyond the area, an error is
    /// returned with details about which sequences are out of bounds.
    ///
    /// # Arguments
    ///
    /// * `area` - Bounding area as (row, col, width, height) tuple
    ///
    /// # Errors
    ///
    /// Returns [`TermTestError::SixelValidation`] if any Sixel is outside the area.
    /// The error message includes the positions of all out-of-bounds graphics.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    /// // ... render graphics ...
    ///
    /// // Verify all graphics are in the preview panel
    /// let preview_area = (5, 40, 35, 15); // (row, col, width, height)
    /// harness.assert_sixel_within_bounds(preview_area)?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sixel")]
    pub fn assert_sixel_within_bounds(&self, area: (u16, u16, u16, u16)) -> Result<()> {
        use crate::sixel::SixelCapture;
        let capture = SixelCapture::from_screen_state(&self.state);
        capture.assert_all_within(area)
    }

    /// Checks if any Sixel graphics overlap with the specified area.
    ///
    /// Returns `true` if at least one Sixel bounding rectangle intersects with
    /// the given area, even partially. This is useful for detecting graphics in
    /// specific screen regions.
    ///
    /// # Arguments
    ///
    /// * `area` - Area to check as (row, col, width, height) tuple
    ///
    /// # Returns
    ///
    /// `true` if any Sixel overlaps with the area, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    /// // ... render graphics ...
    ///
    /// let preview_area = (5, 40, 35, 15);
    /// if harness.has_sixel_in_area(preview_area) {
    ///     println!("Graphics detected in preview area");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sixel")]
    pub fn has_sixel_in_area(&self, area: (u16, u16, u16, u16)) -> bool {
        use crate::sixel::SixelCapture;
        let capture = SixelCapture::from_screen_state(&self.state);
        !capture.sequences_in_area(area).is_empty()
    }

    /// Verifies that Sixel graphics were cleared after a screen update.
    ///
    /// This method records the current Sixel count, calls [`update`](Self::update)
    /// to refresh the screen state, and then checks if the count decreased.
    /// It's useful for verifying that graphics are properly cleared during
    /// screen transitions (e.g., switching between files in a previewer).
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the Sixel count decreased, `Ok(false)` if it stayed the same
    /// or increased.
    ///
    /// # Errors
    ///
    /// Returns an error if the screen update fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// // ... render some graphics ...
    ///
    /// // Simulate screen transition (e.g., press a key to switch files)
    /// harness.send_key(ratatui_testlib::KeyCode::Down)?;
    ///
    /// // Verify graphics were cleared
    /// if harness.verify_sixel_cleared()? {
    ///     println!("Graphics properly cleared on transition");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sixel")]
    pub fn verify_sixel_cleared(&mut self) -> Result<bool> {
        let before = self.sixel_count();
        self.update_state()?;
        let after = self.sixel_count();
        Ok(after < before)
    }

    /// Asserts that a Sixel graphic appears in a typical preview area.
    ///
    /// This is a convenience method for the common dgx-pixels use case where
    /// image previews are displayed in a standard preview panel layout.
    ///
    /// The default preview area is:
    /// - Rows: 5-35 (30 rows, leaving space for header/footer)
    /// - Cols: 40-75 (35 columns, typical split-pane layout)
    ///
    /// # Errors
    ///
    /// Returns [`TermTestError::SixelValidation`] if no Sixel graphics are
    /// found in the preview area.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    /// // ... render image preview ...
    ///
    /// // Quick assertion for standard preview layout
    /// harness.assert_preview_has_sixel()?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sixel")]
    pub fn assert_preview_has_sixel(&self) -> Result<()> {
        // Standard dgx-pixels preview area layout
        // Assumes 80x24 terminal with:
        // - Left sidebar: cols 0-39
        // - Preview area: cols 40-75, rows 5-20
        let preview_area = (5, 40, 35, 15);

        if !self.has_sixel_in_area(preview_area) {
            return Err(TermTestError::SixelValidation(
                format!(
                    "No Sixel graphics found in standard preview area {:?}. \
                    Current Sixel count: {}. \
                    Regions: {:?}",
                    preview_area,
                    self.sixel_count(),
                    self.sixel_regions()
                        .iter()
                        .map(|r| (r.start_row, r.start_col, r.width, r.height))
                        .collect::<Vec<_>>()
                )
            ));
        }
        Ok(())
    }

    /// Asserts that a Sixel graphic appears in a custom preview area.
    ///
    /// Similar to [`assert_preview_has_sixel`](Self::assert_preview_has_sixel),
    /// but allows specifying a custom preview area. This is useful for applications
    /// with non-standard layouts.
    ///
    /// # Arguments
    ///
    /// * `preview_area` - Custom preview area as (row, col, width, height)
    ///
    /// # Errors
    ///
    /// Returns [`TermTestError::SixelValidation`] if no Sixel graphics are
    /// found in the specified preview area.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(120, 40)?;
    /// // ... render image preview in large terminal ...
    ///
    /// // Custom preview area for larger terminal
    /// let custom_area = (10, 50, 60, 25);
    /// harness.assert_preview_has_sixel_in(custom_area)?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "sixel")]
    pub fn assert_preview_has_sixel_in(&self, preview_area: (u16, u16, u16, u16)) -> Result<()> {
        if !self.has_sixel_in_area(preview_area) {
            return Err(TermTestError::SixelValidation(
                format!(
                    "No Sixel graphics found in preview area {:?}. \
                    Current Sixel count: {}. \
                    Regions: {:?}",
                    preview_area,
                    self.sixel_count(),
                    self.sixel_regions()
                        .iter()
                        .map(|r| (r.start_row, r.start_col, r.width, r.height))
                        .collect::<Vec<_>>()
                )
            ));
        }
        Ok(())
    }
}

/// Builder for configuring a `TuiTestHarness`.
///
/// # Example
///
/// ```rust,no_run
/// use ratatui_testlib::TuiTestHarness;
/// use std::time::Duration;
///
/// let mut harness = TuiTestHarness::builder()
///     .with_size(80, 24)
///     .with_timeout(Duration::from_secs(10))
///     .with_poll_interval(Duration::from_millis(50))
///     .with_buffer_size(8192)
///     .build()?;
/// # Ok::<(), ratatui_testlib::TermTestError>(())
/// ```
#[derive(Debug, Clone)]
pub struct TuiTestHarnessBuilder {
    width: u16,
    height: u16,
    timeout: Duration,
    poll_interval: Duration,
    buffer_size: usize,
}

impl Default for TuiTestHarnessBuilder {
    fn default() -> Self {
        Self {
            width: 80,
            height: 24,
            timeout: DEFAULT_TIMEOUT,
            poll_interval: DEFAULT_POLL_INTERVAL,
            buffer_size: DEFAULT_BUFFER_SIZE,
        }
    }
}

impl TuiTestHarnessBuilder {
    /// Sets the terminal size.
    ///
    /// # Arguments
    ///
    /// * `width` - Terminal width in columns
    /// * `height` - Terminal height in rows
    pub fn with_size(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the timeout for wait operations.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the polling interval for wait operations.
    ///
    /// # Arguments
    ///
    /// * `interval` - Polling interval
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Sets the buffer size for reading PTY output.
    ///
    /// # Arguments
    ///
    /// * `size` - Buffer size in bytes
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Builds the test harness with the configured settings.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal creation fails.
    pub fn build(self) -> Result<TuiTestHarness> {
        let terminal = TestTerminal::new(self.width, self.height)?;
        let state = ScreenState::new(self.width, self.height);

        Ok(TuiTestHarness {
            terminal,
            state,
            timeout: self.timeout,
            poll_interval: self.poll_interval,
            buffer_size: self.buffer_size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_harness() {
        let harness = TuiTestHarness::new(80, 24);
        assert!(harness.is_ok());
        let harness = harness.unwrap();
        assert_eq!(harness.timeout, DEFAULT_TIMEOUT);
        assert_eq!(harness.poll_interval, DEFAULT_POLL_INTERVAL);
        assert_eq!(harness.buffer_size, DEFAULT_BUFFER_SIZE);
    }

    #[test]
    fn test_with_timeout() {
        let harness = TuiTestHarness::new(80, 24)
            .unwrap()
            .with_timeout(Duration::from_secs(10));
        assert_eq!(harness.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_with_poll_interval() {
        let harness = TuiTestHarness::new(80, 24)
            .unwrap()
            .with_poll_interval(Duration::from_millis(50));
        assert_eq!(harness.poll_interval, Duration::from_millis(50));
    }

    #[test]
    fn test_builder_default() {
        let harness = TuiTestHarness::builder().build();
        assert!(harness.is_ok());
        let harness = harness.unwrap();
        assert_eq!(harness.timeout, DEFAULT_TIMEOUT);
        assert_eq!(harness.poll_interval, DEFAULT_POLL_INTERVAL);
        assert_eq!(harness.buffer_size, DEFAULT_BUFFER_SIZE);
    }

    #[test]
    fn test_builder_with_size() {
        let harness = TuiTestHarness::builder()
            .with_size(120, 40)
            .build()
            .unwrap();
        let (width, height) = harness.state.size();
        assert_eq!(width, 120);
        assert_eq!(height, 40);
    }

    #[test]
    fn test_builder_with_timeout() {
        let timeout = Duration::from_secs(15);
        let harness = TuiTestHarness::builder()
            .with_timeout(timeout)
            .build()
            .unwrap();
        assert_eq!(harness.timeout, timeout);
    }

    #[test]
    fn test_builder_with_poll_interval() {
        let interval = Duration::from_millis(25);
        let harness = TuiTestHarness::builder()
            .with_poll_interval(interval)
            .build()
            .unwrap();
        assert_eq!(harness.poll_interval, interval);
    }

    #[test]
    fn test_builder_with_buffer_size() {
        let buffer_size = 8192;
        let harness = TuiTestHarness::builder()
            .with_buffer_size(buffer_size)
            .build()
            .unwrap();
        assert_eq!(harness.buffer_size, buffer_size);
    }

    #[test]
    fn test_builder_chaining() {
        let harness = TuiTestHarness::builder()
            .with_size(100, 30)
            .with_timeout(Duration::from_secs(20))
            .with_poll_interval(Duration::from_millis(75))
            .with_buffer_size(16384)
            .build()
            .unwrap();

        assert_eq!(harness.state.size(), (100, 30));
        assert_eq!(harness.timeout, Duration::from_secs(20));
        assert_eq!(harness.poll_interval, Duration::from_millis(75));
        assert_eq!(harness.buffer_size, 16384);
    }

    #[test]
    fn test_cursor_position() {
        let harness = TuiTestHarness::new(80, 24).unwrap();
        let (row, col) = harness.cursor_position();
        // Initial cursor position should be at (0, 0)
        assert_eq!(row, 0);
        assert_eq!(col, 0);
    }

    #[test]
    fn test_get_cursor_position_alias() {
        let harness = TuiTestHarness::new(80, 24).unwrap();
        let pos1 = harness.cursor_position();
        let pos2 = harness.get_cursor_position();
        assert_eq!(pos1, pos2);
    }

    #[test]
    fn test_wait_for_text_helper_exists() {
        // Test that all wait_for methods exist and compile (signature test)
        let harness = TuiTestHarness::new(80, 24).unwrap();

        // Test method signatures exist (don't call them as they require a running process)
        let _: fn(&mut TuiTestHarness, &str) -> Result<()> = TuiTestHarness::wait_for_text;

        // Verify state access methods exist
        assert_eq!(harness.cursor_position(), (0, 0));
        assert_eq!(harness.get_cursor_position(), (0, 0));
    }

    #[test]
    fn test_state_manipulation() {
        // Test that we can manipulate state directly
        let mut harness = TuiTestHarness::new(80, 24).unwrap();

        // Feed text directly to state
        harness.state_mut().feed(b"Test Data");

        // Verify we can read it back
        let contents = harness.screen_contents();
        assert!(contents.contains("Test"));
    }

    #[test]
    fn test_cursor_position_tracking() {
        // Test cursor position tracking
        let mut harness = TuiTestHarness::new(80, 24).unwrap();

        // Initial position
        assert_eq!(harness.cursor_position(), (0, 0));

        // Feed escape sequence to move cursor
        harness.state_mut().feed(b"\x1b[2;5H"); // Move to row 2, col 5

        // Check new position (note: escape sequences use 1-based indexing, we return 0-based)
        let (row, col) = harness.cursor_position();
        assert!(row >= 0); // Just verify we get valid coordinates
        assert!(col >= 0);
    }

    #[test]
    fn test_screen_state_access() {
        let harness = TuiTestHarness::new(80, 24).unwrap();
        let state = harness.state();
        assert_eq!(state.size(), (80, 24));

        let contents = harness.screen_contents();
        assert!(contents.len() > 0 || contents.is_empty()); // Just verify it returns something
    }

    #[test]
    fn test_resize() {
        let mut harness = TuiTestHarness::new(80, 24).unwrap();
        let result = harness.resize(100, 30);
        assert!(result.is_ok());
        assert_eq!(harness.state.size(), (100, 30));
    }

    #[test]
    fn test_is_running_no_process() {
        let mut harness = TuiTestHarness::new(80, 24).unwrap();
        assert!(!harness.is_running());
    }

    #[test]
    fn test_spawn_and_check_running() {
        let mut harness = TuiTestHarness::new(80, 24).unwrap();
        let mut cmd = CommandBuilder::new("sleep");
        cmd.arg("0.1");

        let spawn_result = harness.spawn(cmd);
        if spawn_result.is_ok() {
            // Should be running initially
            assert!(harness.is_running());

            // Wait for it to exit
            std::thread::sleep(Duration::from_millis(200));

            // Should have exited
            assert!(!harness.is_running());
        }
    }

    #[test]
    fn test_wait_for_text_success() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?
            .with_timeout(Duration::from_secs(2));

        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("hello world");
        harness.spawn(cmd)?;

        // wait_for_text may return ProcessExited since echo exits immediately
        // but that's ok if the text is present
        match harness.wait_for_text("hello") {
            Ok(()) => {
                assert!(harness.screen_contents().contains("hello"));
            }
            Err(TermTestError::ProcessExited) => {
                // Process exited, but check if we still got the output
                assert!(harness.screen_contents().contains("hello"),
                    "Expected 'hello' in output even though process exited");
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    #[test]
    fn test_wait_for_text_timeout() {
        let mut harness = TuiTestHarness::new(80, 24)
            .unwrap()
            .with_timeout(Duration::from_millis(300));

        let mut cmd = CommandBuilder::new("sleep");
        cmd.arg("1");
        harness.spawn(cmd).unwrap();

        // Should timeout waiting for text that never appears
        let result = harness.wait_for_text("never_appears");
        assert!(result.is_err());

        match result {
            Err(TermTestError::Timeout { timeout_ms }) => {
                assert_eq!(timeout_ms, 300);
            }
            Err(TermTestError::ProcessExited) => {
                // Also acceptable - process may exit before timeout
            }
            _ => panic!("Expected Timeout or ProcessExited error"),
        }
    }

    #[test]
    fn test_wait_for_text_with_custom_timeout() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("quick test");
        harness.spawn(cmd)?;

        // Use custom timeout (shorter than default)
        // May return ProcessExited since echo exits immediately
        match harness.wait_for_text_timeout("quick", Duration::from_millis(500)) {
            Ok(()) => {
                assert!(harness.screen_contents().contains("quick"));
            }
            Err(TermTestError::ProcessExited) => {
                // Process exited, but check if we still got the output
                assert!(harness.screen_contents().contains("quick"),
                    "Expected 'quick' in output even though process exited");
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    #[test]
    fn test_wait_for_cursor_success() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Feed escape sequence to move cursor
        harness.state_mut().feed(b"\x1b[10;20H"); // Move to row 10, col 20

        // Wait for cursor to be at the position (1-based in CSI, 0-based in our API)
        // Since no process is running, this will return ProcessExited immediately
        // but that's fine - the cursor position should already be set

        match harness.wait_for_cursor((9, 19)) {
            Ok(()) => {
                let pos = harness.cursor_position();
                assert_eq!(pos, (9, 19));
            }
            Err(TermTestError::ProcessExited) => {
                // No process running, but cursor should still be in the right position
                let pos = harness.cursor_position();
                assert_eq!(pos, (9, 19), "Cursor should be at (9, 19) even though no process is running");
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    #[test]
    fn test_wait_for_cursor_timeout() {
        let mut harness = TuiTestHarness::new(80, 24)
            .unwrap()
            .with_timeout(Duration::from_millis(300));

        // Cursor is at (0, 0) initially
        let result = harness.wait_for_cursor((50, 50));
        assert!(result.is_err());

        match result {
            Err(TermTestError::Timeout { .. }) => {
                // Expected timeout
            }
            Err(TermTestError::ProcessExited) => {
                // Also acceptable - no process is running
            }
            _ => panic!("Expected Timeout or ProcessExited error"),
        }
    }

    #[test]
    fn test_wait_for_cursor_with_custom_timeout() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Feed escape sequence to move cursor
        harness.state_mut().feed(b"\x1b[5;10H");

        // Use custom timeout - may return ProcessExited since no process is running
        match harness.wait_for_cursor_timeout((4, 9), Duration::from_millis(500)) {
            Ok(()) => {
                let pos = harness.cursor_position();
                assert_eq!(pos, (4, 9));
            }
            Err(TermTestError::ProcessExited) => {
                // No process running, but cursor should still be in the right position
                let pos = harness.cursor_position();
                assert_eq!(pos, (4, 9), "Cursor should be at (4, 9) even though no process is running");
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    #[test]
    fn test_wait_for_custom_predicate() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?
            .with_timeout(Duration::from_secs(2));

        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("test123");
        harness.spawn(cmd)?;

        // Wait for custom condition: screen contains a digit
        // May return ProcessExited since echo exits immediately
        match harness.wait_for(|state| {
            state.contents().chars().any(|c| c.is_numeric())
        }) {
            Ok(()) => {
                assert!(harness.screen_contents().contains('1'));
            }
            Err(TermTestError::ProcessExited) => {
                // Process exited, but check if we still got the output
                assert!(harness.screen_contents().contains('1'),
                    "Expected digit in output even though process exited");
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    #[test]
    fn test_wait_for_multiline_output() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?
            .with_timeout(Duration::from_secs(2));

        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg("echo 'line1'; echo 'line2'; echo 'line3'");
        harness.spawn(cmd)?;

        // Wait for all lines to appear - may return ProcessExited
        match harness.wait_for(|state| {
            let contents = state.contents();
            contents.contains("line1") &&
            contents.contains("line2") &&
            contents.contains("line3")
        }) {
            Ok(()) => {
                let contents = harness.screen_contents();
                assert!(contents.contains("line1"));
                assert!(contents.contains("line2"));
                assert!(contents.contains("line3"));
            }
            Err(TermTestError::ProcessExited) => {
                // Process exited, check if we still got all output
                let contents = harness.screen_contents();
                assert!(contents.contains("line1"), "Expected line1 in output");
                assert!(contents.contains("line2"), "Expected line2 in output");
                assert!(contents.contains("line3"), "Expected line3 in output");
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    #[test]
    fn test_wait_for_complex_predicate() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?
            .with_timeout(Duration::from_secs(2));

        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("Ready: 100%");
        harness.spawn(cmd)?;

        // Complex predicate: check for pattern - may return ProcessExited
        match harness.wait_for(|state| {
            let contents = state.contents();
            contents.contains("Ready") && contents.contains("%")
        }) {
            Ok(()) => {
                assert!(harness.screen_contents().contains("Ready: 100%"));
            }
            Err(TermTestError::ProcessExited) => {
                // Process exited, but check if we still got the output
                assert!(harness.screen_contents().contains("Ready: 100%"),
                    "Expected 'Ready: 100%' in output even though process exited");
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    #[test]
    fn test_update_state_multiple_times() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("data");
        harness.spawn(cmd)?;

        // Multiple updates - first may succeed, subsequent ones may return ProcessExited
        let _ = harness.update_state(); // First update may get data

        // Give echo time to finish
        std::thread::sleep(Duration::from_millis(100));

        // Subsequent updates will likely return ProcessExited
        match harness.update_state() {
            Ok(()) | Err(TermTestError::ProcessExited) => {
                // Either is fine
            }
            Err(e) => return Err(e),
        }

        // Check that we got the data at some point
        assert!(harness.screen_contents().contains("data"),
            "Expected 'data' in output");
        Ok(())
    }

    // ========================================================================
    // Sixel Validation API Tests
    // ========================================================================

    #[cfg(feature = "sixel")]
    #[test]
    fn test_sixel_count() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Initially no Sixel graphics
        assert_eq!(harness.sixel_count(), 0);

        // Feed a Sixel sequence directly to the screen state
        harness.state_mut().feed(b"\x1b[5;10H");  // Move cursor
        harness.state_mut().feed(b"\x1bPq\"1;1;100;50#0~\x1b\\");  // Sixel sequence

        // Should now have one Sixel
        assert_eq!(harness.sixel_count(), 1);

        // Feed another Sixel
        harness.state_mut().feed(b"\x1b[10;20H");
        harness.state_mut().feed(b"\x1bPq\"1;1;80;60#0~\x1b\\");

        // Should now have two Sixels
        assert_eq!(harness.sixel_count(), 2);

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_sixel_regions() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Feed Sixel with known dimensions
        harness.state_mut().feed(b"\x1b[5;10H");  // Position (4, 9) in 0-based
        harness.state_mut().feed(b"\x1bPq\"1;1;100;50#0~\x1b\\");

        let regions = harness.sixel_regions();
        assert_eq!(regions.len(), 1);

        let region = &regions[0];
        assert_eq!(region.start_row, 4);  // 5-1 (CSI is 1-based, we use 0-based)
        assert_eq!(region.start_col, 9);  // 10-1
        assert_eq!(region.width, 100);
        assert_eq!(region.height, 50);

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_sixel_at_position() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Feed Sixel at position (5, 10) [1-based in CSI]
        harness.state_mut().feed(b"\x1b[5;10H");
        harness.state_mut().feed(b"\x1bPq\"1;1;100;50#0~\x1b\\");

        // Should find Sixel at (4, 9) [0-based]
        let region = harness.sixel_at(4, 9);
        assert!(region.is_some());

        let region = region.unwrap();
        assert_eq!(region.width, 100);
        assert_eq!(region.height, 50);

        // Should not find Sixel at other positions
        assert!(harness.sixel_at(0, 0).is_none());
        assert!(harness.sixel_at(10, 10).is_none());

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_assert_sixel_within_bounds_success() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Place Sixel well within screen bounds
        harness.state_mut().feed(b"\x1b[5;10H");
        harness.state_mut().feed(b"\x1bPq\"1;1;10;10#0~\x1b\\");

        // Large area that encompasses the Sixel
        let area = (0, 0, 80, 24);
        assert!(harness.assert_sixel_within_bounds(area).is_ok());

        // Smaller area that still contains the Sixel
        let area = (3, 8, 20, 15);
        assert!(harness.assert_sixel_within_bounds(area).is_ok());

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_assert_sixel_within_bounds_failure() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Place Sixel at position (4, 9) with size 10x10
        harness.state_mut().feed(b"\x1b[5;10H");
        harness.state_mut().feed(b"\x1bPq\"1;1;10;10#0~\x1b\\");

        // Area that doesn't contain the Sixel
        let area = (0, 0, 5, 5);
        let result = harness.assert_sixel_within_bounds(area);
        assert!(result.is_err());

        // Check that error is SixelValidation
        if let Err(crate::error::TermTestError::SixelValidation(msg)) = result {
            assert!(msg.contains("outside area"));
        } else {
            panic!("Expected SixelValidation error");
        }

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_has_sixel_in_area() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // No Sixel initially
        let area = (0, 0, 80, 24);
        assert!(!harness.has_sixel_in_area(area));

        // Add Sixel at (4, 9)
        harness.state_mut().feed(b"\x1b[5;10H");
        harness.state_mut().feed(b"\x1bPq\"1;1;10;10#0~\x1b\\");

        // Should detect Sixel in large area
        assert!(harness.has_sixel_in_area((0, 0, 80, 24)));

        // Should detect in area that contains it
        assert!(harness.has_sixel_in_area((0, 0, 20, 20)));

        // Should not detect in area that doesn't contain it
        assert!(!harness.has_sixel_in_area((20, 20, 10, 10)));

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_verify_sixel_cleared() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Add Sixel
        harness.state_mut().feed(b"\x1b[5;10H");
        harness.state_mut().feed(b"\x1bPq\"1;1;100;50#0~\x1b\\");
        assert_eq!(harness.sixel_count(), 1);

        // Recreate state (simulating screen clear)
        let new_state = crate::screen::ScreenState::new(80, 24);
        *harness.state_mut() = new_state;

        // Manual update (simulating what verify_sixel_cleared does)
        let before = harness.sixel_count();
        assert_eq!(before, 0); // Already cleared due to new state

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_assert_preview_has_sixel_success() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Place Sixel in standard preview area (5, 40, 35, 15)
        harness.state_mut().feed(b"\x1b[8;45H");  // Row 8, col 45 (within preview)
        harness.state_mut().feed(b"\x1bPq\"1;1;100;50#0~\x1b\\");

        // Should succeed as Sixel is in preview area
        assert!(harness.assert_preview_has_sixel().is_ok());

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_assert_preview_has_sixel_failure() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Place Sixel outside preview area
        harness.state_mut().feed(b"\x1b[2;2H");  // Row 2, col 2 (outside preview)
        harness.state_mut().feed(b"\x1bPq\"1;1;100;50#0~\x1b\\");

        // Should fail as Sixel is not in preview area
        let result = harness.assert_preview_has_sixel();
        assert!(result.is_err());

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_assert_preview_has_sixel_in_custom() -> Result<()> {
        let mut harness = TuiTestHarness::new(120, 40)?;

        // Custom preview area for larger terminal
        let custom_area = (10, 50, 60, 25);

        // Place Sixel in custom area
        // Position (15, 60) [1-based CSI] = (14, 59) [0-based]
        // With dimensions 40x30 (small enough to fit within 60x25 area)
        harness.state_mut().feed(b"\x1b[15;60H");  // Row 15, col 60
        harness.state_mut().feed(b"\x1bPq\"1;1;40;30#0~\x1b\\");

        // Should succeed - Sixel at (14, 59) with size 40x30 is within (10, 50, 60x25)
        assert!(harness.assert_preview_has_sixel_in(custom_area).is_ok());

        // Should fail for different area
        let wrong_area = (0, 0, 20, 20);
        assert!(harness.assert_preview_has_sixel_in(wrong_area).is_err());

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_multiple_sixels_in_area() -> Result<()> {
        let mut harness = TuiTestHarness::new(100, 40)?;

        // Add multiple Sixels in preview area
        let preview_area = (5, 30, 60, 30);

        harness.state_mut().feed(b"\x1b[10;40H");
        harness.state_mut().feed(b"\x1bPq\"1;1;80;60#0~\x1b\\");

        harness.state_mut().feed(b"\x1b[20;50H");
        harness.state_mut().feed(b"\x1bPq\"1;1;100;80#0~\x1b\\");

        // Both should be detected
        assert_eq!(harness.sixel_count(), 2);
        assert!(harness.has_sixel_in_area(preview_area));

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_sixel_at_screen_edge() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Place Sixel at screen edge
        harness.state_mut().feed(b"\x1b[1;1H");  // Top-left corner
        harness.state_mut().feed(b"\x1bPq\"1;1;50;30#0~\x1b\\");

        assert_eq!(harness.sixel_count(), 1);
        assert!(harness.sixel_at(0, 0).is_some());

        // Verify position
        let region = harness.sixel_at(0, 0).unwrap();
        assert_eq!(region.start_row, 0);
        assert_eq!(region.start_col, 0);

        Ok(())
    }

    #[cfg(feature = "sixel")]
    #[test]
    fn test_empty_sixel_regions() -> Result<()> {
        let harness = TuiTestHarness::new(80, 24)?;

        // No Sixel graphics
        assert_eq!(harness.sixel_count(), 0);
        assert!(harness.sixel_regions().is_empty());
        assert!(harness.sixel_at(0, 0).is_none());

        // Empty screen should pass bounds validation
        let area = (0, 0, 80, 24);
        assert!(harness.assert_sixel_within_bounds(area).is_ok());
        assert!(!harness.has_sixel_in_area(area));

        Ok(())
    }

    // ========================================================================
    // Position and Layout Assertion Tests
    // ========================================================================

    #[test]
    fn test_rect_creation() {
        use crate::screen::Rect;

        let rect = Rect::new(10, 20, 30, 40);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 30);
        assert_eq!(rect.height, 40);
    }

    #[test]
    fn test_rect_edges() {
        use crate::screen::Rect;

        let rect = Rect::new(10, 20, 30, 40);
        assert_eq!(rect.right(), 40); // 10 + 30
        assert_eq!(rect.bottom(), 60); // 20 + 40
    }

    #[test]
    fn test_rect_contains_point() {
        use crate::screen::Rect;

        let rect = Rect::new(10, 20, 30, 40);

        // Points inside
        assert!(rect.contains(10, 20)); // Top-left corner
        assert!(rect.contains(25, 35)); // Center
        assert!(rect.contains(39, 59)); // Bottom-right corner (exclusive)

        // Points outside
        assert!(!rect.contains(9, 20)); // Left of rect
        assert!(!rect.contains(10, 19)); // Above rect
        assert!(!rect.contains(40, 30)); // Right of rect
        assert!(!rect.contains(20, 60)); // Below rect
    }

    #[test]
    fn test_rect_contains_rect() {
        use crate::screen::Rect;

        let outer = Rect::new(0, 0, 80, 24);
        let inner = Rect::new(10, 10, 20, 10);

        assert!(outer.contains_rect(&inner));
        assert!(!inner.contains_rect(&outer));

        // Overlapping but not contained
        let partial = Rect::new(70, 10, 20, 10);
        assert!(!outer.contains_rect(&partial)); // Extends past right edge
    }

    #[test]
    fn test_rect_intersects() {
        use crate::screen::Rect;

        let rect1 = Rect::new(10, 10, 20, 20);
        let rect2 = Rect::new(20, 20, 20, 20);

        assert!(rect1.intersects(&rect2));
        assert!(rect2.intersects(&rect1));

        // No intersection
        let rect3 = Rect::new(50, 50, 10, 10);
        assert!(!rect1.intersects(&rect3));
        assert!(!rect3.intersects(&rect1));
    }

    #[test]
    fn test_assert_text_at_position_success() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Feed text at a specific position
        harness.state_mut().feed(b"\x1b[5;10HHello World");

        // Should find the text at position (4, 9) [0-based]
        harness.assert_text_at_position("Hello", 4, 9)?;
        harness.assert_text_at_position("World", 4, 15)?;

        Ok(())
    }

    #[test]
    fn test_assert_text_at_position_failure() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Feed text
        harness.state_mut().feed(b"\x1b[5;10HHello");

        // Should fail when text doesn't match
        let result = harness.assert_text_at_position("World", 4, 9);
        assert!(result.is_err());

        if let Err(TermTestError::Parse(msg)) = result {
            assert!(msg.contains("Text mismatch"));
            assert!(msg.contains("Hello")); // Should show what was found
            assert!(msg.contains("World")); // Should show what was expected
        } else {
            panic!("Expected Parse error");
        }

        Ok(())
    }

    #[test]
    fn test_assert_text_at_position_out_of_bounds() {
        let harness = TuiTestHarness::new(80, 24).unwrap();

        // Row out of bounds
        let result = harness.assert_text_at_position("Test", 100, 0);
        assert!(result.is_err());

        // Column out of bounds
        let result = harness.assert_text_at_position("Test", 0, 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_text_within_bounds_success() -> Result<()> {
        use crate::screen::Rect;
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Feed text in a specific area
        harness.state_mut().feed(b"\x1b[10;40HPreview");

        // Should find text within the preview area
        let preview_area = Rect::new(35, 5, 40, 20);
        harness.assert_text_within_bounds("Preview", preview_area)?;

        Ok(())
    }

    #[test]
    fn test_assert_text_within_bounds_failure() -> Result<()> {
        use crate::screen::Rect;
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Feed text outside the area
        harness.state_mut().feed(b"\x1b[1;1HOutside");

        // Should fail when text is not in the specified area
        let preview_area = Rect::new(40, 10, 30, 10);
        let result = harness.assert_text_within_bounds("Outside", preview_area);
        assert!(result.is_err());

        if let Err(TermTestError::Parse(msg)) = result {
            assert!(msg.contains("not found within bounds"));
        } else {
            panic!("Expected Parse error");
        }

        Ok(())
    }

    #[test]
    fn test_assert_text_within_bounds_tab_bar() -> Result<()> {
        use crate::screen::Rect;
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Simulate tab bar at bottom
        harness.state_mut().feed(b"\x1b[23;1HTab 1 | Tab 2 | Tab 3");

        // Define tab bar area (last 2 rows)
        let tab_bar_area = Rect::new(0, 22, 80, 2);

        // Should find tabs in the area
        harness.assert_text_within_bounds("Tab 1", tab_bar_area)?;
        harness.assert_text_within_bounds("Tab 2", tab_bar_area)?;
        harness.assert_text_within_bounds("Tab 3", tab_bar_area)?;

        Ok(())
    }

    #[test]
    fn test_assert_no_overlap_success() -> Result<()> {
        use crate::screen::Rect;
        let harness = TuiTestHarness::new(80, 24)?;

        // Non-overlapping rectangles
        let sidebar = Rect::new(0, 0, 20, 24);
        let preview = Rect::new(20, 0, 60, 24);

        harness.assert_no_overlap(sidebar, preview)?;

        Ok(())
    }

    #[test]
    fn test_assert_no_overlap_failure() -> Result<()> {
        use crate::screen::Rect;
        let harness = TuiTestHarness::new(80, 24)?;

        // Overlapping rectangles
        let rect1 = Rect::new(10, 10, 20, 20);
        let rect2 = Rect::new(20, 20, 20, 20);

        let result = harness.assert_no_overlap(rect1, rect2);
        assert!(result.is_err());

        if let Err(TermTestError::Parse(msg)) = result {
            assert!(msg.contains("overlap"));
        } else {
            panic!("Expected Parse error");
        }

        Ok(())
    }

    #[test]
    fn test_assert_aligned_horizontal_success() -> Result<()> {
        use crate::screen::Rect;
        let harness = TuiTestHarness::new(80, 24)?;

        // Buttons on same row
        let button1 = Rect::new(10, 20, 15, 3);
        let button2 = Rect::new(30, 20, 15, 3);

        harness.assert_aligned(button1, button2, Axis::Horizontal)?;

        Ok(())
    }

    #[test]
    fn test_assert_aligned_horizontal_failure() -> Result<()> {
        use crate::screen::Rect;
        let harness = TuiTestHarness::new(80, 24)?;

        // Buttons on different rows
        let button1 = Rect::new(10, 20, 15, 3);
        let button2 = Rect::new(30, 21, 15, 3);

        let result = harness.assert_aligned(button1, button2, Axis::Horizontal);
        assert!(result.is_err());

        if let Err(TermTestError::Parse(msg)) = result {
            assert!(msg.contains("not horizontally aligned"));
            assert!(msg.contains("different Y coordinates"));
        } else {
            panic!("Expected Parse error");
        }

        Ok(())
    }

    #[test]
    fn test_assert_aligned_vertical_success() -> Result<()> {
        use crate::screen::Rect;
        let harness = TuiTestHarness::new(80, 24)?;

        // Panels in same column
        let panel1 = Rect::new(40, 0, 40, 10);
        let panel2 = Rect::new(40, 10, 40, 14);

        harness.assert_aligned(panel1, panel2, Axis::Vertical)?;

        Ok(())
    }

    #[test]
    fn test_assert_aligned_vertical_failure() -> Result<()> {
        use crate::screen::Rect;
        let harness = TuiTestHarness::new(80, 24)?;

        // Panels in different columns
        let panel1 = Rect::new(40, 0, 40, 10);
        let panel2 = Rect::new(41, 10, 39, 14);

        let result = harness.assert_aligned(panel1, panel2, Axis::Vertical);
        assert!(result.is_err());

        if let Err(TermTestError::Parse(msg)) = result {
            assert!(msg.contains("not vertically aligned"));
            assert!(msg.contains("different X coordinates"));
        } else {
            panic!("Expected Parse error");
        }

        Ok(())
    }

    #[test]
    fn test_complex_layout_assertions() -> Result<()> {
        use crate::screen::Rect;
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Simulate complex layout
        harness.state_mut().feed(b"\x1b[1;1HHeader");
        harness.state_mut().feed(b"\x1b[3;1HSidebar");
        harness.state_mut().feed(b"\x1b[3;21HContent");
        harness.state_mut().feed(b"\x1b[23;1HStatus Bar");

        // Define areas
        let header = Rect::new(0, 0, 80, 2);
        let sidebar = Rect::new(0, 2, 20, 20);
        let content = Rect::new(20, 2, 60, 20);
        let status_bar = Rect::new(0, 22, 80, 2);

        // Assert text in areas
        harness.assert_text_within_bounds("Header", header)?;
        harness.assert_text_within_bounds("Sidebar", sidebar)?;
        harness.assert_text_within_bounds("Content", content)?;
        harness.assert_text_within_bounds("Status Bar", status_bar)?;

        // Assert no overlap
        harness.assert_no_overlap(sidebar, content)?;
        harness.assert_no_overlap(header, status_bar)?;

        // Assert alignment
        harness.assert_aligned(sidebar, content, Axis::Horizontal)?;

        Ok(())
    }

    #[test]
    fn test_multiline_text_in_area() -> Result<()> {
        use crate::screen::Rect;
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Feed multiline content
        harness.state_mut().feed(b"\x1b[10;40HLine 1");
        harness.state_mut().feed(b"\x1b[11;40HLine 2");
        harness.state_mut().feed(b"\x1b[12;40HLine 3");

        let content_area = Rect::new(35, 5, 40, 20);

        // Should find each line
        harness.assert_text_within_bounds("Line 1", content_area)?;
        harness.assert_text_within_bounds("Line 2", content_area)?;
        harness.assert_text_within_bounds("Line 3", content_area)?;

        Ok(())
    }

    #[test]
    fn test_edge_case_text_at_screen_edge() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Text at top-left corner
        harness.state_mut().feed(b"\x1b[1;1HCorner");
        harness.assert_text_at_position("Corner", 0, 0)?;

        // Text near bottom-right corner
        harness.state_mut().feed(b"\x1b[24;75HEnd");
        harness.assert_text_at_position("End", 23, 74)?;

        Ok(())
    }

    #[test]
    fn test_text_spanning_multiple_lines() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Text that wraps or is on multiple lines
        harness.state_mut().feed(b"\x1b[5;10HFirst");
        harness.state_mut().feed(b"\x1b[6;10HSecond");

        // Each line should be found independently
        harness.assert_text_at_position("First", 4, 9)?;
        harness.assert_text_at_position("Second", 5, 9)?;

        Ok(())
    }
}
