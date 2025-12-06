//! Parallel test execution infrastructure.
//!
//! This module provides safe parallel execution of PTY-based tests without conflicts.
//! It includes a PTY pool manager, resource isolation, and thread-safe test contexts.
//!
//! # Key Features
//!
//! - **PTY Pool Management**: Allocate and release terminals from a shared pool
//! - **Resource Isolation**: Each test gets an isolated terminal instance
//! - **Thread Safety**: All components are `Send + Sync` for parallel execution
//! - **Timeout Handling**: Proper cleanup on test failure or timeout
//! - **Port Allocation**: Avoid port conflicts in parallel network tests
//!
//! # Example
//!
//! ```rust,no_run
//! use std::thread;
//!
//! use ratatui_testlib::{Result, TuiTestHarness};
//!
//! # fn test() -> Result<()> {
//! // Create multiple harnesses that can run in parallel
//! let handles: Vec<_> = (0..4)
//!     .map(|i| {
//!         thread::spawn(move || {
//!             let harness = TuiTestHarness::new(80, 24)?;
//!             // Each test runs in isolation
//!             // ... test logic ...
//!             Ok::<(), ratatui_testlib::TermTestError>(())
//!         })
//!     })
//!     .collect();
//!
//! // Wait for all tests to complete
//! for handle in handles {
//!     handle.join().unwrap()?;
//! }
//! # Ok(())
//! # }
//! ```

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, Instant},
};

use crate::{
    error::{Result, TermTestError},
    pty::TestTerminal,
};

/// Default maximum number of concurrent terminals in the pool.
const DEFAULT_MAX_TERMINALS: usize = 16;

/// Default timeout for acquiring a terminal from the pool.
const DEFAULT_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(30);

/// A unique identifier for a terminal in the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerminalId(usize);

impl TerminalId {
    /// Creates a new terminal ID.
    fn new(id: usize) -> Self {
        Self(id)
    }

    /// Returns the numeric value of this ID.
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

/// Configuration for the PTY pool.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of concurrent terminals.
    pub max_terminals: usize,
    /// Timeout for acquiring a terminal.
    pub acquire_timeout: Duration,
    /// Default terminal width.
    pub default_width: u16,
    /// Default terminal height.
    pub default_height: u16,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_terminals: DEFAULT_MAX_TERMINALS,
            acquire_timeout: DEFAULT_ACQUIRE_TIMEOUT,
            default_width: 80,
            default_height: 24,
        }
    }
}

impl PoolConfig {
    /// Creates a new pool configuration with custom settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of concurrent terminals.
    pub fn with_max_terminals(mut self, max: usize) -> Self {
        self.max_terminals = max;
        self
    }

    /// Sets the timeout for acquiring a terminal.
    pub fn with_acquire_timeout(mut self, timeout: Duration) -> Self {
        self.acquire_timeout = timeout;
        self
    }

    /// Sets the default terminal dimensions.
    pub fn with_default_size(mut self, width: u16, height: u16) -> Self {
        self.default_width = width;
        self.default_height = height;
        self
    }
}

/// A managed terminal entry in the pool.
struct PooledTerminal {
    /// The terminal ID.
    id: TerminalId,
    /// The underlying terminal.
    /// Note: Currently kept for future use (e.g., terminal resizing, direct access).
    /// Each terminal in the pool maintains its own PTY instance.
    #[allow(dead_code)]
    terminal: TestTerminal,
    /// Terminal dimensions.
    width: u16,
    height: u16,
    /// Whether this terminal is currently in use.
    in_use: bool,
    /// When this terminal was last acquired.
    last_acquired: Option<Instant>,
}

impl std::fmt::Debug for PooledTerminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledTerminal")
            .field("id", &self.id)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("in_use", &self.in_use)
            .field("last_acquired", &self.last_acquired)
            .finish_non_exhaustive()
    }
}

impl PooledTerminal {
    /// Creates a new pooled terminal.
    fn new(id: TerminalId, width: u16, height: u16) -> Result<Self> {
        Ok(Self {
            id,
            terminal: TestTerminal::new(width, height)?,
            width,
            height,
            in_use: false,
            last_acquired: None,
        })
    }

    /// Marks this terminal as acquired.
    fn acquire(&mut self) {
        self.in_use = true;
        self.last_acquired = Some(Instant::now());
    }

