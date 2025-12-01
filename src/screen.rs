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
//! # Example
//!
//! ```rust
//! use mimic::ScreenState;
//!
//! let mut screen = ScreenState::new(80, 24);
//!
//! // Feed terminal output
//! screen.feed(b"Hello, World!");
//!
//! // Query screen contents
//! assert!(screen.contains("Hello"));
//! assert_eq!(screen.cursor_position(), (0, 13));
//!
//! // Check specific position
//! assert_eq!(screen.text_at(0, 0), Some('H'));
//! ```

use vtparse::{VTActor, VTParser, CsiParam};

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
/// # use mimic::ScreenState;
/// let mut screen = ScreenState::new(80, 24);
///
/// // After rendering a Sixel image...
/// let regions = screen.sixel_regions();
/// for region in regions {
///     println!("Sixel at ({}, {}), size {}x{}",
///         region.start_row, region.start_col,
///         region.width, region.height);
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

/// Terminal state tracking for vtparse parser.
///
/// Implements VTActor to handle escape sequences including DCS for Sixel.
struct TerminalState {
    cursor_pos: (u16, u16),
    sixel_regions: Vec<SixelRegion>,
    current_sixel_data: Vec<u8>,
    current_sixel_params: Vec<i64>,
    in_sixel_mode: bool,
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
        let parts: Vec<&str> = raster_part
            .split(';')
            .filter(|s| !s.is_empty())
            .collect();

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
                let integers: Vec<i64> = params
                    .iter()
                    .filter_map(|p| p.as_integer())
                    .collect();

                let row = integers
                    .get(0)
                    .copied()
                    .unwrap_or(1)
                    .saturating_sub(1) as u16;
                let col = integers
                    .get(1)
                    .copied()
                    .unwrap_or(1)
                    .saturating_sub(1) as u16;

                self.move_cursor(row, col);
            }
            b'A' => {
                // CUU - Cursor Up
                let n = params
                    .iter()
                    .find_map(|p| p.as_integer())
                    .unwrap_or(1) as u16;
                self.cursor_pos.0 = self.cursor_pos.0.saturating_sub(n);
            }
            b'B' => {
                // CUD - Cursor Down
                let n = params
                    .iter()
                    .find_map(|p| p.as_integer())
                    .unwrap_or(1) as u16;
                self.cursor_pos.0 = (self.cursor_pos.0 + n).min(self.height - 1);
            }
            b'C' => {
                // CUF - Cursor Forward
                let n = params
                    .iter()
                    .find_map(|p| p.as_integer())
                    .unwrap_or(1) as u16;
                self.cursor_pos.1 = (self.cursor_pos.1 + n).min(self.width - 1);
            }
            b'D' => {
                // CUB - Cursor Back
                let n = params
                    .iter()
                    .find_map(|p| p.as_integer())
                    .unwrap_or(1) as u16;
                self.cursor_pos.1 = self.cursor_pos.1.saturating_sub(n);
            }
            b'm' => {
                // SGR - Select Graphic Rendition (colors and attributes)
                let integers: Vec<i64> = params
                    .iter()
                    .filter_map(|p| p.as_integer())
                    .collect();

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

    fn osc_dispatch(&mut self, _params: &[&[u8]]) {
        // Handle OSC sequences (window title, etc.)
        // Not needed for basic functionality
    }

    fn apc_dispatch(&mut self, _data: Vec<u8>) {
        // Handle APC sequences (e.g., Kitty graphics protocol)
        // Not needed for basic functionality
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
/// use mimic::ScreenState;
///
/// let mut screen = ScreenState::new(80, 24);
///
/// // Feed some terminal output
/// screen.feed(b"\x1b[2J"); // Clear screen
/// screen.feed(b"\x1b[5;10H"); // Move cursor to (5, 10)
/// screen.feed(b"Hello!");
///
/// // Query the state
/// assert_eq!(screen.cursor_position(), (4, 16)); // 0-indexed
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
    /// use mimic::ScreenState;
    ///
    /// let screen = ScreenState::new(80, 24);
    /// assert_eq!(screen.size(), (80, 24));
    /// assert_eq!(screen.cursor_position(), (0, 0));
    /// ```
    pub fn new(width: u16, height: u16) -> Self {
        let parser = VTParser::new();
        let state = TerminalState::new(width, height);

        Self {
            parser,
            state,
            width,
            height,
        }
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
    /// use mimic::ScreenState;
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
    /// use mimic::ScreenState;
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
            self.state.cells[row as usize].iter().map(|cell| cell.c).collect()
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
    /// use mimic::ScreenState;
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
    /// use mimic::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    /// // ... render some Sixel graphics ...
    ///
    /// let regions = screen.sixel_regions();
    /// for (i, region) in regions.iter().enumerate() {
    ///     println!("Region {}: position ({}, {}), size {}x{}",
    ///         i, region.start_row, region.start_col,
    ///         region.width, region.height);
    /// }
    /// ```
    pub fn sixel_regions(&self) -> &[SixelRegion] {
        &self.state.sixel_regions
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
    /// use mimic::ScreenState;
    ///
    /// let mut screen = ScreenState::new(80, 24);
    /// // ... render Sixel at position (5, 10) ...
    ///
    /// assert!(screen.has_sixel_at(5, 10));
    /// assert!(!screen.has_sixel_at(0, 0));
    /// ```
    pub fn has_sixel_at(&self, row: u16, col: u16) -> bool {
        self.state.sixel_regions.iter().any(|region| {
            region.start_row == row && region.start_col == col
        })
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
    /// use mimic::ScreenState;
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
        assert_eq!(row, 4);  // 5-1 = 4
        assert_eq!(col, 9);  // 10-1 = 9
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
        assert_eq!(state.parse_raster_attributes(b"\"1;1;100;0"), None, "Should reject zero height");
        assert_eq!(state.parse_raster_attributes(b"\"0;0"), None, "Should reject zero dimensions in abbreviated format");

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
        screen.feed(b"\x1b[5;10H");           // Move cursor to (5, 10) [1-based]
        screen.feed(b"\x1bPq");                // DCS - Start Sixel with 'q'
        screen.feed(b"\"1;1;100;50");          // Raster attributes: 100x50 pixels
        screen.feed(b"#0;2;100;100;100");      // Define color 0
        screen.feed(b"#0~~@@");                // Some sixel data
        screen.feed(b"\x1b\\");                // String terminator (ST)

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
