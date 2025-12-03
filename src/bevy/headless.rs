//! Headless Bevy Runner for in-process CI-friendly testing.
//!
//! This module provides [`HeadlessBevyRunner`], an alternative to [`BevyTuiTestHarness`]
//! that runs Bevy schedules entirely in-process without PTY overhead.

use bevy::{
    app::App,
    ecs::{component::Component, world::World},
    prelude::{Entity, Update, With},
    MinimalPlugins,
};

#[cfg(feature = "sixel")]
use crate::sixel::SixelCapture;
use crate::{
    error::{Result, TermTestError},
    screen::ScreenState,
};

/// In-process headless Bevy runner for CI-friendly testing.
///
/// Unlike [`crate::BevyTuiTestHarness`] which spawns applications in a PTY,
/// `HeadlessBevyRunner` runs Bevy schedules entirely in-process using
/// `MinimalPlugins` and `ScheduleRunnerPlugin`. This provides:
///
/// - **No PTY overhead**: Fast unit-style tests
/// - **Deterministic execution**: Fixed timestep, frame-by-frame control
/// - **CI/CD friendly**: No X11/Wayland/display server required
/// - **ECS testing**: Full access to Bevy World, components, and resources
/// - **Screen capture**: Optional integration with ScreenState for terminal output
///
/// # Use Cases
///
/// - **Component logic testing**: Test Bevy systems without full PTY
/// - **CI/CD pipelines**: Run in GitHub Actions without display server
/// - **Snapshot testing**: Capture ECS and screen state for regression tests
/// - **Deterministic testing**: Precise frame-by-frame execution control
///
/// # Comparison: HeadlessBevyRunner vs BevyTuiTestHarness
///
/// | Feature | HeadlessBevyRunner | BevyTuiTestHarness |
/// |---------|-------------------|-------------------|
/// | Execution | In-process | PTY subprocess |
/// | Speed | Fast (no PTY) | Slower (PTY overhead) |
/// | Display required | No | No (with `headless` feature) |
/// | Real terminal I/O | No | Yes |
/// | Use case | Unit/component tests | E2E integration tests |
/// | Deterministic | Yes (fixed timestep) | Harder (async PTY) |
///
/// # Example: Basic Component Testing
///
/// ```rust,no_run
/// # #[cfg(feature = "bevy")]
/// # {
/// use bevy::prelude::*;
/// use ratatui_testlib::HeadlessBevyRunner;
///
/// #[derive(Component)]
/// struct Counter(u32);
///
/// fn increment(mut query: Query<&mut Counter>) {
///     for mut counter in query.iter_mut() {
///         counter.0 += 1;
///     }
/// }
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let mut runner = HeadlessBevyRunner::new()?;
/// runner.app_mut().add_systems(Update, increment);
/// runner.world_mut().spawn(Counter(0));
///
/// // Run 10 frames
/// runner.tick_n(10)?;
///
/// // Verify component state
/// let counters = runner.query::<Counter>();
/// assert_eq!(counters[0].0, 10);
/// # Ok(())
/// # }
/// # }
/// ```
///
/// # Example: With bevy_ratatui Integration
///
/// When using `bevy_ratatui`, you can capture terminal output directly:
///
/// ```rust,no_run
/// # #[cfg(all(feature = "bevy", feature = "bevy-ratatui"))]
/// # {
/// use ratatui_testlib::HeadlessBevyRunner;
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let mut runner = HeadlessBevyRunner::with_bevy_ratatui()?;
///
/// // Add your rendering systems...
/// // runner.app_mut().add_systems(Update, my_render_system);
///
/// // Tick once to run systems
/// runner.tick()?;
///
/// // Capture screen state (requires custom adapter)
/// // let screen = runner.capture_screen()?;
/// // assert!(screen.contains("Hello"));
/// # Ok(())
/// # }
/// # }
/// ```
///
/// # Snapshot Testing with insta
///
/// ```rust,no_run
/// # #[cfg(all(feature = "bevy", feature = "snapshot-insta"))]
/// # {
/// use ratatui_testlib::HeadlessBevyRunner;
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let mut runner = HeadlessBevyRunner::new()?;
/// // ... add systems and spawn entities ...
///
/// runner.tick_n(5)?;
///
/// // Snapshot screen state
/// let snapshot = runner.snapshot();
/// insta::assert_snapshot!(snapshot);
/// # Ok(())
/// # }
/// # }
/// ```
pub struct HeadlessBevyRunner {
    app: App,
    screen: ScreenState,
    width: u16,
    height: u16,
}

