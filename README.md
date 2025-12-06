# ratatui-testlib

> A Rust library for integration testing of terminal user interface (TUI) applications with first-class support for Ratatui, Bevy ECS integration, and Sixel graphics protocols.

## Overview

`ratatui-testlib` bridges the gap between unit testing with Ratatui's `TestBackend` and real-world integration testing of TUI applications. It provides a PTY-based test harness that enables testing of features requiring actual terminal escape sequence processing, including **Sixel graphics position verification**, **Bevy ECS integration**, **bevy_ratatui support**, and complex user interaction flows.

### MVP Goal

Built to enable comprehensive integration testing for the [**dgx-pixels**](https://github.com/raibid-labs/dgx-pixels) project - a Bevy-based TUI application with Sixel graphics support.

### Why ratatui-testlib?

**Current Limitation**: Ratatui's `TestBackend` is great for unit testing widgets and layouts, but it can't test:
- PTY-specific behavior (terminal size negotiation, TTY detection)
- Graphics protocols (Sixel, iTerm2 images, Kitty graphics)
- Real terminal integration
- User interaction flows
- Event handling in actual terminal context

**Solution**: `ratatui-testlib` runs your TUI application in a real pseudo-terminal (PTY), captures the output using a terminal emulator, and provides an ergonomic API for assertions and snapshot testing.

### Key Features

**MVP (v0.1.0)**:
- ✅ **PTY-Based Testing**: Real terminal environment using `portable-pty`
- ✅ **Sixel Position Tracking**: Verify graphics render at correct coordinates and within bounds
- ✅ **Bevy ECS Integration**: Query entities, control update cycles, test Bevy systems
- ✅ **bevy_ratatui Support**: First-class integration with bevy_ratatui plugin
- ✅ **Event Simulation**: Keyboard events for navigation and input, plus mouse events
- ✅ **Smart Waiting**: Condition-based waiting with timeouts
- ✅ **Snapshot Testing**: Integration with `insta`
- ✅ **Tokio Async Support**: Native `AsyncTuiTestHarness` for async TUI apps
- ✅ **High-Level Assertions**: Ergonomic API (text_at, cursor_position, sixel_within, etc.)
- ✅ **CI/CD Ready**: Headless testing without X11/Wayland
- ✅ **Visual Regression**: Golden file support for verifying screen state changes
- ✅ **Parallel Execution**: Safe parallel testing with isolated PTYs

**Post-MVP**:
- expect-test integration
- async-std support
- Cross-platform (macOS, Windows) - *Currently Linux focused*

## Quick Example

```rust
use ratatui_testlib::{TuiTestHarness, KeyCode};
use portable_pty::CommandBuilder;

#[test]
fn test_navigation() -> ratatui_testlib::Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Spawn your TUI app
    let mut cmd = CommandBuilder::new("./my-tui-app");
    harness.spawn(cmd)?;

    // Wait for initial render
    harness.wait_for_text("Main Menu")?;

    // Simulate user input (Navigation)
    harness.send_key(KeyCode::Down)?;
    harness.send_key(KeyCode::Enter)?;

    // Verify result
    harness.wait_for_text("Settings")?;

    Ok(())
}
```

## Async Support (Tokio)

```rust
use ratatui_testlib::AsyncTuiTestHarness;
use portable_pty::CommandBuilder;

#[tokio::test]
async fn test_async_app() -> ratatui_testlib::Result<()> {
    let mut harness = AsyncTuiTestHarness::new(80, 24).await?;
    let mut cmd = CommandBuilder::new("./my-async-app");
    harness.spawn(cmd).await?;

    harness.wait_for_text("Ready").await?;
    
    // Type text with delays
    harness.type_text("query\n").await?;
    
    harness.wait_for_text("Results").await?;
    Ok(())
}
```

## Testing Sixel Graphics

```rust
use ratatui_testlib::TuiTestHarness;

#[test]
fn test_sixel_renders_in_preview_area() -> ratatui_testlib::Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;
    // ... spawn app ...

    // Define the preview area where Sixel graphics should appear
    let preview_area = (5, 40, 35, 15); // (row, col, width, height)

    // Assert: Sixel graphics within bounds
    harness.assert_sixel_within_bounds(preview_area)?;
    
    // Helper for standard layout
    harness.assert_preview_has_sixel()?;

    Ok(())
}
```

## Bevy ECS Integration

```rust
use ratatui_testlib::BevyTuiTestHarness;

#[test]
fn test_bevy_systems() -> ratatui_testlib::Result<()> {
    let mut test = BevyTuiTestHarness::new()?;
    
    // Manipulate World
    test.world_mut().spawn(MyComponent);
    
    // Run schedule
    test.update()?;
    
    // Assertions
    test.assert_component_exists::<MyComponent>()?;
    
    Ok(())
}
```

## Headless Mode for CI/CD

The `headless` feature flag enables testing in environments without display servers (X11/Wayland), making it perfect for CI/CD pipelines:

```toml
# Cargo.toml
[dev-dependencies]
ratatui-testlib = { version = "0.1", features = ["bevy", "headless"] }
```

```bash
# Run tests in headless mode (works in Docker without DISPLAY)
cargo test --features bevy,headless
```

## Documentation

### Core Documentation
- **[ARCHITECTURE.md](./docs/ARCHITECTURE.md)** - Library architecture and design decisions
- **[docs/STRUCTURE.md](./docs/STRUCTURE.md)** - Documentation organization and versioning policy
- **[CHANGELOG.md](./CHANGELOG.md)** - Version history and release notes

### Versioned Documentation
- **[vNEXT](./docs/versions/vNEXT/)** - Unreleased features and planned enhancements

### Additional Resources
- **[CONTRIBUTING.md](./CONTRIBUTING.md)** - Guidelines for contributors
- **[API Documentation](https://docs.rs/ratatui-testlib)** - Full API reference on docs.rs

## Installation

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
ratatui-testlib = "0.1.0"
```

**Feature Flags**:
- `async-tokio`: Enable `AsyncTuiTestHarness`.
- `bevy`: Enable Bevy ECS integration.
- `sixel`: Enable Sixel graphics support.
- `snapshot-insta`: Enable snapshot testing.
- `headless`: Enable headless mode for CI.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## License

MIT License. See [LICENSE](LICENSE) for details.