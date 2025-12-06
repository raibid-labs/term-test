//! Terminal screen state management using vtparse with Sixel support.
//!
//! This module provides the core terminal emulation layer that tracks screen contents,
//! cursor position, and Sixel graphics regions. It uses the [`vtparse`] crate to parse
//! VT100/ANSI escape sequences.
//!
//! # Key Types
//!
//! - [`ScreenState`]: The main screen state tracking type
//! - [`SixelRegion`]: Represents a Sixel graphics region with position and dimension info
//!
//! # Usage Modes
//!
//! ## 1. Stream-Based Parsing (Zero PTY Overhead)
//!
//! [`ScreenState`] can be used as a headless parser for terminal emulator testing.
//! This is ideal for integration testing where you need to verify terminal behavior
//! against deterministic byte sequences.
//!
//! ```rust
//! use ratatui_testlib::ScreenState;
//!
//! // Create a parser without any PTY
//! let mut screen = ScreenState::new(80, 24);
//!
//! // Feed raw byte sequences directly
//! let input = b"\x1b[31mHello\x1b[0m";
//! screen.feed(input);
//!
//! // Query the parsed state
//! assert!(screen.contains("Hello"));
//! assert_eq!(screen.get_cell(0, 0).unwrap().fg, Some(1)); // Red color
//! ```
//!
//! ## 2. PTY-Based Testing (Full TUI Integration)
//!
//! For testing complete TUI applications, use [`crate::TuiTestHarness`] which
//! combines [`ScreenState`] with PTY management.
//!
//! ```rust,no_run
//! use portable_pty::CommandBuilder;
//! use ratatui_testlib::TuiTestHarness;
//!
//! let mut harness = TuiTestHarness::new(80, 24)?;
//! let cmd = CommandBuilder::new("my-tui-app");
//! harness.spawn(cmd)?;
//! harness.wait_for_text("Welcome")?;
//! # Ok::<(), ratatui_testlib::TermTestError>(())
//! ```
//!
//! # Example: Verification Oracle
//!
//! Use [`ScreenState`] as a reference implementation to verify other terminal emulators:
//!
//! ```rust
//! use ratatui_testlib::ScreenState;
//!
//! // Define a deterministic test sequence
//! let test_sequence = b"\x1b[2J\x1b[H\x1b[31mTest\x1b[0m";
//!
//! // Create oracle
//! let mut oracle = ScreenState::new(80, 24);
//! oracle.feed(test_sequence);
//!
//! // Now compare your system-under-test against the oracle:
//! // - Text: oracle.contents()
//! // - Cursor: oracle.cursor_position()
//! // - Attributes: oracle.get_cell(row, col)
//! // - Sixel regions: oracle.sixel_regions()
//! ```

use vtparse::{CsiParam, VTActor, VTParser};

/// Represents a single terminal cell with character and attributes.
///
/// This struct tracks the complete state of a terminal cell including:
/// - The character being displayed
/// - Foreground color (ANSI color code, 0-255, or None for default)
/// - Background color (ANSI color code, 0-255, or None for default)
/// - Text attributes (bold, italic, underline, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// The character displayed in this cell
    pub c: char,
    /// Foreground color (None = default, Some(0-255) = ANSI color)
    pub fg: Option<u8>,
    /// Background color (None = default, Some(0-255) = ANSI color)
    pub bg: Option<u8>,
    /// Bold attribute
    pub bold: bool,
    /// Italic attribute
    pub italic: bool,
    /// Underline attribute
    pub underline: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            c: ' ',
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
        }
    }
}

/// A rectangular area in terminal coordinate space.
///
/// Represents a rectangular region with a position and size. This is compatible
/// with `ratatui::layout::Rect` and is used for position assertions and bounds
/// checking.
///
/// # Coordinate System
///
/// - Positions are 0-indexed
/// - `x` is the column (horizontal position)
/// - `y` is the row (vertical position)
/// - `width` and `height` define the area dimensions
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::Rect;
///
/// // Create a rectangle at (5, 10) with size 30x20
/// let rect = Rect::new(5, 10, 30, 20);
/// assert_eq!(rect.x, 5);
/// assert_eq!(rect.y, 10);
/// assert_eq!(rect.width, 30);
/// assert_eq!(rect.height, 20);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rect {
    /// X coordinate (column, 0-indexed).
    pub x: u16,
    /// Y coordinate (row, 0-indexed).
    pub y: u16,
    /// Width in columns.
    pub width: u16,
    /// Height in rows.
    pub height: u16,
}

impl Rect {
    /// Creates a new rectangle with the given position and size.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate (column, 0-indexed)
    /// * `y` - Y coordinate (row, 0-indexed)
    /// * `width` - Width in columns
    /// * `height` - Height in rows
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    /// Returns the right edge (x + width).
    #[inline]
    pub const fn right(&self) -> u16 {
        self.x.saturating_add(self.width)
    }

    /// Returns the bottom edge (y + height).
    #[inline]
    pub const fn bottom(&self) -> u16 {
        self.y.saturating_add(self.height)
    }

    /// Checks if this rectangle contains the given point.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate (column)
    /// * `y` - Y coordinate (row)
    pub const fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    /// Checks if this rectangle completely contains another rectangle.
    ///
    /// # Arguments
    ///
    /// * `other` - The rectangle to check
    pub const fn contains_rect(&self, other: &Rect) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    /// Checks if this rectangle intersects with another rectangle.
    ///
    /// # Arguments
    ///
    /// * `other` - The rectangle to check for intersection
    pub const fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right()
            && self.right() > other.x
            && self.y < other.bottom()
            && self.bottom() > other.y
    }
}

/// Represents a Sixel graphics region in the terminal.
///
/// Sixel is a bitmap graphics format used by terminals to display images.
/// This struct tracks the position and dimensions of Sixel graphics rendered
/// on the screen, which is essential for verifying that graphics appear in
/// the correct locations (e.g., within preview areas).
///
/// # Fields
///
/// - `start_row`: The row where the Sixel begins (0-indexed)
/// - `start_col`: The column where the Sixel begins (0-indexed)
/// - `width`: Width of the Sixel image in pixels
/// - `height`: Height of the Sixel image in pixels
/// - `data`: The raw Sixel escape sequence data
///
/// # Example
///
/// ```rust
/// # use ratatui_testlib::ScreenState;
/// let mut screen = ScreenState::new(80, 24);
///
/// // After rendering a Sixel image...
/// let regions = screen.sixel_regions();
/// for region in regions {
///     println!(
///         "Sixel at ({}, {}), size {}x{}",
///         region.start_row, region.start_col, region.width, region.height
///     );
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SixelRegion {
    /// Starting row (0-indexed).
    pub start_row: u16,
    /// Starting column (0-indexed).
    pub start_col: u16,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Raw Sixel escape sequence data.
    pub data: Vec<u8>,
}

