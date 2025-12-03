//! Sixel graphics testing example.
//!
//! This example demonstrates how to test Sixel graphics in terminal applications:
//! - Detecting Sixel escape sequences in terminal output
//! - Tracking Sixel position and dimensions
//! - Validating that Sixel graphics appear within expected bounds
//! - Querying Sixel regions from screen state
//! - Using SixelCapture for comprehensive testing
//!
//! # Running This Example
//!
//! ```bash
//! cargo run --example sixel_test --features sixel
//! ```
//!
//! # What Are Sixel Graphics?
//!
//! Sixel is a bitmap graphics format supported by some terminal emulators
//! (like xterm with -ti vt340, mlterm, WezTerm). It allows applications to
//! display images directly in the terminal.
//!
//! # Example Use Case
//!
//! This is particularly useful for testing TUI applications that display
//! image previews (like file managers or image viewers) to ensure:
//! - Graphics appear in the correct location (e.g., within a preview panel)
//! - Graphics are properly cleared when switching files
//! - Graphics don't overflow their designated areas
//!
//! # Expected Output
//!
//! This example demonstrates:
//! 1. Creating a screen state with mock Sixel data
//! 2. Querying Sixel regions and their properties
//! 3. Validating Sixel bounds against preview areas
//! 4. Using SixelCapture for advanced validation
//! 5. Detecting when Sixel graphics change between states

use ratatui_testlib::{Result, ScreenState, SixelCapture};

fn main() -> Result<()> {
    println!("=== Sixel Graphics Testing Example ===\n");

    // Example 1: Basic Sixel region detection
    example_1_detect_sixel_regions()?;

    // Example 2: Bounds validation for preview areas
    example_2_validate_bounds()?;

    // Example 3: Using SixelCapture for advanced queries
    example_3_sixel_capture()?;

    // Example 4: Detecting Sixel state changes
    example_4_state_changes()?;

    // Example 5: Practical testing scenario
    example_5_practical_scenario()?;

    // Example 6: TuiTestHarness validation APIs (NEW)
    example_6_harness_validation_apis()?;

    // Example 7: dgx-pixels workflow simulation (NEW)
    example_7_dgx_pixels_workflow()?;

    // Example 8: Error handling and debugging (NEW)
    example_8_error_handling()?;

    println!("\n=== All Sixel Examples Completed Successfully ===");
    println!("\nSummary:");
    println!("  Examples 1-5: Core Sixel detection and validation");
    println!("  Example 6: TuiTestHarness convenience APIs");
    println!("  Example 7: dgx-pixels workflow testing");
    println!("  Example 8: Error handling and debugging");
    Ok(())
}

/// Example 1: Basic Sixel region detection
///
/// Demonstrates:
/// - Creating a screen state
/// - Feeding Sixel escape sequences
/// - Detecting Sixel regions
/// - Querying region properties
fn example_1_detect_sixel_regions() -> Result<()> {
    println!("--- Example 1: Detecting Sixel Regions ---");

    let mut screen = ScreenState::new(80, 24);
    println!("Created 80x24 screen state");

    // Simulate Sixel output at position (5, 10)
    // DCS starts Sixel: ESC P q
    // Raster attributes: " Pa ; Pb ; Ph ; Pv
    // Data follows, then: ESC \
    screen.feed(b"\x1b[5;10H"); // Move cursor to (5, 10) [1-based]
    screen.feed(b"\x1bPq"); // DCS - Start Sixel mode with 'q'
    screen.feed(b"\"1;1;100;50"); // Raster attributes: 100x50 pixels
    screen.feed(b"#0;2;100;100;100"); // Define color
    screen.feed(b"#0~"); // Some sixel data
    screen.feed(b"\x1b\\"); // String terminator (ST)

    println!("Fed Sixel escape sequence to screen");
    println!("  Position: (5, 10) [1-based in escape sequence]");
    println!("  Dimensions: 100x50 pixels (from raster attributes)");

    // Query Sixel regions
    let regions = screen.sixel_regions();
    println!("\nDetected {} Sixel region(s)", regions.len());

    for (i, region) in regions.iter().enumerate() {
        println!("\nRegion {}:", i);
        println!("  Start row: {} (0-based)", region.start_row);
        println!("  Start col: {} (0-based)", region.start_col);
        println!("  Width: {} pixels", region.width);
        println!("  Height: {} pixels", region.height);
        println!("  Data size: {} bytes", region.data.len());
    }

    // Check for Sixel at specific position
    if !regions.is_empty() {
        let region = &regions[0];
        let has_sixel = screen.has_sixel_at(region.start_row, region.start_col);
        println!("\nhas_sixel_at({}, {}): {}", region.start_row, region.start_col, has_sixel);
    }

    println!();
    Ok(())
}

