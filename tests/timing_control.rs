use std::time::{Duration, Instant};

use ratatui_testlib::{KeyCode, Result, TuiTestHarness};

#[test]
fn test_event_delay_default() {
    let harness = TuiTestHarness::new(80, 24).unwrap();
    // Default should be zero
    assert_eq!(harness.event_delay(), Duration::ZERO);
}

#[test]
fn test_set_event_delay() {
    let mut harness = TuiTestHarness::new(80, 24).unwrap();

    // Set a custom delay
    harness.set_event_delay(Duration::from_millis(100));
    assert_eq!(harness.event_delay(), Duration::from_millis(100));

    // Change it
    harness.set_event_delay(Duration::from_millis(200));
    assert_eq!(harness.event_delay(), Duration::from_millis(200));

    // Reset to zero
    harness.set_event_delay(Duration::ZERO);
    assert_eq!(harness.event_delay(), Duration::ZERO);
}

#[test]
fn test_advance_time() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // advance_time should succeed
    let start = Instant::now();
    harness.advance_time(Duration::from_millis(100))?;
    let elapsed = start.elapsed();

    // Should have actually waited (with some tolerance for scheduling)
    assert!(elapsed >= Duration::from_millis(95));
    assert!(elapsed <= Duration::from_millis(200));

    Ok(())
}

#[test]
fn test_press_key_repeat() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Test that press_key_repeat sends multiple keys
    let start = Instant::now();
    harness.press_key_repeat('a', 3, Duration::from_millis(50))?;
    let elapsed = start.elapsed();

    // Should take at least 3 * 50ms = 150ms
    // (plus the default 50ms delay after each key from send_key_event)
    // Total: ~300ms minimum
    assert!(elapsed >= Duration::from_millis(250));

    Ok(())
}

#[test]
fn test_event_delay_affects_timing() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // With custom delay
    harness.set_event_delay(Duration::from_millis(100));

    let start = Instant::now();
    // Send 3 characters
    harness.send_keys("abc")?;
    let elapsed = start.elapsed();

    // Each key should take 100ms, so 3 keys = ~300ms
    assert!(elapsed >= Duration::from_millis(250));

    Ok(())
}

#[test]
fn test_press_key_repeat_with_zero_interval() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Should work with zero interval (only default key delay applies)
    let start = Instant::now();
    harness.press_key_repeat('x', 2, Duration::ZERO)?;
    let elapsed = start.elapsed();

    // Should still take time due to the default 50ms delay in send_key_event
    // 2 keys * 50ms = ~100ms minimum
    assert!(elapsed >= Duration::from_millis(80));

    Ok(())
}

#[test]
fn test_timing_combination() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Test combining different timing methods
    harness.set_event_delay(Duration::from_millis(50));

    let start = Instant::now();

    // Send a key (50ms delay)
    harness.send_key(KeyCode::Char('a'))?;

    // Advance time (100ms)
    harness.advance_time(Duration::from_millis(100))?;

    // Send repeated keys (2 * 50ms interval + 2 * 50ms event delay = 200ms)
    harness.press_key_repeat('b', 2, Duration::from_millis(50))?;

    let elapsed = start.elapsed();

    // Total: 50 + 100 + 200 = 350ms minimum
    assert!(elapsed >= Duration::from_millis(300));

    Ok(())
}

#[test]
fn test_zero_event_delay_uses_default() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // With zero delay, should still use the default 50ms
    harness.set_event_delay(Duration::ZERO);

    let start = Instant::now();
    harness.send_key(KeyCode::Char('a'))?;
    let elapsed = start.elapsed();

    // Should have used default 50ms delay
    assert!(elapsed >= Duration::from_millis(40));

    Ok(())
}

#[test]
fn test_multiple_advance_time_calls() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let start = Instant::now();

    // Multiple calls should be cumulative
    harness.advance_time(Duration::from_millis(50))?;
    harness.advance_time(Duration::from_millis(50))?;
    harness.advance_time(Duration::from_millis(50))?;

    let elapsed = start.elapsed();

    // Total: 150ms minimum
    assert!(elapsed >= Duration::from_millis(140));

    Ok(())
}