impl HeadlessBevyRunner {
    /// Creates a new headless Bevy runner with default terminal dimensions (80x24).
    ///
    /// Initializes a Bevy App with `MinimalPlugins` and `ScheduleRunnerPlugin`
    /// configured for manual tick control (no automatic loop).
    ///
    /// # Errors
    ///
    /// Returns an error if Bevy initialization fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::HeadlessBevyRunner;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let runner = HeadlessBevyRunner::new()?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn new() -> Result<Self> {
        Self::with_dimensions(80, 24)
    }

    /// Creates a headless runner with custom terminal dimensions.
    ///
    /// # Arguments
    ///
    /// * `width` - Terminal width in columns
    /// * `height` - Terminal height in rows
    ///
    /// # Errors
    ///
    /// Returns an error if Bevy initialization fails.
    pub fn with_dimensions(width: u16, height: u16) -> Result<Self> {
        let mut app = App::new();

        // Use MinimalPlugins for headless operation
        // MinimalPlugins already includes ScheduleRunnerPlugin with run_once() behavior
        app.add_plugins(MinimalPlugins);

        let screen = ScreenState::new(width, height);

        Ok(Self { app, screen, width, height })
    }

    /// Creates a headless runner with bevy_ratatui plugin pre-configured.
    ///
    /// # Errors
    ///
    /// Returns an error if initialization fails.
    #[cfg(feature = "bevy-ratatui")]
    pub fn with_bevy_ratatui() -> Result<Self> {
        let mut runner = Self::new()?;
        runner
            .app
            .add_plugins(bevy_ratatui::RatatuiPlugins::default());
        Ok(runner)
    }

    /// Creates a headless runner with a custom pre-configured Bevy App.
    ///
    /// Use this when you need full control over plugin configuration.
    ///
    /// # Arguments
    ///
    /// * `app` - Pre-configured Bevy App (should have MinimalPlugins + ScheduleRunnerPlugin)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::{app::ScheduleRunnerPlugin, prelude::*};
    /// use ratatui_testlib::HeadlessBevyRunner;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut app = App::new();
    /// app.add_plugins(MinimalPlugins);
    /// // MinimalPlugins includes ScheduleRunnerPlugin with run_once behavior
    /// // Add your custom plugins/systems here
    ///
    /// let runner = HeadlessBevyRunner::with_app(app);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn with_app(app: App) -> Self {
        let screen = ScreenState::new(80, 24);
        Self { app, screen, width: 80, height: 24 }
    }

    /// Runs one Bevy frame update (ticks all schedules once).
    ///
    /// This executes all Bevy systems registered in the Update schedule.
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
    /// use ratatui_testlib::HeadlessBevyRunner;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut runner = HeadlessBevyRunner::new()?;
    /// runner.tick()?;
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
    /// use ratatui_testlib::HeadlessBevyRunner;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let runner = HeadlessBevyRunner::new()?;
    /// let entity_count = runner.world().entities().len();
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
    pub fn world_mut(&mut self) -> &mut World {
        self.app.world_mut()
    }

    /// Returns a mutable reference to the Bevy App.
    ///
    /// Use this to add systems, plugins, or configure the app.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use bevy::prelude::*;
    /// use ratatui_testlib::HeadlessBevyRunner;
    ///
    /// fn my_system() { // ...
    /// }
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut runner = HeadlessBevyRunner::new()?;
    /// runner.app_mut().add_systems(Update, my_system);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
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
    /// use ratatui_testlib::HeadlessBevyRunner;
    ///
    /// #[derive(Component)]
    /// struct Health(u32);
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut runner = HeadlessBevyRunner::new()?;
    /// runner.world_mut().spawn(Health(100));
    ///
    /// let all_health = runner.query::<Health>();
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

    /// Returns the current screen state.
    ///
    /// This is a placeholder for integrating with bevy_ratatui or custom
    /// rendering systems that output to the internal ScreenState.
    ///
    /// # Note
    ///
    /// By default, the screen state is empty. To populate it, you need to:
    /// 1. Use bevy_ratatui and capture Frame output (requires custom adapter)
    /// 2. Implement a system that writes to a shared ScreenState resource
    /// 3. Manually feed terminal sequences via `feed_terminal_output()`
    pub fn screen(&self) -> &ScreenState {
        &self.screen
    }

    /// Feeds terminal output bytes into the internal screen state.
    ///
    /// Use this to manually populate the screen state with terminal escape
    /// sequences generated by your Bevy systems.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Raw terminal output bytes (including ANSI/VT sequences)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(feature = "bevy")]
    /// # {
    /// use ratatui_testlib::HeadlessBevyRunner;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut runner = HeadlessBevyRunner::new()?;
    /// runner.feed_terminal_output(b"\x1b[31mHello\x1b[0m");
    ///
    /// let screen = runner.screen();
    /// assert!(screen.contains("Hello"));
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn feed_terminal_output(&mut self, bytes: &[u8]) {
        self.screen.feed(bytes);
    }

    /// Creates a snapshot string of the current screen state for snapshot testing.
    ///
    /// Returns a formatted string representation suitable for use with
    /// `insta::assert_snapshot!()` or `expect_test::expect!()`.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # #[cfg(all(feature = "bevy", feature = "snapshot-insta"))]
    /// # {
    /// use ratatui_testlib::HeadlessBevyRunner;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut runner = HeadlessBevyRunner::new()?;
    /// // ... run systems ...
    /// runner.tick_n(5)?;
    ///
    /// // Capture snapshot
    /// let snapshot = runner.snapshot();
    /// insta::assert_snapshot!(snapshot);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    pub fn snapshot(&self) -> String {
        self.screen.contents()
    }

    /// Returns terminal dimensions (width, height).
    pub fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// Checks if Sixel graphics are present in the current screen state.
    #[cfg(feature = "sixel")]
    pub fn has_sixel_graphics(&self) -> bool {
        !self.screen.sixel_regions().is_empty()
    }

    /// Captures the current Sixel state.
    #[cfg(feature = "sixel")]
    pub fn capture_sixel_state(&self) -> Result<SixelCapture> {
        Ok(SixelCapture::from_screen_state(&self.screen))
    }

    /// Asserts that all Sixel graphics are within the specified area.
    #[cfg(feature = "sixel")]
    pub fn assert_sixel_within(&self, area: (u16, u16, u16, u16)) -> Result<()> {
        let capture = self.capture_sixel_state()?;
        capture.assert_all_within(area)
    }
}

