//! Unified graphics protocol detection and testing support.
//!
//! This module provides comprehensive support for testing graphics protocols in terminal
//! emulators, including Sixel, Kitty, and iTerm2 image protocols. It enables position
//! tracking, bounds checking, and validation of graphics rendering in TUI applications.
//!
//! # Supported Protocols
//!
//! - **Sixel**: DCS-based bitmap graphics (ESC P q ... ESC \)
//! - **Kitty**: Advanced graphics protocol with APC sequences (ESC _ G ... ESC \)
//! - **iTerm2**: Inline images via OSC 1337 (ESC ] 1337;File=... BEL)
//!
//! # Overview
//!
//! Graphics protocols allow terminal emulators to display images and other visual
//! content beyond plain text. This module helps test TUI applications that use these
//! protocols by:
//!
//! - Capturing graphics sequences from terminal output
//! - Tracking position and dimensions of graphics
//! - Validating that graphics appear within expected bounds
//! - Detecting graphics clearing on screen transitions
//! - Supporting protocol-specific queries and filtering
//!
//! # Key Types
//!
//! - [`GraphicsProtocol`]: Enum identifying the protocol type
//! - [`GraphicsRegion`]: Represents a single graphic with position/bounds
//! - [`GraphicsCapture`]: Collection of captured graphics with query methods
//!
//! # Example
//!
//! ```rust
//! # #[cfg(feature = "sixel")]
//! # {
//! use ratatui_testlib::{
//!     graphics::{GraphicsCapture, GraphicsProtocol},
//!     ScreenState, TuiTestHarness,
//! };
//!
//! # fn test_graphics() -> ratatui_testlib::Result<()> {
//! let mut harness = TuiTestHarness::new(80, 24)?;
//! // ... spawn app and render graphics ...
//!
//! // Define the preview area where graphics should appear
//! let preview_area = (5, 5, 30, 15); // (row, col, width, height)
//!
//! // Check that all graphics are within bounds
//! let capture = GraphicsCapture::from_screen_state(harness.state());
//! capture.assert_all_within(preview_area)?;
//!
//! // Filter by protocol type
//! let sixel_graphics = capture.by_protocol(GraphicsProtocol::Sixel);
//! assert_eq!(sixel_graphics.len(), 1);
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! # Protocol Details
//!
//! ## Sixel
//!
//! Sixel is a bitmap graphics format supported by terminals like XTerm, WezTerm, and foot.
//! Sequences are wrapped in DCS (Device Control String):
//!
//! ```text
//! ESC P q <raster_attributes> <sixel_data> ESC \
//! ```
//!
//! Raster attributes specify dimensions in the format `"Pan;Pad;Ph;Pv` where:
//! - `Ph`: Horizontal pixel dimension (width)
//! - `Pv`: Vertical pixel dimension (height)
//!
//! ## Kitty Graphics Protocol
//!
//! Kitty's graphics protocol uses APC (Application Program Command) sequences:
//!
//! ```text
//! ESC _ G <control_data> ; <payload> ESC \
//! ```
//!
//! Control data includes key-value pairs for actions, formats, dimensions, etc.
//! The protocol supports multiple transmission methods and progressive loading.
//!
//! ## iTerm2 Inline Images
//!
//! iTerm2 uses OSC (Operating System Command) sequences for inline images:
//!
//! ```text
//! ESC ] 1337 ; File = <params> : <base64_data> BEL
//! ```
//!
//! Parameters include filename, width, height, inline status, and preserve aspect ratio.

use crate::error::{Result, TermTestError};

