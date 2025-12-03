//! Example demonstrating HybridBevyHarness for testing client-daemon architectures.
//!
//! This example shows how to use HybridBevyHarness to test a Bevy-based client
//! that communicates with a daemon process running in a PTY.

#[cfg(feature = "bevy")]
fn main() -> ratatui_testlib::Result<()> {
    use bevy::prelude::*;
    use portable_pty::CommandBuilder;
    use ratatui_testlib::HybridBevyHarness;

    // Define client-side components
    #[derive(Component)]
    struct ConnectionState {
        connected: bool,
        daemon_status: String,
    }

    #[derive(Component)]
    struct ClientMarker;

    // System that updates connection state
    fn update_connection_system(mut query: Query<&mut ConnectionState>) {
        for mut state in query.iter_mut() {
            if !state.connected {
                state.connected = true;
                state.daemon_status = "Connected to daemon".to_string();
            }
        }
    }

    println!("HybridBevyHarness Example: Client-Daemon Architecture\n");

    // Create a hybrid harness with a daemon command
    // Note: In a real scenario, this would be an actual daemon executable
    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("Daemon Ready");

    let mut harness = HybridBevyHarness::builder()
        .with_dimensions(80, 24)
        // For demonstration, we use 'echo' which will exit immediately
        // In production, this would be your daemon process
        .with_pty_command(cmd)
        .build()?;

    println!("✓ Created hybrid harness");

    // Add systems to the in-process Bevy app
    harness
        .app_mut()
        .add_systems(Update, update_connection_system);

    println!("✓ Added client systems");

    // Spawn client entities
    harness.world_mut().spawn((
        ConnectionState {
            connected: false,
            daemon_status: "Not connected".to_string(),
        },
        ClientMarker,
    ));

    println!("✓ Spawned client entities");

    // Check initial client state
    let states = harness.query::<ConnectionState>();
    println!("\nInitial client state:");
    println!("  - Connected: {}", states[0].connected);
    println!("  - Status: {}", states[0].daemon_status);

    // Spawn the daemon process
    harness.spawn_daemon()?;
    println!("\n✓ Spawned daemon process");

    // Wait for daemon output
    // Note: Since we used 'echo', the daemon will exit immediately
    // In a real scenario, you'd wait for specific daemon output
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Run a client update tick
    harness.tick()?;
    println!("✓ Ran client update cycle");

    // Check updated client state
    let states = harness.query::<ConnectionState>();
    println!("\nUpdated client state:");
    println!("  - Connected: {}", states[0].connected);
    println!("  - Status: {}", states[0].daemon_status);

    // Verify ECS state
    harness.assert_component_count::<ClientMarker>(1)?;
    harness.assert_component_exists::<ConnectionState>()?;
    println!("\n✓ Verified client ECS state");

    // Check daemon screen (if still running)
    if let Ok(screen) = harness.daemon_screen_contents() {
        println!("\nDaemon output:");
        println!("{}", screen);
    }

    // Feed some output to the client screen (simulating bevy_ratatui)
    harness.feed_client_output(b"Client UI: Connection established\n");
    println!("\nClient screen:");
    println!("{}", harness.client_screen_contents());

    println!("\n✓ Example completed successfully!");

    Ok(())
}

#[cfg(not(feature = "bevy"))]
fn main() {
    eprintln!("This example requires the 'bevy' feature to be enabled.");
    eprintln!("Run with: cargo run --example hybrid_bevy_client_daemon --features bevy");
    std::process::exit(1);
}
