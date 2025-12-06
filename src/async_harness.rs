//! Async test harness for TUI applications using Tokio.
//!
//! This module provides the [`AsyncTuiTestHarness`] which wraps the synchronous
//! [`TuiTestHarness`] to provide an async API compatible with the Tokio runtime.
//!
//! # Key Features
//!
//! - **Async/Await API**: Native async methods for spawning, sending input, and waiting.
//! - **Non-blocking I/O**: Uses `spawn_blocking` to handle PTY operations without blocking the
//!   runtime.
//! - **Advanced Wait Conditions**: Support for custom timeouts, polling intervals, and multiple
//!   conditions.
//!
//! # Example
//!
//! ```rust,no_run
//! # #[cfg(feature = "async-tokio")]
//! # async fn test() -> ratatui_testlib::Result<()> {
//! use portable_pty::CommandBuilder;
//! use ratatui_testlib::AsyncTuiTestHarness;
//!
//! let mut harness = AsyncTuiTestHarness::new(80, 24).await?;
//! let mut cmd = CommandBuilder::new("echo");
//! cmd.arg("hello");
//! harness.spawn(cmd).await?;
//!
//! harness.wait_for_text("hello").await?;
//! # Ok(())
//! # }
//! ```

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use portable_pty::CommandBuilder;
use tokio::task::spawn_blocking;

use crate::{
    error::{Result, TermTestError},
    events::{KeyCode, Modifiers, MouseButton, MouseEvent, ScrollDirection},
    navigation::{HintLabel, NavigationTestExt},
    screen::ScreenState,
    TuiTestHarness,
};

/// Result of a wait operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    /// The condition was met.
    Ok,
    /// One of multiple conditions was met (index of the condition).
    Condition(usize),
    /// The operation timed out.
    Timeout(u64),
}

/// Async wrapper around [`TuiTestHarness`].
///
/// This struct provides an async interface for testing TUI applications.
/// It wraps the blocking `TuiTestHarness` in an `Arc<Mutex<...>>` and uses
/// `spawn_blocking` for operations that involve PTY I/O.
///
/// # Thread Safety
///
/// The harness is thread-safe and can be cloned to share access across tasks.
#[derive(Clone)]
pub struct AsyncTuiTestHarness {
    inner: Arc<Mutex<TuiTestHarness>>,
}

impl AsyncTuiTestHarness {
    /// Creates a new async test harness.
    ///
    /// # Arguments
    ///
    /// * `width` - Terminal width
    /// * `height` - Terminal height
    pub async fn new(width: u16, height: u16) -> Result<Self> {
        let harness = spawn_blocking(move || TuiTestHarness::new(width, height)).await??;
        Ok(Self { inner: Arc::new(Mutex::new(harness)) })
    }

    /// Get visible hints asynchronously.
    pub async fn visible_hints(&self) -> Vec<HintLabel> {
        let inner = self.inner.clone();
        spawn_blocking(move || inner.lock().unwrap().visible_hints())
            .await
            .unwrap()
    }

    /// Spawns a process in the PTY.
    pub async fn spawn(&mut self, cmd: CommandBuilder) -> Result<()> {
        let inner = self.inner.clone();
        spawn_blocking(move || inner.lock().unwrap().spawn(cmd)).await??;
        Ok(())
    }

    /// Sends text to the PTY.
    pub async fn send_text(&mut self, text: &str) -> Result<()> {
        let inner = self.inner.clone();
        let text = text.to_string();
        spawn_blocking(move || inner.lock().unwrap().send_text(&text)).await??;
        Ok(())
    }

    /// Types a text string by sending each character as a key event.
    ///
    /// This is an alias for `send_keys` in the synchronous harness.
    pub async fn type_text(&mut self, text: &str) -> Result<()> {
        let inner = self.inner.clone();
        let text = text.to_string();
        spawn_blocking(move || inner.lock().unwrap().type_text(&text)).await??;
        Ok(())
    }

    /// Sends a key event.
    pub async fn send_key(&mut self, key: KeyCode) -> Result<()> {
        let inner = self.inner.clone();
        spawn_blocking(move || inner.lock().unwrap().send_key(key)).await??;
        Ok(())
    }

    /// Sends a key with modifiers.
    pub async fn send_key_with_modifiers(
        &mut self,
        key: KeyCode,
        modifiers: Modifiers,
    ) -> Result<()> {
        let inner = self.inner.clone();
        spawn_blocking(move || {
            inner
                .lock()
                .unwrap()
                .send_key_with_modifiers(key, modifiers)
        })
        .await??;
        Ok(())
    }

    /// Sends a mouse event.
    pub async fn send_mouse_event(&mut self, event: MouseEvent) -> Result<()> {
        let inner = self.inner.clone();
        spawn_blocking(move || inner.lock().unwrap().send_mouse_event(event)).await??;
        Ok(())
    }

    /// Simulates a mouse click.
    pub async fn mouse_click(&mut self, x: u16, y: u16, button: MouseButton) -> Result<()> {
        let inner = self.inner.clone();
        spawn_blocking(move || inner.lock().unwrap().mouse_click(x, y, button)).await??;
        Ok(())
    }

    /// Simulates a mouse drag.
    pub async fn mouse_drag(
        &mut self,
        start_x: u16,
        start_y: u16,
        end_x: u16,
        end_y: u16,
        button: MouseButton,
    ) -> Result<()> {
        let inner = self.inner.clone();
        spawn_blocking(move || {
            inner
                .lock()
                .unwrap()
                .mouse_drag(start_x, start_y, end_x, end_y, button)
        })
        .await??;
        Ok(())
    }