/// Identifies the graphics protocol used for rendering.
///
/// Different terminal emulators support different graphics protocols. This enum
/// allows filtering and protocol-specific handling of graphics regions.
///
/// # Protocol Compatibility
///
/// - **Sixel**: XTerm, WezTerm, foot, mlterm, yaft, and many others
/// - **Kitty**: Kitty terminal and compatible emulators (WezTerm experimental)
/// - **iTerm2**: iTerm2 (macOS), WezTerm, Hyper, Tabby
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::graphics::GraphicsProtocol;
///
/// let protocol = GraphicsProtocol::Sixel;
/// assert_eq!(protocol.name(), "Sixel");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GraphicsProtocol {
    /// Sixel bitmap graphics protocol (DCS-based).
    Sixel,
    /// Kitty advanced graphics protocol (APC-based).
    Kitty,
    /// iTerm2 inline images protocol (OSC 1337-based).
    ITerm2,
}

impl GraphicsProtocol {
    /// Returns the human-readable name of the protocol.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::GraphicsProtocol;
    ///
    /// assert_eq!(GraphicsProtocol::Sixel.name(), "Sixel");
    /// assert_eq!(GraphicsProtocol::Kitty.name(), "Kitty");
    /// assert_eq!(GraphicsProtocol::ITerm2.name(), "iTerm2");
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            GraphicsProtocol::Sixel => "Sixel",
            GraphicsProtocol::Kitty => "Kitty",
            GraphicsProtocol::ITerm2 => "iTerm2",
        }
    }

    /// Returns the escape sequence prefix for this protocol.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::GraphicsProtocol;
    ///
    /// assert_eq!(GraphicsProtocol::Sixel.escape_prefix(), "\x1bPq");
    /// assert_eq!(GraphicsProtocol::Kitty.escape_prefix(), "\x1b_G");
    /// assert_eq!(GraphicsProtocol::ITerm2.escape_prefix(), "\x1b]1337;File=");
    /// ```
    pub fn escape_prefix(&self) -> &'static str {
        match self {
            GraphicsProtocol::Sixel => "\x1bPq",           // DCS q
            GraphicsProtocol::Kitty => "\x1b_G",           // APC G
            GraphicsProtocol::ITerm2 => "\x1b]1337;File=", // OSC 1337;File=
        }
    }
}

impl std::fmt::Display for GraphicsProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Represents a captured graphics region with position information.
///
/// This is the core type for graphics testing, tracking where graphics are
/// rendered on the screen along with their dimensions and protocol type.
///
/// # Fields
///
/// - `protocol`: The graphics protocol used for this region
/// - `position`: Cursor position when the graphic was rendered (row, col) in terminal cells
/// - `bounds`: Calculated bounding rectangle (row, col, width, height) in terminal cells
/// - `raw_data`: The raw escape sequence bytes (including protocol wrapper)
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::graphics::{GraphicsProtocol, GraphicsRegion};
///
/// let region = GraphicsRegion::new(
///     GraphicsProtocol::Sixel,
///     (5, 10),           // position
///     (5, 10, 100, 50),  // bounds
///     vec![/* raw bytes */],
/// );
///
/// // Check if within a preview area
/// let preview_area = (0, 0, 200, 100);
/// assert!(region.is_within(preview_area));
/// assert_eq!(region.protocol, GraphicsProtocol::Sixel);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GraphicsRegion {
    /// The graphics protocol used for this region.
    pub protocol: GraphicsProtocol,
    /// Cursor position when the graphic was rendered (row, col).
    pub position: (u16, u16),
    /// Calculated bounding rectangle (row, col, width, height).
    pub bounds: (u16, u16, u16, u16),
    /// Raw escape sequence bytes (including protocol wrapper).
    pub raw_data: Vec<u8>,
}

impl GraphicsRegion {
    /// Creates a new graphics region.
    ///
    /// # Arguments
    ///
    /// * `protocol` - Graphics protocol used
    /// * `position` - Cursor position when rendered
    /// * `bounds` - Bounding rectangle (row, col, width, height)
    /// * `raw_data` - Raw escape sequence bytes
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::{GraphicsProtocol, GraphicsRegion};
    ///
    /// let region = GraphicsRegion::new(
    ///     GraphicsProtocol::Kitty,
    ///     (10, 20),
    ///     (10, 20, 15, 10),
    ///     vec![0x1b, b'_', b'G'],
    /// );
    /// ```
    pub fn new(
        protocol: GraphicsProtocol,
        position: (u16, u16),
        bounds: (u16, u16, u16, u16),
        raw_data: Vec<u8>,
    ) -> Self {
        Self { protocol, position, bounds, raw_data }
    }

