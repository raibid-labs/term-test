//! Hybrid Bevy harness for combined in-process and daemon testing.
//!
//! This module provides [`HybridBevyHarness`], which combines:
//! 1. In-process Bevy ECS testing (like [`HeadlessBevyRunner`])
//! 2. Optional PTY-backed child process for testing daemon/client scenarios
//!
//! This enables testing architectures where a Bevy client communicates with
//! a daemon process running in a separate PTY, allowing full integration testing
//! of distributed TUI applications.
//!
//! # Use Cases
//!
//! - **Client-Server TUI Testing**: Test a Bevy-based client that connects to a daemon
//! - **Daemon Integration**: Launch a background daemon and test client interactions
//! - **Multi-Process Workflows**: Verify behavior across process boundaries
//! - **Hybrid State Validation**: Assert on both ECS state and terminal output simultaneously
//!
//! # Example: Testing Client-Daemon Architecture
//!
//! ```rust,no_run
//! # #[cfg(feature = "bevy")]
//! # {
//! use bevy::prelude::*;
//! use portable_pty::CommandBuilder;
//! use ratatui_testlib::HybridBevyHarness;
//!
//! #[derive(Component)]
//! struct ConnectionState {
//!     connected: bool,
//! }
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! // Create hybrid harness
//! let mut harness = HybridBevyHarness::builder()
//!     .with_dimensions(80, 24)
//!     .with_pty_command(CommandBuilder::new("my-daemon"))
//!     .build()?;
//!
//! // Spawn the daemon in PTY
//! harness.spawn_daemon()?;
//!
//! // Wait for daemon to be ready
//! harness.wait_for_daemon_output(|state| state.contains("Daemon ready"))?;
//!
//! // Set up client ECS state
//! harness
//!     .world_mut()
//!     .spawn(ConnectionState { connected: false });
//!
//! // Run client update to connect to daemon
//! harness.tick()?;
//!
//! // Verify client state
//! let states = harness.query::<ConnectionState>();
//! assert_eq!(states[0].connected, true);
//!
//! // Verify daemon output
//! assert!(harness.daemon_screen().contains("Client connected"));
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! # Architecture
//!
//! The hybrid harness maintains two independent execution contexts:
//!
//! - **In-Process Bevy App**: Full ECS with systems, components, and resources
//! - **Optional PTY Daemon**: Separate process with terminal I/O capture
//!
//! This allows you to test scenarios where a Bevy-based client interacts with
//! a daemon running in a PTY, capturing both ECS state and terminal output.

use std::time::Duration;

use bevy::{
    app::App,
    ecs::{component::Component, world::World},
    prelude::{Entity, With},
    MinimalPlugins,
};
use portable_pty::CommandBuilder;

#[cfg(feature = "sixel")]
use crate::sixel::SixelCapture;
use crate::{
    error::{Result, TermTestError},
    harness::TuiTestHarness,
    screen::ScreenState,
};

/// Builder for configuring a [`HybridBevyHarness`].
///
/// This provides a fluent API for setting up the hybrid harness with custom
/// configurations for both the in-process Bevy app and the optional PTY daemon.
///
/// # Example
///
/// ```rust,no_run
/// # #[cfg(feature = "bevy")]
/// # {
/// use std::time::Duration;
///
/// use portable_pty::CommandBuilder;
/// use ratatui_testlib::HybridBevyHarness;
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let harness = HybridBevyHarness::builder()
///     .with_dimensions(100, 30)
///     .with_pty_command(CommandBuilder::new("daemon"))
///     .with_timeout(Duration::from_secs(10))
///     .build()?;
/// # Ok(())
/// # }
/// # }
/// ```
#[derive(Default)]
pub struct HybridBevyHarnessBuilder {
    width: Option<u16>,
    height: Option<u16>,
    pty_command: Option<CommandBuilder>,
    timeout: Option<Duration>,
    app: Option<App>,
}

