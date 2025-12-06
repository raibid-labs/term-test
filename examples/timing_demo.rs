//! Demonstrates input-to-render latency measurement and timing infrastructure.
//!
//! This example shows how to:
//! - Measure input-to-render latency
//! - Assert latency budgets for responsive UIs
//! - Use timing hooks for custom event tracking
//! - Collect latency statistics across multiple operations
//!
//! Run with: cargo run --example timing_demo

use std::time::Duration;

use ratatui_testlib::{
    timing::{fps_to_frame_budget, LatencyProfile, TimingHooks, TimingRecorder},
    Result,
};

fn main() -> Result<()> {
    println!("=== Timing Infrastructure Demo ===\n");

    // Demo 1: Basic Timing Recorder
    demo_timing_recorder()?;

    // Demo 2: Latency Profile
    demo_latency_profile()?;

    // Demo 3: Latency Statistics
    demo_latency_stats()?;

    // Demo 4: FPS Budget Calculation
    demo_fps_budgets()?;

    Ok(())
}

fn demo_timing_recorder() -> Result<()> {
    println!("--- Demo 1: Basic Timing Recorder ---");

    let mut recorder = TimingRecorder::new();

    // Simulate input→render pipeline
    recorder.record_event("input_received");
    std::thread::sleep(Duration::from_millis(3));

    recorder.record_event("state_updated");
    std::thread::sleep(Duration::from_millis(5));

    recorder.record_event("render_complete");
    std::thread::sleep(Duration::from_millis(2));

    recorder.record_event("frame_displayed");

    // Measure latencies
    if let Some(input_to_render) = recorder.measure_latency("input_received", "render_complete") {
        println!("  Input → Render: {:.2}ms", input_to_render.as_secs_f64() * 1000.0);
    }

    if let Some(total_latency) = recorder.measure_latency("input_received", "frame_displayed") {
        println!("  Total Latency: {:.2}ms", total_latency.as_secs_f64() * 1000.0);
    }

    // Assert latency is within budget (16.67ms for 60 FPS)
    recorder.assert_latency_within(
        "input_received",
        "render_complete",
        Duration::from_millis(16),
    )?;
    println!("  ✓ Latency within 60 FPS budget (16.67ms)");

    println!();
    Ok(())
}

fn demo_latency_profile() -> Result<()> {
    println!("--- Demo 2: Latency Profile ---");

    let mut profile = LatencyProfile::new();

    // Simulate input-to-frame pipeline
    profile.mark_input();
    std::thread::sleep(Duration::from_millis(2));

    profile.mark_render_start();
    std::thread::sleep(Duration::from_millis(8));

    profile.mark_render_end();
    std::thread::sleep(Duration::from_millis(3));

    profile.mark_frame_ready();

    // Analyze latency stages
    if let Some(input_to_render) = profile.input_to_render() {
        println!("  Input → Render: {:.2}ms", input_to_render.as_secs_f64() * 1000.0);
    }

    if let Some(render_duration) = profile.render_duration() {
        println!("  Render Duration: {:.2}ms", render_duration.as_secs_f64() * 1000.0);
    }

    if let Some(total) = profile.total_latency() {
        println!("  Total Latency: {:.2}ms", total.as_secs_f64() * 1000.0);
    }

    println!("\n{}", profile.summary());
    println!();
    Ok(())
}

fn demo_latency_stats() -> Result<()> {
    println!("--- Demo 3: Latency Statistics ---");

    let mut recorder = TimingRecorder::new();

    // Simulate multiple frame renders
    println!("  Simulating 100 frames...");
    for i in 0..100 {
        recorder.record_event("frame_start");

        // Simulate variable render times (8-12ms range)
        let render_time = 8 + (i % 5);
        std::thread::sleep(Duration::from_millis(render_time));

        recorder.record_event("frame_end");
    }

    // Calculate statistics
    if let Some(stats) = recorder.latency_stats("frame_start", "frame_end") {
        println!("\n{}", stats.summary());

        // Check if p95 meets 60 FPS target
        let fps_60_budget = fps_to_frame_budget(60.0);
        if stats.p95 < fps_60_budget {
            println!("  ✓ p95 meets 60 FPS target");
        } else {
            println!("  ✗ p95 exceeds 60 FPS target");
        }
    }

    println!();
    Ok(())
}

fn demo_fps_budgets() -> Result<()> {
    println!("--- Demo 4: FPS Budget Calculation ---");

    let fps_targets = vec![30.0, 60.0, 120.0, 144.0];

    println!("  FPS Target → Frame Budget:");
    for fps in fps_targets {
        let budget = fps_to_frame_budget(fps);
        println!("  {} FPS → {:.2}ms", fps, budget.as_secs_f64() * 1000.0);
    }

    // Simulate frame timing and check against budget
    println!("\n  Testing frame against 60 FPS budget:");
    let mut recorder = TimingRecorder::new();

    recorder.record_event("frame_start");
    std::thread::sleep(Duration::from_millis(10)); // Fast frame
    recorder.record_event("frame_end");

    let budget_60fps = fps_to_frame_budget(60.0);
    match recorder.assert_latency_within("frame_start", "frame_end", budget_60fps) {
        Ok(_) => println!("  ✓ Frame time within 60 FPS budget"),
        Err(e) => println!("  ✗ Frame time exceeded budget: {}", e),
    }

    println!();
    Ok(())
}