    /// Checks if this graphic is completely within the specified area.
    ///
    /// Returns `true` only if the entire bounding rectangle fits within
    /// the given area. This is useful for verifying that graphics don't overflow
    /// their designated regions.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Returns
    ///
    /// `true` if the graphic is entirely within the area, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::{GraphicsProtocol, GraphicsRegion};
    ///
    /// let region = GraphicsRegion::new(GraphicsProtocol::Sixel, (5, 5), (5, 5, 10, 10), vec![]);
    ///
    /// let area = (0, 0, 20, 20);
    /// assert!(region.is_within(area)); // Completely inside
    ///
    /// let small_area = (0, 0, 10, 10);
    /// assert!(!region.is_within(small_area)); // Extends beyond
    /// ```
    pub fn is_within(&self, area: (u16, u16, u16, u16)) -> bool {
        let (row, col, width, height) = self.bounds;
        let (area_row, area_col, area_width, area_height) = area;

        row >= area_row
            && col >= area_col
            && (row + height) <= (area_row + area_height)
            && (col + width) <= (area_col + area_width)
    }

    /// Checks if this graphic overlaps with the specified area.
    ///
    /// Returns `true` if any part of the bounding rectangle intersects
    /// with the given area. This is useful for detecting unwanted graphics in
    /// certain screen regions.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Returns
    ///
    /// `true` if the graphic overlaps with the area, `false` if completely separate.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::{GraphicsProtocol, GraphicsRegion};
    ///
    /// let region = GraphicsRegion::new(GraphicsProtocol::ITerm2, (5, 5), (5, 5, 10, 10), vec![]);
    ///
    /// assert!(region.overlaps((0, 0, 10, 10))); // Partial overlap
    /// assert!(region.overlaps((10, 10, 10, 10))); // Edge overlap
    /// assert!(!region.overlaps((0, 0, 5, 5))); // No overlap
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

/// Captures all graphics sequences from terminal output.
///
/// This type provides methods for querying and validating graphics across all
/// supported protocols. It's the main interface for graphics testing, offering:
///
/// - Query methods to find graphics by location or protocol
/// - Validation methods to assert correct positioning
/// - Comparison methods to detect graphics clearing
/// - Protocol filtering for protocol-specific tests
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "sixel")]
/// # {
/// use ratatui_testlib::{
///     graphics::{GraphicsCapture, GraphicsProtocol},
///     ScreenState,
/// };
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let screen = ScreenState::new(80, 24);
/// let capture = GraphicsCapture::from_screen_state(&screen);
///
/// // Verify no graphics outside preview area
/// let preview_area = (5, 5, 30, 20);
/// capture.assert_all_within(preview_area)?;
///
/// // Check for Sixel graphics specifically
/// let sixel_count = capture.by_protocol(GraphicsProtocol::Sixel).len();
/// println!("Found {} Sixel graphics", sixel_count);
///
/// // Get all graphics in a specific region
/// let regions = capture.regions_in_area(preview_area);
/// println!("Found {} graphics in preview", regions.len());
/// # Ok(())
/// # }
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct GraphicsCapture {
    /// All captured graphics regions.
    regions: Vec<GraphicsRegion>,
}

impl GraphicsCapture {
    /// Creates a new empty graphics capture.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::GraphicsCapture;
    ///
    /// let capture = GraphicsCapture::new();
    /// assert!(capture.is_empty());
    /// ```
    pub fn new() -> Self {
        Self { regions: Vec::new() }
    }

