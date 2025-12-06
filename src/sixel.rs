//! Sixel graphics testing support.
//!
//! This module provides functionality for detecting, parsing, and validating
//! Sixel escape sequences in terminal output, with a focus on position tracking
//! and bounds checking.
//!
//! # Overview
//!
//! Sixel is a bitmap graphics format supported by some terminal emulators. This
//! module helps test TUI applications that render Sixel graphics by:
//!
//! - Capturing Sixel sequences from terminal output
//! - Tracking position and dimensions of Sixel graphics
//! - Validating that Sixel graphics appear within expected bounds
//! - Detecting Sixel clearing on screen transitions
//!
//! # Key Types
//!
//! - [`SixelSequence`]: Represents a single Sixel graphic with position/bounds
//! - [`SixelCapture`]: Collection of captured Sixel sequences with query methods
//!
//! # Migration to Graphics Module
//!
//! This module now re-exports the unified graphics API from [`crate::graphics`].
//! The new graphics module supports multiple protocols (Sixel, Kitty, iTerm2)
//! with a consistent API. For backwards compatibility, this module maintains
//! the original Sixel-specific types.
//!
//! To use the unified API:
//! ```rust
//! # #[cfg(feature = "sixel")]
//! # {
//! use ratatui_testlib::graphics::{GraphicsCapture, GraphicsProtocol};
//!
//! # fn example() -> ratatui_testlib::Result<()> {
//! # let screen = ratatui_testlib::ScreenState::new(80, 24);
//! let capture = GraphicsCapture::from_screen_state(&screen);
//! let sixel_regions = capture.by_protocol(GraphicsProtocol::Sixel);
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! # Example
//!
//! ```rust
//! # #[cfg(feature = "sixel")]
//! # {
//! use ratatui_testlib::{ScreenState, TuiTestHarness};
//!
//! # fn test_sixel() -> ratatui_testlib::Result<()> {
//! let mut harness = TuiTestHarness::new(80, 24)?;
//! // ... spawn app and render Sixel graphics ...
//!
//! // Define the preview area where Sixel graphics should appear
//! let preview_area = (5, 5, 30, 15); // (row, col, width, height)
//!
//! // Check that all Sixel graphics are within bounds
//! let regions = harness.state().sixel_regions();
//! for region in regions {
//!     let within_bounds = region.start_row >= preview_area.0
//!         && region.start_col >= preview_area.1
//!         && (region.start_row as u32 + region.height / 6)
//!             <= (preview_area.0 as u32 + preview_area.3 as u32)
//!         && (region.start_col as u32 + region.width / 8)
//!             <= (preview_area.1 as u32 + preview_area.2 as u32);
//!     assert!(
//!         within_bounds,
//!         "Sixel at ({}, {}) is outside preview area",
//!         region.start_row, region.start_col
//!     );
//! }
//! # Ok(())
//! # }
//! # }
//! ```

use crate::{
    error::{Result, TermTestError},
    graphics::{GraphicsCapture as UnifiedGraphicsCapture, GraphicsProtocol, GraphicsRegion},
};

/// Represents a captured Sixel sequence with position information.
///
/// This is the core type for Sixel testing, tracking where Sixel graphics
/// are rendered on the screen along with their dimensions.
///
/// # Fields
///
/// - `raw`: The raw Sixel escape sequence bytes (including DCS wrapper)
/// - `position`: Cursor position when the Sixel was rendered (row, col) in terminal cells
/// - `bounds`: Calculated bounding rectangle (row, col, width, height) in terminal cells
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::sixel::SixelSequence;
///
/// let seq = SixelSequence::new(
///     vec![/* raw bytes */],
///     (5, 10),           // position
///     (5, 10, 100, 50),  // bounds
/// );
///
/// // Check if within a preview area
/// let preview_area = (0, 0, 200, 100);
/// assert!(seq.is_within(preview_area));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SixelSequence {
    /// Raw Sixel escape sequence bytes (including DCS wrapper).
    pub raw: Vec<u8>,
    /// Cursor position when the Sixel was rendered (row, col).
    pub position: (u16, u16),
    /// Calculated bounding rectangle (row, col, width, height).
    pub bounds: (u16, u16, u16, u16),
}

impl SixelSequence {
    /// Creates a new Sixel sequence.
    ///
    /// # Arguments
    ///
    /// * `raw` - Raw escape sequence bytes
    /// * `position` - Cursor position when rendered
    /// * `bounds` - Bounding rectangle (row, col, width, height)
    pub fn new(raw: Vec<u8>, position: (u16, u16), bounds: (u16, u16, u16, u16)) -> Self {
        Self { raw, position, bounds }
    }

