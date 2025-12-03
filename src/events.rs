//! Keyboard and mouse event types and escape sequence encoding.
//!
//! This module provides types for representing keyboard and mouse input and converting them
//! to VT100/ANSI escape sequences that can be sent to PTY-based applications.
//!
//! # Key Features
//!
//! - **Type-safe key codes**: Enum-based key representation
//! - **Mouse event support**: Click, drag, and scroll simulation
//! - **Modifier support**: Ctrl, Alt, Shift, Meta via bitflags
//! - **VT100 compliance**: Standard escape sequences for terminal compatibility
//! - **SGR mouse encoding**: Modern mouse protocol support
//! - **Zero allocation**: Static byte slices where possible
//!
//! # Example
//!
//! ```rust
//! use ratatui_testlib::events::{KeyCode, KeyEvent, Modifiers, MouseButton};
//!
//! // Simple key
//! let key = KeyEvent::new(KeyCode::Char('a'));
//!
//! // Key with modifiers
//! let ctrl_c = KeyEvent::with_modifiers(KeyCode::Char('c'), Modifiers::CTRL);
//!
//! // Navigation keys
//! let up = KeyEvent::new(KeyCode::Up);
//! let enter = KeyEvent::new(KeyCode::Enter);
//! ```

use bitflags::bitflags;

/// Represents a keyboard key.
///
/// This enum covers all keys commonly used in TUI applications, including:
/// - Alphanumeric characters
/// - Special keys (Enter, Tab, Esc, etc.)
/// - Navigation keys (arrows, Home, End, etc.)
/// - Function keys (F1-F12)
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::events::KeyCode;
///
/// let letter = KeyCode::Char('a');
/// let enter = KeyCode::Enter;
/// let arrow = KeyCode::Up;
/// let function = KeyCode::F(1); // F1
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// A character key (letters, numbers, symbols).
    Char(char),

    /// Enter key (newline).
    Enter,

    /// Escape key.
    Esc,

    /// Tab key.
    Tab,

    /// Backspace key.
    Backspace,

    /// Delete key.
    Delete,

    /// Insert key.
    Insert,

    /// Up arrow key.
    Up,

    /// Down arrow key.
    Down,

    /// Left arrow key.
    Left,

    /// Right arrow key.
    Right,

    /// Home key.
    Home,

    /// End key.
    End,

    /// Page Up key.
    PageUp,

    /// Page Down key.
    PageDown,

    /// Function keys F1-F12.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::events::KeyCode;
    ///
    /// let f1 = KeyCode::F(1);
    /// let f12 = KeyCode::F(12);
    /// ```
    F(u8),
}

bitflags! {
    /// Modifier keys that can be combined with other keys.
    ///
    /// These are bitflags, so multiple modifiers can be combined:
    ///
    /// ```rust
    /// use ratatui_testlib::events::Modifiers;
    ///
    /// let ctrl_shift = Modifiers::CTRL | Modifiers::SHIFT;
    /// let ctrl_alt = Modifiers::CTRL | Modifiers::ALT;
    /// ```
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Modifiers: u8 {
        /// Shift key.
        const SHIFT = 0b0001;

        /// Control key.
        const CTRL  = 0b0010;

        /// Alt/Option key.
        const ALT   = 0b0100;

        /// Meta/Command/Windows key.
        const META  = 0b1000;
    }
}

/// A keyboard event combining a key code and optional modifiers.
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::events::{KeyCode, KeyEvent, Modifiers};
///
/// // Simple key press
/// let key = KeyEvent::new(KeyCode::Char('a'));
///
/// // Ctrl+C
/// let ctrl_c = KeyEvent::with_modifiers(KeyCode::Char('c'), Modifiers::CTRL);
///
/// // Encode to bytes for sending to PTY
/// let bytes = ctrl_c.to_bytes();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyEvent {
    /// The key code.
    pub code: KeyCode,

    /// Modifier keys held during the key press.
    pub modifiers: Modifiers,
}