impl HybridBevyHarnessBuilder {
    /// Sets the terminal dimensions for the PTY.
    ///
    /// # Arguments
    ///
    /// * `width` - Terminal width in columns
    /// * `height` - Terminal height in rows
    pub fn with_dimensions(mut self, width: u16, height: u16) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Sets the command to spawn in the PTY daemon.
    ///
    /// # Arguments
    ///
    /// * `command` - Command to execute in the PTY
    pub fn with_pty_command(mut self, command: CommandBuilder) -> Self {
        self.pty_command = Some(command);
        self
    }

    /// Sets the timeout for wait operations.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Timeout duration
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets a custom Bevy App for the in-process ECS.
    ///
    /// # Arguments
    ///
    /// * `app` - Pre-configured Bevy App
    pub fn with_app(mut self, app: App) -> Self {
        self.app = Some(app);
        self
    }

    /// Builds the [`HybridBevyHarness`].
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub fn build(self) -> Result<HybridBevyHarness> {
        let width = self.width.unwrap_or(80);
        let height = self.height.unwrap_or(24);

        let mut pty_harness = if self.pty_command.is_some() {
            Some(TuiTestHarness::new(width, height)?)
        } else {
            None
        };

        if let Some(timeout) = self.timeout {
            if let Some(ref mut harness) = pty_harness {
                *harness = std::mem::replace(harness, TuiTestHarness::new(width, height)?)
                    .with_timeout(timeout);
            }
        }

        let app = if let Some(app) = self.app {
            app
        } else {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app
        };

        let screen = ScreenState::new(width, height);

        Ok(HybridBevyHarness {
            app,
            pty_harness,
            pty_command: self.pty_command,
            screen,
            width,
            height,
            daemon_spawned: false,
            #[cfg(feature = "shared-state")]
            shared_state_path: None,
        })
    }
}

/// Hybrid harness combining in-process Bevy ECS with optional PTY daemon.
///
/// This harness enables testing architectures where:
/// - A Bevy-based client runs in-process with full ECS access
/// - An optional daemon process runs in a separate PTY
/// - Client and daemon can communicate (e.g., via network, IPC, files)
///
/// # Comparison with Other Harnesses
///
/// | Feature | HybridBevyHarness | HeadlessBevyRunner | BevyTuiTestHarness |
/// |---------|-------------------|-------------------|-------------------|
/// | In-process ECS | Yes | Yes | Yes |
/// | PTY daemon support | Yes | No | No |
/// | Multi-process testing | Yes | No | Limited |
/// | Client-server testing | Yes | No | No |
/// | Terminal output capture | Daemon only | Optional | Yes |
///
/// # Example: Client-Daemon Testing
///
/// ```rust,no_run
/// # #[cfg(feature = "bevy")]
/// # {
/// use bevy::prelude::*;
/// use portable_pty::CommandBuilder;
/// use ratatui_testlib::HybridBevyHarness;
///
/// #[derive(Component)]
/// struct ClientState {
///     status: String,
/// }
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let mut harness = HybridBevyHarness::builder()
///     .with_pty_command(CommandBuilder::new("my-daemon"))
///     .build()?;
///
/// // Start daemon
/// harness.spawn_daemon()?;
/// harness.wait_for_daemon_output(|s| s.contains("Ready"))?;
///
/// // Set up client
/// harness
///     .world_mut()
///     .spawn(ClientState { status: "connecting".to_string() });
///
/// // Run client logic
/// harness.tick()?;
///
/// // Verify both sides
/// let states = harness.query::<ClientState>();
/// assert_eq!(states[0].status, "connected");
/// assert!(harness.daemon_screen().contains("Client connected"));
/// # Ok(())
/// # }
/// # }
/// ```
pub struct HybridBevyHarness {
    app: App,
    pty_harness: Option<TuiTestHarness>,
    pty_command: Option<CommandBuilder>,
    screen: ScreenState,
    width: u16,
    height: u16,
    daemon_spawned: bool,
    #[cfg(feature = "shared-state")]
    shared_state_path: Option<String>,
}

