# Testing Guide for term-test

This document describes the test architecture and patterns used in term-test.

## Test Organization

### Unit Tests (56 tests)

Unit tests are colocated with the source code in `src/` modules using Rust's built-in `#[cfg(test)]` pattern.

#### PTY Module Tests (19 tests)
Location: `src/pty.rs`

Tests cover:
- Terminal creation and validation
- Process spawning and lifecycle
- Reading and writing to PTY
- Timeout handling
- Error conditions (invalid dimensions, no process running, etc.)
- Process exit status tracking

Key test patterns:
```rust
#[test]
fn test_spawn_process() {
    let mut terminal = TestTerminal::new(80, 24).unwrap();
    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("test");
    let result = terminal.spawn(cmd);
    assert!(result.is_ok());
}
```

#### Screen Module Tests (4 tests)
Location: `src/screen.rs`

Tests cover:
- Screen state creation
- Text feeding and parsing
- Cursor position tracking
- Content queries (text_at, contains)

Test patterns:
```rust
#[test]
fn test_feed_simple_text() {
    let mut screen = ScreenState::new(80, 24);
    screen.feed(b"Hello, World!");
    assert!(screen.contains("Hello, World!"));
}
```

#### Harness Module Tests (18 tests)
Location: `src/harness.rs`

Tests cover:
- Harness creation and configuration
- Builder pattern API
- Process spawning and management
- Waiting for conditions
- Timeout configuration
- Screen state access
- Resize operations

Builder pattern tests:
```rust
#[test]
fn test_builder_chaining() {
    let harness = TuiTestHarness::builder()
        .with_size(80, 24)
        .with_timeout(Duration::from_secs(10))
        .with_poll_interval(Duration::from_millis(50))
        .build();
    assert!(harness.is_ok());
}
```

#### Error Module Tests (8 tests)
Location: `src/error.rs`

Tests cover:
- Error type conversions (I/O, anyhow)
- Error message formatting
- Specific error variants (Timeout, InvalidDimensions, etc.)
- Error context preservation

Error testing pattern:
```rust
#[test]
fn test_invalid_dimensions_error() {
    let err = TermTestError::InvalidDimensions { width: 0, height: 24 };
    let msg = err.to_string();
    assert!(msg.contains("Invalid"));
    assert!(msg.contains("width=0"));
}
```

#### Sixel Module Tests (4 tests)
Location: `src/sixel.rs`

Tests cover:
- Sixel sequence bounds checking
- Overlap detection
- Capture filtering
- Validation assertions

#### Bevy Module Tests (3 tests)
Location: `src/bevy.rs`

Tests cover:
- Bevy harness creation
- Update cycle execution
- Frame rendering

### Integration Tests (32 tests)

Integration tests are in `tests/integration/` and test the library API as users would use it.

#### Basic Integration Tests (8 tests)
Location: `tests/integration/basic.rs`

Tests cover:
- Harness creation
- Invalid dimensions handling
- Screen state queries
- Timeout configuration
- Resize operations
- Cursor position tracking

#### Process Lifecycle Tests (6 tests)
Location: `tests/integration/process.rs`

Tests cover:
- Spawning commands (echo, sleep)
- Process status checking
- Sequential command execution
- Cannot spawn twice (process already running)
- Process exit status
- Wait for process completion

#### Error Handling Tests (8 tests)
Location: `tests/integration/errors.rs`

Tests cover:
- Invalid terminal dimensions
- Invalid resize dimensions
- Wait without process
- Spawn invalid command
- Timeout error messages
- Spawn failed errors
- Process state errors
- Error conversions

#### Sixel Integration Tests (5 tests)
Location: `tests/integration/sixel.rs`

Tests cover:
- Empty sixel capture
- Sequence bounds checking
- Overlap detection
- Filtering by area
- Validation assertions

#### Bevy Integration Tests (5 tests)
Location: `tests/integration/bevy.rs`

Tests cover:
- Harness creation
- Update cycles
- Update N frames
- Render frame
- Bevy-ratatui plugin integration

