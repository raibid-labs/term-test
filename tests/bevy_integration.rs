//! Bevy ECS integration tests.
//!
//! Tests for Issue #9: Bevy ECS integration for testing Bevy+Ratatui applications.

#[cfg(feature = "bevy")]
mod bevy_tests {
    use bevy::prelude::*;
    use ratatui_testlib::{BevyTuiTestHarness, Result};

    // ========================================================================
    // Test Components
    // ========================================================================

    #[derive(Component)]
    struct Health(u32);

    #[derive(Component)]
    struct Position {
        x: i32,
        y: i32,
    }

    #[derive(Component)]
    struct Velocity {
        dx: i32,
        dy: i32,
    }

    #[derive(Component)]
    struct CommandPaletteMarker;

    #[derive(Component)]
    struct EnemyMarker;

    // ========================================================================
    // Basic Harness Tests
    // ========================================================================

    #[test]
    fn test_bevy_harness_creation() -> Result<()> {
        let _harness = BevyTuiTestHarness::new()?;
        Ok(())
    }

    #[test]
    fn test_bevy_update() -> Result<()> {
        let mut harness = BevyTuiTestHarness::new()?;
        harness.update()?;
        Ok(())
    }

    #[test]
    fn test_bevy_update_n() -> Result<()> {
        let mut harness = BevyTuiTestHarness::new()?;
        harness.update_n(10)?;
        Ok(())
    }

    // ========================================================================
    // ECS Query Tests (Issue #9)
    // ========================================================================

    #[test]
    fn test_query_components() -> Result<()> {
        let mut harness = BevyTuiTestHarness::new()?;

        // Spawn entities
        harness.world_mut().spawn(Health(100));
        harness.world_mut().spawn(Health(75));
        harness.world_mut().spawn(Health(50));

        // Query all health components
        let health_components = harness.query::<Health>();
        assert_eq!(health_components.len(), 3);

        Ok(())
    }

    #[test]
    fn test_query_filtered_components() -> Result<()> {
        let mut harness = BevyTuiTestHarness::new()?;

        // Spawn entities with different markers
        harness
            .world_mut()
            .spawn((Position { x: 10, y: 20 }, EnemyMarker));
        harness
            .world_mut()
            .spawn((Position { x: 30, y: 40 }, EnemyMarker));
        harness.world_mut().spawn(Position { x: 5, y: 15 }); // No marker

        // Query only enemy positions
        let enemy_positions = harness.query_filtered::<Position, EnemyMarker>();
        assert_eq!(enemy_positions.len(), 2);

        Ok(())
    }

    #[test]
    fn test_get_component_by_entity() -> Result<()> {
        let mut harness = BevyTuiTestHarness::new()?;

        let entity = harness.world_mut().spawn(Health(100)).id();

        // Get component
        let health = harness.get_component::<Health>(entity);
        assert!(health.is_some());
        assert_eq!(health.unwrap().0, 100);

        Ok(())
    }

    #[test]
    fn test_assert_component_exists() -> Result<()> {
        let mut harness = BevyTuiTestHarness::new()?;

        // Should fail initially
        assert!(harness
            .assert_component_exists::<CommandPaletteMarker>()
            .is_err());

        // Spawn component
        harness.world_mut().spawn(CommandPaletteMarker);

        // Should pass now
        harness.assert_component_exists::<CommandPaletteMarker>()?;

        Ok(())
    }

    #[test]
    fn test_assert_component_count() -> Result<()> {
        let mut harness = BevyTuiTestHarness::new()?;

        // Initially zero
        harness.assert_component_count::<EnemyMarker>(0)?;

        // Spawn 3 enemies
        harness.world_mut().spawn(EnemyMarker);
        harness.world_mut().spawn(EnemyMarker);
        harness.world_mut().spawn(EnemyMarker);

        // Assert count
        harness.assert_component_count::<EnemyMarker>(3)?;

        Ok(())
    }

    // ========================================================================
    // System Execution Tests
    // ========================================================================

    #[test]
    fn test_system_execution() -> Result<()> {
        fn movement_system(mut query: Query<'_, '_, (&mut Position, &Velocity)>) {
            for (mut pos, vel) in query.iter_mut() {
                pos.x += vel.dx;
                pos.y += vel.dy;
            }
        }

        // Create app with system
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_systems(Update, movement_system);

        let mut harness = BevyTuiTestHarness::with_app(app)?;

        // Spawn entity with position and velocity
        harness
            .world_mut()
            .spawn((Position { x: 0, y: 0 }, Velocity { dx: 5, dy: 10 }));

        // Initial position
        let positions = harness.query::<Position>();
        assert_eq!(positions[0].x, 0);
        assert_eq!(positions[0].y, 0);

        // Run update (executes movement system)
        harness.update()?;

        // Position should be updated
        let positions = harness.query::<Position>();
        assert_eq!(positions[0].x, 5);
        assert_eq!(positions[0].y, 10);

        // Run 3 more times
        harness.update_bevy(3)?;
        let positions = harness.query::<Position>();
        assert_eq!(positions[0].x, 20);
        assert_eq!(positions[0].y, 40);

        Ok(())
    }

    // ========================================================================
    // Hybrid ECS + Screen State Tests
    // ========================================================================

    #[test]
    fn test_hybrid_ecs_and_screen() -> Result<()> {
        let mut harness = BevyTuiTestHarness::new()?;

        // Test ECS component
        harness.world_mut().spawn(Health(100));
        harness.assert_component_exists::<Health>()?;

        // Test screen state (empty initially)
        let screen = harness.state();
        // Screen should not contain "Health" text
        assert!(!screen.contains("Health"));

        Ok(())
    }

    #[test]
    fn test_command_palette_scenario() -> Result<()> {
        // Simulates the example from the issue description
        let mut harness = BevyTuiTestHarness::new()?;

        // Initially no command palette
        assert!(harness
            .assert_component_exists::<CommandPaletteMarker>()
            .is_err());

        // Simulate opening command palette
        harness.world_mut().spawn(CommandPaletteMarker);
        harness.update()?;

        // Assert: Bevy component exists
        harness.assert_component_exists::<CommandPaletteMarker>()?;

        // Note: Screen assertion would require actual rendering system
        // For now, we just verify the ECS state is correct

        Ok(())
    }
}
