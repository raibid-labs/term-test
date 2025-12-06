//! IPC and shared-memory helpers for split-process terminal testing.
//!
//! This module provides testing utilities for applications using a split architecture
//! (daemon + client), where one process manages the PTY and exposes state via shared
//! memory, while another renders the UI. This pattern is common in GPU-accelerated
//! terminal emulators.
//!
//! # Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        Test Process                              │
//! │  ┌─────────────────────────────────────────────────────────────┐│
//! │  │                    DaemonTestHarness                        ││
//! │  │  ┌─────────────────┐  ┌──────────────────────────────────┐  ││
//! │  │  │ IPC Client      │  │ SharedMemoryReader               │  ││
//! │  │  │ (Unix Socket)   │  │ (Memory-mapped terminal state)   │  ││
//! │  │  └────────┬────────┘  └──────────────┬───────────────────┘  ││
//! │  └───────────┼──────────────────────────┼──────────────────────┘│
//! └──────────────┼──────────────────────────┼───────────────────────┘
//!                │                          │
//!                │ ControlMessage::Input    │ mmap read
//!                ▼                          ▼
//! ┌──────────────────────┐        ┌─────────────────────┐
//! │   Terminal Daemon    │◄──────►│   Shared Memory     │
//! │   (PTY + Parsing)    │        │   (Grid + Cursor)   │
//! └──────────────────────┘        └─────────────────────┘
//! ```
//!
//! # Features
//!
//! - **IPC via Unix sockets**: Connect to daemon and send control messages
//! - **Shared memory reading**: Map terminal state and expose grid contents
//! - **`TerminalStateReader` wrappers**: Access grid contents, cursor position, and attributes
//! - **`wait_for_text`/`wait_for_sequence` helpers**: Polling with timeouts
//!
//! # Quick Start
//!
//! ## Enable the feature
//!
//! ```toml
//! [dependencies]
//! ratatui-testlib = { version = "0.3", features = ["ipc"] }
//! ```
//!
//! ## Environment Variable
//!
//! Enable IPC testing mode by setting:
//!
//! ```bash
//! export RTL_IPC_TEST=1
//! ```
//!
//! ## Basic Test Example
//!
//! ```rust,no_run
//! # #[cfg(feature = "ipc")]
//! # {
//! use std::time::Duration;
//! use ratatui_testlib::ipc::{DaemonTestHarness, DaemonConfig};
//!
//! # fn test() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to running daemon
//! let config = DaemonConfig::builder()
//!     .socket_path("/tmp/my-daemon.sock")
//!     .shm_path("/my_term_shm")
//!     .build();
//!
//! let mut harness = DaemonTestHarness::with_config(config)?;
//!
//! // Send input via IPC
//! harness.send_input("echo hello\n")?;
//!
//! // Wait for output in shared memory grid
//! harness.wait_for_text("hello", Duration::from_secs(5))?;
//!
//! // Assert grid contents
//! let grid = harness.grid_contents()?;
//! assert!(grid.contains("hello"));
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! # Key Types
//!
//! - [`DaemonTestHarness`]: Main test harness combining IPC + shared memory
//! - [`DaemonIpcClient`]: Unix socket client for sending control messages
//! - [`DaemonSharedMemory`]: Shared memory reader for terminal state
//! - [`DaemonTestExt`]: Extension trait for TuiTestHarness integration
//! - [`ControlMessage`]: IPC message types for daemon communication
//!
//! # Testing Patterns
//!
//! ## Pattern 1: Send Command and Verify Output
//!
//! ```rust,no_run
//! # #[cfg(feature = "ipc")]
//! # {
//! use std::time::Duration;
//! use ratatui_testlib::ipc::{DaemonTestHarness, DaemonConfig};
//!
//! # fn test() -> Result<(), Box<dyn std::error::Error>> {
//! let config = DaemonConfig::default();
//! let mut harness = DaemonTestHarness::with_config(config)?;
//!
//! // Send a shell command
//! harness.send_input("ls -la\n")?;
//!
//! // Wait for the output to appear
//! harness.wait_for_text("total", Duration::from_secs(2))?;
//!
//! // Verify specific content
//! let grid = harness.grid_contents()?;
//! assert!(grid.contains("total"));
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Pattern 2: Test Keyboard Sequences
//!
//! ```rust,no_run
//! # #[cfg(feature = "ipc")]
//! # {
//! use std::time::Duration;
//! use ratatui_testlib::ipc::{DaemonTestHarness, DaemonConfig};
//!
//! # fn test() -> Result<(), Box<dyn std::error::Error>> {
//! let config = DaemonConfig::default();
//! let mut harness = DaemonTestHarness::with_config(config)?;
//!
//! // Send escape sequence for cursor movement
//! harness.send_input("\x1b[A")?; // Up arrow
//!
//! // Verify cursor moved
//! let (row, col) = harness.cursor_position()?;
//! assert!(row < 10);
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Pattern 3: Spawning a Fresh Daemon
//!
//! ```rust,no_run
//! # #[cfg(feature = "ipc")]
//! # {
//! use std::time::Duration;
//! use ratatui_testlib::ipc::{DaemonTestHarness, DaemonConfig};
//!
//! # fn test() -> Result<(), Box<dyn std::error::Error>> {
//! let config = DaemonConfig::builder()
//!     .socket_path("/tmp/test-daemon.sock")
//!     .shm_path("/test_term_shm")
//!     .daemon_command("my-term-daemon")
//!     .spawn_daemon(true)
//!     .build();
//!
//! let mut harness = DaemonTestHarness::with_config(config)?;
//!
//! // Daemon is spawned and ready
//! harness.send_input("echo test\n")?;
//! harness.wait_for_text("test", Duration::from_secs(2))?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! # Error Handling
//!
//! All operations return [`IpcResult`] which wraps [`IpcError`] for detailed diagnostics:
//!
//! ```rust,no_run
//! # #[cfg(feature = "ipc")]
//! # {
//! use ratatui_testlib::ipc::{DaemonTestHarness, DaemonConfig, IpcError};
//!
//! # fn test() -> Result<(), Box<dyn std::error::Error>> {
//! let config = DaemonConfig::default();
//! match DaemonTestHarness::with_config(config) {
//!     Ok(harness) => println!("Connected!"),
//!     Err(IpcError::SocketNotFound(path)) => {
//!         eprintln!("Daemon not running: {}", path);
//!     }
//!     Err(IpcError::SharedMemoryNotFound(path)) => {
//!         eprintln!("Shared memory not available: {}", path);
//!     }
//!     Err(e) => eprintln!("Other error: {}", e),
//! }
//! # Ok(())
//! # }
//! # }
//! ```

