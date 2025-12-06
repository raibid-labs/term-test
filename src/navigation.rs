//! Navigation testing helpers for keyboard-driven navigation systems.
//!
//! This module provides specialized testing support for TUI applications with
//! keyboard-driven navigation features like:
//!
//! - **Vimium-style hint mode**: Label-based element selection
//! - **Focus tracking**: Tab navigation and focus indicators
//! - **Mode transitions**: Normal, Hints, Visual, Insert, Search, Command modes
//! - **Prompt markers**: OSC 133 shell integration for command tracking
//!
//! # Example: Testing Hint Mode
//!
//! ```rust,no_run
//! use ratatui_testlib::{navigation::NavigationTestExt, TuiTestHarness};
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! let mut harness = TuiTestHarness::new(80, 24)?;
//! // ... spawn your app ...
//!
//! // Enter hint mode
//! harness.enter_hint_mode()?;
//!
//! // Find and activate hints
//! let hints = harness.visible_hints();
//! if let Some(hint) = hints.first() {
//!     harness.activate_hint(&hint.label)?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Testing Focus Navigation
//!
//! ```rust,no_run
//! use ratatui_testlib::{navigation::NavigationTestExt, TuiTestHarness};
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! let mut harness = TuiTestHarness::new(80, 24)?;
//! // ... spawn your app ...
//!
//! // Navigate through focusable elements
//! harness.focus_next()?;
//! let focused = harness.focused_element();
//! assert!(focused.is_some());
//! # Ok(())
//! # }
//! ```

use std::time::Duration;

use regex::Regex;

use crate::{
    error::{Result, TermTestError},
    events::KeyCode,
    screen::Rect,
    TuiTestHarness,
};

/// Navigation mode for modal TUI applications.
///
/// Represents different interaction modes commonly found in keyboard-driven
/// TUI applications, particularly those inspired by Vim-style modal editing.
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::navigation::NavMode;
///
/// let mode = NavMode::Normal;
/// assert_eq!(mode, NavMode::default());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NavMode {
    /// Normal mode - default navigation and command input.
    #[default]
    Normal,
    /// Hints mode - Vimium-style label-based selection.
    Hints,
    /// Visual mode - text selection and visual manipulation.
    Visual,
    /// Insert mode - direct text input.
    Insert,
    /// Search mode - search input and navigation.
    Search,
    /// Command mode - command palette or command line.
    Command,
}

impl NavMode {
    /// Returns a human-readable string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            NavMode::Normal => "Normal",
            NavMode::Hints => "Hints",
            NavMode::Visual => "Visual",
            NavMode::Insert => "Insert",
            NavMode::Search => "Search",
            NavMode::Command => "Command",
        }
    }
}

/// Type of hint-labeled element.
///
/// Identifies what kind of target a hint label points to, enabling
/// type-specific handling in tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintElementType {
    /// Clickable URL or hyperlink.
    Url,
    /// File system path.
    FilePath,
    /// Email address.
    Email,
    /// Git commit hash.
    GitHash,
    /// IP address (IPv4 or IPv6).
    IpAddress,
    /// Custom or application-specific element.
    Custom,
}

/// A hint label displayed in hint mode.
///
/// Represents a single hint label that can be activated to select or
/// interact with the underlying element. Commonly used in Vimium-style
/// navigation where elements are labeled with short key sequences.
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::navigation::{HintElementType, HintLabel};
///
/// let hint = HintLabel {
///     label: "a".to_string(),
///     position: (10, 5),
///     target_url: Some("https://example.com".to_string()),
///     element_type: HintElementType::Url,
/// };
///
/// assert_eq!(hint.label, "a");
/// assert_eq!(hint.position, (10, 5));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HintLabel {
    /// The hint label text (e.g., "a", "aa", "ab").
    pub label: String,
    /// Position on screen as (col, row), 0-indexed.
    pub position: (u16, u16),
    /// Target URL if this is a link hint.
    pub target_url: Option<String>,
    /// Type of element this hint targets.
    pub element_type: HintElementType,
}

