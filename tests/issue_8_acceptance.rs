//! Acceptance test for Issue #8 - Expose Screen/Grid state for verification.
//!
//! This test verifies the exact use case described in the issue:
//! comparing the final state of another terminal emulator against ratatui-testlib.

use ratatui_testlib::ScreenState;

/// Mock terminal emulator for testing comparison.
/// In real usage, this would be Scarab or another terminal emulator.
struct MockTerminalEmulator {
    screen: ScreenState,
}

impl MockTerminalEmulator {
    fn new(cols: u16, rows: u16) -> Self {
        Self { screen: ScreenState::new(cols, rows) }
    }

    fn feed(&mut self, data: &[u8]) {
        self.screen.feed(data);
    }

    fn get_cell(&self, row: u16, col: u16) -> Option<MockCell> {
        self.screen.get_cell(row, col).map(|cell| MockCell {
            char: cell.c,
            fg: cell.fg,
            bg: cell.bg,
        })
    }

    fn rows(&self) -> u16 {
        self.screen.rows()
    }

    fn cols(&self) -> u16 {
        self.screen.cols()
    }
}

#[derive(Debug, PartialEq)]
struct MockCell {
    char: char,
    fg: Option<u8>,
    bg: Option<u8>,
}

#[test]
fn test_issue_8_exact_use_case() {
    // Deterministic test sequence
    let test_sequence = b"\x1b[2J\x1b[H\x1b[31mTest\x1b[0m";

    // Create oracle (ratatui-testlib)
    let mut oracle = ScreenState::new(80, 24);
    oracle.feed(test_sequence);

    // Create system under test (would be Scarab in real usage)
    let mut sut = MockTerminalEmulator::new(80, 24);
    sut.feed(test_sequence);

    // Compare using the API from the issue proposal:
    // "for row in 0..screen.rows() {
    //     for col in 0..screen.cols() {
    //         let cell: &Cell = screen.get_cell(col, row)?;
    //         println!("Char: {}, FG: {:?}, BG: {:?}", cell.char, cell.fg, cell.bg);
    //     }
    // }"

    for row in 0..oracle.rows() {
        for col in 0..oracle.cols() {
            let oracle_cell = oracle.get_cell(row, col).expect("Oracle cell should exist");
            let sut_cell = sut.get_cell(row, col).expect("SUT cell should exist");

            // Compare character
            assert_eq!(oracle_cell.c, sut_cell.char, "Character mismatch at ({}, {})", row, col);

            // Compare foreground color
            assert_eq!(
                oracle_cell.fg, sut_cell.fg,
                "Foreground color mismatch at ({}, {})",
                row, col
            );

            // Compare background color
            assert_eq!(
                oracle_cell.bg, sut_cell.bg,
                "Background color mismatch at ({}, {})",
                row, col
            );
        }
    }
}

#[test]
fn test_issue_8_snapshot_export() {
    // Test the snapshot() API mentioned in the issue
    let test_sequence = b"\x1b[31mRed\x1b[32mGreen\x1b[34mBlue";

    let mut screen = ScreenState::new(80, 24);
    screen.feed(test_sequence);

    // Get structured export
    let snapshot = screen.snapshot();

    // Verify snapshot structure matches issue requirements
    assert_eq!(snapshot.width, 80);
    assert_eq!(snapshot.height, 24);
    assert_eq!(snapshot.cells.len(), 24); // rows
    assert_eq!(snapshot.cells[0].len(), 80); // cols

    // Verify we can access cell data
    assert_eq!(snapshot.cells[0][0].c, 'R');
    assert_eq!(snapshot.cells[0][0].fg, Some(1)); // Red
    assert_eq!(snapshot.cells[0][3].c, 'G');
    assert_eq!(snapshot.cells[0][3].fg, Some(2)); // Green
}

#[test]
fn test_issue_8_dimension_accessors() {
    // Test rows() and cols() mentioned in acceptance criteria
    let screen = ScreenState::new(100, 30);

    assert_eq!(screen.rows(), 30);
    assert_eq!(screen.cols(), 100);
}

#[test]
fn test_issue_8_cell_field_access() {
    // Test that Cell exposes all required fields
    let mut screen = ScreenState::new(80, 24);
    screen.feed(b"\x1b[1;3;4;31;42mTest");

    let cell = screen.get_cell(0, 0).expect("Cell should exist");

    // All fields mentioned in issue should be accessible
    let _ = cell.c; // char
    let _ = cell.fg; // foreground color
    let _ = cell.bg; // background color

    // Additional attributes
    let _ = cell.bold;
    let _ = cell.italic;
    let _ = cell.underline;

    // Verify they have correct values
    assert_eq!(cell.c, 'T');
    assert_eq!(cell.fg, Some(1)); // Red
    assert_eq!(cell.bg, Some(2)); // Green
    assert!(cell.bold);
    assert!(cell.italic);
    assert!(cell.underline);
}

#[test]
fn test_issue_8_iterator_support() {
    // Test iterator support mentioned in acceptance criteria
    let mut screen = ScreenState::new(10, 3);
    screen.feed(b"ABC\r\nDEF\r\nGHI");

    // Test iter_rows()
    let row_count = screen.iter_rows().count();
    assert_eq!(row_count, 3);

    // Test iter_row()
    let first_row: Vec<_> = screen.iter_row(0).expect("Row 0 exists").take(3).collect();
    assert_eq!(first_row.len(), 3);
    assert_eq!(first_row[0].c, 'A');
    assert_eq!(first_row[1].c, 'B');
    assert_eq!(first_row[2].c, 'C');
}

#[test]
fn test_issue_8_complete_workflow() {
    // Complete workflow from issue description:
    // 1. Create screens
    // 2. Feed same sequence
    // 3. Compare final grid state

    let test_sequence = b"\x1b[2J\x1b[H\x1b[31;1mHello, World!\x1b[0m";

    // Oracle
    let mut oracle = ScreenState::new(80, 24);
    oracle.feed(test_sequence);

    // SUT
    let mut sut = ScreenState::new(80, 24);
    sut.feed(test_sequence);

    // Use snapshot for deep comparison
    let oracle_snapshot = oracle.snapshot();
    let sut_snapshot = sut.snapshot();

    assert_eq!(
        oracle_snapshot, sut_snapshot,
        "Identical sequences should produce identical snapshots"
    );

    // Verify specific cells have expected values
    assert_eq!(oracle_snapshot.cells[0][0].c, 'H');
    assert_eq!(oracle_snapshot.cells[0][0].fg, Some(1)); // Red
    assert!(oracle_snapshot.cells[0][0].bold);

    // After the text, cells should be reset
    assert_eq!(oracle_snapshot.cells[0][13].fg, None); // After "Hello, World!"
}