impl KeyEvent {
    /// Creates a new key event without modifiers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::events::{KeyCode, KeyEvent};
    ///
    /// let key = KeyEvent::new(KeyCode::Char('a'));
    /// ```
    pub fn new(code: KeyCode) -> Self {
        Self { code, modifiers: Modifiers::empty() }
    }

    /// Creates a new key event with modifiers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::events::{KeyCode, KeyEvent, Modifiers};
    ///
    /// let ctrl_c = KeyEvent::with_modifiers(KeyCode::Char('c'), Modifiers::CTRL);
    /// ```
    pub fn with_modifiers(code: KeyCode, modifiers: Modifiers) -> Self {
        Self { code, modifiers }
    }

    /// Converts the key event to bytes suitable for sending to a PTY.
    ///
    /// This generates standard VT100/ANSI escape sequences that are
    /// compatible with most terminal applications.
    ///
    /// # Returns
    ///
    /// A vector of bytes representing the escape sequence for this key event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::events::{KeyCode, KeyEvent, Modifiers};
    ///
    /// let key = KeyEvent::new(KeyCode::Char('a'));
    /// let bytes = key.to_bytes();
    /// assert_eq!(bytes, b"a");
    ///
    /// let ctrl_c = KeyEvent::with_modifiers(KeyCode::Char('c'), Modifiers::CTRL);
    /// let bytes = ctrl_c.to_bytes();
    /// assert_eq!(bytes, vec![3]); // Ctrl+C = 0x03
    /// ```
    pub fn to_bytes(&self) -> Vec<u8> {
        encode_key_event(self)
    }
}

/// Represents a mouse button.
///
/// Used for mouse click and drag events in terminal applications.
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::events::MouseButton;
///
/// let left = MouseButton::Left;
/// let right = MouseButton::Right;
/// let middle = MouseButton::Middle;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// Left mouse button.
    Left,
    /// Right mouse button.
    Right,
    /// Middle mouse button (scroll wheel click).
    Middle,
}

impl MouseButton {
    /// Returns the SGR button code for this mouse button.
    ///
    /// SGR (Select Graphic Rendition) mouse encoding uses these codes:
    /// - 0: Left button
    /// - 1: Middle button
    /// - 2: Right button
    fn to_sgr_code(self) -> u8 {
        match self {
            MouseButton::Left => 0,
            MouseButton::Middle => 1,
            MouseButton::Right => 2,
        }
    }
}

/// Represents a scroll direction.
///
/// Used for mouse wheel events in terminal applications.
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::events::ScrollDirection;
///
/// let up = ScrollDirection::Up;
/// let down = ScrollDirection::Down;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollDirection {
    /// Scroll up.
    Up,
    /// Scroll down.
    Down,
    /// Scroll left.
    Left,
    /// Scroll right.
    Right,
}

impl ScrollDirection {
    /// Returns the SGR button code for this scroll direction.
    ///
    /// SGR mouse encoding uses these codes for scroll events:
    /// - 64: Scroll up
    /// - 65: Scroll down
    /// - 66: Scroll left
    /// - 67: Scroll right
    fn to_sgr_code(self) -> u8 {
        match self {
            ScrollDirection::Up => 64,
            ScrollDirection::Down => 65,
            ScrollDirection::Left => 66,
            ScrollDirection::Right => 67,
        }
    }
}

/// A mouse event combining position, button/scroll, and optional modifiers.
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::events::{Modifiers, MouseButton, MouseEvent};
///
/// // Simple click
/// let click = MouseEvent::press(10, 5, MouseButton::Left);
///
/// // Click with modifiers
/// let ctrl_click = MouseEvent::press_with_modifiers(10, 5, MouseButton::Left, Modifiers::CTRL);
///
/// // Release
/// let release = MouseEvent::release(10, 5, MouseButton::Left);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseEvent {
    /// X coordinate (column, 0-indexed).
    pub x: u16,
    /// Y coordinate (row, 0-indexed).
    pub y: u16,
    /// Mouse button or scroll direction code.
    pub button_code: u8,
    /// Whether this is a button press (true) or release (false).
    pub is_press: bool,
    /// Modifier keys held during the mouse event.
    pub modifiers: Modifiers,
}