/// Example 2: Bounds validation for preview areas
///
/// Demonstrates:
/// - Defining preview area boundaries
/// - Validating Sixel graphics stay within bounds
/// - Detecting out-of-bounds graphics
fn example_2_validate_bounds() -> Result<()> {
    println!("--- Example 2: Bounds Validation ---");

    // Create a scenario: file manager with preview panel
    let mut screen = ScreenState::new(120, 40);
    println!("Created 120x40 screen (simulating file manager layout)");

    // Define preview area: starts at row 5, col 70, size 45x30
    let preview_area = (5, 70, 45, 30);
    println!("\nPreview panel area:");
    println!("  Top-left: ({}, {})", preview_area.0, preview_area.1);
    println!("  Size: {}x{}", preview_area.2, preview_area.3);

    // Simulate Sixel within preview area
    screen.feed(b"\x1b[10;75H"); // Position within preview
    screen.feed(b"\x1bPq\"1;1;200;150#0~\x1b\\");

    println!("\nSimulated Sixel graphic within preview area");

    // Create SixelCapture and validate bounds
    let capture = SixelCapture::from_screen_state(&screen);
    println!("Created SixelCapture from screen state");
    println!("  Total sequences: {}", capture.sequences().len());

    // Check if all Sixel graphics are within the preview area
    match capture.assert_all_within(preview_area) {
        Ok(()) => {
            println!("\n✓ SUCCESS: All Sixel graphics are within the preview area!");
        }
        Err(e) => {
            println!("\n✗ VALIDATION FAILED: {}", e);
        }
    }

    // Query sequences by location
    let inside = capture.sequences_in_area(preview_area);
    let outside = capture.sequences_outside_area(preview_area);

    println!("\nSixel statistics:");
    println!("  Inside preview area: {}", inside.len());
    println!("  Outside preview area: {}", outside.len());

    println!();
    Ok(())
}

/// Example 3: Using SixelCapture for advanced queries
///
/// Demonstrates:
/// - Creating SixelCapture from screen state
/// - Querying sequences by area
/// - Checking overlap and containment
/// - Using SixelSequence methods
fn example_3_sixel_capture() -> Result<()> {
    println!("--- Example 3: SixelCapture Advanced Queries ---");

    let mut screen = ScreenState::new(100, 30);

    // Create multiple Sixel regions
    screen.feed(b"\x1b[5;5H\x1bPq\"1;1;80;60#0~\x1b\\"); // Region 1
    screen.feed(b"\x1b[15;50H\x1bPq\"1;1;100;80#0~\x1b\\"); // Region 2

    println!("Created screen with 2 Sixel regions:");
    println!("  Region 1: near (5, 5)");
    println!("  Region 2: near (15, 50)");

    let capture = SixelCapture::from_screen_state(&screen);
    println!("\nSixelCapture statistics:");
    println!("  Total sequences: {}", capture.sequences().len());
    println!("  Is empty: {}", capture.is_empty());

    // Define multiple areas of interest
    let left_panel = (0, 0, 30, 30);
    let right_panel = (0, 40, 60, 30);

    println!("\nQuerying by area:");
    println!(
        "  Left panel (0, 0, 30x30): {} sequences",
        capture.sequences_in_area(left_panel).len()
    );
    println!(
        "  Right panel (0, 40, 60x30): {} sequences",
        capture.sequences_in_area(right_panel).len()
    );

    // Inspect individual sequences
    for (i, seq) in capture.sequences().iter().enumerate() {
        println!("\nSequence {}:", i);
        println!("  Position: {:?}", seq.position);
        println!("  Bounds: {:?}", seq.bounds);
        println!("  Is within left panel: {}", seq.is_within(left_panel));
        println!("  Is within right panel: {}", seq.is_within(right_panel));
        println!("  Overlaps left panel: {}", seq.overlaps(left_panel));
        println!("  Overlaps right panel: {}", seq.overlaps(right_panel));
    }

    println!();
    Ok(())
}