impl HybridBevyHarness {
    /// Creates a new hybrid harness with default settings (80x24, no daemon).
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::HybridBevyHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = HybridBevyHarness::new()?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// Creates a builder for configuring the harness.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::HybridBevyHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = HybridBevyHarness::builder()
    ///     .with_dimensions(100, 30)
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn builder() -> HybridBevyHarnessBuilder {
        HybridBevyHarnessBuilder::default()
    }

    /// Creates a hybrid harness with custom dimensions.
    ///
    /// # Arguments
    ///
    /// * `width` - Terminal width in columns
    /// * `height` - Terminal height in rows
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    pub fn with_dimensions(width: u16, height: u16) -> Result<Self> {
        Self::builder().with_dimensions(width, height).build()
    }

    /// Configures shared state access for this harness.
    ///
    /// This method sets up memory-mapped shared state access, allowing tests to
    /// inspect application state that's been written to a shared memory file.
    /// Useful for testing applications that expose state via protocols like
    /// scarab-protocol.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the memory-mapped shared state file
    ///
    /// # Errors
    ///
    /// Returns an error if the harness cannot be configured.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(all(feature = "bevy", feature = "shared-state"))]
    /// # {
    /// use ratatui_testlib::HybridBevyHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = HybridBevyHarness::new()?.with_shared_state("/tmp/hybrid_state.mmap")?;
    ///
    /// // Access shared state in tests
    /// harness.tick()?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "shared-state")]
    pub fn with_shared_state(mut self, path: impl Into<String>) -> Result<Self> {
        self.shared_state_path = Some(path.into());
        Ok(self)
    }

