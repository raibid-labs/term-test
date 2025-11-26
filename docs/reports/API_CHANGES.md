# API Changes Summary

## Overview
All changes to `/home/beengud/raibid-labs/mimic/src/pty.rs` are **backward compatible**. No breaking changes were made to existing APIs.

## New Public Methods

### Configuration
```rust
/// Configure the buffer size for read operations
pub fn with_buffer_size(self, size: usize) -> Self
```
- Chainable builder method
- Default: 8192 bytes
- Use larger buffers for high-throughput applications

### Enhanced Spawning
```rust
/// Spawn a process with a custom timeout
pub fn spawn_with_timeout(&mut self, cmd: CommandBuilder, timeout: Duration) -> Result<()>
```
- Existing `spawn()` method now delegates to this with 5-second default timeout
- Supports full CommandBuilder API:
  - `cmd.arg()` - Add command arguments
  - `cmd.env()` - Set environment variables
  - `cmd.cwd()` - Set working directory
- Better error context: "Failed to spawn process in PTY: ..."

### Advanced I/O Operations

#### Read Methods
```rust
/// Read with timeout
pub fn read_timeout(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize>
```
- Blocks until data available or timeout
- Returns number of bytes read
- Returns `TermTestError::Timeout` on timeout
- Handles EINTR automatically

```rust
/// Read all available data
pub fn read_all(&mut self) -> Result<Vec<u8>>
```
- Non-blocking batch read
- Uses configured buffer size
- Returns all immediately available data
- Empty Vec if no data available

#### Write Methods
```rust
/// Write all data, ensuring complete buffer is written
pub fn write_all(&mut self, data: &[u8]) -> Result<()>
```
- Guarantees entire buffer is written
- Handles EINTR automatically
- Returns only after all bytes written

### Process Lifecycle Management

```rust
/// Terminate the child process
pub fn kill(&mut self) -> Result<()>
```
- Sends SIGTERM (graceful termination)
- Then SIGKILL if needed (forced termination)
- Cleans up child process handle
- Returns error if no process running

```rust
/// Wait for process exit with timeout
pub fn wait_timeout(&mut self, timeout: Duration) -> Result<ExitStatus>
```
- Polls every 10ms for process exit
- Returns ExitStatus on success
- Returns `TermTestError::Timeout` on timeout
- Caches exit status for later retrieval

```rust
/// Get cached exit status
pub fn get_exit_status(&self) -> Option<ExitStatus>
```
- Returns None if process still running
- Returns Some(ExitStatus) after process exits
- Call `is_running()` or `wait()` to update cache

## Enhanced Existing Methods

### spawn()
```rust
pub fn spawn(&mut self, cmd: CommandBuilder) -> Result<()>
```
**Changes:**
- Now uses 5-second default timeout via `spawn_with_timeout()`
- Better error messages: "Failed to spawn process in PTY: ..."
- Resets `exit_status` on new spawn

### read()
```rust
pub fn read(&mut self, buf: &mut [u8]) -> Result<usize>
```
**Enhancements:**
- Automatic retry on EINTR (interrupted system calls)
- Returns 0 on EAGAIN/EWOULDBLOCK (no data available)
- Better error context: "Failed to clone PTY reader: ..."

### write()
```rust
pub fn write(&mut self, data: &[u8]) -> Result<usize>
```
**Enhancements:**
- Automatic retry on EINTR
- Better error context: "Failed to get PTY writer: ..."

### is_running()
```rust
pub fn is_running(&mut self) -> bool
```
**Enhancements:**
- Automatically caches exit status when process terminates
- More robust process state checking
- Handles errors gracefully (returns false on error)

### wait()
```rust
pub fn wait(&mut self) -> Result<ExitStatus>
```
**Enhancements:**
- Caches exit status for later retrieval via `get_exit_status()`
- Better error message: "Failed to wait for child process: ..."

## New Internal Constants

```rust
const DEFAULT_BUFFER_SIZE: usize = 8192;
const DEFAULT_SPAWN_TIMEOUT: Duration = Duration::from_secs(5);
```

## New Struct Fields

```rust
pub struct TestTerminal {
    pty_pair: PtyPair,
    child: Option<Box<dyn Child + Send + Sync>>,
    exit_status: Option<ExitStatus>,  // NEW: Cached exit status
    buffer_size: usize,                // NEW: Configurable buffer size
}
```

## Error Handling Improvements

All operations now provide better error context:

| Operation | Old Error | New Error |
|-----------|-----------|-----------|
| spawn | "Failed to spawn process: {e}" | "Failed to spawn process in PTY: {e}" |
| read | Generic I/O error | "Failed to clone PTY reader: {e}" |
| write | Generic I/O error | "Failed to get PTY writer: {e}" |
| kill | Generic I/O error | "Failed to kill child process: {e}" |
| wait | Generic error | "Failed to wait for child process: {e}" |
| wait_timeout | N/A | "Failed to check process status: {e}" |

## Compatibility Matrix

| API | Backward Compatible | Notes |
|-----|---------------------|-------|
| `new()` | ✅ Yes | No changes |
| `with_buffer_size()` | ✅ Yes | New method, chainable |
| `spawn()` | ✅ Yes | Enhanced but compatible |
| `spawn_with_timeout()` | ✅ Yes | New method |
| `read()` | ✅ Yes | Enhanced but compatible |
| `read_timeout()` | ✅ Yes | New method |
| `read_all()` | ✅ Yes | New method |
| `write()` | ✅ Yes | Enhanced but compatible |
| `write_all()` | ✅ Yes | New method |
| `resize()` | ✅ Yes | No changes |
| `size()` | ✅ Yes | No changes |
| `is_running()` | ✅ Yes | Enhanced but compatible |
| `wait()` | ✅ Yes | Enhanced but compatible |
| `wait_timeout()` | ✅ Yes | New method |
| `kill()` | ✅ Yes | New method |
| `get_exit_status()` | ✅ Yes | New method |

## Migration Guide

No migration needed! All existing code continues to work.

### Optional Enhancements

If you want to take advantage of new features:

1. **Add timeout handling:**
   ```rust
   // Old
   terminal.spawn(cmd)?;

   // New (optional)
   terminal.spawn_with_timeout(cmd, Duration::from_secs(3))?;
   ```

2. **Add process lifecycle management:**
   ```rust
   // Old
   terminal.wait()?;

   // New (optional)
   if let Err(_) = terminal.wait_timeout(Duration::from_secs(5)) {
       terminal.kill()?;  // Force kill on timeout
   }
   ```

3. **Add read timeouts:**
   ```rust
   // Old
   let mut buf = [0u8; 1024];
   terminal.read(&mut buf)?;

   // New (optional)
   let mut buf = [0u8; 1024];
   terminal.read_timeout(&mut buf, Duration::from_secs(1))?;
   ```

4. **Configure buffer size:**
   ```rust
   // Old
   let terminal = TestTerminal::new(80, 24)?;

   // New (optional)
   let terminal = TestTerminal::new(80, 24)?
       .with_buffer_size(16384);
   ```

## Compatibility with src/harness.rs

The TuiTestHarness in `/home/beengud/raibid-labs/mimic/src/harness.rs` continues to work without any modifications.

All methods it uses remain compatible:
- ✅ `spawn()` - Enhanced but compatible
- ✅ `read()` - Enhanced but compatible
- ✅ `write()` - Enhanced but compatible
- ✅ `is_running()` - Enhanced with exit status caching
- ✅ `wait()` - Enhanced with exit status caching
- ✅ `resize()` - No changes

The harness can optionally be enhanced later to use new features like `read_timeout()` and `wait_timeout()`.
