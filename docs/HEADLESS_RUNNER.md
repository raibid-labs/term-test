# HeadlessBevyRunner Guide

## Overview

`HeadlessBevyRunner` provides in-process, deterministic testing of Bevy-based TUI applications without the overhead of spawning a PTY. It's specifically designed for CI/CD environments and fast unit-style tests.

## When to Use Which Harness

### Use `HeadlessBevyRunner` for:

- ✅ **Component logic testing** - Test ECS systems and component state
- ✅ **CI/CD pipelines** - No display server required (GitHub Actions, Docker, etc.)
- ✅ **Fast unit tests** - No PTY spawning overhead
- ✅ **Deterministic testing** - Fixed timestep, frame-by-frame control
- ✅ **Snapshot testing** - Capture ECS and screen state for regression tests
- ✅ **System execution verification** - Test that Bevy systems run correctly

### Use `BevyTuiTestHarness` for:

- ✅ **End-to-end testing** - Full application testing with real terminal I/O
- ✅ **User interaction testing** - Keyboard input, mouse events, etc.
- ✅ **Terminal emulation testing** - Verify actual escape sequence handling
- ✅ **Integration testing** - Test complete user workflows
- ✅ **Real PTY behavior** - Terminal size changes, signal handling, etc.

## Comparison Matrix

| Feature | HeadlessBevyRunner | BevyTuiTestHarness |
|---------|-------------------|-------------------|
| **Execution Model** | In-process | PTY subprocess |
| **Speed** | Fast (no PTY overhead) | Slower (PTY + process spawn) |
| **Display Server** | Not required | Not required (with `headless` feature) |
| **Terminal I/O** | Simulated (manual feed) | Real terminal I/O |
| **Determinism** | High (fixed timestep) | Lower (async PTY timing) |
| **Use Case** | Unit/component tests | E2E integration tests |
| **Setup Complexity** | Simple (direct API) | Moderate (process spawning) |
| **CI/CD Suitability** | Excellent | Good |
| **Debug Experience** | Direct (in-process) | Harder (separate process) |

## Architecture

### HeadlessBevyRunner Architecture

```
┌─────────────────────────────────────┐
│    HeadlessBevyRunner               │
│                                     │
│  ┌────────────┐   ┌──────────────┐ │
│  │  Bevy App  │   │ ScreenState  │ │
│  │            │   │              │ │
│  │ MinimalPlugins  │  VT Parser │ │
│  │ ScheduleRunner  │  Terminal  │ │
│  └────────────┘   └──────────────┘ │
│                                     │
│  In-Process Testing                 │
└─────────────────────────────────────┘
```

### BevyTuiTestHarness Architecture

```
┌─────────────────────────────────────┐
│    BevyTuiTestHarness               │
│                                     │
│  ┌────────────┐   ┌──────────────┐ │
│  │  Bevy App  │   │ TuiTestHarness│ │
│  │            │   │              │ │
│  │ MinimalPlugins  │     PTY    │ │
│  │            │   │  Process   │ │
│  └────────────┘   └──────┬───────┘ │
│                           │         │
└───────────────────────────┼─────────┘
                            │
                    ┌───────▼────────┐
                    │  Your TUI App  │
                    │  (subprocess)  │
                    └────────────────┘
```

## Usage Examples

### Example 1: Component Testing

```rust
use ratatui_testlib::HeadlessBevyRunner;
use bevy::prelude::*;

#[derive(Component)]
struct Health(i32);

fn damage_system(mut query: Query<&mut Health>) {
    for mut health in query.iter_mut() {
        health.0 -= 10;
    }
}

#[test]
fn test_damage_system() -> ratatui_testlib::Result<()> {
    let mut runner = HeadlessBevyRunner::new()?;

    // Setup
    runner.app_mut().add_systems(Update, damage_system);
    runner.world_mut().spawn(Health(100));

    // Execute
    runner.tick_n(3)?;

    // Verify
    let health = runner.query::<Health>();
    assert_eq!(health[0].0, 70); // 100 - (10 * 3)

    Ok(())
}
```

### Example 2: Terminal Output Testing

```rust
use ratatui_testlib::HeadlessBevyRunner;

#[test]
fn test_terminal_output() -> ratatui_testlib::Result<()> {
    let mut runner = HeadlessBevyRunner::new()?;

    // Simulate terminal output from your render system
    runner.feed_terminal_output(b"\x1b[32mSuccess!\x1b[0m\n");

    // Verify output
    let screen = runner.screen();
    assert!(screen.contains("Success!"));

    // Create snapshot for regression testing
    let snapshot = runner.snapshot();
    insta::assert_snapshot!(snapshot);

    Ok(())
}
```

### Example 3: Filtered Queries

```rust
use ratatui_testlib::HeadlessBevyRunner;
use bevy::prelude::*;

#[derive(Component)]
struct Position(f32, f32);

#[derive(Component)]
struct Enemy;

#[test]
fn test_enemy_positions() -> ratatui_testlib::Result<()> {
    let mut runner = HeadlessBevyRunner::new()?;

    // Spawn entities
    runner.world_mut().spawn((Position(10.0, 20.0), Enemy));
    runner.world_mut().spawn(Position(5.0, 15.0)); // Not an enemy

    // Query only enemies
    let enemy_positions = runner.query_filtered::<Position, Enemy>();
    assert_eq!(enemy_positions.len(), 1);
    assert_eq!(enemy_positions[0].0, 10.0);

    Ok(())
}
```

