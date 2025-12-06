//! Terminal profile definitions for multi-terminal compatibility testing.
//!
//! This module provides terminal profiles that represent different terminal emulators
//! with their specific capabilities and behaviors. This enables testing TUI applications
//! across different terminal environments to ensure compatibility.
//!
//! # Overview
//!
//! Different terminal emulators support different features:
//! - Color depth (8, 16, 256, or true color)
//! - Unicode support (basic ASCII, UTF-8, wide characters)
//! - Mouse protocols (X10, VT200, SGR, UTF-8, etc.)
//! - Graphics protocols (Sixel, iTerm2, Kitty)
//! - Special features (synchronized output, bracketed paste)
//!
//! # Example
//!
//! ```rust
//! use ratatui_testlib::{Feature, TerminalProfile, TuiTestHarness};
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! // Create a harness configured for xterm
//! let mut harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::Xterm256);
//!
//! // Check if current profile supports Sixel
//! if harness.supports_feature(Feature::Sixel) {
//!     println!("Sixel graphics are supported");
//! }
//!
//! // Get full capability report
//! let caps = harness.terminal_capabilities();
//! println!("Color depth: {:?}", caps.color_depth);
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

/// Features that may be supported by terminal emulators.
///
/// This enum represents optional capabilities that vary across different
/// terminal emulators. Use [`TerminalProfile::supports`] to check if a
/// specific terminal supports a given feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    /// Sixel graphics protocol support
    Sixel,
    /// iTerm2 inline image protocol
    ITerm2Images,
    /// Kitty graphics protocol
    KittyGraphics,
    /// 256-color palette support
    Colors256,
    /// 24-bit true color support
    TrueColor,
    /// UTF-8 unicode support
    Unicode,
    /// Wide character (emoji, CJK) support
    WideCharacters,
    /// X10 mouse protocol (button press/release)
    MouseX10,
    /// VT200 mouse protocol (button press/release with modifiers)
    MouseVT200,
    /// SGR mouse protocol (1006) - extended coordinates
    MouseSGR,
    /// UTF-8 mouse protocol (1005)
    MouseUTF8,
    /// Mouse motion tracking
    MouseMotion,
    /// Bracketed paste mode
    BracketedPaste,
    /// Synchronized output (2026)
    SynchronizedOutput,
    /// Alternate screen buffer
    AlternateScreen,
    /// Title setting support
    SetTitle,
    /// Focus in/out events
    FocusEvents,
}

/// Color depth supported by a terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ColorDepth {
    /// Monochrome (no color)
    Monochrome,
    /// 8 colors (basic ANSI)
    Colors8,
    /// 16 colors (ANSI + bright variants)
    Colors16,
    /// 256 colors (ANSI extended)
    Colors256,
    /// 24-bit true color (16.7 million colors)
    TrueColor,
}

/// Mouse protocol encoding format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseProtocol {
    /// No mouse support
    None,
    /// X10 mouse protocol (press/release only)
    X10,
    /// VT200 mouse protocol (press/release with modifiers)
    VT200,
    /// SGR mouse protocol (1006) - extended coordinates
    SGR,
    /// UTF-8 mouse protocol (1005)
    UTF8,
}