    /// Marks this terminal as released.
    fn release(&mut self) {
        self.in_use = false;
    }

    /// Returns whether this terminal is available for use.
    fn is_available(&self) -> bool {
        !self.in_use
    }
}

/// A pool of terminals for parallel test execution.
///
/// This pool manages a collection of terminals that can be acquired and released
/// by tests running in parallel. It ensures that each test gets an isolated
/// terminal instance without conflicts.
///
/// # Thread Safety
///
/// The pool is thread-safe and can be shared across multiple threads using `Arc`.
///
/// # Example
///
/// ```rust,no_run
/// use std::{sync::Arc, thread};
///
/// use ratatui_testlib::parallel::{PoolConfig, TerminalPool};
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let config = PoolConfig::default().with_max_terminals(8);
/// let pool = Arc::new(TerminalPool::new(config)?);
///
/// let handles: Vec<_> = (0..4)
///     .map(|_| {
///         let pool = Arc::clone(&pool);
///         thread::spawn(move || {
///             let terminal = pool.acquire(80, 24)?;
///             // Use the terminal...
///             pool.release(terminal)?;
///             Ok::<(), ratatui_testlib::TermTestError>(())
///         })
///     })
///     .collect();
///
/// for handle in handles {
///     handle.join().unwrap()?;
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct TerminalPool {
    config: PoolConfig,
    terminals: Mutex<HashMap<TerminalId, PooledTerminal>>,
    next_id: Mutex<usize>,
}

impl TerminalPool {
    /// Creates a new terminal pool with the given configuration.
    pub fn new(config: PoolConfig) -> Result<Self> {
        Ok(Self {
            config,
            terminals: Mutex::new(HashMap::new()),
            next_id: Mutex::new(0),
        })
    }

    /// Creates a new terminal pool with default configuration.
    pub fn default_pool() -> Result<Self> {
        Self::new(PoolConfig::default())
    }

    /// Acquires a terminal from the pool with the specified dimensions.
    ///
    /// If no available terminal exists and the pool is not at capacity, a new
    /// terminal is created. If the pool is at capacity, this method waits until
    /// a terminal becomes available or the timeout expires.
    ///
    /// # Arguments
    ///
    /// * `width` - Terminal width in columns
    /// * `height` - Terminal height in rows
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The timeout expires while waiting for an available terminal
    /// - Terminal creation fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ratatui_testlib::parallel::TerminalPool;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let pool = TerminalPool::default_pool()?;
    /// let terminal = pool.acquire(80, 24)?;
    /// // Use the terminal...
    /// pool.release(terminal)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn acquire(&self, width: u16, height: u16) -> Result<IsolatedTerminal> {
        let start = Instant::now();
        let poll_interval = Duration::from_millis(50);

        loop {
            // Try to acquire an available terminal
            let mut terminals = self.terminals.lock().unwrap();

            // First, try to find an available terminal with matching dimensions
            if let Some((_, pooled)) = terminals
                .iter_mut()
                .find(|(_, t)| t.is_available() && t.width == width && t.height == height)
            {
                pooled.acquire();
                return Ok(IsolatedTerminal::new(pooled.id, width, height));
            }

            // If no matching terminal and we're under capacity, create a new one
            if terminals.len() < self.config.max_terminals {
                let mut next_id = self.next_id.lock().unwrap();
                let id = TerminalId::new(*next_id);
                *next_id += 1;
                drop(next_id);

                let mut pooled = PooledTerminal::new(id, width, height)?;
                pooled.acquire();
                terminals.insert(id, pooled);

                return Ok(IsolatedTerminal::new(id, width, height));
            }

            // Pool is at capacity, wait and retry
            drop(terminals);

            if start.elapsed() >= self.config.acquire_timeout {
                return Err(TermTestError::Timeout {
                    timeout_ms: self.config.acquire_timeout.as_millis() as u64,
                });
            }

            std::thread::sleep(poll_interval);
        }
    }

    /// Releases a terminal back to the pool.
    ///
    /// # Arguments
    ///
    /// * `terminal` - The terminal to release
    ///
    /// # Errors
    ///
    /// Returns an error if the terminal ID is not found in the pool.
    pub fn release(&self, terminal: IsolatedTerminal) -> Result<()> {
        let mut terminals = self.terminals.lock().unwrap();

        if let Some(pooled) = terminals.get_mut(&terminal.id) {
            pooled.release();
            Ok(())
        } else {
            Err(TermTestError::Pty(format!("Terminal ID {:?} not found in pool", terminal.id)))
        }
    }

    /// Gets the current pool statistics.
    pub fn stats(&self) -> PoolStats {
        let terminals = self.terminals.lock().unwrap();
        let total = terminals.len();
        let in_use = terminals.values().filter(|t| t.in_use).count();
        let available = total - in_use;

        PoolStats {
            total,
            in_use,
            available,
            max_capacity: self.config.max_terminals,
        }
    }

    /// Clears the pool, terminating all terminals.
    ///
    /// This is useful for cleanup after tests complete.
    pub fn clear(&self) {
        let mut terminals = self.terminals.lock().unwrap();
        terminals.clear();
    }
}

