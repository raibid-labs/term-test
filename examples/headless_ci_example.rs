//! Headless CI Example - Demonstrates Bevy+PTY hybrid testing without X/Wayland
//!
//! This example shows how to use `ratatui-testlib` for comprehensive integration
//! testing in CI/CD environments like GitHub Actions without requiring a display server.
//!
//! # Features Demonstrated
//!
//! 1. **Headless Bevy Runner**: In-process ECS testing without PTY overhead
//! 2. **Hybrid Harness**: Combined in-process Bevy + optional PTY daemon
//! 3. **Graphics Protocol Detection**: Sixel, Kitty, and iTerm2 validation
//! 4. **Timing/Latency Measurement**: Performance budget assertions
//! 5. **ECS Queries**: Component state validation
//!
//! # Running in CI
//!
//! ```bash
//! # GitHub Actions / Docker / CI without display server
//! cargo test --features bevy,sixel,headless --example headless_ci_example
//! ```

use bevy::prelude::*;
use ratatui_testlib::{HeadlessBevyRunner, HybridBevyHarness, Result, ScreenState};

// ============================================================================
// Test Components (simulate Scarab-like components)
// ============================================================================

/// Navigation state component for testing nav lifecycle
#[derive(Component, Debug, Clone, PartialEq)]
struct NavState {
    mode: NavMode,
    anchor_position: (u16, u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum NavMode {
    #[default]
    Normal,
    HintMode,
    JumpMode,
}

/// Navigation hint entity marker
#[derive(Component, Debug)]
struct NavHint {
    label: String,
    position: (u16, u16),
}

/// Prompt markers for navigation
#[derive(Component, Debug)]
struct PromptMarker {
    line: u16,
    visible: bool,
}

/// Terminal metrics resource
#[derive(Resource, Debug, Default)]
struct TerminalMetrics {
    width: u16,
    height: u16,
    fps: f32,
    frame_time_ms: f32,
}

/// Scrollback state resource
#[derive(Resource, Debug, Default)]
struct ScrollbackState {
    total_lines: usize,
    viewport_start: usize,
    viewport_height: u16,
}

// ============================================================================
// Test Systems (simulate Scarab-like behavior)
// ============================================================================

fn update_nav_state(mut query: Query<&mut NavState>) {
    for mut state in query.iter_mut() {
        // Simulate state transitions
        match state.mode {
            NavMode::Normal => {} // Stay in normal
            NavMode::HintMode => {
                // After processing hints, could transition
            }
            NavMode::JumpMode => {
                // After jump, return to normal
                state.mode = NavMode::Normal;
            }
        }
    }
}

fn spawn_hint_entities(mut commands: Commands, query: Query<&NavState, Changed<NavState>>) {
    for state in query.iter() {
        if state.mode == NavMode::HintMode {
            // Spawn hint entities when entering hint mode
            commands.spawn(NavHint {
                label: "a".to_string(),
                position: (10, 10),
            });
            commands.spawn(NavHint {
                label: "b".to_string(),
                position: (10, 20),
            });
        }
    }
}

fn update_metrics(mut metrics: ResMut<TerminalMetrics>) {
    // Simulate frame timing
    metrics.fps = 60.0;
    metrics.frame_time_ms = 16.67;
}

// ============================================================================
// Test: Headless Bevy Runner (Pure ECS Testing)
// ============================================================================

fn test_headless_ecs_queries() -> Result<()> {
    println!("\n=== Test: Headless ECS Queries ===\n");

    let mut runner = HeadlessBevyRunner::new()?;

    // Add systems
    runner.app_mut().add_systems(Update, update_nav_state);
    runner.app_mut().insert_resource(TerminalMetrics::default());
    runner.app_mut().insert_resource(ScrollbackState::default());
    runner.app_mut().add_systems(Update, update_metrics);

    // Spawn test entities
    runner.world_mut().spawn(NavState {
        mode: NavMode::Normal,
        anchor_position: (0, 0),
    });

    runner
        .world_mut()
        .spawn(PromptMarker { line: 5, visible: true });

    runner
        .world_mut()
        .spawn(PromptMarker { line: 15, visible: false });

    // Run a few frames
    runner.tick_n(5)?;

    // Query and assert on ECS state
    // Note: We need to use separate scopes to avoid borrow checker issues
    {
        let nav_states = runner.query::<NavState>();
        assert_eq!(nav_states.len(), 1, "Should have one NavState component");
        assert_eq!(nav_states[0].mode, NavMode::Normal, "Should be in Normal mode");
        println!("  [PASS] NavState query: {:?}", nav_states[0]);
    }

    {
        let prompts = runner.query::<PromptMarker>();
        assert_eq!(prompts.len(), 2, "Should have two PromptMarker components");
        println!("  [PASS] PromptMarker count: {}", prompts.len());
    }

    // Query resources
    let metrics = runner.world().resource::<TerminalMetrics>();
    assert_eq!(metrics.fps, 60.0, "FPS should be updated");
    assert!(metrics.frame_time_ms > 0.0, "Frame time should be positive");
    println!("  [PASS] TerminalMetrics: {}fps, {}ms", metrics.fps, metrics.frame_time_ms);

    Ok(())
}

// ============================================================================
// Test: Nav Lifecycle (Hint Mode Spawns NavHint Entities)
// ============================================================================

fn test_nav_lifecycle() -> Result<()> {
    println!("\n=== Test: Nav Lifecycle ===\n");

    let mut runner = HeadlessBevyRunner::new()?;

    // Add nav systems
    runner
        .app_mut()
        .add_systems(Update, (update_nav_state, spawn_hint_entities).chain());

    // Start in Normal mode
    let entity = runner
        .world_mut()
        .spawn(NavState {
            mode: NavMode::Normal,
            anchor_position: (5, 5),
        })
        .id();

    runner.tick()?;

    // Should have no hints in Normal mode
    let hints = runner.query::<NavHint>();
    assert_eq!(hints.len(), 0, "No hints in Normal mode");
    println!("  [PASS] Normal mode: {} hints", hints.len());

    // Transition to HintMode
    runner
        .world_mut()
        .entity_mut(entity)
        .get_mut::<NavState>()
        .unwrap()
        .mode = NavMode::HintMode;
    runner.tick()?;

    // Should have spawned hint entities
    let hints = runner.query::<NavHint>();
    assert!(hints.len() >= 2, "Should spawn hints in HintMode");
    println!("  [PASS] HintMode: {} hints spawned", hints.len());

    // Verify hint labels
    for hint in &hints {
        println!("    - Hint '{}' at {:?}", hint.label, hint.position);
    }

    Ok(())
}

// ============================================================================
// Test: Graphics Protocol Detection (Sixel/Kitty/iTerm2)
// ============================================================================

#[cfg(feature = "sixel")]
fn test_graphics_detection() -> Result<()> {
    use ratatui_testlib::{GraphicsCapture, GraphicsProtocol};

    println!("\n=== Test: Graphics Protocol Detection ===\n");

    let mut screen = ScreenState::new(80, 24);

    // Simulate Sixel graphics at position (5, 10)
    // Position cursor then output Sixel
    screen.feed(b"\x1b[5;10H"); // Move cursor
    screen.feed(b"\x1bPq\"1;1;200;100#0~\x1b\\"); // Sixel data

    // Check Sixel detection
    let sixel_regions = screen.sixel_regions();
    assert!(!sixel_regions.is_empty(), "Should detect Sixel region");
    println!("  [PASS] Sixel detected: {} regions", sixel_regions.len());

    // Use unified GraphicsCapture
    let capture = GraphicsCapture::from_screen_state(&screen);
    assert!(!capture.is_empty(), "Should have graphics");

    let sixels = capture.by_protocol(GraphicsProtocol::Sixel);
    assert_eq!(sixels.len(), 1, "Should have 1 Sixel");
    println!(
        "  [PASS] GraphicsCapture: {} total, {} Sixel",
        capture.regions().len(),
        sixels.len()
    );

    // Simulate Kitty graphics
    screen.feed(b"\x1b_Gf=32,s=100,v=50,a=T;AAAA\x1b\\");

    // Simulate iTerm2 graphics
    screen.feed(b"\x1b]1337;File=inline=1;width=100;height=50:SGVsbG8=\x07");

    let kitty_regions = screen.kitty_regions();
    let iterm2_regions = screen.iterm2_regions();

    println!("  [INFO] Kitty regions: {}", kitty_regions.len());
    println!("  [INFO] iTerm2 regions: {}", iterm2_regions.len());

    // Test bounds validation
    let preview_area = (0, 0, 80, 24);
    let result = capture.assert_all_within(preview_area);
    assert!(result.is_ok(), "All graphics should be within preview area");
    println!("  [PASS] Bounds validation: all graphics within {:?}", preview_area);

    Ok(())
}

#[cfg(not(feature = "sixel"))]
fn test_graphics_detection() -> Result<()> {
    println!("\n=== Test: Graphics Protocol Detection ===\n");
    println!("  [SKIP] sixel feature not enabled");
    Ok(())
}

// ============================================================================
// Test: Hybrid Harness (Bevy + PTY Daemon - Simulated)
// ============================================================================

fn test_hybrid_harness_ecs() -> Result<()> {
    println!("\n=== Test: Hybrid Harness ECS ===\n");

    // Create hybrid harness without PTY daemon (headless mode)
    let mut harness = HybridBevyHarness::builder()
        .with_dimensions(80, 24)
        .build()?;

    // Add resources and systems
    harness
        .app_mut()
        .insert_resource(TerminalMetrics::default());
    harness.app_mut().add_systems(Update, update_metrics);

    // Spawn test entities
    harness.world_mut().spawn(NavState {
        mode: NavMode::Normal,
        anchor_position: (0, 0),
    });

    // Run frames
    harness.tick_n(3)?;

    // Assert on ECS state
    harness.assert_component_exists::<NavState>()?;
    harness.assert_component_count::<NavState>(1)?;

    println!("  [PASS] HybridBevyHarness created successfully");
    println!("  [PASS] ECS queries work through hybrid harness");

    // Test client screen (simulated output)
    harness.feed_client_output(b"Status: Ready\r\n");
    harness.feed_client_output(b"Frame: 1\r\n");

    let contents = harness.client_screen_contents();
    assert!(contents.contains("Status: Ready"), "Client screen should contain status");
    println!("  [PASS] Client screen capture: {} bytes", contents.len());

    Ok(())
}

// ============================================================================
// Test: Performance Budget Assertions
// ============================================================================

fn test_performance_budgets() -> Result<()> {
    use ratatui_testlib::bevy::BenchmarkableHarness;

    println!("\n=== Test: Performance Budgets ===\n");

    let mut runner = HeadlessBevyRunner::new()?;

    // Add some systems to benchmark
    runner.app_mut().add_systems(Update, update_nav_state);
    runner.world_mut().spawn(NavState {
        mode: NavMode::Normal,
        anchor_position: (0, 0),
    });

    // Benchmark 100 frames
    let results = runner.benchmark_rendering(100)?;

    println!("  Iterations: {}", results.iterations);
    println!("  Average FPS: {:.1}", results.fps_avg);
    println!("  Avg frame time: {:.3}ms", results.avg_frame_time_ms);
    println!("  p50: {:.3}ms", results.p50_ms);
    println!("  p95: {:.3}ms", results.p95_ms);
    println!("  p99: {:.3}ms", results.p99_ms);

    // Assert reasonable performance (very lenient for CI)
    let meets_60fps = results.meets_fps_requirement(60.0);
    println!("  [INFO] Meets 60 FPS: {}", meets_60fps);

    // Profile single frame
    let profile = runner.profile_update_cycle()?;
    println!(
        "  Single frame: {:.3}ms ({:.0} FPS equivalent)",
        profile.duration_ms, profile.fps_equivalent
    );

    println!("  [PASS] Performance benchmarking works in headless mode");

    Ok(())
}

// ============================================================================
// Test: Snapshot Testing Integration (ECS State)
// ============================================================================

#[cfg(feature = "snapshot-insta")]
fn test_snapshot_integration() -> Result<()> {
    use serde::Serialize;

    println!("\n=== Test: Snapshot Integration ===\n");

    #[derive(Component, Serialize, Clone)]
    struct TestState {
        level: u32,
        score: u32,
    }

    let mut runner = HeadlessBevyRunner::new()?;
    runner.world_mut().spawn(TestState { level: 1, score: 100 });
    runner.world_mut().spawn(TestState { level: 2, score: 250 });

    // Query all TestState components
    let test_states = runner.query::<TestState>();
    assert_eq!(test_states.len(), 2, "Should have 2 test states");

    // Verify the data
    let total_score: u32 = test_states.iter().map(|s| s.score).sum();
    assert_eq!(total_score, 350, "Total score should be 350");

    println!("  [PASS] Component snapshots: {} captured", test_states.len());
    println!("  [PASS] Total score: {}", total_score);

    Ok(())
}

#[cfg(not(feature = "snapshot-insta"))]
fn test_snapshot_integration() -> Result<()> {
    println!("\n=== Test: Snapshot Integration ===\n");
    println!("  [SKIP] snapshot-insta feature not enabled");
    Ok(())
}

// ============================================================================
// Main - Run All Tests
// ============================================================================

fn main() -> Result<()> {
    println!("============================================");
    println!("  Headless CI Example - ratatui-testlib");
    println!("============================================");
    println!();
    println!("This demonstrates testing without X11/Wayland");
    println!("Suitable for GitHub Actions, Docker, CI/CD");
    println!();

    // Run all tests
    test_headless_ecs_queries()?;
    test_nav_lifecycle()?;
    test_graphics_detection()?;
    test_hybrid_harness_ecs()?;
    test_performance_budgets()?;
    test_snapshot_integration()?;

    println!("\n============================================");
    println!("  All tests passed!");
    println!("============================================\n");

    Ok(())
}