/// Terminal capabilities configuration.
///
/// This struct describes the capabilities of a terminal emulator, including
/// color support, unicode handling, mouse protocols, and graphics support.
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::{ColorDepth, MouseProtocol, TerminalCapabilities};
///
/// let caps = TerminalCapabilities {
///     color_depth: ColorDepth::TrueColor,
///     unicode_support: true,
///     wide_char_support: true,
///     mouse_protocol: MouseProtocol::SGR,
///     sixel_support: true,
///     ..Default::default()
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TerminalCapabilities {
    /// Maximum color depth supported
    pub color_depth: ColorDepth,
    /// Whether UTF-8 unicode is supported
    pub unicode_support: bool,
    /// Whether wide characters (emoji, CJK) render correctly
    pub wide_char_support: bool,
    /// Primary mouse protocol supported
    pub mouse_protocol: MouseProtocol,
    /// Whether Sixel graphics are supported
    pub sixel_support: bool,
    /// Whether iTerm2 inline images are supported
    pub iterm2_images: bool,
    /// Whether Kitty graphics protocol is supported
    pub kitty_graphics: bool,
    /// Whether bracketed paste mode is supported
    pub bracketed_paste: bool,
    /// Whether synchronized output is supported
    pub synchronized_output: bool,
    /// Whether alternate screen buffer is supported
    pub alternate_screen: bool,
    /// Whether title setting is supported
    pub set_title: bool,
    /// Whether focus in/out events are supported
    pub focus_events: bool,
    /// TERM environment variable value
    pub term_name: String,
    /// Additional custom capabilities
    pub custom: HashMap<String, String>,
}

impl Default for TerminalCapabilities {
    fn default() -> Self {
        Self {
            color_depth: ColorDepth::Colors16,
            unicode_support: true,
            wide_char_support: false,
            mouse_protocol: MouseProtocol::None,
            sixel_support: false,
            iterm2_images: false,
            kitty_graphics: false,
            bracketed_paste: false,
            synchronized_output: false,
            alternate_screen: true,
            set_title: false,
            focus_events: false,
            term_name: "xterm".to_string(),
            custom: HashMap::new(),
        }
    }
}

impl TerminalCapabilities {
    /// Checks if a specific feature is supported.
    ///
    /// # Arguments
    ///
    /// * `feature` - The feature to check
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::{Feature, TerminalCapabilities};
    ///
    /// let caps = TerminalCapabilities::default();
    /// assert!(caps.supports(Feature::AlternateScreen));
    /// ```
    pub fn supports(&self, feature: Feature) -> bool {
        match feature {
            Feature::Sixel => self.sixel_support,
            Feature::ITerm2Images => self.iterm2_images,
            Feature::KittyGraphics => self.kitty_graphics,
            Feature::Colors256 => self.color_depth >= ColorDepth::Colors256,
            Feature::TrueColor => self.color_depth >= ColorDepth::TrueColor,
            Feature::Unicode => self.unicode_support,
            Feature::WideCharacters => self.wide_char_support,
            Feature::MouseX10 => self.mouse_protocol != MouseProtocol::None,
            Feature::MouseVT200 => matches!(
                self.mouse_protocol,
                MouseProtocol::VT200 | MouseProtocol::SGR | MouseProtocol::UTF8
            ),
            Feature::MouseSGR => matches!(self.mouse_protocol, MouseProtocol::SGR),
            Feature::MouseUTF8 => matches!(self.mouse_protocol, MouseProtocol::UTF8),
            Feature::MouseMotion => self.mouse_protocol != MouseProtocol::None,
            Feature::BracketedPaste => self.bracketed_paste,
            Feature::SynchronizedOutput => self.synchronized_output,
            Feature::AlternateScreen => self.alternate_screen,
            Feature::SetTitle => self.set_title,
            Feature::FocusEvents => self.focus_events,
        }
    }

    /// Returns a human-readable summary of capabilities.
    pub fn summary(&self) -> String {
        format!(
            "Terminal Capabilities:\n\
             - TERM: {}\n\
             - Color Depth: {:?}\n\
             - Unicode: {}\n\
             - Wide Chars: {}\n\
             - Mouse: {:?}\n\
             - Sixel: {}\n\
             - iTerm2 Images: {}\n\
             - Kitty Graphics: {}\n\
             - Bracketed Paste: {}\n\
             - Synchronized Output: {}\n\
             - Alternate Screen: {}\n\
             - Set Title: {}\n\
             - Focus Events: {}",
            self.term_name,
            self.color_depth,
            self.unicode_support,
            self.wide_char_support,
            self.mouse_protocol,
            self.sixel_support,
            self.iterm2_images,
            self.kitty_graphics,
            self.bracketed_paste,
            self.synchronized_output,
            self.alternate_screen,
            self.set_title,
            self.focus_events,
        )
    }
}