    /// Checks if this Sixel is completely within the specified area.
    ///
    /// Returns `true` only if the entire Sixel bounding rectangle fits within
    /// the given area. This is useful for verifying that graphics don't overflow
    /// their designated regions.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Returns
    ///
    /// `true` if the Sixel is entirely within the area, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::sixel::SixelSequence;
    ///
    /// let seq = SixelSequence::new(vec![], (5, 5), (5, 5, 10, 10));
    /// let area = (0, 0, 20, 20);
    ///
    /// assert!(seq.is_within(area)); // Completely inside
    ///
    /// let small_area = (0, 0, 10, 10);
    /// assert!(!seq.is_within(small_area)); // Extends beyond
    /// ```
    pub fn is_within(&self, area: (u16, u16, u16, u16)) -> bool {
        let (row, col, width, height) = self.bounds;
        let (area_row, area_col, area_width, area_height) = area;

        row >= area_row
            && col >= area_col
            && (row + height) <= (area_row + area_height)
            && (col + width) <= (area_col + area_width)
    }

    /// Checks if this Sixel overlaps with the specified area.
    ///
    /// Returns `true` if any part of the Sixel bounding rectangle intersects
    /// with the given area. This is useful for detecting unwanted graphics in
    /// certain screen regions.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Returns
    ///
    /// `true` if the Sixel overlaps with the area, `false` if completely separate.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::sixel::SixelSequence;
    ///
    /// let seq = SixelSequence::new(vec![], (5, 5), (5, 5, 10, 10));
    ///
    /// assert!(seq.overlaps((0, 0, 10, 10))); // Partial overlap
    /// assert!(seq.overlaps((10, 10, 10, 10))); // Edge overlap
    /// assert!(!seq.overlaps((0, 0, 5, 5))); // No overlap
    /// ```
    pub fn overlaps(&self, area: (u16, u16, u16, u16)) -> bool {
        let (row, col, width, height) = self.bounds;
        let (area_row, area_col, area_width, area_height) = area;

        !(row + height <= area_row
            || col + width <= area_col
            || row >= area_row + area_height
            || col >= area_col + area_width)
    }
}

/// Captures all Sixel sequences from terminal output.
///
/// This type provides methods for querying and validating Sixel graphics
/// in the terminal screen state. It's the main interface for Sixel testing,
/// offering:
///
/// - Query methods to find Sixel graphics by location
/// - Validation methods to assert correct positioning
/// - Comparison methods to detect Sixel clearing
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "sixel")]
/// # {
/// use ratatui_testlib::{sixel::SixelCapture, ScreenState};
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let screen = ScreenState::new(80, 24);
/// let capture = SixelCapture::from_screen_state(&screen);
///
/// // Verify no Sixel graphics outside preview area
/// let preview_area = (5, 5, 30, 20);
/// capture.assert_all_within(preview_area)?;
///
/// // Check for Sixel graphics in specific region
/// let sequences = capture.sequences_in_area(preview_area);
/// println!("Found {} Sixel graphics in preview", sequences.len());
/// # Ok(())
/// # }
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SixelCapture {
    /// All captured Sixel sequences.
    sequences: Vec<SixelSequence>,
}

impl SixelCapture {
    /// Creates a new empty Sixel capture.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::sixel::SixelCapture;
    ///
    /// let capture = SixelCapture::new();
    /// assert!(capture.is_empty());
    /// ```
    pub fn new() -> Self {
        Self { sequences: Vec::new() }
    }

    /// Creates a Sixel capture from raw terminal output.
    ///
    /// This parses the output and extracts all Sixel sequences with their positions.
    ///
    /// # Arguments
    ///
    /// * `output` - Raw terminal output bytes
    /// * `cursor_positions` - Cursor positions corresponding to each sequence
    ///
    /// # Note
    ///
    /// Phase 1 implementation is a stub. Full Sixel parsing will be implemented
    /// in Phase 3 after validating vt100 capabilities.
    pub fn from_output(_output: &[u8], _cursor_positions: &[(u16, u16)]) -> Self {
        // TODO: Phase 3 - Implement Sixel sequence detection and parsing
        // This requires:
        // 1. Scanning for Sixel escape sequences (ESC P ... ESC \)
        // 2. Parsing Sixel data to extract dimensions
        // 3. Associating cursor positions with sequences
        // 4. Calculating bounding rectangles
        Self::new()
    }

    /// Creates a Sixel capture from a ScreenState.
    ///
    /// This extracts all detected Sixel sequences from the screen state.
    ///
    /// # Arguments
    ///
    /// * `screen` - Reference to the ScreenState containing Sixel information
    pub fn from_screen_state(screen: &crate::screen::ScreenState) -> Self {
        // Use the unified graphics capture and filter for Sixel
        let unified = UnifiedGraphicsCapture::from_screen_state(screen);
        let sequences = unified
            .by_protocol(GraphicsProtocol::Sixel)
            .iter()
            .map(|region: &&GraphicsRegion| {
                SixelSequence::new(region.raw_data.clone(), region.position, region.bounds)
            })
            .collect();

        Self { sequences }
    }

    /// Returns all captured sequences.
    ///
    /// # Returns
    ///
    /// A slice containing all Sixel sequences captured from the screen state.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::sixel::SixelCapture;
    ///
    /// let capture = SixelCapture::new();
    /// let sequences = capture.sequences();
    /// println!("Captured {} Sixel graphics", sequences.len());
    /// ```
    pub fn sequences(&self) -> &[SixelSequence] {
        &self.sequences
    }

