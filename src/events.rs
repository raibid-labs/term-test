//! Keyboard event types and escape sequence encoding.
//!
//! This module provides types for representing keyboard input and converting them
//! to VT100/ANSI escape sequences that can be sent to PTY-based applications.
//!
//! # Key Features
//!
//! - **Type-safe key codes**: Enum-based key representation
//! - **Modifier support**: Ctrl, Alt, Shift, Meta via bitflags
//! - **VT100 compliance**: Standard escape sequences for terminal compatibility
//! - **Zero allocation**: Static byte slices where possible
//!
//! # Example
//!
//! ```rust
//! use mimic::events::{KeyCode, Modifiers, KeyEvent};
//!
//! // Simple key
//! let key = KeyEvent::new(KeyCode::Char('a'));
//!
//! // Key with modifiers
//! let ctrl_c = KeyEvent::with_modifiers(
//!     KeyCode::Char('c'),
//!     Modifiers::CTRL
//! );
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
/// use mimic::events::KeyCode;
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
    /// use mimic::events::KeyCode;
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
    /// use mimic::events::Modifiers;
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
/// use mimic::events::{KeyCode, Modifiers, KeyEvent};
///
/// // Simple key press
/// let key = KeyEvent::new(KeyCode::Char('a'));
///
/// // Ctrl+C
/// let ctrl_c = KeyEvent::with_modifiers(
///     KeyCode::Char('c'),
///     Modifiers::CTRL
/// );
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
    /// use mimic::events::{KeyCode, KeyEvent};
    ///
    /// let key = KeyEvent::new(KeyCode::Char('a'));
    /// ```
    pub fn new(code: KeyCode) -> Self {
        Self {
            code,
            modifiers: Modifiers::empty(),
        }
    }

    /// Creates a new key event with modifiers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use mimic::events::{KeyCode, Modifiers, KeyEvent};
    ///
    /// let ctrl_c = KeyEvent::with_modifiers(
    ///     KeyCode::Char('c'),
    ///     Modifiers::CTRL
    /// );
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
    /// use mimic::events::{KeyCode, Modifiers, KeyEvent};
    ///
    /// let key = KeyEvent::new(KeyCode::Char('a'));
    /// let bytes = key.to_bytes();
    /// assert_eq!(bytes, b"a");
    ///
    /// let ctrl_c = KeyEvent::with_modifiers(
    ///     KeyCode::Char('c'),
    ///     Modifiers::CTRL
    /// );
    /// let bytes = ctrl_c.to_bytes();
    /// assert_eq!(bytes, vec![3]); // Ctrl+C = 0x03
    /// ```
    pub fn to_bytes(&self) -> Vec<u8> {
        encode_key_event(self)
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
        '@' => 0,      // Ctrl+@ = NUL
        '[' => 27,     // Ctrl+[ = ESC
        '\\' => 28,    // Ctrl+\ = FS
        ']' => 29,     // Ctrl+] = GS
        '^' => 30,     // Ctrl+^ = RS
        '_' => 31,     // Ctrl+_ = US
        '?' => 127,    // Ctrl+? = DEL

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
}
