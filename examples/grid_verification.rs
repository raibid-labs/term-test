//! Example demonstrating grid state verification API (Issue #8).
//!
//! This example shows how to use ratatui-testlib as a verification oracle
//! to compare terminal emulator implementations by inspecting the final
//! grid state after processing ANSI sequences.

use ratatui_testlib::ScreenState;

fn main() {
    println!("=== Grid State Verification Example ===\n");

    // Create a test screen
    let mut screen = ScreenState::new(80, 24);

    // Feed a complex ANSI sequence
    let test_sequence = b"\x1b[2J\x1b[H\x1b[31mRed\x1b[0m \x1b[32mGreen\x1b[0m \x1b[34mBlue\x1b[0m";
    screen.feed(test_sequence);

    println!("1. Dimension Accessors:");
    println!("   Screen size: {}x{}", screen.cols(), screen.rows());
    println!("   (Also available via size(): {:?})", screen.size());
    println!();

    println!("2. Cell-by-Cell Access:");
    println!("   Using get_cell(row, col):");
    for col in 0..15 {
        if let Some(cell) = screen.get_cell(0, col) {
            if cell.c != ' ' {
                println!(
                    "     Cell (0, {}): '{}' fg={:?} bg={:?} bold={} italic={} underline={}",
                    col, cell.c, cell.fg, cell.bg, cell.bold, cell.italic, cell.underline
                );
            }
        }
    }
    println!();

    println!("3. Row Iteration:");
    println!("   Using iter_rows():");
    for (row_idx, row) in screen.iter_rows().take(3).enumerate() {
        let non_space_cells: Vec<_> = row
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.c != ' ')
            .take(5)
            .collect();
        if !non_space_cells.is_empty() {
            println!("     Row {}: {} non-space cells", row_idx, non_space_cells.len());
            for (col_idx, cell) in non_space_cells {
                println!("       [{},{}] = '{}' (fg={:?})", row_idx, col_idx, cell.c, cell.fg);
            }
        }
    }
    println!();

    println!("4. Single Row Iteration:");
    println!("   Using iter_row(0):");
    if let Some(cells) = screen.iter_row(0) {
        let colored_chars: Vec<_> = cells
            .enumerate()
            .filter(|(_, cell)| cell.fg.is_some())
            .map(|(col, cell)| (col, cell.c, cell.fg))
            .collect();
        for (col, ch, color) in colored_chars {
            println!("     Column {}: '{}' with color {:?}", col, ch, color);
        }
    }
    println!();

    println!("5. Grid Snapshot:");
    println!("   Using snapshot() for complete state capture:");
    let snapshot = screen.snapshot();
    println!("   Snapshot dimensions: {}x{}", snapshot.width, snapshot.height);
    println!("   Cursor position: {:?}", snapshot.cursor);
    println!("   Total cells: {}", snapshot.cells.len() * snapshot.cells[0].len());

    // Verify specific cells in snapshot
    assert_eq!(snapshot.cells[0][0].c, 'R');
    assert_eq!(snapshot.cells[0][0].fg, Some(1)); // Red
    assert_eq!(snapshot.cells[0][4].c, 'G');
    assert_eq!(snapshot.cells[0][4].fg, Some(2)); // Green
    assert_eq!(snapshot.cells[0][10].c, 'B');
    assert_eq!(snapshot.cells[0][10].fg, Some(4)); // Blue
    println!("   Verification: Red='R', Green='G', Blue='B' all found with correct colors!");
    println!();

    println!("6. Complete Grid Iteration Pattern:");
    println!("   Iterating all cells (showing first 20 non-space):");
    let mut count = 0;
    'outer: for row in 0..screen.rows() {
        for col in 0..screen.cols() {
            if let Some(cell) = screen.get_cell(row, col) {
                if cell.c != ' ' && count < 20 {
                    println!(
                        "     ({:2},{:2}): '{}' fg={:?} bg={:?}",
                        row, col, cell.c, cell.fg, cell.bg
                    );
                    count += 1;
                    if count >= 20 {
                        break 'outer;
                    }
                }
            }
        }
    }
    println!();

    println!("7. Verification Oracle Use Case:");
    println!("   This API enables comparing terminal emulator implementations:");
    println!();
    println!("   // 1. Create oracle with ratatui-testlib");
    println!("   let mut oracle = ScreenState::new(80, 24);");
    println!("   oracle.feed(test_sequence);");
    println!();
    println!("   // 2. Feed same sequence to your system-under-test (SUT)");
    println!("   // let mut sut = YourTerminalEmulator::new(80, 24);");
    println!("   // sut.feed(test_sequence);");
    println!();
    println!("   // 3. Compare cell-by-cell");
    println!("   for row in 0..oracle.rows() {{");
    println!("       for col in 0..oracle.cols() {{");
    println!("           let oracle_cell = oracle.get_cell(row, col).unwrap();");
    println!("           // let sut_cell = sut.get_cell(row, col);");
    println!("           // assert_eq!(oracle_cell.c, sut_cell.c);");
    println!("           // assert_eq!(oracle_cell.fg, sut_cell.fg);");
    println!("           // assert_eq!(oracle_cell.bg, sut_cell.bg);");
    println!("       }}");
    println!("   }}");
    println!();

    println!("8. Snapshot Comparison:");
    println!("   Snapshots can be compared directly:");
    let mut screen2 = ScreenState::new(80, 24);
    screen2.feed(test_sequence);
    let snapshot2 = screen2.snapshot();

    if snapshot == snapshot2 {
        println!("   ✓ Snapshots match! Same sequence produces identical state.");
    } else {
        println!("   ✗ Snapshots differ!");
    }
    println!();

    println!("9. Advanced: Testing with Sixel Graphics:");
    let mut screen_sixel = ScreenState::new(80, 24);
    screen_sixel.feed(b"\x1b[5;10H"); // Position cursor
    screen_sixel.feed(b"\x1bPq"); // Start Sixel
    screen_sixel.feed(b"\"1;1;100;50"); // Raster: 100x50 pixels
    screen_sixel.feed(b"#0;2;100;100;100"); // Color definition
    screen_sixel.feed(b"#0~"); // Sixel data
    screen_sixel.feed(b"\x1b\\"); // End Sixel

    let sixel_regions = screen_sixel.sixel_regions();
    println!("   Sixel regions detected: {}", sixel_regions.len());
    for (i, region) in sixel_regions.iter().enumerate() {
        println!(
            "     Region {}: position=({},{}), size={}x{} pixels",
            i, region.start_row, region.start_col, region.width, region.height
        );
    }
    println!();

    println!("=== Example Complete ===");
    println!("\nKey API Features:");
    println!("  - rows() / cols() - dimension accessors");
    println!("  - get_cell(row, col) - individual cell access");
    println!("  - iter_rows() - iterate over all rows");
    println!("  - iter_row(row) - iterate cells in specific row");
    println!("  - snapshot() - capture complete grid state");
    println!("  - Cell struct with public fields (c, fg, bg, bold, italic, underline)");
    println!("\nUse Case: Verification oracle for terminal emulator testing");
}