use std::{
    io::Write,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use thiserror::Error;

// Default paths - can be overridden via config
const DEFAULT_SOCKET_PATH: &str = "/tmp/term-daemon.sock";
const DEFAULT_SHM_PATH: &str = "/term_shm_v1";
const DEFAULT_DAEMON_COMMAND: &str = "term-daemon";

/// Errors that can occur during IPC test operations.
#[derive(Debug, Error)]
pub enum IpcError {
    /// Unix socket not found at expected path.
    #[error("Daemon socket not found at: {0}")]
    SocketNotFound(PathBuf),

    /// Failed to connect to Unix socket.
    #[error("Failed to connect to daemon: {0}")]
    ConnectionFailed(#[source] std::io::Error),

    /// Failed to send message via IPC.
    #[error("Failed to send IPC message: {0}")]
    SendFailed(#[source] std::io::Error),

    /// Shared memory segment not found.
    #[error("Shared memory not found at: {0}")]
    SharedMemoryNotFound(String),

    /// Failed to map shared memory.
    #[error("Failed to map shared memory: {0}")]
    MmapFailed(String),

    /// Timeout waiting for condition.
    #[error("Timeout after {0:?} waiting for condition")]
    Timeout(Duration),

    /// Invalid shared memory format or data.
    #[error("Invalid shared memory data: {0}")]
    InvalidData(String),

    /// Daemon process failed to spawn.
    #[error("Failed to spawn daemon: {0}")]
    SpawnFailed(#[source] std::io::Error),

    /// I/O error during operation.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Environment variable RTL_IPC_TEST not set.
    #[error("RTL_IPC_TEST environment variable not set - IPC testing disabled")]
    TestingDisabled,
}

/// Result type for IPC operations.
pub type IpcResult<T> = std::result::Result<T, IpcError>;

/// Control messages sent to the terminal daemon via IPC.
///
/// These messages provide a generic interface for controlling terminal
/// daemons. Implementations should adapt this to their specific protocol.
#[derive(Debug, Clone)]
pub enum ControlMessage {
    /// Send keyboard/text input to the PTY.
    Input(Vec<u8>),

    /// Resize the terminal.
    Resize {
        /// Number of columns.
        cols: u16,
        /// Number of rows.
        rows: u16,
    },

    /// Request a state refresh.
    Refresh,

    /// Shutdown the daemon gracefully.
    Shutdown,
}

impl ControlMessage {
    /// Serialize the message for transmission.
    ///
    /// Uses a simple wire format:
    /// - 1 byte: message type (0=Input, 1=Resize, 2=Refresh, 3=Shutdown)
    /// - Variable: payload
    ///
    /// Override this if your daemon uses a different protocol.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            ControlMessage::Input(data) => {
                let mut buf = vec![0u8]; // Type 0
                buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
                buf.extend_from_slice(data);
                buf
            }
            ControlMessage::Resize { cols, rows } => {
                let mut buf = vec![1u8]; // Type 1
                buf.extend_from_slice(&cols.to_le_bytes());
                buf.extend_from_slice(&rows.to_le_bytes());
                buf
            }
            ControlMessage::Refresh => vec![2u8], // Type 2
            ControlMessage::Shutdown => vec![3u8], // Type 3
        }
    }
}

/// Configuration for the daemon test harness.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Path to the Unix socket for IPC.
    pub socket_path: PathBuf,

    /// Path to the shared memory segment for terminal state.
    pub shm_path: String,

    /// Path to optional image shared memory segment.
    pub image_shm_path: Option<String>,

    /// Whether to spawn a fresh daemon process.
    pub spawn_daemon: bool,

    /// Command to spawn the daemon (if spawn_daemon is true).
    pub daemon_command: String,

    /// Additional arguments for the daemon command.
    pub daemon_args: Vec<String>,

    /// Terminal dimensions (cols, rows) if spawning daemon.
    pub dimensions: Option<(u16, u16)>,

    /// Connection timeout.
    pub connect_timeout: Duration,

    /// Default timeout for wait operations.
    pub default_timeout: Duration,

    /// Custom message serializer (if None, uses default protocol).
    pub custom_serializer: Option<fn(&ControlMessage) -> Vec<u8>>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: PathBuf::from(DEFAULT_SOCKET_PATH),
            shm_path: DEFAULT_SHM_PATH.to_string(),
            image_shm_path: None,
            spawn_daemon: false,
            daemon_command: DEFAULT_DAEMON_COMMAND.to_string(),
            daemon_args: Vec::new(),
            dimensions: Some((80, 24)),
            connect_timeout: Duration::from_secs(5),
            default_timeout: Duration::from_secs(10),
            custom_serializer: None,
        }
    }
}

