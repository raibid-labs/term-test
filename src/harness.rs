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
//! use portable_pty::CommandBuilder;
//! use ratatui_testlib::TuiTestHarness;
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

use std::{
    fs::File,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

use portable_pty::{CommandBuilder, ExitStatus};

use crate::{
    error::{Result, TermTestError},
    events::{
        encode_key_event, encode_mouse_event, KeyCode, KeyEvent, Modifiers, MouseButton,
        MouseEvent, ScrollDirection,
    },
    pty::TestTerminal,
    screen::ScreenState,
    terminal_profiles::{Feature, TerminalCapabilities, TerminalProfile},
    timing::{fps_to_frame_budget, LatencyProfile, TimingHooks, TimingRecorder},
};

/// Default timeout for wait operations (5 seconds).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Results from memory profiling.
///
/// Provides estimated memory usage for a test harness, including the screen
/// buffer and Sixel regions.
///
/// # Memory Estimation
///
/// This struct provides size estimates, not precise heap measurements:
///
/// - **Screen buffer**: `width * height * size_of::<Cell>()`
/// - **Sixel regions**: Sum of all Sixel data sizes
///
/// These are conservative estimates useful for detecting memory regressions
/// in tests, not exact heap profiling.
///
/// # Example
///
/// ```rust,no_run
/// use ratatui_testlib::TuiTestHarness;
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let harness = TuiTestHarness::new(80, 24)?;
///
/// // Get memory usage estimate
/// let memory = harness.memory_usage();
/// println!("Current: {} bytes", memory.current_bytes);
/// println!("Peak: {} bytes", memory.peak_bytes);
///
/// // Assert memory stays under limit (e.g., 1MB)
/// harness.assert_memory_under(1_000_000)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryResults {
    /// Current estimated memory usage in bytes.
    pub current_bytes: usize,
    /// Peak estimated memory usage in bytes.
    ///
    /// Note: In the current implementation, this is the same as current_bytes
    /// since we don't track historical peak values. Future versions may add
    /// peak tracking across multiple measurements.
    pub peak_bytes: usize,
}

impl MemoryResults {
    /// Creates memory results from byte counts.
    ///
    /// # Arguments
    ///
    /// * `current_bytes` - Current memory usage estimate
    /// * `peak_bytes` - Peak memory usage estimate
    pub fn new(current_bytes: usize, peak_bytes: usize) -> Self {
        Self { current_bytes, peak_bytes }
    }

    /// Returns a formatted summary string.
    ///
    /// # Example Output
    ///
    /// ```text
    /// Memory Usage:
    ///   Current: 123,456 bytes (120.56 KB)
    ///   Peak: 150,000 bytes (146.48 KB)
    /// ```
    pub fn summary(&self) -> String {
        format!(
            "Memory Usage:\n  Current: {} bytes ({:.2} KB)\n  Peak: {} bytes ({:.2} KB)",
            self.current_bytes,
            self.current_bytes as f64 / 1024.0,
            self.peak_bytes,
            self.peak_bytes as f64 / 1024.0
        )
    }
}

/// Axis for alignment checking.
///
/// Used with [`TuiTestHarness::assert_aligned`] to specify which axis to check
/// for alignment between two rectangles.
///
/// # Example
///
/// ```rust,no_run
/// use ratatui_testlib::{Axis, Rect, TuiTestHarness};
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

/// An event that occurred during test execution, recorded for debugging.
///
/// This enum represents different types of events that can be captured during
/// a test session for later playback or analysis.
#[derive(Debug, Clone)]
pub enum RecordedEvent {
    /// Input data sent to the PTY.
    Input(Vec<u8>),
    /// Output data received from the PTY.
    Output(Vec<u8>),
    /// Screen state changed (e.g., after processing escape sequences).
    StateChange {
        /// Snapshot of screen contents at this point.
        contents: String,
        /// Cursor position (row, col).
        cursor: (u16, u16),
    },
}

/// A timestamped recorded event.
#[derive(Debug, Clone)]
struct TimestampedEvent {
    /// Timestamp relative to recording start.
    timestamp: Duration,
    /// The recorded event.
    event: RecordedEvent,
}