/// Example 4: Detecting Sixel state changes
///
/// Demonstrates:
/// - Comparing Sixel states
/// - Detecting when graphics are cleared
/// - Verifying graphics changes between screens
fn example_4_state_changes() -> Result<()> {
    println!("--- Example 4: Detecting Sixel State Changes ---");

    // Initial state with Sixel
    let mut screen1 = ScreenState::new(80, 24);
    screen1.feed(b"\x1b[10;10H\x1bPq\"1;1;100;100#0~\x1b\\");
    let capture1 = SixelCapture::from_screen_state(&screen1);

    println!("Initial screen state:");
    println!("  Sixel sequences: {}", capture1.sequences().len());

    // Changed state (different Sixel)
    let mut screen2 = ScreenState::new(80, 24);
    screen2.feed(b"\x1b[10;10H\x1bPq\"1;1;200;200#0~\x1b\\");
    let capture2 = SixelCapture::from_screen_state(&screen2);

    println!("\nAfter changing image:");
    println!("  Sixel sequences: {}", capture2.sequences().len());
    println!("  State differs: {}", capture1.differs_from(&capture2));

    // Cleared state (no Sixel)
    let screen3 = ScreenState::new(80, 24);
    let capture3 = SixelCapture::from_screen_state(&screen3);

    println!("\nAfter clearing screen:");
    println!("  Sixel sequences: {}", capture3.sequences().len());
    println!("  State differs: {}", capture2.differs_from(&capture3));

    // Use case: verify graphics are cleared on screen transition
    if capture3.is_empty() {
        println!("\n✓ SUCCESS: Sixel graphics properly cleared on transition");
    } else {
        println!("\n✗ WARNING: Sixel graphics still present after clear");
    }

    println!();
    Ok(())
}

/// Example 5: Practical testing scenario
///
/// Demonstrates:
/// - Complete test scenario for an image viewer TUI
/// - Combining multiple validation checks
/// - Realistic error detection
fn example_5_practical_scenario() -> Result<()> {
    println!("--- Example 5: Practical Testing Scenario ---");
    println!("Scenario: Testing an image viewer TUI application\n");

    // Simulate image viewer layout: 100x40 terminal
    // Left sidebar: 0-29 columns
    // Preview area: 30-99 columns, rows 5-35
    let mut screen = ScreenState::new(100, 40);
    let preview_area = (5, 30, 70, 30);

    println!("Image viewer layout:");
    println!("  Terminal size: 100x40");
    println!("  Sidebar: columns 0-29");
    println!("  Preview: columns 30-99, rows 5-35");

    // Test 1: Display image in preview area
    println!("\nTest 1: Display image in preview area");
    screen.feed(b"\x1b[10;40H"); // Position in preview (row 10, col 40)
                                 // Image: 400x150 pixels = 50x25 cells, should fit in preview (70 wide x 30 tall)
    screen.feed(b"\x1bPq\"1;1;400;150#0~\x1b\\");

    let capture = SixelCapture::from_screen_state(&screen);
    match capture.assert_all_within(preview_area) {
        Ok(()) => println!("  ✓ Image correctly positioned in preview area"),
        Err(e) => println!("  ✗ Position validation failed: {}", e),
    }

    // Test 2: Verify no graphics in sidebar
    let sidebar_area = (0, 0, 30, 40);
    let sidebar_sequences = capture.sequences_in_area(sidebar_area);
    if sidebar_sequences.is_empty() {
        println!("  ✓ No graphics in sidebar (correct)");
    } else {
        println!("  ✗ WARNING: {} graphics in sidebar", sidebar_sequences.len());
    }

    // Test 3: Count graphics in preview
    let preview_sequences = capture.sequences_in_area(preview_area);
    println!("\nTest 2: Count graphics in preview");
    println!("  Graphics in preview: {}", preview_sequences.len());
    if preview_sequences.len() == 1 {
        println!("  ✓ Exactly one preview image (correct)");
    } else {
        println!("  ✗ WARNING: Expected 1 image, found {}", preview_sequences.len());
    }

    // Test 4: Verify graphics properties
    if let Some(seq) = preview_sequences.first() {
        println!("\nTest 3: Verify graphics properties");
        println!("  Position: ({}, {})", seq.position.0, seq.position.1);
        println!("  Bounds: {:?}", seq.bounds);

        let (_, _, width, height) = seq.bounds;
        if width > 0 && height > 0 {
            println!("  ✓ Graphics have valid dimensions");
        } else {
            println!("  ✗ WARNING: Invalid dimensions");
        }
    }

    println!("\n✓ Practical scenario tests completed");
    println!();
    Ok(())
}