    /// Creates a graphics capture from a ScreenState.
    ///
    /// This extracts all detected graphics sequences from the screen state,
    /// including Sixel, Kitty, and iTerm2 graphics.
    ///
    /// # Arguments
    ///
    /// * `screen` - Reference to the ScreenState containing graphics information
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::{graphics::GraphicsCapture, ScreenState};
    ///
    /// let screen = ScreenState::new(80, 24);
    /// let capture = GraphicsCapture::from_screen_state(&screen);
    /// assert!(capture.is_empty());
    /// ```
    pub fn from_screen_state(screen: &crate::screen::ScreenState) -> Self {
        let mut regions = Vec::new();

        // Convert Sixel regions to GraphicsRegions
        for sixel_region in screen.sixel_regions() {
            const PIXELS_PER_COL: u32 = 8;
            const PIXELS_PER_ROW: u32 = 6;

            let width_cells = if sixel_region.width > 0 {
                ((sixel_region.width + PIXELS_PER_COL - 1) / PIXELS_PER_COL) as u16
            } else {
                0
            };

            let height_cells = if sixel_region.height > 0 {
                ((sixel_region.height + PIXELS_PER_ROW - 1) / PIXELS_PER_ROW) as u16
            } else {
                0
            };

            regions.push(GraphicsRegion::new(
                GraphicsProtocol::Sixel,
                (sixel_region.start_row, sixel_region.start_col),
                (sixel_region.start_row, sixel_region.start_col, width_cells, height_cells),
                sixel_region.data.clone(),
            ));
        }

        // Convert Kitty regions to GraphicsRegions
        for kitty_region in screen.kitty_regions() {
            const PIXELS_PER_COL: u32 = 8;
            const PIXELS_PER_ROW: u32 = 6;

            let width_cells = if kitty_region.width > 0 {
                ((kitty_region.width + PIXELS_PER_COL - 1) / PIXELS_PER_COL) as u16
            } else {
                0
            };

            let height_cells = if kitty_region.height > 0 {
                ((kitty_region.height + PIXELS_PER_ROW - 1) / PIXELS_PER_ROW) as u16
            } else {
                0
            };

            regions.push(GraphicsRegion::new(
                GraphicsProtocol::Kitty,
                (kitty_region.start_row, kitty_region.start_col),
                (kitty_region.start_row, kitty_region.start_col, width_cells, height_cells),
                kitty_region.data.clone(),
            ));
        }

        // Convert iTerm2 regions to GraphicsRegions
        // iTerm2 dimensions are already in cells, not pixels
        for iterm2_region in screen.iterm2_regions() {
            let width_cells = iterm2_region.width as u16;
            let height_cells = iterm2_region.height as u16;

            regions.push(GraphicsRegion::new(
                GraphicsProtocol::ITerm2,
                (iterm2_region.start_row, iterm2_region.start_col),
                (iterm2_region.start_row, iterm2_region.start_col, width_cells, height_cells),
                iterm2_region.data.clone(),
            ));
        }

        Self { regions }
    }

    /// Returns all captured graphics regions.
    ///
    /// # Returns
    ///
    /// A slice containing all graphics regions captured from the screen state.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::GraphicsCapture;
    ///
    /// let capture = GraphicsCapture::new();
    /// let regions = capture.regions();
    /// println!("Captured {} graphics", regions.len());
    /// ```
    pub fn regions(&self) -> &[GraphicsRegion] {
        &self.regions
    }

    /// Checks if any graphics were captured.
    ///
    /// # Returns
    ///
    /// `true` if no graphics were captured, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::GraphicsCapture;
    ///
    /// let capture = GraphicsCapture::new();
    /// assert!(capture.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// Returns regions that are completely within the specified area.
    ///
    /// This filters the captured regions to only those whose bounding
    /// rectangles are entirely contained within the given area.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Returns
    ///
    /// A vector of references to regions within the area.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::GraphicsCapture;
    ///
    /// let capture = GraphicsCapture::new();
    /// let preview_area = (5, 5, 30, 20);
    /// let regions = capture.regions_in_area(preview_area);
    /// println!("Found {} graphics in preview area", regions.len());
    /// ```
    pub fn regions_in_area(&self, area: (u16, u16, u16, u16)) -> Vec<&GraphicsRegion> {
        self.regions.iter().filter(|r| r.is_within(area)).collect()
    }

