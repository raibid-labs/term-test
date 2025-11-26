# Integration Notes for Enhanced Harness

## Overview

The enhanced `TuiTestHarness` is now complete with robust waiting mechanisms, cursor tracking, and a builder pattern. This document provides integration guidance for using the harness with other components of the mimic project.

## Quick Start

### Basic Usage

```rust
use term_test::{TuiTestHarness, Result};
use portable_pty::CommandBuilder;

#[test]
fn test_my_tui_app() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    let mut cmd = CommandBuilder::new("./my-app");
    harness.spawn(cmd)?;

    // Wait for app to be ready
    harness.wait_for_text("Ready")?;

    // Send input
    harness.send_text("hello\n")?;

    // Verify output
    harness.wait_for(|state| state.contains("Output: hello"))?;

    Ok(())
}
```

### Builder Pattern

```rust
let mut harness = TuiTestHarness::builder()
    .with_size(100, 30)
    .with_timeout(Duration::from_secs(10))
    .with_poll_interval(Duration::from_millis(50))
    .build()?;
```

## Key Methods

### Waiting for Conditions

#### `wait_for(condition: Fn(&ScreenState) -> bool)`
Most flexible method - wait for any condition:

```rust
harness.wait_for(|state| {
    state.contains("Status: OK") && !state.contains("Error")
})?;
```

#### `wait_for_text(text: &str)`
Convenience method for simple text matching:

```rust
harness.wait_for_text("Connected")?;
```

#### `wait_for_with_context(condition, description)`
Same as `wait_for` but with custom error messages:

```rust
harness.wait_for_with_context(
    |state| state.contains("Loaded"),
    "application to finish loading"
)?;
```

### Cursor Position Tracking

Required for Phase 3 Sixel verification:

```rust
let (row, col) = harness.cursor_position();
// or
let (row, col) = harness.get_cursor_position();
```

Both methods return 0-based coordinates.

### State Inspection

```rust
// Get full screen contents
let contents = harness.screen_contents();

// Access state directly
let state = harness.state();
if state.contains("Ready") {
    // ...
}

// Modify state (for testing without process)
harness.state_mut().feed(b"Test data");
```

## Integration with Phase 3 Sixel Testing

The harness provides the cursor position tracking required for Sixel tests:

```rust
#[test]
#[cfg(feature = "sixel")]
fn test_sixel_position() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Render Sixel image
    harness.send_text("\x1bPq<sixel_data>\x1b\\")?;

    // Wait for render to complete
    std::thread::sleep(Duration::from_millis(100));
    harness.update_state()?;

    // Verify cursor moved to correct position
    let (row, col) = harness.get_cursor_position();
    assert!(row >= expected_start_row);
    assert!(col >= expected_start_col);

    Ok(())
}
```

## Error Handling

### Timeout Errors

When a timeout occurs, detailed diagnostic information is printed to stderr:

```
=== Timeout waiting for: text 'Ready' ===
Waited: 5.002s (50 iterations)
Cursor position: row=0, col=15
Current screen state:
  0 | Welcome to the app
  1 | Loading...
  2 |
==========================================
```

### Best Practices

1. **Use appropriate timeouts**: Default is 5s, adjust based on your app's behavior
2. **Descriptive conditions**: Use `wait_for_with_context` for complex conditions
3. **Check screen state**: On test failures, examine the printed screen state
4. **Poll interval**: Lower intervals (50ms) for fast apps, higher (100-200ms) for slower apps

## Configuration Guidelines

### Choosing Buffer Size

- **4KB (default)**: Good for most applications
- **8KB**: For apps with lots of output
- **2KB**: For minimal apps or testing

```rust
.with_buffer_size(8192)
```

### Choosing Poll Interval

- **50ms**: Fast, responsive apps
- **100ms (default)**: General purpose
- **200ms**: Slower apps, reduced CPU usage

```rust
.with_poll_interval(Duration::from_millis(50))
```

### Choosing Timeout

- **5s (default)**: Most integration tests
- **10s**: Slow startup apps
- **2s**: Fast unit-style tests

```rust
.with_timeout(Duration::from_secs(10))
```

## Common Patterns

### Pattern 1: Wait for Initialization

```rust
harness.spawn(cmd)?;
harness.wait_for_text("Ready")?;
// Now app is ready for input
```