/// Predefined terminal emulator profiles.
///
/// This enum represents common terminal emulators with their typical
/// capability configurations. Use these profiles to test your TUI application
/// across different terminal environments.
///
/// # Example
///
/// ```rust
/// use ratatui_testlib::{TerminalProfile, TuiTestHarness};
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// // Test with basic VT100 compatibility
/// let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::VT100);
///
/// // Test with modern terminal features
/// let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::WezTerm);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalProfile {
    /// Basic VT100 terminal (monochrome, no mouse, minimal features)
    VT100,
    /// xterm with 256 colors and basic mouse support
    Xterm256,
    /// Modern xterm with true color support
    XtermTrueColor,
    /// GNU Screen multiplexer (limited color, no graphics)
    Screen,
    /// tmux terminal multiplexer (256 colors, limited mouse)
    Tmux,
    /// Konsole KDE terminal emulator (true color, no Sixel)
    Konsole,
    /// GNOME Terminal (true color, limited features)
    GnomeTerminal,
    /// Alacritty GPU-accelerated terminal (true color, no graphics protocols)
    Alacritty,
    /// Kitty terminal with graphics protocol (true color, Kitty graphics)
    Kitty,
    /// WezTerm with full features (true color, Sixel, modern protocols)
    WezTerm,
    /// iTerm2 macOS terminal (true color, iTerm2 images)
    ITerm2,
    /// Windows Terminal (true color, modern features)
    WindowsTerminal,
    /// VSCode integrated terminal (limited features)
    VSCode,
    /// Minimal testing profile (basic features only)
    Minimal,
    /// Maximum feature set for testing (all features enabled)
    Maximum,
}

