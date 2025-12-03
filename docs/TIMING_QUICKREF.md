# Timing Infrastructure Quick Reference

## Basic Usage

```rust
use ratatui_testlib::TuiTestHarness;
use std::time::Duration;

let mut harness = TuiTestHarness::new(80, 24)?;

// Send input (automatic timing)
harness.send_text("hello")?;

// Measure latency
let latency = harness.measure_input_to_render_latency();

// Assert budget
harness.assert_render_budget(60.0)?; // 60 FPS
```

## API Cheat Sheet

### Measurement

```rust
// Input→render latency
harness.measure_input_to_render_latency() -> Option<Duration>

// Custom events
harness.measure_latency("start", "end") -> Option<Duration>
```

### Assertions

```rust
// Specific budget
harness.assert_input_latency_within(Duration::from_millis(16))?;

// FPS target
harness.assert_render_budget(60.0)?; // or 30.0, 120.0, etc.

// Custom events
harness.assert_latency_within("start", "end", budget)?;
```

### Statistics

```rust
let timings = harness.get_timings();
let stats = timings.latency_stats("input_sent", "render_complete")?;

println!("Mean: {:.2}ms", stats.mean.as_secs_f64() * 1000.0);
println!("p95: {:.2}ms", stats.p95.as_secs_f64() * 1000.0);
println!("p99: {:.2}ms", stats.p99.as_secs_f64() * 1000.0);
```

### Profiling

```rust
let profile = harness.latency_profile();

profile.input_to_render()     // Input → Render end
profile.render_duration()     // Render start → end
profile.total_latency()       // Input → Frame ready
profile.post_render_duration() // Render end → Frame ready

println!("{}", profile.summary());
```

### Custom Events

```rust
// Record events
harness.record_event("custom_start");
// ... work ...
harness.record_event("custom_end");

// Measure
let latency = harness.measure_latency("custom_start", "custom_end");

// Assert
harness.assert_latency_within("custom_start", "custom_end", budget)?;
```

### Reset

```rust
// Clear all timing data
harness.reset_timing();
```

## Common Patterns

### Warm-up Before Measurement

```rust
// Warm-up
harness.send_text("warmup")?;

// Reset timing
harness.reset_timing();

// Measure
harness.send_text("actual")?;
harness.assert_render_budget(60.0)?;
```

### Collect Statistics

```rust
// Multiple samples
for i in 0..100 {
    harness.send_text(&format!("input{}", i))?;
}

// Analyze
let timings = harness.get_timings();
let stats = timings.latency_stats("input_sent", "render_complete")?;
println!("{}", stats.summary());
```

### CI-Friendly Assertions

```rust
// Use generous budgets for CI variability
harness.assert_render_budget(5.0)?; // 200ms budget
```

## FPS Budgets

| FPS | Frame Time | Use Case |
|-----|------------|----------|
| 30 | 33.33ms | Minimum responsive |
| 60 | 16.67ms | Standard target |
| 120 | 8.33ms | High performance |
| 144 | 6.94ms | Gaming/premium |

```rust
use ratatui_testlib::timing::fps_to_frame_budget;

let budget = fps_to_frame_budget(60.0); // 16.67ms
```

## Standard Events

- `input_sent`: Recorded by `send_text()`/`send_key()`
- `render_complete`: Recorded after screen update

## Error Handling

```rust
match harness.assert_render_budget(60.0) {
    Ok(_) => println!("✓ Performance OK"),
    Err(e) => {
        eprintln!("✗ Performance issue: {}", e);
        // Log additional diagnostics
        if let Some(latency) = harness.measure_input_to_render_latency() {
            eprintln!("Actual latency: {:.2}ms", latency.as_secs_f64() * 1000.0);
        }
    }
}
```

## Best Practices

1. Always warm up before measurements
2. Use `reset_timing()` to clear warm-up data
3. Use generous budgets in CI (5-10 FPS)
4. Collect multiple samples for statistics
5. Check p95/p99 for consistency
6. Log timing data in failures

## See Also

- [Full Documentation](TIMING.md)
- [Examples](../examples/timing_demo.rs)
- [Integration Tests](../tests/timing_integration.rs)
