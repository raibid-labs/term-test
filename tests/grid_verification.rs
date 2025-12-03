//! Tests for grid state verification API (issue #8).
//!
//! These tests verify the API for exposing screen/grid state for verification,
//! enabling comparison between different terminal emulator implementations.

use ratatui_testlib::{Cell, ScreenState};

#[test]
fn test_rows_cols_accessors() {
    let screen = ScreenState::new(80, 24);

    assert_eq!(screen.rows(), 24, "rows() should return height");
    assert_eq!(screen.cols(), 80, "cols() should return width");
    assert_eq!(screen.size(), (80, 24), "size() should match");
}

#[test]
fn test_get_cell_basic() {
    let mut screen = ScreenState::new(80, 24);
    screen.feed(b"\x1b[31mRed\x1b[0m");

    // First character should be 'R' with red foreground
    let cell = screen.get_cell(0, 0).expect("Cell should exist");
    assert_eq!(cell.c, 'R');
    assert_eq!(cell.fg, Some(1), "Red color");
    assert_eq!(cell.bg, None, "No background");
    assert!(!cell.bold);
    assert!(!cell.italic);
    assert!(!cell.underline);

    // After reset, should have default attributes
    let cell = screen.get_cell(0, 3).expect("Cell should exist");
    assert_eq!(cell.fg, None, "Should be reset");
}

#[test]
fn test_get_cell_with_all_attributes() {
    let mut screen = ScreenState::new(80, 24);

    // Bold + italic + underline + foreground + background
    screen.feed(b"\x1b[1;3;4;31;42mStyled\x1b[0m");

    let cell = screen.get_cell(0, 0).expect("Cell should exist");
    assert_eq!(cell.c, 'S');
    assert_eq!(cell.fg, Some(1), "Red foreground");
    assert_eq!(cell.bg, Some(2), "Green background");
    assert!(cell.bold, "Should be bold");
    assert!(cell.italic, "Should be italic");
    assert!(cell.underline, "Should be underlined");
}

#[test]
fn test_get_cell_256_colors() {
    let mut screen = ScreenState::new(80, 24);

    // 256-color mode
    screen.feed(b"\x1b[38;5;196m\x1b[48;5;21mColor");

    let cell = screen.get_cell(0, 0).expect("Cell should exist");
    assert_eq!(cell.fg, Some(196), "256-color foreground");
    assert_eq!(cell.bg, Some(21), "256-color background");
}

#[test]
fn test_get_cell_out_of_bounds() {
    let screen = ScreenState::new(80, 24);

    assert!(screen.get_cell(0, 0).is_some(), "Valid position");
    assert!(screen.get_cell(23, 79).is_some(), "Last valid position");
    assert!(screen.get_cell(24, 0).is_none(), "Row out of bounds");
    assert!(screen.get_cell(0, 80).is_none(), "Col out of bounds");
    assert!(screen.get_cell(100, 100).is_none(), "Both out of bounds");
}

#[test]
fn test_iter_rows() {
    let mut screen = ScreenState::new(10, 3);
    screen.feed(b"Line1\r\nLine2\r\nLine3");

    let rows: Vec<&[Cell]> = screen.iter_rows().collect();
    assert_eq!(rows.len(), 3, "Should have 3 rows");

    // Check first row
    assert_eq!(rows[0].len(), 10, "Each row should have 10 cells");
    assert_eq!(rows[0][0].c, 'L');
    assert_eq!(rows[0][1].c, 'i');
    assert_eq!(rows[0][4].c, '1');

    // Check second row
    assert_eq!(rows[1][0].c, 'L');
    assert_eq!(rows[1][4].c, '2');

    // Check third row
    assert_eq!(rows[2][0].c, 'L');
    assert_eq!(rows[2][4].c, '3');
}

#[test]
fn test_iter_rows_with_attributes() {
    let mut screen = ScreenState::new(20, 3);
    screen.feed(b"\x1b[31mRed\r\n\x1b[32mGreen\r\n\x1b[34mBlue");

    let rows: Vec<&[Cell]> = screen.iter_rows().collect();

    // First row - red
    assert_eq!(rows[0][0].c, 'R');
    assert_eq!(rows[0][0].fg, Some(1));

    // Second row - green
    assert_eq!(rows[1][0].c, 'G');
    assert_eq!(rows[1][0].fg, Some(2));

    // Third row - blue
    assert_eq!(rows[2][0].c, 'B');
    assert_eq!(rows[2][0].fg, Some(4));
}

