//! Terminal screen state management using vtparse with Sixel support.

use vtparse::{VTActor, VTParser, CsiParam};

/// Represents a Sixel graphics region in the terminal.
#[derive(Debug, Clone)]
pub struct SixelRegion {
    /// Starting row (0-indexed)
    pub start_row: u16,
    /// Starting column (0-indexed)
    pub start_col: u16,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Raw Sixel data
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
    cells: Vec<Vec<char>>,
}

impl TerminalState {
    fn new(width: u16, height: u16) -> Self {
        let cells = vec![vec![' '; width as usize]; height as usize];

        Self {
            cursor_pos: (0, 0),
            sixel_regions: Vec::new(),
            current_sixel_data: Vec::new(),
            current_sixel_params: Vec::new(),
            in_sixel_mode: false,
            width,
            height,
            cells,
        }
    }

    fn put_char(&mut self, ch: char) {
        let (row, col) = self.cursor_pos;
        if row < self.height && col < self.width {
            self.cells[row as usize][col as usize] = ch;
            // Move cursor forward, but don't wrap automatically
            if col + 1 < self.width {
                self.cursor_pos.1 = col + 1;
            }
        }
    }

    fn move_cursor(&mut self, row: u16, col: u16) {
        self.cursor_pos = (row.min(self.height - 1), col.min(self.width - 1));
    }

    /// Parse raster attributes from sixel data: "Pa;Pb;Ph;Pv
    /// Returns (width, height) in pixels if found
    fn parse_raster_attributes(&self, data: &[u8]) -> Option<(u32, u32)> {
        let data_str = std::str::from_utf8(data).ok()?;

        // Find the raster attributes command starting with '"'
        if let Some(raster_start) = data_str.find('"') {
            let raster_part = &data_str[raster_start + 1..];

            // Parse format: Pa;Pb;Ph;Pv
            // We want Ph (width) and Pv (height)
            let parts: Vec<&str> = raster_part
                .split(|c: char| !c.is_ascii_digit() && c != ';')
                .filter(|s| !s.is_empty())
                .take(4)
                .collect();

            if parts.len() >= 4 {
                let width = parts[2].parse::<u32>().ok()?;
                let height = parts[3].parse::<u32>().ok()?;
                return Some((width, height));
            }
        }
        None
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
/// This wraps a vtparse parser and provides methods to query screen contents,
/// cursor position, cell attributes, and Sixel graphics regions.
pub struct ScreenState {
    parser: VTParser,
    state: TerminalState,
    width: u16,
    height: u16,
}

impl ScreenState {
    /// Creates a new screen state with the specified dimensions.
    ///
    /// # Arguments
    ///
    /// * `width` - Screen width in columns
    /// * `height` - Screen height in rows
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
    /// This processes escape sequences and updates the screen state,
    /// including tracking Sixel graphics via DCS callbacks.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw bytes from PTY output
    pub fn feed(&mut self, data: &[u8]) {
        self.parser.parse(data, &mut self.state);
    }

    /// Returns the screen contents as a string.
    ///
    /// This includes all visible characters, preserving layout.
    pub fn contents(&self) -> String {
        self.state
            .cells
            .iter()
            .map(|row| row.iter().collect::<String>())
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
            self.state.cells[row as usize].iter().collect()
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
            Some(self.state.cells[row as usize][col as usize])
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
    /// This is essential for Phase 3 Sixel position tracking.
    pub fn sixel_regions(&self) -> &[SixelRegion] {
        &self.state.sixel_regions
    }

    /// Checks if a Sixel region exists at the given position.
    ///
    /// # Arguments
    ///
    /// * `row` - Row to check (0-based)
    /// * `col` - Column to check (0-based)
    pub fn has_sixel_at(&self, row: u16, col: u16) -> bool {
        self.state.sixel_regions.iter().any(|region| {
            region.start_row == row && region.start_col == col
        })
    }

    /// Returns the screen contents for debugging purposes.
    ///
    /// This is similar to `contents()` but may include additional debug information.
    pub fn debug_contents(&self) -> String {
        self.contents()
    }

    /// Checks if the screen contains the specified text.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to search for
    ///
    /// # Returns
    ///
    /// `true` if the text appears anywhere on the screen
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
}