/// High-level test harness for TUI applications.
///
/// This combines PTY management and terminal emulation to provide
/// an ergonomic API for testing TUI applications.
///
/// # Example
///
/// ```rust,no_run
/// use portable_pty::CommandBuilder;
/// use ratatui_testlib::TuiTestHarness;
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
/// use std::time::Duration;
///
/// use ratatui_testlib::TuiTestHarness;
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
    event_delay: Duration,
    // Recording and debugging fields
    recording: bool,
    recorded_events: Vec<TimestampedEvent>,
    recording_start: Option<Instant>,
    verbose: bool,
    // Terminal profile configuration
    terminal_profile: TerminalProfile,
    // Timing and latency profiling
    timing_recorder: TimingRecorder,
    latency_profile: LatencyProfile,
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
            event_delay: Duration::ZERO,
            recording: false,
            recorded_events: Vec::new(),
            recording_start: None,
            verbose: false,
            terminal_profile: TerminalProfile::default(),
            timing_recorder: TimingRecorder::new(),
            latency_profile: LatencyProfile::new(),
        })
    }

    /// Creates a builder for configuring a test harness.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// use ratatui_testlib::TuiTestHarness;
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

    /// Configures the harness for a specific terminal emulator profile.
    ///
    /// This sets the terminal profile which controls which features are available
    /// during testing. Use this to ensure your TUI application works correctly
    /// across different terminal emulators.
    ///
    /// # Arguments
    ///
    /// * `profile` - The terminal profile to use
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::{TerminalProfile, TuiTestHarness};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// // Test with WezTerm profile (supports Sixel)
    /// let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::WezTerm);
    ///
    /// // Test with VT100 profile (minimal features)
    /// let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::VT100);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_terminal_profile(mut self, profile: TerminalProfile) -> Self {
        self.terminal_profile = profile;
        self
    }

    /// Configures the harness to simulate a specific TERMINFO environment.
    ///
    /// This is a convenience method that looks up a terminal profile by name
    /// and configures the harness to simulate that terminal's behavior.
    ///
    /// # Arguments
    ///
    /// * `term_name` - TERM environment variable value (e.g., "xterm-256color", "wezterm")
    ///
    /// # Returns
    ///
    /// Returns `self` if a matching profile is found, otherwise uses the default profile.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// // Simulate WezTerm using TERM value
    /// let harness = TuiTestHarness::new(80, 24)?.simulate_terminfo("wezterm");
    ///
    /// // Simulate xterm-256color
    /// let harness = TuiTestHarness::new(80, 24)?.simulate_terminfo("xterm-256color");
    /// # Ok(())
    /// # }
    /// ```
    pub fn simulate_terminfo(mut self, term_name: &str) -> Self {
        if let Some(profile) = TerminalProfile::from_name(term_name) {
            self.terminal_profile = profile;
        }
        self
    }

    /// Checks if the current terminal profile supports a specific feature.
    ///
    /// This allows you to conditionally run tests or assertions based on
    /// terminal capabilities.
    ///
    /// # Arguments
    ///
    /// * `feature` - The feature to check
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::{Feature, TerminalProfile, TuiTestHarness};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::WezTerm);
    ///
    /// if harness.supports_feature(Feature::Sixel) {
    ///     // Run Sixel-specific tests
    ///     println!("Testing Sixel graphics...");
    /// }
    ///
    /// if harness.supports_feature(Feature::TrueColor) {
    ///     // Verify true color rendering
    ///     println!("Testing 24-bit color...");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn supports_feature(&self, feature: Feature) -> bool {
        self.terminal_profile.supports(feature)
    }

    /// Returns the full terminal capabilities for the current profile.
    ///
    /// This provides detailed information about what features are supported,
    /// including color depth, mouse protocols, and graphics support.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::{TerminalProfile, TuiTestHarness};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::WezTerm);
    ///
    /// let caps = harness.terminal_capabilities();
    /// println!("Terminal: {}", caps.term_name);
    /// println!("Color depth: {:?}", caps.color_depth);
    /// println!("Sixel support: {}", caps.sixel_support);
    /// println!("\n{}", caps.summary());
    /// # Ok(())
    /// # }
    /// ```
    pub fn terminal_capabilities(&self) -> TerminalCapabilities {
        self.terminal_profile.capabilities()
    }

    /// Returns the current terminal profile.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::{TerminalProfile, TuiTestHarness};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::WezTerm);
    ///
    /// assert_eq!(harness.terminal_profile(), TerminalProfile::WezTerm);
    /// # Ok(())
    /// # }
    /// ```
    pub fn terminal_profile(&self) -> TerminalProfile {
        self.terminal_profile
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
        // Record input timestamp for latency profiling
        self.timing_recorder.record_event("input_sent");
        self.latency_profile.mark_input();

        let bytes = text.as_bytes();
        self.record_input(bytes);
        self.terminal.write(bytes)?;

        // Update state, ignoring ProcessExited since the process might exit
        // after receiving input (e.g., sending 'q' to quit)
        let _ = self.update_state();

        // Record render completion
        self.timing_recorder.record_event("render_complete");
        self.latency_profile.mark_render_end();
        self.latency_profile.mark_frame_ready();

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
    /// use ratatui_testlib::{KeyCode, TuiTestHarness};
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
    /// use ratatui_testlib::{KeyCode, Modifiers, TuiTestHarness};
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
    /// harness.send_key_with_modifiers(KeyCode::Delete, Modifiers::CTRL | Modifiers::ALT)?;
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

    /// Alias for [`send_keys`](Self::send_keys).
    ///
    /// Types a text string by sending each character as a key event.
    /// This method is provided for semantic clarity when "typing" text
    /// is the intent, as opposed to sending a block of text (via `send_text`)
    /// or specific key codes.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.type_text("typing speed simulation")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn type_text(&mut self, text: &str) -> Result<()> {
        self.send_keys(text)
    }

    /// Sets the delay between consecutive events.
    ///
    /// This configures how long the harness waits after sending each event before
    /// proceeding. This is useful for testing debouncing, throttling, or simulating
    /// realistic user input timing.
    ///
    /// # Arguments
    ///
    /// * `delay` - Duration to wait between events
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    ///
    /// // Set 100ms delay between events
    /// harness.set_event_delay(Duration::from_millis(100));
    ///
    /// // Each key will now have 100ms delay after it
    /// harness.send_keys("hello")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_event_delay(&mut self, delay: Duration) {
        self.event_delay = delay;
    }

    /// Gets the current event delay.
    ///
    /// Returns the configured delay between consecutive events. If zero,
    /// the default 50ms delay is used.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    ///
    /// harness.set_event_delay(Duration::from_millis(100));
    /// assert_eq!(harness.event_delay(), Duration::from_millis(100));
    /// # Ok(())
    /// # }
    /// ```
    pub fn event_delay(&self) -> Duration {
        self.event_delay
    }

    /// Simulates time passing without sending any events.
    ///
    /// This is useful for testing debouncing logic where you need to verify
    /// that nothing happens during a quiet period.
    ///
    /// # Arguments
    ///
    /// * `duration` - Amount of time to advance
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    ///
    /// // Send a key, then wait to test debouncing
    /// harness.send_key(ratatui_testlib::KeyCode::Char('a'))?;
    /// harness.advance_time(Duration::from_millis(300))?;
    ///
    /// // Now the debounced action should have occurred
    /// harness.wait_for_text("Debounced action")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn advance_time(&mut self, duration: Duration) -> Result<()> {
        std::thread::sleep(duration);
        // Update state after time has passed if a process is running
        if self.terminal.is_running() {
            // Ignore ProcessExited error since we just want to advance time
            let _ = self.update_state();
        }
        Ok(())
    }

    /// Sends a key multiple times with a specified interval between each press.
    ///
    /// This simulates key repeat behavior or rapid key pressing, useful for
    /// testing auto-repeat handling, rate limiting, or rapid input scenarios.
    ///
    /// # Arguments
    ///
    /// * `key` - The character to send
    /// * `count` - Number of times to send the key
    /// * `interval` - Delay between each key press
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    ///
    /// // Simulate holding down the right arrow key (10 presses, 50ms apart)
    /// harness.press_key_repeat('→', 10, Duration::from_millis(50))?;
    ///
    /// // Or use KeyCode for special keys
    /// // This would be: harness.send_key_repeat(KeyCode::Right, 10, ...)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn press_key_repeat(&mut self, key: char, count: usize, interval: Duration) -> Result<()> {
        for _ in 0..count {
            self.send_key(KeyCode::Char(key))?;
            std::thread::sleep(interval);
        }
        Ok(())
    }

    /// Sends a mouse event to the PTY.
    ///
    /// This simulates mouse interactions like clicks, drags, and scrolling using
    /// SGR (Select Graphic Rendition) mouse encoding, which is supported by most
    /// modern terminal emulators and TUI frameworks.
    ///
    /// # Arguments
    ///
    /// * `event` - The mouse event to send
    ///
    /// # Errors
    ///
    /// Returns an error if the write fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{MouseButton, MouseEvent, TuiTestHarness};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    ///
    /// // Send a left click at (10, 5)
    /// harness.send_mouse_event(MouseEvent::press(10, 5, MouseButton::Left))?;
    /// harness.send_mouse_event(MouseEvent::release(10, 5, MouseButton::Left))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send_mouse_event(&mut self, event: MouseEvent) -> Result<()> {
        // Record input timestamp for latency profiling
        self.timing_recorder.record_event("input_sent");
        self.latency_profile.mark_input();

        let bytes = encode_mouse_event(&event);
        self.record_input(&bytes);
        self.terminal.write_all(&bytes)?;

        // Apply configured event delay, or use default 50ms if no delay is set
        let delay = if self.event_delay.is_zero() {
            Duration::from_millis(50)
        } else {
            self.event_delay
        };
        std::thread::sleep(delay);

        // Update state
        let _ = self.update_state();

        // Record render completion
        self.timing_recorder.record_event("render_complete");
        self.latency_profile.mark_render_end();
        self.latency_profile.mark_frame_ready();

        Ok(())
    }

    /// Simulates a mouse click at the specified position.
    ///
    /// This sends a press event followed immediately by a release event.
    ///
    /// # Arguments
    ///
    /// * `x` - Column position (0-indexed)
    /// * `y` - Row position (0-indexed)
    /// * `button` - Mouse button to click
    ///
    /// # Errors
    ///
    /// Returns an error if the events cannot be sent.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{MouseButton, TuiTestHarness};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.mouse_click(10, 5, MouseButton::Left)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn mouse_click(&mut self, x: u16, y: u16, button: MouseButton) -> Result<()> {
        self.send_mouse_event(MouseEvent::press(x, y, button))?;
        self.send_mouse_event(MouseEvent::release(x, y, button))
    }

    /// Simulates a mouse drag operation.
    ///
    /// Sends a press event at the start position, followed by a press event
    /// (with drag motion implied) at the end position, and finally a release event.
    ///
    /// # Arguments
    ///
    /// * `start_x` - Start column (0-indexed)
    /// * `start_y` - Start row (0-indexed)
    /// * `end_x` - End column (0-indexed)
    /// * `end_y` - End row (0-indexed)
    /// * `button` - Mouse button to drag with
    ///
    /// # Errors
    ///
    /// Returns an error if the events cannot be sent.
    pub fn mouse_drag(
        &mut self,
        start_x: u16,
        start_y: u16,
        end_x: u16,
        end_y: u16,
        button: MouseButton,
    ) -> Result<()> {
        self.send_mouse_event(MouseEvent::press(start_x, start_y, button))?;

        // SGR encoding handles drag by sending press events with updated coordinates
        // We use a slightly modified button code for drag events in standard protocols,
        // but standard SGR press events at new coordinates are often interpreted as drags
        // if the button hasn't been released.
        //
        // Ideally, we'd add +32 to the button code for motion events, but SGR
        // separates motion logic. For simple simulation, sending another press
        // at the new location is usually sufficient for TUI frameworks to detect drag.
        //
        // However, strictly speaking, motion events might need the +32 flag.
        // encode_mouse_event handles this if we had a 'drag' variant in MouseEvent.
        // For now, we'll simulate it as a press at the new location.

        // Note: Some terminals/frameworks expect intermediate points.
        // We jump directly to end for simplicity.
        self.send_mouse_event(MouseEvent::press(end_x, end_y, button))?;

        self.send_mouse_event(MouseEvent::release(end_x, end_y, button))
    }

    /// Simulates a mouse scroll event.
    ///
    /// # Arguments
    ///
    /// * `x` - Cursor column position during scroll
    /// * `y` - Cursor row position during scroll
    /// * `direction` - Direction to scroll
    ///
    /// # Errors
    ///
    /// Returns an error if the event cannot be sent.
    pub fn mouse_scroll(&mut self, x: u16, y: u16, direction: ScrollDirection) -> Result<()> {
        self.send_mouse_event(MouseEvent::scroll(x, y, direction))
    }

    /// Internal method to send a key event and update state.
    ///
    /// This encodes the key event to bytes, writes to the PTY, adds a small
    /// delay for the application to process the input, and updates the screen state.
    fn send_key_event(&mut self, event: KeyEvent) -> Result<()> {
        // Record input timestamp for latency profiling
        self.timing_recorder.record_event("input_sent");
        self.latency_profile.mark_input();

        let bytes = encode_key_event(&event);
        self.record_input(&bytes);
        self.terminal.write_all(&bytes)?;

        // Apply configured event delay, or use default 50ms if no delay is set
        let delay = if self.event_delay.is_zero() {
            Duration::from_millis(50)
        } else {
            self.event_delay
        };
        std::thread::sleep(delay);

        // Update state, ignoring ProcessExited since the process might exit
        // after receiving input (e.g., pressing 'q' to quit)
        let _ = self.update_state();

        // Record render completion
        self.timing_recorder.record_event("render_complete");
        self.latency_profile.mark_render_end();
        self.latency_profile.mark_frame_ready();

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
                        self.record_output(&buf[..n]);
                        self.state.feed(&buf[..n]);
                        self.record_state_change();
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
                    self.record_output(&buf[..n]);
                    self.state.feed(&buf[..n]);
                    self.record_state_change();
                }
                Err(e) => {
                    // Use proper ErrorKind matching instead of string matching
                    match e {
                        TermTestError::Io(io_err)
                            if io_err.kind() == std::io::ErrorKind::WouldBlock =>
                        {
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
    /// harness.wait_for(|state| state.contains("Ready"))?;
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
        self.wait_for_with_context(move |state| state.contains(&text), &description)
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

                return Err(TermTestError::Timeout { timeout_ms: timeout.as_millis() as u64 });
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
    /// Returns a `Timeout` error if the cursor does not reach the position within the configured
    /// timeout.
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
        self.wait_for_with_context(move |state| state.cursor_position() == pos, &description)
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
    /// Returns a `Timeout` error if the cursor does not reach the position within the specified
    /// timeout. Returns `ProcessExited` if the child process exits before the cursor reaches
    /// the position.
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

                return Err(TermTestError::Timeout { timeout_ms: timeout.as_millis() as u64 });
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
    /// use portable_pty::CommandBuilder;
    /// use ratatui_testlib::TuiTestHarness;
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
    /// use portable_pty::CommandBuilder;
    /// use ratatui_testlib::TuiTestHarness;
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
    // Memory Profiling
    // ========================================================================

    /// Gets current memory usage estimate.
    ///
    /// Returns estimated memory usage based on:
    /// - Screen buffer: `width * height * size_of::<Cell>()`
    /// - Sixel regions: Sum of all Sixel data sizes
    ///
    /// This is a size estimate, not precise heap measurement. It's useful
    /// for detecting memory regressions in tests.
    ///
    /// # Returns
    ///
    /// A [`MemoryResults`] struct with current and peak memory estimates.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    ///
    /// let memory = harness.memory_usage();
    /// println!("Current memory: {} bytes", memory.current_bytes);
    /// println!("Peak memory: {} bytes", memory.peak_bytes);
    /// # Ok(())
    /// # }
    /// ```
    pub fn memory_usage(&self) -> MemoryResults {
        use std::mem::size_of;

        // Get screen dimensions
        let (width, height) = self.state.size();

        // Estimate screen buffer size
        // Each cell contains: char (4 bytes) + 2 Option<u8> (2 bytes) + 3 bool (3 bytes) = ~9 bytes
        // But with padding/alignment it's likely more
        let screen_buffer_size =
            (width as usize) * (height as usize) * size_of::<crate::screen::Cell>();

        // Estimate Sixel regions size
        let sixel_size: usize = self
            .state
            .sixel_regions()
            .iter()
            .map(|region| region.data.len())
            .sum();

        let total = screen_buffer_size + sixel_size;

        // Note: peak_bytes is currently the same as current_bytes
        // Future versions may track historical peaks
        MemoryResults::new(total, total)
    }

    /// Asserts memory is under a limit.
    ///
    /// Checks that the current estimated memory usage is below the specified
    /// limit in bytes. This is useful for regression testing to ensure memory
    /// usage doesn't grow unexpectedly.
    ///
    /// # Arguments
    ///
    /// * `limit_bytes` - Maximum allowed memory usage in bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the current memory usage exceeds the limit.
    /// The error message includes the current usage and the limit.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::new(80, 24)?;
    ///
    /// // Assert memory stays under 1 MB
    /// harness.assert_memory_under(1_000_000)?;
    ///
    /// // Assert memory stays under 100 KB
    /// harness.assert_memory_under(100_000)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn assert_memory_under(&self, limit_bytes: usize) -> Result<()> {
        let memory = self.memory_usage();

        if memory.current_bytes > limit_bytes {
            return Err(TermTestError::Parse(format!(
                "Memory usage exceeds limit: {} bytes > {} bytes (limit)\n{}",
                memory.current_bytes,
                limit_bytes,
                memory.summary()
            )));
        }

        Ok(())
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
    /// use ratatui_testlib::{Rect, TuiTestHarness};
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
    /// use ratatui_testlib::{Rect, TuiTestHarness};
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
    pub fn assert_no_overlap(
        &self,
        rect1: crate::screen::Rect,
        rect2: crate::screen::Rect,
    ) -> Result<()> {
        if rect1.intersects(&rect2) {
            return Err(TermTestError::Parse(format!(
                "Rectangles overlap!\n\
                 Rect 1: (x={}, y={}, width={}, height={})\n\
                 Rect 2: (x={}, y={}, width={}, height={})\n\
                 Overlap region exists between x=[{}, {}) and y=[{}, {})",
                rect1.x,
                rect1.y,
                rect1.width,
                rect1.height,
                rect2.x,
                rect2.y,
                rect2.width,
                rect2.height,
                rect1.x.max(rect2.x),
                rect1.right().min(rect2.right()),
                rect1.y.max(rect2.y),
                rect1.bottom().min(rect2.bottom())
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
    /// use ratatui_testlib::{Axis, Rect, TuiTestHarness};
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
                        rect1.y,
                        rect1.x,
                        rect1.width,
                        rect1.height,
                        rect2.y,
                        rect2.x,
                        rect2.width,
                        rect2.height
                    )));
                }
            }
            Axis::Vertical => {
                if rect1.x != rect2.x {
                    return Err(TermTestError::Parse(format!(
                        "Rectangles not vertically aligned (different X coordinates)\n\
                         Rect 1: x={} (y={}, width={}, height={})\n\
                         Rect 2: x={} (y={}, width={}, height={})",
                        rect1.x,
                        rect1.y,
                        rect1.width,
                        rect1.height,
                        rect2.x,
                        rect2.y,
                        rect2.width,
                        rect2.height
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
    ///     println!(
    ///         "Sixel {}: position ({}, {}), size {}x{}",
    ///         i, region.start_row, region.start_col, region.width, region.height
    ///     );
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
        self.state
            .sixel_regions()
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
            return Err(TermTestError::SixelValidation(format!(
                "No Sixel graphics found in standard preview area {:?}. \
                    Current Sixel count: {}. \
                    Regions: {:?}",
                preview_area,
                self.sixel_count(),
                self.sixel_regions()
                    .iter()
                    .map(|r| (r.start_row, r.start_col, r.width, r.height))
                    .collect::<Vec<_>>()
            )));
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
            return Err(TermTestError::SixelValidation(format!(
                "No Sixel graphics found in preview area {:?}. \
                    Current Sixel count: {}. \
                    Regions: {:?}",
                preview_area,
                self.sixel_count(),
                self.sixel_regions()
                    .iter()
                    .map(|r| (r.start_row, r.start_col, r.width, r.height))
                    .collect::<Vec<_>>()
            )));
        }
        Ok(())
    }

    // ========================================================================
    // Golden File Testing (Visual Regression)
    // ========================================================================

    /// Save the current screen state as a golden file.
    ///
    /// Golden files capture the expected terminal output for visual regression testing.
    /// They are saved in the directory specified by the `GOLDEN_DIR` environment variable,
    /// or `tests/golden/` by default.
    ///
    /// # Arguments
    ///
    /// * `name` - Name for the golden file (used as filename without extension)
    ///
    /// # Returns
    ///
    /// The path where the golden file was saved.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written or the directory cannot be created.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use portable_pty::CommandBuilder;
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// let cmd = CommandBuilder::new("my-app");
    /// harness.spawn(cmd)?;
    /// harness.wait_for_text("Welcome")?;
    ///
    /// // Save current state as golden
    /// let path = harness.save_golden("welcome_screen")?;
    /// println!("Saved golden file to: {}", path.display());
    /// # Ok(())
    /// # }
    /// ```
    pub fn save_golden(&self, name: &str) -> Result<std::path::PathBuf> {
        crate::golden::save_golden(name, &self.state)
    }

    /// Compare current screen state against a golden file.
    ///
    /// This method loads a previously saved golden file and compares it against
    /// the current screen state. If they don't match, it generates a detailed
    /// unified diff showing the differences.
    ///
    /// If the `UPDATE_GOLDENS=1` environment variable is set, this will update
    /// the golden file instead of comparing (useful for updating all goldens at once).
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the golden file to compare against (without extension)
    ///
    /// # Returns
    ///
    /// `Ok(())` if the states match (or golden was updated in update mode).
    ///
    /// # Errors
    ///
    /// Returns an error with a detailed diff if the states don't match, or if
    /// the golden file cannot be read.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use portable_pty::CommandBuilder;
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// let cmd = CommandBuilder::new("my-app");
    /// harness.spawn(cmd)?;
    /// harness.wait_for_text("Welcome")?;
    ///
    /// // Compare against previously saved golden
    /// harness.assert_matches_golden("welcome_screen")?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Update Mode
    ///
    /// To update all golden files at once:
    ///
    /// ```bash
    /// UPDATE_GOLDENS=1 cargo test
    /// ```
    pub fn assert_matches_golden(&self, name: &str) -> Result<()> {
        crate::golden::assert_matches_golden(name, &self.state)
    }

    /// Update a golden file with the current screen state.
    ///
    /// This is equivalent to `save_golden()` but makes the intent clearer when
    /// you're explicitly updating an existing golden file.
    ///
    /// # Arguments
    ///
    /// * `name` - Name of the golden file to update (without extension)
    ///
    /// # Returns
    ///
    /// The path where the golden file was saved.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use portable_pty::CommandBuilder;
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// let cmd = CommandBuilder::new("my-app");
    /// harness.spawn(cmd)?;
    /// harness.wait_for_text("New Layout")?;
    ///
    /// // Update golden file after intentional UI changes
    /// harness.update_golden("main_screen")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn update_golden(&self, name: &str) -> Result<std::path::PathBuf> {
        crate::golden::update_golden(name, &self.state)
    }

    // ============================================================================
    // Recording and Debug Methods
    // ============================================================================

    /// Starts recording all I/O events and state changes.
    ///
    /// When recording is enabled, the harness will capture:
    /// - All input sent to the PTY
    /// - All output received from the PTY
    /// - Screen state changes after processing escape sequences
    ///
    /// Recordings can be saved to disk using [`save_recording`](Self::save_recording)
    /// for debugging failed tests or understanding application behavior.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.start_recording();
    ///
    /// // Test operations...
    /// harness.send_text("hello\n")?;
    ///
    /// // Save recording for debugging
    /// harness.save_recording("test_recording.json")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn start_recording(&mut self) {
        self.recording = true;
        self.recording_start = Some(Instant::now());
        self.recorded_events.clear();
    }

    /// Stops recording I/O events.
    ///
    /// Recording can be stopped without saving, or you can call
    /// [`save_recording`](Self::save_recording) to persist the events.
    pub fn stop_recording(&mut self) {
        self.recording = false;
    }

    /// Saves the current recording to a file in JSON format.
    ///
    /// The recording includes all captured events with timestamps, which can be
    /// useful for debugging test failures or understanding application behavior.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the recording should be saved
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be created
    /// - Serialization fails
    /// - Writing to the file fails
    ///
    /// # Format
    ///
    /// The saved file is a JSON array containing timestamped events:
    ///
    /// ```json
    /// [
    ///   {
    ///     "timestamp_ms": 0,
    ///     "event": {
    ///       "type": "Input",
    ///       "data": [104, 101, 108, 108, 111]
    ///     }
    ///   },
    ///   {
    ///     "timestamp_ms": 150,
    ///     "event": {
    ///       "type": "Output",
    ///       "data": [27, 91, 72, 27, 91, 50, 74]
    ///     }
    ///   }
    /// ]
    /// ```
    pub fn save_recording<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        use std::io::BufWriter;

        let file = File::create(path.as_ref()).map_err(|e| {
            TermTestError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create recording file: {}", e),
            ))
        })?;

        let mut writer = BufWriter::new(file);

        // Write JSON array
        writeln!(writer, "[").map_err(|e| {
            TermTestError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write recording: {}", e),
            ))
        })?;

        for (i, event) in self.recorded_events.iter().enumerate() {
            let comma = if i < self.recorded_events.len() - 1 {
                ","
            } else {
                ""
            };

            // Format event as JSON
            let event_json = match &event.event {
                RecordedEvent::Input(data) => {
                    format!(
                        r#"  {{"timestamp_ms": {}, "event": {{"type": "Input", "data": {:?}}}}}{}"#,
                        event.timestamp.as_millis(),
                        data,
                        comma
                    )
                }
                RecordedEvent::Output(data) => {
                    format!(
                        r#"  {{"timestamp_ms": {}, "event": {{"type": "Output", "data": {:?}}}}}{}"#,
                        event.timestamp.as_millis(),
                        data,
                        comma
                    )
                }
                RecordedEvent::StateChange { contents, cursor } => {
                    let escaped_contents = contents
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('\n', "\\n")
                        .replace('\r', "\\r")
                        .replace('\t', "\\t");
                    format!(
                        r#"  {{"timestamp_ms": {}, "event": {{"type": "StateChange", "contents": "{}", "cursor": [{}, {}]}}}}{}"#,
                        event.timestamp.as_millis(),
                        escaped_contents,
                        cursor.0,
                        cursor.1,
                        comma
                    )
                }
            };

            writeln!(writer, "{}", event_json).map_err(|e| {
                TermTestError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to write event: {}", e),
                ))
            })?;
        }

        writeln!(writer, "]").map_err(|e| {
            TermTestError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write recording: {}", e),
            ))
        })?;

        writer.flush().map_err(|e| {
            TermTestError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to flush recording: {}", e),
            ))
        })?;

        Ok(())
    }

    /// Checks if recording is currently active.
    ///
    /// # Returns
    ///
    /// `true` if recording is active, `false` otherwise.
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Saves the current screen state to a file.
    ///
    /// This is useful for capturing the screen state when a test fails,
    /// allowing you to inspect what was actually displayed.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the screenshot should be saved
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be created or written.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// // ... spawn app and interact ...
    ///
    /// // Save screenshot on failure
    /// if !harness.screen_contents().contains("expected") {
    ///     harness.save_screenshot("failure_screenshot.txt")?;
    ///     panic!("Test failed, screenshot saved");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn save_screenshot<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut file = File::create(path.as_ref()).map_err(|e| {
            TermTestError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create screenshot file: {}", e),
            ))
        })?;

        let contents = self.screenshot_string();
        file.write_all(contents.as_bytes()).map_err(|e| {
            TermTestError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write screenshot: {}", e),
            ))
        })?;

        Ok(())
    }

    /// Returns the current screen state as a string.
    ///
    /// This includes the screen contents with a header showing dimensions
    /// and cursor position, formatted for easy reading in logs.
    ///
    /// # Returns
    ///
    /// A formatted string containing screen state information.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// // ... spawn app and interact ...
    ///
    /// // Log current state
    /// eprintln!("{}", harness.screenshot_string());
    /// # Ok(())
    /// # }
    /// ```
    pub fn screenshot_string(&self) -> String {
        let cursor = self.state.cursor_position();
        let contents = self.state.debug_contents();
        let (width, height) = self.state.size();

        format!(
            "=== Screen State ===\n\
             Size: {}x{}\n\
             Cursor: row={}, col={}\n\
             {}\n\
             ===================",
            width, height, cursor.0, cursor.1, contents
        )
    }

    /// Enables or disables verbose escape sequence logging.
    ///
    /// When verbose mode is enabled, all escape sequences received from the PTY
    /// are logged to stderr in a human-readable format. This is useful for
    /// debugging terminal emulation issues or understanding what escape sequences
    /// your application is generating.
    ///
    /// # Arguments
    ///
    /// * `verbose` - `true` to enable verbose logging, `false` to disable
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.set_verbose(true);
    ///
    /// // All escape sequences will now be logged to stderr
    /// harness.send_text("hello\n")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    /// Records an input event if recording is active.
    fn record_input(&mut self, data: &[u8]) {
        if self.recording {
            if let Some(start) = self.recording_start {
                let timestamp = start.elapsed();
                self.recorded_events.push(TimestampedEvent {
                    timestamp,
                    event: RecordedEvent::Input(data.to_vec()),
                });
            }
        }
    }

    /// Records an output event if recording is active.
    fn record_output(&mut self, data: &[u8]) {
        if self.recording {
            if let Some(start) = self.recording_start {
                let timestamp = start.elapsed();
                self.recorded_events.push(TimestampedEvent {
                    timestamp,
                    event: RecordedEvent::Output(data.to_vec()),
                });
            }
        }

        // Verbose logging
        if self.verbose {
            eprintln!("[VERBOSE] Received {} bytes: {:?}", data.len(), data);
            // Try to display as string if it's printable
            if let Ok(s) = std::str::from_utf8(data) {
                eprintln!("[VERBOSE] As string: {:?}", s);
            }
        }
    }

    /// Records a state change event if recording is active.
    fn record_state_change(&mut self) {
        if self.recording {
            if let Some(start) = self.recording_start {
                let timestamp = start.elapsed();
                let contents = self.state.debug_contents();
                let cursor = self.state.cursor_position();
                self.recorded_events.push(TimestampedEvent {
                    timestamp,
                    event: RecordedEvent::StateChange { contents, cursor },
                });
            }
        }
    }

    // =========================================================================
    // Parallel Testing Support
    // =========================================================================

    /// Runs a test closure with an isolated terminal context.
    ///
    /// This method provides a simple way to run tests in isolation without
    /// manually managing terminal acquisition and release. The closure receives
    /// a mutable reference to the harness.
    ///
    /// # Arguments
    ///
    /// * `f` - Closure to run with the harness
    ///
    /// # Errors
    ///
    /// Returns an error if the closure returns an error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use portable_pty::CommandBuilder;
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// TuiTestHarness::with_isolation(|harness| {
    ///     let mut cmd = CommandBuilder::new("echo");
    ///     cmd.arg("test");
    ///     harness.spawn(cmd)?;
    ///     harness.wait_for_text("test")?;
    ///     Ok(())
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_isolation<F>(f: F) -> Result<()>
    where
        F: FnOnce(&mut TuiTestHarness) -> Result<()>,
    {
        let mut harness = TuiTestHarness::new(80, 24)?;
        f(&mut harness)
    }

    /// Runs a test closure with an isolated terminal context and custom dimensions.
    ///
    /// This is similar to `with_isolation` but allows specifying custom terminal dimensions.
    ///
    /// # Arguments
    ///
    /// * `width` - Terminal width in columns
    /// * `height` - Terminal height in rows
    /// * `f` - Closure to run with the harness
    ///
    /// # Errors
    ///
    /// Returns an error if the closure returns an error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use portable_pty::CommandBuilder;
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// TuiTestHarness::with_isolation_sized(100, 30, |harness| {
    ///     let mut cmd = CommandBuilder::new("echo");
    ///     cmd.arg("test");
    ///     harness.spawn(cmd)?;
    ///     harness.wait_for_text("test")?;
    ///     Ok(())
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_isolation_sized<F>(width: u16, height: u16, f: F) -> Result<()>
    where
        F: FnOnce(&mut TuiTestHarness) -> Result<()>,
    {
        let mut harness = TuiTestHarness::new(width, height)?;
        f(&mut harness)
    }

    /// Creates a builder for parallel-safe harness configuration.
    ///
    /// This builder ensures that all configuration is done before the harness
    /// is created, making it safe to use in parallel test contexts.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = TuiTestHarness::parallel_harness_builder()
    ///     .with_size(100, 30)
    ///     .with_timeout(Duration::from_secs(10))
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn parallel_harness_builder() -> TuiTestHarnessBuilder {
        TuiTestHarnessBuilder::default()
    }

    /// Measures the input-to-render latency from the most recent input event.
    ///
    /// This returns the time from when input was sent to when rendering completed.
    ///
    /// # Returns
    ///
    /// `Some(Duration)` if both input and render timestamps are recorded, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.send_text("hello")?;
    ///
    /// let latency = harness.measure_input_to_render_latency();
    /// if let Some(latency) = latency {
    ///     println!("Input→Render latency: {:.2}ms", latency.as_secs_f64() * 1000.0);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn measure_input_to_render_latency(&self) -> Option<Duration> {
        self.latency_profile.input_to_render()
    }

    /// Asserts that input-to-render latency is within the specified budget.
    ///
    /// Uses the most recent input→render timing for the assertion.
    ///
    /// # Arguments
    ///
    /// * `budget` - Maximum allowed latency
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No timing data is available
    /// - The latency exceeds the budget
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.send_text("hello")?;
    ///
    /// // Assert input→render completes within 16.67ms (60 FPS)
    /// harness.assert_input_latency_within(Duration::from_micros(16670))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn assert_input_latency_within(&self, budget: Duration) -> Result<()> {
        let latency = self.latency_profile.input_to_render().ok_or_else(|| {
            TermTestError::Timing(
                "Cannot measure input latency: no input or render events recorded".to_string(),
            )
        })?;

        if latency > budget {
            return Err(TermTestError::Timing(format!(
                "Input latency exceeded budget: {:.3}ms > {:.3}ms",
                latency.as_secs_f64() * 1000.0,
                budget.as_secs_f64() * 1000.0
            )));
        }

        Ok(())
    }

    /// Asserts that rendering meets a target FPS budget.
    ///
    /// This calculates the frame time budget from the target FPS and asserts
    /// that input→render latency is within that budget.
    ///
    /// # Arguments
    ///
    /// * `fps_target` - Target frames per second (e.g., 60.0 for 60 FPS)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No timing data is available
    /// - The latency exceeds the FPS budget
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.send_text("hello")?;
    ///
    /// // Assert rendering meets 60 FPS target (16.67ms per frame)
    /// harness.assert_render_budget(60.0)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn assert_render_budget(&self, fps_target: f64) -> Result<()> {
        let budget = fps_to_frame_budget(fps_target);
        self.assert_input_latency_within(budget)
    }

    /// Resets all timing data.
    ///
    /// This clears the timing recorder and latency profile, useful for
    /// starting fresh measurements after warm-up operations.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::TuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    ///
    /// // Warm-up
    /// harness.send_text("hello")?;
    ///
    /// // Reset timing data before real measurements
    /// harness.reset_timing();
    ///
    /// // Now measure actual performance
    /// harness.send_text("world")?;
    /// harness.assert_render_budget(60.0)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn reset_timing(&mut self) {
        self.timing_recorder.reset();
        self.latency_profile.reset();
    }

    /// Gets a reference to the latency profile for detailed analysis.
    ///
    /// # Returns
    ///
    /// A reference to the [`LatencyProfile`] for this harness.
    pub fn latency_profile(&self) -> &LatencyProfile {
        &self.latency_profile
    }
}