    /// Returns the shared state path if configured.
    ///
    /// This provides access to the path of the memory-mapped shared state file,
    /// allowing tests to create their own [`MemoryMappedState`] instances for
    /// custom state types.
    ///
    /// # Returns
    ///
    /// `Some(path)` if shared state was configured, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(all(feature = "bevy", feature = "shared-state"))]
    /// # {
    /// use ratatui_testlib::{
    ///     shared_state::{MemoryMappedState, SharedStateAccess},
    ///     HybridBevyHarness,
    /// };
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct ClientState {
    ///     connected: bool,
    /// }
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = HybridBevyHarness::new()?.with_shared_state("/tmp/client.mmap")?;
    ///
    /// if let Some(path) = harness.shared_state_path() {
    ///     let state = MemoryMappedState::<ClientState>::open(path)?;
    ///     let client = state.read()?;
    ///     assert_eq!(client.connected, true);
    /// }
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "shared-state")]
    pub fn shared_state_path(&self) -> Option<&str> {
        self.shared_state_path.as_deref()
    }

    // ========================================================================
    // In-Process Bevy Methods
    // ========================================================================

    /// Runs one Bevy frame update (ticks all schedules once).
    ///
    /// This executes all Bevy systems registered in the Update schedule
    /// for the in-process client.
    ///
    /// # Errors
    ///
    /// Returns an error if the update fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::HybridBevyHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = HybridBevyHarness::new()?;
    /// harness.tick()?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn tick(&mut self) -> Result<()> {
        self.app.update();
        Ok(())
    }

    /// Runs N Bevy frame updates.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of frames to tick
    ///
    /// # Errors
    ///
    /// Returns an error if any update fails.
    pub fn tick_n(&mut self, count: usize) -> Result<()> {
        for _ in 0..count {
            self.tick()?;
        }
        Ok(())
    }

    /// Returns a reference to the Bevy World for ECS access.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::HybridBevyHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = HybridBevyHarness::new()?;
    /// let entity_count = harness.world().entities().len();
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn world(&self) -> &World {
        self.app.world()
    }

    /// Returns a mutable reference to the Bevy World.
    ///
    /// Use this to spawn entities, insert resources, or mutate world state.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::HybridBevyHarness;
    ///
    /// #[derive(Component)]
    /// struct Player;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = HybridBevyHarness::new()?;
    /// harness.world_mut().spawn(Player);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }

    /// Returns a mutable reference to the Bevy App.
    ///
    /// Use this to add systems, plugins, or configure the app.
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    /// Queries for all entities with a specific component.
    ///
    /// Returns a vector of references to all components of type `T`.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Component type to query for
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::HybridBevyHarness;
    ///
    /// #[derive(Component)]
    /// struct Health(u32);
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = HybridBevyHarness::new()?;
    /// harness.world_mut().spawn(Health(100));
    ///
    /// let all_health = harness.query::<Health>();
    /// assert_eq!(all_health.len(), 1);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn query<T: Component>(&mut self) -> Vec<&T> {
        let world = self.app.world_mut();
        let mut query = world.query::<&T>();
        query.iter(world).collect()
    }

    /// Queries for components filtered by a marker component.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Component type to query for
    /// * `F` - Filter component (entities must have this)
    pub fn query_filtered<T: Component, F: Component>(&mut self) -> Vec<&T> {
        let world = self.app.world_mut();
        let mut query = world.query_filtered::<&T, With<F>>();
        query.iter(world).collect()
    }

    /// Gets a single component by entity ID.
    ///
    /// Returns `None` if the entity doesn't exist or doesn't have the component.
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.app.world().entity(entity).get::<T>()
    }

    /// Asserts that at least one entity with the given component exists.
    ///
    /// # Errors
    ///
    /// Returns an error if no entities with component `T` exist.
    pub fn assert_component_exists<T: Component>(&mut self) -> Result<()> {
        let count = self.query::<T>().len();
        if count == 0 {
            return Err(TermTestError::Bevy(format!(
                "Expected at least one entity with component '{}', but found none",
                std::any::type_name::<T>()
            )));
        }
        Ok(())
    }

    /// Asserts that exactly N entities with the given component exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the count doesn't match.
    pub fn assert_component_count<T: Component>(&mut self, expected_count: usize) -> Result<()> {
        let actual_count = self.query::<T>().len();
        if actual_count != expected_count {
            return Err(TermTestError::Bevy(format!(
                "Expected {} entities with component '{}', but found {}",
                expected_count,
                std::any::type_name::<T>(),
                actual_count
            )));
        }
        Ok(())
    }

    // ========================================================================
    // PTY Daemon Methods
    // ========================================================================

    /// Spawns the daemon process in the PTY.
    ///
    /// This starts the daemon configured via [`HybridBevyHarnessBuilder::with_pty_command`].
    /// The daemon runs in a separate process with its own PTY for terminal I/O.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No PTY command was configured
    /// - Daemon is already running
    /// - Process spawn fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use portable_pty::CommandBuilder;
    /// use ratatui_testlib::HybridBevyHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = HybridBevyHarness::builder()
    ///     .with_pty_command(CommandBuilder::new("my-daemon"))
    ///     .build()?;
    ///
    /// harness.spawn_daemon()?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn spawn_daemon(&mut self) -> Result<()> {
        if self.daemon_spawned {
            return Err(TermTestError::ProcessAlreadyRunning);
        }

        let harness = self
            .pty_harness
            .as_mut()
            .ok_or_else(|| TermTestError::SpawnFailed("No PTY command configured".to_string()))?;

        let cmd = self
            .pty_command
            .clone()
            .ok_or_else(|| TermTestError::SpawnFailed("No PTY command configured".to_string()))?;

        harness.spawn(cmd)?;
        self.daemon_spawned = true;
        Ok(())
    }

    /// Sends text to the daemon PTY.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to send to daemon
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No daemon is running
    /// - Write fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use portable_pty::CommandBuilder;
    /// use ratatui_testlib::HybridBevyHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = HybridBevyHarness::builder()
    ///     .with_pty_command(CommandBuilder::new("daemon"))
    ///     .build()?;
    ///
    /// harness.spawn_daemon()?;
    /// harness.send_to_daemon("status\n")?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn send_to_daemon(&mut self, text: &str) -> Result<()> {
        let harness = self
            .pty_harness
            .as_mut()
            .ok_or(TermTestError::NoProcessRunning)?;
        harness.send_text(text)
    }

    /// Waits for the daemon's terminal output to match a condition.
    ///
    /// This repeatedly checks the daemon's screen state until the condition is met
    /// or the timeout expires.
    ///
    /// # Arguments
    ///
    /// * `condition` - Condition function that receives the daemon's screen state
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No daemon is running
    /// - Timeout expires before condition is met
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use portable_pty::CommandBuilder;
    /// use ratatui_testlib::HybridBevyHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = HybridBevyHarness::builder()
    ///     .with_pty_command(CommandBuilder::new("daemon"))
    ///     .build()?;
    ///
    /// harness.spawn_daemon()?;
    /// harness.wait_for_daemon_output(|state| state.contains("Ready"))?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn wait_for_daemon_output<F>(&mut self, condition: F) -> Result<()>
    where
        F: Fn(&ScreenState) -> bool,
    {
        let harness = self
            .pty_harness
            .as_mut()
            .ok_or(TermTestError::NoProcessRunning)?;
        harness.wait_for(condition)
    }

    /// Waits for specific text to appear in the daemon's output.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to search for
    ///
    /// # Errors
    ///
    /// Returns an error if the text doesn't appear within the timeout.
    pub fn wait_for_daemon_text(&mut self, text: &str) -> Result<()> {
        self.wait_for_daemon_output(|state| state.contains(text))
    }

    /// Returns the daemon's current screen state.
    ///
    /// # Errors
    ///
    /// Returns an error if no daemon is running.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use portable_pty::CommandBuilder;
    /// use ratatui_testlib::HybridBevyHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = HybridBevyHarness::builder()
    ///     .with_pty_command(CommandBuilder::new("daemon"))
    ///     .build()?;
    ///
    /// harness.spawn_daemon()?;
    /// let screen = harness.daemon_screen()?;
    /// assert!(screen.contains("Daemon"));
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn daemon_screen(&self) -> Result<&ScreenState> {
        self.pty_harness
            .as_ref()
            .ok_or(TermTestError::NoProcessRunning)
            .map(|h| h.state())
    }

    /// Returns the daemon's screen contents as a string.
    ///
    /// # Errors
    ///
    /// Returns an error if no daemon is running.
    pub fn daemon_screen_contents(&self) -> Result<String> {
        Ok(self.daemon_screen()?.contents())
    }

    /// Checks if the daemon process is currently running.
    ///
    /// Note: This requires mutable access to check the PTY status.
    pub fn is_daemon_running(&mut self) -> bool {
        if !self.daemon_spawned {
            return false;
        }
        self.pty_harness
            .as_mut()
            .map(|h| {
                // Check if terminal has a running child process
                h.is_running()
            })
            .unwrap_or(false)
    }

    /// Checks if the daemon process has a PTY configured (not necessarily running).
    pub fn has_pty_daemon(&self) -> bool {
        self.pty_harness.is_some()
    }

    // ========================================================================
    // Client Screen State (for bevy_ratatui integration)
    // ========================================================================

    /// Returns the in-process client's screen state.
    ///
    /// This is for integrating with bevy_ratatui or custom rendering systems
    /// that output to the internal ScreenState. By default, this is empty unless
    /// you manually feed terminal output via [`feed_client_output`].
    pub fn client_screen(&self) -> &ScreenState {
        &self.screen
    }

    /// Feeds terminal output bytes into the client's screen state.
    ///
    /// Use this to populate the client screen state with terminal escape
    /// sequences generated by your Bevy systems.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Raw terminal output bytes (including ANSI/VT sequences)
    pub fn feed_client_output(&mut self, bytes: &[u8]) {
        self.screen.feed(bytes);
    }

    /// Returns the client's screen contents as a string.
    pub fn client_screen_contents(&self) -> String {
        self.screen.contents()
    }

    /// Returns terminal dimensions (width, height).
    pub fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    // ========================================================================
    // Sixel Support
    // ========================================================================

    /// Checks if Sixel graphics are present in the daemon's screen state.
    #[cfg(feature = "sixel")]
    pub fn daemon_has_sixel_graphics(&self) -> Result<bool> {
        Ok(!self.daemon_screen()?.sixel_regions().is_empty())
    }

    /// Checks if Sixel graphics are present in the client's screen state.
    #[cfg(feature = "sixel")]
    pub fn client_has_sixel_graphics(&self) -> bool {
        !self.screen.sixel_regions().is_empty()
    }

    /// Captures the daemon's Sixel state.
    #[cfg(feature = "sixel")]
    pub fn capture_daemon_sixel_state(&self) -> Result<SixelCapture> {
        Ok(SixelCapture::from_screen_state(self.daemon_screen()?))
    }

    /// Captures the client's Sixel state.
    #[cfg(feature = "sixel")]
    pub fn capture_client_sixel_state(&self) -> Result<SixelCapture> {
        Ok(SixelCapture::from_screen_state(&self.screen))
    }

    /// Asserts that all daemon Sixel graphics are within the specified area.
    #[cfg(feature = "sixel")]
    pub fn assert_daemon_sixel_within(&self, area: (u16, u16, u16, u16)) -> Result<()> {
        let capture = self.capture_daemon_sixel_state()?;
        capture.assert_all_within(area)
    }

    /// Asserts that all client Sixel graphics are within the specified area.
    #[cfg(feature = "sixel")]
    pub fn assert_client_sixel_within(&self, area: (u16, u16, u16, u16)) -> Result<()> {
        let capture = self.capture_client_sixel_state()?;
        capture.assert_all_within(area)
    }
}

