//! Example: Detecting and validating multiple graphics protocols
//!
//! This example demonstrates the unified graphics protocol detection system
//! that supports Sixel, Kitty, and iTerm2 image protocols.
//!
//! Run with:
//! ```bash
//! cargo run --example graphics_detection --features sixel
//! ```

use ratatui_testlib::{
    graphics::{GraphicsCapture, GraphicsProtocol},
    ITerm2Region, KittyRegion, ScreenState, SixelRegion,
};

fn main() {
    println!("=== Graphics Protocol Detection Example ===\n");

    // Create a screen state
    let mut screen = ScreenState::new(80, 24);

    println!("1. Creating mock graphics regions for testing...\n");

    // Add a Sixel graphic at position (5, 10) - 100x60 pixels
    screen.sixel_regions_mut().push(SixelRegion {
        start_row: 5,
        start_col: 10,
        width: 100,
        height: 60,
        data: b"\x1bPq\"1;1;100;60#0~\x1b\\".to_vec(),
    });
    println!("   Added Sixel graphic at (5, 10), 100x60 pixels");

    // Add a Kitty graphic at position (10, 20) - 200x100 pixels
    screen.kitty_regions_mut().push(KittyRegion {
        start_row: 10,
        start_col: 20,
        width: 200,
        height: 100,
        data: b"Gw=200,h=100".to_vec(),
    });
    println!("   Added Kitty graphic at (10, 20), 200x100 pixels");

    // Add an iTerm2 inline image at position (15, 5) - 30x15 cells
    screen.iterm2_regions_mut().push(ITerm2Region {
        start_row: 15,
        start_col: 5,
        width: 30,
        height: 15,
        data: b"1337;File=width=30;height=15".to_vec(),
    });
    println!("   Added iTerm2 inline image at (15, 5), 30x15 cells\n");

    // Create a unified graphics capture
    let capture = GraphicsCapture::from_screen_state(&screen);

    println!("2. Analyzing detected graphics...\n");
    println!("   Total graphics detected: {}", capture.regions().len());
    println!("   - Sixel: {}", capture.count_by_protocol(GraphicsProtocol::Sixel));
    println!("   - Kitty: {}", capture.count_by_protocol(GraphicsProtocol::Kitty));
    println!("   - iTerm2: {}\n", capture.count_by_protocol(GraphicsProtocol::ITerm2));

    // Display details for each graphic
    println!("3. Graphics details:\n");
    for (i, region) in capture.regions().iter().enumerate() {
        let (row, col) = region.position;
        let (_, _, width, height) = region.bounds;
        println!(
            "   Graphic {}: {} at ({}, {}), size {}x{} cells",
            i + 1,
            region.protocol,
            row,
            col,
            width,
            height
        );
    }

    // Filter by protocol
    println!("\n4. Protocol-specific filtering:\n");

    let sixel_graphics = capture.by_protocol(GraphicsProtocol::Sixel);
    println!("   Sixel graphics: {}", sixel_graphics.len());
    for region in sixel_graphics {
        println!("      Position: {:?}, Bounds: {:?}", region.position, region.bounds);
    }

    let kitty_graphics = capture.by_protocol(GraphicsProtocol::Kitty);
    println!("   Kitty graphics: {}", kitty_graphics.len());
    for region in kitty_graphics {
        println!("      Position: {:?}, Bounds: {:?}", region.position, region.bounds);
    }

    let iterm2_graphics = capture.by_protocol(GraphicsProtocol::ITerm2);
    println!("   iTerm2 graphics: {}", iterm2_graphics.len());
    for region in iterm2_graphics {
        println!("      Position: {:?}, Bounds: {:?}", region.position, region.bounds);
    }

    // Area-based filtering
    println!("\n5. Area-based validation:\n");

    // Define a preview area (row=0, col=0, width=40, height=20)
    let preview_area = (0, 0, 40, 20);
    println!(
        "   Preview area: row={}, col={}, width={}, height={}",
        preview_area.0, preview_area.1, preview_area.2, preview_area.3
    );

    let in_area = capture.regions_in_area(preview_area);
    let outside_area = capture.regions_outside_area(preview_area);

    println!("   Graphics within preview area: {}", in_area.len());
    for region in in_area {
        println!("      {} at {:?}", region.protocol, region.position);
    }

    println!("   Graphics outside preview area: {}", outside_area.len());
    for region in outside_area {
        println!("      {} at {:?}", region.protocol, region.position);
    }

    // Validation example
    println!("\n6. Validation examples:\n");

    // Check if all graphics are within screen bounds
    let screen_bounds = (0, 0, 80, 24);
    match capture.assert_all_within(screen_bounds) {
        Ok(_) => println!("   ✓ All graphics are within screen bounds"),
        Err(e) => println!("   ✗ Graphics outside screen: {}", e),
    }

    // Check if specific protocols exist
    match capture.assert_protocol_exists(GraphicsProtocol::Sixel) {
        Ok(_) => println!("   ✓ Sixel graphics detected"),
        Err(e) => println!("   ✗ No Sixel graphics: {}", e),
    }

    match capture.assert_protocol_exists(GraphicsProtocol::Kitty) {
        Ok(_) => println!("   ✓ Kitty graphics detected"),
        Err(e) => println!("   ✗ No Kitty graphics: {}", e),
    }

    match capture.assert_protocol_exists(GraphicsProtocol::ITerm2) {
        Ok(_) => println!("   ✓ iTerm2 graphics detected"),
        Err(e) => println!("   ✗ No iTerm2 graphics: {}", e),
    }

    // Demonstrate overlaps check
    println!("\n7. Overlap detection:\n");

    for region in capture.regions() {
        let test_area = (0, 0, 20, 20);
        if region.overlaps(test_area) {
            println!(
                "   {} at {:?} overlaps with area {:?}",
                region.protocol, region.position, test_area
            );
        } else {
            println!(
                "   {} at {:?} does not overlap with area {:?}",
                region.protocol, region.position, test_area
            );
        }
    }

    println!("\n=== Example Complete ===");
    println!("\nKey takeaways:");
    println!("  • GraphicsCapture provides unified detection for all protocols");
    println!("  • Filter by protocol type with by_protocol()");
    println!("  • Validate positioning with regions_in_area() and assert_all_within()");
    println!("  • Check for specific protocols with assert_protocol_exists()");
    println!("  • Detect overlaps and bounds violations automatically");
}