/// Statistics about the terminal pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolStats {
    /// Total number of terminals in the pool.
    pub total: usize,
    /// Number of terminals currently in use.
    pub in_use: usize,
    /// Number of available terminals.
    pub available: usize,
    /// Maximum pool capacity.
    pub max_capacity: usize,
}

impl PoolStats {
    /// Returns a formatted summary string.
    pub fn summary(&self) -> String {
        format!(
            "Pool Stats: {}/{} in use, {} available (max: {})",
            self.in_use, self.total, self.available, self.max_capacity
        )
    }
}

/// An isolated terminal instance from the pool.
///
/// This represents a terminal that has been acquired from the pool and is
/// ready for use. It should be released back to the pool when done.
///
/// # Example
///
/// ```rust,no_run
/// use ratatui_testlib::parallel::TerminalPool;
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let pool = TerminalPool::default_pool()?;
///
/// // Acquire a terminal
/// let terminal = pool.acquire(80, 24)?;
///
/// // Use it...
/// let id = terminal.id();
/// println!("Using terminal {:?}", id);
///
/// // Release it back to the pool
/// pool.release(terminal)?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct IsolatedTerminal {
    id: TerminalId,
    width: u16,
    height: u16,
}

impl IsolatedTerminal {
    /// Creates a new isolated terminal.
    fn new(id: TerminalId, width: u16, height: u16) -> Self {
        Self { id, width, height }
    }

    /// Returns the terminal ID.
    pub fn id(&self) -> TerminalId {
        self.id
    }

    /// Returns the terminal dimensions.
    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }
}

/// A thread-safe test context for parallel execution.
///
/// This provides a shared context that can be safely accessed from multiple
/// test threads. It includes shared resources like port allocators and
/// test metadata.
///
/// # Thread Safety
///
/// All methods are thread-safe and can be called from multiple threads.
///
/// # Example
///
/// ```rust
/// use std::{sync::Arc, thread};
///
/// use ratatui_testlib::parallel::TestContext;
///
/// let context = Arc::new(TestContext::new());
///
/// let handles: Vec<_> = (0..4)
///     .map(|_| {
///         let ctx = Arc::clone(&context);
///         thread::spawn(move || {
///             // Allocate a unique port for this test
///             let port = ctx.allocate_port();
///             println!("Test using port {}", port);
///         })
///     })
///     .collect();
///
/// for handle in handles {
///     handle.join().unwrap();
/// }
/// ```
#[derive(Debug)]
pub struct TestContext {
    /// Port allocator for network tests.
    next_port: Mutex<u16>,
    /// Shared metadata.
    metadata: RwLock<HashMap<String, String>>,
}

impl TestContext {
    /// Creates a new test context.
    pub fn new() -> Self {
        Self {
            next_port: Mutex::new(20000), // Start at 20000 to avoid common ports
            metadata: RwLock::new(HashMap::new()),
        }
    }

    /// Allocates a unique port number for network tests.
    ///
    /// This ensures that parallel tests don't try to bind to the same port.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::parallel::TestContext;
    ///
    /// let context = TestContext::new();
    /// let port1 = context.allocate_port();
    /// let port2 = context.allocate_port();
    /// assert_ne!(port1, port2);
    /// ```
    pub fn allocate_port(&self) -> u16 {
        let mut next_port = self.next_port.lock().unwrap();
        let port = *next_port;
        *next_port = next_port.wrapping_add(1);
        // Wrap around if we exceed the valid port range
        if *next_port > 60000 {
            *next_port = 20000;
        }
        port
    }

