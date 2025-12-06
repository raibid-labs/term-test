//! Integration tests for timing infrastructure with TuiTestHarness.
//!
//! These tests demonstrate how to measure and assert input-to-render latency
//! in real TUI applications.

use std::time::Duration;

use ratatui_testlib::{timing::TimingHooks, Result, TuiTestHarness};

#[test]
fn test_basic_input_latency_measurement() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Send input and measure latency
    harness.send_text("hello")?;

    // Verify latency was recorded
    let latency = harness.measure_input_to_render_latency();
    assert!(latency.is_some(), "Latency should be recorded");

    // Verify we can access the latency value
    if let Some(latency) = latency {
        // Note: default event delay is 50ms, so expect at least that
        // Should complete within 200ms in test environment
        assert!(latency < Duration::from_millis(200));
        println!("Input latency: {:.2}ms", latency.as_secs_f64() * 1000.0);
    }

    Ok(())
}

#[test]
fn test_assert_input_latency_within_budget() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    harness.send_text("test")?;

    // Assert latency is within a reasonable budget (200ms for test environment)
    // Note: includes 50ms default event delay
    harness.assert_input_latency_within(Duration::from_millis(200))?;

    Ok(())
}

#[test]
fn test_assert_render_budget_fps_target() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    harness.send_text("test")?;

    // Assert we could theoretically render at 5 FPS (very generous for tests)
    // This is 200ms budget per frame (accounts for 50ms default delay)
    harness.assert_render_budget(5.0)?;

    Ok(())
}

#[test]
fn test_multiple_input_events_timing() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Send multiple inputs and verify each is timed
    for i in 0..5 {
        harness.send_text(&format!("input{}", i))?;

        let latency = harness.measure_input_to_render_latency();
        assert!(latency.is_some(), "Latency should be recorded for input {}", i);
    }

    Ok(())
}

#[test]
fn test_timing_reset() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Record initial timing
    harness.send_text("first")?;
    let first_latency = harness.measure_input_to_render_latency();
    assert!(first_latency.is_some());

    // Reset timing
    harness.reset_timing();

    // Verify no latency data after reset
    let after_reset = harness.measure_input_to_render_latency();
    assert!(after_reset.is_none(), "Latency should be cleared after reset");

    // Record new timing
    harness.send_text("second")?;
    let second_latency = harness.measure_input_to_render_latency();
    assert!(second_latency.is_some(), "Should record new latency after reset");

    Ok(())
}

#[test]
fn test_timing_hooks_trait() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Use TimingHooks trait methods
    harness.record_event("custom_event_start");
    std::thread::sleep(Duration::from_millis(10));
    harness.record_event("custom_event_end");

    // Measure custom event latency
    let latency = harness.measure_latency("custom_event_start", "custom_event_end");
    assert!(latency.is_some(), "Should measure custom event latency");

    if let Some(latency) = latency {
        assert!(latency >= Duration::from_millis(10));
    }

    // Assert custom latency within budget
    harness.assert_latency_within(
        "custom_event_start",
        "custom_event_end",
        Duration::from_millis(50),
    )?;

    Ok(())
}

#[test]
fn test_latency_profile_access() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    harness.send_text("test")?;

    // Access latency profile for detailed analysis
    let profile = harness.latency_profile();

    // Verify profile has recorded data
    assert!(profile.input_to_render().is_some());
    assert!(profile.total_latency().is_some());

    Ok(())
}

#[test]
fn test_timing_with_key_events() -> Result<()> {
    use ratatui_testlib::KeyCode;

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Send key event and measure latency
    harness.send_key(KeyCode::Enter)?;

    let latency = harness.measure_input_to_render_latency();
    assert!(latency.is_some(), "Key event latency should be recorded");

    // Assert reasonable latency for key events (200ms includes 50ms default delay)
    harness.assert_input_latency_within(Duration::from_millis(200))?;

    Ok(())
}

#[test]
fn test_timing_stats_across_operations() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Perform multiple operations and collect timing stats
    for i in 0..10 {
        harness.send_text(&format!("op{}", i))?;
    }

    // Access timing recorder for statistics
    let timings = harness.get_timings();

    // Verify multiple samples were recorded
    assert!(timings.sample_count() >= 20); // input_sent + render_complete per operation

    // Calculate statistics for input→render latency
    let stats = timings.latency_stats("input_sent", "render_complete");
    assert!(stats.is_some(), "Should have latency statistics");

    if let Some(stats) = stats {
        println!("Latency stats:\n{}", stats.summary());
        assert_eq!(stats.count, 10);
    }

    Ok(())
}

#[test]
#[should_panic(expected = "Input latency exceeded budget")]
fn test_latency_budget_violation() {
    let mut harness = TuiTestHarness::new(80, 24).unwrap();

    harness.send_text("test").unwrap();

    // This should fail - 1 microsecond budget is impossibly tight
    harness
        .assert_input_latency_within(Duration::from_micros(1))
        .unwrap();
}

#[test]
fn test_timing_with_warm_up() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Warm-up phase (first operations may be slower)
    harness.send_text("warmup1")?;
    harness.send_text("warmup2")?;

    // Reset timing before actual measurements
    harness.reset_timing();

    // Now measure actual performance
    harness.send_text("measured")?;

    // Assert performance budget (5 FPS = 200ms, accounts for 50ms delay)
    harness.assert_render_budget(5.0)?;

    Ok(())
}

#[test]
fn test_timing_recorder_event_names() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Record some standard events
    harness.send_text("test")?;

    // Access timing recorder
    let timings = harness.get_timings();

    // Verify standard event names are present
    let event_names: Vec<&str> = timings.event_names().collect();
    assert!(event_names.contains(&"input_sent"), "Should have 'input_sent' event");
    assert!(event_names.contains(&"render_complete"), "Should have 'render_complete' event");

    Ok(())
}