impl TerminalProfile {
    /// Returns the capabilities for this terminal profile.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::{Feature, TerminalProfile};
    ///
    /// let caps = TerminalProfile::WezTerm.capabilities();
    /// assert!(caps.supports(Feature::Sixel));
    /// assert!(caps.supports(Feature::TrueColor));
    /// ```
    pub fn capabilities(&self) -> TerminalCapabilities {
        match self {
            Self::VT100 => TerminalCapabilities {
                color_depth: ColorDepth::Monochrome,
                unicode_support: false,
                wide_char_support: false,
                mouse_protocol: MouseProtocol::None,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: false,
                bracketed_paste: false,
                synchronized_output: false,
                alternate_screen: false,
                set_title: false,
                focus_events: false,
                term_name: "vt100".to_string(),
                custom: HashMap::new(),
            },
            Self::Xterm256 => TerminalCapabilities {
                color_depth: ColorDepth::Colors256,
                unicode_support: true,
                wide_char_support: false,
                mouse_protocol: MouseProtocol::VT200,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: false,
                bracketed_paste: true,
                synchronized_output: false,
                alternate_screen: true,
                set_title: true,
                focus_events: true,
                term_name: "xterm-256color".to_string(),
                custom: HashMap::new(),
            },
            Self::XtermTrueColor => TerminalCapabilities {
                color_depth: ColorDepth::TrueColor,
                unicode_support: true,
                wide_char_support: true,
                mouse_protocol: MouseProtocol::SGR,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: false,
                bracketed_paste: true,
                synchronized_output: false,
                alternate_screen: true,
                set_title: true,
                focus_events: true,
                term_name: "xterm-256color".to_string(),
                custom: HashMap::new(),
            },
            Self::Screen => TerminalCapabilities {
                color_depth: ColorDepth::Colors256,
                unicode_support: true,
                wide_char_support: false,
                mouse_protocol: MouseProtocol::X10,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: false,
                bracketed_paste: false,
                synchronized_output: false,
                alternate_screen: true,
                set_title: false,
                focus_events: false,
                term_name: "screen".to_string(),
                custom: HashMap::new(),
            },
            Self::Tmux => TerminalCapabilities {
                color_depth: ColorDepth::Colors256,
                unicode_support: true,
                wide_char_support: true,
                mouse_protocol: MouseProtocol::SGR,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: false,
                bracketed_paste: true,
                synchronized_output: false,
                alternate_screen: true,
                set_title: true,
                focus_events: true,
                term_name: "tmux-256color".to_string(),
                custom: HashMap::new(),
            },
            Self::Konsole => TerminalCapabilities {
                color_depth: ColorDepth::TrueColor,
                unicode_support: true,
                wide_char_support: true,
                mouse_protocol: MouseProtocol::SGR,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: false,
                bracketed_paste: true,
                synchronized_output: false,
                alternate_screen: true,
                set_title: true,
                focus_events: true,
                term_name: "konsole-256color".to_string(),
                custom: HashMap::new(),
            },
            Self::GnomeTerminal => TerminalCapabilities {
                color_depth: ColorDepth::TrueColor,
                unicode_support: true,
                wide_char_support: true,
                mouse_protocol: MouseProtocol::SGR,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: false,
                bracketed_paste: true,
                synchronized_output: false,
                alternate_screen: true,
                set_title: true,
                focus_events: true,
                term_name: "xterm-256color".to_string(),
                custom: HashMap::new(),
            },
            Self::Alacritty => TerminalCapabilities {
                color_depth: ColorDepth::TrueColor,
                unicode_support: true,
                wide_char_support: true,
                mouse_protocol: MouseProtocol::SGR,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: false,
                bracketed_paste: true,
                synchronized_output: true,
                alternate_screen: true,
                set_title: true,
                focus_events: true,
                term_name: "alacritty".to_string(),
                custom: HashMap::new(),
            },
            Self::Kitty => TerminalCapabilities {
                color_depth: ColorDepth::TrueColor,
                unicode_support: true,
                wide_char_support: true,
                mouse_protocol: MouseProtocol::SGR,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: true,
                bracketed_paste: true,
                synchronized_output: true,
                alternate_screen: true,
                set_title: true,
                focus_events: true,
                term_name: "xterm-kitty".to_string(),
                custom: HashMap::new(),
            },
            Self::WezTerm => TerminalCapabilities {
                color_depth: ColorDepth::TrueColor,
                unicode_support: true,
                wide_char_support: true,
                mouse_protocol: MouseProtocol::SGR,
                sixel_support: true,
                iterm2_images: true,
                kitty_graphics: false,
                bracketed_paste: true,
                synchronized_output: true,
                alternate_screen: true,
                set_title: true,
                focus_events: true,
                term_name: "wezterm".to_string(),
                custom: HashMap::new(),
            },
            Self::ITerm2 => TerminalCapabilities {
                color_depth: ColorDepth::TrueColor,
                unicode_support: true,
                wide_char_support: true,
                mouse_protocol: MouseProtocol::SGR,
                sixel_support: true,
                iterm2_images: true,
                kitty_graphics: false,
                bracketed_paste: true,
                synchronized_output: false,
                alternate_screen: true,
                set_title: true,
                focus_events: true,
                term_name: "xterm-256color".to_string(),
                custom: HashMap::new(),
            },
            Self::WindowsTerminal => TerminalCapabilities {
                color_depth: ColorDepth::TrueColor,
                unicode_support: true,
                wide_char_support: true,
                mouse_protocol: MouseProtocol::SGR,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: false,
                bracketed_paste: true,
                synchronized_output: true,
                alternate_screen: true,
                set_title: true,
                focus_events: true,
                term_name: "xterm-256color".to_string(),
                custom: HashMap::new(),
            },
            Self::VSCode => TerminalCapabilities {
                color_depth: ColorDepth::TrueColor,
                unicode_support: true,
                wide_char_support: true,
                mouse_protocol: MouseProtocol::SGR,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: false,
                bracketed_paste: true,
                synchronized_output: false,
                alternate_screen: true,
                set_title: false,
                focus_events: false,
                term_name: "xterm-256color".to_string(),
                custom: HashMap::new(),
            },
            Self::Minimal => TerminalCapabilities {
                color_depth: ColorDepth::Colors16,
                unicode_support: false,
                wide_char_support: false,
                mouse_protocol: MouseProtocol::None,
                sixel_support: false,
                iterm2_images: false,
                kitty_graphics: false,
                bracketed_paste: false,
                synchronized_output: false,
                alternate_screen: true,
                set_title: false,
                focus_events: false,
                term_name: "xterm".to_string(),
                custom: HashMap::new(),
            },
            Self::Maximum => TerminalCapabilities {
                color_depth: ColorDepth::TrueColor,
                unicode_support: true,
                wide_char_support: true,
                mouse_protocol: MouseProtocol::SGR,
                sixel_support: true,
                iterm2_images: true,
                kitty_graphics: true,
                bracketed_paste: true,
                synchronized_output: true,
                alternate_screen: true,
                set_title: true,
                focus_events: true,
                term_name: "xterm-256color".to_string(),
                custom: HashMap::new(),
            },
        }
    }