// ============================================================================
// Benchmarking Support (Issue #13)
// ============================================================================

impl crate::bevy::bench::BenchmarkableHarness for HybridBevyHarness {
    fn tick_once(&mut self) -> Result<()> {
        self.tick()
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;

    #[derive(Component)]
    struct TestComponent(u32);

    #[derive(Component)]
    struct ClientState {
        status: String,
    }

    #[derive(Component)]
    struct TestMarker;

    #[test]
    fn test_create_hybrid_harness() {
        let result = HybridBevyHarness::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_with_dimensions() {
        let harness = HybridBevyHarness::builder()
            .with_dimensions(100, 30)
            .build()
            .unwrap();
        assert_eq!(harness.dimensions(), (100, 30));
    }

    #[test]
    fn test_tick() {
        let mut harness = HybridBevyHarness::new().unwrap();
        assert!(harness.tick().is_ok());
    }

    #[test]
    fn test_tick_n() {
        let mut harness = HybridBevyHarness::new().unwrap();
        assert!(harness.tick_n(5).is_ok());
    }

    #[test]
    fn test_spawn_and_query() {
        let mut harness = HybridBevyHarness::new().unwrap();
        harness.world_mut().spawn(TestComponent(42));

        let components = harness.query::<TestComponent>();
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].0, 42);
    }