    /// Returns regions that are not completely within the specified area.
    ///
    /// This is the inverse of [`regions_in_area`](Self::regions_in_area).
    /// It returns regions that extend beyond the area boundaries.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Returns
    ///
    /// A vector of references to regions outside or partially outside the area.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::GraphicsCapture;
    ///
    /// let capture = GraphicsCapture::new();
    /// let preview_area = (5, 5, 30, 20);
    /// let outside = capture.regions_outside_area(preview_area);
    /// assert_eq!(outside.len(), 0, "No graphics should be outside preview area");
    /// ```
    pub fn regions_outside_area(&self, area: (u16, u16, u16, u16)) -> Vec<&GraphicsRegion> {
        self.regions.iter().filter(|r| !r.is_within(area)).collect()
    }

    /// Filters regions by graphics protocol.
    ///
    /// Returns only the regions that use the specified graphics protocol.
    ///
    /// # Arguments
    ///
    /// * `protocol` - The protocol to filter by
    ///
    /// # Returns
    ///
    /// A vector of references to regions using the specified protocol.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::{GraphicsCapture, GraphicsProtocol};
    ///
    /// let capture = GraphicsCapture::new();
    /// let sixel_regions = capture.by_protocol(GraphicsProtocol::Sixel);
    /// let kitty_regions = capture.by_protocol(GraphicsProtocol::Kitty);
    ///
    /// println!("Sixel: {}, Kitty: {}", sixel_regions.len(), kitty_regions.len());
    /// ```
    pub fn by_protocol(&self, protocol: GraphicsProtocol) -> Vec<&GraphicsRegion> {
        self.regions
            .iter()
            .filter(|r| r.protocol == protocol)
            .collect()
    }

    /// Returns the count of regions using a specific protocol.
    ///
    /// # Arguments
    ///
    /// * `protocol` - The protocol to count
    ///
    /// # Returns
    ///
    /// The number of regions using the specified protocol.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::graphics::{GraphicsCapture, GraphicsProtocol};
    ///
    /// let capture = GraphicsCapture::new();
    /// assert_eq!(capture.count_by_protocol(GraphicsProtocol::Sixel), 0);
    /// ```
    pub fn count_by_protocol(&self, protocol: GraphicsProtocol) -> usize {
        self.regions
            .iter()
            .filter(|r| r.protocol == protocol)
            .count()
    }

    /// Asserts that all graphics regions are within the specified area.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Errors
    ///
    /// Returns an error if any region is outside the area.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use ratatui_testlib::graphics::GraphicsCapture;
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let capture = GraphicsCapture::new();
    /// let preview_area = (5, 5, 30, 20);
    /// capture.assert_all_within(preview_area)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn assert_all_within(&self, area: (u16, u16, u16, u16)) -> Result<()> {
        let outside = self.regions_outside_area(area);
        if !outside.is_empty() {
            let details: Vec<_> = outside
                .iter()
                .map(|r| format!("{} at {:?}", r.protocol, r.position))
                .collect();

            return Err(TermTestError::SixelValidation(format!(
                "Found {} graphics region(s) outside area {:?}: [{}]",
                outside.len(),
                area,
                details.join(", ")
            )));
        }
        Ok(())
    }

    /// Asserts that at least one region of the specified protocol exists.
    ///
    /// # Arguments
    ///
    /// * `protocol` - The protocol to check for
    ///
    /// # Errors
    ///
    /// Returns an error if no regions of the specified protocol are found.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use ratatui_testlib::graphics::{GraphicsCapture, GraphicsProtocol};
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let capture = GraphicsCapture::new();
    /// // This will fail since capture is empty
    /// // capture.assert_protocol_exists(GraphicsProtocol::Sixel)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn assert_protocol_exists(&self, protocol: GraphicsProtocol) -> Result<()> {
        if self.count_by_protocol(protocol) == 0 {
            return Err(TermTestError::SixelValidation(format!(
                "No {} graphics found",
                protocol.name()
            )));
        }
        Ok(())
    }

    /// Checks if this capture differs from another.
    ///
    /// This method compares two captures to detect changes in graphics state,
    /// which is useful for verifying that graphics are cleared on screen
    /// transitions.
    ///
    /// # Arguments
    ///
    /// * `other` - Other capture to compare with
    ///
    /// # Returns
    ///
    /// `true` if the captures contain different graphics, `false` if identical.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::{graphics::GraphicsCapture, ScreenState};
    ///
    /// let screen1 = ScreenState::new(80, 24);
    /// let capture1 = GraphicsCapture::from_screen_state(&screen1);
    ///
    /// // ... screen transition occurs ...
    ///
    /// let screen2 = ScreenState::new(80, 24);
    /// let capture2 = GraphicsCapture::from_screen_state(&screen2);
    ///
    /// // Verify graphics state changed during transition
    /// if capture1.differs_from(&capture2) {
    ///     println!("Graphics state changed during transition");
    /// }
    /// ```
    pub fn differs_from(&self, other: &GraphicsCapture) -> bool {
        self.regions != other.regions
    }
}

