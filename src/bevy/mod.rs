//! Bevy ECS integration for testing Bevy-based TUI applications.
//!
//! This module provides three test harnesses for different testing scenarios:
//!
//! ## PTY-Based Testing: [`BevyTuiTestHarness`]
//!
//! Wraps a TUI test harness with Bevy App, spawning your application in a PTY
//! and capturing real terminal output. Use this for full end-to-end testing.
//!
//! ## In-Process Headless Testing: [`HeadlessBevyRunner`]
//!
//! Runs Bevy schedules in-process without spawning a PTY, using `MinimalPlugins`
//! and `ScheduleRunnerPlugin`. Ideal for CI/CD environments, fast unit tests,
//! and deterministic frame-by-frame execution.
//!
//! ## Hybrid Testing: [`HybridBevyHarness`]
//!
//! Combines in-process Bevy ECS testing with an optional PTY-backed daemon process.
//! Perfect for testing client-server architectures where a Bevy client communicates
//! with a daemon process. Provides access to both ECS state and daemon terminal output.
//!
//! # Overview
//!
//! [`BevyTuiTestHarness`] combines terminal testing with Bevy ECS capabilities:
//!
//! - Run Bevy update cycles frame-by-frame
//! - Query ECS entities and components
//! - Test system execution and state transitions
//! - Verify terminal output from Bevy systems
//! - Test Sixel graphics from Bevy rendering systems
//!
//! # Status
//!
//! Wave 3 (Issue #9): Bevy ECS integration complete with component querying and state management.
//! Wave 4 (Issue #16): HeadlessBevyRunner added for in-process CI-friendly testing.
//!
//! # Headless Mode
//!
//! When the `headless` feature flag is enabled, the Bevy app runs without any
//! display dependencies, making it suitable for CI/CD environments:
//!
//! ```bash
//! # Run tests in headless mode (no X11/Wayland required)
//! cargo test --features bevy,headless
//!
//! # Works in Docker without DISPLAY
//! docker run --rm rust:latest cargo test --features bevy,headless
//! ```
//!
//! In headless mode:
//! - Uses Bevy's `MinimalPlugins` instead of `DefaultPlugins`
//! - No windowing or rendering systems
//! - No GPU dependencies
//! - Suitable for GitHub Actions and other CI platforms
//!
//! # Implemented Features
//!
//! - Headless Bevy app initialization ✓
//! - Frame-by-frame update control ✓
//! - ECS entity/component queries ✓
//! - System execution testing ✓
//! - Integration with bevy_ratatui plugin ✓
//!
//! # Example
//!
//! ```rust,no_run
//! # #[cfg(feature = "bevy")]
//! # {
//! use ratatui_testlib::BevyTuiTestHarness;
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! let mut test = BevyTuiTestHarness::new()?;
//!
//! // Run one Bevy frame
//! test.update()?;
//!
//! // Run multiple frames
//! test.update_n(5)?;
//!
//! // Render and check screen state
//! test.render_frame()?;
//! let state = test.state();
//! assert!(state.contains("Game Over"));
//! # Ok(())
//! # }
//! # }
//! ```

// Submodules
pub mod bench;
pub mod headless;
pub mod hybrid;

// Re-exports
pub use bench::{BenchmarkResults, BenchmarkableHarness, ProfileResults};
// Bevy ECS imports
use bevy::app::App;
use bevy::{
    ecs::{component::Component, world::World},
    prelude::{Entity, Update, With},
    MinimalPlugins,
};
pub use headless::HeadlessBevyRunner;
pub use hybrid::{HybridBevyHarness, HybridBevyHarnessBuilder};
// Snapshot testing imports
#[cfg(feature = "snapshot-insta")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "snapshot-insta")]
use serde_json;

#[cfg(feature = "sixel")]
use crate::sixel::SixelCapture;
use crate::{
    error::{Result, TermTestError},
    harness::TuiTestHarness,
    screen::ScreenState,
};

// ============================================================================
// Component Snapshot (Issue #12)
// ============================================================================

/// Snapshot representation of a Bevy component with metadata.
///
/// This struct captures the state of a component for snapshot testing,
/// including its type information, entity ID, and serialized value.
/// It enables regression testing of ECS component state alongside
/// screen snapshots.
///
/// # Type Parameters
///
/// * `T` - The component type being captured (must implement `Serialize`)
///
/// # Examples
///
/// ```rust,no_run
/// # #[cfg(all(feature = "bevy", feature = "snapshot-insta"))]
/// # {
/// use bevy::prelude::*;
/// use ratatui_testlib::BevyTuiTestHarness;
/// use serde::Serialize;
///
/// #[derive(Component, Serialize)]
/// struct Position {
///     x: f32,
///     y: f32,
/// }
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let mut harness = BevyTuiTestHarness::new()?;
/// harness.world_mut().spawn(Position { x: 10.0, y: 20.0 });
///
/// let snapshots = harness.snapshot_components::<Position>();
/// insta::assert_json_snapshot!(snapshots);
/// # Ok(())
/// # }
/// # }
/// ```
#[cfg(feature = "snapshot-insta")]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentSnapshot<T: Serialize> {
    /// Entity ID this component belongs to
    pub entity_id: u32,

    /// Type name of the component
    pub component_type: String,

    /// The component data
    pub data: T,
}

#[cfg(feature = "snapshot-insta")]
impl<T: Serialize> ComponentSnapshot<T> {
    /// Creates a new component snapshot.
    ///
    /// # Arguments
    ///
    /// * `entity` - Entity the component belongs to
    /// * `data` - The component data
    ///
    /// # Returns
    ///
    /// A new `ComponentSnapshot` capturing the component state.
    pub fn new(entity: Entity, data: T) -> Self {
        Self {
            entity_id: entity.index(),
            component_type: std::any::type_name::<T>().to_string(),
            data,
        }
    }

    /// Returns the entity ID.
    pub fn entity_id(&self) -> u32 {
        self.entity_id
    }

    /// Returns the component type name.
    pub fn component_type(&self) -> &str {
        &self.component_type
    }

    /// Returns a reference to the component data.
    pub fn data(&self) -> &T {
        &self.data
    }
}