/// Information about a focused UI element.
///
/// Tracks the currently focused element in the TUI, including its
/// visual bounds, identifier, and tab order information.
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::{navigation::FocusInfo, Rect};
///
/// let focus = FocusInfo {
///     bounds: Rect::new(10, 5, 20, 3),
///     id: Some("submit-button".to_string()),
///     element_type: "Button".to_string(),
///     tab_index: Some(3),
/// };
///
/// assert_eq!(focus.bounds.x, 10);
/// assert_eq!(focus.element_type, "Button");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusInfo {
    /// Visual boundaries of the focused element.
    pub bounds: Rect,
    /// Optional identifier (e.g., widget ID, element name).
    pub id: Option<String>,
    /// Type or category of the element (e.g., "Button", "Input", "List").
    pub element_type: String,
    /// Tab order index if applicable.
    pub tab_index: Option<u16>,
}

/// Type of prompt marker from OSC 133 shell integration.
///
/// OSC 133 is a shell integration protocol that marks different phases
/// of command execution in the terminal, enabling features like:
/// - Jumping between prompts
/// - Selecting command output
/// - Re-running previous commands
///
/// Reference: <https://gitlab.freedesktop.org/Per_Bothner/specifications/blob/master/proposals/semantic-prompts.md>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptMarkerType {
    /// OSC 133 ; A - Start of prompt.
    PromptStart,
    /// OSC 133 ; B - Start of command input.
    CommandStart,
    /// OSC 133 ; C - Command executed (start of output).
    CommandExecuted,
    /// OSC 133 ; D - Command finished (with exit code).
    CommandFinished,
}

/// A shell prompt marker detected via OSC 133 sequences.
///
/// Represents a specific prompt marker in the terminal output,
/// useful for testing shell-integrated TUI applications.
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::navigation::{PromptMarker, PromptMarkerType};
///
/// let marker = PromptMarker {
///     line: 10,
///     marker_type: PromptMarkerType::PromptStart,
///     command: None,
/// };
///
/// assert_eq!(marker.line, 10);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptMarker {
    /// Line number where the marker was detected (0-indexed).
    pub line: usize,
    /// Type of prompt marker.
    pub marker_type: PromptMarkerType,
    /// Command text if this is a CommandStart marker.
    pub command: Option<String>,
}

/// Extension trait for navigation testing.
///
/// Provides specialized methods for testing keyboard-driven navigation
/// systems in TUI applications. Implement this trait on your test harness
/// to gain navigation testing capabilities.
///
/// This trait is automatically implemented for [`TuiTestHarness`].
pub trait NavigationTestExt {
    /// Enter hint mode (typically by pressing 'f').
    ///
    /// Sends the hint mode activation key and waits for hint labels to appear.
    /// Uses a default timeout of 1 second.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The key cannot be sent
    /// - Hint mode doesn't activate within the timeout
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{navigation::NavigationTestExt, TuiTestHarness};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// // ... spawn app ...
    /// harness.enter_hint_mode()?;
    /// # Ok(())
    /// # }
    /// ```
    fn enter_hint_mode(&mut self) -> Result<()>;

    /// Exit to normal mode (typically by pressing Escape).
    ///
    /// Sends the Escape key to exit the current mode and return to normal mode.
    ///
    /// # Errors
    ///
    /// Returns an error if the key cannot be sent.
    fn exit_to_normal(&mut self) -> Result<()>;