## Running Tests

### Run All Tests
```bash
cargo test
```

### Run Unit Tests Only
```bash
cargo test --lib
```

### Run Integration Tests Only
```bash
cargo test --test '*'
```

### Run Specific Test Module
```bash
cargo test --lib pty::tests
cargo test --test basic
```

### Run with Output
```bash
cargo test -- --nocapture
```

### Run with Features
```bash
cargo test --features sixel
cargo test --features bevy
cargo test --features mvp  # All MVP features
```

## Test Patterns

### 1. Error Testing Pattern
Always test both success and error cases:

```rust
#[test]
fn test_invalid_dimensions() {
    let result = TestTerminal::new(0, 24);
    assert!(matches!(result, Err(TermTestError::InvalidDimensions { .. })));
}
```

### 2. State Verification Pattern
Verify state changes explicitly:

```rust
#[test]
fn test_cursor_position() {
    let mut screen = ScreenState::new(80, 24);
    assert_eq!(screen.cursor_position(), (0, 0));

    screen.feed(b"Hello");
    assert_eq!(screen.cursor_position(), (0, 5));
}
```

### 3. Builder Pattern Testing
Test both individual settings and chaining:

```rust
#[test]
fn test_builder_with_timeout() {
    let harness = TuiTestHarness::builder()
        .with_timeout(Duration::from_secs(10))
        .build()?;
    // Verify timeout is set
}
```

### 4. Integration Testing Pattern
Test the full API flow as users would:

```rust
#[test]
fn test_spawn_and_wait() -> Result<()> {
    let mut harness = TuiTestHarness::new(80, 24)?;
    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("hello");

    harness.spawn(cmd)?;
    std::thread::sleep(Duration::from_millis(100));
    harness.update_state()?;

    Ok(())
}
```

## Test Coverage Summary

**Total Tests: 88**
- **Unit Tests: 56** (PTY: 19, Screen: 4, Harness: 18, Error: 8, Sixel: 4, Bevy: 3)
- **Integration Tests: 32** (Basic: 8, Process: 6, Errors: 8, Sixel: 5, Bevy: 5)

### Coverage by Category
- **Core PTY Operations**: 25 tests (19 unit + 6 integration)
- **Screen State Management**: 4 tests
- **Harness API**: 26 tests (18 unit + 8 integration)
- **Error Handling**: 16 tests (8 unit + 8 integration)
- **Sixel Support**: 9 tests (4 unit + 5 integration)
- **Bevy Integration**: 8 tests (3 unit + 5 integration)

## CI/CD Integration

Tests run automatically in GitHub Actions on:
- Every push to main
- Every pull request
- Linux (headless)
- Multiple Rust versions (stable, beta, nightly)

See `.github/workflows/ci.yml` for details.

## Test Development Guidelines

1. **Write tests first** (TDD) when adding new features
2. **Test error cases** as thoroughly as success cases
3. **Use descriptive test names** that explain what is being tested
4. **Keep tests focused** - one assertion concept per test
5. **Avoid test interdependencies** - each test should be independent
6. **Use helper functions** to reduce boilerplate
7. **Document complex test scenarios** with comments
8. **Run tests locally** before pushing

## Common Test Issues

### Timing-Dependent Tests
Some tests involve process spawning and may have timing dependencies:

```rust
// Give process time to start
std::thread::sleep(Duration::from_millis(100));
harness.update_state()?;
```

### Resource Cleanup
PTY tests create system resources. The `Drop` impl ensures cleanup:

```rust
impl Drop for TestTerminal {
    fn drop(&mut self) {
        // Automatically kills child process
    }
}
```

### Headless CI
All tests work in headless Linux CI environments (no X11/Wayland required).

## Future Test Additions

For Phase 2-4:
- Event simulation tests
- Advanced Sixel parsing tests
- Bevy ECS integration tests
- Snapshot testing with insta
- Async/await tests with Tokio
- Performance benchmarks

---

Last Updated: 2025-11-20 after Phase 1 test expansion