    #[test]
    fn test_system_execution() {
        fn increment(mut query: Query<&mut TestComponent>) {
            for mut comp in query.iter_mut() {
                comp.0 += 1;
            }
        }

        let mut harness = HybridBevyHarness::new().unwrap();
        harness.app_mut().add_systems(Update, increment);
        harness.world_mut().spawn(TestComponent(0));

        harness.tick().unwrap();
        let components = harness.query::<TestComponent>();
        assert_eq!(components[0].0, 1);

        harness.tick_n(5).unwrap();
        let components = harness.query::<TestComponent>();
        assert_eq!(components[0].0, 6);
    }

    #[test]
    fn test_query_filtered() {
        let mut harness = HybridBevyHarness::new().unwrap();
        harness.world_mut().spawn((TestComponent(10), TestMarker));
        harness.world_mut().spawn(TestComponent(20));

        let marked = harness.query_filtered::<TestComponent, TestMarker>();
        assert_eq!(marked.len(), 1);
        assert_eq!(marked[0].0, 10);
    }

    #[test]
    fn test_assert_component_exists() {
        let mut harness = HybridBevyHarness::new().unwrap();

        assert!(harness.assert_component_exists::<TestComponent>().is_err());

        harness.world_mut().spawn(TestComponent(1));

        assert!(harness.assert_component_exists::<TestComponent>().is_ok());
    }