    /// Checks if any Sixel sequences were captured.
    ///
    /// # Returns
    ///
    /// `true` if no Sixel sequences were captured, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::sixel::SixelCapture;
    ///
    /// let capture = SixelCapture::new();
    /// assert!(capture.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.sequences.is_empty()
    }

    /// Returns sequences that are completely within the specified area.
    ///
    /// This filters the captured sequences to only those whose bounding
    /// rectangles are entirely contained within the given area.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Returns
    ///
    /// A vector of references to sequences within the area.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::sixel::SixelCapture;
    ///
    /// let capture = SixelCapture::new();
    /// let preview_area = (5, 5, 30, 20);
    /// let sequences = capture.sequences_in_area(preview_area);
    /// println!("Found {} graphics in preview area", sequences.len());
    /// ```
    pub fn sequences_in_area(&self, area: (u16, u16, u16, u16)) -> Vec<&SixelSequence> {
        self.sequences
            .iter()
            .filter(|seq| seq.is_within(area))
            .collect()
    }

    /// Returns sequences that are not completely within the specified area.
    ///
    /// This is the inverse of [`sequences_in_area`](Self::sequences_in_area).
    /// It returns sequences that extend beyond the area boundaries.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Returns
    ///
    /// A vector of references to sequences outside or partially outside the area.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::sixel::SixelCapture;
    ///
    /// let capture = SixelCapture::new();
    /// let preview_area = (5, 5, 30, 20);
    /// let outside = capture.sequences_outside_area(preview_area);
    /// assert_eq!(outside.len(), 0, "No graphics should be outside preview area");
    /// ```
    pub fn sequences_outside_area(&self, area: (u16, u16, u16, u16)) -> Vec<&SixelSequence> {
        self.sequences
            .iter()
            .filter(|seq| !seq.is_within(area))
            .collect()
    }

    /// Asserts that all Sixel sequences are within the specified area.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Errors
    ///
    /// Returns an error if any sequence is outside the area.
    pub fn assert_all_within(&self, area: (u16, u16, u16, u16)) -> Result<()> {
        let outside = self.sequences_outside_area(area);
        if !outside.is_empty() {
            return Err(TermTestError::SixelValidation(format!(
                "Found {} Sixel sequence(s) outside area {:?}: {:?}",
                outside.len(),
                area,
                outside.iter().map(|s| s.position).collect::<Vec<_>>()
            )));
        }
        Ok(())
    }

    /// Checks if this capture differs from another.
    ///
    /// This method compares two captures to detect changes in Sixel state,
    /// which is useful for verifying that Sixel graphics are cleared on
    /// screen transitions.
    ///
    /// # Arguments
    ///
    /// * `other` - Other capture to compare with
    ///
    /// # Returns
    ///
    /// `true` if the captures contain different Sixel sequences, `false` if identical.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::{sixel::SixelCapture, ScreenState};
    ///
    /// let screen1 = ScreenState::new(80, 24);
    /// let capture1 = SixelCapture::from_screen_state(&screen1);
    ///
    /// // ... screen transition occurs ...
    ///
    /// let screen2 = ScreenState::new(80, 24);
    /// let capture2 = SixelCapture::from_screen_state(&screen2);
    ///
    /// // Verify Sixel graphics were cleared
    /// if capture1.differs_from(&capture2) {
    ///     println!("Sixel state changed during transition");
    /// }
    /// ```
    pub fn differs_from(&self, other: &SixelCapture) -> bool {
        self.sequences != other.sequences
    }
}

impl Default for SixelCapture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sixel_sequence_within() {
        let seq = SixelSequence::new(vec![], (5, 5), (5, 5, 10, 10));
        assert!(seq.is_within((0, 0, 20, 20)));
        assert!(!seq.is_within((0, 0, 10, 10)));
    }

    #[test]
    fn test_sixel_sequence_overlaps() {
        let seq = SixelSequence::new(vec![], (5, 5), (5, 5, 10, 10));
        assert!(seq.overlaps((0, 0, 10, 10)));
        assert!(seq.overlaps((10, 10, 10, 10)));
        assert!(!seq.overlaps((0, 0, 5, 5)));
    }

    #[test]
    fn test_sixel_capture_empty() {
        let capture = SixelCapture::new();
        assert!(capture.is_empty());
        assert_eq!(capture.sequences().len(), 0);
    }

    #[test]
    fn test_sixel_capture_filtering() {
        let mut capture = SixelCapture::new();
        capture
            .sequences
            .push(SixelSequence::new(vec![], (5, 5), (5, 5, 10, 10)));
        capture
            .sequences
            .push(SixelSequence::new(vec![], (20, 20), (20, 20, 10, 10)));

        let area = (0, 0, 15, 15);
        assert_eq!(capture.sequences_in_area(area).len(), 1);
        assert_eq!(capture.sequences_outside_area(area).len(), 1);
    }
}