/// Example 6: Using TuiTestHarness Sixel validation APIs
///
/// Demonstrates:
/// - TuiTestHarness convenience methods for Sixel validation
/// - dgx-pixels workflow testing
/// - Simplified validation API usage
fn example_6_harness_validation_apis() -> Result<()> {
    use ratatui_testlib::TuiTestHarness;

    println!("--- Example 6: TuiTestHarness Validation APIs ---");
    println!("Demonstrates the new harness-level Sixel validation methods\n");

    // Create harness (simulating dgx-pixels standard 80x24 terminal)
    let mut harness = TuiTestHarness::new(80, 24)?;
    println!("Created 80x24 test harness");

    // Initially no Sixels
    println!("\nInitial state:");
    println!("  Sixel count: {}", harness.sixel_count());
    assert_eq!(harness.sixel_count(), 0);

    // Feed Sixel in preview area
    println!("\nRendering Sixel in preview area (10, 50)...");
    harness.state_mut().feed(b"\x1b[10;50H");
    harness
        .state_mut()
        .feed(b"\x1bPq\"1;1;200;120#0;2;100;100;100#0~~@@\x1b\\");

    // Use sixel_count()
    println!("  Sixel count: {}", harness.sixel_count());
    assert_eq!(harness.sixel_count(), 1);
    println!("  ✓ Sixel detected");

    // Use sixel_at() to find specific Sixel
    println!("\nFinding Sixel at position (9, 49) [0-based]:");
    if let Some(region) = harness.sixel_at(9, 49) {
        println!("  Found Sixel:");
        println!("    Position: ({}, {})", region.start_row, region.start_col);
        println!("    Dimensions: {}x{} pixels", region.width, region.height);
        println!("  ✓ sixel_at() works correctly");
    } else {
        println!("  ✗ Sixel not found!");
    }

    // Use assert_preview_has_sixel() - dgx-pixels standard layout
    println!("\nValidating with dgx-pixels standard preview area:");
    match harness.assert_preview_has_sixel() {
        Ok(()) => println!("  ✓ Sixel found in standard preview area (5, 40, 35x15)"),
        Err(e) => println!("  ✗ Validation failed: {}", e),
    }

    // Use has_sixel_in_area() to check sidebar
    println!("\nChecking sidebar for graphics:");
    let sidebar = (0, 0, 40, 24);
    if harness.has_sixel_in_area(sidebar) {
        println!("  ✗ WARNING: Graphics detected in sidebar!");
    } else {
        println!("  ✓ No graphics in sidebar (correct)");
    }

    // Use assert_sixel_within_bounds() for custom areas
    println!("\nValidating bounds:");
    let full_screen = (0, 0, 80, 24);
    match harness.assert_sixel_within_bounds(full_screen) {
        Ok(()) => println!("  ✓ All Sixels within screen bounds"),
        Err(e) => println!("  ✗ Out of bounds: {}", e),
    }

    println!("\n✓ Harness validation APIs example completed");
    println!();
    Ok(())
}

