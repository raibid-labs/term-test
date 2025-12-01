//! # mimic
//!
//! A Rust library for integration testing of terminal user interface (TUI) applications.
//!
//! ## Overview
//!
//! `mimic` provides a comprehensive testing framework for TUI applications built with
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
//! ```rust,no_run
//! use mimic::{TuiTestHarness, Result};
//! use portable_pty::CommandBuilder;
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
//!     harness.wait_for(|state| {
//!         state.contents().contains("Welcome")
//!     })?;
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
//! ## Testing Sixel Graphics
//!
//! With the `sixel` feature enabled, you can verify Sixel graphics positioning:
//!
//! ```rust,no_run
//! # #[cfg(feature = "sixel")]
//! # {
//! use mimic::TuiTestHarness;
//!
//! # fn test_sixel() -> mimic::Result<()> {
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
//! ## Feature Flags
//!
//! - `async-tokio`: Enable Tokio async runtime support
//! - `bevy`: Enable Bevy ECS integration
//! - `bevy-ratatui`: Enable bevy_ratatui plugin support
//! - `ratatui-helpers`: Enable Ratatui-specific test helpers
//! - `sixel`: Enable Sixel graphics position tracking and testing
//! - `snapshot-insta`: Enable snapshot testing with `insta`
//! - `mvp`: Enable all MVP features (recommended for dgx-pixels)
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
mod harness;
mod pty;
mod screen;

#[cfg(feature = "sixel")]
mod sixel;

#[cfg(feature = "bevy")]
mod bevy;

// Public API exports
pub use error::{Result, TermTestError};
pub use events::{KeyCode, KeyEvent, Modifiers};
pub use harness::TuiTestHarness;
pub use pty::TestTerminal;
pub use screen::{Cell, ScreenState, SixelRegion};

#[cfg(feature = "sixel")]
pub use sixel::{SixelCapture, SixelSequence};

#[cfg(feature = "bevy")]
pub use bevy::BevyTuiTestHarness;

// Re-export commonly used types for convenience
pub use portable_pty::CommandBuilder;