impl DaemonConfig {
    /// Create a new configuration builder.
    pub fn builder() -> DaemonConfigBuilder {
        DaemonConfigBuilder::default()
    }
}

/// Builder for DaemonConfig.
#[derive(Debug, Default)]
pub struct DaemonConfigBuilder {
    config: DaemonConfig,
}

impl DaemonConfigBuilder {
    /// Set the Unix socket path.
    pub fn socket_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.socket_path = path.into();
        self
    }

    /// Set the shared memory path.
    pub fn shm_path(mut self, path: impl Into<String>) -> Self {
        self.config.shm_path = path.into();
        self
    }

    /// Set the image shared memory path.
    pub fn image_shm_path(mut self, path: impl Into<String>) -> Self {
        self.config.image_shm_path = Some(path.into());
        self
    }

    /// Set whether to spawn a daemon.
    pub fn spawn_daemon(mut self, spawn: bool) -> Self {
        self.config.spawn_daemon = spawn;
        self
    }

    /// Set the daemon command to spawn.
    pub fn daemon_command(mut self, cmd: impl Into<String>) -> Self {
        self.config.daemon_command = cmd.into();
        self
    }

    /// Add daemon command arguments.
    pub fn daemon_args(mut self, args: Vec<String>) -> Self {
        self.config.daemon_args = args;
        self
    }

    /// Set terminal dimensions.
    pub fn dimensions(mut self, cols: u16, rows: u16) -> Self {
        self.config.dimensions = Some((cols, rows));
        self
    }

    /// Set connection timeout.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = timeout;
        self
    }

    /// Set default wait timeout.
    pub fn default_timeout(mut self, timeout: Duration) -> Self {
        self.config.default_timeout = timeout;
        self
    }

    /// Set a custom message serializer.
    pub fn custom_serializer(mut self, serializer: fn(&ControlMessage) -> Vec<u8>) -> Self {
        self.config.custom_serializer = Some(serializer);
        self
    }

    /// Build the configuration.
    pub fn build(self) -> DaemonConfig {
        self.config
    }
}