    /// Checks if this profile supports a specific feature.
    ///
    /// # Arguments
    ///
    /// * `feature` - The feature to check
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::{Feature, TerminalProfile};
    ///
    /// assert!(TerminalProfile::WezTerm.supports(Feature::Sixel));
    /// assert!(!TerminalProfile::Alacritty.supports(Feature::Sixel));
    /// ```
    pub fn supports(&self, feature: Feature) -> bool {
        self.capabilities().supports(feature)
    }

    /// Returns the TERM environment variable value for this profile.
    pub fn term_name(&self) -> &str {
        match self {
            Self::VT100 => "vt100",
            Self::Xterm256 | Self::XtermTrueColor => "xterm-256color",
            Self::Screen => "screen",
            Self::Tmux => "tmux-256color",
            Self::Konsole => "konsole-256color",
            Self::GnomeTerminal | Self::WindowsTerminal | Self::VSCode | Self::ITerm2 => {
                "xterm-256color"
            }
            Self::Alacritty => "alacritty",
            Self::Kitty => "xterm-kitty",
            Self::WezTerm => "wezterm",
            Self::Minimal => "xterm",
            Self::Maximum => "xterm-256color",
        }
    }

    /// Returns all available terminal profiles.
    pub fn all() -> Vec<Self> {
        vec![
            Self::VT100,
            Self::Xterm256,
            Self::XtermTrueColor,
            Self::Screen,
            Self::Tmux,
            Self::Konsole,
            Self::GnomeTerminal,
            Self::Alacritty,
            Self::Kitty,
            Self::WezTerm,
            Self::ITerm2,
            Self::WindowsTerminal,
            Self::VSCode,
            Self::Minimal,
            Self::Maximum,
        ]
    }