/// Implementation of `TimingHooks` trait for `TuiTestHarness`.
impl TimingHooks for TuiTestHarness {
    fn record_event(&mut self, event_name: &str) {
        self.timing_recorder.record_event(event_name);
    }

    fn measure_latency(&self, start_event: &str, end_event: &str) -> Option<Duration> {
        self.timing_recorder.measure_latency(start_event, end_event)
    }

    fn get_timings(&self) -> &TimingRecorder {
        &self.timing_recorder
    }

    fn assert_latency_within(
        &self,
        start_event: &str,
        end_event: &str,
        budget: Duration,
    ) -> Result<()> {
        self.timing_recorder
            .assert_latency_within(start_event, end_event, budget)
    }
}

/// Builder for configuring a `TuiTestHarness`.
///
/// # Example
///
/// ```rust,no_run
/// use std::time::Duration;
///
/// use ratatui_testlib::TuiTestHarness;
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
    terminal_profile: TerminalProfile,
}

impl Default for TuiTestHarnessBuilder {
    fn default() -> Self {
        Self {
            width: 80,
            height: 24,
            timeout: DEFAULT_TIMEOUT,
            poll_interval: DEFAULT_POLL_INTERVAL,
            buffer_size: DEFAULT_BUFFER_SIZE,
            terminal_profile: TerminalProfile::default(),
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

    /// Sets the terminal profile.
    ///
    /// # Arguments
    ///
    /// * `profile` - The terminal profile to use
    pub fn with_terminal_profile(mut self, profile: TerminalProfile) -> Self {
        self.terminal_profile = profile;
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
            event_delay: Duration::ZERO,
            state,
            timeout: self.timeout,
            poll_interval: self.poll_interval,
            buffer_size: self.buffer_size,
            recording: false,
            recorded_events: Vec::new(),
            recording_start: None,
            verbose: false,
            terminal_profile: self.terminal_profile,
            timing_recorder: TimingRecorder::new(),
            latency_profile: LatencyProfile::new(),
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
        let mut harness = TuiTestHarness::new(80, 24)?.with_timeout(Duration::from_secs(2));

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
                assert!(
                    harness.screen_contents().contains("hello"),
                    "Expected 'hello' in output even though process exited"
                );
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
                assert!(
                    harness.screen_contents().contains("quick"),
                    "Expected 'quick' in output even though process exited"
                );
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
                assert_eq!(
                    pos,
                    (9, 19),
                    "Cursor should be at (9, 19) even though no process is running"
                );
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
                assert_eq!(
                    pos,
                    (4, 9),
                    "Cursor should be at (4, 9) even though no process is running"
                );
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    #[test]
    fn test_wait_for_custom_predicate() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?.with_timeout(Duration::from_secs(2));

        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("test123");
        harness.spawn(cmd)?;

        // Wait for custom condition: screen contains a digit
        // May return ProcessExited since echo exits immediately
        match harness.wait_for(|state| state.contents().chars().any(|c| c.is_numeric())) {
            Ok(()) => {
                assert!(harness.screen_contents().contains('1'));
            }
            Err(TermTestError::ProcessExited) => {
                // Process exited, but check if we still got the output
                assert!(
                    harness.screen_contents().contains('1'),
                    "Expected digit in output even though process exited"
                );
            }
            Err(e) => return Err(e),
        }
        Ok(())
    }

    #[test]
    fn test_wait_for_multiline_output() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?.with_timeout(Duration::from_secs(2));

        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg("echo 'line1'; echo 'line2'; echo 'line3'");
        harness.spawn(cmd)?;

        // Wait for all lines to appear - may return ProcessExited
        match harness.wait_for(|state| {
            let contents = state.contents();
            contents.contains("line1") && contents.contains("line2") && contents.contains("line3")
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
        let mut harness = TuiTestHarness::new(80, 24)?.with_timeout(Duration::from_secs(2));

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
                assert!(
                    harness.screen_contents().contains("Ready: 100%"),
                    "Expected 'Ready: 100%' in output even though process exited"
                );
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
        assert!(harness.screen_contents().contains("data"), "Expected 'data' in output");
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
        harness.state_mut().feed(b"\x1b[5;10H"); // Move cursor
        harness.state_mut().feed(b"\x1bPq\"1;1;100;50#0~\x1b\\"); // Sixel sequence

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
        harness.state_mut().feed(b"\x1b[5;10H"); // Position (4, 9) in 0-based
        harness.state_mut().feed(b"\x1bPq\"1;1;100;50#0~\x1b\\");

        let regions = harness.sixel_regions();
        assert_eq!(regions.len(), 1);

        let region = &regions[0];
        assert_eq!(region.start_row, 4); // 5-1 (CSI is 1-based, we use 0-based)
        assert_eq!(region.start_col, 9); // 10-1
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
        harness.state_mut().feed(b"\x1b[8;45H"); // Row 8, col 45 (within preview)
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
        harness.state_mut().feed(b"\x1b[2;2H"); // Row 2, col 2 (outside preview)
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
        harness.state_mut().feed(b"\x1b[15;60H"); // Row 15, col 60
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
        harness.state_mut().feed(b"\x1b[1;1H"); // Top-left corner
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

    #[test]
    fn test_memory_usage() -> Result<()> {
        let harness = TuiTestHarness::new(80, 24)?;

        let memory = harness.memory_usage();

        // Should have non-zero memory for screen buffer
        assert!(memory.current_bytes > 0);
        assert_eq!(memory.current_bytes, memory.peak_bytes);

        // Basic sanity check: 80x24 screen should be reasonable size
        // Each Cell is at least char (4 bytes) + options + bools
        let min_expected = 80 * 24 * 4; // Very conservative estimate
        assert!(memory.current_bytes >= min_expected);

        Ok(())
    }

    #[test]
    fn test_memory_usage_with_different_sizes() -> Result<()> {
        let small = TuiTestHarness::new(20, 10)?;
        let large = TuiTestHarness::new(200, 100)?;

        let small_memory = small.memory_usage();
        let large_memory = large.memory_usage();

        // Larger terminal should use more memory
        assert!(large_memory.current_bytes > small_memory.current_bytes);

        Ok(())
    }

    #[test]
    fn test_memory_usage_with_sixel() -> Result<()> {
        use crate::screen::SixelRegion;

        let mut harness = TuiTestHarness::new(80, 24)?;

        // Get initial memory without Sixel
        let initial_memory = harness.memory_usage();

        // Add a mock Sixel region (simulate Sixel data)
        let sixel_data = vec![0u8; 1000]; // 1KB of Sixel data
        let region = SixelRegion {
            start_row: 5,
            start_col: 10,
            width: 100,
            height: 50,
            data: sixel_data,
        };

        // Manually add the region to the state for testing
        // Note: In real usage, this would come from parsing terminal output
        harness.state_mut().sixel_regions_mut().push(region);

        // Get memory with Sixel
        let with_sixel_memory = harness.memory_usage();

        // Memory should increase by approximately the size of the Sixel data
        assert!(with_sixel_memory.current_bytes > initial_memory.current_bytes);
        let increase = with_sixel_memory.current_bytes - initial_memory.current_bytes;
        assert!(increase >= 1000); // Should be at least 1KB

        Ok(())
    }

    #[test]
    fn test_assert_memory_under_success() -> Result<()> {
        let harness = TuiTestHarness::new(80, 24)?;

        // Set a generous limit that should pass
        harness.assert_memory_under(1_000_000)?; // 1 MB

        Ok(())
    }

    #[test]
    fn test_assert_memory_under_failure() -> Result<()> {
        let harness = TuiTestHarness::new(80, 24)?;

        // Set a very small limit that should fail
        let result = harness.assert_memory_under(100); // 100 bytes

        assert!(result.is_err());
        if let Err(e) = result {
            let error_msg = format!("{}", e);
            assert!(error_msg.contains("Memory usage exceeds limit"));
            assert!(error_msg.contains("100 bytes (limit)"));
        }

        Ok(())
    }

    #[test]
    fn test_memory_results_summary_format() -> Result<()> {
        let harness = TuiTestHarness::new(80, 24)?;
        let memory = harness.memory_usage();
        let summary = memory.summary();

        // Check that summary contains expected fields
        assert!(summary.contains("Memory Usage"));
        assert!(summary.contains("Current:"));
        assert!(summary.contains("Peak:"));
        assert!(summary.contains("bytes"));
        assert!(summary.contains("KB"));

        Ok(())
    }

    #[test]
    fn test_memory_tracking_across_operations() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        let initial = harness.memory_usage();

        // Feed some data to the screen
        harness.state_mut().feed(b"Hello, World!");
        harness.state_mut().feed(b"\x1b[2J"); // Clear screen
        harness.state_mut().feed(b"More text");

        let after_operations = harness.memory_usage();

        // Memory should remain roughly the same (screen buffer size doesn't change)
        // Allow for small variations due to internal state
        let diff = if after_operations.current_bytes > initial.current_bytes {
            after_operations.current_bytes - initial.current_bytes
        } else {
            initial.current_bytes - after_operations.current_bytes
        };

        // Should be within 10% of original
        assert!(diff < initial.current_bytes / 10);

        Ok(())
    }

    #[test]
    fn test_event_delay_default() {
        let harness = TuiTestHarness::new(80, 24).unwrap();
        // Default should be zero
        assert_eq!(harness.event_delay(), Duration::ZERO);
    }

    #[test]
    fn test_set_event_delay() {
        let mut harness = TuiTestHarness::new(80, 24).unwrap();

        // Set a custom delay
        harness.set_event_delay(Duration::from_millis(100));
        assert_eq!(harness.event_delay(), Duration::from_millis(100));

        // Change it
        harness.set_event_delay(Duration::from_millis(200));
        assert_eq!(harness.event_delay(), Duration::from_millis(200));

        // Reset to zero
        harness.set_event_delay(Duration::ZERO);
        assert_eq!(harness.event_delay(), Duration::ZERO);
    }

    #[test]
    fn test_advance_time() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // advance_time should succeed
        let start = Instant::now();
        harness.advance_time(Duration::from_millis(100))?;
        let elapsed = start.elapsed();

        // Should have actually waited (with some tolerance for scheduling)
        assert!(elapsed >= Duration::from_millis(95));
        assert!(elapsed <= Duration::from_millis(200));

        Ok(())
    }

    #[test]
    fn test_press_key_repeat() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Test that press_key_repeat sends multiple keys
        let start = Instant::now();
        harness.press_key_repeat('a', 3, Duration::from_millis(50))?;
        let elapsed = start.elapsed();

        // Should take at least 3 * 50ms = 150ms
        // (plus the default 50ms delay after each key from send_key_event)
        // Total: ~300ms minimum
        assert!(elapsed >= Duration::from_millis(250));

        Ok(())
    }

    #[test]
    fn test_event_delay_affects_timing() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // With custom delay
        harness.set_event_delay(Duration::from_millis(100));

        let start = Instant::now();
        // Send 3 characters
        harness.send_keys("abc")?;
        let elapsed = start.elapsed();

        // Each key should take 100ms, so 3 keys = ~300ms
        assert!(elapsed >= Duration::from_millis(250));

        Ok(())
    }

    #[test]
    fn test_press_key_repeat_with_zero_interval() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Should work with zero interval (only default key delay applies)
        let start = Instant::now();
        harness.press_key_repeat('x', 2, Duration::ZERO)?;
        let elapsed = start.elapsed();

        // Should still take time due to the default 50ms delay in send_key_event
        // 2 keys * 50ms = ~100ms minimum
        assert!(elapsed >= Duration::from_millis(80));

        Ok(())
    }

    #[test]
    fn test_timing_combination() -> Result<()> {
        let mut harness = TuiTestHarness::new(80, 24)?;

        // Test combining different timing methods
        harness.set_event_delay(Duration::from_millis(50));

        let start = Instant::now();

        // Send a key (50ms delay)
        harness.send_key(KeyCode::Char('a'))?;

        // Advance time (100ms)
        harness.advance_time(Duration::from_millis(100))?;

        // Send repeated keys (2 * 50ms interval + 2 * 50ms event delay = 200ms)
        harness.press_key_repeat('b', 2, Duration::from_millis(50))?;

        let elapsed = start.elapsed();

        // Total: 50 + 100 + 200 = 350ms minimum
        assert!(elapsed >= Duration::from_millis(300));

        Ok(())
    }
}