// ============================================================================
// Benchmarking Support (Issue #13)
// ============================================================================

impl crate::bevy::bench::BenchmarkableHarness for HeadlessBevyRunner {
    fn tick_once(&mut self) -> Result<()> {
        self.tick()
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::*;

    use super::*;

    #[derive(Component)]
    struct TestCounter(u32);

    #[derive(Component)]
    struct TestMarker;

    #[test]
    fn test_create_headless_runner() {
        let result = HeadlessBevyRunner::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_with_dimensions() {
        let runner = HeadlessBevyRunner::with_dimensions(100, 30).unwrap();
        assert_eq!(runner.dimensions(), (100, 30));
    }

    #[test]
    fn test_tick() {
        let mut runner = HeadlessBevyRunner::new().unwrap();
        assert!(runner.tick().is_ok());
    }

    #[test]
    fn test_tick_n() {
        let mut runner = HeadlessBevyRunner::new().unwrap();
        assert!(runner.tick_n(5).is_ok());
    }

    #[test]
    fn test_spawn_and_query() {
        let mut runner = HeadlessBevyRunner::new().unwrap();
        runner.world_mut().spawn(TestCounter(42));

        let counters = runner.query::<TestCounter>();
        assert_eq!(counters.len(), 1);
        assert_eq!(counters[0].0, 42);
    }

    #[test]
    fn test_system_execution() {
        fn increment(mut query: Query<&mut TestCounter>) {
            for mut counter in query.iter_mut() {
                counter.0 += 1;
            }
        }

        let mut runner = HeadlessBevyRunner::new().unwrap();
        runner.app_mut().add_systems(Update, increment);
        runner.world_mut().spawn(TestCounter(0));

        // Initial value
        let counters = runner.query::<TestCounter>();
        assert_eq!(counters[0].0, 0);

        // Run one tick
        runner.tick().unwrap();
        let counters = runner.query::<TestCounter>();
        assert_eq!(counters[0].0, 1);

        // Run 5 more ticks
        runner.tick_n(5).unwrap();
        let counters = runner.query::<TestCounter>();
        assert_eq!(counters[0].0, 6);
    }

    #[test]
    fn test_query_filtered() {
        let mut runner = HeadlessBevyRunner::new().unwrap();
        runner.world_mut().spawn((TestCounter(10), TestMarker));
        runner.world_mut().spawn(TestCounter(20)); // No marker

        let marked = runner.query_filtered::<TestCounter, TestMarker>();
        assert_eq!(marked.len(), 1);
        assert_eq!(marked[0].0, 10);
    }

    #[test]
    fn test_assert_component_exists() {
        let mut runner = HeadlessBevyRunner::new().unwrap();

        // Should fail initially
        assert!(runner.assert_component_exists::<TestCounter>().is_err());

        // Spawn entity
        runner.world_mut().spawn(TestCounter(1));

        // Should pass now
        assert!(runner.assert_component_exists::<TestCounter>().is_ok());
    }

    #[test]
    fn test_assert_component_count() {
        let mut runner = HeadlessBevyRunner::new().unwrap();

        assert!(runner.assert_component_count::<TestCounter>(0).is_ok());
        assert!(runner.assert_component_count::<TestCounter>(1).is_err());

        runner.world_mut().spawn(TestCounter(1));
        runner.world_mut().spawn(TestCounter(2));
        runner.world_mut().spawn(TestCounter(3));

        assert!(runner.assert_component_count::<TestCounter>(3).is_ok());
        assert!(runner.assert_component_count::<TestCounter>(2).is_err());
    }

    #[test]
    fn test_feed_terminal_output() {
        let mut runner = HeadlessBevyRunner::new().unwrap();
        runner.feed_terminal_output(b"Hello World");

        let screen = runner.screen();
        assert!(screen.contains("Hello World"));
    }

    #[test]
    fn test_snapshot() {
        let mut runner = HeadlessBevyRunner::new().unwrap();
        runner.feed_terminal_output(b"Test Output");

        let snapshot = runner.snapshot();
        assert!(snapshot.contains("Test Output"));
    }

    #[test]
    fn test_get_component() {
        let mut runner = HeadlessBevyRunner::new().unwrap();
        let entity = runner.world_mut().spawn(TestCounter(99)).id();

        let component = runner.get_component::<TestCounter>(entity);
        assert!(component.is_some());
        assert_eq!(component.unwrap().0, 99);
    }

    // ========================================================================
    // Benchmarking Tests (Issue #13)
    // ========================================================================

    #[test]
    fn test_benchmark_rendering_headless() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut runner = HeadlessBevyRunner::new().unwrap();

        let results = runner.benchmark_rendering(100).unwrap();

        assert_eq!(results.iterations, 100);
        assert!(results.total_duration_ms > 0.0);
        assert!(results.avg_frame_time_ms > 0.0);
        assert!(results.fps_avg > 0.0);
    }

    #[test]
    fn test_profile_update_cycle_headless() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut runner = HeadlessBevyRunner::new().unwrap();

        let profile = runner.profile_update_cycle().unwrap();

        assert!(profile.duration_ms > 0.0);
        assert!(profile.fps_equivalent > 0.0);
    }