/// IPC client for communicating with the terminal daemon.
#[derive(Debug)]
pub struct DaemonIpcClient {
    stream: UnixStream,
    serializer: Option<fn(&ControlMessage) -> Vec<u8>>,
}

impl DaemonIpcClient {
    /// Connect to the daemon at the given socket path.
    pub fn connect(socket_path: impl AsRef<Path>) -> IpcResult<Self> {
        Self::connect_with_serializer(socket_path, None)
    }

    /// Connect with a custom message serializer.
    pub fn connect_with_serializer(
        socket_path: impl AsRef<Path>,
        serializer: Option<fn(&ControlMessage) -> Vec<u8>>,
    ) -> IpcResult<Self> {
        let path = socket_path.as_ref();
        if !path.exists() {
            return Err(IpcError::SocketNotFound(path.to_path_buf()));
        }

        let stream = UnixStream::connect(path).map_err(IpcError::ConnectionFailed)?;

        // Set non-blocking for timeout support
        stream
            .set_nonblocking(false)
            .map_err(IpcError::ConnectionFailed)?;

        Ok(Self { stream, serializer })
    }

    /// Send a control message to the daemon.
    pub fn send(&mut self, message: ControlMessage) -> IpcResult<()> {
        let bytes = if let Some(serializer) = self.serializer {
            serializer(&message)
        } else {
            message.to_bytes()
        };
        self.stream
            .write_all(&bytes)
            .map_err(IpcError::SendFailed)?;
        self.stream.flush().map_err(IpcError::SendFailed)?;
        Ok(())
    }

    /// Send raw input bytes to the PTY.
    pub fn send_input(&mut self, input: &[u8]) -> IpcResult<()> {
        self.send(ControlMessage::Input(input.to_vec()))
    }

    /// Send a string as input to the PTY.
    pub fn send_text(&mut self, text: &str) -> IpcResult<()> {
        self.send_input(text.as_bytes())
    }

    /// Request the daemon to resize the terminal.
    pub fn resize(&mut self, cols: u16, rows: u16) -> IpcResult<()> {
        self.send(ControlMessage::Resize { cols, rows })
    }

    /// Request a state refresh from the daemon.
    pub fn refresh(&mut self) -> IpcResult<()> {
        self.send(ControlMessage::Refresh)
    }
}

/// Shared memory header structure for terminal state.
///
/// This is a generic header format. Override via custom implementations
/// if your daemon uses a different layout.
///
/// ```text
/// Offset  Size    Field
/// 0       4       magic
/// 4       4       version
/// 8       2       cols
/// 10      2       rows
/// 12      2       cursor_col
/// 14      2       cursor_row
/// 16      4       sequence_number
/// 20      4       grid_offset
/// 24      4       grid_size
/// 28      4       attrs_offset
/// 32      4       attrs_size
/// ```
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ShmHeader {
    /// Magic number for validation.
    pub magic: u32,
    /// Protocol version.
    pub version: u32,
    /// Terminal columns.
    pub cols: u16,
    /// Terminal rows.
    pub rows: u16,
    /// Cursor column position.
    pub cursor_col: u16,
    /// Cursor row position.
    pub cursor_row: u16,
    /// Sequence number for change detection.
    pub sequence_number: u32,
    /// Offset to grid data.
    pub grid_offset: u32,
    /// Size of grid data.
    pub grid_size: u32,
    /// Offset to attributes data.
    pub attrs_offset: u32,
    /// Size of attributes data.
    pub attrs_size: u32,
}

impl ShmHeader {
    /// Default magic number (can be overridden).
    pub const DEFAULT_MAGIC: u32 = 0x5445_524D; // "TERM"

    /// Default protocol version.
    pub const DEFAULT_VERSION: u32 = 1;

    /// Validate the header with default magic/version.
    pub fn validate(&self) -> IpcResult<()> {
        self.validate_with(Self::DEFAULT_MAGIC, Self::DEFAULT_VERSION)
    }