impl MouseEvent {
    /// Creates a mouse button press event.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate (column, 0-indexed)
    /// * `y` - Y coordinate (row, 0-indexed)
    /// * `button` - Mouse button
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::events::{MouseButton, MouseEvent};
    ///
    /// let click = MouseEvent::press(10, 5, MouseButton::Left);
    /// ```
    pub fn press(x: u16, y: u16, button: MouseButton) -> Self {
        Self {
            x,
            y,
            button_code: button.to_sgr_code(),
            is_press: true,
            modifiers: Modifiers::empty(),
        }
    }

    /// Creates a mouse button press event with modifiers.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate (column, 0-indexed)
    /// * `y` - Y coordinate (row, 0-indexed)
    /// * `button` - Mouse button
    /// * `modifiers` - Modifier keys
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::events::{Modifiers, MouseButton, MouseEvent};
    ///
    /// let ctrl_click = MouseEvent::press_with_modifiers(10, 5, MouseButton::Left, Modifiers::CTRL);
    /// ```
    pub fn press_with_modifiers(x: u16, y: u16, button: MouseButton, modifiers: Modifiers) -> Self {
        Self {
            x,
            y,
            button_code: button.to_sgr_code(),
            is_press: true,
            modifiers,
        }
    }

    /// Creates a mouse button release event.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate (column, 0-indexed)
    /// * `y` - Y coordinate (row, 0-indexed)
    /// * `button` - Mouse button
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::events::{MouseButton, MouseEvent};
    ///
    /// let release = MouseEvent::release(10, 5, MouseButton::Left);
    /// ```
    pub fn release(x: u16, y: u16, button: MouseButton) -> Self {
        Self {
            x,
            y,
            button_code: button.to_sgr_code(),
            is_press: false,
            modifiers: Modifiers::empty(),
        }
    }

    /// Creates a scroll event.
    ///
    /// # Arguments
    ///
    /// * `x` - X coordinate (column, 0-indexed)
    /// * `y` - Y coordinate (row, 0-indexed)
    /// * `direction` - Scroll direction
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::events::{MouseEvent, ScrollDirection};
    ///
    /// let scroll = MouseEvent::scroll(10, 5, ScrollDirection::Up);
    /// ```
    pub fn scroll(x: u16, y: u16, direction: ScrollDirection) -> Self {
        Self {
            x,
            y,
            button_code: direction.to_sgr_code(),
            is_press: true, // Scroll events use press encoding
            modifiers: Modifiers::empty(),
        }
    }

    /// Converts the mouse event to bytes suitable for sending to a PTY.
    ///
    /// This generates SGR (Select Graphic Rendition) mouse encoding sequences
    /// which are the modern standard for mouse reporting in terminals.
    ///
    /// # Returns
    ///
    /// A vector of bytes representing the SGR escape sequence for this mouse event.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::events::{MouseButton, MouseEvent};
    ///
    /// let click = MouseEvent::press(10, 5, MouseButton::Left);
    /// let bytes = click.to_bytes();
    /// assert_eq!(bytes, b"\x1b[<0;11;6M"); // SGR format with 1-indexed coords
    ///
    /// let release = MouseEvent::release(10, 5, MouseButton::Left);
    /// let bytes = release.to_bytes();
    /// assert_eq!(bytes, b"\x1b[<0;11;6m"); // Note lowercase 'm' for release
    /// ```
    pub fn to_bytes(&self) -> Vec<u8> {
        encode_mouse_event(self)
    }
}

