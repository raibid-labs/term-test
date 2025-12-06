//! Integration tests for terminal profile functionality.

use ratatui_testlib::{
    ColorDepth, Feature, MouseProtocol, TerminalCapabilities, TerminalProfile, TuiTestHarness,
};

#[test]
fn test_default_profile() {
    let harness = TuiTestHarness::new(80, 24).unwrap();
    assert_eq!(harness.terminal_profile(), TerminalProfile::default());
    assert_eq!(harness.terminal_profile(), TerminalProfile::Xterm256);
}

#[test]
fn test_with_terminal_profile() {
    let harness = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::WezTerm);

    assert_eq!(harness.terminal_profile(), TerminalProfile::WezTerm);
}

#[test]
fn test_simulate_terminfo() {
    let harness = TuiTestHarness::new(80, 24)
        .unwrap()
        .simulate_terminfo("wezterm");

    assert_eq!(harness.terminal_profile(), TerminalProfile::WezTerm);
}

#[test]
fn test_simulate_terminfo_case_insensitive() {
    let harness = TuiTestHarness::new(80, 24)
        .unwrap()
        .simulate_terminfo("WEZTERM");

    assert_eq!(harness.terminal_profile(), TerminalProfile::WezTerm);
}

#[test]
fn test_simulate_terminfo_by_term_value() {
    let harness = TuiTestHarness::new(80, 24)
        .unwrap()
        .simulate_terminfo("xterm-256color");

    assert_eq!(harness.terminal_profile(), TerminalProfile::Xterm256);
}

#[test]
fn test_simulate_terminfo_unknown_uses_current() {
    let harness = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::WezTerm)
        .simulate_terminfo("unknown-terminal");

    // Should keep the WezTerm profile since "unknown-terminal" doesn't match
    assert_eq!(harness.terminal_profile(), TerminalProfile::WezTerm);
}

#[test]
fn test_supports_feature_sixel() {
    let wezterm = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::WezTerm);
    assert!(wezterm.supports_feature(Feature::Sixel));

    let alacritty = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::Alacritty);
    assert!(!alacritty.supports_feature(Feature::Sixel));
}

#[test]
fn test_supports_feature_true_color() {
    let modern = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::WezTerm);
    assert!(modern.supports_feature(Feature::TrueColor));

    let vt100 = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::VT100);
    assert!(!vt100.supports_feature(Feature::TrueColor));
}

#[test]
fn test_supports_feature_unicode() {
    let modern = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::WezTerm);
    assert!(modern.supports_feature(Feature::Unicode));

    let vt100 = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::VT100);
    assert!(!vt100.supports_feature(Feature::Unicode));
}

#[test]
fn test_terminal_capabilities() {
    let harness = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::WezTerm);

    let caps = harness.terminal_capabilities();
    assert_eq!(caps.color_depth, ColorDepth::TrueColor);
    assert!(caps.unicode_support);
    assert!(caps.wide_char_support);
    assert!(caps.sixel_support);
    assert_eq!(caps.mouse_protocol, MouseProtocol::SGR);
    assert_eq!(caps.term_name, "wezterm");
}

#[test]
fn test_vt100_capabilities() {
    let harness = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::VT100);

    let caps = harness.terminal_capabilities();
    assert_eq!(caps.color_depth, ColorDepth::Monochrome);
    assert!(!caps.unicode_support);
    assert!(!caps.wide_char_support);
    assert!(!caps.sixel_support);
    assert_eq!(caps.mouse_protocol, MouseProtocol::None);
}

#[test]
fn test_kitty_graphics_protocol() {
    let kitty = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::Kitty);
    assert!(kitty.supports_feature(Feature::KittyGraphics));
    assert!(!kitty.supports_feature(Feature::Sixel));

    let wezterm = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::WezTerm);
    assert!(!wezterm.supports_feature(Feature::KittyGraphics));
    assert!(wezterm.supports_feature(Feature::Sixel));
}

#[test]
fn test_iterm2_images() {
    let iterm2 = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::ITerm2);
    assert!(iterm2.supports_feature(Feature::ITerm2Images));
    assert!(iterm2.supports_feature(Feature::Sixel));

    let alacritty = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::Alacritty);
    assert!(!alacritty.supports_feature(Feature::ITerm2Images));
}

#[test]
fn test_mouse_protocol_hierarchy() {
    let vt100 = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::VT100);
    assert!(!vt100.supports_feature(Feature::MouseX10));

    let screen = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::Screen);
    assert!(screen.supports_feature(Feature::MouseX10));
    assert!(!screen.supports_feature(Feature::MouseSGR));

    let wezterm = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::WezTerm);
    assert!(wezterm.supports_feature(Feature::MouseX10));
    assert!(wezterm.supports_feature(Feature::MouseVT200));
    assert!(wezterm.supports_feature(Feature::MouseSGR));
}