    /// Detect the current navigation mode.
    ///
    /// Analyzes the screen state to determine which mode the application
    /// is currently in. Detection is based on visual indicators like:
    /// - Presence of hint labels (Hints mode)
    /// - Status line text
    /// - Visual selection indicators
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{
    ///     navigation::{NavMode, NavigationTestExt},
    ///     TuiTestHarness,
    /// };
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// assert_eq!(harness.current_mode(), NavMode::Normal);
    /// # Ok(())
    /// # }
    /// ```
    fn current_mode(&self) -> NavMode;

    /// Wait for a specific navigation mode.
    ///
    /// Polls the screen state until the specified mode is detected or
    /// the timeout is reached.
    ///
    /// # Arguments
    ///
    /// * `mode` - The mode to wait for
    /// * `timeout` - Maximum time to wait
    ///
    /// # Errors
    ///
    /// Returns a timeout error if the mode is not reached within the timeout.
    fn wait_for_mode(&mut self, mode: NavMode, timeout: Duration) -> Result<()>;

    /// Get all visible hint labels on the screen.
    ///
    /// Scans the screen for hint label patterns like `[a]`, `[ab]`, etc.
    /// Returns a vector of detected hints with their positions.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{navigation::NavigationTestExt, TuiTestHarness};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.enter_hint_mode()?;
    ///
    /// let hints = harness.visible_hints();
    /// for hint in &hints {
    ///     println!("Hint '{}' at ({}, {})", hint.label, hint.position.0, hint.position.1);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn visible_hints(&self) -> Vec<HintLabel>;

    /// Get the hint at a specific screen position.
    ///
    /// # Arguments
    ///
    /// * `col` - Column position (0-indexed)
    /// * `row` - Row position (0-indexed)
    ///
    /// # Returns
    ///
    /// The hint at that position, or `None` if no hint exists there.
    fn hint_at(&self, col: u16, row: u16) -> Option<HintLabel>;

    /// Activate a hint by typing its label.
    ///
    /// Types the hint label characters to activate the hint and select
    /// the underlying element.
    ///
    /// # Arguments
    ///
    /// * `label` - The hint label to activate (e.g., "a", "ab")
    ///
    /// # Errors
    ///
    /// Returns an error if the keys cannot be sent.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{navigation::NavigationTestExt, TuiTestHarness};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// harness.enter_hint_mode()?;
    ///
    /// // Activate the first hint
    /// let hints = harness.visible_hints();
    /// if let Some(hint) = hints.first() {
    ///     harness.activate_hint(&hint.label)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn activate_hint(&mut self, label: &str) -> Result<()>;

    /// Get information about the currently focused element.
    ///
    /// Detects focus indicators on the screen such as:
    /// - Highlighted borders
    /// - Cursor position in input fields
    /// - Selection markers
    ///
    /// # Returns
    ///
    /// Focus information if an element is focused, or `None` otherwise.
    fn focused_element(&self) -> Option<FocusInfo>;

    /// Move focus to the next element (typically Tab key).
    ///
    /// # Errors
    ///
    /// Returns an error if the Tab key cannot be sent.
    fn focus_next(&mut self) -> Result<()>;

    /// Move focus to the previous element (typically Shift+Tab).
    ///
    /// # Errors
    ///
    /// Returns an error if the Shift+Tab key cannot be sent.
    fn focus_prev(&mut self) -> Result<()>;

    /// Get all shell prompt markers from OSC 133 sequences.
    ///
    /// Parses the screen state for OSC 133 prompt markers, returning
    /// a list of all detected markers in order.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::{navigation::NavigationTestExt, TuiTestHarness};
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = TuiTestHarness::new(80, 24)?;
    /// // ... spawn shell ...
    ///
    /// let markers = harness.prompt_markers();
    /// println!("Found {} prompts", markers.len());
    /// # Ok(())
    /// # }
    /// ```
    fn prompt_markers(&self) -> Vec<PromptMarker>;

    /// Jump to a specific prompt by index.
    ///
    /// Moves the cursor or scrolls to the Nth prompt marker.
    ///
    /// # Arguments
    ///
    /// * `index` - The 0-based prompt index to jump to
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The index is out of bounds
    /// - The jump command cannot be sent
    fn jump_to_prompt(&mut self, index: usize) -> Result<()>;

    /// Get the index of the current prompt.
    ///
    /// Determines which prompt the cursor is currently at or near.
    ///
    /// # Returns
    ///
    /// The 0-based prompt index, or `None` if not at a prompt.
    fn current_prompt_index(&self) -> Option<usize>;
}

impl NavigationTestExt for TuiTestHarness {
    fn enter_hint_mode(&mut self) -> Result<()> {
        // Send 'f' key which is the common Vimium-style hint activation
        self.send_key(KeyCode::Char('f'))?;

        // Wait for hints to appear (look for hint label patterns)
        self.wait_for(|state| {
            // Look for hint patterns like [a], [b], [aa], etc.
            let contents = state.contents();
            contents.contains('[') && contents.contains(']')
        })
    }

