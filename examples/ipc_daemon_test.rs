//! Example demonstrating IPC testing with split-process terminal daemons.
//!
//! This example shows how to use `ratatui-testlib` to test terminal applications
//! using a daemon + client architecture where the daemon manages the PTY and
//! exposes state via shared memory.
//!
//! # Prerequisites
//!
//! 1. A running terminal daemon that:
//!    - Listens on a Unix socket
//!    - Exposes terminal state via POSIX shared memory
//!
//! 2. Environment variable set:
//!    ```bash
//!    export RTL_IPC_TEST=1
//!    ```
//!
//! # Running this example
//!
//! ```bash
//! # Start your daemon first
//! my-term-daemon &
//!
//! # Run the example
//! RTL_IPC_TEST=1 cargo run --example ipc_daemon_test --features ipc
//! ```

#[cfg(feature = "ipc")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Duration;

    use ratatui_testlib::ipc::{DaemonConfig, DaemonTestHarness, IpcError};

    println!("IPC Daemon Test Example");
    println!("=======================\n");

    // Check if IPC testing is enabled
    if !DaemonTestHarness::is_enabled() {
        println!("IPC testing is disabled.");
        println!("Set RTL_IPC_TEST=1 to enable IPC testing.\n");
        println!("Example:");
        println!("  RTL_IPC_TEST=1 cargo run --example ipc_daemon_test --features ipc\n");
        return Ok(());
    }

    println!("IPC testing is enabled!\n");

    // Configure the test harness
    let config = DaemonConfig::builder()
        .socket_path("/tmp/term-daemon.sock")
        .shm_path("/term_shm_v1")
        .connect_timeout(Duration::from_secs(5))
        .default_timeout(Duration::from_secs(10))
        .build();

    println!("Configuration:");
    println!("  Socket: {:?}", config.socket_path);
    println!("  Shared memory: {}", config.shm_path);
    println!();

    // Try to connect to the daemon
    match DaemonTestHarness::with_config(config) {
        Ok(mut harness) => {
            println!("Connected to daemon successfully!\n");

            // Get terminal dimensions
            let (cols, rows) = harness.dimensions();
            println!("Terminal size: {}x{}", cols, rows);

            // Get cursor position
            let (row, col) = harness.cursor_position()?;
            println!("Cursor position: row={}, col={}", row, col);

            // Read initial grid contents
            let grid = harness.grid_contents()?;
            println!("\nInitial grid contents:");
            println!("---");
            for line in grid.lines().take(5) {
                println!("{}", line);
            }
            if grid.lines().count() > 5 {
                println!("... ({} more lines)", grid.lines().count() - 5);
            }
            println!("---\n");

            // Send a test command
            println!("Sending test command: echo 'Hello from ratatui-testlib!'");
            harness.send_input("echo 'Hello from ratatui-testlib!'\n")?;

            // Wait for the output
            println!("Waiting for output...");
            match harness.wait_for_text("Hello from ratatui-testlib!", Duration::from_secs(5)) {
                Ok(()) => {
                    println!("Output received!\n");

                    // Read updated grid
                    let grid = harness.grid_contents()?;
                    println!("Updated grid contents:");
                    println!("---");
                    for line in grid.lines().take(10) {
                        println!("{}", line);
                    }
                    println!("---\n");
                }
                Err(IpcError::Timeout(duration)) => {
                    println!("Timeout after {:?} waiting for output.", duration);
                    println!("This might mean the daemon isn't processing input.\n");
                }
                Err(e) => {
                    println!("Error waiting for output: {}", e);
                }
            }

            println!("IPC test completed successfully!");
        }
        Err(IpcError::SocketNotFound(path)) => {
            println!("Daemon socket not found at: {}", path.display());
            println!("\nMake sure the daemon is running:");
            println!("  my-term-daemon &");
        }
        Err(IpcError::SharedMemoryNotFound(path)) => {
            println!("Shared memory not found at: {}", path);
            println!("\nMake sure the daemon creates shared memory.");
        }
        Err(e) => {
            println!("Failed to connect to daemon: {}", e);
        }
    }

    Ok(())
}

#[cfg(not(feature = "ipc"))]
fn main() {
    println!("This example requires the 'ipc' feature.");
    println!("Run with: cargo run --example ipc_daemon_test --features ipc");
}