#[test]
fn test_color_depth_hierarchy() {
    let caps_256 = TerminalCapabilities {
        color_depth: ColorDepth::Colors256,
        ..Default::default()
    };
    assert!(caps_256.supports(Feature::Colors256));
    assert!(!caps_256.supports(Feature::TrueColor));

    let caps_true = TerminalCapabilities {
        color_depth: ColorDepth::TrueColor,
        ..Default::default()
    };
    assert!(caps_true.supports(Feature::Colors256));
    assert!(caps_true.supports(Feature::TrueColor));
}

#[test]
fn test_minimal_profile() {
    let minimal = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::Minimal);

    assert!(!minimal.supports_feature(Feature::Sixel));
    assert!(!minimal.supports_feature(Feature::TrueColor));
    assert!(!minimal.supports_feature(Feature::Unicode));
    assert!(!minimal.supports_feature(Feature::WideCharacters));
    assert!(!minimal.supports_feature(Feature::MouseX10));

    let caps = minimal.terminal_capabilities();
    assert_eq!(caps.color_depth, ColorDepth::Colors16);
}

#[test]
fn test_maximum_profile() {
    let maximum = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::Maximum);

    assert!(maximum.supports_feature(Feature::Sixel));
    assert!(maximum.supports_feature(Feature::TrueColor));
    assert!(maximum.supports_feature(Feature::Unicode));
    assert!(maximum.supports_feature(Feature::WideCharacters));
    assert!(maximum.supports_feature(Feature::MouseSGR));
    assert!(maximum.supports_feature(Feature::KittyGraphics));
    assert!(maximum.supports_feature(Feature::ITerm2Images));
}

#[test]
fn test_builder_with_terminal_profile() {
    let harness = TuiTestHarness::builder()
        .with_size(100, 30)
        .with_terminal_profile(TerminalProfile::WezTerm)
        .build()
        .unwrap();

    assert_eq!(harness.terminal_profile(), TerminalProfile::WezTerm);
    assert!(harness.supports_feature(Feature::Sixel));
}

#[test]
fn test_all_profiles_defined() {
    let profiles = TerminalProfile::all();
    assert!(profiles.len() >= 15);
    assert!(profiles.contains(&TerminalProfile::VT100));
    assert!(profiles.contains(&TerminalProfile::Xterm256));
    assert!(profiles.contains(&TerminalProfile::WezTerm));
    assert!(profiles.contains(&TerminalProfile::Alacritty));
    assert!(profiles.contains(&TerminalProfile::Kitty));
}

#[test]
fn test_profile_display_names() {
    assert_eq!(TerminalProfile::VT100.display_name(), "VT100");
    assert_eq!(TerminalProfile::WezTerm.display_name(), "WezTerm");
    assert_eq!(TerminalProfile::Alacritty.display_name(), "Alacritty");
}

#[test]
fn test_profile_term_names() {
    assert_eq!(TerminalProfile::VT100.term_name(), "vt100");
    assert_eq!(TerminalProfile::WezTerm.term_name(), "wezterm");
    assert_eq!(TerminalProfile::Xterm256.term_name(), "xterm-256color");
}

#[test]
fn test_capabilities_summary() {
    let caps = TerminalProfile::WezTerm.capabilities();
    let summary = caps.summary();

    assert!(summary.contains("wezterm"));
    assert!(summary.contains("TrueColor"));
    assert!(summary.contains("Unicode: true"));
    assert!(summary.contains("Sixel: true"));
}

#[test]
fn test_synchronized_output_support() {
    let alacritty = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::Alacritty);
    assert!(alacritty.supports_feature(Feature::SynchronizedOutput));

    let screen = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::Screen);
    assert!(!screen.supports_feature(Feature::SynchronizedOutput));
}

#[test]
fn test_bracketed_paste_support() {
    let modern = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::WezTerm);
    assert!(modern.supports_feature(Feature::BracketedPaste));

    let vt100 = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::VT100);
    assert!(!vt100.supports_feature(Feature::BracketedPaste));
}

#[test]
fn test_wide_character_support() {
    let modern = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::WezTerm);
    assert!(modern.supports_feature(Feature::WideCharacters));

    let xterm256 = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::Xterm256);
    assert!(!xterm256.supports_feature(Feature::WideCharacters));

    let xterm_true = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::XtermTrueColor);
    assert!(xterm_true.supports_feature(Feature::WideCharacters));
}

#[test]
fn test_tmux_profile() {
    let tmux = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::Tmux);

    let caps = tmux.terminal_capabilities();
    assert_eq!(caps.color_depth, ColorDepth::Colors256);
    assert!(caps.unicode_support);
    assert!(caps.wide_char_support);
    assert!(!caps.sixel_support);
    assert_eq!(caps.mouse_protocol, MouseProtocol::SGR);
}