/// Test harness for Bevy-based TUI applications.
///
/// This combines TUI testing with Bevy ECS querying and update cycle control,
/// specifically designed for testing applications built with bevy_ratatui.
///
/// # Current Status
///
/// Wave 3 implementation complete with full ECS integration for querying components,
/// running Bevy schedules, and testing hybrid Bevy+Ratatui applications.
///
/// # Headless Mode
///
/// When built with the `headless` feature, this harness runs Bevy in headless
/// mode (using `MinimalPlugins`), suitable for CI/CD environments without
/// display servers:
///
/// ```bash
/// cargo test --features bevy,headless
/// ```
///
/// Without the `headless` feature, rendering plugins may attempt to initialize
/// graphics contexts, which could fail in CI environments.
///
/// # Planned Architecture
///
/// - Wraps a [`TuiTestHarness`] for terminal I/O
/// - Contains a headless Bevy App for ECS operations
/// - Provides frame-by-frame control of update cycles
/// - Exposes ECS query methods for testing
///
/// # Example
///
/// ```rust,no_run
/// # #[cfg(feature = "bevy")]
/// # {
/// use ratatui_testlib::BevyTuiTestHarness;
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let mut harness = BevyTuiTestHarness::new()?;
///
/// // Run one frame
/// harness.update()?;
///
/// // Send input
/// harness.send_text("quit\n")?;
///
/// // Wait for result
/// harness.wait_for(|state| state.contains("Goodbye"))?;
/// # Ok(())
/// # }
/// # }
/// ```
pub struct BevyTuiTestHarness {
    harness: TuiTestHarness,
    is_headless: bool,
    app: App,
    #[cfg(feature = "shared-state")]
    shared_state_path: Option<String>,
}

impl BevyTuiTestHarness {
    /// Creates a new Bevy TUI test harness.
    ///
    /// Initializes a new test harness with default terminal dimensions (80x24).
    /// In Phase 4, this will also initialize a headless Bevy App.
    ///
    /// # Headless Mode
    ///
    /// When the `headless` feature is enabled, Bevy will be configured for
    /// headless operation (no display server required). This is the recommended
    /// configuration for CI/CD environments.
    ///
    /// # Errors
    ///
    /// Returns an error if terminal or Bevy initialization fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn new() -> Result<Self> {
        let harness = TuiTestHarness::new(80, 24)?;

        // Determine if running in headless mode
        #[cfg(feature = "headless")]
        let is_headless = true;
        #[cfg(not(feature = "headless"))]
        let is_headless = false;

        // Initialize Bevy App
        // In headless mode, use MinimalPlugins for CI/CD compatibility
        // Otherwise, use MinimalPlugins + ScheduleRunnerPlugin for deterministic testing
        let mut app = App::new();

        #[cfg(feature = "headless")]
        {
            app.add_plugins(MinimalPlugins);
        }

        #[cfg(not(feature = "headless"))]
        {
            // Even in non-headless mode, we use MinimalPlugins for testing
            // to avoid GPU/windowing dependencies
            app.add_plugins(MinimalPlugins);
        }

