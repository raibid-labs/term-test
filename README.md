# mimic

> A Rust library for integration testing of terminal user interface (TUI) applications with first-class support for Ratatui, Bevy ECS integration, and Sixel graphics protocols.

## Overview

`mimic` bridges the gap between unit testing with Ratatui's `TestBackend` and real-world integration testing of TUI applications. It provides a PTY-based test harness that enables testing of features requiring actual terminal escape sequence processing, including **Sixel graphics position verification**, **Bevy ECS integration**, **bevy_ratatui support**, and complex user interaction flows.

### MVP Goal

Built to enable comprehensive integration testing for the [**dgx-pixels**](https://github.com/raibid-labs/dgx-pixels) project - a Bevy-based TUI application with Sixel graphics support.

### Why mimic?

**Current Limitation**: Ratatui's `TestBackend` is great for unit testing widgets and layouts, but it can't test:
- PTY-specific behavior (terminal size negotiation, TTY detection)
- Graphics protocols (Sixel, iTerm2 images, Kitty graphics)
- Real terminal integration
- User interaction flows
- Event handling in actual terminal context

**Solution**: `mimic` runs your TUI application in a real pseudo-terminal (PTY), captures the output using a terminal emulator, and provides an ergonomic API for assertions and snapshot testing.

### Key Features

**MVP (v0.1.0)**:
- ✅ **PTY-Based Testing**: Real terminal environment using `portable-pty`
- ✅ **Sixel Position Tracking**: Verify graphics render at correct coordinates and within bounds
- ✅ **Bevy ECS Integration**: Query entities, control update cycles, test Bevy systems
- ✅ **bevy_ratatui Support**: First-class integration with bevy_ratatui plugin
- ✅ **Event Simulation**: Keyboard events for navigation and input
- ✅ **Smart Waiting**: Condition-based waiting with timeouts
- ✅ **Snapshot Testing**: Integration with `insta`
- ✅ **Tokio Async Support**: Test async TUI apps
- ✅ **High-Level Assertions**: Ergonomic API (text_at, cursor_position, sixel_within, etc.)
- ✅ **CI/CD Ready**: Headless testing without X11/Wayland

**Post-MVP**:
- Mouse and resize events
- expect-test integration
- async-std support
- Cross-platform (macOS, Windows)
- Visual Sixel comparison

## Status

**🚧 Research & Design Phase Complete → Implementation Starting**

**MVP Target**: v0.1.0 in 3-4 months (Phases 1-6)
**Primary Use Case**: dgx-pixels integration testing

See [ROADMAP.md](./docs/ROADMAP.md) for detailed implementation plan and [DGX_PIXELS_REQUIREMENTS.md](./docs/DGX_PIXELS_REQUIREMENTS.md) for MVP requirements analysis.

## Quick Example

```rust
use mimic::TuiTestHarness;
use std::process::Command;

#[test]
fn test_navigation() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Spawn your TUI app
    harness.spawn(Command::new("./my-tui-app"))?;

    // Wait for initial render
    harness.wait_for(|state| {
        state.contents().contains("Main Menu")
    })?;

    // Simulate user input
    harness.send_key(Key::Down)?;
    harness.send_key(Key::Enter)?;

    // Verify result
    harness.wait_for(|state| {
        state.contents().contains("Settings")
    })?;

    Ok(())
}
```

## Testing Sixel Graphics (MVP Use Case)

```rust
use mimic::BevyTuiTestHarness;

#[tokio::test]
async fn test_sixel_renders_in_preview_area() -> Result<()> {
    let mut test = BevyTuiTestHarness::with_bevy_ratatui()?;

    // Load test image and navigate to Gallery
    test.load_test_image("tests/fixtures/test-sprite.png")?;
    test.press_key(KeyCode::Char('2'))?;  // Gallery screen
    test.update()?;
    test.render_frame()?;

    // Get preview area from Bevy component
    let preview_panel = test.query::<PreviewPanel>().first().unwrap();
    let preview_area = preview_panel.area;

    // Assert: Sixel graphics within bounds
    test.assert_sixel_within(preview_area)?;
    test.assert_no_sixel_outside(preview_area)?;

    Ok(())
}

#[tokio::test]
async fn test_sixel_clears_on_screen_change() -> Result<()> {
    let mut test = BevyTuiTestHarness::with_bevy_ratatui()?;

    // Render image on Gallery screen
    test.load_test_image("tests/fixtures/test-sprite.png")?;
    test.press_key(KeyCode::Char('2'))?;
    test.update()?;
    test.render_frame()?;

    assert!(test.has_sixel_graphics());

    // Navigate away
    test.press_key(KeyCode::Char('1'))?;  // Generation screen
    test.update()?;
    test.render_frame()?;

    // Assert: Sixel cleared
    assert!(!test.has_sixel_graphics());

    Ok(())
}
```

## Documentation

### 📚 Core Documentation

- **[RESEARCH.md](./docs/RESEARCH.md)** - Comprehensive research on existing terminal testing solutions, parsing libraries (VTE, vt100, termwiz), PTY libraries (portable-pty), snapshot testing frameworks (insta, expect-test), and Sixel testing approaches. Essential background for understanding the problem space.

- **[ARCHITECTURE.md](./docs/ARCHITECTURE.md)** - Complete library architecture including:
  - Layer design (PTY management, terminal emulation, test harness, snapshot integration, Ratatui helpers)
  - Module structure and API design
  - Example usage patterns
  - Dependencies and feature flags
  - Error handling strategy
  - Performance considerations

- **[EXISTING_SOLUTIONS.md](./docs/EXISTING_SOLUTIONS.md)** - Analysis of existing Ratatui testing approaches:
  - Ratatui's TestBackend (unit testing)
  - Snapshot testing with insta/expect-test
  - term-transcript (CLI testing)
  - tui-term (pseudoterminal widget)
  - Comparison matrix showing gaps that mimic fills

- **[TESTING_APPROACHES.md](./docs/TESTING_APPROACHES.md)** - Comprehensive guide to TUI testing methodologies:
  - The testing pyramid for TUI applications
  - Unit testing vs integration testing vs E2E testing
  - Snapshot testing patterns
  - PTY-based testing strategies
  - Sixel/graphics testing
  - Async/event-driven testing
  - Property-based testing for TUIs
  - Testing strategy recommendations for different application types

- **[ROADMAP.md](./docs/ROADMAP.md)** - **Updated for dgx-pixels MVP**:
  - MVP definition (v0.1.0 in 3-4 months)
  - 6 MVP phases + 2 post-MVP phases
  - Focus on Bevy integration and Sixel position tracking
  - dgx-pixels integration checklist
  - Version planning and timeline estimates
  - Risk mitigation for critical features

- **[DGX_PIXELS_REQUIREMENTS.md](./docs/DGX_PIXELS_REQUIREMENTS.md)** - **MVP Requirements Analysis**:
  - Gap analysis from GitHub Issue #1
  - Detailed use case mapping
  - Sixel position tracking requirements
  - Bevy ECS integration design
  - API comparison and enhancements needed
  - Implementation priority adjustments

### 🎯 Quick Navigation

| Topic | Document | Key Sections |
|-------|----------|--------------|
| **MVP Requirements** | DGX_PIXELS_REQUIREMENTS.md | Use Cases, Gap Analysis, Roadmap Adjustments |
| **Implementation Plan** | ROADMAP.md | MVP Phases 1-6, Timeline, dgx-pixels Checklist |
| **API Design** | ARCHITECTURE.md | Bevy Integration, Sixel Position Tracking |
| **Understand the Problem** | EXISTING_SOLUTIONS.md | Gap Analysis, Comparison Matrix |
| **Testing Strategies** | TESTING_APPROACHES.md | Testing Pyramid, Common Patterns |
| **Technical Research** | RESEARCH.md | VTE vs vt100, PTY Libraries, Sixel Testing |

## How mimic Complements Existing Tools

| Testing Level | Use This | For What |
|---------------|----------|----------|
| **Unit Tests** | Ratatui's TestBackend + insta | Individual widgets, layout calculations |
| **Integration Tests** | **mimic** | Full app behavior, PTY interaction, graphics |
| **CLI Tests** | assert_cmd | Binary execution, exit codes |
| **Snapshot Tests** | insta or expect-test | Both unit and integration levels |

`mimic` is **complementary, not competitive** - it fills the integration testing gap that TestBackend cannot address.

## Project Goals

1. **Ease of Use**: Simple API that gets out of your way
2. **Comprehensive**: Test all terminal features including graphics
3. **Cross-Platform**: Reliable on Linux, macOS, and Windows
4. **Well-Documented**: Examples for every use case
5. **Battle-Tested**: High test coverage and production-ready

## Comparison with Ratatui's TestBackend

| Feature | TestBackend | mimic |
|---------|-------------|-----------|
| **Speed** | Very Fast | Moderate |
| **Setup Complexity** | Simple | Moderate |
| **PTY Testing** | ❌ | ✅ |
| **Graphics (Sixel)** | ❌ | ✅ |
| **Widget Unit Tests** | ✅ | ✅ |
| **Integration Tests** | ❌ | ✅ |
| **Event Simulation** | Limited | Full |
| **Async Support** | Basic | Full |
| **Snapshot Testing** | Via insta/expect | Built-in |

**Recommendation**: Use TestBackend for unit tests, mimic for integration tests.

## Architecture Highlights

### Layer Structure

```
┌─────────────────────────────────────────┐
│   Ratatui Integration Helpers (Layer 5) │ Widget assertions, layout verification
├─────────────────────────────────────────┤
│   Snapshot Testing (Layer 4)            │ insta/expect-test integration
├─────────────────────────────────────────┤
│   Test Harness (Layer 3)                │ TuiTestHarness, event simulation
├─────────────────────────────────────────┤
│   Terminal Emulation (Layer 2)          │ vt100 parser, screen state
├─────────────────────────────────────────┤
│   PTY Management (Layer 1)              │ portable-pty wrapper
└─────────────────────────────────────────┘
```

### Core Dependencies

- **portable-pty**: Cross-platform PTY creation (from WezTerm)
- **vt100**: Terminal emulation and escape sequence parsing
- **insta/expect-test**: Snapshot testing (optional)
- **tokio/async-std**: Async runtime support (optional)

See [ARCHITECTURE.md](./docs/ARCHITECTURE.md) for complete details.

## Roadmap Summary

### MVP (v0.1.0) - 3-4 months

**Target**: Enable dgx-pixels integration testing

- **Phase 1**: Core PTY + Cursor Tracking (2-3 weeks)
- **Phase 2**: Events + Tokio Async (1-2 weeks)
- **Phase 3**: Sixel Position Tracking ⭐ (2-3 weeks)
- **Phase 4**: Bevy ECS Integration ⭐ (2-3 weeks)
- **Phase 5**: Snapshots + Assertions (1-2 weeks)
- **Phase 6**: Polish + Docs (2-3 weeks)

**Success Criteria**: dgx-pixels can detect and prevent Sixel positioning/persistence bugs

### Post-MVP

- **v0.2.0** - Enhanced features (mouse, resize, async-std)
- **v0.3.0** - Cross-platform (macOS, Windows)
- **v1.0.0** - Production ready, stable API

See [ROADMAP.md](./docs/ROADMAP.md) for the complete implementation plan.

## Contributing

**This project is in the design phase.** Feedback on the architecture and approach is welcome!

Once implementation begins:
- Check the [ROADMAP.md](./docs/ROADMAP.md) for current phase
- Look for "good first issue" labels
- Read CONTRIBUTING.md (to be created)

## Research Acknowledgments

This project builds on excellent work from:
- **Ratatui** - The TUI framework we're testing
- **WezTerm** - Source of portable-pty and termwiz
- **Alacritty** - Source of VTE parser
- **vt100-rust** - Terminal emulation library
- **insta** - Snapshot testing framework
- The broader Rust TUI ecosystem

Special thanks to the maintainers of these projects for their well-documented, reusable components.

## Related Projects

- [ratatui](https://github.com/ratatui/ratatui) - Rust library for cooking up TUIs
- [WezTerm](https://github.com/wez/wezterm) - GPU-accelerated terminal emulator
- [Alacritty](https://github.com/alacritty/alacritty) - GPU-accelerated terminal emulator
- [vt100-rust](https://github.com/doy/vt100-rust) - Parser for terminal byte streams
- [tui-term](https://github.com/a-kenji/tui-term) - Pseudoterminal widget for Ratatui
- [term-transcript](https://github.com/slowli/term-transcript) - CLI/REPL snapshot testing

## License

TBD (likely MIT or MIT/Apache-2.0 dual license)

## Contact

- **Issues**: [GitHub Issues](https://github.com/[user]/mimic/issues)
- **Discussions**: [GitHub Discussions](https://github.com/[user]/mimic/discussions)

---

**Status**: 🚧 Research & Design Phase Complete → Ready for Phase 1 Implementation
**MVP Target**: v0.1.0 for dgx-pixels in 3-4 months
**See**: [ROADMAP.md](./docs/ROADMAP.md) | [DGX_PIXELS_REQUIREMENTS.md](./docs/DGX_PIXELS_REQUIREMENTS.md)
