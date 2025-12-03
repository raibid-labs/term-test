//! Example: Using ratatui-testlib for headless stream-based parsing.
//!
//! This example demonstrates how to use ratatui-testlib as a verification oracle
//! for terminal emulators without any PTY overhead. Instead of spawning processes,
//! you feed raw byte sequences directly to the parser.
//!
//! Run with: cargo run --example stream_parsing

use ratatui_testlib::ScreenState;

fn main() {
    println!("=== Stream-Based Parsing Example ===\n");

    // Example 1: Basic text with ANSI colors
    basic_ansi_colors();

    // Example 2: Cursor positioning
    cursor_positioning();

    // Example 3: Complex formatting
    complex_formatting();

    // Example 4: Sixel graphics detection
    sixel_detection();

    // Example 5: Verification oracle pattern
    verification_oracle();
}

fn basic_ansi_colors() {
    println!("--- Example 1: Basic ANSI Colors ---");

    let mut screen = ScreenState::new(80, 24);

    // Feed raw bytes with ANSI color codes
    let input = b"\x1b[31mRed Text\x1b[0m \x1b[32mGreen Text\x1b[0m \x1b[34mBlue Text\x1b[0m";
    screen.feed(input);

    // Query the parsed state
    println!("Screen contents: {}", screen.contents().lines().next().unwrap());

    // Check individual cell attributes
    if let Some(cell) = screen.get_cell(0, 0) {
        println!("First character: '{}' with fg color {:?}", cell.c, cell.fg);
    }

    if let Some(cell) = screen.get_cell(0, 9) {
        println!("Tenth character: '{}' with fg color {:?}", cell.c, cell.fg);
    }

    println!();
}

fn cursor_positioning() {
    println!("--- Example 2: Cursor Positioning ---");

    let mut screen = ScreenState::new(80, 24);

    // Move cursor to different positions and write text
    screen.feed(b"\x1b[5;10HFirst");
    screen.feed(b"\x1b[10;20HSecond");
    screen.feed(b"\x1b[15;30HThird");

    println!("Cursor position: {:?}", screen.cursor_position());

    // Extract text at specific locations
    println!("Row 4: {}", screen.row_contents(4).trim());
    println!("Row 9: {}", screen.row_contents(9).trim());
    println!("Row 14: {}", screen.row_contents(14).trim());

    println!();
}

fn complex_formatting() {
    println!("--- Example 3: Complex Formatting ---");

    let mut screen = ScreenState::new(80, 24);

    // Bold, italic, underlined text with color
    let input = b"\x1b[1;3;4;31mStyled Text\x1b[0m";
    screen.feed(input);

    if let Some(cell) = screen.get_cell(0, 0) {
        println!("Character: '{}'", cell.c);
        println!("  Bold: {}", cell.bold);
        println!("  Italic: {}", cell.italic);
        println!("  Underline: {}", cell.underline);
        println!("  Foreground: {:?}", cell.fg);
    }

    println!();
}

fn sixel_detection() {
    println!("--- Example 4: Sixel Graphics Detection ---");

    let mut screen = ScreenState::new(80, 24);

    // Position cursor and render a Sixel image
    screen.feed(b"\x1b[10;20H"); // Move cursor to (10, 20)
    screen.feed(b"\x1bPq"); // Start Sixel sequence
    screen.feed(b"\"1;1;100;50"); // Raster attributes: 100x50 pixels
    screen.feed(b"#0;2;100;100;100"); // Color definition
    screen.feed(b"#0~"); // Sixel data
    screen.feed(b"\x1b\\"); // End Sixel sequence

    // Query Sixel regions
    let regions = screen.sixel_regions();
    println!("Detected {} Sixel region(s)", regions.len());

    for (i, region) in regions.iter().enumerate() {
        println!(
            "  Region {}: pos=({}, {}), size={}x{} pixels",
            i, region.start_row, region.start_col, region.width, region.height
        );
    }

    // Check if Sixel exists at specific position
    if screen.has_sixel_at(9, 19) {
        println!("  Confirmed: Sixel detected at position (9, 19)");
    }

    println!();
}

fn verification_oracle() {
    println!("--- Example 5: Verification Oracle Pattern ---");

    // Simulate testing a terminal emulator implementation
    let test_sequence = b"\x1b[2J\x1b[H\x1b[31mHello, World!\x1b[0m";

    // Create the oracle
    let mut oracle = ScreenState::new(80, 24);
    oracle.feed(test_sequence);

    // In a real test, you would:
    // 1. Feed the same sequence to your system under test (SUT)
    // 2. Compare the SUT's state to the oracle's state

    println!("Expected text content: {}", oracle.contents().lines().next().unwrap().trim());
    println!("Expected cursor position: {:?}", oracle.cursor_position());

    if let Some(cell) = oracle.get_cell(0, 0) {
        println!("Expected first cell color: {:?}", cell.fg);
    }

    // This pattern is useful for:
    // - Integration testing terminal emulators
    // - Verifying ANSI sequence parsing
    // - Regression testing terminal behavior
    // - Comparing implementations

    println!();
}
