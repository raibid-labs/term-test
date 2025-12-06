# IPC Testing for Split-Process Terminals

> Testing guide for terminal applications using daemon + client architecture

## Overview

The `ipc` module provides testing utilities for applications using a split architecture where:
- A **daemon process** manages the PTY, parses terminal state, and exposes it via shared memory
- A **client process** (often GPU-accelerated) renders the UI

This pattern is common in modern terminal emulators that separate parsing from rendering for performance.

## Quick Start

### 1. Enable the Feature

```toml
[dependencies]
ratatui-testlib = { version = "0.3", features = ["ipc"] }
```

### 2. Set Environment Variable

Enable IPC testing mode:

```bash
export RTL_IPC_TEST=1
```

### 3. Basic Test

```rust
use std::time::Duration;
use ratatui_testlib::ipc::{DaemonTestHarness, DaemonConfig};

#[test]
fn test_echo_command() -> Result<(), Box<dyn std::error::Error>> {
    let config = DaemonConfig::builder()
        .socket_path("/tmp/my-daemon.sock")
        .shm_path("/my_term_shm")
        .build();

    let mut harness = DaemonTestHarness::with_config(config)?;

    // Send input via IPC
    harness.send_input("echo hello\n")?;

    // Wait for output in shared memory
    harness.wait_for_text("hello", Duration::from_secs(5))?;

    Ok(())
}
```

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Test Process                              │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                    DaemonTestHarness                        ││
│  │  ┌─────────────────┐  ┌──────────────────────────────────┐  ││
│  │  │ IPC Client      │  │ SharedMemoryReader               │  ││
│  │  │ (Unix Socket)   │  │ (Memory-mapped terminal state)   │  ││
│  │  └────────┬────────┘  └──────────────┬───────────────────┘  ││
│  └───────────┼──────────────────────────┼──────────────────────┘│
└──────────────┼──────────────────────────┼───────────────────────┘
               │                          │
               │ ControlMessage::Input    │ mmap read
               ▼                          ▼
┌──────────────────────┐        ┌─────────────────────┐
│   Terminal Daemon    │◄──────►│   Shared Memory     │
│   (PTY + Parsing)    │        │   (Grid + Cursor)   │
└──────────────────────┘        └─────────────────────┘
```

## Configuration

### DaemonConfig Options

| Option | Default | Description |
|--------|---------|-------------|
| `socket_path` | `/tmp/term-daemon.sock` | Unix socket path for IPC |
| `shm_path` | `/term_shm_v1` | POSIX shared memory path |
| `spawn_daemon` | `false` | Auto-spawn daemon process |
| `daemon_command` | `term-daemon` | Command to spawn daemon |
| `dimensions` | `(80, 24)` | Terminal size (cols, rows) |
| `connect_timeout` | `5s` | Socket connection timeout |
| `default_timeout` | `10s` | Default wait timeout |

### Example: Custom Configuration

```rust
let config = DaemonConfig::builder()
    .socket_path("/tmp/my-app.sock")
    .shm_path("/my_app_term")
    .spawn_daemon(true)
    .daemon_command("my-term-daemon")
    .daemon_args(vec!["--shell".into(), "/bin/zsh".into()])
    .dimensions(120, 40)
    .connect_timeout(Duration::from_secs(10))
    .build();