## CI/CD Integration

### GitHub Actions Example

```yaml
# .github/workflows/ci.yml
headless-bevy-tests:
  name: Headless Bevy Tests
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable

    - name: Run headless tests (no display required)
      run: |
        unset DISPLAY
        cargo test --features bevy --lib
      env:
        DISPLAY: ""

    - name: Run headless example
      run: cargo run --example headless_bevy --features bevy
```

### Docker Example

```dockerfile
FROM rust:latest

WORKDIR /app
COPY . .

# No X11/Wayland required!
RUN cargo test --features bevy
RUN cargo run --example headless_bevy --features bevy
```

## bevy_ratatui Integration

When using `bevy_ratatui`, you can capture terminal output from Bevy systems:

```rust
use ratatui_testlib::HeadlessBevyRunner;
use bevy::prelude::*;
use bevy_ratatui::RatatuiContext;

fn my_render_system(mut ctx: RatatuiContext) {
    ctx.draw(|frame| {
        // Your rendering code here
    });
}

#[test]
#[cfg(feature = "bevy-ratatui")]
fn test_bevy_ratatui_rendering() -> ratatui_testlib::Result<()> {
    let mut runner = HeadlessBevyRunner::with_bevy_ratatui()?;
    runner.app_mut().add_systems(Update, my_render_system);

    // Run system
    runner.tick()?;

    // Note: Capturing Frame output requires custom adapter
    // See bevy_ratatui documentation for details

    Ok(())
}
```

## Snapshot Testing

`HeadlessBevyRunner` integrates seamlessly with `insta` for snapshot testing:

```rust
use ratatui_testlib::HeadlessBevyRunner;

#[test]
#[cfg(feature = "snapshot-insta")]
fn test_game_state_snapshot() -> ratatui_testlib::Result<()> {
    let mut runner = HeadlessBevyRunner::new()?;

    // Setup game state...
    runner.tick_n(10)?;

    // Capture snapshot
    let snapshot = runner.snapshot();
    insta::assert_snapshot!(snapshot);

    Ok(())
}
```

## Best Practices

### 1. Use HeadlessBevyRunner for Unit Tests

```rust
// Good: Fast, deterministic unit test
#[test]
fn test_component_logic() {
    let mut runner = HeadlessBevyRunner::new().unwrap();
    // Test component/system logic...
}
```

### 2. Use BevyTuiTestHarness for Integration Tests

```rust
// Good: Full E2E test with real terminal
#[test]
fn test_complete_user_flow() {
    let mut harness = BevyTuiTestHarness::new().unwrap();
    harness.spawn(cmd)?;
    harness.send_text("input\n")?;
    // Test complete workflow...
}
```

### 3. Prefer Filtered Queries

```rust
// Good: Specific query
let enemies = runner.query_filtered::<Health, Enemy>();

// Less ideal: Query all, then filter manually
let all_health = runner.query::<Health>();
let enemies: Vec<_> = all_health.iter()
    .filter(|h| /* check if enemy */)
    .collect();
```

### 4. Use Assertions for Clarity

```rust
// Good: Clear intent
runner.assert_component_exists::<Player>()?;
runner.assert_component_count::<Enemy>(5)?;

// Less ideal: Manual checks
assert!(runner.query::<Player>().len() > 0);
assert_eq!(runner.query::<Enemy>().len(), 5);
```

## Limitations

### HeadlessBevyRunner Limitations

1. **No real terminal I/O**: You must manually feed terminal output
2. **No PTY features**: No terminal size changes, signal handling, etc.
3. **Simulated rendering**: Requires manual screen state population
4. **bevy_ratatui capture**: Requires custom adapter to capture Frame output

### Workarounds

1. **Manual output**: Use `feed_terminal_output()` to simulate rendering
2. **Dimension control**: Use `with_dimensions()` for custom sizes
3. **Custom adapters**: Implement systems that write to ScreenState resource

## Future Enhancements

Planned features for future releases:

- [ ] Automatic `bevy_ratatui` Frame capture adapter
- [ ] Built-in support for custom rendering plugins
- [ ] ScreenState resource for direct Bevy system access
- [ ] Component snapshot helpers (see Issue #12)
- [ ] Event simulation helpers for Bevy Input events

## Related Documentation

- [BevyTuiTestHarness API](https://docs.rs/ratatui-testlib/latest/ratatui_testlib/struct.BevyTuiTestHarness.html)
- [HeadlessBevyRunner API](https://docs.rs/ratatui-testlib/latest/ratatui_testlib/struct.HeadlessBevyRunner.html)
- [Bevy Headless Example](https://github.com/bevyengine/bevy/blob/main/examples/app/headless.rs)
- [bevy_ratatui Documentation](https://docs.rs/bevy_ratatui)

## Support

For issues, questions, or contributions:
- GitHub Issues: https://github.com/raibid-labs/ratatui-testlib/issues
- Discussions: https://github.com/raibid-labs/ratatui-testlib/discussions