        Ok(Self {
            harness,
            is_headless,
            app,
            #[cfg(feature = "shared-state")]
            shared_state_path: None,
        })
    }

    /// Creates a Bevy TUI test harness with bevy_ratatui plugin.
    ///
    /// This is a convenience method for the common case of testing
    /// applications built with bevy_ratatui.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[cfg(feature = "bevy-ratatui")]
    pub fn with_bevy_ratatui() -> Result<Self> {
        let mut harness = Self::new()?;

        // Add bevy_ratatui plugin
        harness
            .app
            .add_plugins(bevy_ratatui::RatatuiPlugins::default());

        Ok(harness)
    }

    /// Creates a new harness with a custom Bevy App.
    ///
    /// This allows full control over the Bevy configuration for advanced testing scenarios.
    ///
    /// # Arguments
    ///
    /// * `app` - Pre-configured Bevy App
    ///
    /// # Errors
    ///
    /// Returns an error if harness initialization fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut app = App::new();
    /// app.add_plugins(MinimalPlugins);
    /// // Add custom systems, resources, etc.
    ///
    /// let harness = BevyTuiTestHarness::with_app(app)?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn with_app(app: App) -> Result<Self> {
        let harness = TuiTestHarness::new(80, 24)?;

        #[cfg(feature = "headless")]
        let is_headless = true;
        #[cfg(not(feature = "headless"))]
        let is_headless = false;

        Ok(Self {
            harness,
            is_headless,
            app,
            #[cfg(feature = "shared-state")]
            shared_state_path: None,
        })
    }

    /// Runs one Bevy frame update.
    ///
    /// This executes all Bevy systems for one frame by calling `app.update()`.
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
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness.update()?; // Run one frame
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn update(&mut self) -> Result<()> {
        self.app.update();
        Ok(())
    }

    /// Runs N Bevy frame updates.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of frames to update
    ///
    /// # Errors
    ///
    /// Returns an error if any update fails.
    pub fn update_n(&mut self, count: usize) -> Result<()> {
        for _ in 0..count {
            self.update()?;
        }
        Ok(())
    }

    /// Updates Bevy and renders to the terminal.
    ///
    /// This is equivalent to one complete frame: update ECS, then render to PTY.
    ///
    /// # Errors
    ///
    /// Returns an error if update or render fails.
    pub fn render_frame(&mut self) -> Result<()> {
        // Run Bevy update cycle
        self.update()?;
        // Update terminal screen state
        self.harness.update_state()?;
        Ok(())
    }

    // ========================================================================
    // Bevy ECS Query Methods (Issue #9)
    // ========================================================================

    /// Returns a reference to the Bevy World for direct ECS access.
    ///
    /// This provides low-level access to the ECS world for advanced queries
    /// and operations not covered by the helper methods.
    ///
    /// # Returns
    ///
    /// A reference to the Bevy [`World`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = BevyTuiTestHarness::new()?;
    /// let world = harness.world();
    /// let entity_count = world.entities().len();
    /// println!("Total entities: {}", entity_count);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn world(&self) -> &World {
        self.app.world()
    }

    /// Returns a mutable reference to the Bevy World for direct ECS mutations.
    ///
    /// Use this for spawning entities, inserting resources, or other
    /// world-modifying operations during tests.
    ///
    /// # Returns
    ///
    /// A mutable reference to the Bevy [`World`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// #[derive(Component)]
    /// struct TestMarker;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness.world_mut().spawn(TestMarker);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }

    /// Queries for all entities with a specific component.
    ///
    /// Returns a vector of all component instances of type `T` found in the world.
    /// This is useful for asserting on component state across all matching entities.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Component type to query for (must implement `Component`)
    ///
    /// # Returns
    ///
    /// A vector of references to all components of type `T`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// #[derive(Component)]
    /// struct Health(u32);
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness.world_mut().spawn(Health(100));
    /// harness.world_mut().spawn(Health(75));
    ///
    /// let all_health = harness.query::<Health>();
    /// assert_eq!(all_health.len(), 2);
    /// assert_eq!(all_health[0].0, 100);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn query<T: Component>(&mut self) -> Vec<&T> {
        let world = self.app.world_mut();
        let mut query = world.query::<&T>();
        query.iter(world).collect()
    }

    /// Queries for all entities with a specific component, filtered by a marker component.
    ///
    /// This is useful for finding components on entities that also have a specific marker.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Component type to query for
    /// * `F` - Filter component (entities must have this component)
    ///
    /// # Returns
    ///
    /// A vector of references to components of type `T` on entities that also have component `F`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// #[derive(Component)]
    /// struct Position(i32, i32);
    ///
    /// #[derive(Component)]
    /// struct Enemy;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness.world_mut().spawn((Position(10, 20), Enemy));
    /// harness.world_mut().spawn(Position(5, 15)); // No Enemy marker
    ///
    /// let enemy_positions = harness.query_filtered::<Position, Enemy>();
    /// assert_eq!(enemy_positions.len(), 1);
    /// assert_eq!(enemy_positions[0].0, 10);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn query_filtered<T: Component, F: Component>(&mut self) -> Vec<&T> {
        let world = self.app.world_mut();
        let mut query = world.query_filtered::<&T, With<F>>();
        query.iter(world).collect()
    }

    /// Gets a single component by entity ID.
    ///
    /// Returns `None` if the entity doesn't exist or doesn't have the component.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Component type to retrieve
    ///
    /// # Arguments
    ///
    /// * `entity` - Entity ID to query
    ///
    /// # Returns
    ///
    /// `Some(&T)` if the component exists, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// #[derive(Component)]
    /// struct Name(String);
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// let entity = harness.world_mut().spawn(Name("Player".to_string())).id();
    ///
    /// let name = harness.get_component::<Name>(entity);
    /// assert!(name.is_some());
    /// assert_eq!(name.unwrap().0, "Player");
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.app.world().entity(entity).get::<T>()
    }

    /// Asserts that at least one entity with the given component exists.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Component type to check for
    ///
    /// # Errors
    ///
    /// Returns a [`TermTestError::AssertionFailed`] if no entities with component `T` exist.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// #[derive(Component)]
    /// struct CommandPaletteMarker;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness.world_mut().spawn(CommandPaletteMarker);
    ///
    /// harness.assert_component_exists::<CommandPaletteMarker>()?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
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
    /// # Type Parameters
    ///
    /// * `T` - Component type to count
    ///
    /// # Arguments
    ///
    /// * `expected_count` - Expected number of entities with component `T`
    ///
    /// # Errors
    ///
    /// Returns a [`TermTestError::AssertionFailed`] if the count doesn't match.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// #[derive(Component)]
    /// struct Enemy;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness.world_mut().spawn(Enemy);
    /// harness.world_mut().spawn(Enemy);
    /// harness.world_mut().spawn(Enemy);
    ///
    /// harness.assert_component_count::<Enemy>(3)?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
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

    /// Updates Bevy schedules multiple times.
    ///
    /// This is an alias for `update_n` to match Bevy naming conventions.
    ///
    /// # Arguments
    ///
    /// * `count` - Number of update cycles to run
    ///
    /// # Errors
    ///
    /// Returns an error if any update fails.
    pub fn update_bevy(&mut self, count: usize) -> Result<()> {
        self.update_n(count)
    }

    // ========================================================================
    // Component Snapshot Methods (Issue #12)
    // ========================================================================

    /// Captures snapshots of all components of a given type.
    ///
    /// This method queries all entities with component type `T` and creates
    /// a serializable snapshot of each, including entity ID, component type,
    /// and component data. The snapshots can be used with `insta::assert_json_snapshot!`
    /// for regression testing of ECS state.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Component type to snapshot (must implement `Component + Serialize`)
    ///
    /// # Returns
    ///
    /// A vector of `ComponentSnapshot<T>` containing all components of type `T`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(all(feature = "bevy", feature = "snapshot-insta"))]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::BevyTuiTestHarness;
    /// use serde::Serialize;
    ///
    /// #[derive(Component, Serialize)]
    /// struct Health {
    ///     current: u32,
    ///     max: u32,
    /// }
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness.world_mut().spawn(Health { current: 100, max: 100 });
    /// harness.world_mut().spawn(Health { current: 75, max: 100 });
    ///
    /// let snapshots = harness.snapshot_components::<Health>();
    /// insta::assert_json_snapshot!("health_state", snapshots);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "snapshot-insta")]
    pub fn snapshot_components<T>(&mut self) -> Vec<ComponentSnapshot<T>>
    where
        T: Component + Serialize + Clone,
    {
        let world = self.app.world_mut();
        let mut query = world.query::<(Entity, &T)>();

        query
            .iter(world)
            .map(|(entity, component)| ComponentSnapshot::new(entity, component.clone()))
            .collect()
    }

    /// Captures snapshots of components filtered by a marker component.
    ///
    /// This is useful for capturing only specific subsets of components,
    /// such as all enemy positions or all UI elements.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Component type to snapshot (must implement `Component + Serialize`)
    /// * `F` - Filter component (entities must have this component)
    ///
    /// # Returns
    ///
    /// A vector of `ComponentSnapshot<T>` for components on entities that also have `F`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(all(feature = "bevy", feature = "snapshot-insta"))]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::BevyTuiTestHarness;
    /// use serde::Serialize;
    ///
    /// #[derive(Component, Serialize, Clone)]
    /// struct Position {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// #[derive(Component)]
    /// struct Enemy;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness
    ///     .world_mut()
    ///     .spawn((Position { x: 10.0, y: 20.0 }, Enemy));
    /// harness.world_mut().spawn(Position { x: 5.0, y: 15.0 }); // Not an enemy
    ///
    /// let snapshots = harness.snapshot_components_filtered::<Position, Enemy>();
    /// assert_eq!(snapshots.len(), 1);
    /// insta::assert_json_snapshot!("enemy_positions", snapshots);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "snapshot-insta")]
    pub fn snapshot_components_filtered<T, F>(&mut self) -> Vec<ComponentSnapshot<T>>
    where
        T: Component + Serialize + Clone,
        F: Component,
    {
        let world = self.app.world_mut();
        let mut query = world.query_filtered::<(Entity, &T), With<F>>();

        query
            .iter(world)
            .map(|(entity, component)| ComponentSnapshot::new(entity, component.clone()))
            .collect()
    }

    /// Asserts that component state matches a snapshot using insta.
    ///
    /// This is a convenience method that combines `snapshot_components` with
    /// insta's assertion. It provides better error messages specific to
    /// component snapshots.
    ///
    /// # Type Parameters
    ///
    /// * `T` - Component type to snapshot and assert
    ///
    /// # Arguments
    ///
    /// * `snapshot_name` - Name for the snapshot file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # #[cfg(all(feature = "bevy", feature = "snapshot-insta"))]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::BevyTuiTestHarness;
    /// use serde::Serialize;
    ///
    /// #[derive(Component, Serialize, Clone)]
    /// struct UiLayout {
    ///     width: u32,
    ///     height: u32,
    /// }
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness
    ///     .world_mut()
    ///     .spawn(UiLayout { width: 800, height: 600 });
    ///
    /// // This will create/compare against: snapshots/test__ui_layout_initial.snap
    /// harness.assert_component_snapshot::<UiLayout>("ui_layout_initial");
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "snapshot-insta")]
    pub fn assert_component_snapshot<T>(&mut self, snapshot_name: &str)
    where
        T: Component + Serialize + Clone,
    {
        let snapshots = self.snapshot_components::<T>();
        let json = serde_json::to_string_pretty(&snapshots)
            .expect("Failed to serialize component snapshot");
        insta::assert_snapshot!(snapshot_name, json);
    }

    /// Sends keyboard input (delegates to inner harness).
    ///
    /// # Arguments
    ///
    /// * `text` - Text to send
    ///
    /// # Errors
    ///
    /// Returns an error if sending fails.
    pub fn send_text(&mut self, text: &str) -> Result<()> {
        self.harness.send_text(text)
    }

    /// Checks if the harness is running in headless mode.
    ///
    /// Returns `true` if the `headless` feature flag is enabled, which means
    /// the Bevy app (when initialized in Phase 4) will use `MinimalPlugins`
    /// instead of `DefaultPlugins`.
    ///
    /// # Returns
    ///
    /// `true` if headless mode is enabled, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = BevyTuiTestHarness::new()?;
    /// if harness.is_headless() {
    ///     println!("Running in headless mode - suitable for CI");
    /// }
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn is_headless(&self) -> bool {
        self.is_headless
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
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?.with_shared_state("/tmp/tui_state.mmap")?;
    ///
    /// // Now you can access shared state in tests
    /// harness.update_n(10)?;
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
    ///     BevyTuiTestHarness,
    /// };
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Debug, Clone, Serialize, Deserialize)]
    /// struct GameState {
    ///     score: u32,
    /// }
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = BevyTuiTestHarness::new()?.with_shared_state("/tmp/game.mmap")?;
    ///
    /// if let Some(path) = harness.shared_state_path() {
    ///     let state = MemoryMappedState::<GameState>::open(path)?;
    ///     let game = state.read()?;
    ///     assert_eq!(game.score, 100);
    /// }
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "shared-state")]
    pub fn shared_state_path(&self) -> Option<&str> {
        self.shared_state_path.as_deref()
    }

    /// Returns the current screen state.
    ///
    /// Provides access to the terminal screen state for inspecting rendered
    /// output from Bevy systems.
    ///
    /// # Returns
    ///
    /// A reference to the current [`ScreenState`].
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = BevyTuiTestHarness::new()?;
    /// let state = harness.state();
    /// println!("Cursor at: {:?}", state.cursor_position());
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn state(&self) -> &ScreenState {
        self.harness.state()
    }

    /// Waits for a screen condition (delegates to inner harness).
    ///
    /// This method repeatedly checks the screen state until the condition is met
    /// or the timeout expires. Useful for waiting for Bevy systems to render output.
    ///
    /// # Arguments
    ///
    /// * `condition` - Condition function that receives the current screen state
    ///
    /// # Errors
    ///
    /// Returns a [`TermTestError::Timeout`] if the condition is not met within
    /// the configured timeout.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut harness = BevyTuiTestHarness::new()?;
    /// harness.update()?;
    ///
    /// // Wait for specific text to appear
    /// harness.wait_for(|state| state.contains("Ready"))?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn wait_for<F>(&mut self, condition: F) -> Result<()>
    where
        F: Fn(&ScreenState) -> bool,
    {
        self.harness.wait_for(condition)
    }

    /// Checks if Sixel graphics are present in the current screen state.
    ///
    /// Returns `true` if any Sixel graphics regions have been detected in the
    /// screen state, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = BevyTuiTestHarness::new()?;
    /// // ... render some graphics ...
    ///
    /// if harness.has_sixel_graphics() {
    ///     println!("Sixel graphics detected!");
    /// }
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "sixel")]
    pub fn has_sixel_graphics(&self) -> bool {
        !self.state().sixel_regions().is_empty()
    }

    /// Captures the current Sixel state.
    ///
    /// This creates a [`SixelCapture`] containing all Sixel graphics regions
    /// detected in the current screen state. The capture can be used for
    /// advanced querying and validation.
    ///
    /// # Returns
    ///
    /// A [`SixelCapture`] containing all detected Sixel sequences with their
    /// position and dimension information.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::BevyTuiTestHarness;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let harness = BevyTuiTestHarness::new()?;
    /// // ... render graphics ...
    ///
    /// let capture = harness.capture_sixel_state()?;
    /// println!("Captured {} Sixel sequences", capture.sequences().len());
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    #[cfg(feature = "sixel")]
    pub fn capture_sixel_state(&self) -> Result<SixelCapture> {
        Ok(SixelCapture::from_screen_state(self.state()))
    }

    /// Asserts that all Sixel graphics are within the specified area.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Errors
    ///
    /// Returns an error if any Sixel is outside the area.
    #[cfg(feature = "sixel")]
    pub fn assert_sixel_within(&self, area: (u16, u16, u16, u16)) -> Result<()> {
        let capture = self.capture_sixel_state()?;
        capture.assert_all_within(area)
    }

    /// Asserts that no Sixel graphics are outside the specified area.
    ///
    /// This is the inverse of `assert_sixel_within`.
    ///
    /// # Arguments
    ///
    /// * `area` - Area as (row, col, width, height)
    ///
    /// # Errors
    ///
    /// Returns an error if any Sixel is outside the area.
    #[cfg(feature = "sixel")]
    pub fn assert_no_sixel_outside(&self, area: (u16, u16, u16, u16)) -> Result<()> {
        self.assert_sixel_within(area)
    }
}

