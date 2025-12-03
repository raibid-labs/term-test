//! Headless Bevy Runner example demonstrating in-process testing without PTY.
//!
//! This example shows how to use `HeadlessBevyRunner` for:
//! - Fast component/system testing
//! - CI/CD-friendly execution (no display server required)
//! - Deterministic frame-by-frame control
//!
//! Run with:
//! ```bash
//! cargo run --example headless_bevy --features bevy
//! ```

use bevy::prelude::*;
use ratatui_testlib::{HeadlessBevyRunner, Result};

// Example components
#[derive(Component)]
struct Counter(u32);

#[derive(Component)]
struct Health(i32);

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Player;

// Example systems
fn increment_counter(mut query: Query<&mut Counter>) {
    for mut counter in query.iter_mut() {
        counter.0 += 1;
    }
}

fn damage_enemies(mut query: Query<&mut Health, With<Enemy>>) {
    for mut health in query.iter_mut() {
        health.0 -= 10;
    }
}

fn main() -> Result<()> {
    println!("=== HeadlessBevyRunner Example ===\n");

    // Example 1: Basic component testing
    example_basic_component_testing()?;

    // Example 2: System execution testing
    example_system_execution()?;

    // Example 3: Filtered queries
    example_filtered_queries()?;

    // Example 4: Terminal output capture
    example_terminal_output()?;

    println!("\nAll examples completed successfully!");
    Ok(())
}

fn example_basic_component_testing() -> Result<()> {
    println!("Example 1: Basic Component Testing");
    println!("-----------------------------------");

    let mut runner = HeadlessBevyRunner::new()?;

    // Spawn some entities
    runner.world_mut().spawn(Counter(0));
    runner.world_mut().spawn(Counter(10));
    runner.world_mut().spawn(Counter(20));

    // Query and verify
    let counters = runner.query::<Counter>();
    println!("Spawned {} counters", counters.len());
    assert_eq!(counters.len(), 3);

    // Verify component count
    runner.assert_component_count::<Counter>(3)?;
    println!("Component count assertion passed");

    println!("✓ Basic component testing complete\n");
    Ok(())
}

fn example_system_execution() -> Result<()> {
    println!("Example 2: System Execution Testing");
    println!("------------------------------------");

    let mut runner = HeadlessBevyRunner::new()?;

    // Add system
    runner.app_mut().add_systems(Update, increment_counter);

    // Spawn entities
    runner.world_mut().spawn(Counter(0));
    runner.world_mut().spawn(Counter(100));

    println!("Initial counter values:");
    let counters = runner.query::<Counter>();
    for (i, counter) in counters.iter().enumerate() {
        println!("  Counter {}: {}", i, counter.0);
    }

    // Run 5 frames
    println!("\nRunning 5 frames...");
    runner.tick_n(5)?;

    println!("After 5 frames:");
    let counters = runner.query::<Counter>();
    for (i, counter) in counters.iter().enumerate() {
        println!("  Counter {}: {}", i, counter.0);
    }

    // Verify increments
    assert_eq!(counters[0].0, 5);
    assert_eq!(counters[1].0, 105);

    println!("✓ System execution testing complete\n");
    Ok(())
}

fn example_filtered_queries() -> Result<()> {
    println!("Example 3: Filtered Queries");
    println!("----------------------------");

    let mut runner = HeadlessBevyRunner::new()?;

    // Add damage system
    runner.app_mut().add_systems(Update, damage_enemies);

    // Spawn entities with different markers
    runner.world_mut().spawn((Health(100), Enemy));
    runner.world_mut().spawn((Health(150), Enemy));
    runner.world_mut().spawn((Health(200), Player));

    println!("Initial health values:");
    let all_health = runner.query::<Health>();
    println!("  Total entities with Health: {}", all_health.len());

    let enemy_health = runner.query_filtered::<Health, Enemy>();
    println!("  Enemies: {}", enemy_health.len());

    let player_health = runner.query_filtered::<Health, Player>();
    println!("  Players: {}", player_health.len());

    // Run damage
    println!("\nApplying damage for 3 frames...");
    runner.tick_n(3)?;

    println!("After damage:");
    let enemy_health = runner.query_filtered::<Health, Enemy>();
    for (i, health) in enemy_health.iter().enumerate() {
        println!("  Enemy {}: {} HP", i, health.0);
    }

    // Verify damage (30 damage over 3 frames)
    assert_eq!(enemy_health[0].0, 70);
    assert_eq!(enemy_health[1].0, 120);

    // Player should be unaffected
    let player_health = runner.query_filtered::<Health, Player>();
    assert_eq!(player_health[0].0, 200);
    println!("  Player: {} HP (unaffected)", player_health[0].0);

    println!("✓ Filtered queries complete\n");
    Ok(())
}

fn example_terminal_output() -> Result<()> {
    println!("Example 4: Terminal Output Capture");
    println!("-----------------------------------");

    let mut runner = HeadlessBevyRunner::new()?;

    // Manually feed terminal output
    runner.feed_terminal_output(b"Game Status:\n");
    runner.feed_terminal_output(b"\x1b[32mLevel 1 Complete!\x1b[0m\n");
    runner.feed_terminal_output(b"Score: 1000\n");

    // Capture screen state
    let screen = runner.screen();
    println!("Screen contents:");
    println!("{}", screen.contents());

    // Verify output
    assert!(screen.contains("Game Status"));
    assert!(screen.contains("Level 1 Complete"));
    assert!(screen.contains("Score: 1000"));

    // Create snapshot
    let snapshot = runner.snapshot();
    println!("Snapshot captured ({} bytes)", snapshot.len());

    println!("✓ Terminal output capture complete\n");
    Ok(())
}