    /// Returns a profile by name (case-insensitive).
    ///
    /// # Arguments
    ///
    /// * `name` - The profile name or TERM value
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::TerminalProfile;
    ///
    /// assert_eq!(TerminalProfile::from_name("wezterm"), Some(TerminalProfile::WezTerm));
    /// assert_eq!(TerminalProfile::from_name("xterm-256color"), Some(TerminalProfile::Xterm256));
    /// ```
    pub fn from_name(name: &str) -> Option<Self> {
        let name_lower = name.to_lowercase();
        match name_lower.as_str() {
            "vt100" => Some(Self::VT100),
            "xterm-256color" | "xterm256" => Some(Self::Xterm256),
            "xterm-truecolor" | "xterm-direct" => Some(Self::XtermTrueColor),
            "screen" => Some(Self::Screen),
            "tmux" | "tmux-256color" => Some(Self::Tmux),
            "konsole" | "konsole-256color" => Some(Self::Konsole),
            "gnome" | "gnome-terminal" => Some(Self::GnomeTerminal),
            "alacritty" => Some(Self::Alacritty),
            "kitty" | "xterm-kitty" => Some(Self::Kitty),
            "wezterm" => Some(Self::WezTerm),
            "iterm2" | "iterm" => Some(Self::ITerm2),
            "windows-terminal" | "wt" => Some(Self::WindowsTerminal),
            "vscode" => Some(Self::VSCode),
            "minimal" => Some(Self::Minimal),
            "maximum" | "max" => Some(Self::Maximum),
            _ => None,
        }
    }

    /// Returns a human-readable name for this profile.
    pub fn display_name(&self) -> &str {
        match self {
            Self::VT100 => "VT100",
            Self::Xterm256 => "xterm-256color",
            Self::XtermTrueColor => "xterm (true color)",
            Self::Screen => "GNU Screen",
            Self::Tmux => "tmux",
            Self::Konsole => "Konsole",
            Self::GnomeTerminal => "GNOME Terminal",
            Self::Alacritty => "Alacritty",
            Self::Kitty => "Kitty",
            Self::WezTerm => "WezTerm",
            Self::ITerm2 => "iTerm2",
            Self::WindowsTerminal => "Windows Terminal",
            Self::VSCode => "VSCode Terminal",
            Self::Minimal => "Minimal (testing)",
            Self::Maximum => "Maximum (testing)",
        }
    }
}