    /// Simulates a mouse scroll.
    pub async fn mouse_scroll(&mut self, x: u16, y: u16, direction: ScrollDirection) -> Result<()> {
        let inner = self.inner.clone();
        spawn_blocking(move || inner.lock().unwrap().mouse_scroll(x, y, direction)).await??;
        Ok(())
    }

    /// Waits for specific text to appear.
    pub async fn wait_for_text(&mut self, text: &str) -> Result<()> {
        let text = text.to_string();
        self.wait_for_async(move |state| state.contains(&text))
            .execute()
            .await
    }

    /// Waits for a condition with custom configuration.
    ///
    /// Returns a builder for configuring the wait operation.
    pub fn wait_for_async<F>(&self, condition: F) -> AsyncWaitBuilder<F>
    where
        F: Fn(&ScreenState) -> bool + Send + Sync + 'static,
    {
        AsyncWaitBuilder::new(self.inner.clone(), condition)
    }

    /// Waits for any of multiple conditions.
    ///
    /// Returns a builder for configuring the wait operation.
    pub fn wait_for_any_async(&self) -> AsyncWaitAnyBuilder {
        AsyncWaitAnyBuilder::new(self.inner.clone())
    }

    /// Returns the current screen contents.
    pub async fn screen_contents(&self) -> String {
        let inner = self.inner.clone();
        spawn_blocking(move || inner.lock().unwrap().screen_contents())
            .await
            .unwrap()
    }

    /// Resizes the terminal.
    pub async fn resize(&mut self, width: u16, height: u16) -> Result<()> {
        let inner = self.inner.clone();
        spawn_blocking(move || inner.lock().unwrap().resize(width, height)).await??;
        Ok(())
    }
}

/// Builder for single condition async wait.
pub struct AsyncWaitBuilder<F> {
    harness: Arc<Mutex<TuiTestHarness>>,
    condition: F,
    timeout: Duration,
    poll_interval: Duration,
}

impl<F> AsyncWaitBuilder<F>
where
    F: Fn(&ScreenState) -> bool + Send + Sync + 'static,
{
    fn new(harness: Arc<Mutex<TuiTestHarness>>, condition: F) -> Self {
        Self {
            harness,
            condition,
            timeout: Duration::from_secs(5),
            poll_interval: Duration::from_millis(50),
        }
    }

    /// Sets the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the poll interval.
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Executes the wait operation.
    pub async fn execute(self) -> Result<()> {
        let start = tokio::time::Instant::now();
        let mut interval = tokio::time::interval(self.poll_interval);
        let condition = Arc::new(self.condition);

        loop {
            interval.tick().await;

            let harness = self.harness.clone();
            let cond = condition.clone();

            let is_met = spawn_blocking(move || {
                let mut h = harness.lock().unwrap();
                match h.update_state() {
                    Ok(_) | Err(TermTestError::ProcessExited) => {}
                    Err(e) => return Err(e),
                }
                Ok(cond(h.state()))
            })
            .await??;

            if is_met {
                return Ok(());
            }

            if start.elapsed() >= self.timeout {
                return Err(TermTestError::Timeout {
                    timeout_ms: self.timeout.as_millis() as u64,
                });
            }
        }
    }
}

/// Builder for multiple condition async wait.
pub struct AsyncWaitAnyBuilder {
    harness: Arc<Mutex<TuiTestHarness>>,
    conditions: Vec<Box<dyn Fn(&ScreenState) -> bool + Send + Sync>>,
    timeout: Duration,
    poll_interval: Duration,
}

impl AsyncWaitAnyBuilder {
    fn new(harness: Arc<Mutex<TuiTestHarness>>) -> Self {
        Self {
            harness,
            conditions: Vec::new(),
            timeout: Duration::from_secs(5),
            poll_interval: Duration::from_millis(50),
        }
    }

    /// Adds a condition to wait for.
    pub fn add_condition<F>(mut self, condition: F) -> Self
    where
        F: Fn(&ScreenState) -> bool + Send + Sync + 'static,
    {
        self.conditions.push(Box::new(condition));
        self
    }

    /// Sets the timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the poll interval.
    pub fn poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Executes the wait operation.
    pub async fn execute(self) -> Result<WaitResult> {
        let start = tokio::time::Instant::now();
        let mut interval = tokio::time::interval(self.poll_interval);
        let conditions = Arc::new(self.conditions);

        loop {
            interval.tick().await;

            let harness = self.harness.clone();
            let conditions = conditions.clone();

            let matched_index = spawn_blocking(move || {
                let mut h = harness.lock().unwrap();
                // Update state
                match h.update_state() {
                    Ok(_) | Err(TermTestError::ProcessExited) => {}
                    Err(e) => return Err(e),
                }

                // Check all conditions
                for (i, cond) in conditions.iter().enumerate() {
                    if cond(h.state()) {
                        return Ok(Some(i));
                    }
                }
                Ok(None)
            })
            .await??;

            if let Some(idx) = matched_index {
                return Ok(WaitResult::Condition(idx));
            }

            if start.elapsed() >= self.timeout {
                return Ok(WaitResult::Timeout(self.timeout.as_millis() as u64));
            }
        }
    }
}
