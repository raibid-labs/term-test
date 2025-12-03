# Timing Infrastructure

This document describes the timing and latency measurement infrastructure in `ratatui-testlib`.

## Overview

The timing infrastructure enables measurement and assertion of input→render latency in TUI applications. This is critical for ensuring responsive user interfaces and validating performance requirements in CI/CD pipelines.

## Components

### TimingRecorder

Records timestamped events for latency analysis.

```rust
use ratatui_testlib::timing::TimingRecorder;
use std::time::Duration;

let mut recorder = TimingRecorder::new();

// Record events
recorder.record_event("input_received");
// ... processing ...
recorder.record_event("render_complete");

// Measure latency
let latency = recorder.measure_latency("input_received", "render_complete");

// Assert within budget
recorder.assert_latency_within(
    "input_received",
    "render_complete",
    Duration::from_millis(16) // 60 FPS budget
)?;
```

### LatencyProfile

Tracks specific stages of the input→render pipeline:

1. **Input**: User input received
2. **Render Start**: Application begins processing
3. **Render End**: Frame rendering completed
4. **Frame Ready**: Frame ready for display

```rust
use ratatui_testlib::timing::LatencyProfile;

let mut profile = LatencyProfile::new();

profile.mark_input();
// ... processing ...
profile.mark_render_start();
// ... rendering ...
profile.mark_render_end();
// ... finalization ...
profile.mark_frame_ready();

// Analyze stages
let input_to_render = profile.input_to_render().unwrap();
let render_duration = profile.render_duration().unwrap();
let total = profile.total_latency().unwrap();

println!("{}", profile.summary());
```

### LatencyStats

Statistical analysis of latency measurements:

```rust
use ratatui_testlib::timing::LatencyStats;
use std::time::Duration;

let samples = vec![
    Duration::from_millis(10),
    Duration::from_millis(15),
    Duration::from_millis(12),
    // ... more samples
];

let stats = LatencyStats::from_samples(samples);

println!("Mean: {:.2}ms", stats.mean.as_secs_f64() * 1000.0);
println!("p95: {:.2}ms", stats.p95.as_secs_f64() * 1000.0);
println!("p99: {:.2}ms", stats.p99.as_secs_f64() * 1000.0);
```

### TimingHooks Trait

Interface for integrating timing into test harnesses:

```rust
pub trait TimingHooks {
    fn record_event(&mut self, event_name: &str);
    fn measure_latency(&self, start_event: &str, end_event: &str) -> Option<Duration>;
    fn get_timings(&self) -> &TimingRecorder;
    fn assert_latency_within(&self, start: &str, end: &str, budget: Duration) -> Result<()>;
}
```

## Integration with TuiTestHarness

The `TuiTestHarness` automatically records timing events during input operations:

- `input_sent`: When `send_text()` or `send_key()` is called
- `render_complete`: When the screen state is updated

```rust
use ratatui_testlib::TuiTestHarness;
use std::time::Duration;

let mut harness = TuiTestHarness::new(80, 24)?;

// Send input - timing is automatically recorded
harness.send_text("hello")?;

// Measure input→render latency
let latency = harness.measure_input_to_render_latency();
println!("Latency: {:.2}ms", latency.unwrap().as_secs_f64() * 1000.0);

// Assert latency budget
harness.assert_input_latency_within(Duration::from_millis(16))?;

// Or use FPS-based assertion
harness.assert_render_budget(60.0)?; // 60 FPS = 16.67ms budget
```

## Common Use Cases

### 1. Assert 60 FPS Performance

```rust
use ratatui_testlib::TuiTestHarness;

let mut harness = TuiTestHarness::new(80, 24)?;

// Warm-up
harness.send_text("warmup")?;

// Reset timing before measurement
harness.reset_timing();

// Measure actual performance
harness.send_text("measured_input")?;

// Assert 60 FPS (16.67ms per frame)
harness.assert_render_budget(60.0)?;
```

### 2. Collect Latency Statistics

```rust
use ratatui_testlib::{TuiTestHarness, timing::TimingHooks};

let mut harness = TuiTestHarness::new(80, 24)?;

// Perform multiple operations
for i in 0..100 {
    harness.send_text(&format!("input{}", i))?;
}

// Get statistics
let timings = harness.get_timings();
let stats = timings.latency_stats("input_sent", "render_complete").unwrap();

println!("{}", stats.summary());

// Check p95 performance
assert!(stats.p95 < Duration::from_millis(16)); // 60 FPS p95
```

### 3. Custom Event Timing

```rust
use ratatui_testlib::{TuiTestHarness, timing::TimingHooks};
use std::time::Duration;

let mut harness = TuiTestHarness::new(80, 24)?;

// Record custom events
harness.record_event("custom_start");
// ... custom operation ...
harness.record_event("custom_end");

// Measure custom latency
let latency = harness.measure_latency("custom_start", "custom_end");
println!("Custom operation: {:.2}ms", latency.unwrap().as_secs_f64() * 1000.0);

// Assert custom budget
harness.assert_latency_within(
    "custom_start",
    "custom_end",
    Duration::from_millis(50)
)?;
```