#[test]
fn test_konsole_profile() {
    let konsole = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::Konsole);

    let caps = konsole.terminal_capabilities();
    assert_eq!(caps.color_depth, ColorDepth::TrueColor);
    assert!(!caps.sixel_support);
    assert!(caps.bracketed_paste);
}

#[test]
fn test_windows_terminal_profile() {
    let wt = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::WindowsTerminal);

    let caps = wt.terminal_capabilities();
    assert_eq!(caps.color_depth, ColorDepth::TrueColor);
    assert!(!caps.sixel_support);
    assert!(caps.synchronized_output);
}

#[test]
fn test_vscode_terminal_profile() {
    let vscode = TuiTestHarness::new(80, 24)
        .unwrap()
        .with_terminal_profile(TerminalProfile::VSCode);

    let caps = vscode.terminal_capabilities();
    assert_eq!(caps.color_depth, ColorDepth::TrueColor);
    assert!(!caps.sixel_support);
    assert!(!caps.focus_events);
}

#[test]
fn test_profile_from_name_variants() {
    // Test various name formats
    assert_eq!(TerminalProfile::from_name("tmux-256color"), Some(TerminalProfile::Tmux));
    assert_eq!(TerminalProfile::from_name("xterm-kitty"), Some(TerminalProfile::Kitty));
    assert_eq!(TerminalProfile::from_name("konsole-256color"), Some(TerminalProfile::Konsole));
    assert_eq!(TerminalProfile::from_name("max"), Some(TerminalProfile::Maximum));
}

#[test]
fn test_capabilities_custom_fields() {
    let mut caps = TerminalProfile::WezTerm.capabilities();
    caps.custom
        .insert("test_key".to_string(), "test_value".to_string());

    assert_eq!(caps.custom.get("test_key").unwrap(), "test_value");
}

#[test]
fn test_multiple_profile_switches() {
    let mut harness = TuiTestHarness::new(80, 24).unwrap();

    // Start with default
    assert_eq!(harness.terminal_profile(), TerminalProfile::Xterm256);

    // Switch to WezTerm
    harness = harness.with_terminal_profile(TerminalProfile::WezTerm);
    assert!(harness.supports_feature(Feature::Sixel));

    // Switch to VT100
    harness = harness.with_terminal_profile(TerminalProfile::VT100);
    assert!(!harness.supports_feature(Feature::Sixel));
    assert!(!harness.supports_feature(Feature::TrueColor));
}

#[test]
fn test_feature_checking_without_harness() {
    // Test direct profile feature checking
    assert!(TerminalProfile::WezTerm.supports(Feature::Sixel));
    assert!(!TerminalProfile::Alacritty.supports(Feature::Sixel));

    // Test capabilities feature checking
    let caps = TerminalProfile::WezTerm.capabilities();
    assert!(caps.supports(Feature::Sixel));
    assert!(caps.supports(Feature::TrueColor));
}

#[test]
fn test_color_depth_comparison() {
    assert!(ColorDepth::TrueColor > ColorDepth::Colors256);
    assert!(ColorDepth::Colors256 > ColorDepth::Colors16);
    assert!(ColorDepth::Colors16 > ColorDepth::Colors8);
    assert!(ColorDepth::Colors8 > ColorDepth::Monochrome);
}

#[test]
fn test_all_terminal_features_enum() {
    // Ensure all Feature variants are handled in capabilities.supports()
    let caps = TerminalProfile::Maximum.capabilities();

    // Test each feature variant compiles and works
    let _sixel = caps.supports(Feature::Sixel);
    let _iterm2 = caps.supports(Feature::ITerm2Images);
    let _kitty = caps.supports(Feature::KittyGraphics);
    let _colors256 = caps.supports(Feature::Colors256);
    let _true_color = caps.supports(Feature::TrueColor);
    let _unicode = caps.supports(Feature::Unicode);
    let _wide = caps.supports(Feature::WideCharacters);
    let _mouse_x10 = caps.supports(Feature::MouseX10);
    let _mouse_vt200 = caps.supports(Feature::MouseVT200);
    let _mouse_sgr = caps.supports(Feature::MouseSGR);
    let _mouse_utf8 = caps.supports(Feature::MouseUTF8);
    let _mouse_motion = caps.supports(Feature::MouseMotion);
    let _bracketed = caps.supports(Feature::BracketedPaste);
    let _sync = caps.supports(Feature::SynchronizedOutput);
    let _alt_screen = caps.supports(Feature::AlternateScreen);
    let _title = caps.supports(Feature::SetTitle);
    let _focus = caps.supports(Feature::FocusEvents);
}
