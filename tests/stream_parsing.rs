//! Integration tests for headless stream-based parsing.
//!
//! These tests demonstrate using ratatui-testlib as a verification oracle
//! for terminal emulators by feeding raw byte streams directly without PTY overhead.

use ratatui_testlib::ScreenState;

#[test]
fn test_basic_ansi_color_sequences() {
    let mut screen = ScreenState::new(80, 24);

    // Feed raw ANSI color sequence: red text "Hello"
    let input = b"\x1b[31mHello\x1b[0m";
    screen.feed(input);

    // Verify text was rendered
    assert!(screen.contains("Hello"));

    // Verify color attribute (red = color 1 in ANSI)
    let cell = screen.get_cell(0, 0).expect("Cell should exist");
    assert_eq!(cell.c, 'H');
    assert_eq!(cell.fg, Some(1), "Foreground should be red (color 1)");

    // Verify reset worked (cell after "Hello" should have default color)
    let cell = screen.get_cell(0, 5).expect("Cell should exist");
    assert_eq!(cell.fg, None, "Foreground should be reset to default");
}

#[test]
fn test_cursor_positioning() {
    let mut screen = ScreenState::new(80, 24);

    // Move cursor to (10, 20) and write "Test"
    // ESC [ 10 ; 20 H = move to row 10, col 20 (1-based)
    let input = b"\x1b[10;20HTest";
    screen.feed(input);

    // Verify cursor position (0-based)
    let (row, col) = screen.cursor_position();
    assert_eq!(row, 9, "Row should be 9 (0-based from 10)");
    assert_eq!(col, 23, "Col should be 23 (0-based from 20, plus 4 for 'Test')");

    // Verify text at correct position
    assert_eq!(screen.text_at(9, 19), Some('T'));
    assert_eq!(screen.text_at(9, 20), Some('e'));
    assert_eq!(screen.text_at(9, 21), Some('s'));
    assert_eq!(screen.text_at(9, 22), Some('t'));
}

#[test]
fn test_incremental_stream_parsing() {
    let mut screen = ScreenState::new(80, 24);

    // Feed bytes incrementally (simulating streaming data)
    screen.feed(b"Hello");
    assert!(screen.contains("Hello"));

    screen.feed(b", ");
    assert!(screen.contains("Hello, "));

    screen.feed(b"World!");
    assert!(screen.contains("Hello, World!"));

    // Verify cursor advanced correctly
    let (row, col) = screen.cursor_position();
    assert_eq!(row, 0);
    assert_eq!(col, 13, "Cursor should be after 'Hello, World!'");
}

#[test]
fn test_partial_escape_sequence_handling() {
    let mut screen = ScreenState::new(80, 24);

    // Feed escape sequence in parts (tests state machine robustness)
    screen.feed(b"\x1b"); // ESC
    screen.feed(b"["); // CSI start
    screen.feed(b"3"); // Parameter digit
    screen.feed(b"1"); // Parameter digit
    screen.feed(b"m"); // SGR command
    screen.feed(b"Red"); // Text
    screen.feed(b"\x1b[0m"); // Reset

    // Verify red text was rendered
    assert!(screen.contains("Red"));
    let cell = screen.get_cell(0, 0).unwrap();
    assert_eq!(cell.fg, Some(1), "Should be red");
}

#[test]
fn test_complex_sgr_sequence() {
    let mut screen = ScreenState::new(80, 24);

    // Bold + italic + underline + red text
    let input = b"\x1b[1;3;4;31mStyled\x1b[0m";
    screen.feed(input);

    let cell = screen.get_cell(0, 0).unwrap();
    assert_eq!(cell.c, 'S');
    assert_eq!(cell.fg, Some(1), "Should be red");
    assert!(cell.bold, "Should be bold");
    assert!(cell.italic, "Should be italic");
    assert!(cell.underline, "Should be underlined");

    // After reset
    let cell = screen.get_cell(0, 6).unwrap();
    assert!(!cell.bold, "Bold should be reset");
    assert!(!cell.italic, "Italic should be reset");
    assert!(!cell.underline, "Underline should be reset");
}

#[test]
fn test_256_color_mode() {
    let mut screen = ScreenState::new(80, 24);

    // ESC [ 38 ; 5 ; 196 m = foreground color 196 (bright red)
    // ESC [ 48 ; 5 ; 21 m = background color 21 (blue)
    let input = b"\x1b[38;5;196m\x1b[48;5;21mColor\x1b[0m";
    screen.feed(input);

    let cell = screen.get_cell(0, 0).unwrap();
    assert_eq!(cell.c, 'C');
    assert_eq!(cell.fg, Some(196), "Foreground should be color 196");
    assert_eq!(cell.bg, Some(21), "Background should be color 21");
}