// ============================================================================
// Benchmarking Support (Issue #13)
// ============================================================================

impl bench::BenchmarkableHarness for BevyTuiTestHarness {
    fn tick_once(&mut self) -> Result<()> {
        self.update()
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;

    #[derive(Component)]
    struct TestHealth(u32);

    #[derive(Component)]
    struct TestPosition(i32, i32);

    #[derive(Component)]
    struct EnemyMarker;

    #[derive(Component)]
    struct PlayerMarker;

    #[test]
    fn test_create_bevy_harness() {
        let harness = BevyTuiTestHarness::new();
        assert!(harness.is_ok());
    }

    #[test]
    fn test_update() {
        let mut harness = BevyTuiTestHarness::new().unwrap();
        assert!(harness.update().is_ok());
    }

    #[test]
    fn test_update_n() {
        let mut harness = BevyTuiTestHarness::new().unwrap();
        assert!(harness.update_n(5).is_ok());
    }

    // ========================================================================
    // Bevy ECS Integration Tests (Issue #9)
    // ========================================================================

    #[test]
    fn test_world_access() {
        let harness = BevyTuiTestHarness::new().unwrap();
        let world = harness.world();
        // Just verify we can access the world
        let _ = world.entities().len();
    }

    #[test]
    fn test_world_mut_spawn() {
        let mut harness = BevyTuiTestHarness::new().unwrap();
        let entity = harness.world_mut().spawn(TestHealth(100)).id();

        // Verify entity was created
        let health = harness.get_component::<TestHealth>(entity);
        assert!(health.is_some());
        assert_eq!(health.unwrap().0, 100);
    }

    #[test]
    fn test_query_single_component() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Spawn entities with health
        harness.world_mut().spawn(TestHealth(100));
        harness.world_mut().spawn(TestHealth(75));
        harness.world_mut().spawn(TestHealth(50));

        // Query all health components
        let all_health = harness.query::<TestHealth>();
        assert_eq!(all_health.len(), 3);

        // Verify values
        let mut values: Vec<u32> = all_health.iter().map(|h| h.0).collect();
        values.sort();
        assert_eq!(values, vec![50, 75, 100]);
    }

