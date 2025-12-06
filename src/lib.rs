//! # ratatui_testlib
//!
//! A Rust library for integration testing of terminal user interface (TUI) applications.
//!
//! ## Overview
//!
//! `ratatui_testlib` provides a comprehensive testing framework for TUI applications built with
//! libraries like Ratatui, with first-class support for:
//!
//! - **PTY-based testing**: Real terminal emulation with pseudo-terminal support
//! - **Sixel graphics testing**: Position verification and bounds checking for Sixel images
//! - **Bevy ECS integration**: Test Bevy-based TUI applications with `bevy_ratatui`
//! - **Async runtime support**: Full Tokio async/await support
//! - **Snapshot testing**: Integration with `insta` for visual regression testing
//! - **Headless CI/CD**: Works without X11/Wayland for GitHub Actions
//!
//! ## Quick Start
//!
//! ### PTY-Based Testing (Full TUI Applications)
//!
//! ```rust,no_run
//! use portable_pty::CommandBuilder;
//! use ratatui_testlib::{Result, TuiTestHarness};
//!
//! #[test]
//! fn test_my_tui_app() -> Result<()> {
//!     // Create a test harness with 80x24 terminal
//!     let mut harness = TuiTestHarness::new(80, 24)?;
//!
//!     // Spawn your TUI application
//!     let mut cmd = CommandBuilder::new("./my-tui-app");
//!     harness.spawn(cmd)?;
//!
//!     // Wait for initial render
//!     harness.wait_for(|state| state.contents().contains("Welcome"))?;
//!
//!     // Send keyboard input
//!     harness.send_text("hello")?;
//!
//!     // Capture screen state
//!     let contents = harness.screen_contents();
//!     assert!(contents.contains("hello"));
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Stream-Based Parsing (Headless/Oracle Mode)
//!
//! For testing terminal emulators or parsing raw escape sequences without PTY overhead:
//!
//! ```rust
//! use ratatui_testlib::ScreenState;
//!
//! #[test]
//! fn test_ansi_sequence_parsing() {
//!     // Create parser without PTY
//!     let mut screen = ScreenState::new(80, 24);
//!
//!     // Feed raw byte sequence
//!     let input = b"\x1b[31mHello\x1b[0m";
//!     screen.feed(input);
//!
//!     // Verify parsed state
//!     assert!(screen.contains("Hello"));
//!     assert_eq!(screen.get_cell(0, 0).unwrap().fg, Some(1)); // Red
//!     assert_eq!(screen.cursor_position(), (0, 5));
//! }
//! ```
//!
//! ## Testing Sixel Graphics
//!
//! With the `sixel` feature enabled, you can verify Sixel graphics positioning:
//!
//! ```rust,no_run
//! # #[cfg(feature = "sixel")]
//! # {
//! use ratatui_testlib::TuiTestHarness;
//!
//! # fn test_sixel() -> ratatui_testlib::Result<()> {
//! let mut harness = TuiTestHarness::new(80, 24)?;
//! // ... spawn your app and trigger Sixel rendering ...
//!
//! // Verify all Sixel graphics are within the preview area
//! let preview_area = (5, 5, 30, 15); // row, col, width, height
//! let sixel_regions = harness.state().sixel_regions();
//! for region in sixel_regions {
//!     assert!(region.start_row >= preview_area.0);
//!     assert!(region.start_col >= preview_area.1);
//! }
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Testing with Shared Memory State
//!
//! With the `shared-state` feature, you can access memory-mapped shared state:
//!
//! ```rust,no_run
//! # #[cfg(all(feature = "bevy", feature = "shared-state"))]
//! # {
//! use ratatui_testlib::{
//!     shared_state::{MemoryMappedState, SharedStateAccess},
//!     BevyTuiTestHarness,
//! };
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct AppState {
//!     frame_count: u32,
//!     status: String,
//! }
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! let harness = BevyTuiTestHarness::new()?.with_shared_state("/tmp/app_state.mmap")?;
//!
//! // Access shared state for assertions
//! if let Some(path) = harness.shared_state_path() {
//!     let state = MemoryMappedState::<AppState>::open(path)?;
//!     let app_state = state.read()?;
//!     assert!(app_state.frame_count > 0);
//! }
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Feature Flags
//!
//! - `async-tokio`: Enable Tokio async runtime support
//! - `bevy`: Enable Bevy ECS integration
//! - `bevy-ratatui`: Enable bevy_ratatui plugin support
//! - `ratatui-helpers`: Enable Ratatui-specific test helpers
//! - `sixel`: Enable Sixel graphics position tracking and testing
//! - `snapshot-insta`: Enable snapshot testing with `insta`
//! - `headless`: Enable headless mode for CI/CD (no display server required)
//! - `shared-state`: Enable memory-mapped shared state access for testing
//! - `mvp`: Enable all MVP features (recommended for dgx-pixels)
//!
//! ### Headless Mode for CI/CD
//!
//! The `headless` feature flag configures the library to run without any display
//! server dependencies, making it ideal for CI/CD environments:
//!
//! ```bash
//! # Run tests in CI without X11/Wayland
//! cargo test --features bevy,headless
//!
//! # Works in Docker containers
//! docker run --rm rust:latest cargo test --features bevy,headless
//! ```
//!
//! In headless mode, the Bevy integration uses `MinimalPlugins` instead of
//! `DefaultPlugins`, eliminating all graphics and windowing dependencies.
//!
//! ## Architecture
//!
//! The library is organized into several layers:
//!
//! 1. **PTY Management** (`pty`): Pseudo-terminal creation and lifecycle
//! 2. **Terminal Emulation** (`screen`): VT100 parsing and screen state
//! 3. **Test Harness** (`harness`): High-level testing API
//! 4. **Sixel Support** (`sixel`): Graphics protocol testing (MVP requirement)
//! 5. **Bevy Integration** (`bevy`): ECS testing support (MVP requirement)
//!
//! ## Primary Use Case: dgx-pixels
//!
//! This library was built to support comprehensive integration testing for the
//! [dgx-pixels](https://github.com/raibid-labs/dgx-pixels) project, with focus on:
//!
//! - Sixel graphics positioning within designated preview areas
//! - Sixel clearing on screen transitions
//! - Bevy ECS entity and component testing
//! - Text input and cursor position verification
//! - Async Tokio runtime compatibility

#![warn(
    missing_docs,
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]
#![deny(unsafe_code)]

