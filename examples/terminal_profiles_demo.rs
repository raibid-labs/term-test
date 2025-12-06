//! Example demonstrating terminal profile functionality.
//!
//! This example shows how to use terminal profiles to test your TUI application
//! across different terminal emulators.
//!
//! Run with:
//! ```bash
//! cargo run --example terminal_profiles_demo --features sixel
//! ```

use ratatui_testlib::{ColorDepth, Feature, TerminalProfile, TuiTestHarness};

fn main() -> ratatui_testlib::Result<()> {
    println!("Terminal Profile Demo\n");
    println!("===================\n");

    // Example 1: Create harness with specific terminal profile
    println!("Example 1: Testing with WezTerm profile");
    println!("----------------------------------------");
    let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::WezTerm);

    println!("Terminal: {}", harness.terminal_profile().display_name());
    println!("TERM value: {}", harness.terminal_profile().term_name());

    // Check feature support
    println!("\nFeature Support:");
    println!("  Sixel: {}", harness.supports_feature(Feature::Sixel));
    println!("  True Color: {}", harness.supports_feature(Feature::TrueColor));
    println!("  Unicode: {}", harness.supports_feature(Feature::Unicode));
    println!("  Wide Characters: {}", harness.supports_feature(Feature::WideCharacters));
    println!("  Mouse SGR: {}", harness.supports_feature(Feature::MouseSGR));

    // Get full capabilities
    let caps = harness.terminal_capabilities();
    println!("\nFull Capabilities:");
    println!("{}", caps.summary());

    // Example 2: Test across multiple profiles
    println!("\n\nExample 2: Testing across multiple terminal profiles");
    println!("----------------------------------------------------");

    let profiles = vec![
        TerminalProfile::VT100,
        TerminalProfile::Xterm256,
        TerminalProfile::Alacritty,
        TerminalProfile::Kitty,
        TerminalProfile::WezTerm,
    ];

    println!(
        "{:<20} | {:>10} | {:>10} | {:>10} | {:>10}",
        "Terminal", "Sixel", "TrueColor", "Mouse", "Unicode"
    );
    println!("{:-<20}-+-{:-<10}-+-{:-<10}-+-{:-<10}-+-{:-<10}", "", "", "", "", "");

    for profile in profiles {
        let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(profile);
        println!(
            "{:<20} | {:>10} | {:>10} | {:>10} | {:>10}",
            profile.display_name(),
            harness.supports_feature(Feature::Sixel),
            harness.supports_feature(Feature::TrueColor),
            harness.supports_feature(Feature::MouseSGR),
            harness.supports_feature(Feature::Unicode),
        );
    }

    // Example 3: Using simulate_terminfo
    println!("\n\nExample 3: Simulating TERMINFO values");
    println!("-------------------------------------");

    let term_values = vec!["xterm-256color", "wezterm", "tmux-256color", "alacritty"];

    for term in term_values {
        let harness = TuiTestHarness::new(80, 24)?.simulate_terminfo(term);
        let caps = harness.terminal_capabilities();
        println!(
            "TERM={:<20} -> {} ({:?})",
            term,
            harness.terminal_profile().display_name(),
            caps.color_depth
        );
    }

    // Example 4: Conditional testing based on features
    println!("\n\nExample 4: Conditional testing based on features");
    println!("------------------------------------------------");

    let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::WezTerm);

    if harness.supports_feature(Feature::Sixel) {
        println!("✓ Sixel is supported - would run Sixel graphics tests");
    } else {
        println!("✗ Sixel not supported - skipping Sixel tests");
    }

    if harness.supports_feature(Feature::TrueColor) {
        println!("✓ True color is supported - would test 24-bit colors");
    } else {
        println!("✗ True color not supported - using 256 colors");
    }

    // Example 5: Testing graphics protocol differences
    println!("\n\nExample 5: Graphics protocol comparison");
    println!("---------------------------------------");

    let graphics_terminals = vec![
        ("WezTerm", TerminalProfile::WezTerm),
        ("Kitty", TerminalProfile::Kitty),
        ("iTerm2", TerminalProfile::ITerm2),
        ("Alacritty", TerminalProfile::Alacritty),
    ];

    println!("{:<15} | {:>10} | {:>10} | {:>10}", "Terminal", "Sixel", "Kitty", "iTerm2");
    println!("{:-<15}-+-{:-<10}-+-{:-<10}-+-{:-<10}", "", "", "", "");

    for (name, profile) in graphics_terminals {
        let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(profile);
        println!(
            "{:<15} | {:>10} | {:>10} | {:>10}",
            name,
            harness.supports_feature(Feature::Sixel),
            harness.supports_feature(Feature::KittyGraphics),
            harness.supports_feature(Feature::ITerm2Images),
        );
    }

    // Example 6: Color depth testing
    println!("\n\nExample 6: Color depth comparison");
    println!("---------------------------------");

    let color_terminals = vec![
        ("VT100", TerminalProfile::VT100),
        ("xterm-256", TerminalProfile::Xterm256),
        ("Alacritty", TerminalProfile::Alacritty),
        ("WezTerm", TerminalProfile::WezTerm),
    ];

    for (name, profile) in color_terminals {
        let caps = profile.capabilities();
        println!("{:<15} -> {:?}", name, caps.color_depth);

        // Check what color features are supported
        let mut features = vec![];
        if caps.supports(Feature::Colors256) {
            features.push("256");
        }
        if caps.supports(Feature::TrueColor) {
            features.push("TrueColor");
        }
        if !features.is_empty() {
            println!("                   Supports: {}", features.join(", "));
        }
    }

    // Example 7: Mouse protocol comparison
    println!("\n\nExample 7: Mouse protocol comparison");
    println!("------------------------------------");

    let mouse_terminals = vec![
        ("VT100", TerminalProfile::VT100),
        ("GNU Screen", TerminalProfile::Screen),
        ("xterm-256", TerminalProfile::Xterm256),
        ("WezTerm", TerminalProfile::WezTerm),
    ];

    println!("{:<15} | {:>10} | {:>10} | {:>10}", "Terminal", "Basic", "VT200", "SGR");
    println!("{:-<15}-+-{:-<10}-+-{:-<10}-+-{:-<10}", "", "", "", "");

    for (name, profile) in mouse_terminals {
        let harness = TuiTestHarness::new(80, 24)?.with_terminal_profile(profile);
        println!(
            "{:<15} | {:>10} | {:>10} | {:>10}",
            name,
            harness.supports_feature(Feature::MouseX10),
            harness.supports_feature(Feature::MouseVT200),
            harness.supports_feature(Feature::MouseSGR),
        );
    }

    // Example 8: Using builder with profile
    println!("\n\nExample 8: Using builder pattern with profiles");
    println!("----------------------------------------------");

    let harness = TuiTestHarness::builder()
        .with_size(100, 30)
        .with_terminal_profile(TerminalProfile::WezTerm)
        .build()?;

    println!(
        "Created {}x{} terminal with {} profile",
        100,
        30,
        harness.terminal_profile().display_name()
    );

    // Example 9: Minimal vs Maximum profiles
    println!("\n\nExample 9: Minimal vs Maximum testing profiles");
    println!("----------------------------------------------");

    let minimal = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::Minimal);
    let maximum = TuiTestHarness::new(80, 24)?.with_terminal_profile(TerminalProfile::Maximum);

    println!("Minimal profile features:");
    println!("  Sixel: {}", minimal.supports_feature(Feature::Sixel));
    println!("  TrueColor: {}", minimal.supports_feature(Feature::TrueColor));
    println!("  Unicode: {}", minimal.supports_feature(Feature::Unicode));

    println!("\nMaximum profile features:");
    println!("  Sixel: {}", maximum.supports_feature(Feature::Sixel));
    println!("  TrueColor: {}", maximum.supports_feature(Feature::TrueColor));
    println!("  Unicode: {}", maximum.supports_feature(Feature::Unicode));
    println!("  Kitty: {}", maximum.supports_feature(Feature::KittyGraphics));
    println!("  iTerm2: {}", maximum.supports_feature(Feature::ITerm2Images));

    // Example 10: All available profiles
    println!("\n\nExample 10: All available terminal profiles");
    println!("-------------------------------------------");

    let all_profiles = TerminalProfile::all();
    println!("Total profiles available: {}", all_profiles.len());
    println!("\nProfiles:");
    for profile in all_profiles {
        let caps = profile.capabilities();
        println!("  - {:<20} (TERM={})", profile.display_name(), caps.term_name);
    }

    println!("\n\nDemo complete!");
    println!("\nKey Takeaways:");
    println!("1. Use .with_terminal_profile() to configure a specific terminal");
    println!("2. Use .supports_feature() to check capabilities");
    println!("3. Use .terminal_capabilities() for detailed info");
    println!("4. Use .simulate_terminfo() for TERM-based selection");
    println!("5. Test across multiple profiles to ensure compatibility");

    Ok(())
}