    fn exit_to_normal(&mut self) -> Result<()> {
        self.send_key(KeyCode::Esc)
    }

    fn current_mode(&self) -> NavMode {
        let contents = self.screen_contents();

        // Check for hint mode (presence of hint labels)
        if contains_hint_labels(&contents) {
            return NavMode::Hints;
        }

        // Check status line for mode indicators
        let lines: Vec<&str> = contents.lines().collect();

        // Check last line (common status line location)
        if let Some(last_line) = lines.last() {
            let lower = last_line.to_lowercase();
            if lower.contains("-- visual --") || lower.contains("visual mode") {
                return NavMode::Visual;
            }
            if lower.contains("-- insert --") || lower.contains("insert mode") {
                return NavMode::Insert;
            }
            if lower.contains("-- search --")
                || lower.contains("search:")
                || lower.contains("search mode")
            {
                return NavMode::Search;
            }
            if lower.contains("-- command --")
                || lower.contains("command:")
                || lower.contains("command mode")
            {
                return NavMode::Command;
            }
        }

        // Check first line (alternative status line location)
        if let Some(first_line) = lines.first() {
            let lower = first_line.to_lowercase();
            if lower.contains("visual") {
                return NavMode::Visual;
            }
            if lower.contains("insert") {
                return NavMode::Insert;
            }
            if lower.contains("search") {
                return NavMode::Search;
            }
            if lower.contains("command") {
                return NavMode::Command;
            }
        }

        // Default to Normal mode
        NavMode::Normal
    }