    #[test]
    fn test_assert_fps_headless() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut runner = HeadlessBevyRunner::new().unwrap();

        // Assert a very low FPS requirement
        let results = runner.assert_fps(1.0, 50);
        assert!(results.is_ok());
    }

    #[test]
    fn test_benchmark_with_systems_headless() {
        use crate::bevy::bench::BenchmarkableHarness;

        fn increment(mut query: Query<&mut TestCounter>) {
            for mut counter in query.iter_mut() {
                counter.0 += 1;
            }
        }

        let mut runner = HeadlessBevyRunner::new().unwrap();
        runner.app_mut().add_systems(Update, increment);
        runner.world_mut().spawn(TestCounter(0));

        // Benchmark 50 frames
        let results = runner.benchmark_rendering(50).unwrap();

        assert_eq!(results.iterations, 50);

        // Verify system ran 50 times
        let counters = runner.query::<TestCounter>();
        assert_eq!(counters[0].0, 50);
    }

    #[test]
    fn test_headless_performance_comparison() {
        use crate::bevy::bench::BenchmarkableHarness;

        let mut runner = HeadlessBevyRunner::new().unwrap();

        // Benchmark multiple frames
        let benchmark1 = runner.benchmark_rendering(100).unwrap();
        let benchmark2 = runner.benchmark_rendering(100).unwrap();

        // Two consecutive benchmarks should have similar results
        // (allowing for variance - single samples can vary significantly)
        // Just verify both complete successfully and produce reasonable results
        assert!(benchmark1.avg_frame_time_ms > 0.0);
        assert!(benchmark2.avg_frame_time_ms > 0.0);
        assert!(benchmark1.fps_avg > 0.0);
        assert!(benchmark2.fps_avg > 0.0);

        // Verify percentiles are ordered correctly
        assert!(benchmark1.min_frame_time_ms <= benchmark1.p50_ms);
        assert!(benchmark1.p50_ms <= benchmark1.p95_ms);
        assert!(benchmark1.p95_ms <= benchmark1.p99_ms);
        assert!(benchmark1.p99_ms <= benchmark1.max_frame_time_ms);
    }
}