### 4. Performance Regression Testing

```rust
use ratatui_testlib::TuiTestHarness;
use std::time::Duration;

#[test]
fn test_input_latency_regression() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Warm-up phase
    harness.send_text("warmup")?;
    harness.reset_timing();

    // Measure actual latency
    harness.send_text("test_input")?;

    // Assert latency hasn't regressed beyond acceptable threshold
    // For CI tests, we use a generous budget accounting for variability
    harness.assert_input_latency_within(Duration::from_millis(200))?;

    Ok(())
}
```

## FPS Budget Helper

The `fps_to_frame_budget()` function converts FPS targets to frame time budgets:

```rust
use ratatui_testlib::timing::fps_to_frame_budget;
use std::time::Duration;

let budget_60fps = fps_to_frame_budget(60.0);  // 16.67ms
let budget_30fps = fps_to_frame_budget(30.0);  // 33.33ms
let budget_120fps = fps_to_frame_budget(120.0); // 8.33ms

assert_eq!(budget_60fps, Duration::from_secs_f64(1.0 / 60.0));
```

## Snapshot Testing Support

When the `snapshot-insta` feature is enabled, `TimingRecorder` and `LatencyStats` can be serialized for snapshot testing:

```rust
#[cfg(feature = "snapshot-insta")]
use insta::assert_yaml_snapshot;
use ratatui_testlib::timing::TimingRecorder;

let mut recorder = TimingRecorder::new();
// ... record events ...

#[cfg(feature = "snapshot-insta")]
assert_yaml_snapshot!(recorder);
```

## Performance Considerations

### Default Event Delay

`TuiTestHarness` includes a 50ms default delay after sending key events to allow the application time to process input. This delay is included in latency measurements.

To customize the delay:

```rust
let mut harness = TuiTestHarness::builder()
    .with_size(80, 24)
    .build()?;

// Or disable delay (may cause race conditions)
// harness.with_event_delay(Duration::ZERO)
```

### Test Environment Variability

CI/CD environments can have higher latency variability due to resource contention. Use generous budgets in automated tests:

```rust
// Production target: 60 FPS (16.67ms)
// CI test budget: 5 FPS (200ms) - accounts for CI variability
harness.assert_render_budget(5.0)?;
```

### Warm-up

First operations may be slower due to cold caches. Always warm up before measurements:

```rust
// Warm-up
harness.send_text("warmup")?;

// Reset timing
harness.reset_timing();

// Now measure
harness.send_text("measured")?;
harness.assert_render_budget(60.0)?;
```

## Example: Complete Latency Test

```rust
use ratatui_testlib::{TuiTestHarness, timing::TimingHooks, Result};
use std::time::Duration;

#[test]
fn test_comprehensive_latency() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Warm-up phase
    for _ in 0..3 {
        harness.send_text("warmup")?;
    }

    // Reset timing before measurement
    harness.reset_timing();

    // Measure multiple operations
    for i in 0..10 {
        harness.send_text(&format!("input{}", i))?;
    }

    // Get statistics
    let timings = harness.get_timings();
    let stats = timings.latency_stats("input_sent", "render_complete")
        .expect("Should have timing stats");

    // Log statistics
    println!("{}", stats.summary());

    // Assert performance requirements
    assert!(stats.mean < Duration::from_millis(200), "Mean latency too high");
    assert!(stats.p95 < Duration::from_millis(200), "p95 latency too high");
    assert!(stats.p99 < Duration::from_millis(200), "p99 latency too high");

    Ok(())
}
```

## Best Practices

1. **Always warm up** before measurements to avoid cold cache effects
2. **Use `reset_timing()`** to clear warm-up data before actual measurements
3. **Use generous budgets in CI** to account for environment variability
4. **Collect statistics** across multiple samples for reliable measurements
5. **Check p95 and p99** to ensure consistent performance, not just averages
6. **Log timing data** in failed tests to aid debugging
7. **Document timing assumptions** in test comments

## Troubleshooting

### Timing assertions fail in CI but pass locally

**Solution**: Increase budget or use FPS-based assertions with conservative targets:

```rust
// Instead of:
harness.assert_input_latency_within(Duration::from_millis(16))?;

// Use:
harness.assert_render_budget(5.0)?; // Very generous for CI
```

### Inconsistent latency measurements

**Solution**: Ensure warm-up and reset timing before measurements:

```rust
// Warm-up
harness.send_text("warmup")?;
harness.reset_timing();

// Now measure
harness.send_text("actual")?;
```

### First operation always slow

**Solution**: This is expected (cold caches). Always warm up first.

## API Reference

See the [module documentation](../src/timing.rs) for complete API details.