### Pattern 2: Send Input and Verify Output

```rust
harness.send_text("command\n")?;
harness.wait_for(|state| {
    state.contains("Result:")
})?;
```

### Pattern 3: Test Cursor Movement

```rust
harness.state_mut().feed(b"\x1b[10;5H"); // Move cursor
let (row, col) = harness.cursor_position();
assert_eq!(row, 9); // 0-based
assert_eq!(col, 4); // 0-based
```

### Pattern 4: Test with Multiple Conditions

```rust
harness.wait_for_with_context(
    |state| {
        let contents = state.contents();
        contents.contains("Status: OK") &&
        contents.lines().count() > 5
    },
    "status message and sufficient output"
)?;
```

## Testing Without Spawning Processes

For unit tests that don't need real processes:

```rust
#[test]
fn test_state_manipulation() {
    let mut harness = TuiTestHarness::new(80, 24).unwrap();

    // Feed data directly
    harness.state_mut().feed(b"Test output");

    // Verify
    assert!(harness.screen_contents().contains("Test"));
}
```

**Note**: Avoid calling `wait_for` methods without a running process, as they may block on PTY reads.

## Troubleshooting

### Tests Hang

**Symptom**: Test never completes
**Cause**: Calling `wait_for` without a running process
**Solution**: Either spawn a process first or use direct state manipulation

### Timeouts Too Frequent

**Symptom**: Tests timeout but should pass
**Cause**: Insufficient timeout or too slow poll interval
**Solution**: Increase timeout and/or decrease poll interval:

```rust
.with_timeout(Duration::from_secs(10))
.with_poll_interval(Duration::from_millis(50))
```

### Missing Output

**Symptom**: Expected text not found in screen state
**Cause**: Need to call `update_state()` or wait for condition
**Solution**: Use `wait_for_text()` or call `update_state()` explicitly

### Cursor Position Incorrect

**Symptom**: Cursor not at expected position
**Cause**: Escape sequence processing or timing issue
**Solution**: Call `update_state()` and add small delay after sending commands

## Performance Considerations

### Memory Usage

- Each `update_state()` allocates a buffer of size `buffer_size`
- Screen state maintains full terminal buffer (width × height)

### CPU Usage

- Polling-based waiting uses CPU during wait periods
- Poll interval controls CPU vs responsiveness trade-off
- For long-running tests, consider longer poll intervals

### I/O

- Non-blocking reads minimize blocking
- Multiple rapid reads may occur during `update_state()`

## Future Enhancements

Planned improvements for post-MVP:

1. **Async Support**: Async versions of wait_for methods
2. **Event-Based**: Replace polling with event notifications
3. **Pattern Matching**: Regex support in wait_for_text
4. **Metrics**: Track time spent waiting, iterations, etc.
5. **Snapshot Integration**: Automatic screen capture on timeout

## Dependencies

### Required from ScreenState

The harness requires these methods from `ScreenState`:

- `feed(&mut self, data: &[u8])` - Process raw PTY output
- `contents(&self) -> String` - Get full screen contents
- `contains(&self, text: &str) -> bool` - Text search
- `cursor_position(&self) -> (u16, u16)` - Get cursor position
- `debug_contents(&self) -> String` - Formatted output with line numbers
- `size(&self) -> (u16, u16)` - Get terminal dimensions

### Required from TestTerminal

The harness requires these methods from `TestTerminal`:

- `new(width: u16, height: u16) -> Result<Self>` - Create terminal
- `spawn(&mut self, cmd: CommandBuilder) -> Result<()>` - Spawn process
- `read(&mut self, buf: &mut [u8]) -> Result<usize>` - Non-blocking read
- `write(&mut self, data: &[u8]) -> Result<usize>` - Write to PTY
- `resize(&mut self, width: u16, height: u16) -> Result<()>` - Resize terminal
- `is_running(&mut self) -> bool` - Check process status
- `wait(&mut self) -> Result<ExitStatus>` - Wait for process exit

## Examples

See `examples/harness_demo.rs` for a comprehensive demonstration of all features.

Run with: `cargo run --example harness_demo`

## Support

For issues or questions about the harness:

1. Check this integration guide
2. Review `HARNESS_SUMMARY.md` for implementation details
3. Examine test cases in `src/harness.rs` (18 tests)
4. Run the demo example for working code samples
