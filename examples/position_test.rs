//! Example demonstrating position and layout assertions.
//!
//! This example shows how to use the position assertion APIs to verify
//! that UI components are rendered in the correct locations.

use ratatui_testlib::{Axis, Rect, Result, TuiTestHarness};

fn main() -> Result<()> {
    println!("Position and Layout Assertion Example\n");

    // Create a test harness
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Simulate a complex TUI layout
    println!("1. Simulating complex TUI layout...");

    // Header at top
    harness.state_mut().feed(b"\x1b[1;1H");
    harness
        .state_mut()
        .feed(b"========== My Application ==========");

    // Sidebar on left (rows 3-22)
    harness.state_mut().feed(b"\x1b[3;1HFiles");
    harness.state_mut().feed(b"\x1b[5;1H- file1.txt");
    harness.state_mut().feed(b"\x1b[6;1H- file2.txt");
    harness.state_mut().feed(b"\x1b[7;1H- file3.txt");

    // Content area on right
    harness.state_mut().feed(b"\x1b[3;25HContent Area");
    harness
        .state_mut()
        .feed(b"\x1b[5;25HThis is the main content");
    harness
        .state_mut()
        .feed(b"\x1b[6;25Hwhere text is displayed.");

    // Tab bar at bottom
    harness.state_mut().feed(b"\x1b[23;1HTab 1 | Tab 2 | Tab 3");

    // Status bar at very bottom
    harness.state_mut().feed(b"\x1b[24;1HStatus: Ready");

    println!("   Layout created\n");

    // Define layout areas
    let header = Rect::new(0, 0, 80, 2);
    let sidebar = Rect::new(0, 2, 24, 20);
    let content = Rect::new(24, 2, 56, 20);
    let tab_bar = Rect::new(0, 22, 80, 1);
    let status_bar = Rect::new(0, 23, 80, 1);

    // Test 1: Assert text at specific positions
    println!("2. Testing text at specific positions...");
    harness.assert_text_at_position("Files", 2, 0)?;
    println!("   ✓ Found 'Files' at (2, 0)");

    harness.assert_text_at_position("Content Area", 2, 24)?;
    println!("   ✓ Found 'Content Area' at (2, 24)");

    harness.assert_text_at_position("Status: Ready", 23, 0)?;
    println!("   ✓ Found 'Status: Ready' at (23, 0)\n");

    // Test 2: Assert text within bounds
    println!("3. Testing text within bounds...");
    harness.assert_text_within_bounds("file1.txt", sidebar)?;
    println!("   ✓ Found 'file1.txt' in sidebar area");

    harness.assert_text_within_bounds("main content", content)?;
    println!("   ✓ Found 'main content' in content area");

    harness.assert_text_within_bounds("Tab 1", tab_bar)?;
    harness.assert_text_within_bounds("Tab 2", tab_bar)?;
    harness.assert_text_within_bounds("Tab 3", tab_bar)?;
    println!("   ✓ Found all tabs in tab bar area\n");

    // Test 3: Assert no overlap between sidebar and content
    println!("4. Testing no overlap between areas...");
    harness.assert_no_overlap(sidebar, content)?;
    println!("   ✓ Sidebar and content don't overlap");

    harness.assert_no_overlap(header, status_bar)?;
    println!("   ✓ Header and status bar don't overlap\n");

    // Test 4: Assert alignment
    println!("5. Testing alignment...");
    harness.assert_aligned(sidebar, content, Axis::Horizontal)?;
    println!("   ✓ Sidebar and content are horizontally aligned (same Y coordinate)");

    // Create some buttons to test vertical alignment
    let button1 = Rect::new(10, 20, 10, 1);
    let button2 = Rect::new(25, 20, 10, 1);
    harness.assert_aligned(button1, button2, Axis::Horizontal)?;
    println!("   ✓ Buttons are horizontally aligned\n");

    // Test 5: Demonstrate failure case (commented out to not fail)
    println!("6. Example failure case (commented out):");
    println!("   // This would fail:");
    println!("   // harness.assert_text_at_position(\"NotThere\", 0, 0)?;");
    println!("   // Error: Text mismatch at position (0, 0)");
    println!("   //   Expected: \"NotThere\"");
    println!("   //   Found:    \"========\"\n");

    // Print current screen state
    println!("7. Current screen state:");
    println!("{}", harness.state().debug_contents());

    println!("\n✅ All position assertions passed!");

    Ok(())
}