impl Default for GraphicsCapture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ITerm2Region, KittyRegion, ScreenState, SixelRegion};

    #[test]
    fn test_protocol_name() {
        assert_eq!(GraphicsProtocol::Sixel.name(), "Sixel");
        assert_eq!(GraphicsProtocol::Kitty.name(), "Kitty");
        assert_eq!(GraphicsProtocol::ITerm2.name(), "iTerm2");
    }

    #[test]
    fn test_protocol_escape_prefix() {
        assert_eq!(GraphicsProtocol::Sixel.escape_prefix(), "\x1bPq");
        assert_eq!(GraphicsProtocol::Kitty.escape_prefix(), "\x1b_G");
        assert_eq!(GraphicsProtocol::ITerm2.escape_prefix(), "\x1b]1337;File=");
    }

    #[test]
    fn test_protocol_display() {
        assert_eq!(format!("{}", GraphicsProtocol::Sixel), "Sixel");
        assert_eq!(format!("{}", GraphicsProtocol::Kitty), "Kitty");
        assert_eq!(format!("{}", GraphicsProtocol::ITerm2), "iTerm2");
    }

    #[test]
    fn test_region_within() {
        let region = GraphicsRegion::new(GraphicsProtocol::Sixel, (5, 5), (5, 5, 10, 10), vec![]);

        assert!(region.is_within((0, 0, 20, 20)));
        assert!(!region.is_within((0, 0, 10, 10)));
        assert!(region.is_within((5, 5, 10, 10)));
    }

    #[test]
    fn test_region_overlaps() {
        let region = GraphicsRegion::new(GraphicsProtocol::Kitty, (5, 5), (5, 5, 10, 10), vec![]);

        assert!(region.overlaps((0, 0, 10, 10)));
        assert!(region.overlaps((10, 10, 10, 10)));
        assert!(!region.overlaps((0, 0, 5, 5)));
        assert!(region.overlaps((5, 5, 10, 10)));
    }

    #[test]
    fn test_capture_empty() {
        let capture = GraphicsCapture::new();
        assert!(capture.is_empty());
        assert_eq!(capture.regions().len(), 0);
    }

    #[test]
    fn test_capture_filtering() {
        let mut capture = GraphicsCapture::new();

        capture.regions.push(GraphicsRegion::new(
            GraphicsProtocol::Sixel,
            (5, 5),
            (5, 5, 10, 10),
            vec![],
        ));

        capture.regions.push(GraphicsRegion::new(
            GraphicsProtocol::Kitty,
            (20, 20),
            (20, 20, 10, 10),
            vec![],
        ));

        capture.regions.push(GraphicsRegion::new(
            GraphicsProtocol::ITerm2,
            (10, 10),
            (10, 10, 5, 5),
            vec![],
        ));

        let area = (0, 0, 15, 15);
        assert_eq!(capture.regions_in_area(area).len(), 2); // Sixel and iTerm2
        assert_eq!(capture.regions_outside_area(area).len(), 1); // Kitty
    }

    #[test]
    fn test_capture_by_protocol() {
        let mut capture = GraphicsCapture::new();

        capture.regions.push(GraphicsRegion::new(
            GraphicsProtocol::Sixel,
            (5, 5),
            (5, 5, 10, 10),
            vec![],
        ));

        capture.regions.push(GraphicsRegion::new(
            GraphicsProtocol::Sixel,
            (15, 15),
            (15, 15, 10, 10),
            vec![],
        ));

        capture.regions.push(GraphicsRegion::new(
            GraphicsProtocol::Kitty,
            (20, 20),
            (20, 20, 10, 10),
            vec![],
        ));

        assert_eq!(capture.by_protocol(GraphicsProtocol::Sixel).len(), 2);
        assert_eq!(capture.by_protocol(GraphicsProtocol::Kitty).len(), 1);
        assert_eq!(capture.by_protocol(GraphicsProtocol::ITerm2).len(), 0);
    }

    #[test]
    fn test_count_by_protocol() {
        let mut capture = GraphicsCapture::new();

        capture.regions.push(GraphicsRegion::new(
            GraphicsProtocol::Sixel,
            (5, 5),
            (5, 5, 10, 10),
            vec![],
        ));

        capture.regions.push(GraphicsRegion::new(
            GraphicsProtocol::Kitty,
            (10, 10),
            (10, 10, 10, 10),
            vec![],
        ));

        assert_eq!(capture.count_by_protocol(GraphicsProtocol::Sixel), 1);
        assert_eq!(capture.count_by_protocol(GraphicsProtocol::Kitty), 1);
        assert_eq!(capture.count_by_protocol(GraphicsProtocol::ITerm2), 0);
    }

    #[test]
    fn test_assert_all_within_success() {
        let mut capture = GraphicsCapture::new();

        capture.regions.push(GraphicsRegion::new(
            GraphicsProtocol::Sixel,
            (5, 5),
            (5, 5, 10, 10),
            vec![],
        ));

        let area = (0, 0, 20, 20);
        assert!(capture.assert_all_within(area).is_ok());
    }

    #[test]
    fn test_assert_all_within_failure() {
        let mut capture = GraphicsCapture::new();

        capture.regions.push(GraphicsRegion::new(
            GraphicsProtocol::Sixel,
            (5, 5),
            (5, 5, 10, 10),
            vec![],
        ));

        let area = (0, 0, 10, 10);
        let result = capture.assert_all_within(area);
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_protocol_exists() {
        let mut capture = GraphicsCapture::new();

        capture.regions.push(GraphicsRegion::new(
            GraphicsProtocol::Sixel,
            (5, 5),
            (5, 5, 10, 10),
            vec![],
        ));

        assert!(capture
            .assert_protocol_exists(GraphicsProtocol::Sixel)
            .is_ok());
        assert!(capture
            .assert_protocol_exists(GraphicsProtocol::Kitty)
            .is_err());
    }

    #[test]
    fn test_differs_from() {
        let mut capture1 = GraphicsCapture::new();
        capture1.regions.push(GraphicsRegion::new(
            GraphicsProtocol::Sixel,
            (5, 5),
            (5, 5, 10, 10),
            vec![],
        ));

        let capture2 = GraphicsCapture::new();

        assert!(capture1.differs_from(&capture2));
        assert!(capture2.differs_from(&capture1));
        assert!(!capture1.differs_from(&capture1));
    }

    #[test]
    fn test_from_screen_state_with_sixel() {
        let mut screen = ScreenState::new(80, 24);

        // Add a mock Sixel region
        screen.sixel_regions_mut().push(SixelRegion {
            start_row: 5,
            start_col: 10,
            width: 100,
            height: 60,
            data: vec![0x1b, b'P', b'q'],
        });

        let capture = GraphicsCapture::from_screen_state(&screen);
        assert_eq!(capture.regions().len(), 1);
        assert_eq!(capture.count_by_protocol(GraphicsProtocol::Sixel), 1);

        let region = &capture.regions()[0];
        assert_eq!(region.protocol, GraphicsProtocol::Sixel);
        assert_eq!(region.position, (5, 10));
    }

    #[test]
    fn test_from_screen_state_with_kitty() {
        let mut screen = ScreenState::new(80, 24);

        // Add a mock Kitty region
        screen.kitty_regions_mut().push(KittyRegion {
            start_row: 8,
            start_col: 15,
            width: 200,
            height: 100,
            data: vec![b'G', b'w', b'=', b'2', b'0', b'0'],
        });

        let capture = GraphicsCapture::from_screen_state(&screen);
        assert_eq!(capture.regions().len(), 1);
        assert_eq!(capture.count_by_protocol(GraphicsProtocol::Kitty), 1);

        let region = &capture.regions()[0];
        assert_eq!(region.protocol, GraphicsProtocol::Kitty);
        assert_eq!(region.position, (8, 15));
    }

    #[test]
    fn test_from_screen_state_with_iterm2() {
        let mut screen = ScreenState::new(80, 24);

        // Add a mock iTerm2 region
        screen.iterm2_regions_mut().push(ITerm2Region {
            start_row: 12,
            start_col: 20,
            width: 30,
            height: 15,
            data: vec![b'1', b'3', b'3', b'7'],
        });

        let capture = GraphicsCapture::from_screen_state(&screen);
        assert_eq!(capture.regions().len(), 1);
        assert_eq!(capture.count_by_protocol(GraphicsProtocol::ITerm2), 1);

        let region = &capture.regions()[0];
        assert_eq!(region.protocol, GraphicsProtocol::ITerm2);
        assert_eq!(region.position, (12, 20));
    }

    #[test]
    fn test_from_screen_state_with_multiple_protocols() {
        let mut screen = ScreenState::new(80, 24);

        // Add regions from all protocols
        screen.sixel_regions_mut().push(SixelRegion {
            start_row: 5,
            start_col: 5,
            width: 80,
            height: 60,
            data: vec![],
        });

        screen.kitty_regions_mut().push(KittyRegion {
            start_row: 10,
            start_col: 10,
            width: 160,
            height: 120,
            data: vec![],
        });

        screen.iterm2_regions_mut().push(ITerm2Region {
            start_row: 15,
            start_col: 15,
            width: 20,
            height: 10,
            data: vec![],
        });

        let capture = GraphicsCapture::from_screen_state(&screen);
        assert_eq!(capture.regions().len(), 3);
        assert_eq!(capture.count_by_protocol(GraphicsProtocol::Sixel), 1);
        assert_eq!(capture.count_by_protocol(GraphicsProtocol::Kitty), 1);
        assert_eq!(capture.count_by_protocol(GraphicsProtocol::ITerm2), 1);
    }

    #[test]
    fn test_mixed_protocol_area_filtering() {
        let mut screen = ScreenState::new(80, 24);

        // Add graphics in different areas
        screen.sixel_regions_mut().push(SixelRegion {
            start_row: 5,
            start_col: 5,
            width: 80,  // 10 cells
            height: 60, // 10 cells
            data: vec![],
        });

        screen.kitty_regions_mut().push(KittyRegion {
            start_row: 20,
            start_col: 20,
            width: 80,
            height: 60,
            data: vec![],
        });

        let capture = GraphicsCapture::from_screen_state(&screen);

        // Define an area that only contains the Sixel
        let area = (0, 0, 20, 20);
        let in_area = capture.regions_in_area(area);
        assert_eq!(in_area.len(), 1);
        assert_eq!(in_area[0].protocol, GraphicsProtocol::Sixel);

        let outside = capture.regions_outside_area(area);
        assert_eq!(outside.len(), 1);
        assert_eq!(outside[0].protocol, GraphicsProtocol::Kitty);
    }
}