    #[test]
    fn test_assert_component_count() {
        let mut harness = HybridBevyHarness::new().unwrap();

        assert!(harness.assert_component_count::<TestComponent>(0).is_ok());
        assert!(harness.assert_component_count::<TestComponent>(1).is_err());

        harness.world_mut().spawn(TestComponent(1));
        harness.world_mut().spawn(TestComponent(2));

        assert!(harness.assert_component_count::<TestComponent>(2).is_ok());
        assert!(harness.assert_component_count::<TestComponent>(1).is_err());
    }

    #[test]
    fn test_get_component() {
        let mut harness = HybridBevyHarness::new().unwrap();
        let entity = harness.world_mut().spawn(TestComponent(99)).id();

        let component = harness.get_component::<TestComponent>(entity);
        assert!(component.is_some());
        assert_eq!(component.unwrap().0, 99);
    }

    #[test]
    fn test_feed_client_output() {
        let mut harness = HybridBevyHarness::new().unwrap();
        harness.feed_client_output(b"Hello Client");

        let contents = harness.client_screen_contents();
        assert!(contents.contains("Hello Client"));
    }

    #[test]
    fn test_client_screen() {
        let mut harness = HybridBevyHarness::new().unwrap();
        harness.feed_client_output(b"Test Output");

        let screen = harness.client_screen();
        assert!(screen.contains("Test Output"));
    }

    #[test]
    fn test_has_pty_daemon_false_by_default() {
        let harness = HybridBevyHarness::new().unwrap();
        assert!(!harness.has_pty_daemon());
    }

    #[test]
    fn test_has_pty_daemon_with_command() {
        let harness = HybridBevyHarness::builder()
            .with_pty_command(CommandBuilder::new("echo"))
            .build()
            .unwrap();
        assert!(harness.has_pty_daemon());
    }

    #[test]
    fn test_is_daemon_running_false_initially() {
        let mut harness = HybridBevyHarness::builder()
            .with_pty_command(CommandBuilder::new("echo"))
            .build()
            .unwrap();
        assert!(!harness.is_daemon_running());
    }