/// Represents a Kitty graphics region in the terminal.
///
/// Kitty graphics protocol is an advanced protocol that supports various
/// image formats and transmission methods.
///
/// # Fields
///
/// - `start_row`: The row where the graphic begins (0-indexed)
/// - `start_col`: The column where the graphic begins (0-indexed)
/// - `width`: Width in pixels (if known from control data)
/// - `height`: Height in pixels (if known from control data)
/// - `data`: The raw APC escape sequence data
#[derive(Debug, Clone)]
pub struct KittyRegion {
    /// Starting row (0-indexed).
    pub start_row: u16,
    /// Starting column (0-indexed).
    pub start_col: u16,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Raw Kitty graphics escape sequence data.
    pub data: Vec<u8>,
}

/// Represents an iTerm2 inline image region in the terminal.
///
/// iTerm2 inline images use OSC 1337;File= sequences to embed
/// base64-encoded images directly in terminal output.
///
/// # Fields
///
/// - `start_row`: The row where the image begins (0-indexed)
/// - `start_col`: The column where the image begins (0-indexed)
/// - `width`: Width in cells (if specified in params)
/// - `height`: Height in cells (if specified in params)
/// - `data`: The raw OSC escape sequence data
#[derive(Debug, Clone)]
pub struct ITerm2Region {
    /// Starting row (0-indexed).
    pub start_row: u16,
    /// Starting column (0-indexed).
    pub start_col: u16,
    /// Width in cells.
    pub width: u32,
    /// Height in cells.
    pub height: u32,
    /// Raw iTerm2 inline image escape sequence data.
    pub data: Vec<u8>,
}

/// A complete snapshot of the terminal screen grid state.
///
/// This structure provides a point-in-time capture of the entire screen state,
/// including all cells with their characters and attributes, the cursor position,
/// and grid dimensions. It's designed for deep comparison between terminal
/// emulators or for serialization/debugging purposes.
///
/// # Fields
///
/// - `width`: Screen width in columns
/// - `height`: Screen height in rows
/// - `cells`: 2D vector of cells (row-major order: `cells[row][col]`)
/// - `cursor`: Current cursor position as (row, col), both 0-indexed
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::ScreenState;
///
/// let mut screen = ScreenState::new(80, 24);
/// screen.feed(b"\x1b[31mTest");
///
/// let snapshot = screen.snapshot();
///
/// // Compare against another emulator
/// for row in 0..snapshot.height {
///     for col in 0..snapshot.width {
///         let cell = &snapshot.cells[row as usize][col as usize];
///         println!("({}, {}): '{}' fg={:?} bg={:?}", row, col, cell.c, cell.fg, cell.bg);
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridSnapshot {
    /// Screen width in columns.
    pub width: u16,
    /// Screen height in rows.
    pub height: u16,
    /// Complete grid of cells in row-major order: `cells[row][col]`.
    pub cells: Vec<Vec<Cell>>,
    /// Cursor position as (row, col), both 0-indexed.
    pub cursor: (u16, u16),
}

/// Terminal state tracking for vtparse parser.
///
/// Implements VTActor to handle escape sequences including DCS for Sixel,
/// APC for Kitty graphics, and OSC for iTerm2 inline images.
struct TerminalState {
    cursor_pos: (u16, u16),
    sixel_regions: Vec<SixelRegion>,
    current_sixel_data: Vec<u8>,
    current_sixel_params: Vec<i64>,
    in_sixel_mode: bool,

    // Kitty graphics protocol state
    kitty_regions: Vec<KittyRegion>,
    current_kitty_data: Vec<u8>,
    in_kitty_mode: bool,

    // iTerm2 inline images state
    iterm2_regions: Vec<ITerm2Region>,
    current_iterm2_data: Vec<u8>,
    in_iterm2_mode: bool,

    width: u16,
    height: u16,
    cells: Vec<Vec<Cell>>,
    /// Current text attributes (for SGR sequences)
    current_fg: Option<u8>,
    current_bg: Option<u8>,
    current_bold: bool,
    current_italic: bool,
    current_underline: bool,
}

impl TerminalState {
    fn new(width: u16, height: u16) -> Self {
        let cells = vec![vec![Cell::default(); width as usize]; height as usize];

        Self {
            cursor_pos: (0, 0),
            sixel_regions: Vec::new(),
            current_sixel_data: Vec::new(),
            current_sixel_params: Vec::new(),
            in_sixel_mode: false,
            kitty_regions: Vec::new(),
            current_kitty_data: Vec::new(),
            in_kitty_mode: false,
            iterm2_regions: Vec::new(),
            current_iterm2_data: Vec::new(),
            in_iterm2_mode: false,
            width,
            height,
            cells,
            current_fg: None,
            current_bg: None,
            current_bold: false,
            current_italic: false,
            current_underline: false,
        }
    }

    fn put_char(&mut self, ch: char) {
        let (row, col) = self.cursor_pos;
        if row < self.height && col < self.width {
            self.cells[row as usize][col as usize] = Cell {
                c: ch,
                fg: self.current_fg,
                bg: self.current_bg,
                bold: self.current_bold,
                italic: self.current_italic,
                underline: self.current_underline,
            };
            // Move cursor forward, but don't wrap automatically
            if col + 1 < self.width {
                self.cursor_pos.1 = col + 1;
            }
        }
    }

    fn move_cursor(&mut self, row: u16, col: u16) {
        self.cursor_pos = (row.min(self.height - 1), col.min(self.width - 1));
    }