    fn wait_for_mode(&mut self, mode: NavMode, timeout: Duration) -> Result<()> {
        use std::time::Instant;

        let start = Instant::now();

        loop {
            // Update state
            match self.update_state() {
                Ok(()) => {
                    let contents = self.state().contents();
                    if detect_mode(&contents) == mode {
                        return Ok(());
                    }
                }
                Err(TermTestError::ProcessExited) => {
                    let contents = self.state().contents();
                    if detect_mode(&contents) == mode {
                        return Ok(());
                    }
                    return Err(TermTestError::ProcessExited);
                }
                Err(e) => return Err(e),
            }

            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return Err(TermTestError::Timeout { timeout_ms: timeout.as_millis() as u64 });
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn visible_hints(&self) -> Vec<HintLabel> {
        let mut hints = Vec::new();
        let contents = self.screen_contents();

        // Regex to match hint labels: [a], [aa], [ab], etc.
        // Matches: [ followed by 1-2 lowercase letters followed by ]
        let hint_regex = Regex::new(r"\[([a-z]{1,2})\]").unwrap();

        for (row_idx, line) in contents.lines().enumerate() {
            for capture in hint_regex.captures_iter(line) {
                if let Some(matched) = capture.get(0) {
                    let label = capture.get(1).unwrap().as_str().to_string();
                    let col = matched.start() as u16;
                    let row = row_idx as u16;

                    // Try to determine element type from context
                    let element_type = if line.contains("http://") || line.contains("https://") {
                        HintElementType::Url
                    } else if line.contains('/') && line.contains('.') {
                        HintElementType::FilePath
                    } else if line.contains('@') {
                        HintElementType::Email
                    } else if line.len() >= 7
                        && line[matched.end()..]
                            .chars()
                            .take(7)
                            .all(|c| c.is_ascii_hexdigit())
                    {
                        HintElementType::GitHash
                    } else {
                        HintElementType::Custom
                    };

                    hints.push(HintLabel {
                        label,
                        position: (col, row),
                        target_url: None, // Would need more context to extract
                        element_type,
                    });
                }
            }
        }

        hints
    }

    fn hint_at(&self, col: u16, row: u16) -> Option<HintLabel> {
        self.visible_hints()
            .into_iter()
            .find(|hint| hint.position == (col, row))
    }

    fn activate_hint(&mut self, label: &str) -> Result<()> {
        // Type each character of the hint label
        for ch in label.chars() {
            self.send_key(KeyCode::Char(ch))?;
        }
        Ok(())
    }

    fn focused_element(&self) -> Option<FocusInfo> {
        // Look for focus indicators on the screen
        // This is a basic implementation that looks for common patterns

        let contents = self.screen_contents();
        let lines: Vec<&str> = contents.lines().collect();

        // Look for visual focus indicators like borders or highlights
        for (row_idx, line) in lines.iter().enumerate() {
            // Pattern 1: Box-drawing borders that might indicate focus
            if line.contains('┃') || line.contains('║') {
                // Check if there's a highlighted region
                let col = line.find('┃').or_else(|| line.find('║')).unwrap_or(0);

                return Some(FocusInfo {
                    bounds: Rect::new(col as u16, row_idx as u16, 20, 3),
                    id: None,
                    element_type: "Widget".to_string(),
                    tab_index: None,
                });
            }

            // Pattern 2: Cursor position might indicate focus
            // This would require cursor position tracking from the terminal state
        }

        // Check cursor position from state
        let (cursor_row, cursor_col) = self.state().cursor_position();
        if cursor_row < lines.len() as u16 {
            return Some(FocusInfo {
                bounds: Rect::new(cursor_col, cursor_row, 10, 1),
                id: None,
                element_type: "Input".to_string(),
                tab_index: None,
            });
        }

        None
    }

    fn focus_next(&mut self) -> Result<()> {
        self.send_key(KeyCode::Tab)
    }

    fn focus_prev(&mut self) -> Result<()> {
        use crate::events::Modifiers;
        self.send_key_with_modifiers(KeyCode::Tab, Modifiers::SHIFT)
    }

    fn prompt_markers(&self) -> Vec<PromptMarker> {
        // OSC 133 sequences are typically not visible in the rendered output
        // They would need to be tracked during parsing
        // This is a placeholder that would need integration with screen.rs

        // For now, return empty - this requires OSC 133 tracking in TerminalState
        Vec::new()
    }

    fn jump_to_prompt(&mut self, _index: usize) -> Result<()> {
        // This would require application-specific commands
        // Placeholder implementation
        Err(TermTestError::Parse(
            "Prompt jumping not yet implemented - requires OSC 133 tracking".to_string(),
        ))
    }

    fn current_prompt_index(&self) -> Option<usize> {
        // Would need to compare cursor position to prompt marker positions
        None
    }
}

/// Helper function to detect if content contains hint labels.
fn contains_hint_labels(contents: &str) -> bool {
    // Look for hint label patterns like [a], [b], [aa]
    let hint_regex = Regex::new(r"\[([a-z]{1,2})\]").unwrap();
    hint_regex.is_match(contents)
}

/// Helper function to detect navigation mode from screen contents.
fn detect_mode(contents: &str) -> NavMode {
    if contains_hint_labels(contents) {
        return NavMode::Hints;
    }

    let lower = contents.to_lowercase();
    if lower.contains("-- visual --") || lower.contains("visual mode") {
        return NavMode::Visual;
    }
    if lower.contains("-- insert --") || lower.contains("insert mode") {
        return NavMode::Insert;
    }
    if lower.contains("-- search --") || lower.contains("search:") || lower.contains("search mode")
    {
        return NavMode::Search;
    }
    if lower.contains("-- command --")
        || lower.contains("command:")
        || lower.contains("command mode")
    {
        return NavMode::Command;
    }

    NavMode::Normal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nav_mode_default() {
        assert_eq!(NavMode::default(), NavMode::Normal);
    }

    #[test]
    fn test_nav_mode_as_str() {
        assert_eq!(NavMode::Normal.as_str(), "Normal");
        assert_eq!(NavMode::Hints.as_str(), "Hints");
        assert_eq!(NavMode::Visual.as_str(), "Visual");
        assert_eq!(NavMode::Insert.as_str(), "Insert");
        assert_eq!(NavMode::Search.as_str(), "Search");
        assert_eq!(NavMode::Command.as_str(), "Command");
    }

    #[test]
    fn test_contains_hint_labels() {
        assert!(contains_hint_labels("Link [a] and [b]"));
        assert!(contains_hint_labels("[aa] [ab] [ac]"));
        assert!(!contains_hint_labels("No hints here"));
        assert!(!contains_hint_labels("[123] [ABC]")); // Not lowercase letters
    }

    #[test]
    fn test_detect_mode_normal() {
        let contents = "Just regular content";
        assert_eq!(detect_mode(contents), NavMode::Normal);
    }

    #[test]
    fn test_detect_mode_hints() {
        let contents = "Links: [a] Google [b] GitHub";
        assert_eq!(detect_mode(contents), NavMode::Hints);
    }

    #[test]
    fn test_detect_mode_visual() {
        let contents = "Some text\n-- VISUAL --";
        assert_eq!(detect_mode(contents), NavMode::Visual);
    }

    #[test]
    fn test_detect_mode_insert() {
        let contents = "-- INSERT --\nTyping...";
        assert_eq!(detect_mode(contents), NavMode::Insert);
    }

    #[test]
    fn test_detect_mode_search() {
        let contents = "Search: query\n-- SEARCH --";
        assert_eq!(detect_mode(contents), NavMode::Search);
    }

    #[test]
    fn test_detect_mode_command() {
        let contents = "Command: :wq\n-- COMMAND --";
        assert_eq!(detect_mode(contents), NavMode::Command);
    }

    #[test]
    fn test_hint_label_creation() {
        let hint = HintLabel {
            label: "a".to_string(),
            position: (10, 5),
            target_url: Some("https://example.com".to_string()),
            element_type: HintElementType::Url,
        };

        assert_eq!(hint.label, "a");
        assert_eq!(hint.position, (10, 5));
        assert!(hint.target_url.is_some());
        assert_eq!(hint.element_type, HintElementType::Url);
    }

    #[test]
    fn test_focus_info_creation() {
        let focus = FocusInfo {
            bounds: Rect::new(10, 5, 20, 3),
            id: Some("button-1".to_string()),
            element_type: "Button".to_string(),
            tab_index: Some(0),
        };

        assert_eq!(focus.bounds.x, 10);
        assert_eq!(focus.bounds.y, 5);
        assert_eq!(focus.bounds.width, 20);
        assert_eq!(focus.bounds.height, 3);
        assert_eq!(focus.id, Some("button-1".to_string()));
        assert_eq!(focus.element_type, "Button");
        assert_eq!(focus.tab_index, Some(0));
    }

    #[test]
    fn test_prompt_marker_creation() {
        let marker = PromptMarker {
            line: 10,
            marker_type: PromptMarkerType::PromptStart,
            command: None,
        };

        assert_eq!(marker.line, 10);
        assert_eq!(marker.marker_type, PromptMarkerType::PromptStart);
        assert!(marker.command.is_none());
    }

    #[test]
    fn test_prompt_marker_with_command() {
        let marker = PromptMarker {
            line: 15,
            marker_type: PromptMarkerType::CommandStart,
            command: Some("ls -la".to_string()),
        };

        assert_eq!(marker.line, 15);
        assert_eq!(marker.marker_type, PromptMarkerType::CommandStart);
        assert_eq!(marker.command, Some("ls -la".to_string()));
    }

    #[test]
    fn test_hint_element_types() {
        // Just verify the types exist and can be compared
        assert_ne!(HintElementType::Url, HintElementType::FilePath);
        assert_ne!(HintElementType::Email, HintElementType::GitHash);
        assert_ne!(HintElementType::IpAddress, HintElementType::Custom);
    }
}