impl Default for TerminalProfile {
    /// Returns the default terminal profile (Xterm256).
    ///
    /// This provides a good balance of features for testing without assuming
    /// advanced graphics protocol support.
    fn default() -> Self {
        Self::Xterm256
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vt100_profile() {
        let profile = TerminalProfile::VT100;
        let caps = profile.capabilities();

        assert_eq!(caps.color_depth, ColorDepth::Monochrome);
        assert!(!caps.unicode_support);
        assert!(!caps.sixel_support);
        assert_eq!(caps.mouse_protocol, MouseProtocol::None);
    }

    #[test]
    fn test_wezterm_profile() {
        let profile = TerminalProfile::WezTerm;
        let caps = profile.capabilities();

        assert_eq!(caps.color_depth, ColorDepth::TrueColor);
        assert!(caps.unicode_support);
        assert!(caps.wide_char_support);
        assert!(caps.sixel_support);
        assert_eq!(caps.mouse_protocol, MouseProtocol::SGR);
    }

    #[test]
    fn test_feature_checking() {
        let wezterm = TerminalProfile::WezTerm;
        assert!(wezterm.supports(Feature::Sixel));
        assert!(wezterm.supports(Feature::TrueColor));
        assert!(wezterm.supports(Feature::Unicode));

        let vt100 = TerminalProfile::VT100;
        assert!(!vt100.supports(Feature::Sixel));
        assert!(!vt100.supports(Feature::TrueColor));
        assert!(!vt100.supports(Feature::Unicode));
    }

    #[test]
    fn test_profile_from_name() {
        assert_eq!(TerminalProfile::from_name("wezterm"), Some(TerminalProfile::WezTerm));
        assert_eq!(TerminalProfile::from_name("WEZTERM"), Some(TerminalProfile::WezTerm));
        assert_eq!(TerminalProfile::from_name("xterm-256color"), Some(TerminalProfile::Xterm256));
        assert_eq!(TerminalProfile::from_name("unknown"), None);
    }

    #[test]
    fn test_term_name() {
        assert_eq!(TerminalProfile::WezTerm.term_name(), "wezterm");
        assert_eq!(TerminalProfile::Xterm256.term_name(), "xterm-256color");
        assert_eq!(TerminalProfile::VT100.term_name(), "vt100");
    }

    #[test]
    fn test_color_depth_ordering() {
        assert!(ColorDepth::TrueColor > ColorDepth::Colors256);
        assert!(ColorDepth::Colors256 > ColorDepth::Colors16);
        assert!(ColorDepth::Colors16 > ColorDepth::Colors8);
        assert!(ColorDepth::Colors8 > ColorDepth::Monochrome);
    }

    #[test]
    fn test_capabilities_supports() {
        let caps = TerminalCapabilities {
            color_depth: ColorDepth::TrueColor,
            sixel_support: true,
            mouse_protocol: MouseProtocol::SGR,
            ..Default::default()
        };

        assert!(caps.supports(Feature::TrueColor));
        assert!(caps.supports(Feature::Colors256));
        assert!(caps.supports(Feature::Sixel));
        assert!(caps.supports(Feature::MouseSGR));
    }

    #[test]
    fn test_all_profiles() {
        let profiles = TerminalProfile::all();
        assert!(profiles.len() >= 15);
        assert!(profiles.contains(&TerminalProfile::WezTerm));
        assert!(profiles.contains(&TerminalProfile::VT100));
    }

    #[test]
    fn test_default_profile() {
        let profile = TerminalProfile::default();
        assert_eq!(profile, TerminalProfile::Xterm256);
    }

    #[test]
    fn test_capabilities_summary() {
        let caps = TerminalProfile::WezTerm.capabilities();
        let summary = caps.summary();

        assert!(summary.contains("wezterm"));
        assert!(summary.contains("TrueColor"));
        assert!(summary.contains("Sixel: true"));
    }

    #[test]
    fn test_display_name() {
        assert_eq!(TerminalProfile::WezTerm.display_name(), "WezTerm");
        assert_eq!(TerminalProfile::VT100.display_name(), "VT100");
    }

    #[test]
    fn test_mouse_protocol_hierarchy() {
        let caps_none = TerminalCapabilities {
            mouse_protocol: MouseProtocol::None,
            ..Default::default()
        };
        assert!(!caps_none.supports(Feature::MouseX10));

        let caps_x10 = TerminalCapabilities {
            mouse_protocol: MouseProtocol::X10,
            ..Default::default()
        };
        assert!(caps_x10.supports(Feature::MouseX10));
        assert!(!caps_x10.supports(Feature::MouseVT200));

        let caps_sgr = TerminalCapabilities {
            mouse_protocol: MouseProtocol::SGR,
            ..Default::default()
        };
        assert!(caps_sgr.supports(Feature::MouseX10));
        assert!(caps_sgr.supports(Feature::MouseVT200));
        assert!(caps_sgr.supports(Feature::MouseSGR));
    }

    #[test]
    fn test_kitty_graphics() {
        let kitty = TerminalProfile::Kitty;
        assert!(kitty.supports(Feature::KittyGraphics));
        assert!(!kitty.supports(Feature::Sixel));

        let wezterm = TerminalProfile::WezTerm;
        assert!(!wezterm.supports(Feature::KittyGraphics));
        assert!(wezterm.supports(Feature::Sixel));
    }

    #[test]
    fn test_minimal_vs_maximum() {
        let minimal = TerminalProfile::Minimal;
        let maximum = TerminalProfile::Maximum;

        // Minimal should support very few features
        assert!(!minimal.supports(Feature::Sixel));
        assert!(!minimal.supports(Feature::TrueColor));
        assert!(!minimal.supports(Feature::Unicode));

        // Maximum should support all features
        assert!(maximum.supports(Feature::Sixel));
        assert!(maximum.supports(Feature::TrueColor));
        assert!(maximum.supports(Feature::Unicode));
        assert!(maximum.supports(Feature::KittyGraphics));
        assert!(maximum.supports(Feature::ITerm2Images));
    }
}