    /// Validate the header with custom magic and version.
    pub fn validate_with(&self, expected_magic: u32, expected_version: u32) -> IpcResult<()> {
        if self.magic != expected_magic {
            return Err(IpcError::InvalidData(format!(
                "Invalid magic: expected 0x{:08X}, got 0x{:08X}",
                expected_magic, self.magic
            )));
        }
        if self.version != expected_version {
            return Err(IpcError::InvalidData(format!(
                "Unsupported version: expected {}, got {}",
                expected_version, self.version
            )));
        }
        Ok(())
    }
}

/// Reader for shared memory terminal state.
///
/// This provides read-only access to the terminal grid, cursor position,
/// and cell attributes stored in shared memory.
#[cfg(target_family = "unix")]
pub struct DaemonSharedMemory {
    #[allow(dead_code)]
    shm_fd: std::os::fd::RawFd,
    mmap: *const u8,
    size: usize,
    header: ShmHeader,
    expected_magic: u32,
    expected_version: u32,
}

#[cfg(target_family = "unix")]
impl std::fmt::Debug for DaemonSharedMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DaemonSharedMemory")
            .field("size", &self.size)
            .field("header", &self.header)
            .finish()
    }
}

#[cfg(target_family = "unix")]
impl DaemonSharedMemory {
    /// Open and map the shared memory segment with default validation.
    pub fn open(shm_path: &str) -> IpcResult<Self> {
        Self::open_with_validation(
            shm_path,
            ShmHeader::DEFAULT_MAGIC,
            ShmHeader::DEFAULT_VERSION,
        )
    }

    /// Open and map with custom magic/version validation.
    #[allow(unsafe_code)]
    pub fn open_with_validation(
        shm_path: &str,
        expected_magic: u32,
        expected_version: u32,
    ) -> IpcResult<Self> {
        use std::ffi::CString;

        let path_cstr = CString::new(shm_path)
            .map_err(|_| IpcError::InvalidData("Invalid shm path".to_string()))?;

        // Open the shared memory object
        let fd = unsafe { libc::shm_open(path_cstr.as_ptr(), libc::O_RDONLY, 0o644) };

        if fd < 0 {
            return Err(IpcError::SharedMemoryNotFound(shm_path.to_string()));
        }

        // Get the size
        let mut stat: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(fd, &mut stat) } < 0 {
            unsafe { libc::close(fd) };
            return Err(IpcError::MmapFailed("fstat failed".to_string()));
        }

        let size = stat.st_size as usize;
        if size < std::mem::size_of::<ShmHeader>() {
            unsafe { libc::close(fd) };
            return Err(IpcError::InvalidData(
                "Shared memory too small for header".to_string(),
            ));
        }

        // Map the memory
        let mmap = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                size,
                libc::PROT_READ,
                libc::MAP_SHARED,
                fd,
                0,
            )
        };

        if mmap == libc::MAP_FAILED {
            unsafe { libc::close(fd) };
            return Err(IpcError::MmapFailed("mmap failed".to_string()));
        }

        // Read and validate header
        let header: ShmHeader = unsafe { std::ptr::read(mmap as *const ShmHeader) };
        header.validate_with(expected_magic, expected_version)?;

        Ok(Self {
            shm_fd: fd,
            mmap: mmap as *const u8,
            size,
            header,
            expected_magic,
            expected_version,
        })
    }

    /// Refresh the header from shared memory.
    #[allow(unsafe_code)]
    pub fn refresh(&mut self) -> IpcResult<()> {
        self.header = unsafe { std::ptr::read(self.mmap as *const ShmHeader) };
        self.header
            .validate_with(self.expected_magic, self.expected_version)?;
        Ok(())
    }

    /// Get the terminal dimensions (cols, rows).
    pub fn dimensions(&self) -> (u16, u16) {
        (self.header.cols, self.header.rows)
    }

    /// Get the cursor position (row, col).
    pub fn cursor_position(&self) -> (u16, u16) {
        (self.header.cursor_row, self.header.cursor_col)
    }

    /// Get the sequence number for change detection.
    pub fn sequence_number(&self) -> u32 {
        self.header.sequence_number
    }

    /// Read the terminal grid as a string.
    ///
    /// Returns the grid content with newlines between rows.
    #[allow(unsafe_code)]
    pub fn grid_contents(&self) -> IpcResult<String> {
        let offset = self.header.grid_offset as usize;
        let size = self.header.grid_size as usize;

        if offset + size > self.size {
            return Err(IpcError::InvalidData(
                "Grid extends beyond shared memory".to_string(),
            ));
        }

        let grid_ptr = unsafe { self.mmap.add(offset) };
        let grid_slice = unsafe { std::slice::from_raw_parts(grid_ptr, size) };

        // Convert to string, replacing invalid UTF-8
        let text = String::from_utf8_lossy(grid_slice).to_string();

        // Format as rows
        let cols = self.header.cols as usize;
        let mut result = String::new();
        for (i, chunk) in text.chars().collect::<Vec<_>>().chunks(cols).enumerate() {
            if i > 0 {
                result.push('\n');
            }
            result.extend(chunk);
        }

        Ok(result)
    }

    /// Check if the grid contains the given text.
    pub fn contains(&self, text: &str) -> IpcResult<bool> {
        let grid = self.grid_contents()?;
        Ok(grid.contains(text))
    }

    /// Get a specific cell character at (row, col).
    #[allow(unsafe_code)]
    pub fn cell_at(&self, row: u16, col: u16) -> IpcResult<char> {
        if row >= self.header.rows || col >= self.header.cols {
            return Err(IpcError::InvalidData(format!(
                "Position ({}, {}) out of bounds ({}x{})",
                row, col, self.header.rows, self.header.cols
            )));
        }

        let offset = self.header.grid_offset as usize;
        let index = (row as usize * self.header.cols as usize) + col as usize;

        if offset + index >= self.size {
            return Err(IpcError::InvalidData("Cell index out of bounds".to_string()));
        }

        let cell_ptr = unsafe { self.mmap.add(offset + index) };
        let byte = unsafe { *cell_ptr };

        Ok(byte as char)
    }
}