#[test]
fn test_iter_rows_enumeration() {
    let mut screen = ScreenState::new(10, 3);
    screen.feed(b"ABC\r\nDEF\r\nGHI");

    let mut chars_found = Vec::new();

    for (row_idx, row) in screen.iter_rows().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            if cell.c != ' ' {
                chars_found.push((row_idx, col_idx, cell.c));
            }
        }
    }

    assert_eq!(chars_found[0], (0, 0, 'A'));
    assert_eq!(chars_found[1], (0, 1, 'B'));
    assert_eq!(chars_found[2], (0, 2, 'C'));
    assert_eq!(chars_found[3], (1, 0, 'D'));
    assert_eq!(chars_found[4], (1, 1, 'E'));
    assert_eq!(chars_found[5], (1, 2, 'F'));
    assert_eq!(chars_found[6], (2, 0, 'G'));
    assert_eq!(chars_found[7], (2, 1, 'H'));
    assert_eq!(chars_found[8], (2, 2, 'I'));
}

#[test]
fn test_iter_row_valid() {
    let mut screen = ScreenState::new(10, 3);
    screen.feed(b"\x1b[31mRed\x1b[32mGreen\x1b[34mBlue");

    let cells: Vec<&Cell> = screen.iter_row(0).expect("Row 0 should exist").collect();

    assert_eq!(cells.len(), 10, "Should have 10 cells");
    assert_eq!(cells[0].c, 'R');
    assert_eq!(cells[0].fg, Some(1), "Red");
    assert_eq!(cells[3].c, 'G');
    assert_eq!(cells[3].fg, Some(2), "Green");
    assert_eq!(cells[8].c, 'B');
    assert_eq!(cells[8].fg, Some(4), "Blue");
}

#[test]
fn test_iter_row_out_of_bounds() {
    let screen = ScreenState::new(80, 24);

    assert!(screen.iter_row(0).is_some(), "Row 0 exists");
    assert!(screen.iter_row(23).is_some(), "Row 23 exists");
    assert!(screen.iter_row(24).is_none(), "Row 24 doesn't exist");
    assert!(screen.iter_row(100).is_none(), "Row 100 doesn't exist");
}

#[test]
fn test_iter_row_with_colors() {
    let mut screen = ScreenState::new(20, 5);

    // Create a row with alternating colors
    screen.feed(b"\x1b[31mA\x1b[32mB\x1b[33mC\x1b[34mD\x1b[35mE");

    let cells: Vec<&Cell> = screen.iter_row(0).expect("Row exists").collect();

    let colors: Vec<Option<u8>> = cells.iter().take(5).map(|c| c.fg).collect();
    assert_eq!(colors, vec![Some(1), Some(2), Some(3), Some(4), Some(5)]);
}

#[test]
fn test_snapshot_basic() {
    let mut screen = ScreenState::new(10, 3);
    screen.feed(b"Test");

    let snapshot = screen.snapshot();

    assert_eq!(snapshot.width, 10);
    assert_eq!(snapshot.height, 3);
    assert_eq!(snapshot.cells.len(), 3, "Should have 3 rows");
    assert_eq!(snapshot.cells[0].len(), 10, "Each row has 10 cells");
    assert_eq!(snapshot.cursor, (0, 4), "Cursor after 'Test'");
}

#[test]
fn test_snapshot_with_colors() {
    let mut screen = ScreenState::new(20, 5);
    screen.feed(b"\x1b[31mRed\x1b[0m Normal");

    let snapshot = screen.snapshot();

    // Verify red text
    assert_eq!(snapshot.cells[0][0].c, 'R');
    assert_eq!(snapshot.cells[0][0].fg, Some(1));

    // Verify normal text (reset)
    assert_eq!(snapshot.cells[0][4].c, 'N');
    assert_eq!(snapshot.cells[0][4].fg, None);
}

#[test]
fn test_snapshot_clone() {
    let mut screen = ScreenState::new(10, 3);
    screen.feed(b"Original");

    let snapshot1 = screen.snapshot();
    let snapshot2 = snapshot1.clone();

    assert_eq!(snapshot1, snapshot2, "Cloned snapshots should be equal");
    assert_eq!(snapshot2.cells[0][0].c, 'O');
}

#[test]
fn test_snapshot_comparison() {
    let mut screen1 = ScreenState::new(10, 3);
    let mut screen2 = ScreenState::new(10, 3);

    // Feed identical sequences
    screen1.feed(b"\x1b[31mTest");
    screen2.feed(b"\x1b[31mTest");

    let snap1 = screen1.snapshot();
    let snap2 = screen2.snapshot();

    assert_eq!(snap1, snap2, "Identical sequences should produce identical snapshots");
}

#[test]
fn test_snapshot_different() {
    let mut screen1 = ScreenState::new(10, 3);
    let mut screen2 = ScreenState::new(10, 3);

    screen1.feed(b"\x1b[31mRed");
    screen2.feed(b"\x1b[32mGreen");

    let snap1 = screen1.snapshot();
    let snap2 = screen2.snapshot();

    assert_ne!(snap1, snap2, "Different sequences should produce different snapshots");
}

