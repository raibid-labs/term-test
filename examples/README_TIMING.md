# Timing Examples

This directory contains examples demonstrating the timing and latency measurement infrastructure.

## Examples

### `timing_demo.rs`

Comprehensive demonstration of timing features:
- Basic timing recorder usage
- Latency profile stages
- Statistical analysis
- FPS budget calculations

Run with:
```bash
cargo run --example timing_demo
```

## Integration Tests

See `tests/timing_integration.rs` for complete integration tests showing:
- Input-to-render latency measurement
- Latency budget assertions
- FPS-based performance testing
- Custom timing events
- Statistical analysis across multiple operations
- Warm-up and timing reset patterns

Run integration tests:
```bash
cargo test --test timing_integration
```

## Quick Start

```rust
use ratatui_testlib::{TuiTestHarness, Result};
use std::time::Duration;

#[test]
fn test_input_latency() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;
    
    // Send input - timing is automatically recorded
    harness.send_text("hello")?;
    
    // Measure latency
    let latency = harness.measure_input_to_render_latency();
    println!("Latency: {:.2}ms", latency.unwrap().as_secs_f64() * 1000.0);
    
    // Assert 60 FPS performance
    harness.assert_render_budget(60.0)?;
    
    Ok(())
}
```

## Documentation

See [docs/TIMING.md](../docs/TIMING.md) for comprehensive documentation.