#[cfg(target_family = "unix")]
impl Drop for DaemonSharedMemory {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.mmap as *mut libc::c_void, self.size);
            libc::close(self.shm_fd);
        }
    }
}

// SAFETY: The shared memory is read-only and we handle synchronization via sequence numbers
#[cfg(target_family = "unix")]
#[allow(unsafe_code)]
unsafe impl Send for DaemonSharedMemory {}

/// Main test harness for split-process terminal daemon testing.
///
/// Combines IPC communication and shared memory reading into a single
/// ergonomic API for integration testing.
#[cfg(target_family = "unix")]
#[derive(Debug)]
pub struct DaemonTestHarness {
    ipc: DaemonIpcClient,
    shm: DaemonSharedMemory,
    config: DaemonConfig,
    #[allow(dead_code)]
    daemon_process: Option<std::process::Child>,
}

#[cfg(target_family = "unix")]
impl DaemonTestHarness {
    /// Check if IPC testing is enabled via environment variable.
    pub fn is_enabled() -> bool {
        std::env::var("RTL_IPC_TEST").is_ok()
    }

    /// Create a harness with custom configuration.
    pub fn with_config(config: DaemonConfig) -> IpcResult<Self> {
        let daemon_process = if config.spawn_daemon {
            Some(Self::spawn_daemon(&config)?)
        } else {
            None
        };

        // Connect to IPC socket
        let ipc = DaemonIpcClient::connect_with_serializer(
            &config.socket_path,
            config.custom_serializer,
        )?;

        // Open shared memory
        let shm = DaemonSharedMemory::open(&config.shm_path)?;

        Ok(Self { ipc, shm, config, daemon_process })
    }

    /// Spawn a new daemon process.
    fn spawn_daemon(config: &DaemonConfig) -> IpcResult<std::process::Child> {
        let mut cmd = std::process::Command::new(&config.daemon_command);

        cmd.arg("--socket").arg(&config.socket_path);
        cmd.arg("--shm").arg(&config.shm_path);

        if let Some((cols, rows)) = config.dimensions {
            cmd.arg("--cols").arg(cols.to_string());
            cmd.arg("--rows").arg(rows.to_string());
        }

        for arg in &config.daemon_args {
            cmd.arg(arg);
        }

        cmd.spawn().map_err(IpcError::SpawnFailed)
    }

    /// Send input text to the PTY via IPC.
    pub fn send_input(&mut self, text: &str) -> IpcResult<()> {
        self.ipc.send_text(text)
    }

    /// Send raw bytes to the PTY via IPC.
    pub fn send_bytes(&mut self, bytes: &[u8]) -> IpcResult<()> {
        self.ipc.send_input(bytes)
    }

    /// Resize the terminal.
    pub fn resize(&mut self, cols: u16, rows: u16) -> IpcResult<()> {
        self.ipc.resize(cols, rows)
    }

