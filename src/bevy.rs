//! Bevy ECS integration for testing Bevy-based TUI applications.
//!
//! This module provides a test harness that wraps both the TUI test harness
//! and a Bevy App, enabling comprehensive testing of applications built with
//! bevy_ratatui or other Bevy-based TUI frameworks.
//!
//! # Overview
//!
//! [`BevyTuiTestHarness`] combines terminal testing with Bevy ECS capabilities:
//!
//! - Run Bevy update cycles frame-by-frame
//! - Query ECS entities and components
//! - Test system execution and state transitions
//! - Verify terminal output from Bevy systems
//! - Test Sixel graphics from Bevy rendering systems
//!
//! # Status
//!
//! This module is currently a stub implementation (Phase 1). Full Bevy integration
//! will be implemented in Phase 4 after core PTY and Sixel support is validated.
//!
//! # Planned Features
//!
//! - Headless Bevy app initialization
//! - Frame-by-frame update control
//! - ECS entity/component queries
//! - System execution testing
//! - Integration with bevy_ratatui plugin
//!
//! # Example
//!
//! ```rust,no_run
//! # #[cfg(feature = "bevy")]
//! # {
//! use mimic::BevyTuiTestHarness;
//!
//! # fn test() -> mimic::Result<()> {
//! let mut test = BevyTuiTestHarness::new()?;
//!
//! // Run one Bevy frame
//! test.update()?;
//!
//! // Run multiple frames
//! test.update_n(5)?;
//!
//! // Render and check screen state
//! test.render_frame()?;
//! let state = test.state();
//! assert!(state.contains("Game Over"));
//! # Ok(())
//! # }
//! # }
//! ```

use crate::error::{Result, TermTestError};
use crate::harness::TuiTestHarness;
use crate::screen::ScreenState;

#[cfg(feature = "sixel")]
use crate::sixel::SixelCapture;

/// Test harness for Bevy-based TUI applications.
///
/// This combines TUI testing with Bevy ECS querying and update cycle control,
/// specifically designed for testing applications built with bevy_ratatui.
///
/// # Current Status
///
/// This is a Phase 1 stub implementation. The full Bevy integration will be
/// added in Phase 4 after validating core PTY and Sixel capabilities.
///
/// # Planned Architecture
///
/// - Wraps a [`TuiTestHarness`] for terminal I/O
/// - Contains a headless Bevy App for ECS operations
/// - Provides frame-by-frame control of update cycles
/// - Exposes ECS query methods for testing
///
/// # Example
///
/// ```rust,no_run
/// # #[cfg(feature = "bevy")]
/// # {
/// use mimic::BevyTuiTestHarness;
///
/// # fn test() -> mimic::Result<()> {
/// let mut harness = BevyTuiTestHarness::new()?;
///
/// // Run one frame
/// harness.update()?;
///
/// // Send input
/// harness.send_text("quit\n")?;
///
/// // Wait for result
/// harness.wait_for(|state| state.contains("Goodbye"))?;
/// # Ok(())
/// # }
/// # }
/// ```
pub struct BevyTuiTestHarness {
    harness: TuiTestHarness,
    // TODO: Phase 4 - Add Bevy App field
    // app: bevy::app::App,
}

impl BevyTuiTestHarness {
    /// Creates a new Bevy TUI test harness.
    ///
    /// Initializes a new test harness with default terminal dimensions (80x24).
    /// In Phase 4, this will also initialize a headless Bevy App.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal or Bevy initialization fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use mimic::BevyTuiTestHarness;
    ///
    /// # fn test() -> mimic::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn new() -> Result<Self> {
        let harness = TuiTestHarness::new(80, 24)?;

        // TODO: Phase 4 - Initialize Bevy App
        // This will include:
        // - Creating a headless Bevy app
        // - Setting up minimal plugins
        // - Disabling rendering

