# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial implementation of mimic TUI testing library
- PTY-based terminal emulation with `portable-pty`
- VT100/ANSI escape sequence parsing with `vtparse` and `termwiz`
- Sixel graphics position tracking and verification
- Bevy ECS integration for testing Bevy TUI applications
- Async/await support with Tokio
- Snapshot testing integration with `insta`
- Comprehensive test harness API (`TuiTestHarness`)
- Screen state inspection (`ScreenState`, `Cell`, `SixelRegion`)
- Event simulation (keyboard input, wait conditions)
- Examples for common testing scenarios
- Full documentation and API reference

### Features
- `async-tokio`: Tokio async runtime support
- `bevy`: Bevy ECS integration
- `bevy-ratatui`: bevy_ratatui plugin support
- `ratatui-helpers`: Ratatui-specific test helpers
- `sixel`: Sixel graphics position tracking
- `snapshot-insta`: Snapshot testing with insta
- `mvp`: Bundle of all MVP features

### Public API
- `TuiTestHarness`: Main test harness for PTY-based testing
- `ScreenState`: Terminal screen state with VT100 parsing
- `ScreenState::new(width, height)`: Create new screen state
- `ScreenState::feed(data)`: Process terminal escape sequences
- `ScreenState::get_cell(row, col)`: Access individual cells
- `ScreenState::contents()`: Get screen contents as string
- `ScreenState::sixel_regions()`: Get Sixel graphics regions
- `Cell`: Terminal cell with character and attributes
- `SixelRegion`: Sixel graphics position and data
- `TestTerminal`: PTY management wrapper
- `BevyTuiTestHarness`: Bevy-specific test harness

### Addresses
- Issue #1: TUI Integration Testing Framework Requirements - Fully implemented
- Issue #7: Add public API for headless/stream-based parsing - Implemented via `ScreenState::feed()`
- Issue #8: Expose Screen/Grid state for verification - Implemented via `get_cell()` and public `Cell` struct

## [0.1.0] - TBD

Initial release of mimic.

[Unreleased]: https://github.com/raibid-labs/mimic/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/raibid-labs/mimic/releases/tag/v0.1.0