```

## Testing Patterns

### Pattern 1: Send and Verify

```rust
#[test]
fn test_ls_command() -> Result<()> {
    let mut harness = DaemonTestHarness::with_config(config)?;

    harness.send_input("ls -la\n")?;
    harness.wait_for_text("total", Duration::from_secs(2))?;

    let grid = harness.grid_contents()?;
    assert!(grid.contains("total"));

    Ok(())
}
```

### Pattern 2: Escape Sequences

```rust
#[test]
fn test_cursor_movement() -> Result<()> {
    let mut harness = DaemonTestHarness::with_config(config)?;

    // Send escape sequence for cursor up
    harness.send_input("\x1b[A")?;

    // Verify cursor position
    let (row, col) = harness.cursor_position()?;
    assert!(row < 10);

    Ok(())
}
```

### Pattern 3: Wait for Sequence

```rust
#[test]
fn test_multi_step_workflow() -> Result<()> {
    let mut harness = DaemonTestHarness::with_config(config)?;

    harness.send_input("cd /tmp && ls\n")?;

    // Wait for multiple texts in order
    harness.wait_for_sequence(&["cd /tmp", "ls"], Duration::from_secs(5))?;

    Ok(())
}
```

### Pattern 4: Spawn Fresh Daemon

```rust
#[test]
fn test_with_fresh_daemon() -> Result<()> {
    let config = DaemonConfig::builder()
        .socket_path("/tmp/test-daemon.sock")
        .shm_path("/test_term_shm")
        .spawn_daemon(true)
        .build();

    let mut harness = DaemonTestHarness::with_config(config)?;
    // Daemon is spawned and ready

    harness.send_input("echo test\n")?;
    harness.wait_for_text("test", Duration::from_secs(2))?;

    Ok(())
}
```

## Shared Memory Format

The default shared memory format uses a header followed by grid data:

```
Offset  Size    Field
0       4       magic (0x5445_524D = "TERM")
4       4       version (1)
8       2       cols
10      2       rows
12      2       cursor_col
14      2       cursor_row
16      4       sequence_number
20      4       grid_offset
24      4       grid_size
28      4       attrs_offset
32      4       attrs_size
36+     N       grid data (chars)
```

### Custom Protocol

If your daemon uses a different shared memory format, you can provide custom validation:

```rust
use ratatui_testlib::ipc::DaemonSharedMemory;

// Open with custom magic/version
let shm = DaemonSharedMemory::open_with_validation(
    "/my_custom_shm",
    0x5343_5241,  // "SCRA" - custom magic
    1,            // version
)?;
```

## Error Handling

```rust
use ratatui_testlib::ipc::{DaemonTestHarness, DaemonConfig, IpcError};

match DaemonTestHarness::with_config(config) {
    Ok(harness) => println!("Connected!"),
    Err(IpcError::SocketNotFound(path)) => {
        eprintln!("Daemon not running at: {}", path.display());
    }
    Err(IpcError::SharedMemoryNotFound(path)) => {
        eprintln!("Shared memory not available: {}", path);
    }
    Err(IpcError::Timeout(duration)) => {
        eprintln!("Operation timed out after {:?}", duration);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## CI/CD Integration

### GitHub Actions Example

```yaml
jobs:
  test-ipc:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Start daemon
        run: |
          ./my-term-daemon &
          sleep 2

      - name: Run IPC tests
        env:
          RTL_IPC_TEST: "1"
        run: cargo test --features ipc
```

### Docker Example

```dockerfile
FROM rust:latest

# Install daemon
COPY my-term-daemon /usr/local/bin/

# Run tests
ENV RTL_IPC_TEST=1
CMD ["cargo", "test", "--features", "ipc"]
```

## Troubleshooting

### Issue: Socket Not Found

**Symptoms**: `IpcError::SocketNotFound`

**Solutions**:
- Verify daemon is running
- Check socket path matches configuration
- Ensure socket has correct permissions

```bash
# Check if socket exists
ls -la /tmp/term-daemon.sock

# Check daemon process
pgrep -f term-daemon
```

### Issue: Shared Memory Not Found

**Symptoms**: `IpcError::SharedMemoryNotFound`

**Solutions**:
- Verify daemon creates shared memory
- Check shared memory path
- Ensure sufficient permissions

```bash
# List shared memory segments
ls -la /dev/shm/

# Check specific segment
cat /proc/sysvipc/shm
```

### Issue: Timeout Waiting for Text

**Symptoms**: `IpcError::Timeout`

**Solutions**:
- Increase timeout duration
- Verify daemon is processing input
- Check shared memory is being updated

```rust
// Increase timeout
harness.wait_for_text("expected", Duration::from_secs(30))?;

// Check sequence number changes
let seq1 = harness.shm.sequence_number();
std::thread::sleep(Duration::from_millis(100));
harness.shm.refresh()?;
let seq2 = harness.shm.sequence_number();
assert_ne!(seq1, seq2, "Shared memory not updating");
```

## Related Resources

- [Shared State Module](../src/shared_state.rs) - Generic shared memory helpers
- [TuiTestHarness](../src/harness.rs) - Standard PTY-based testing
- [Examples](../examples/ipc_daemon_test.rs) - Working example code