/// Encodes a key event into VT100/ANSI escape sequence bytes.
///
/// This function handles:
/// - Regular characters
/// - Control key combinations (Ctrl+A-Z)
/// - Alt key combinations (ESC + key)
/// - Special keys (arrows, function keys, etc.)
/// - Navigation keys (Home, End, PageUp, PageDown)
///
/// # Arguments
///
/// * `event` - The key event to encode
///
/// # Returns
///
/// A vector of bytes representing the escape sequence.
pub fn encode_key_event(event: &KeyEvent) -> Vec<u8> {
    // Handle Ctrl modifier first for character keys
    if event.modifiers.contains(Modifiers::CTRL) {
        if let KeyCode::Char(c) = event.code {
            // Ctrl+A-Z maps to 1-26
            // Ctrl+[ = ESC (27), Ctrl+\ = 28, Ctrl+] = 29, Ctrl+^ = 30, Ctrl+_ = 31
            return encode_ctrl_char(c);
        }
    }

    // Handle Alt modifier for character keys
    if event.modifiers.contains(Modifiers::ALT) {
        if let KeyCode::Char(c) = event.code {
            // Alt+key = ESC + key
            let mut bytes = vec![0x1b]; // ESC
            bytes.extend_from_slice(c.to_string().as_bytes());
            return bytes;
        }
    }

    // Handle unmodified keys
    match event.code {
        KeyCode::Char(c) => c.to_string().into_bytes(),
        KeyCode::Enter => b"\n".to_vec(),
        KeyCode::Tab => b"\t".to_vec(),
        KeyCode::Esc => vec![0x1b],
        KeyCode::Backspace => vec![0x7f], // DEL character
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::F(n) => encode_function_key(n),
    }
}

/// Encodes Ctrl+character combinations.
///
/// Ctrl key combinations use the ASCII control character range:
/// - Ctrl+A = 0x01
/// - Ctrl+B = 0x02
/// - ...
/// - Ctrl+Z = 0x1A
/// - Ctrl+[ = 0x1B (ESC)
/// - Ctrl+\ = 0x1C
/// - Ctrl+] = 0x1D
/// - Ctrl+^ = 0x1E (often Ctrl+Shift+6)
/// - Ctrl+_ = 0x1F (often Ctrl+Shift+-)
///
/// # Arguments
///
/// * `c` - The character to combine with Ctrl
///
/// # Returns
///
/// A vector containing the control character byte.
fn encode_ctrl_char(c: char) -> Vec<u8> {
    let c_upper = c.to_ascii_uppercase();

    let byte = match c_upper {
        // Ctrl+A through Ctrl+Z
        'A'..='Z' => (c_upper as u8) - b'A' + 1,

        // Special control characters
        '@' => 0,   // Ctrl+@ = NUL
        '[' => 27,  // Ctrl+[ = ESC
        '\\' => 28, // Ctrl+\ = FS
        ']' => 29,  // Ctrl+] = GS
        '^' => 30,  // Ctrl+^ = RS
        '_' => 31,  // Ctrl+_ = US
        '?' => 127, // Ctrl+? = DEL

        // For lowercase, convert to uppercase
        'a'..='z' => (c_upper as u8) - b'A' + 1,

        // For other characters, try to map sensibly
        _ => {
            // Default: just send the character unchanged
            return c.to_string().into_bytes();
        }
    };

    vec![byte]
}

/// Encodes function keys (F1-F12) to their VT100 escape sequences.
///
/// Function key mappings:
/// - F1-F4 use SS3 sequences (ESC O ...)
/// - F5-F12 use CSI sequences (ESC [ ... ~)
///
/// # Arguments
///
/// * `n` - Function key number (1-12)
///
/// # Returns
///
/// A vector containing the escape sequence for the function key.
fn encode_function_key(n: u8) -> Vec<u8> {
    match n {
        // F1-F4 use SS3 (ESC O) sequences
        1 => b"\x1bOP".to_vec(),
        2 => b"\x1bOQ".to_vec(),
        3 => b"\x1bOR".to_vec(),
        4 => b"\x1bOS".to_vec(),

        // F5-F12 use CSI (ESC [) sequences
        5 => b"\x1b[15~".to_vec(),
        6 => b"\x1b[17~".to_vec(),
        7 => b"\x1b[18~".to_vec(),
        8 => b"\x1b[19~".to_vec(),
        9 => b"\x1b[20~".to_vec(),
        10 => b"\x1b[21~".to_vec(),
        11 => b"\x1b[23~".to_vec(),
        12 => b"\x1b[24~".to_vec(),

        // For invalid function key numbers, return empty sequence
        _ => Vec::new(),
    }
}

