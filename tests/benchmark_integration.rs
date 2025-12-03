//! Integration tests for performance benchmarking utilities.
//!
//! These tests verify that the benchmarking API works correctly across
//! different scenarios and harness types.

#![cfg(feature = "bevy")]

use bevy::prelude::*;
use ratatui_testlib::{BevyTuiTestHarness, HeadlessBevyRunner};
use ratatui_testlib::bevy::bench::BenchmarkableHarness;

#[test]
fn test_benchmark_basic_harness() {
    let mut harness = BevyTuiTestHarness::new().unwrap();

    let results = harness.benchmark_rendering(50).unwrap();

    assert_eq!(results.iterations, 50);
    assert!(results.avg_frame_time_ms > 0.0);
    assert!(results.fps_avg > 0.0);
    assert!(results.min_frame_time_ms <= results.avg_frame_time_ms);
    assert!(results.avg_frame_time_ms <= results.max_frame_time_ms);
}

#[test]
fn test_benchmark_headless_runner() {
    let mut runner = HeadlessBevyRunner::new().unwrap();

    let results = runner.benchmark_rendering(50).unwrap();

    assert_eq!(results.iterations, 50);
    assert!(results.avg_frame_time_ms > 0.0);
    assert!(results.fps_avg > 0.0);
}

#[test]
fn test_profile_single_frame() {
    let mut harness = BevyTuiTestHarness::new().unwrap();

    let profile = harness.profile_update_cycle().unwrap();

    assert!(profile.duration_ms > 0.0);
    assert!(profile.fps_equivalent > 0.0);
    assert!((profile.fps_equivalent * profile.duration_ms - 1000.0).abs() < 1.0);
}

#[test]
fn test_fps_assertion_pass() {
    let mut harness = BevyTuiTestHarness::new().unwrap();

    // Should pass with low FPS requirement
    let result = harness.assert_fps(1.0, 50);
    assert!(result.is_ok());

    let results = result.unwrap();
    assert!(results.fps_avg >= 1.0);
}

#[test]
fn test_fps_assertion_fail() {
    let mut harness = BevyTuiTestHarness::new().unwrap();

    // Should fail with impossibly high FPS requirement
    let result = harness.assert_fps(1_000_000.0, 50);
    assert!(result.is_err());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("FPS requirement not met"));
}

#[test]
fn test_benchmark_with_ecs_systems() {
    #[derive(Component)]
    struct Counter(u32);

    fn increment_system(mut query: Query<'_, '_, &mut Counter>) {
        for mut counter in query.iter_mut() {
            counter.0 += 1;
        }
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, increment_system);

    let mut harness = BevyTuiTestHarness::with_app(app).unwrap();
    harness.world_mut().spawn(Counter(0));

    // Benchmark 100 frames
    let results = harness.benchmark_rendering(100).unwrap();

    assert_eq!(results.iterations, 100);

    // Verify system executed correctly
    let counters = harness.query::<Counter>();
    assert_eq!(counters[0].0, 100);
}

#[test]
fn test_benchmark_percentiles() {
    let mut harness = BevyTuiTestHarness::new().unwrap();

    let results = harness.benchmark_rendering(200).unwrap();

    // Verify percentile ordering
    assert!(results.min_frame_time_ms <= results.p50_ms);
    assert!(results.p50_ms <= results.p95_ms);
    assert!(results.p95_ms <= results.p99_ms);
    assert!(results.p99_ms <= results.max_frame_time_ms);

    // p50 should be close to average (for normal distributions)
    let diff_ratio = (results.p50_ms - results.avg_frame_time_ms).abs() / results.avg_frame_time_ms;
    // Allow up to 200% difference (can be skewed by outliers)
    assert!(diff_ratio < 2.0);
}

#[test]
fn test_benchmark_summary_output() {
    let mut harness = BevyTuiTestHarness::new().unwrap();

    let results = harness.benchmark_rendering(50).unwrap();
    let summary = results.summary();

    // Verify summary contains expected fields
    assert!(summary.contains("50 iterations"));
    assert!(summary.contains("Average FPS"));
    assert!(summary.contains("p50"));
    assert!(summary.contains("p95"));
    assert!(summary.contains("p99"));
    assert!(summary.contains("Total Duration"));
}