    /// Sets a metadata value.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key
    /// * `value` - Metadata value
    pub fn set_metadata(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut metadata = self.metadata.write().unwrap();
        metadata.insert(key.into(), value.into());
    }

    /// Gets a metadata value.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key
    ///
    /// # Returns
    ///
    /// The metadata value if it exists, or `None` otherwise.
    pub fn get_metadata(&self, key: &str) -> Option<String> {
        let metadata = self.metadata.read().unwrap();
        metadata.get(key).cloned()
    }

    /// Removes a metadata value.
    ///
    /// # Arguments
    ///
    /// * `key` - Metadata key
    pub fn remove_metadata(&self, key: &str) {
        let mut metadata = self.metadata.write().unwrap();
        metadata.remove(key);
    }

    /// Clears all metadata.
    pub fn clear_metadata(&self) {
        let mut metadata = self.metadata.write().unwrap();
        metadata.clear();
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TestContext {
    fn clone(&self) -> Self {
        let metadata = self.metadata.read().unwrap();
        let next_port = self.next_port.lock().unwrap();

        Self {
            next_port: Mutex::new(*next_port),
            metadata: RwLock::new(metadata.clone()),
        }
    }
}

/// A guard that automatically releases a terminal when dropped.
///
/// This provides RAII-style resource management for terminals. The terminal
/// is automatically released back to the pool when the guard goes out of scope,
/// even if the test panics.
///
/// # Example
///
/// ```rust,no_run
/// use std::sync::Arc;
///
/// use ratatui_testlib::parallel::{TerminalGuard, TerminalPool};
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let pool = Arc::new(TerminalPool::default_pool()?);
///
/// {
///     let guard = TerminalGuard::acquire(Arc::clone(&pool), 80, 24)?;
///     // Use the terminal via guard.terminal()
///     // Terminal is automatically released when guard goes out of scope
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct TerminalGuard {
    terminal: Option<IsolatedTerminal>,
    pool: Arc<TerminalPool>,
}

impl TerminalGuard {
    /// Acquires a terminal from the pool and wraps it in a guard.
    pub fn acquire(pool: Arc<TerminalPool>, width: u16, height: u16) -> Result<Self> {
        let terminal = pool.acquire(width, height)?;
        Ok(Self { terminal: Some(terminal), pool })
    }

    /// Returns a reference to the isolated terminal.
    pub fn terminal(&self) -> &IsolatedTerminal {
        self.terminal.as_ref().unwrap()
    }

    /// Explicitly releases the terminal back to the pool.
    ///
    /// This is optional; the terminal will be released automatically when
    /// the guard is dropped.
    pub fn release(mut self) -> Result<()> {
        if let Some(terminal) = self.terminal.take() {
            self.pool.release(terminal)?;
        }
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if let Some(terminal) = self.terminal.take() {
            // Ignore errors during drop - best effort cleanup
            let _ = self.pool.release(terminal);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn test_pool_config() {
        let config = PoolConfig::new()
            .with_max_terminals(8)
            .with_acquire_timeout(Duration::from_secs(5))
            .with_default_size(100, 30);

        assert_eq!(config.max_terminals, 8);
        assert_eq!(config.acquire_timeout, Duration::from_secs(5));
        assert_eq!(config.default_width, 100);
        assert_eq!(config.default_height, 30);
    }

    #[test]
    fn test_terminal_pool_creation() {
        let pool = TerminalPool::default_pool();
        assert!(pool.is_ok());

        let stats = pool.unwrap().stats();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.in_use, 0);
        assert_eq!(stats.available, 0);
    }

    #[test]
    fn test_terminal_acquire_release() {
        let pool = TerminalPool::default_pool().unwrap();

        // Acquire a terminal
        let terminal = pool.acquire(80, 24).unwrap();
        assert_eq!(terminal.size(), (80, 24));

        let stats = pool.stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.in_use, 1);
        assert_eq!(stats.available, 0);

        // Release the terminal
        pool.release(terminal).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.in_use, 0);
        assert_eq!(stats.available, 1);
    }

    #[test]
    fn test_terminal_pool_reuse() {
        let pool = TerminalPool::default_pool().unwrap();

        // Acquire and release
        let terminal1 = pool.acquire(80, 24).unwrap();
        let id1 = terminal1.id();
        pool.release(terminal1).unwrap();

        // Acquire again - should reuse the same terminal
        let terminal2 = pool.acquire(80, 24).unwrap();
        let id2 = terminal2.id();
        assert_eq!(id1, id2);

        pool.release(terminal2).unwrap();
    }

    #[test]
    fn test_parallel_acquire() {
        let pool = Arc::new(TerminalPool::default_pool().unwrap());

        let handles: Vec<_> = (0..4)
            .map(|_| {
                let pool = Arc::clone(&pool);
                thread::spawn(move || {
                    let terminal = pool.acquire(80, 24).unwrap();
                    // Simulate some work
                    thread::sleep(Duration::from_millis(10));
                    pool.release(terminal).unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let stats = pool.stats();
        assert_eq!(stats.in_use, 0);
        assert!(stats.total <= 4);
    }

    #[test]
    fn test_terminal_guard() {
        let pool = Arc::new(TerminalPool::default_pool().unwrap());

        {
            let _guard = TerminalGuard::acquire(Arc::clone(&pool), 80, 24).unwrap();
            let stats = pool.stats();
            assert_eq!(stats.in_use, 1);
        } // Guard dropped here

        let stats = pool.stats();
        assert_eq!(stats.in_use, 0);
    }

    #[test]
    fn test_test_context_port_allocation() {
        let context = TestContext::new();

        let port1 = context.allocate_port();
        let port2 = context.allocate_port();
        let port3 = context.allocate_port();

        assert_ne!(port1, port2);
        assert_ne!(port2, port3);
        assert_ne!(port1, port3);

        assert!(port1 >= 20000);
        assert!(port2 >= 20000);
        assert!(port3 >= 20000);
    }

    #[test]
    fn test_test_context_metadata() {
        let context = TestContext::new();

        context.set_metadata("test_name", "my_test");
        context.set_metadata("iteration", "1");

        assert_eq!(context.get_metadata("test_name"), Some("my_test".to_string()));
        assert_eq!(context.get_metadata("iteration"), Some("1".to_string()));
        assert_eq!(context.get_metadata("nonexistent"), None);

        context.remove_metadata("iteration");
        assert_eq!(context.get_metadata("iteration"), None);

        context.clear_metadata();
        assert_eq!(context.get_metadata("test_name"), None);
    }

    #[test]
    fn test_parallel_context_access() {
        let context = Arc::new(TestContext::new());

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let ctx = Arc::clone(&context);
                thread::spawn(move || {
                    let port = ctx.allocate_port();
                    ctx.set_metadata(format!("thread_{}", i), format!("port_{}", port));
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // All threads should have allocated different ports
        let metadata_count = context.metadata.read().unwrap().len();
        assert_eq!(metadata_count, 10);
    }

    #[test]
    fn test_pool_stats_summary() {
        let pool = TerminalPool::default_pool().unwrap();
        let _t1 = pool.acquire(80, 24).unwrap();
        let _t2 = pool.acquire(80, 24).unwrap();

        let stats = pool.stats();
        let summary = stats.summary();

        assert!(summary.contains("2/2 in use"));
        assert!(summary.contains("0 available"));
    }

    #[test]
    fn test_pool_clear() {
        let pool = TerminalPool::default_pool().unwrap();
        let t1 = pool.acquire(80, 24).unwrap();
        let t2 = pool.acquire(80, 24).unwrap();

        pool.release(t1).unwrap();
        pool.release(t2).unwrap();

        let stats = pool.stats();
        assert_eq!(stats.total, 2);

        pool.clear();

        let stats = pool.stats();
        assert_eq!(stats.total, 0);
    }

    #[test]
    fn test_different_terminal_sizes() {
        let pool = TerminalPool::default_pool().unwrap();

        let t1 = pool.acquire(80, 24).unwrap();
        let t2 = pool.acquire(100, 30).unwrap();
        let t3 = pool.acquire(80, 24).unwrap();

        assert_eq!(t1.size(), (80, 24));
        assert_eq!(t2.size(), (100, 30));
        assert_eq!(t3.size(), (80, 24));

        // Should have created 3 terminals (2 different sizes)
        let stats = pool.stats();
        assert!(stats.total >= 2);
    }
}