#[test]
fn test_newline_and_carriage_return() {
    let mut screen = ScreenState::new(80, 24);

    // Line 1, newline, carriage return, Line 2
    screen.feed(b"Line 1\r\nLine 2");

    // Verify both lines
    assert_eq!(screen.text_at(0, 0), Some('L'));
    assert_eq!(screen.row_contents(0).trim(), "Line 1");
    assert_eq!(screen.row_contents(1).trim(), "Line 2");

    // Cursor should be on row 1, after "Line 2"
    let (row, col) = screen.cursor_position();
    assert_eq!(row, 1);
    assert_eq!(col, 6);
}

#[test]
fn test_tab_character() {
    let mut screen = ScreenState::new(80, 24);

    // Tab advances to next tab stop (every 8 columns)
    screen.feed(b"AB\tCD");

    // 'A' at 0, 'B' at 1, tab advances to 8, 'C' at 8, 'D' at 9
    assert_eq!(screen.text_at(0, 0), Some('A'));
    assert_eq!(screen.text_at(0, 1), Some('B'));
    assert_eq!(screen.text_at(0, 8), Some('C'));
    assert_eq!(screen.text_at(0, 9), Some('D'));
}

#[test]
fn test_cursor_movement_sequences() {
    let mut screen = ScreenState::new(80, 24);

    // Position cursor, write, move cursor, write again
    screen.feed(b"\x1b[5;5HFirst"); // Row 5, col 5
    screen.feed(b"\x1b[10;10HSecond"); // Row 10, col 10

    // Verify both texts at correct positions
    assert_eq!(screen.text_at(4, 4), Some('F'));
    assert!(screen.row_contents(4).contains("First"));

    assert_eq!(screen.text_at(9, 9), Some('S'));
    assert!(screen.row_contents(9).contains("Second"));
}

#[test]
fn test_cursor_up_down() {
    let mut screen = ScreenState::new(80, 24);

    screen.feed(b"\x1b[10;10H"); // Move to (10, 10)
    assert_eq!(screen.cursor_position(), (9, 9));

    screen.feed(b"\x1b[3A"); // Cursor up 3
    assert_eq!(screen.cursor_position(), (6, 9));

    screen.feed(b"\x1b[5B"); // Cursor down 5
    assert_eq!(screen.cursor_position(), (11, 9));
}

#[test]
fn test_cursor_forward_backward() {
    let mut screen = ScreenState::new(80, 24);

    screen.feed(b"\x1b[10;10H"); // Move to (10, 10)
    assert_eq!(screen.cursor_position(), (9, 9));

    screen.feed(b"\x1b[5C"); // Cursor forward 5
    assert_eq!(screen.cursor_position(), (9, 14));

    screen.feed(b"\x1b[7D"); // Cursor backward 7
    assert_eq!(screen.cursor_position(), (9, 7));
}

#[test]
fn test_deterministic_byte_sequence() {
    // This test demonstrates using ScreenState as a verification oracle
    // for another terminal emulator implementation

    let mut screen = ScreenState::new(80, 24);

    // Complex deterministic sequence
    let sequence = b"\x1b[2J\x1b[H\x1b[31mRed\x1b[0m \x1b[32mGreen\x1b[0m \x1b[34mBlue\x1b[0m";

    screen.feed(sequence);

    // Verify the oracle state
    assert_eq!(screen.text_at(0, 0), Some('R'));
    assert_eq!(screen.text_at(0, 1), Some('e'));
    assert_eq!(screen.text_at(0, 2), Some('d'));
    assert_eq!(screen.text_at(0, 4), Some('G'));

    // Verify colors
    assert_eq!(screen.get_cell(0, 0).unwrap().fg, Some(1)); // Red
    assert_eq!(screen.get_cell(0, 4).unwrap().fg, Some(2)); // Green
    assert_eq!(screen.get_cell(0, 10).unwrap().fg, Some(4)); // Blue
}

#[test]
fn test_multiple_screen_instances() {
    // Verify that multiple independent screen instances can coexist
    let mut screen1 = ScreenState::new(80, 24);
    let mut screen2 = ScreenState::new(100, 30);

    screen1.feed(b"Screen 1");
    screen2.feed(b"Screen 2");

    assert!(screen1.contains("Screen 1"));
    assert!(!screen1.contains("Screen 2"));

    assert!(screen2.contains("Screen 2"));
    assert!(!screen2.contains("Screen 1"));
}