    #[test]
    fn test_spawn_daemon_without_command() {
        let mut harness = HybridBevyHarness::new().unwrap();
        let result = harness.spawn_daemon();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TermTestError::SpawnFailed(_)));
    }

    #[test]
    fn test_send_to_daemon_without_daemon() {
        let mut harness = HybridBevyHarness::new().unwrap();
        let result = harness.send_to_daemon("test");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TermTestError::NoProcessRunning));
    }

    #[test]
    fn test_daemon_screen_without_daemon() {
        let harness = HybridBevyHarness::new().unwrap();
        let result = harness.daemon_screen();
        assert!(result.is_err());
        match result {
            Err(TermTestError::NoProcessRunning) => (),
            _ => panic!("Expected NoProcessRunning error"),
        }
    }

    #[test]
    fn test_daemon_screen_contents_without_daemon() {
        let harness = HybridBevyHarness::new().unwrap();
        let result = harness.daemon_screen_contents();
        assert!(result.is_err());
    }

    #[test]
    fn test_wait_for_daemon_output_without_daemon() {
        let mut harness = HybridBevyHarness::new().unwrap();
        let result = harness.wait_for_daemon_output(|_| true);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), TermTestError::NoProcessRunning));
    }

    #[test]
    fn test_with_custom_app() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);

        let harness = HybridBevyHarness::builder().with_app(app).build().unwrap();

        assert!(harness.world().entities().len() >= 0);
    }

    #[test]
    fn test_dimensions() {
        let harness = HybridBevyHarness::with_dimensions(120, 40).unwrap();
        assert_eq!(harness.dimensions(), (120, 40));
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_client_has_sixel_graphics_initially_false() {
        let harness = HybridBevyHarness::new().unwrap();
        assert!(!harness.client_has_sixel_graphics());
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_client_has_sixel_graphics_after_feed() {
        let mut harness = HybridBevyHarness::new().unwrap();
        harness.feed_client_output(b"\x1b[10;10H\x1bPq\"1;1;100;50#0~\x1b\\");

        assert!(harness.client_has_sixel_graphics());
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_capture_client_sixel_state() {
        let mut harness = HybridBevyHarness::new().unwrap();
        harness.feed_client_output(b"\x1b[5;10H\x1bPq\"1;1;200;150#0~\x1b\\");

        let capture = harness.capture_client_sixel_state().unwrap();
        assert!(!capture.is_empty());
        assert_eq!(capture.sequences().len(), 1);
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_daemon_has_sixel_graphics_without_daemon() {
        let harness = HybridBevyHarness::new().unwrap();
        let result = harness.daemon_has_sixel_graphics();
        assert!(result.is_err());
    }

    // ========================================================================
    // Benchmarking Tests (Issue #13)
    // ========================================================================

    #[test]
    fn test_benchmark_rendering() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut harness = HybridBevyHarness::new().unwrap();

        let results = harness.benchmark_rendering(100).unwrap();

        assert_eq!(results.iterations, 100);
        assert!(results.total_duration_ms > 0.0);
        assert!(results.avg_frame_time_ms > 0.0);
        assert!(results.fps_avg > 0.0);
    }

    #[test]
    fn test_profile_update_cycle() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut harness = HybridBevyHarness::new().unwrap();

        let profile = harness.profile_update_cycle().unwrap();

        assert!(profile.duration_ms > 0.0);
        assert!(profile.fps_equivalent > 0.0);
    }

    #[test]
    fn test_assert_fps() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut harness = HybridBevyHarness::new().unwrap();

        let results = harness.assert_fps(1.0, 50);
        assert!(results.is_ok());
    }

    #[test]
    fn test_benchmark_with_systems() {
        use crate::bevy::bench::BenchmarkableHarness;

        fn increment(mut query: Query<&mut TestComponent>) {
            for mut comp in query.iter_mut() {
                comp.0 += 1;
            }
        }

        let mut harness = HybridBevyHarness::new().unwrap();
        harness.app_mut().add_systems(Update, increment);
        harness.world_mut().spawn(TestComponent(0));

        let results = harness.benchmark_rendering(50).unwrap();
        assert_eq!(results.iterations, 50);

        let components = harness.query::<TestComponent>();
        assert_eq!(components[0].0, 50);
    }

    #[test]
    fn test_combined_client_daemon_workflow() {
        // This test demonstrates the typical workflow without actually spawning a daemon
        let mut harness = HybridBevyHarness::new().unwrap();

        // Set up client state
        harness
            .world_mut()
            .spawn(ClientState { status: "initializing".to_string() });

        // Verify client state
        let states = harness.query::<ClientState>();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].status, "initializing");

        // Feed some client output
        harness.feed_client_output(b"Client started\n");
        assert!(harness.client_screen_contents().contains("Client started"));

        // Run client tick
        harness.tick().unwrap();

        // Verify dimensions work
        assert_eq!(harness.dimensions(), (80, 24));
    }
}