    /// Request a state refresh from the daemon.
    pub fn refresh(&mut self) -> IpcResult<()> {
        self.ipc.refresh()?;
        self.shm.refresh()
    }

    /// Get the current grid contents as a string.
    pub fn grid_contents(&self) -> IpcResult<String> {
        self.shm.grid_contents()
    }

    /// Get the current cursor position (row, col).
    pub fn cursor_position(&self) -> IpcResult<(u16, u16)> {
        Ok(self.shm.cursor_position())
    }

    /// Get the terminal dimensions (cols, rows).
    pub fn dimensions(&self) -> (u16, u16) {
        self.shm.dimensions()
    }

    /// Check if the grid contains the given text.
    pub fn contains(&self, text: &str) -> IpcResult<bool> {
        self.shm.contains(text)
    }

    /// Wait until the grid contains the specified text.
    pub fn wait_for_text(&mut self, text: &str, timeout: Duration) -> IpcResult<()> {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(50);

        loop {
            self.shm.refresh()?;

            if self.shm.contains(text)? {
                return Ok(());
            }

            if start.elapsed() >= timeout {
                return Err(IpcError::Timeout(timeout));
            }

            std::thread::sleep(poll_interval);
        }
    }

    /// Wait until the grid does NOT contain the specified text.
    pub fn wait_for_text_absent(&mut self, text: &str, timeout: Duration) -> IpcResult<()> {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(50);

        loop {
            self.shm.refresh()?;

            if !self.shm.contains(text)? {
                return Ok(());
            }

            if start.elapsed() >= timeout {
                return Err(IpcError::Timeout(timeout));
            }

            std::thread::sleep(poll_interval);
        }
    }

    /// Wait for a sequence of text strings to appear in order.
    pub fn wait_for_sequence(&mut self, texts: &[&str], timeout: Duration) -> IpcResult<()> {
        let start = Instant::now();

        for text in texts {
            let remaining = timeout.saturating_sub(start.elapsed());
            if remaining.is_zero() {
                return Err(IpcError::Timeout(timeout));
            }
            self.wait_for_text(text, remaining)?;
        }

        Ok(())
    }

    /// Wait for the sequence number to change, indicating a state update.
    pub fn wait_for_update(&mut self, timeout: Duration) -> IpcResult<()> {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(10);
        let initial_seq = self.shm.sequence_number();

        loop {
            self.shm.refresh()?;

            if self.shm.sequence_number() != initial_seq {
                return Ok(());
            }

            if start.elapsed() >= timeout {
                return Err(IpcError::Timeout(timeout));
            }

            std::thread::sleep(poll_interval);
        }
    }

    /// Assert that the grid contains the expected text.
    pub fn assert_contains(&self, text: &str) -> IpcResult<()> {
        if self.shm.contains(text)? {
            Ok(())
        } else {
            Err(IpcError::InvalidData(format!(
                "Expected grid to contain '{}', but it didn't.\nGrid:\n{}",
                text,
                self.shm.grid_contents().unwrap_or_default()
            )))
        }
    }

    /// Get the default timeout from configuration.
    pub fn default_timeout(&self) -> Duration {
        self.config.default_timeout
    }
}

#[cfg(target_family = "unix")]
impl Drop for DaemonTestHarness {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.daemon_process {
            // Try to shut down gracefully
            let _ = self.ipc.send(ControlMessage::Shutdown);
            let _ = child.wait();
        }
    }
}

/// Extension trait for integrating IPC testing with TuiTestHarness.
///
/// This trait provides IPC-specific testing methods that can be used
/// alongside the standard TuiTestHarness functionality.
pub trait DaemonTestExt {
    /// Connect to a daemon for testing with the given config.
    fn connect_daemon(&self, config: DaemonConfig) -> IpcResult<DaemonTestHarness>;

    /// Check if IPC testing mode is enabled.
    fn ipc_enabled(&self) -> bool;
}

impl DaemonTestExt for crate::TuiTestHarness {
    fn connect_daemon(&self, config: DaemonConfig) -> IpcResult<DaemonTestHarness> {
        DaemonTestHarness::with_config(config)
    }

    fn ipc_enabled(&self) -> bool {
        DaemonTestHarness::is_enabled()
    }
}

/// Terminal state reader wrapper for compatibility with existing test patterns.
///
/// This provides a read-only view of the terminal state from shared memory,
/// compatible with ScreenState-like APIs.
#[cfg(target_family = "unix")]
#[derive(Debug)]
pub struct TerminalStateReader<'a> {
    shm: &'a DaemonSharedMemory,
}