#[test]
fn test_profile_summary_output() {
    let mut harness = BevyTuiTestHarness::new().unwrap();

    let profile = harness.profile_update_cycle().unwrap();
    let summary = profile.summary();

    assert!(summary.contains("Frame Profile"));
    assert!(summary.contains("Duration"));
    assert!(summary.contains("FPS Equivalent"));
}

#[test]
fn test_60fps_target_validation() {
    let mut harness = BevyTuiTestHarness::new().unwrap();

    let results = harness.benchmark_rendering(100).unwrap();

    // Most systems should easily hit 60 FPS for empty Bevy updates
    // 16.67ms = 60 FPS threshold
    // Use p95 to allow for occasional spikes
    assert!(
        results.p95_ms < 16.67,
        "p95 frame time {:.2}ms exceeds 60 FPS target (16.67ms)",
        results.p95_ms
    );
}

#[test]
fn test_benchmark_consistency() {
    let mut harness = BevyTuiTestHarness::new().unwrap();

    // Run two consecutive benchmarks
    let results1 = harness.benchmark_rendering(100).unwrap();
    let results2 = harness.benchmark_rendering(100).unwrap();

    // Results should be reasonably consistent
    // Allow up to 300% variance (warmup effects, scheduler, etc.)
    let ratio = results1.avg_frame_time_ms / results2.avg_frame_time_ms;
    assert!(
        ratio > 0.3 && ratio < 3.0,
        "Inconsistent benchmark results: {:.2}ms vs {:.2}ms",
        results1.avg_frame_time_ms,
        results2.avg_frame_time_ms
    );
}

#[test]
fn test_headless_vs_pty_performance() {
    // Benchmark both harness types
    let mut pty_harness = BevyTuiTestHarness::new().unwrap();
    let pty_results = pty_harness.benchmark_rendering(100).unwrap();

    let mut headless_runner = HeadlessBevyRunner::new().unwrap();
    let headless_results = headless_runner.benchmark_rendering(100).unwrap();

    // Both should produce valid results
    assert!(pty_results.avg_frame_time_ms > 0.0);
    assert!(headless_results.avg_frame_time_ms > 0.0);

    // Headless should be at least as fast (or within 200% due to variance)
    let ratio = pty_results.avg_frame_time_ms / headless_results.avg_frame_time_ms;
    assert!(
        ratio > 0.5,
        "Headless runner unexpectedly slower: PTY {:.2}ms, Headless {:.2}ms",
        pty_results.avg_frame_time_ms,
        headless_results.avg_frame_time_ms
    );
}

#[test]
fn test_benchmark_with_workload() {
    #[derive(Component)]
    struct Position { x: f32, y: f32 }

    #[derive(Component)]
    struct Velocity { dx: f32, dy: f32 }

    fn physics_system(mut query: Query<'_, '_, (&mut Position, &Velocity)>) {
        for (mut pos, vel) in query.iter_mut() {
            pos.x += vel.dx;
            pos.y += vel.dy;

            // Add some computation to make workload realistic
            pos.x = pos.x.sin() * 100.0;
            pos.y = pos.y.cos() * 100.0;
        }
    }

    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_systems(Update, physics_system);

    let mut harness = BevyTuiTestHarness::with_app(app).unwrap();

    // Spawn 50 entities
    for i in 0..50 {
        harness.world_mut().spawn((
            Position { x: i as f32, y: i as f32 },
            Velocity { dx: 0.1, dy: 0.1 },
        ));
    }

    // Benchmark with realistic workload
    let results = harness.benchmark_rendering(100).unwrap();

    assert_eq!(results.iterations, 100);
    // With workload, should still be reasonably fast
    // Most systems should handle 50 entities at >1000 FPS
    assert!(results.fps_avg > 100.0, "FPS too low: {:.2}", results.fps_avg);
}

#[test]
fn test_meets_fps_requirement() {
    let mut harness = BevyTuiTestHarness::new().unwrap();
    let results = harness.benchmark_rendering(50).unwrap();

    // Should meet very low requirement
    assert!(results.meets_fps_requirement(1.0));

    // Should not meet impossibly high requirement
    assert!(!results.meets_fps_requirement(1_000_000.0));
}

#[test]
fn test_benchmark_zero_iterations() {
    let mut harness = BevyTuiTestHarness::new().unwrap();

    let results = harness.benchmark_rendering(0).unwrap();

    assert_eq!(results.iterations, 0);
    assert_eq!(results.total_duration_ms, 0.0);
    // FPS should be 0 for zero iterations
    assert_eq!(results.fps_avg, 0.0);
}