        Ok(Self { harness })
    }

    /// Creates a Bevy TUI test harness with bevy_ratatui plugin.
    ///
    /// This is a convenience method for the common case of testing
    /// applications built with bevy_ratatui.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[cfg(feature = "bevy-ratatui")]
    pub fn with_bevy_ratatui() -> Result<Self> {
        // TODO: Phase 4 - Initialize with bevy_ratatui plugin
        Self::new()
    }

    /// Runs one Bevy frame update.
    ///
    /// This executes all Bevy systems for one frame. In Phase 4, this will
    /// call `app.update()` on the contained Bevy App.
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use mimic::BevyTuiTestHarness;
    ///
    /// # fn test() -> mimic::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness.update()?; // Run one frame
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn update(&mut self) -> Result<()> {
        // TODO: Phase 4 - Implement Bevy update
        // This will call app.update()
        Ok(())
    }

    /// Runs N Bevy frame updates.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of frames to update
    ///
    /// # Errors
    ///
    /// Returns an error if any update fails.
    pub fn update_n(&mut self, count: usize) -> Result<()> {
        for _ in 0..count {
            self.update()?;
        }
        Ok(())
    }

    /// Updates Bevy and renders to the terminal.
    ///
    /// This is equivalent to one complete frame: update ECS, then render to PTY.
    ///
    /// # Errors
    ///
    /// Returns an error if update or render fails.
    pub fn render_frame(&mut self) -> Result<()> {
        // TODO: Phase 4 - Implement frame rendering
        // This will:
        // 1. Run app.update()
        // 2. Trigger bevy_ratatui rendering
        // 3. Update harness screen state
        self.update()?;
        self.harness.update_state()?;
        Ok(())
    }

    /// Sends keyboard input (delegates to inner harness).
    ///
    /// # Arguments
    ///
    /// * `text` - Text to send
    ///
    /// # Errors
    ///
    /// Returns an error if sending fails.
    pub fn send_text(&mut self, text: &str) -> Result<()> {
        self.harness.send_text(text)
    }

    /// Returns the current screen state.
    ///
    /// Provides access to the terminal screen state for inspecting rendered
    /// output from Bevy systems.
    ///
    /// # Returns
    ///
    /// A reference to the current [`ScreenState`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use mimic::BevyTuiTestHarness;
    ///
    /// # fn test() -> mimic::Result<()> {
    /// let harness = BevyTuiTestHarness::new()?;
    /// let state = harness.state();
    /// println!("Cursor at: {:?}", state.cursor_position());
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn state(&self) -> &ScreenState {
        self.harness.state()
    }

    /// Waits for a screen condition (delegates to inner harness).
    ///
    /// This method repeatedly checks the screen state until the condition is met
    /// or the timeout expires. Useful for waiting for Bevy systems to render output.
    ///
    /// # Arguments
    ///
    /// * `condition` - Condition function that receives the current screen state
    ///
    /// # Errors
    ///
    /// Returns a [`TermTestError::Timeout`] if the condition is not met within
    /// the configured timeout.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use mimic::BevyTuiTestHarness;
    ///
    /// # fn test() -> mimic::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness.update()?;
    ///
    /// // Wait for specific text to appear
    /// harness.wait_for(|state| {
    ///     state.contains("Ready")
    /// })?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn wait_for<F>(&mut self, condition: F) -> Result<()>
    where
        F: Fn(&ScreenState) -> bool,
    {
        self.harness.wait_for(condition)
    }

    /// Checks if Sixel graphics are present in the current screen state.
    #[cfg(feature = "sixel")]
    pub fn has_sixel_graphics(&self) -> bool {
        // TODO: Phase 3 - Implement Sixel detection
        // This will check the screen state for Sixel sequences
        false
    }

    /// Captures the current Sixel state.
    ///
    /// # Errors
    ///
    /// Returns an error if capture fails.
    #[cfg(feature = "sixel")]
    pub fn capture_sixel_state(&self) -> Result<SixelCapture> {
        // TODO: Phase 3 - Implement Sixel capture
        Ok(SixelCapture::new())
    }

    /// Asserts that all Sixel graphics are within the specified area.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Errors
    ///
    /// Returns an error if any Sixel is outside the area.
    #[cfg(feature = "sixel")]
    pub fn assert_sixel_within(&self, area: (u16, u16, u16, u16)) -> Result<()> {
        let capture = self.capture_sixel_state()?;
        capture.assert_all_within(area)
    }

    /// Asserts that no Sixel graphics are outside the specified area.
    ///
    /// This is the inverse of `assert_sixel_within`.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Errors
    ///
    /// Returns an error if any Sixel is outside the area.
    #[cfg(feature = "sixel")]
    pub fn assert_no_sixel_outside(&self, area: (u16, u16, u16, u16)) -> Result<()> {
        self.assert_sixel_within(area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_bevy_harness() {
        let harness = BevyTuiTestHarness::new();
        assert!(harness.is_ok());
    }

    #[test]
    fn test_update() {
        let mut harness = BevyTuiTestHarness::new().unwrap();
        assert!(harness.update().is_ok());
    }

    #[test]
    fn test_update_n() {
        let mut harness = BevyTuiTestHarness::new().unwrap();
        assert!(harness.update_n(5).is_ok());
    }
}