#[cfg(target_family = "unix")]
impl<'a> TerminalStateReader<'a> {
    /// Create a new reader from shared memory.
    pub fn new(shm: &'a DaemonSharedMemory) -> Self {
        Self { shm }
    }

    /// Get the grid contents as a string.
    pub fn contents(&self) -> String {
        self.shm.grid_contents().unwrap_or_default()
    }

    /// Check if the contents contain the given text.
    pub fn contains(&self, text: &str) -> bool {
        self.shm.contains(text).unwrap_or(false)
    }

    /// Get cursor position (row, col).
    pub fn cursor_position(&self) -> (u16, u16) {
        self.shm.cursor_position()
    }

    /// Get terminal dimensions (cols, rows).
    pub fn dimensions(&self) -> (u16, u16) {
        self.shm.dimensions()
    }

    /// Get a character at position (row, col).
    pub fn cell_at(&self, row: u16, col: u16) -> Option<char> {
        self.shm.cell_at(row, col).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_message_serialization() {
        let msg = ControlMessage::Input(b"hello".to_vec());
        let bytes = msg.to_bytes();
        assert_eq!(bytes[0], 0); // Type 0 = Input
        assert_eq!(&bytes[1..5], &5u32.to_le_bytes()); // Length
        assert_eq!(&bytes[5..], b"hello");
    }

    #[test]
    fn test_control_message_resize() {
        let msg = ControlMessage::Resize { cols: 80, rows: 24 };
        let bytes = msg.to_bytes();
        assert_eq!(bytes[0], 1); // Type 1 = Resize
        assert_eq!(&bytes[1..3], &80u16.to_le_bytes());
        assert_eq!(&bytes[3..5], &24u16.to_le_bytes());
    }

    #[test]
    fn test_control_message_refresh() {
        let msg = ControlMessage::Refresh;
        let bytes = msg.to_bytes();
        assert_eq!(bytes, vec![2]);
    }

    #[test]
    fn test_control_message_shutdown() {
        let msg = ControlMessage::Shutdown;
        let bytes = msg.to_bytes();
        assert_eq!(bytes, vec![3]);
    }

    #[test]
    fn test_config_builder() {
        let config = DaemonConfig::builder()
            .socket_path("/custom/socket.sock")
            .shm_path("/custom_shm")
            .spawn_daemon(true)
            .daemon_command("my-daemon")
            .dimensions(120, 40)
            .connect_timeout(Duration::from_secs(10))
            .build();

        assert_eq!(config.socket_path, PathBuf::from("/custom/socket.sock"));
        assert_eq!(config.shm_path, "/custom_shm");
        assert!(config.spawn_daemon);
        assert_eq!(config.daemon_command, "my-daemon");
        assert_eq!(config.dimensions, Some((120, 40)));
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_shm_header_validation() {
        let valid_header = ShmHeader {
            magic: ShmHeader::DEFAULT_MAGIC,
            version: ShmHeader::DEFAULT_VERSION,
            cols: 80,
            rows: 24,
            cursor_col: 0,
            cursor_row: 0,
            sequence_number: 1,
            grid_offset: 36,
            grid_size: 1920,
            attrs_offset: 0,
            attrs_size: 0,
        };
        assert!(valid_header.validate().is_ok());

        let invalid_magic = ShmHeader {
            magic: 0xDEADBEEF,
            ..valid_header
        };
        assert!(invalid_magic.validate().is_err());

        let invalid_version = ShmHeader {
            version: 999,
            ..valid_header
        };
        assert!(invalid_version.validate().is_err());
    }

    #[test]
    fn test_custom_magic_validation() {
        let header = ShmHeader {
            magic: 0x5343_5241, // "SCRA"
            version: 1,
            cols: 80,
            rows: 24,
            cursor_col: 0,
            cursor_row: 0,
            sequence_number: 1,
            grid_offset: 36,
            grid_size: 1920,
            attrs_offset: 0,
            attrs_size: 0,
        };

        // Should fail with default magic
        assert!(header.validate().is_err());

        // Should pass with custom magic
        assert!(header.validate_with(0x5343_5241, 1).is_ok());
    }

    #[test]
    fn test_is_enabled_false() {
        // Clear the env var if set
        std::env::remove_var("RTL_IPC_TEST");
        assert!(!DaemonTestHarness::is_enabled());
    }
}