    /// Parse raster attributes from sixel data.
    ///
    /// Sixel raster attributes follow the format: "Pan;Pad;Ph;Pv
    /// Where:
    /// - Pan: Pixel aspect ratio numerator (typically 1)
    /// - Pad: Pixel aspect ratio denominator (typically 1)
    /// - Ph: Horizontal pixel dimension (width)
    /// - Pv: Vertical pixel dimension (height)
    ///
    /// # Arguments
    ///
    /// * `data` - Raw sixel data bytes containing raster attributes
    ///
    /// # Returns
    ///
    /// `Some((width, height))` in pixels if raster attributes are found and valid,
    /// `None` otherwise.
    ///
    /// # Examples
    ///
    /// - "1;1;100;50" → Some((100, 50))
    /// - "100;50" → Some((100, 50)) (missing aspect ratio parameters)
    /// - "" → None (no raster attributes)
    fn parse_raster_attributes(&self, data: &[u8]) -> Option<(u32, u32)> {
        let data_str = std::str::from_utf8(data).ok()?;

        // Find the raster attributes command starting with '"'
        let raster_start = data_str.find('"')?;
        let after_quote = &data_str[raster_start + 1..];

        // Find where the raster attributes end (terminated by non-digit, non-semicolon)
        let end_pos = after_quote
            .find(|c: char| !c.is_ascii_digit() && c != ';')
            .unwrap_or(after_quote.len());

        let raster_part = &after_quote[..end_pos];

        // Parse semicolon-separated numeric parameters
        // Format: Pa;Pb;Ph;Pv where we need Ph (index 2) and Pv (index 3)
        let parts: Vec<&str> = raster_part.split(';').filter(|s| !s.is_empty()).collect();

        // Handle different parameter counts:
        // - 4 params: Pan;Pad;Ph;Pv (full format)
        // - 2 params: Ph;Pv (abbreviated format, aspect ratio omitted)
        match parts.len() {
            4 => {
                // Full format: Pan;Pad;Ph;Pv
                let width = parts[2].parse::<u32>().ok()?;
                let height = parts[3].parse::<u32>().ok()?;
                if width > 0 && height > 0 {
                    Some((width, height))
                } else {
                    None
                }
            }
            2 => {
                // Abbreviated format: Ph;Pv
                let width = parts[0].parse::<u32>().ok()?;
                let height = parts[1].parse::<u32>().ok()?;
                if width > 0 && height > 0 {
                    Some((width, height))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Converts pixel dimensions to terminal cell dimensions.
    ///
    /// Uses standard Sixel-to-terminal conversion ratios:
    /// - 8 pixels per column (horizontal)
    /// - 6 pixels per row (vertical - based on Sixel sixel height)
    ///
    /// These ratios are typical for Sixel graphics in VT340-compatible terminals.
    /// Each Sixel band is 6 pixels tall, and character cells are typically 8 pixels wide.
    ///
    /// # Arguments
    ///
    /// * `width_px` - Width in pixels
    /// * `height_px` - Height in pixels
    ///
    /// # Returns
    ///
    /// A tuple of (columns, rows) in terminal cells, with fractional cells rounded up.
    ///
    /// # Examples
    ///
    /// - (80, 60) pixels → (10, 10) cells
    /// - (100, 50) pixels → (13, 9) cells (rounded up)
    /// - (0, 0) pixels → (0, 0) cells
    fn pixels_to_cells(width_px: u32, height_px: u32) -> (u16, u16) {
        // Standard Sixel pixel-to-cell ratios
        const PIXELS_PER_COL: u32 = 8;
        const PIXELS_PER_ROW: u32 = 6;

        let cols = if width_px > 0 {
            ((width_px + PIXELS_PER_COL - 1) / PIXELS_PER_COL) as u16
        } else {
            0
        };

        let rows = if height_px > 0 {
            ((height_px + PIXELS_PER_ROW - 1) / PIXELS_PER_ROW) as u16
        } else {
            0
        };

        (cols, rows)
    }

    /// Parse Kitty graphics control data to extract dimensions.
    ///
    /// Kitty graphics protocol uses key-value pairs like:
    /// - `w=<width>` - width in pixels
    /// - `h=<height>` - height in pixels
    /// - `c=<cols>` - width in terminal cells
    /// - `r=<rows>` - height in terminal cells
    ///
    /// # Arguments
    ///
    /// * `data` - Raw Kitty graphics control data
    ///
    /// # Returns
    ///
    /// `Some((width, height))` in pixels if dimensions are found, `None` otherwise.
    fn parse_kitty_dimensions(&self, data: &[u8]) -> Option<(u32, u32)> {
        let data_str = std::str::from_utf8(data).ok()?;

        let mut width = None;
        let mut height = None;

        // Parse key-value pairs separated by commas or semicolons
        for part in data_str.split(|c| c == ',' || c == ';') {
            if let Some((key, value)) = part.split_once('=') {
                match key.trim() {
                    "w" => width = value.trim().parse::<u32>().ok(),
                    "h" => height = value.trim().parse::<u32>().ok(),
                    _ => {}
                }
            }
        }

        match (width, height) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some((w, h)),
            _ => None,
        }
    }

    /// Parse iTerm2 inline image parameters to extract dimensions.
    ///
    /// iTerm2 uses parameters like:
    /// - `width=<n>` or `width=<n>px` - width in pixels
    /// - `height=<n>` or `height=<n>px` - height in pixels
    /// - `width=auto` or `height=auto` - automatic sizing
    ///
    /// # Arguments
    ///
    /// * `data` - Raw iTerm2 parameter data (before the base64 payload)
    ///
    /// # Returns
    ///
    /// `Some((width, height))` in cells if dimensions are found, `None` otherwise.
    /// Note: iTerm2 dimensions are typically specified in cells, not pixels.
    fn parse_iterm2_dimensions(&self, data: &[u8]) -> Option<(u32, u32)> {
        let data_str = std::str::from_utf8(data).ok()?;

        let mut width = None;
        let mut height = None;

        // Parse semicolon-separated key=value pairs
        for part in data_str.split(';') {
            if let Some((key, value)) = part.split_once('=') {
                match key.trim() {
                    "width" => {
                        // Extract numeric value, ignoring units like "px"
                        let val_str = value.trim().trim_end_matches("px");
                        if val_str != "auto" {
                            width = val_str.parse::<u32>().ok();
                        }
                    }
                    "height" => {
                        let val_str = value.trim().trim_end_matches("px");
                        if val_str != "auto" {
                            height = val_str.parse::<u32>().ok();
                        }
                    }
                    _ => {}
                }
            }
        }

        match (width, height) {
            (Some(w), Some(h)) if w > 0 && h > 0 => Some((w, h)),
            _ => None,
        }
    }
}

impl VTActor for TerminalState {
    fn print(&mut self, ch: char) {
        self.put_char(ch);
    }

    fn execute_c0_or_c1(&mut self, control: u8) {
        match control {
            b'\r' => {
                // Carriage return
                self.cursor_pos.1 = 0;
            }
            b'\n' => {
                // Line feed
                if self.cursor_pos.0 + 1 < self.height {
                    self.cursor_pos.0 += 1;
                }
            }
            b'\t' => {
                // Tab - advance to next tab stop (every 8 columns)
                let next_tab = ((self.cursor_pos.1 / 8) + 1) * 8;
                self.cursor_pos.1 = next_tab.min(self.width - 1);
            }
            _ => {}
        }
    }

    fn dcs_hook(
        &mut self,
        mode: u8,
        params: &[i64],
        _intermediates: &[u8],
        _ignored_excess_intermediates: bool,
    ) {
        // Sixel sequences are identified by mode byte 'q' (0x71)
        if mode == b'q' {
            self.in_sixel_mode = true;
            self.current_sixel_data.clear();
            self.current_sixel_params = params.to_vec();
        }
    }

    fn dcs_put(&mut self, byte: u8) {
        if self.in_sixel_mode {
            self.current_sixel_data.push(byte);
        }
    }

    fn dcs_unhook(&mut self) {
        if self.in_sixel_mode {
            // Parse dimensions from raster attributes if present
            let (width, height) = self
                .parse_raster_attributes(&self.current_sixel_data)
                .unwrap_or((0, 0));

            let region = SixelRegion {
                start_row: self.cursor_pos.0,
                start_col: self.cursor_pos.1,
                width,
                height,
                data: self.current_sixel_data.clone(),
            };
            self.sixel_regions.push(region);

            self.in_sixel_mode = false;
            self.current_sixel_data.clear();
            self.current_sixel_params.clear();
        }
    }

    fn csi_dispatch(&mut self, params: &[CsiParam], _truncated: bool, byte: u8) {
        match byte {
            b'H' | b'f' => {
                // CUP - Cursor Position ESC [ row ; col H
                // CSI uses 1-based indexing, convert to 0-based
                // Filter out P variants (separators) and collect only integers
                let integers: Vec<i64> = params.iter().filter_map(|p| p.as_integer()).collect();

                let row = integers.get(0).copied().unwrap_or(1).saturating_sub(1) as u16;
                let col = integers.get(1).copied().unwrap_or(1).saturating_sub(1) as u16;

                self.move_cursor(row, col);
            }
            b'A' => {
                // CUU - Cursor Up
                let n = params.iter().find_map(|p| p.as_integer()).unwrap_or(1) as u16;
                self.cursor_pos.0 = self.cursor_pos.0.saturating_sub(n);
            }
            b'B' => {
                // CUD - Cursor Down
                let n = params.iter().find_map(|p| p.as_integer()).unwrap_or(1) as u16;
                self.cursor_pos.0 = (self.cursor_pos.0 + n).min(self.height - 1);
            }
            b'C' => {
                // CUF - Cursor Forward
                let n = params.iter().find_map(|p| p.as_integer()).unwrap_or(1) as u16;
                self.cursor_pos.1 = (self.cursor_pos.1 + n).min(self.width - 1);
            }
            b'D' => {
                // CUB - Cursor Back
                let n = params.iter().find_map(|p| p.as_integer()).unwrap_or(1) as u16;
                self.cursor_pos.1 = self.cursor_pos.1.saturating_sub(n);
            }
            b'm' => {
                // SGR - Select Graphic Rendition (colors and attributes)
                let integers: Vec<i64> = params.iter().filter_map(|p| p.as_integer()).collect();

                // Handle empty params (reset)
                if integers.is_empty() {
                    self.current_fg = None;
                    self.current_bg = None;
                    self.current_bold = false;
                    self.current_italic = false;
                    self.current_underline = false;
                    return;
                }

                let mut i = 0;
                while i < integers.len() {
                    match integers[i] {
                        0 => {
                            // Reset all attributes
                            self.current_fg = None;
                            self.current_bg = None;
                            self.current_bold = false;
                            self.current_italic = false;
                            self.current_underline = false;
                        }
                        1 => self.current_bold = true,
                        3 => self.current_italic = true,
                        4 => self.current_underline = true,
                        22 => self.current_bold = false,
                        23 => self.current_italic = false,
                        24 => self.current_underline = false,
                        // Foreground colors (30-37: standard, 90-97: bright)
                        30..=37 => self.current_fg = Some((integers[i] - 30) as u8),
                        90..=97 => self.current_fg = Some((integers[i] - 90 + 8) as u8),
                        39 => self.current_fg = None, // Default foreground
                        // Background colors (40-47: standard, 100-107: bright)
                        40..=47 => self.current_bg = Some((integers[i] - 40) as u8),
                        100..=107 => self.current_bg = Some((integers[i] - 100 + 8) as u8),
                        49 => self.current_bg = None, // Default background
                        // 256-color mode: ESC[38;5;N or ESC[48;5;N
                        38 | 48 => {
                            if i + 2 < integers.len() && integers[i + 1] == 5 {
                                let color = integers[i + 2] as u8;
                                if integers[i] == 38 {
                                    self.current_fg = Some(color);
                                } else {
                                    self.current_bg = Some(color);
                                }
                                i += 2; // Skip the '5' and color value
                            }
                        }
                        _ => {} // Ignore unknown SGR codes
                    }
                    i += 1;
                }
            }
            _ => {}
        }
    }

    fn esc_dispatch(
        &mut self,
        _params: &[i64],
        _intermediates: &[u8],
        _ignored_excess_intermediates: bool,
        byte: u8,
    ) {
        match byte {
            b'D' => {
                // IND - Index (move cursor down)
                if self.cursor_pos.0 + 1 < self.height {
                    self.cursor_pos.0 += 1;
                }
            }
            b'E' => {
                // NEL - Next Line
                if self.cursor_pos.0 + 1 < self.height {
                    self.cursor_pos.0 += 1;
                }
                self.cursor_pos.1 = 0;
            }
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]]) {
        // Handle OSC sequences
        // iTerm2 inline images use OSC 1337;File=...
        if params.is_empty() {
            return;
        }

        // Check if this is an iTerm2 inline image (OSC 1337;File=...)
        if let Ok(first_param) = std::str::from_utf8(params[0]) {
            if first_param.starts_with("1337;File=") || first_param == "1337" {
                self.in_iterm2_mode = true;
                self.current_iterm2_data.clear();

                // Collect all the data
                for param in params {
                    self.current_iterm2_data.extend_from_slice(param);
                    self.current_iterm2_data.push(b';');
                }

                // Parse dimensions from the parameters
                let (width, height) = self
                    .parse_iterm2_dimensions(&self.current_iterm2_data)
                    .unwrap_or((0, 0));

                let region = ITerm2Region {
                    start_row: self.cursor_pos.0,
                    start_col: self.cursor_pos.1,
                    width,
                    height,
                    data: self.current_iterm2_data.clone(),
                };
                self.iterm2_regions.push(region);

                self.in_iterm2_mode = false;
                self.current_iterm2_data.clear();
            }
        }
    }

    fn apc_dispatch(&mut self, data: Vec<u8>) {
        // Handle APC sequences
        // Kitty graphics protocol uses APC with 'G' command: ESC _ G <data> ESC \
        if data.is_empty() {
            return;
        }

        // Check if this is a Kitty graphics command (starts with 'G')
        if data[0] == b'G' {
            self.in_kitty_mode = true;
            self.current_kitty_data = data.clone();

            // Parse dimensions from the control data
            let (width, height) = self
                .parse_kitty_dimensions(&self.current_kitty_data)
                .unwrap_or((0, 0));

            let region = KittyRegion {
                start_row: self.cursor_pos.0,
                start_col: self.cursor_pos.1,
                width,
                height,
                data: self.current_kitty_data.clone(),
            };
            self.kitty_regions.push(region);

            self.in_kitty_mode = false;
            self.current_kitty_data.clear();
        }
    }
}

/// Represents the current state of the terminal screen.
///
/// `ScreenState` is the core terminal emulator that tracks:
/// - Text content at each cell position
/// - Current cursor position
/// - Sixel graphics regions (when rendered via DCS sequences)
///
/// It wraps a [`vtparse`] parser that processes VT100/ANSI escape sequences
/// and maintains the screen state accordingly.
///
/// # Usage
///
/// The typical workflow is:
/// 1. Create a `ScreenState` with desired dimensions
/// 2. Feed PTY output bytes using [`feed()`](Self::feed)
/// 3. Query the state using various accessor methods
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::ScreenState;
///
/// let mut screen = ScreenState::new(80, 24);
///
/// // Feed some terminal output
/// screen.feed(b"\x1b[2J"); // Clear screen
/// screen.feed(b"\x1b[5;10H"); // Move cursor to (5, 10)
/// screen.feed(b"Hello!");
///
/// // Query the state
/// assert_eq!(screen.cursor_position(), (4, 15)); // 0-indexed (row 4, col 15)
/// assert_eq!(screen.text_at(4, 9), Some('H'));
/// assert!(screen.contains("Hello"));
/// ```
pub struct ScreenState {
    parser: VTParser,
    state: TerminalState,
    width: u16,
    height: u16,
}

impl ScreenState {
    /// Creates a new screen state with the specified dimensions.
    ///
    /// Initializes an empty screen filled with spaces, with the cursor at (0, 0).
    ///
    /// # Arguments
    ///
    /// * `width` - Screen width in columns
    /// * `height` - Screen height in rows
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let screen = ScreenState::new(80, 24);
    /// assert_eq!(screen.size(), (80, 24));
    /// assert_eq!(screen.cursor_position(), (0, 0));
    /// ```
    pub fn new(width: u16, height: u16) -> Self {
        let parser = VTParser::new();
        let state = TerminalState::new(width, height);

        Self { parser, state, width, height }
    }

    /// Feeds data from the PTY to the parser.
    ///
    /// This processes VT100/ANSI escape sequences and updates the screen state,
    /// including:
    /// - Text output
    /// - Cursor movements
    /// - Sixel graphics (tracked via DCS callbacks)
    ///
    /// This method can be called multiple times to incrementally feed data.
    /// The parser maintains state across calls, so partial escape sequences
    /// are handled correctly.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw bytes from PTY output
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    ///
    /// // Feed data incrementally
    /// screen.feed(b"Hello, ");
    /// screen.feed(b"World!");
    ///
    /// assert!(screen.contains("Hello, World!"));
    /// ```
    pub fn feed(&mut self, data: &[u8]) {
        self.parser.parse(data, &mut self.state);
    }

    /// Returns the screen contents as a string.
    ///
    /// This includes all visible characters, preserving layout with newlines
    /// between rows. Empty cells are represented as spaces.
    ///
    /// # Returns
    ///
    /// A string containing the entire screen contents, with rows separated by newlines.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let mut screen = ScreenState::new(10, 3);
    /// screen.feed(b"Hello");
    ///
    /// let contents = screen.contents();
    /// // First line contains "Hello     " (padded to 10 chars)
    /// // Second and third lines are all spaces
    /// assert!(contents.contains("Hello"));
    /// ```
    pub fn contents(&self) -> String {
        self.state
            .cells
            .iter()
            .map(|row| row.iter().map(|cell| cell.c).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Returns the contents of a specific row.
    ///
    /// # Arguments
    ///
    /// * `row` - Row index (0-based)
    ///
    /// # Returns
    ///
    /// The row contents as a string, or empty string if row is out of bounds.
    pub fn row_contents(&self, row: u16) -> String {
        if row < self.height {
            self.state.cells[row as usize]
                .iter()
                .map(|cell| cell.c)
                .collect()
        } else {
            String::new()
        }
    }

    /// Returns the character at a specific position.
    ///
    /// # Arguments
    ///
    /// * `row` - Row index (0-based)
    /// * `col` - Column index (0-based)
    ///
    /// # Returns
    ///
    /// The character at the position, or None if out of bounds.
    pub fn text_at(&self, row: u16, col: u16) -> Option<char> {
        if row < self.height && col < self.width {
            Some(self.state.cells[row as usize][col as usize].c)
        } else {
            None
        }
    }

    /// Returns the complete cell (character + attributes) at a specific position.
    ///
    /// This method provides access to the full cell state including colors and
    /// text attributes, enabling verification of ANSI escape sequence handling.
    ///
    /// # Arguments
    ///
    /// * `row` - Row index (0-based)
    /// * `col` - Column index (0-based)
    ///
    /// # Returns
    ///
    /// A reference to the cell, or None if out of bounds.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    /// screen.feed(b"\x1b[31mRed\x1b[0m");
    ///
    /// if let Some(cell) = screen.get_cell(0, 0) {
    ///     assert_eq!(cell.c, 'R');
    ///     assert_eq!(cell.fg, Some(1)); // Red = color 1
    /// }
    /// ```
    pub fn get_cell(&self, row: u16, col: u16) -> Option<&Cell> {
        if row < self.height && col < self.width {
            Some(&self.state.cells[row as usize][col as usize])
        } else {
            None
        }
    }

    /// Returns the current cursor position.
    ///
    /// # Returns
    ///
    /// A tuple of (row, col) with 0-based indexing.
    pub fn cursor_position(&self) -> (u16, u16) {
        self.state.cursor_pos
    }

    /// Returns the screen dimensions.
    ///
    /// # Returns
    ///
    /// A tuple of (width, height) in columns and rows.
    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// Returns the screen width in columns.
    ///
    /// This is a convenience method equivalent to `self.size().0`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let screen = ScreenState::new(80, 24);
    /// assert_eq!(screen.cols(), 80);
    /// ```
    pub fn cols(&self) -> u16 {
        self.width
    }

    /// Returns the screen height in rows.
    ///
    /// This is a convenience method equivalent to `self.size().1`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let screen = ScreenState::new(80, 24);
    /// assert_eq!(screen.rows(), 24);
    /// ```
    pub fn rows(&self) -> u16 {
        self.height
    }

    /// Returns an iterator over all rows in the screen.
    ///
    /// Each item in the iterator is a reference to a row (a slice of cells).
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    /// screen.feed(b"Line 1\r\nLine 2");
    ///
    /// for (row_idx, row) in screen.iter_rows().enumerate() {
    ///     println!("Row {}: {} cells", row_idx, row.len());
    ///     for (col_idx, cell) in row.iter().enumerate() {
    ///         if cell.c != ' ' {
    ///             println!("  Cell ({}, {}): '{}'", row_idx, col_idx, cell.c);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn iter_rows(&self) -> impl Iterator<Item = &[Cell]> {
        self.state.cells.iter().map(|row| row.as_slice())
    }

    /// Returns an iterator over all cells in a specific row.
    ///
    /// Returns `None` if the row index is out of bounds.
    ///
    /// # Arguments
    ///
    /// * `row` - Row index (0-based)
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    /// screen.feed(b"\x1b[31mRed\x1b[32mGreen\x1b[34mBlue");
    ///
    /// // Use the iterator immediately to avoid lifetime issues
    /// let colored_cells: Vec<_> = screen
    ///     .iter_row(0)
    ///     .unwrap()
    ///     .enumerate()
    ///     .filter(|(_, cell)| cell.fg.is_some())
    ///     .collect();
    ///
    /// assert!(colored_cells.len() >= 3, "Should have colored cells");
    /// ```
    pub fn iter_row(&self, row: u16) -> Option<impl Iterator<Item = &Cell>> {
        if row < self.height {
            Some(self.state.cells[row as usize].iter())
        } else {
            None
        }
    }

    /// Returns a structured snapshot of the entire screen grid.
    ///
    /// This method provides a complete copy of the screen state suitable for
    /// deep comparison with other terminal emulators or for serialization.
    ///
    /// # Returns
    ///
    /// A `GridSnapshot` containing:
    /// - Grid dimensions (width, height)
    /// - Complete cell data (2D vector of cells)
    /// - Current cursor position
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    /// screen.feed(b"\x1b[31mHello");
    ///
    /// let snapshot = screen.snapshot();
    /// assert_eq!(snapshot.width, 80);
    /// assert_eq!(snapshot.height, 24);
    /// assert_eq!(snapshot.cells[0][0].c, 'H');
    /// assert_eq!(snapshot.cells[0][0].fg, Some(1)); // Red
    /// assert_eq!(snapshot.cursor, (0, 5));
    /// ```
    pub fn snapshot(&self) -> GridSnapshot {
        GridSnapshot {
            width: self.width,
            height: self.height,
            cells: self.state.cells.clone(),
            cursor: self.state.cursor_pos,
        }
    }

    /// Returns all Sixel graphics regions currently on screen.
    ///
    /// This method provides access to all Sixel graphics that have been rendered
    /// via DCS (Device Control String) sequences. Each region includes position
    /// and dimension information.
    ///
    /// This is essential for verifying Sixel positioning in tests, particularly
    /// for ensuring that graphics appear within designated preview areas.
    ///
    /// # Returns
    ///
    /// A slice of [`SixelRegion`] containing all detected Sixel graphics.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    /// // ... render some Sixel graphics ...
    ///
    /// let regions = screen.sixel_regions();
    /// for (i, region) in regions.iter().enumerate() {
    ///     println!(
    ///         "Region {}: position ({}, {}), size {}x{}",
    ///         i, region.start_row, region.start_col, region.width, region.height
    ///     );
    /// }
    /// ```
    pub fn sixel_regions(&self) -> &[SixelRegion] {
        &self.state.sixel_regions
    }

    /// Returns a mutable reference to the Sixel regions.
    ///
    /// This is primarily intended for testing purposes, allowing tests to
    /// inject mock Sixel data for memory profiling and other tests.
    ///
    /// # Returns
    ///
    /// A mutable reference to the vector of Sixel regions.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::{screen::SixelRegion, ScreenState};
    ///
    /// let mut screen = ScreenState::new(80, 24);
    ///
    /// // Add a mock Sixel region for testing
    /// screen.sixel_regions_mut().push(SixelRegion {
    ///     start_row: 5,
    ///     start_col: 10,
    ///     width: 100,
    ///     height: 50,
    ///     data: vec![0u8; 1000],
    /// });
    ///
    /// assert_eq!(screen.sixel_regions().len(), 1);
    /// ```
    pub fn sixel_regions_mut(&mut self) -> &mut Vec<SixelRegion> {
        &mut self.state.sixel_regions
    }

    /// Checks if a Sixel region exists at the given position.
    ///
    /// This method checks if any Sixel region has its starting position
    /// at the exact (row, col) coordinates provided.
    ///
    /// # Arguments
    ///
    /// * `row` - Row to check (0-indexed)
    /// * `col` - Column to check (0-indexed)
    ///
    /// # Returns
    ///
    /// `true` if a Sixel region starts at the given position, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    ///
    /// // Render a Sixel at position (5, 10) - ESC[5;10H moves to row 5, col 10 (1-based)
    /// screen.feed(b"\x1b[5;10H"); // Move cursor
    /// screen.feed(b"\x1bPq"); // Start Sixel
    /// screen.feed(b"\"1;1;100;50#0~"); // Raster + data
    /// screen.feed(b"\x1b\\"); // End Sixel
    ///
    /// // Check for Sixel at 0-based coordinates (4, 9)
    /// assert!(screen.has_sixel_at(4, 9));
    /// assert!(!screen.has_sixel_at(0, 0));
    /// ```
    pub fn has_sixel_at(&self, row: u16, col: u16) -> bool {
        self.state
            .sixel_regions
            .iter()
            .any(|region| region.start_row == row && region.start_col == col)
    }

    /// Returns all Kitty graphics regions currently on screen.
    ///
    /// This method provides access to all Kitty graphics that have been rendered
    /// via APC (Application Program Command) sequences. Each region includes position
    /// and dimension information.
    ///
    /// # Returns
    ///
    /// A slice of [`KittyRegion`] containing all detected Kitty graphics.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    /// // ... render some Kitty graphics ...
    ///
    /// let regions = screen.kitty_regions();
    /// for (i, region) in regions.iter().enumerate() {
    ///     println!(
    ///         "Kitty region {}: position ({}, {}), size {}x{}",
    ///         i, region.start_row, region.start_col, region.width, region.height
    ///     );
    /// }
    /// ```
    pub fn kitty_regions(&self) -> &[KittyRegion] {
        &self.state.kitty_regions
    }

    /// Returns a mutable reference to the Kitty regions.
    ///
    /// This is primarily intended for testing purposes.
    ///
    /// # Returns
    ///
    /// A mutable reference to the vector of Kitty regions.
    pub fn kitty_regions_mut(&mut self) -> &mut Vec<KittyRegion> {
        &mut self.state.kitty_regions
    }

    /// Returns all iTerm2 inline image regions currently on screen.
    ///
    /// This method provides access to all iTerm2 inline images that have been rendered
    /// via OSC 1337 sequences. Each region includes position and dimension information.
    ///
    /// # Returns
    ///
    /// A slice of [`ITerm2Region`] containing all detected iTerm2 inline images.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    /// // ... render some iTerm2 inline images ...
    ///
    /// let regions = screen.iterm2_regions();
    /// for (i, region) in regions.iter().enumerate() {
    ///     println!(
    ///         "iTerm2 region {}: position ({}, {}), size {}x{}",
    ///         i, region.start_row, region.start_col, region.width, region.height
    ///     );
    /// }
    /// ```
    pub fn iterm2_regions(&self) -> &[ITerm2Region] {
        &self.state.iterm2_regions
    }

    /// Returns a mutable reference to the iTerm2 regions.
    ///
    /// This is primarily intended for testing purposes.
    ///
    /// # Returns
    ///
    /// A mutable reference to the vector of iTerm2 regions.
    pub fn iterm2_regions_mut(&mut self) -> &mut Vec<ITerm2Region> {
        &mut self.state.iterm2_regions
    }

    /// Returns the screen contents for debugging purposes.
    ///
    /// This is currently an alias for [`contents()`](Self::contents), but may
    /// include additional debug information in the future.
    ///
    /// # Returns
    ///
    /// A string containing the screen contents.
    pub fn debug_contents(&self) -> String {
        self.contents()
    }

    /// Checks if the screen contains the specified text.
    ///
    /// This is a convenience method that searches the entire screen contents
    /// for the given substring. It's useful for simple text-based assertions
    /// in tests.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to search for
    ///
    /// # Returns
    ///
    /// `true` if the text appears anywhere on the screen, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    /// screen.feed(b"Welcome to the application");
    ///
    /// assert!(screen.contains("Welcome"));
    /// assert!(screen.contains("application"));
    /// assert!(!screen.contains("goodbye"));
    /// ```
    pub fn contains(&self, text: &str) -> bool {
        self.contents().contains(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_screen() {
        let screen = ScreenState::new(80, 24);
        assert_eq!(screen.size(), (80, 24));
    }

    #[test]
    fn test_feed_simple_text() {
        let mut screen = ScreenState::new(80, 24);
        screen.feed(b"Hello, World!");
        assert!(screen.contents().contains("Hello, World!"));
    }

    #[test]
    fn test_cursor_position() {
        let mut screen = ScreenState::new(80, 24);

        // Initial position
        assert_eq!(screen.cursor_position(), (0, 0));

        // Move cursor using CSI sequence (ESC [ 5 ; 10 H = row 5, col 10)
        screen.feed(b"\x1b[5;10H");
        let (row, col) = screen.cursor_position();

        // CSI uses 1-based, we convert to 0-based
        assert_eq!(row, 4); // 5-1 = 4
        assert_eq!(col, 9); // 10-1 = 9
    }

    #[test]
    fn test_text_at() {
        let mut screen = ScreenState::new(80, 24);
        screen.feed(b"Test");

        assert_eq!(screen.text_at(0, 0), Some('T'));
        assert_eq!(screen.text_at(0, 1), Some('e'));
        assert_eq!(screen.text_at(0, 2), Some('s'));
        assert_eq!(screen.text_at(0, 3), Some('t'));
        assert_eq!(screen.text_at(0, 4), Some(' '));
        assert_eq!(screen.text_at(100, 100), None);
    }

    #[test]
    fn test_parse_raster_full() {
        let state = TerminalState::new(80, 24);

        // Full format: Pan;Pad;Ph;Pv
        let data = b"\"1;1;100;50#0;2;100;100;100#0~";
        assert_eq!(state.parse_raster_attributes(data), Some((100, 50)));

        // Different aspect ratios
        let data = b"\"2;1;200;100#0~";
        assert_eq!(state.parse_raster_attributes(data), Some((200, 100)));
    }

    #[test]
    fn test_parse_raster_partial() {
        let state = TerminalState::new(80, 24);

        // Abbreviated format: Ph;Pv (aspect ratio omitted)
        let data = b"\"100;50#0~";
        assert_eq!(state.parse_raster_attributes(data), Some((100, 50)));

        let data = b"\"80;60#0;2;0;0;0";
        assert_eq!(state.parse_raster_attributes(data), Some((80, 60)));
    }

    #[test]
    fn test_parse_raster_malformed() {
        let state = TerminalState::new(80, 24);

        // No raster attributes
        assert_eq!(state.parse_raster_attributes(b"#0~"), None);

        // Empty string
        assert_eq!(state.parse_raster_attributes(b""), None);

        // Invalid UTF-8
        assert_eq!(state.parse_raster_attributes(&[0xFF, 0xFE]), None);

        // Single parameter
        assert_eq!(state.parse_raster_attributes(b"\"100"), None);

        // Three parameters (invalid)
        assert_eq!(state.parse_raster_attributes(b"\"1;1;100"), None);

        // Zero dimensions (invalid)
        assert_eq!(state.parse_raster_attributes(b"\"1;1;0;50"), None, "Should reject zero width");
        assert_eq!(
            state.parse_raster_attributes(b"\"1;1;100;0"),
            None,
            "Should reject zero height"
        );
        assert_eq!(
            state.parse_raster_attributes(b"\"0;0"),
            None,
            "Should reject zero dimensions in abbreviated format"
        );

        // Non-numeric values
        assert_eq!(state.parse_raster_attributes(b"\"abc;def"), None);

        // Mixed numeric/non-numeric: parser stops at first non-numeric, non-semicolon
        // "1;1;abc;def" becomes "1;1" which is valid 2-param format
        // This is intentional - we parse up to the first non-numeric character
        assert_eq!(state.parse_raster_attributes(b"\"1;1;abc;def"), Some((1, 1)));
    }

    #[test]
    fn test_parse_raster_edge_cases() {
        let state = TerminalState::new(80, 24);

        // Large dimensions
        let data = b"\"1;1;4096;2048#0~";
        assert_eq!(state.parse_raster_attributes(data), Some((4096, 2048)));

        // Minimum valid dimensions
        let data = b"\"1;1;1;1#0~";
        assert_eq!(state.parse_raster_attributes(data), Some((1, 1)));

        // Extra whitespace/characters after parameters
        let data = b"\"1;1;100;50  \t#0~";
        assert_eq!(state.parse_raster_attributes(data), Some((100, 50)));
    }

    #[test]
    fn test_pixels_to_cells() {
        // Standard conversions (8 pixels/col, 6 pixels/row)
        assert_eq!(TerminalState::pixels_to_cells(80, 60), (10, 10));
        assert_eq!(TerminalState::pixels_to_cells(0, 0), (0, 0));

        // Exact multiples
        assert_eq!(TerminalState::pixels_to_cells(800, 600), (100, 100));
        assert_eq!(TerminalState::pixels_to_cells(16, 12), (2, 2));

        // Fractional cells (should round up)
        assert_eq!(TerminalState::pixels_to_cells(81, 61), (11, 11));
        assert_eq!(TerminalState::pixels_to_cells(100, 50), (13, 9));
        assert_eq!(TerminalState::pixels_to_cells(1, 1), (1, 1));

        // Typical Sixel dimensions from real use
        assert_eq!(TerminalState::pixels_to_cells(640, 480), (80, 80));
        assert_eq!(TerminalState::pixels_to_cells(320, 240), (40, 40));
    }

    #[test]
    fn test_sixel_region_tracking() {
        let mut screen = ScreenState::new(80, 24);

        // Feed a complete Sixel sequence with raster attributes
        screen.feed(b"\x1b[5;10H"); // Move cursor to (5, 10) [1-based]
        screen.feed(b"\x1bPq"); // DCS - Start Sixel with 'q'
        screen.feed(b"\"1;1;100;50"); // Raster attributes: 100x50 pixels
        screen.feed(b"#0;2;100;100;100"); // Define color 0
        screen.feed(b"#0~~@@"); // Some sixel data
        screen.feed(b"\x1b\\"); // String terminator (ST)

        // Verify the Sixel region was captured
        let regions = screen.sixel_regions();
        assert_eq!(regions.len(), 1, "Should capture exactly one Sixel region");

        let region = &regions[0];
        assert_eq!(region.start_row, 4, "Row should be 4 (0-based from 5)");
        assert_eq!(region.start_col, 9, "Col should be 9 (0-based from 10)");
        assert_eq!(region.width, 100, "Width should be 100 pixels");
        assert_eq!(region.height, 50, "Height should be 50 pixels");
        assert!(!region.data.is_empty(), "Data should be captured");

        // Verify has_sixel_at
        assert!(screen.has_sixel_at(4, 9), "Should detect Sixel at position");
        assert!(!screen.has_sixel_at(0, 0), "Should not detect Sixel at wrong position");
    }

    #[test]
    fn test_multiple_sixel_regions() {
        let mut screen = ScreenState::new(100, 30);

        // First Sixel
        screen.feed(b"\x1b[5;5H\x1bPq\"1;1;80;60#0~\x1b\\");

        // Second Sixel
        screen.feed(b"\x1b[15;50H\x1bPq\"1;1;100;80#0~\x1b\\");

        let regions = screen.sixel_regions();
        assert_eq!(regions.len(), 2, "Should capture both Sixel regions");

        // Verify first region
        assert_eq!(regions[0].start_row, 4);
        assert_eq!(regions[0].start_col, 4);
        assert_eq!(regions[0].width, 80);
        assert_eq!(regions[0].height, 60);

        // Verify second region
        assert_eq!(regions[1].start_row, 14);
        assert_eq!(regions[1].start_col, 49);
        assert_eq!(regions[1].width, 100);
        assert_eq!(regions[1].height, 80);
    }

    #[test]
    fn test_sixel_without_raster_attributes() {
        let mut screen = ScreenState::new(80, 24);

        // Sixel without raster attributes (legacy format)
        screen.feed(b"\x1b[10;10H\x1bPq#0~\x1b\\");

        let regions = screen.sixel_regions();
        assert_eq!(regions.len(), 1, "Should still capture region");

        let region = &regions[0];
        assert_eq!(region.width, 0, "Width should be 0 without raster attributes");
        assert_eq!(region.height, 0, "Height should be 0 without raster attributes");
    }

    #[test]
    fn test_sixel_abbreviated_format() {
        let mut screen = ScreenState::new(80, 24);

        // Abbreviated raster format (just width;height)
        screen.feed(b"\x1b[1;1H\x1bPq\"200;150#0~\x1b\\");

        let regions = screen.sixel_regions();
        assert_eq!(regions.len(), 1);

        let region = &regions[0];
        assert_eq!(region.width, 200);
        assert_eq!(region.height, 150);
    }
}
