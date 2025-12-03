//! Component snapshot testing example for Bevy ECS.
//!
//! This example demonstrates how to use ratatui_testlib to create and compare
//! snapshots of Bevy ECS component state for regression testing:
//!
//! - Capturing component state as JSON snapshots
//! - Using insta for snapshot comparison
//! - Testing component changes over time
//! - Filtering components by marker types
//! - Best practices for ECS regression testing
//!
//! # Running This Example
//!
//! ```bash
//! cargo run --example bevy_component_snapshot --features bevy,snapshot-insta
//! ```
//!
//! # Why Snapshot ECS State?
//!
//! - Catch regressions in UI layout calculations
//! - Document expected component state with real examples
//! - Test system behavior over multiple frames
//! - Easier than manually asserting every field
//! - Quickly review changes with `cargo insta review`

use bevy::prelude::*;
use ratatui_testlib::{BevyTuiTestHarness, Result};
use serde::Serialize;

fn main() -> Result<()> {
    println!("=== Bevy Component Snapshot Testing Example ===\n");

    println!("Note: This example demonstrates snapshot testing patterns for Bevy ECS.");
    println!("In a real test file, you would use #[test] functions and");
    println!("the insta::assert_snapshot!() macro.\n");

    // Example 1: Basic component snapshot
    example_1_basic_component_snapshot()?;

    // Example 2: Snapshot multiple components
    example_2_multiple_components()?;

    // Example 3: Filtered component snapshots
    example_3_filtered_snapshots()?;

    // Example 4: Snapshot after system execution
    example_4_system_execution()?;

    // Example 5: Complex UI layout snapshot
    example_5_ui_layout()?;

    println!("\n=== All Component Snapshot Examples Completed ===");
    println!("\nTo use in real tests:");
    println!("1. Add #[derive(Serialize)] to your components");
    println!("2. Use harness.snapshot_components::<T>()");
    println!("3. Assert with insta::assert_json_snapshot!()");
    println!("4. Review snapshots: cargo insta review");

    Ok(())
}

/// Example 1: Basic component snapshot
///
/// Demonstrates:
/// - Capturing a simple component snapshot
/// - Serializing component state to JSON
/// - What information is captured
fn example_1_basic_component_snapshot() -> Result<()> {
    println!("--- Example 1: Basic Component Snapshot ---");

    #[derive(Component, Serialize, Clone, Debug)]
    struct Health {
        current: u32,
        max: u32,
    }

    let mut harness = BevyTuiTestHarness::new()?;

    // Spawn an entity with health
    harness.world_mut().spawn(Health { current: 100, max: 100 });

    // Capture snapshot
    let snapshots = harness.snapshot_components::<Health>();

    println!("Captured {} Health component(s)", snapshots.len());
    println!("Component type: {}", snapshots[0].component_type());
    println!("Entity ID: {}", snapshots[0].entity_id());
    println!("Data: current={}, max={}", snapshots[0].data().current, snapshots[0].data().max);

    println!("\nJSON representation:");
    let json = serde_json::to_string_pretty(&snapshots).unwrap();
    println!("{}", json);

    println!("\nIn a real test, you would use:");
    println!("  let snapshots = harness.snapshot_components::<Health>();");
    println!("  insta::assert_json_snapshot!(\"health_initial\", snapshots);");

    println!();
    Ok(())
}

/// Example 2: Snapshot multiple components
///
/// Demonstrates:
/// - Capturing snapshots of multiple entities
/// - Verifying component count and values
/// - Testing collections of components
fn example_2_multiple_components() -> Result<()> {
    println!("--- Example 2: Multiple Component Snapshots ---");

    #[derive(Component, Serialize, Clone, Debug)]
    struct Position {
        x: f32,
        y: f32,
    }

    let mut harness = BevyTuiTestHarness::new()?;

    // Spawn multiple entities with positions
    harness.world_mut().spawn(Position { x: 10.0, y: 20.0 });
    harness.world_mut().spawn(Position { x: 30.0, y: 40.0 });
    harness.world_mut().spawn(Position { x: 50.0, y: 60.0 });

    // Capture all position snapshots
    let snapshots = harness.snapshot_components::<Position>();

    println!("Captured {} Position components:", snapshots.len());
    for (i, snapshot) in snapshots.iter().enumerate() {
        println!(
            "  [{}] Entity {}: ({}, {})",
            i,
            snapshot.entity_id(),
            snapshot.data().x,
            snapshot.data().y
        );
    }

    println!("\nUse case: Verify UI element positions after layout calculation");
    println!("  insta::assert_json_snapshot!(\"ui_positions\", snapshots);");

    println!();
    Ok(())
}

/// Example 3: Filtered component snapshots
///
/// Demonstrates:
/// - Filtering components by marker types
/// - Testing specific subsets of entities
/// - Separating different entity types
fn example_3_filtered_snapshots() -> Result<()> {
    println!("--- Example 3: Filtered Component Snapshots ---");

    #[derive(Component, Serialize, Clone, Debug)]
    struct Size {
        width: u32,
        height: u32,
    }

    #[derive(Component)]
    struct Button;

    #[derive(Component)]
    struct Panel;

    let mut harness = BevyTuiTestHarness::new()?;

    // Spawn buttons
    harness
        .world_mut()
        .spawn((Size { width: 100, height: 30 }, Button));
    harness
        .world_mut()
        .spawn((Size { width: 120, height: 30 }, Button));

    // Spawn panels
    harness
        .world_mut()
        .spawn((Size { width: 800, height: 600 }, Panel));

    // Snapshot only button sizes
    let button_snapshots = harness.snapshot_components_filtered::<Size, Button>();
    println!("Button sizes ({} total):", button_snapshots.len());
    for snapshot in &button_snapshots {
        println!("  {}x{}", snapshot.data().width, snapshot.data().height);
    }

    // Snapshot only panel sizes
    let panel_snapshots = harness.snapshot_components_filtered::<Size, Panel>();
    println!("\nPanel sizes ({} total):", panel_snapshots.len());
    for snapshot in &panel_snapshots {
        println!("  {}x{}", snapshot.data().width, snapshot.data().height);
    }

    println!("\nIn a real test:");
    println!("  insta::assert_json_snapshot!(\"button_sizes\", button_snapshots);");
    println!("  insta::assert_json_snapshot!(\"panel_sizes\", panel_snapshots);");

    println!();
    Ok(())
}