#[test]
fn test_zero_cost_without_pty() {
    // This test verifies that ScreenState can be used without any PTY overhead
    // Just create, feed, and query - no process spawning needed

    let mut screen = ScreenState::new(80, 24);

    // Simulate terminal output from a hypothetical emulator
    let emulator_output = b"\x1b[1;1H\x1b[32mTest Output\x1b[0m";

    screen.feed(emulator_output);

    // Verify we can query the state
    assert!(screen.contains("Test Output"));
    assert_eq!(screen.size(), (80, 24));
    assert_eq!(screen.get_cell(0, 0).unwrap().fg, Some(2)); // Green
}

#[test]
fn test_sixel_sequence_detection() {
    let mut screen = ScreenState::new(80, 24);

    // Feed a minimal Sixel sequence with raster attributes
    screen.feed(b"\x1b[5;10H"); // Position cursor
    screen.feed(b"\x1bPq"); // DCS - Start Sixel
    screen.feed(b"\"1;1;100;50"); // Raster: 100x50 pixels
    screen.feed(b"#0;2;100;100;100"); // Color definition
    screen.feed(b"#0~"); // Sixel data
    screen.feed(b"\x1b\\"); // String terminator

    // Verify Sixel region was captured
    let regions = screen.sixel_regions();
    assert_eq!(regions.len(), 1, "Should detect one Sixel region");

    let region = &regions[0];
    assert_eq!(region.start_row, 4, "Sixel at row 4 (0-based)");
    assert_eq!(region.start_col, 9, "Sixel at col 9 (0-based)");
    assert_eq!(region.width, 100, "Sixel width in pixels");
    assert_eq!(region.height, 50, "Sixel height in pixels");

    // Verify has_sixel_at works
    assert!(screen.has_sixel_at(4, 9));
    assert!(!screen.has_sixel_at(0, 0));
}

#[test]
fn test_stream_based_comparison() {
    // Demonstrate comparing two terminal emulator implementations

    // System under test output
    let sut_output = b"\x1b[31mHello\x1b[0m";

    // Reference implementation (ratatui-testlib as oracle)
    let mut oracle = ScreenState::new(80, 24);
    oracle.feed(sut_output);

    // Now you would feed the same sequence to your SUT and compare:
    // - Text content: oracle.contents()
    // - Cell attributes: oracle.get_cell(row, col)
    // - Cursor position: oracle.cursor_position()
    // - Sixel regions: oracle.sixel_regions()

    assert_eq!(oracle.contents().lines().next().unwrap().trim(), "Hello");
    assert_eq!(oracle.get_cell(0, 0).unwrap().fg, Some(1));
}

#[test]
fn test_escape_sequence_index() {
    let mut screen = ScreenState::new(80, 24);

    // ESC D = Index (move cursor down)
    screen.feed(b"\x1b[5;10H"); // Position to (5, 10)
    screen.feed(b"\x1bD"); // Index (down)

    let (row, col) = screen.cursor_position();
    assert_eq!(row, 5, "Row should increment");
    assert_eq!(col, 9, "Column should stay same");
}

#[test]
fn test_escape_sequence_nel() {
    let mut screen = ScreenState::new(80, 24);

    // ESC E = Next Line (down + carriage return)
    screen.feed(b"\x1b[5;10H"); // Position to (5, 10)
    screen.feed(b"\x1bE"); // Next line

    let (row, col) = screen.cursor_position();
    assert_eq!(row, 5, "Row should increment");
    assert_eq!(col, 0, "Column should reset to 0");
}

#[test]
fn test_row_contents_extraction() {
    let mut screen = ScreenState::new(80, 24);

    screen.feed(b"\x1b[1;1HFirst Line\r\n");
    screen.feed(b"Second Line\r\n");
    screen.feed(b"Third Line");

    assert_eq!(screen.row_contents(0).trim(), "First Line");
    assert_eq!(screen.row_contents(1).trim(), "Second Line");
    assert_eq!(screen.row_contents(2).trim(), "Third Line");
    assert_eq!(screen.row_contents(100), "", "Out of bounds should return empty");
}

#[test]
fn test_size_query() {
    let screen1 = ScreenState::new(80, 24);
    assert_eq!(screen1.size(), (80, 24));

    let screen2 = ScreenState::new(120, 40);
    assert_eq!(screen2.size(), (120, 40));

    let screen3 = ScreenState::new(1, 1);
    assert_eq!(screen3.size(), (1, 1));
}

#[test]
fn test_debug_contents() {
    let mut screen = ScreenState::new(80, 24);
    screen.feed(b"Debug Test");

    let contents = screen.debug_contents();
    assert!(contents.contains("Debug Test"));

    // debug_contents should be equivalent to contents (for now)
    assert_eq!(contents, screen.contents());
}