mod error;
pub mod events;
pub mod golden;
mod harness;
pub mod navigation;
pub mod parallel;
mod pty;
mod screen;
pub mod terminal_profiles;
pub mod timing;

#[cfg(feature = "sixel")]
pub mod graphics;

#[cfg(feature = "sixel")]
pub mod sixel;

#[cfg(feature = "bevy")]
pub mod bevy;

#[cfg(feature = "shared-state")]
pub mod shared_state;

#[cfg(feature = "async-tokio")]
mod async_harness;

// Public API exports
#[cfg(feature = "async-tokio")]
pub use async_harness::{AsyncTuiTestHarness, WaitResult};
pub use error::{Result, TermTestError};
pub use events::{KeyCode, KeyEvent, Modifiers, MouseButton, MouseEvent, ScrollDirection};
pub use golden::{GoldenFile, GoldenMetadata};
pub use harness::{Axis, MemoryResults, RecordedEvent, TuiTestHarness};
pub use navigation::{
    FocusInfo, HintElementType, HintLabel, NavMode, NavigationTestExt, PromptMarker,
    PromptMarkerType,
};
pub use parallel::{
    IsolatedTerminal, PoolConfig, PoolStats, TerminalGuard, TerminalId, TerminalPool, TestContext,
};
pub use pty::TestTerminal;
pub use screen::{Cell, GridSnapshot, ITerm2Region, KittyRegion, Rect, ScreenState, SixelRegion};
pub use terminal_profiles::{
    ColorDepth, Feature, MouseProtocol, TerminalCapabilities, TerminalProfile,
};

/// Re-export of [`ScreenState`] for clarity in stream-based parsing contexts.
///
/// This is provided as a convenience for users who want to emphasize that they're
/// using the library in headless/stream-based mode rather than PTY mode.
///
/// # Example
///
/// ```rust
/// // Both of these are equivalent:
/// use ratatui_testlib::{Parser, ScreenState}; // Clearer name for stream-based usage
///
/// let mut screen = ScreenState::new(80, 24);
/// let mut parser = Parser::new(80, 24);
/// ```
pub type Parser = ScreenState;

#[cfg(all(feature = "bevy", feature = "snapshot-insta"))]
pub use bevy::ComponentSnapshot;
#[cfg(feature = "bevy")]
pub use bevy::{
    BevyTuiTestHarness, HeadlessBevyRunner, HybridBevyHarness, HybridBevyHarnessBuilder,
};
#[cfg(feature = "sixel")]
pub use graphics::{GraphicsCapture, GraphicsProtocol, GraphicsRegion};
// Re-export commonly used types for convenience
pub use portable_pty::CommandBuilder;
#[cfg(feature = "sixel")]
pub use sixel::{SixelCapture, SixelSequence};
