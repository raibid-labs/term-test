//! Demonstration of mouse event simulation.
//!
//! This example shows how to simulate mouse clicks, drags, and scrolling
//! using the `ratatui_testlib` harness.
//!
//! Run with:
//! ```bash
//! cargo run --example mouse_demo
//! ```

use std::time::Duration;

use portable_pty::CommandBuilder;
use ratatui_testlib::{MouseButton, Result, ScrollDirection, TuiTestHarness};

fn main() -> Result<()> {
    println!("=== Mouse Event Demo ===\n");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Spawn 'cat -v' to echo back the escape sequences visible as text
    // cat -v converts ESC to ^[ so they don't get interpreted by the terminal
    let mut cmd = CommandBuilder::new("cat");
    cmd.arg("-v");
    harness.spawn(cmd)?;

    println!("1. Simulating Left Click at (10, 5)...");
    harness.mouse_click(10, 5, MouseButton::Left)?;
    harness.send_text("\n")?;

    // Wait for echo. Note: cat -v outputs ^[ for ESC
    // Left Click Press: ^[[<0;11;6M
    // Left Click Release: ^[[<0;11;6m
    harness.wait_for(|state| state.contents().contains("[<0;11;6M"))?;
    println!("   ✓ Received mouse click sequence echo");

    println!("2. Simulating Right Click at (20, 10)...");
    harness.mouse_click(20, 10, MouseButton::Right)?;
    harness.send_text("\n")?;
    harness.wait_for(|state| state.contents().contains("[<2;21;11M"))?;
    println!("   ✓ Received right click sequence echo");

    println!("3. Simulating Scroll Up...");
    harness.mouse_scroll(10, 10, ScrollDirection::Up)?;
    harness.send_text("\n")?;
    harness.wait_for(|state| state.contents().contains("[<64;11;11M"))?;
    println!("   ✓ Received scroll up sequence echo");

    println!("\n=== Mouse Demo Complete ===");

    // Print the final screen state to show the echoed sequences
    println!("\nFinal Screen State (Cat Output):");
    println!("--------------------------------");
    println!("{}", harness.screen_contents());
    println!("--------------------------------");

    Ok(())
}