/// Example 4: Snapshot after system execution
///
/// Demonstrates:
/// - Testing component changes over time
/// - Verifying system behavior
/// - Capturing state at multiple points
fn example_4_system_execution() -> Result<()> {
    println!("--- Example 4: Snapshot After System Execution ---");

    #[derive(Component, Serialize, Clone, Debug)]
    struct Position {
        x: i32,
        y: i32,
    }

    #[derive(Component, Serialize, Clone, Debug)]
    struct Velocity {
        dx: i32,
        dy: i32,
    }

    fn movement_system(mut query: Query<'_, '_, (&mut Position, &Velocity)>) {
        for (mut pos, vel) in query.iter_mut() {
            pos.x += vel.dx;
            pos.y += vel.dy;
        }
    }

    // Create app with system
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, movement_system);

    let mut harness = BevyTuiTestHarness::with_app(app)?;

    // Spawn entity
    harness
        .world_mut()
        .spawn((Position { x: 0, y: 0 }, Velocity { dx: 10, dy: 20 }));

    // Initial snapshot
    let initial = harness.snapshot_components::<Position>();
    println!("Initial position: ({}, {})", initial[0].data().x, initial[0].data().y);

    // Run 3 frames
    harness.update_bevy(3)?;

    // After update snapshot
    let after_frames = harness.snapshot_components::<Position>();
    println!("After 3 frames: ({}, {})", after_frames[0].data().x, after_frames[0].data().y);

    println!("\nIn a real test:");
    println!("  insta::assert_json_snapshot!(\"position_initial\", initial);");
    println!("  harness.update_bevy(3)?;");
    println!("  let after = harness.snapshot_components::<Position>();");
    println!("  insta::assert_json_snapshot!(\"position_after_3_frames\", after);");

    println!();
    Ok(())
}

/// Example 5: Complex UI layout snapshot
///
/// Demonstrates:
/// - Snapshotting complex component structures
/// - Testing complete UI layout state
/// - Documenting expected UI configuration
fn example_5_ui_layout() -> Result<()> {
    println!("--- Example 5: Complex UI Layout Snapshot ---");

    #[derive(Component, Serialize, Clone, Debug)]
    struct UiNode {
        position: (f32, f32),
        size: (f32, f32),
        visible: bool,
        z_index: i32,
        style: String,
    }

    #[derive(Component)]
    struct CommandPalette;

    let mut harness = BevyTuiTestHarness::new()?;

    // Spawn command palette UI
    harness.world_mut().spawn((
        UiNode {
            position: (100.0, 50.0),
            size: (600.0, 400.0),
            visible: true,
            z_index: 10,
            style: "modal".to_string(),
        },
        CommandPalette,
    ));

    // Capture layout
    let layout = harness.snapshot_components_filtered::<UiNode, CommandPalette>();

    println!("Command palette layout:");
    println!("  Position: ({}, {})", layout[0].data().position.0, layout[0].data().position.1);
    println!("  Size: {}x{}", layout[0].data().size.0, layout[0].data().size.1);
    println!("  Visible: {}", layout[0].data().visible);
    println!("  Z-index: {}", layout[0].data().z_index);
    println!("  Style: {}", layout[0].data().style);

    println!("\nJSON snapshot:");
    let json = serde_json::to_string_pretty(&layout).unwrap();
    println!("{}", json);

    println!("\nUse case: Verify UI layout calculations after opening command palette");
    println!("  insta::assert_json_snapshot!(\"command_palette_layout\", layout);");

    println!();
    Ok(())
}

// Example of how component snapshot tests would look in a real test file:
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use insta::assert_json_snapshot;
//     use ratatui_testlib::BevyTuiTestHarness;
//
//     #[derive(Component, Serialize, Clone)]
//     struct UiLayout {
//         position: (f32, f32),
//         size: (f32, f32),
//     }
//
//     #[test]
//     fn test_initial_layout() -> Result<()> {
//         let mut harness = BevyTuiTestHarness::new()?;
//
//         // Setup initial UI
//         harness.world_mut().spawn(UiLayout {
//             position: (0.0, 0.0),
//             size: (800.0, 600.0),
//         });
//
//         // Snapshot initial state
//         let snapshots = harness.snapshot_components::<UiLayout>();
//         assert_json_snapshot!("ui_layout_initial", snapshots);
//
//         Ok(())
//     }
//
//     #[test]
//     fn test_layout_after_resize() -> Result<()> {
//         let mut harness = BevyTuiTestHarness::new()?;
//
//         // ... trigger resize ...
//
//         // Snapshot after resize
//         let snapshots = harness.snapshot_components::<UiLayout>();
//         assert_json_snapshot!("ui_layout_after_resize", snapshots);
//
//         Ok(())
//     }
// }
