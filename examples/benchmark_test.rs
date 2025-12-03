//! Example demonstrating performance profiling and benchmarking for Bevy TUI applications.
//!
//! This example shows how to:
//! - Benchmark rendering performance over multiple frames
//! - Profile individual update cycles
//! - Assert FPS requirements for CI/CD performance gates
//! - Analyze percentile statistics (p50, p95, p99)
//!
//! Run this example with:
//! ```bash
//! cargo run --example benchmark_test --features bevy
//! ```

#[cfg(feature = "bevy")]
fn main() -> ratatui_testlib::Result<()> {
    use bevy::prelude::*;
    use ratatui_testlib::{BevyTuiTestHarness, HeadlessBevyRunner};
    use ratatui_testlib::bevy::bench::BenchmarkableHarness;

    println!("=== Performance Profiling Example ===\n");

    // ========================================================================
    // Example 1: Basic Benchmarking
    // ========================================================================
    println!("1. Basic Benchmarking with BevyTuiTestHarness\n");

    let mut harness = BevyTuiTestHarness::new()?;

    // Benchmark 1000 frames
    println!("Benchmarking 1000 frames...");
    let results = harness.benchmark_rendering(1000)?;

    println!("\n{}\n", results.summary());

    // Check if meets 60 FPS target
    if results.meets_fps_requirement(60.0) {
        println!("✓ Meets 60 FPS target!");
    } else {
        println!("✗ Does not meet 60 FPS target (avg: {:.2} FPS)", results.fps_avg);
    }

    // ========================================================================
    // Example 2: Single Frame Profiling
    // ========================================================================
    println!("\n2. Single Frame Profiling\n");

    let profile = harness.profile_update_cycle()?;
    println!("{}\n", profile.summary());

    // ========================================================================
    // Example 3: FPS Assertions (for CI/CD)
    // ========================================================================
    println!("3. FPS Assertions for CI/CD Performance Gates\n");

    // This would fail the test if FPS is below target
    match harness.assert_fps(1.0, 100) {
        Ok(results) => {
            println!("✓ FPS assertion passed: {:.2} FPS >= 1.0 FPS", results.fps_avg);
        }
        Err(e) => {
            println!("✗ FPS assertion failed: {}", e);
        }
    }

    // ========================================================================
    // Example 4: Benchmarking with Heavy Workload
    // ========================================================================
    println!("\n4. Benchmarking with Heavy Workload\n");

    #[derive(Component)]
    struct Position { x: f32, y: f32 }

    #[derive(Component)]
    struct Velocity { dx: f32, dy: f32 }

    // Physics simulation system
    fn update_positions(mut query: Query<(&mut Position, &Velocity)>) {
        for (mut pos, vel) in query.iter_mut() {
            pos.x += vel.dx;
            pos.y += vel.dy;

            // Simulate some computation
            pos.x = pos.x.sin() * 100.0;
            pos.y = pos.y.cos() * 100.0;
        }
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, update_positions);

    let mut heavy_harness = BevyTuiTestHarness::with_app(app)?;

    // Spawn 100 entities with physics components
    for i in 0..100 {
        heavy_harness.world_mut().spawn((
            Position { x: i as f32, y: i as f32 },
            Velocity { dx: 0.1, dy: 0.1 },
        ));
    }

    println!("Benchmarking with 100 entities and physics simulation...");
    let heavy_results = heavy_harness.benchmark_rendering(500)?;

    println!("\n{}\n", heavy_results.summary());

    // Analyze percentiles
    println!("Percentile Analysis:");
    println!("  p50 (median): {:.2}ms - 50% of frames are faster than this", heavy_results.p50_ms);
    println!("  p95: {:.2}ms - 95% of frames are faster than this", heavy_results.p95_ms);
    println!("  p99: {:.2}ms - 99% of frames are faster than this", heavy_results.p99_ms);
    println!("  max: {:.2}ms - worst-case frame time", heavy_results.max_frame_time_ms);

    // Check if p95 meets 60 FPS target
    if heavy_results.p95_ms < 16.67 {
        println!("\n✓ p95 meets 60 FPS target (95% of frames < 16.67ms)");
    } else {
        println!("\n✗ p95 does not meet 60 FPS target (95% of frames > 16.67ms)");
    }

    // ========================================================================
    // Example 5: Headless Runner Performance
    // ========================================================================
    println!("\n5. Headless Runner Performance (In-Process, No PTY)\n");

    let mut runner = HeadlessBevyRunner::new()?;

    // Spawn same workload
    for i in 0..100 {
        runner.world_mut().spawn((
            Position { x: i as f32, y: i as f32 },
            Velocity { dx: 0.1, dy: 0.1 },
        ));
    }

    // Add physics system
    runner.app_mut().add_systems(Update, update_positions);

    println!("Benchmarking HeadlessBevyRunner with same workload...");
    let headless_results = runner.benchmark_rendering(500)?;

    println!("\n{}\n", headless_results.summary());

    // Compare performance
    println!("Performance Comparison:");
    println!("  PTY Harness avg: {:.2}ms ({:.2} FPS)",
             heavy_results.avg_frame_time_ms, heavy_results.fps_avg);
    println!("  Headless avg: {:.2}ms ({:.2} FPS)",
             headless_results.avg_frame_time_ms, headless_results.fps_avg);

    let speedup = heavy_results.avg_frame_time_ms / headless_results.avg_frame_time_ms;
    println!("  Speedup: {:.2}x (headless is faster due to no PTY overhead)", speedup);

    // ========================================================================
    // Example 6: CI Performance Gate Example
    // ========================================================================
    println!("\n6. CI Performance Gate Example\n");

    let mut ci_harness = BevyTuiTestHarness::new()?;

    println!("Running CI performance gate...");
    match ci_harness.assert_fps(30.0, 100) {
        Ok(results) => {
            println!("✓ Performance gate passed!");
            println!("  Average FPS: {:.2}", results.fps_avg);
            println!("  p95 frame time: {:.2}ms", results.p95_ms);
            println!("  p99 frame time: {:.2}ms", results.p99_ms);
        }
        Err(e) => {
            println!("✗ Performance gate failed!");
            println!("  Error: {}", e);
            // In CI, this would cause the test to fail
        }
    }

    println!("\n=== Benchmark Example Complete ===");

    Ok(())
}

#[cfg(not(feature = "bevy"))]
fn main() {
    eprintln!("This example requires the 'bevy' feature flag.");
    eprintln!("Run with: cargo run --example benchmark_test --features bevy");
    std::process::exit(1);
}