    #[test]
    fn test_query_filtered() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Spawn enemies with positions
        harness
            .world_mut()
            .spawn((TestPosition(10, 20), EnemyMarker));
        harness
            .world_mut()
            .spawn((TestPosition(30, 40), EnemyMarker));

        // Spawn player with position (no enemy marker)
        harness
            .world_mut()
            .spawn((TestPosition(5, 15), PlayerMarker));

        // Query only enemy positions
        let enemy_positions = harness.query_filtered::<TestPosition, EnemyMarker>();
        assert_eq!(enemy_positions.len(), 2);

        // Verify no player positions in results
        let player_positions = harness.query_filtered::<TestPosition, PlayerMarker>();
        assert_eq!(player_positions.len(), 1);
        assert_eq!(player_positions[0].0, 5);
    }

    #[test]
    fn test_get_component() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        let entity1 = harness.world_mut().spawn(TestHealth(100)).id();
        let entity2 = harness.world_mut().spawn(TestPosition(10, 20)).id();

        // Get existing component
        let health = harness.get_component::<TestHealth>(entity1);
        assert!(health.is_some());
        assert_eq!(health.unwrap().0, 100);

        // Get non-existing component
        let position = harness.get_component::<TestPosition>(entity1);
        assert!(position.is_none());

        // Get from different entity
        let position = harness.get_component::<TestPosition>(entity2);
        assert!(position.is_some());
        assert_eq!(position.unwrap().0, 10);
    }

    #[test]
    fn test_assert_component_exists() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Initially no components
        assert!(harness.assert_component_exists::<TestHealth>().is_err());

        // Spawn one
        harness.world_mut().spawn(TestHealth(100));

        // Now should pass
        assert!(harness.assert_component_exists::<TestHealth>().is_ok());
    }

    #[test]
    fn test_assert_component_count() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Initially zero
        assert!(harness.assert_component_count::<TestHealth>(0).is_ok());
        assert!(harness.assert_component_count::<TestHealth>(1).is_err());

        // Spawn 3 entities
        harness.world_mut().spawn(TestHealth(100));
        harness.world_mut().spawn(TestHealth(75));
        harness.world_mut().spawn(TestHealth(50));

        // Assert count
        assert!(harness.assert_component_count::<TestHealth>(3).is_ok());
        assert!(harness.assert_component_count::<TestHealth>(2).is_err());
        assert!(harness.assert_component_count::<TestHealth>(4).is_err());
    }

    #[test]
    fn test_update_bevy_alias() {
        let mut harness = BevyTuiTestHarness::new().unwrap();
        assert!(harness.update_bevy(5).is_ok());
    }

    #[test]
    fn test_bevy_with_systems() {
        #[derive(Component)]
        struct Counter(u32);

        fn increment_system(mut query: Query<'_, '_, &mut Counter>) {
            for mut counter in query.iter_mut() {
                counter.0 += 1;
            }
        }

        // Create custom app with system
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, increment_system);

        let mut harness = BevyTuiTestHarness::with_app(app).unwrap();

        // Spawn counter
        harness.world_mut().spawn(Counter(0));

        // Initial value
        let counters = harness.query::<Counter>();
        assert_eq!(counters[0].0, 0);

        // Run update (should execute system)
        harness.update().unwrap();

        // Value should be incremented
        let counters = harness.query::<Counter>();
        assert_eq!(counters[0].0, 1);

        // Run 5 more times
        harness.update_bevy(5).unwrap();
        let counters = harness.query::<Counter>();
        assert_eq!(counters[0].0, 6);
    }

    #[test]
    fn test_hybrid_ecs_screen_assertions() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Spawn a game state component
        #[derive(Component)]
        struct GameState {
            level: u32,
            score: u32,
        }

        harness
            .world_mut()
            .spawn(GameState { level: 1, score: 100 });

        // Verify ECS state
        assert!(harness.assert_component_exists::<GameState>().is_ok());

        let game_states = harness.query::<GameState>();
        assert_eq!(game_states.len(), 1);
        assert_eq!(game_states[0].level, 1);
        assert_eq!(game_states[0].score, 100);

        // Verify screen state (empty initially)
        let screen = harness.state();
        assert!(!screen.contains("Level"));
    }

    // ========================================================================
    // Headless Mode Tests
    // ========================================================================

    #[test]
    fn test_headless_flag_detection() {
        let harness = BevyTuiTestHarness::new().unwrap();

        #[cfg(feature = "headless")]
        {
            assert!(harness.is_headless(), "Should detect headless mode when feature is enabled");
        }

        #[cfg(not(feature = "headless"))]
        {
            assert!(!harness.is_headless(), "Should not be in headless mode without feature flag");
        }
    }

    #[test]
    #[cfg(feature = "headless")]
    fn test_headless_initialization_without_display() {
        // This test verifies that the harness can be created without DISPLAY
        // environment variable, which is crucial for CI/CD environments

        // Save current DISPLAY value
        let original_display = std::env::var("DISPLAY").ok();

        // Remove DISPLAY to simulate headless environment
        std::env::remove_var("DISPLAY");

        // Should succeed in headless mode
        let result = BevyTuiTestHarness::new();
        assert!(result.is_ok(), "Headless harness should work without DISPLAY");

        let harness = result.unwrap();
        assert!(harness.is_headless());

        // Restore DISPLAY if it was set
        if let Some(display) = original_display {
            std::env::set_var("DISPLAY", display);
        }
    }

    #[test]
    #[cfg(feature = "headless")]
    fn test_headless_operations() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        // All operations that don't require a spawned process should work
        assert!(harness.update().is_ok());
        assert!(harness.update_n(3).is_ok());

        // Note: render_frame() and send_text() require a spawned process
        // They would be tested in integration tests with actual TUI apps
        // Here we just verify the harness can be created in headless mode
    }

    #[test]
    fn test_headless_with_bevy_ratatui() {
        #[cfg(feature = "bevy-ratatui")]
        {
            let harness = BevyTuiTestHarness::with_bevy_ratatui();
            assert!(harness.is_ok());

            #[cfg(feature = "headless")]
            {
                assert!(harness.unwrap().is_headless());
            }
        }
    }

    // ========================================================================
    // Sixel Detection Tests
    // ========================================================================

    #[test]
    #[cfg(feature = "sixel")]
    fn test_has_sixel_graphics_initially_false() {
        let harness = BevyTuiTestHarness::new().unwrap();
        assert!(!harness.has_sixel_graphics());
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_has_sixel_graphics_after_feed() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Feed a Sixel sequence
        let state = harness.harness.state_mut();
        state.feed(b"\x1b[10;10H");
        state.feed(b"\x1bPq\"1;1;100;50#0~\x1b\\");

        assert!(harness.has_sixel_graphics());
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_capture_sixel_state_empty() {
        let harness = BevyTuiTestHarness::new().unwrap();
        let capture = harness.capture_sixel_state().unwrap();
        assert!(capture.is_empty());
        assert_eq!(capture.sequences().len(), 0);
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_capture_sixel_state_with_graphics() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Feed a Sixel sequence
        let state = harness.harness.state_mut();
        state.feed(b"\x1b[5;10H");
        state.feed(b"\x1bPq\"1;1;200;150#0~\x1b\\");

        let capture = harness.capture_sixel_state().unwrap();
        assert!(!capture.is_empty());
        assert_eq!(capture.sequences().len(), 1);

        let seq = &capture.sequences()[0];
        assert_eq!(seq.position, (4, 9)); // 0-based conversion from 1-based CSI
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_capture_multiple_sixel_sequences() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        let state = harness.harness.state_mut();
        // First Sixel
        state.feed(b"\x1b[5;5H\x1bPq\"1;1;80;60#0~\x1b\\");
        // Second Sixel
        state.feed(b"\x1b[15;20H\x1bPq\"1;1;100;80#0~\x1b\\");

        let capture = harness.capture_sixel_state().unwrap();
        assert_eq!(capture.sequences().len(), 2);
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_assert_sixel_within_valid() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        let state = harness.harness.state_mut();
        state.feed(b"\x1b[10;10H\x1bPq\"1;1;100;50#0~\x1b\\");

        // Large area should contain the Sixel
        let area = (0, 0, 80, 24);
        assert!(harness.assert_sixel_within(area).is_ok());
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_assert_sixel_within_invalid() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        let state = harness.harness.state_mut();
        // Place Sixel at (10, 10) with 100x50 pixels = 13x9 cells (rounded up)
        state.feed(b"\x1b[10;10H\x1bPq\"1;1;100;50#0~\x1b\\");

        // Small area that doesn't contain the Sixel
        let area = (0, 0, 5, 5);
        assert!(harness.assert_sixel_within(area).is_err());
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_sixel_dimensions_conversion() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        let state = harness.harness.state_mut();
        // 800 pixels wide, 600 pixels tall
        // Should convert to 100 cells wide (800/8), 100 cells tall (600/6)
        state.feed(b"\x1b[1;1H\x1bPq\"1;1;800;600#0~\x1b\\");

        let capture = harness.capture_sixel_state().unwrap();
        let seq = &capture.sequences()[0];
        let (_, _, width, height) = seq.bounds;

        assert_eq!(width, 100, "Width should be 100 cells (800px / 8px per cell)");
        assert_eq!(height, 100, "Height should be 100 cells (600px / 6px per cell)");
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_sixel_abbreviated_raster_format() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        let state = harness.harness.state_mut();
        // Abbreviated format: just width;height (no aspect ratio)
        state.feed(b"\x1b[5;10H\x1bPq\"400;300#0~\x1b\\");

        let capture = harness.capture_sixel_state().unwrap();
        assert_eq!(capture.sequences().len(), 1);

        let seq = &capture.sequences()[0];
        let (_, _, width, height) = seq.bounds;
        assert_eq!(width, 50); // 400px / 8px per cell
        assert_eq!(height, 50); // 300px / 6px per cell
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_sixel_without_raster_attributes() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        let state = harness.harness.state_mut();
        // Legacy Sixel without raster attributes
        state.feed(b"\x1b[10;10H\x1bPq#0;2;100;100;100#0~~\x1b\\");

        let capture = harness.capture_sixel_state().unwrap();
        assert_eq!(capture.sequences().len(), 1);

        let seq = &capture.sequences()[0];
        let (_, _, width, height) = seq.bounds;
        assert_eq!(width, 0, "Width should be 0 without raster attributes");
        assert_eq!(height, 0, "Height should be 0 without raster attributes");
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_sixel_preview_area_validation() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Simulate dgx-pixels preview area at (5, 40) with size 35x15
        let preview_area = (5, 40, 35, 15);

        let state = harness.harness.state_mut();
        // Place Sixel within preview area
        // Position: (8, 45) [1-based] = (7, 44) [0-based]
        // Size: 120x60 pixels = 15x10 cells
        // End position: (7+10, 44+15) = (17, 59)
        // Preview area: (5, 40) to (20, 75), so Sixel fits
        state.feed(b"\x1b[8;45H\x1bPq\"1;1;120;60#0~\x1b\\");

        // Should pass validation
        assert!(harness.assert_sixel_within(preview_area).is_ok());

        // Capture should detect it in the area
        let capture = harness.capture_sixel_state().unwrap();
        let in_area = capture.sequences_in_area(preview_area);
        assert_eq!(in_area.len(), 1);
    }

    // ========================================================================
    // Component Snapshot Tests (Issue #12)
    // ========================================================================

    #[test]
    #[cfg(feature = "snapshot-insta")]
    fn test_component_snapshot_creation() {
        use serde::Serialize;

        #[derive(Component, Serialize, Clone, Debug, PartialEq)]
        struct TestComponent {
            value: u32,
        }

        let mut harness = BevyTuiTestHarness::new().unwrap();
        let entity = harness.world_mut().spawn(TestComponent { value: 42 }).id();

        let snapshots = harness.snapshot_components::<TestComponent>();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].entity_id(), entity.index());
        assert_eq!(snapshots[0].data().value, 42);
        assert!(snapshots[0].component_type().contains("TestComponent"));
    }

    #[test]
    #[cfg(feature = "snapshot-insta")]
    fn test_snapshot_multiple_components() {
        use serde::Serialize;

        #[derive(Component, Serialize, Clone, Debug, PartialEq)]
        struct Position {
            x: f32,
            y: f32,
        }

        let mut harness = BevyTuiTestHarness::new().unwrap();
        harness.world_mut().spawn(Position { x: 10.0, y: 20.0 });
        harness.world_mut().spawn(Position { x: 30.0, y: 40.0 });
        harness.world_mut().spawn(Position { x: 50.0, y: 60.0 });

        let snapshots = harness.snapshot_components::<Position>();
        assert_eq!(snapshots.len(), 3);

        // Verify all positions are captured
        let mut x_values: Vec<f32> = snapshots.iter().map(|s| s.data().x).collect();
        x_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(x_values, vec![10.0, 30.0, 50.0]);
    }

    #[test]
    #[cfg(feature = "snapshot-insta")]
    fn test_snapshot_components_filtered() {
        use serde::Serialize;

        #[derive(Component, Serialize, Clone, Debug)]
        struct Health {
            current: u32,
            max: u32,
        }

        #[derive(Component)]
        struct Enemy;

        #[derive(Component)]
        struct Player;

        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Spawn enemies with health
        harness
            .world_mut()
            .spawn((Health { current: 50, max: 100 }, Enemy));
        harness
            .world_mut()
            .spawn((Health { current: 75, max: 100 }, Enemy));

        // Spawn player with health
        harness
            .world_mut()
            .spawn((Health { current: 100, max: 100 }, Player));

        // Snapshot only enemy health
        let enemy_snapshots = harness.snapshot_components_filtered::<Health, Enemy>();
        assert_eq!(enemy_snapshots.len(), 2);

        // Snapshot only player health
        let player_snapshots = harness.snapshot_components_filtered::<Health, Player>();
        assert_eq!(player_snapshots.len(), 1);
        assert_eq!(player_snapshots[0].data().current, 100);
    }

    #[test]
    #[cfg(feature = "snapshot-insta")]
    fn test_snapshot_complex_component() {
        use serde::Serialize;

        #[derive(Component, Serialize, Clone, Debug, PartialEq)]
        struct UiLayout {
            position: (f32, f32),
            size: (f32, f32),
            visible: bool,
            z_index: i32,
            style: String,
        }

        let mut harness = BevyTuiTestHarness::new().unwrap();

        let layout = UiLayout {
            position: (100.0, 200.0),
            size: (800.0, 600.0),
            visible: true,
            z_index: 5,
            style: "bordered".to_string(),
        };

        harness.world_mut().spawn(layout.clone());

        let snapshots = harness.snapshot_components::<UiLayout>();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].data(), &layout);
    }

    #[test]
    #[cfg(feature = "snapshot-insta")]
    fn test_snapshot_after_system_execution() {
        use serde::Serialize;

        #[derive(Component, Serialize, Clone, Debug)]
        struct Counter(u32);

        fn increment_system(mut query: Query<'_, '_, &mut Counter>) {
            for mut counter in query.iter_mut() {
                counter.0 += 1;
            }
        }

        // Create app with system
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, increment_system);

        let mut harness = BevyTuiTestHarness::with_app(app).unwrap();

        // Spawn counter
        harness.world_mut().spawn(Counter(0));

        // Initial snapshot
        let initial = harness.snapshot_components::<Counter>();
        assert_eq!(initial[0].data().0, 0);

        // Run update
        harness.update().unwrap();

        // After update snapshot
        let after_update = harness.snapshot_components::<Counter>();
        assert_eq!(after_update[0].data().0, 1);

        // Run 5 more updates
        harness.update_bevy(5).unwrap();

        // Final snapshot
        let final_state = harness.snapshot_components::<Counter>();
        assert_eq!(final_state[0].data().0, 6);
    }

    #[test]
    #[cfg(feature = "snapshot-insta")]
    fn test_snapshot_empty() {
        use serde::Serialize;

        #[derive(Component, Serialize, Clone)]
        struct NonExistent;

        let mut harness = BevyTuiTestHarness::new().unwrap();

        let snapshots = harness.snapshot_components::<NonExistent>();
        assert_eq!(snapshots.len(), 0);
    }

    #[test]
    #[cfg(feature = "snapshot-insta")]
    fn test_component_snapshot_json_serialization() {
        use serde::Serialize;
        use serde_json;

        #[derive(Component, Serialize, Clone, Debug, PartialEq)]
        struct TestData {
            name: String,
            value: i32,
        }

        let mut harness = BevyTuiTestHarness::new().unwrap();
        harness
            .world_mut()
            .spawn(TestData { name: "test".to_string(), value: 42 });

        let snapshots = harness.snapshot_components::<TestData>();

        // Verify JSON serialization works
        let json = serde_json::to_string(&snapshots).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"value\":42"));
        assert!(json.contains("TestData"));
    }

    #[test]
    #[cfg(feature = "snapshot-insta")]
    fn test_snapshot_workflow_integration() {
        use serde::Serialize;

        #[derive(Component, Serialize, Clone, Debug)]
        struct GameState {
            level: u32,
            score: u32,
            lives: u32,
        }

        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Initial game state
        harness
            .world_mut()
            .spawn(GameState { level: 1, score: 0, lives: 3 });

        let initial_snapshot = harness.snapshot_components::<GameState>();
        assert_eq!(initial_snapshot.len(), 1);

        // In a real test, you would use:
        // insta::assert_json_snapshot!("game_state_initial", initial_snapshot);

        // Simulate game progression
        let mut state_query = harness.world_mut().query::<&mut GameState>();
        for mut state in state_query.iter_mut(harness.world_mut()) {
            state.level = 2;
            state.score = 1000;
            state.lives = 2;
        }

        let after_level_snapshot = harness.snapshot_components::<GameState>();
        assert_eq!(after_level_snapshot[0].data().level, 2);

        // In a real test:
        // insta::assert_json_snapshot!("game_state_after_level", after_level_snapshot);
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_sixel_state_changes() {
        use crate::screen::ScreenState;

        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Initial state with Sixel
        let state = harness.harness.state_mut();
        state.feed(b"\x1b[10;10H\x1bPq\"1;1;100;100#0~\x1b\\");

        let capture1 = harness.capture_sixel_state().unwrap();
        assert_eq!(capture1.sequences().len(), 1);

        // Simulate screen clear (replace state)
        *harness.harness.state_mut() = ScreenState::new(80, 24);

        let capture2 = harness.capture_sixel_state().unwrap();
        assert!(capture2.is_empty());
        assert!(!harness.has_sixel_graphics());

        // Verify state differs
        assert!(capture1.differs_from(&capture2));
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_assert_no_sixel_outside() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        let state = harness.harness.state_mut();
        state.feed(b"\x1b[10;10H\x1bPq\"1;1;100;50#0~\x1b\\");

        // Large area contains the Sixel
        let area = (0, 0, 80, 24);
        assert!(harness.assert_no_sixel_outside(area).is_ok());

        // Small area doesn't contain the Sixel
        let small_area = (0, 0, 5, 5);
        assert!(harness.assert_no_sixel_outside(small_area).is_err());
    }

    #[test]
    #[cfg(feature = "sixel")]
    fn test_complex_sixel_sequence() {
        let mut harness = BevyTuiTestHarness::new().unwrap();

        let state = harness.harness.state_mut();
        // Complex Sixel with multiple colors
        state.feed(b"\x1b[15;25H");
        state.feed(b"\x1bPq\"1;1;640;480");
        state.feed(b"#0;2;0;0;0"); // Black
        state.feed(b"#1;2;100;0;0"); // Red
        state.feed(b"#2;2;0;100;0"); // Green
        state.feed(b"#0~~~#1@@@#2~~~"); // Data
        state.feed(b"\x1b\\");

        let capture = harness.capture_sixel_state().unwrap();
        assert_eq!(capture.sequences().len(), 1);

        let seq = &capture.sequences()[0];
        assert_eq!(seq.position, (14, 24)); // 0-based

        let (_, _, width, height) = seq.bounds;
        assert_eq!(width, 80); // 640px / 8px per cell
        assert_eq!(height, 80); // 480px / 6px per cell
    }

    // ========================================================================
    // Benchmarking Tests (Issue #13)
    // ========================================================================

    #[test]
    fn test_benchmark_rendering() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Benchmark 100 frames
        let results = harness.benchmark_rendering(100).unwrap();

        assert_eq!(results.iterations, 100);
        assert!(results.total_duration_ms > 0.0);
        assert!(results.avg_frame_time_ms > 0.0);
        assert!(results.fps_avg > 0.0);
        assert!(results.min_frame_time_ms <= results.avg_frame_time_ms);
        assert!(results.max_frame_time_ms >= results.avg_frame_time_ms);
    }

    #[test]
    fn test_profile_update_cycle() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut harness = BevyTuiTestHarness::new().unwrap();

        let profile = harness.profile_update_cycle().unwrap();

        assert!(profile.duration_ms > 0.0);
        assert!(profile.fps_equivalent > 0.0);
        // Basic sanity check: FPS equivalent should be 1000 / duration_ms
        assert!((profile.fps_equivalent - (1000.0 / profile.duration_ms)).abs() < 0.1);
    }

    #[test]
    fn test_assert_fps_success() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Assert a very low FPS requirement that should always pass
        let results = harness.assert_fps(1.0, 50);
        assert!(results.is_ok());

        let results = results.unwrap();
        assert!(results.meets_fps_requirement(1.0));
    }

    #[test]
    fn test_benchmark_with_systems() {
        use crate::bevy::bench::BenchmarkableHarness;

        #[derive(Component)]
        struct BenchCounter(u32);

        fn increment_system(mut query: Query<'_, '_, &mut BenchCounter>) {
            for mut counter in query.iter_mut() {
                counter.0 += 1;
            }
        }

        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, increment_system);

        let mut harness = BevyTuiTestHarness::with_app(app).unwrap();
        harness.world_mut().spawn(BenchCounter(0));

        // Benchmark 50 frames
        let results = harness.benchmark_rendering(50).unwrap();

        assert_eq!(results.iterations, 50);

        // Verify system ran 50 times
        let counters = harness.query::<BenchCounter>();
        assert_eq!(counters[0].0, 50);
    }

    #[test]
    fn test_benchmark_results_percentiles() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut harness = BevyTuiTestHarness::new().unwrap();

        // Run enough iterations to get meaningful percentiles
        let results = harness.benchmark_rendering(100).unwrap();

        // Percentiles should be ordered: min <= p50 <= p95 <= p99 <= max
        assert!(results.min_frame_time_ms <= results.p50_ms);
        assert!(results.p50_ms <= results.p95_ms);
        assert!(results.p95_ms <= results.p99_ms);
        assert!(results.p99_ms <= results.max_frame_time_ms);
    }

    #[test]
    fn test_benchmark_results_summary() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut harness = BevyTuiTestHarness::new().unwrap();
        let results = harness.benchmark_rendering(50).unwrap();

        let summary = results.summary();

        // Verify summary contains key information
        assert!(summary.contains("50 iterations"));
        assert!(summary.contains("Average FPS"));
        assert!(summary.contains("p50"));
        assert!(summary.contains("p95"));
        assert!(summary.contains("p99"));
    }

    #[test]
    fn test_profile_results_summary() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut harness = BevyTuiTestHarness::new().unwrap();
        let profile = harness.profile_update_cycle().unwrap();

        let summary = profile.summary();

        assert!(summary.contains("Frame Profile"));
        assert!(summary.contains("Duration"));
        assert!(summary.contains("FPS Equivalent"));
    }
}