/// Example 7: dgx-pixels workflow simulation
///
/// Demonstrates:
/// - Complete dgx-pixels file browsing workflow
/// - Screen transition verification
/// - Preview area validation
fn example_7_dgx_pixels_workflow() -> Result<()> {
    use ratatui_testlib::TuiTestHarness;

    println!("--- Example 7: dgx-pixels Workflow Simulation ---");
    println!("Simulates browsing through images in dgx-pixels\n");

    let mut harness = TuiTestHarness::new(80, 24)?;
    let preview_area = (5, 40, 35, 15);

    // Step 1: Display first image
    println!("Step 1: Display first image");
    harness
        .state_mut()
        .feed(b"\x1b[10;50H\x1bPq\"1;1;180;100#0~\x1b\\");

    assert_eq!(harness.sixel_count(), 1);
    assert!(harness.assert_preview_has_sixel().is_ok());
    println!("  ✓ First image displayed in preview");

    // Step 2: Navigate to next file (simulate screen clear + new image)
    println!("\nStep 2: Navigate to next image");
    use ratatui_testlib::ScreenState;
    *harness.state_mut() = ScreenState::new(80, 24);

    // New image
    harness
        .state_mut()
        .feed(b"\x1b[10;50H\x1bPq\"1;1;220;140#0~\x1b\\");

    assert_eq!(harness.sixel_count(), 1);
    assert!(harness.assert_preview_has_sixel().is_ok());
    println!("  ✓ Previous image cleared, new image displayed");

    // Step 3: Verify sidebar remains empty throughout
    println!("\nStep 3: Verify sidebar integrity");
    let sidebar = (0, 0, 40, 24);
    assert!(!harness.has_sixel_in_area(sidebar));
    println!("  ✓ Sidebar remains graphics-free");

    // Step 4: Test custom preview area for different terminal size
    println!("\nStep 4: Test larger terminal layout");
    let mut harness_large = TuiTestHarness::new(120, 40)?;
    let large_preview = (10, 60, 55, 25);

    harness_large
        .state_mut()
        .feed(b"\x1b[20;80H\x1bPq\"1;1;400;300#0~\x1b\\");

    match harness_large.assert_preview_has_sixel_in(large_preview) {
        Ok(()) => println!("  ✓ Large terminal preview validation works"),
        Err(e) => println!("  ✗ Validation failed: {}", e),
    }

    println!("\n✓ dgx-pixels workflow simulation completed");
    println!();
    Ok(())
}

/// Example 8: Error handling and debugging
///
/// Demonstrates:
/// - Helpful error messages
/// - Debugging Sixel position issues
/// - Validation failure scenarios
fn example_8_error_handling() -> Result<()> {
    use ratatui_testlib::TuiTestHarness;

    println!("--- Example 8: Error Handling and Debugging ---");
    println!("Demonstrates validation failures with helpful error messages\n");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Scenario 1: No Sixel in preview area
    println!("Scenario 1: No Sixel graphics present");
    match harness.assert_preview_has_sixel() {
        Ok(()) => println!("  Unexpected success"),
        Err(e) => {
            println!("  Expected error occurred:");
            println!("  {}", e);
            println!("  ✓ Error message provides helpful debugging info");
        }
    }

    // Scenario 2: Sixel outside expected area
    println!("\nScenario 2: Sixel outside preview area");
    harness
        .state_mut()
        .feed(b"\x1b[2;5H\x1bPq\"1;1;100;80#0~\x1b\\");

    match harness.assert_preview_has_sixel() {
        Ok(()) => println!("  Unexpected success"),
        Err(e) => {
            println!("  Expected error occurred:");
            println!("  {}", e);

            // Show actual Sixel position for debugging
            let regions = harness.sixel_regions();
            if !regions.is_empty() {
                println!("\n  Actual Sixel locations:");
                for (i, region) in regions.iter().enumerate() {
                    println!(
                        "    Sixel {}: row={}, col={}, size={}x{}",
                        i, region.start_row, region.start_col, region.width, region.height
                    );
                }
            }
            println!("  ✓ Error provides position information for debugging");
        }
    }

    // Scenario 3: Multiple Sixels, some out of bounds
    println!("\nScenario 3: Multiple Sixels with boundary violations");
    harness
        .state_mut()
        .feed(b"\x1b[10;50H\x1bPq\"1;1;150;100#0~\x1b\\"); // In preview
    harness
        .state_mut()
        .feed(b"\x1b[20;10H\x1bPq\"1;1;80;60#0~\x1b\\"); // Outside preview

    let preview_area = (5, 40, 35, 15);
    match harness.assert_sixel_within_bounds(preview_area) {
        Ok(()) => println!("  Unexpected success"),
        Err(e) => {
            println!("  Expected error occurred:");
            println!("  {}", e);
            println!("\n  Total Sixels: {}", harness.sixel_count());
            println!(
                "  In preview: {}",
                harness
                    .sixel_regions()
                    .iter()
                    .filter(|r| r.start_row >= 5 && r.start_col >= 40)
                    .count()
            );
            println!("  ✓ Can identify which Sixels violate bounds");
        }
    }

    println!("\n✓ Error handling examples completed");
    println!();
    Ok(())
}