#[test]
fn test_snapshot_cursor_tracking() {
    let mut screen = ScreenState::new(20, 10);

    // Initial position
    let snap1 = screen.snapshot();
    assert_eq!(snap1.cursor, (0, 0));

    // After text
    screen.feed(b"Hello");
    let snap2 = screen.snapshot();
    assert_eq!(snap2.cursor, (0, 5));

    // After cursor movement
    screen.feed(b"\x1b[5;10H");
    let snap3 = screen.snapshot();
    assert_eq!(snap3.cursor, (4, 9), "0-based from ESC[5;10H");
}

#[test]
fn test_verification_oracle_use_case() {
    // This test demonstrates the intended use case from issue #8:
    // Using ratatui-testlib as a verification oracle for another terminal emulator

    let test_sequence = b"\x1b[2J\x1b[H\x1b[31mRed\x1b[0m \x1b[32mGreen\x1b[0m";

    // Create the oracle
    let mut oracle = ScreenState::new(80, 24);
    oracle.feed(test_sequence);

    // Now in a real scenario, you'd feed the same sequence to your system-under-test
    // and compare using the snapshot API

    let snapshot = oracle.snapshot();

    // Verify dimensions
    assert_eq!(snapshot.width, 80);
    assert_eq!(snapshot.height, 24);

    // Verify cell-by-cell comparison is possible
    for row in 0..snapshot.height {
        for col in 0..snapshot.width {
            let cell = &snapshot.cells[row as usize][col as usize];
            // In real usage, compare cell.c, cell.fg, cell.bg, etc.
            // against your SUT's cell at the same position
            let _ = (cell.c, cell.fg, cell.bg, cell.bold, cell.italic, cell.underline);
        }
    }

    // Verify specific cells
    assert_eq!(snapshot.cells[0][0].c, 'R');
    assert_eq!(snapshot.cells[0][0].fg, Some(1), "Red");
    assert_eq!(snapshot.cells[0][4].c, 'G');
    assert_eq!(snapshot.cells[0][4].fg, Some(2), "Green");
}

#[test]
fn test_grid_iteration_pattern() {
    // Test the exact pattern shown in the issue proposal
    let mut screen = ScreenState::new(10, 3);
    screen.feed(b"\x1b[31mA\x1b[32mB\x1b[33mC");

    let mut cells_inspected = Vec::new();

    for row in 0..screen.rows() {
        for col in 0..screen.cols() {
            let cell = screen.get_cell(row, col).expect("Cell should exist");
            if cell.c != ' ' {
                cells_inspected.push((row, col, cell.c, cell.fg, cell.bg));
            }
        }
    }

    assert_eq!(cells_inspected.len(), 3);
    assert_eq!(cells_inspected[0], (0, 0, 'A', Some(1), None));
    assert_eq!(cells_inspected[1], (0, 1, 'B', Some(2), None));
    assert_eq!(cells_inspected[2], (0, 2, 'C', Some(3), None));
}

#[test]
fn test_snapshot_export_pattern() {
    // Test the exact pattern shown in the issue proposal
    let mut screen = ScreenState::new(10, 3);
    screen.feed(b"\x1b[31mTest");

    let snapshot = screen.snapshot();

    // Verify all fields are accessible
    assert_eq!(snapshot.width, 10);
    assert_eq!(snapshot.height, 3);
    assert_eq!(snapshot.cursor, (0, 4));

    // Verify cells are accessible
    assert_eq!(snapshot.cells[0][0].c, 'T');
    assert_eq!(snapshot.cells[0][0].fg, Some(1));
}

#[test]
fn test_cell_public_fields() {
    // Verify that Cell fields are public and accessible
    let cell = Cell {
        c: 'A',
        fg: Some(1),
        bg: Some(2),
        bold: true,
        italic: true,
        underline: true,
    };

    assert_eq!(cell.c, 'A');
    assert_eq!(cell.fg, Some(1));
    assert_eq!(cell.bg, Some(2));
    assert!(cell.bold);
    assert!(cell.italic);
    assert!(cell.underline);
}

#[test]
fn test_multiline_grid_inspection() {
    let mut screen = ScreenState::new(15, 5);
    screen.feed(b"Line 1\r\n");
    screen.feed(b"\x1b[31mLine 2\x1b[0m\r\n");
    screen.feed(b"\x1b[1mLine 3\x1b[0m");

    let snapshot = screen.snapshot();

    // Verify we can inspect all rows
    assert!(snapshot.cells[0][0].c == 'L');
    assert!(snapshot.cells[1][0].fg == Some(1)); // Red
    assert!(snapshot.cells[2][0].bold); // Bold
}

#[test]
fn test_iterator_and_snapshot_consistency() {
    let mut screen = ScreenState::new(10, 3);
    screen.feed(b"\x1b[31mTest");

    let snapshot = screen.snapshot();

    // Compare iterator and snapshot results
    for (row_idx, row) in screen.iter_rows().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            let snapshot_cell = &snapshot.cells[row_idx][col_idx];
            assert_eq!(cell, snapshot_cell, "Iterator and snapshot should match");
        }
    }
}
