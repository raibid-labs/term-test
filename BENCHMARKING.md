# Performance Profiling and Benchmarking

This document describes the performance profiling and benchmarking utilities added to ratatui-testlib (Issue #13).

## Overview

The benchmarking module provides tools for measuring rendering performance, profiling update cycles, and ensuring TUI applications meet FPS targets (e.g., 60 FPS for Scarab).

## Features

- **Frame timing**: Measure individual frame durations
- **Percentile statistics**: p50, p95, p99 timing analysis
- **FPS validation**: Assert minimum FPS requirements
- **Profile reporting**: Detailed breakdown of update cycle performance
- **CI-ready**: Easy integration with CI/CD performance gates

## API

### Trait: `BenchmarkableHarness`

Both `BevyTuiTestHarness` and `HeadlessBevyRunner` implement the `BenchmarkableHarness` trait:

```rust
pub trait BenchmarkableHarness {
    fn benchmark_rendering(&mut self, iterations: usize) -> Result<BenchmarkResults>;
    fn profile_update_cycle(&mut self) -> Result<ProfileResults>;
    fn assert_fps(&mut self, min_fps: f64, iterations: usize) -> Result<BenchmarkResults>;
}
```

### Struct: `BenchmarkResults`

Contains comprehensive timing statistics:

```rust
pub struct BenchmarkResults {
    pub iterations: usize,
    pub total_duration_ms: f64,
    pub avg_frame_time_ms: f64,
    pub min_frame_time_ms: f64,
    pub max_frame_time_ms: f64,
    pub p50_ms: f64,  // Median
    pub p95_ms: f64,  // 95th percentile
    pub p99_ms: f64,  // 99th percentile
    pub fps_avg: f64,
}
```

Methods:
- `from_frame_times(Vec<f64>) -> Self`: Create from raw timing data
- `meets_fps_requirement(min_fps: f64) -> bool`: Check if meets FPS target
- `summary() -> String`: Formatted report for logging

### Struct: `ProfileResults`

Single-frame profiling data:

```rust
pub struct ProfileResults {
    pub duration_ms: f64,
    pub fps_equivalent: f64,
}
```

Methods:
- `from_duration(Duration) -> Self`: Create from duration
- `summary() -> String`: Formatted report

## Usage Examples

### Basic Benchmarking

```rust
use ratatui_testlib::BevyTuiTestHarness;
use ratatui_testlib::bevy::bench::BenchmarkableHarness;

let mut harness = BevyTuiTestHarness::new()?;

// Benchmark 1000 frames
let results = harness.benchmark_rendering(1000)?;

println!("Average FPS: {:.2}", results.fps_avg);
println!("p95 frame time: {:.2}ms", results.p95_ms);

// Check 60 FPS target
assert!(results.avg_frame_time_ms < 16.67);
```

### Single Frame Profiling

```rust
let profile = harness.profile_update_cycle()?;

println!("Frame took {:.2}ms", profile.duration_ms);
println!("FPS equivalent: {:.2}", profile.fps_equivalent);
```

### FPS Assertions (CI/CD)

```rust
// Assert minimum FPS requirement
// Will fail with detailed error if FPS is below target
harness.assert_fps(60.0, 1000)?;
```

### Percentile Analysis

```rust
let results = harness.benchmark_rendering(1000)?;

println!("Percentile Analysis:");
println!("  p50 (median): {:.2}ms - 50% of frames faster", results.p50_ms);
println!("  p95: {:.2}ms - 95% of frames faster", results.p95_ms);
println!("  p99: {:.2}ms - 99% of frames faster", results.p99_ms);

// Ensure 95% of frames meet 60 FPS (16.67ms)
assert!(results.p95_ms < 16.67);
```

### Benchmarking with Heavy Workload

```rust
use bevy::prelude::*;

#[derive(Component)]
struct Position { x: f32, y: f32 }

#[derive(Component)]
struct Velocity { dx: f32, dy: f32 }

fn physics_system(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut pos, vel) in query.iter_mut() {
        pos.x += vel.dx;
        pos.y += vel.dy;
    }
}

let mut app = App::new();
app.add_plugins(MinimalPlugins);
app.add_systems(Update, physics_system);

let mut harness = BevyTuiTestHarness::with_app(app)?;

// Spawn 100 entities
for i in 0..100 {
    harness.world_mut().spawn((
        Position { x: i as f32, y: i as f32 },
        Velocity { dx: 0.1, dy: 0.1 },
    ));
}

// Benchmark with realistic workload
let results = harness.benchmark_rendering(500)?;
println!("{}", results.summary());
```

## CI/CD Integration

### GitHub Actions Example

```yaml
- name: Run Performance Tests
  run: |
    cargo test --features bevy --release -- --test-threads=1 performance_gate

# In your test file:
#[test]
fn performance_gate() {
    let mut harness = BevyTuiTestHarness::new().unwrap();

    // Assert 30 FPS minimum for CI
    // This will fail the CI build if performance degrades
    harness.assert_fps(30.0, 1000).unwrap();
}
```

### Performance Regression Detection

```rust
// Baseline: Store previous benchmark results
const BASELINE_AVG_MS: f64 = 0.05;
const BASELINE_P95_MS: f64 = 0.08;

let results = harness.benchmark_rendering(1000)?;

// Detect regressions (>10% slower)
assert!(
    results.avg_frame_time_ms < BASELINE_AVG_MS * 1.1,
    "Average frame time regressed: {:.2}ms > {:.2}ms",
    results.avg_frame_time_ms,
    BASELINE_AVG_MS * 1.1
);

assert!(
    results.p95_ms < BASELINE_P95_MS * 1.1,
    "p95 frame time regressed: {:.2}ms > {:.2}ms",
    results.p95_ms,
    BASELINE_P95_MS * 1.1
);
```

## Comparison: BevyTuiTestHarness vs HeadlessBevyRunner

Both harnesses support benchmarking:

| Feature | BevyTuiTestHarness | HeadlessBevyRunner |
|---------|-------------------|-------------------|
| Benchmark API | ✓ | ✓ |
| Profile API | ✓ | ✓ |
| FPS assertions | ✓ | ✓ |
| PTY overhead | Yes | No |
| Speed | Slower | Faster |
| Use case | E2E tests | Unit tests, CI |

**Recommendation**: Use `HeadlessBevyRunner` for pure performance benchmarks to eliminate PTY overhead. Use `BevyTuiTestHarness` when you need to benchmark the full application including terminal I/O.

## Implementation Details

### Percentile Calculation

The library uses linear interpolation for percentile calculation:

```rust
fn percentile(sorted_data: &[f64], percentile: f64) -> f64 {
    let index = (percentile / 100.0) * (data.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        sorted_data[lower]
    } else {
        let weight = index - lower as f64;
        sorted_data[lower] * (1.0 - weight) + sorted_data[upper] * weight
    }
}
```

### Timing Accuracy

- Uses `std::time::Instant` for high-resolution timing
- Microsecond precision (displayed as milliseconds)
- Minimal measurement overhead (<1μs per sample)

### Memory Usage

- Pre-allocates vector for frame times
- Memory = iterations × 8 bytes (f64)
- 1000 iterations ≈ 8KB memory

## FPS Targets

Common FPS targets for TUI applications:

| Target | Frame Time | Use Case |
|--------|-----------|----------|
| 30 FPS | 33.33ms | Minimum acceptable |
| 60 FPS | 16.67ms | Smooth animation |
| 120 FPS | 8.33ms | Ultra-smooth |

**Note**: Most terminal emulators refresh at 60 Hz, so targeting >60 FPS may not provide visible benefits.

## Troubleshooting

### High Variance

If you see high variance (large difference between min and max):
- Run more iterations for stable averages
- Check for background processes
- Use release builds for accurate measurements

### Low FPS

If benchmarks show low FPS:
- Profile with `cargo flamegraph` to find hotspots
- Check for expensive system logic
- Consider optimizing ECS queries
- Reduce entity count or system complexity

### CI Flakiness

If CI benchmarks are flaky:
- Lower FPS requirements for CI (e.g., 30 FPS vs 60 FPS)
- Increase iteration count for more stable averages
- Use percentiles (p95, p99) instead of max for assertions
- Run benchmarks in release mode

## Examples

See `examples/benchmark_test.rs` for comprehensive examples of all benchmarking features.

Run the example:
```bash
cargo run --example benchmark_test --features bevy
```

## Testing

The benchmarking module includes comprehensive tests:
- Unit tests for percentile calculation
- Integration tests with Bevy systems
- Performance comparison tests
- FPS assertion tests

Run tests:
```bash
cargo test --features bevy bench
```

## Future Enhancements

Potential future additions:
- Criterion.rs integration for statistical analysis
- Flame graph generation from profiles
- JSON export of benchmark results
- Comparison with previous runs
- Automatic baseline updates
- Memory profiling
- System-level profiling (per-system timing)