/// Encodes a mouse event into SGR (Select Graphic Rendition) format.
///
/// SGR mouse encoding is the modern standard for mouse reporting in terminals.
/// Format: `ESC [ < button ; x ; y M/m` where:
/// - `button` is the button code (possibly with modifier bits)
/// - `x` and `y` are 1-indexed coordinates (we convert from 0-indexed)
/// - `M` indicates button press, `m` indicates button release
///
/// Modifiers are encoded by adding to the button code:
/// - Shift: +4
/// - Alt/Meta: +8
/// - Ctrl: +16
///
/// # Arguments
///
/// * `event` - The mouse event to encode
///
/// # Returns
///
/// A vector of bytes representing the SGR escape sequence.
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::events::{MouseButton, MouseEvent};
///
/// let click = MouseEvent::press(10, 5, MouseButton::Left);
/// let bytes = click.to_bytes();
/// // Results in: \x1b[<0;11;6M (coordinates are 1-indexed)
/// ```
pub fn encode_mouse_event(event: &MouseEvent) -> Vec<u8> {
    let mut button_code = event.button_code;

    // Add modifier bits to button code
    if event.modifiers.contains(Modifiers::SHIFT) {
        button_code += 4;
    }
    if event.modifiers.contains(Modifiers::ALT) {
        button_code += 8;
    }
    if event.modifiers.contains(Modifiers::CTRL) {
        button_code += 16;
    }

    // Convert 0-indexed coordinates to 1-indexed for SGR format
    let x = event.x + 1;
    let y = event.y + 1;

    // SGR format: ESC[<button;x;yM (press) or ESC[<button;x;ym (release)
    let terminator = if event.is_press { 'M' } else { 'm' };

    format!("\x1b[<{};{};{}{}", button_code, x, y, terminator).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_event_new() {
        let event = KeyEvent::new(KeyCode::Char('a'));
        assert_eq!(event.code, KeyCode::Char('a'));
        assert_eq!(event.modifiers, Modifiers::empty());
    }

    #[test]
    fn test_key_event_with_modifiers() {
        let event = KeyEvent::with_modifiers(KeyCode::Char('c'), Modifiers::CTRL);
        assert_eq!(event.code, KeyCode::Char('c'));
        assert_eq!(event.modifiers, Modifiers::CTRL);
    }

    #[test]
    fn test_encode_simple_char() {
        let event = KeyEvent::new(KeyCode::Char('a'));
        assert_eq!(event.to_bytes(), b"a");

        let event = KeyEvent::new(KeyCode::Char('Z'));
        assert_eq!(event.to_bytes(), b"Z");

        let event = KeyEvent::new(KeyCode::Char('5'));
        assert_eq!(event.to_bytes(), b"5");
    }

    #[test]
    fn test_encode_special_chars() {
        let event = KeyEvent::new(KeyCode::Enter);
        assert_eq!(event.to_bytes(), b"\n");

        let event = KeyEvent::new(KeyCode::Tab);
        assert_eq!(event.to_bytes(), b"\t");

        let event = KeyEvent::new(KeyCode::Esc);
        assert_eq!(event.to_bytes(), vec![0x1b]);

        let event = KeyEvent::new(KeyCode::Backspace);
        assert_eq!(event.to_bytes(), vec![0x7f]);
    }

    #[test]
    fn test_encode_navigation_keys() {
        let event = KeyEvent::new(KeyCode::Up);
        assert_eq!(event.to_bytes(), b"\x1b[A");

        let event = KeyEvent::new(KeyCode::Down);
        assert_eq!(event.to_bytes(), b"\x1b[B");

        let event = KeyEvent::new(KeyCode::Right);
        assert_eq!(event.to_bytes(), b"\x1b[C");

        let event = KeyEvent::new(KeyCode::Left);
        assert_eq!(event.to_bytes(), b"\x1b[D");

        let event = KeyEvent::new(KeyCode::Home);
        assert_eq!(event.to_bytes(), b"\x1b[H");

        let event = KeyEvent::new(KeyCode::End);
        assert_eq!(event.to_bytes(), b"\x1b[F");
    }

    #[test]
    fn test_encode_page_keys() {
        let event = KeyEvent::new(KeyCode::PageUp);
        assert_eq!(event.to_bytes(), b"\x1b[5~");

        let event = KeyEvent::new(KeyCode::PageDown);
        assert_eq!(event.to_bytes(), b"\x1b[6~");
    }

    #[test]
    fn test_encode_delete_insert() {
        let event = KeyEvent::new(KeyCode::Delete);
        assert_eq!(event.to_bytes(), b"\x1b[3~");

        let event = KeyEvent::new(KeyCode::Insert);
        assert_eq!(event.to_bytes(), b"\x1b[2~");
    }

    #[test]
    fn test_encode_function_keys() {
        // F1-F4 use SS3 sequences
        let event = KeyEvent::new(KeyCode::F(1));
        assert_eq!(event.to_bytes(), b"\x1bOP");

        let event = KeyEvent::new(KeyCode::F(2));
        assert_eq!(event.to_bytes(), b"\x1bOQ");

        let event = KeyEvent::new(KeyCode::F(3));
        assert_eq!(event.to_bytes(), b"\x1bOR");

        let event = KeyEvent::new(KeyCode::F(4));
        assert_eq!(event.to_bytes(), b"\x1bOS");

        // F5-F12 use CSI sequences
        let event = KeyEvent::new(KeyCode::F(5));
        assert_eq!(event.to_bytes(), b"\x1b[15~");

        let event = KeyEvent::new(KeyCode::F(12));
        assert_eq!(event.to_bytes(), b"\x1b[24~");
    }

    #[test]
    fn test_encode_ctrl_combinations() {
        // Ctrl+A = 0x01
        let event = KeyEvent::with_modifiers(KeyCode::Char('a'), Modifiers::CTRL);
        assert_eq!(event.to_bytes(), vec![1]);

        // Ctrl+C = 0x03
        let event = KeyEvent::with_modifiers(KeyCode::Char('c'), Modifiers::CTRL);
        assert_eq!(event.to_bytes(), vec![3]);

        // Ctrl+D = 0x04 (EOF)
        let event = KeyEvent::with_modifiers(KeyCode::Char('d'), Modifiers::CTRL);
        assert_eq!(event.to_bytes(), vec![4]);

        // Ctrl+Z = 0x1A
        let event = KeyEvent::with_modifiers(KeyCode::Char('z'), Modifiers::CTRL);
        assert_eq!(event.to_bytes(), vec![26]);

        // Ctrl+[ = ESC (0x1B)
        let event = KeyEvent::with_modifiers(KeyCode::Char('['), Modifiers::CTRL);
        assert_eq!(event.to_bytes(), vec![27]);
    }

    #[test]
    fn test_encode_ctrl_uppercase() {
        // Ctrl+A and Ctrl+a should produce the same result
        let event_lower = KeyEvent::with_modifiers(KeyCode::Char('a'), Modifiers::CTRL);
        let event_upper = KeyEvent::with_modifiers(KeyCode::Char('A'), Modifiers::CTRL);
        assert_eq!(event_lower.to_bytes(), event_upper.to_bytes());
    }

    #[test]
    fn test_encode_alt_combinations() {
        // Alt+a = ESC + 'a'
        let event = KeyEvent::with_modifiers(KeyCode::Char('a'), Modifiers::ALT);
        assert_eq!(event.to_bytes(), b"\x1ba");

        // Alt+x = ESC + 'x'
        let event = KeyEvent::with_modifiers(KeyCode::Char('x'), Modifiers::ALT);
        assert_eq!(event.to_bytes(), b"\x1bx");
    }

    #[test]
    fn test_modifier_combinations() {
        let ctrl = Modifiers::CTRL;
        let shift = Modifiers::SHIFT;
        let alt = Modifiers::ALT;

        let ctrl_shift = ctrl | shift;
        assert!(ctrl_shift.contains(Modifiers::CTRL));
        assert!(ctrl_shift.contains(Modifiers::SHIFT));
        assert!(!ctrl_shift.contains(Modifiers::ALT));

        let ctrl_alt = ctrl | alt;
        assert!(ctrl_alt.contains(Modifiers::CTRL));
        assert!(ctrl_alt.contains(Modifiers::ALT));
    }

    #[test]
    fn test_keycode_equality() {
        assert_eq!(KeyCode::Char('a'), KeyCode::Char('a'));
        assert_ne!(KeyCode::Char('a'), KeyCode::Char('b'));
        assert_eq!(KeyCode::Enter, KeyCode::Enter);
        assert_eq!(KeyCode::F(1), KeyCode::F(1));
        assert_ne!(KeyCode::F(1), KeyCode::F(2));
    }

    #[test]
    fn test_key_event_equality() {
        let event1 = KeyEvent::new(KeyCode::Char('a'));
        let event2 = KeyEvent::new(KeyCode::Char('a'));
        assert_eq!(event1, event2);

        let event3 = KeyEvent::with_modifiers(KeyCode::Char('a'), Modifiers::CTRL);
        assert_ne!(event1, event3);
    }

    // Mouse event tests

    #[test]
    fn test_mouse_button_codes() {
        assert_eq!(MouseButton::Left.to_sgr_code(), 0);
        assert_eq!(MouseButton::Middle.to_sgr_code(), 1);
        assert_eq!(MouseButton::Right.to_sgr_code(), 2);
    }

    #[test]
    fn test_scroll_direction_codes() {
        assert_eq!(ScrollDirection::Up.to_sgr_code(), 64);
        assert_eq!(ScrollDirection::Down.to_sgr_code(), 65);
        assert_eq!(ScrollDirection::Left.to_sgr_code(), 66);
        assert_eq!(ScrollDirection::Right.to_sgr_code(), 67);
    }

    #[test]
    fn test_mouse_event_press() {
        let event = MouseEvent::press(10, 5, MouseButton::Left);
        assert_eq!(event.x, 10);
        assert_eq!(event.y, 5);
        assert_eq!(event.button_code, 0);
        assert!(event.is_press);
        assert_eq!(event.modifiers, Modifiers::empty());
    }

    #[test]
    fn test_mouse_event_release() {
        let event = MouseEvent::release(10, 5, MouseButton::Right);
        assert_eq!(event.x, 10);
        assert_eq!(event.y, 5);
        assert_eq!(event.button_code, 2);
        assert!(!event.is_press);
    }

    #[test]
    fn test_mouse_event_scroll() {
        let event = MouseEvent::scroll(15, 8, ScrollDirection::Up);
        assert_eq!(event.x, 15);
        assert_eq!(event.y, 8);
        assert_eq!(event.button_code, 64);
        assert!(event.is_press); // Scroll uses press encoding
    }

    #[test]
    fn test_encode_mouse_left_click() {
        // Left button press at (10, 5) -> 0-indexed coords
        // Should convert to 1-indexed: (11, 6)
        let event = MouseEvent::press(10, 5, MouseButton::Left);
        let bytes = event.to_bytes();
        assert_eq!(bytes, b"\x1b[<0;11;6M");
    }

    #[test]
    fn test_encode_mouse_left_release() {
        let event = MouseEvent::release(10, 5, MouseButton::Left);
        let bytes = event.to_bytes();
        // Note lowercase 'm' for release
        assert_eq!(bytes, b"\x1b[<0;11;6m");
    }

    #[test]
    fn test_encode_mouse_right_click() {
        let event = MouseEvent::press(20, 15, MouseButton::Right);
        let bytes = event.to_bytes();
        // Right button = 2, coords 21,16 (1-indexed)
        assert_eq!(bytes, b"\x1b[<2;21;16M");
    }

    #[test]
    fn test_encode_mouse_middle_click() {
        let event = MouseEvent::press(5, 3, MouseButton::Middle);
        let bytes = event.to_bytes();
        // Middle button = 1, coords 6,4 (1-indexed)
        assert_eq!(bytes, b"\x1b[<1;6;4M");
    }

    #[test]
    fn test_encode_mouse_scroll_up() {
        let event = MouseEvent::scroll(10, 5, ScrollDirection::Up);
        let bytes = event.to_bytes();
        // Scroll up = 64
        assert_eq!(bytes, b"\x1b[<64;11;6M");
    }

    #[test]
    fn test_encode_mouse_scroll_down() {
        let event = MouseEvent::scroll(10, 5, ScrollDirection::Down);
        let bytes = event.to_bytes();
        // Scroll down = 65
        assert_eq!(bytes, b"\x1b[<65;11;6M");
    }

    #[test]
    fn test_encode_mouse_with_shift() {
        let event = MouseEvent::press_with_modifiers(10, 5, MouseButton::Left, Modifiers::SHIFT);
        let bytes = event.to_bytes();
        // Shift adds 4 to button code: 0 + 4 = 4
        assert_eq!(bytes, b"\x1b[<4;11;6M");
    }

    #[test]
    fn test_encode_mouse_with_alt() {
        let event = MouseEvent::press_with_modifiers(10, 5, MouseButton::Left, Modifiers::ALT);
        let bytes = event.to_bytes();
        // Alt adds 8 to button code: 0 + 8 = 8
        assert_eq!(bytes, b"\x1b[<8;11;6M");
    }

    #[test]
    fn test_encode_mouse_with_ctrl() {
        let event = MouseEvent::press_with_modifiers(10, 5, MouseButton::Left, Modifiers::CTRL);
        let bytes = event.to_bytes();
        // Ctrl adds 16 to button code: 0 + 16 = 16
        assert_eq!(bytes, b"\x1b[<16;11;6M");
    }

    #[test]
    fn test_encode_mouse_with_multiple_modifiers() {
        // Ctrl+Shift
        let event = MouseEvent::press_with_modifiers(
            10,
            5,
            MouseButton::Left,
            Modifiers::CTRL | Modifiers::SHIFT,
        );
        let bytes = event.to_bytes();
        // Ctrl (16) + Shift (4) = 20
        assert_eq!(bytes, b"\x1b[<20;11;6M");
    }

    #[test]
    fn test_encode_mouse_at_origin() {
        // Test coordinates at (0,0)
        let event = MouseEvent::press(0, 0, MouseButton::Left);
        let bytes = event.to_bytes();
        // Should convert to 1-indexed: (1, 1)
        assert_eq!(bytes, b"\x1b[<0;1;1M");
    }

    #[test]
    fn test_encode_mouse_large_coordinates() {
        // Test large coordinates
        let event = MouseEvent::press(255, 100, MouseButton::Right);
        let bytes = event.to_bytes();
        // Should convert to 1-indexed: (256, 101)
        assert_eq!(bytes, b"\x1b[<2;256;101M");
    }

    #[test]
    fn test_mouse_event_equality() {
        let event1 = MouseEvent::press(10, 5, MouseButton::Left);
        let event2 = MouseEvent::press(10, 5, MouseButton::Left);
        assert_eq!(event1, event2);

        let event3 = MouseEvent::press(10, 5, MouseButton::Right);
        assert_ne!(event1, event3);

        let event4 = MouseEvent::release(10, 5, MouseButton::Left);
        assert_ne!(event1, event4); // Different is_press state
    }
}
