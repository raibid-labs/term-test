//! Demonstration of enhanced PTY features
//!
//! This example showcases all the new process management and I/O features
//! added to the TestTerminal PTY wrapper.

use portable_pty::CommandBuilder;
use std::time::Duration;
use mimic::TestTerminal;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("PTY Enhanced Features Demo");
    println!("===========================\n");

    // Demo 1: Custom buffer configuration
    demo_custom_buffer()?;

    // Demo 2: Spawn with arguments and environment
    demo_spawn_with_env()?;

    // Demo 3: Read with timeout
    demo_read_timeout()?;

    // Demo 4: Process lifecycle management
    demo_process_lifecycle()?;

    // Demo 5: Robust write operations
    demo_robust_write()?;

    println!("\nAll demos completed successfully!");
    Ok(())
}

fn demo_custom_buffer() -> Result<(), Box<dyn std::error::Error>> {
    println!("Demo 1: Custom Buffer Configuration");
    println!("------------------------------------");

    let _terminal = TestTerminal::new(80, 24)?
        .with_buffer_size(16384);  // 16KB buffer for high-throughput

    println!("Created terminal with 16KB buffer");
    println!("Use larger buffers for applications with high output volume\n");

    Ok(())
}

fn demo_spawn_with_env() -> Result<(), Box<dyn std::error::Error>> {
    println!("Demo 2: Spawn with Arguments and Environment");
    println!("---------------------------------------------");

    let mut terminal = TestTerminal::new(80, 24)?;

    // Build command with args and env vars
    let mut cmd = CommandBuilder::new("bash");
    cmd.arg("-c");
    cmd.arg("echo Hello from $USER in $PWD");
    cmd.env("USER", "test-user");
    cmd.env("PWD", "/tmp");

    // Spawn with custom timeout
    terminal.spawn_with_timeout(cmd, Duration::from_secs(3))?;
    println!("Spawned bash with custom environment");

    // Give it time to execute
    std::thread::sleep(Duration::from_millis(100));

    // Read output
    let output = terminal.read_all()?;
    println!("Output: {}", String::from_utf8_lossy(&output).trim());

    // Wait for process to exit
    terminal.wait()?;
    println!("Process exited successfully\n");

    Ok(())
}

fn demo_read_timeout() -> Result<(), Box<dyn std::error::Error>> {
    println!("Demo 3: Read with Timeout");
    println!("-------------------------");

    let mut terminal = TestTerminal::new(80, 24)?;

    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("Quick output");
    terminal.spawn(cmd)?;

    // Wait up to 1 second for output
    let mut buf = [0u8; 1024];
    match terminal.read_timeout(&mut buf, Duration::from_secs(1)) {
        Ok(n) => {
            println!("Read {} bytes within timeout", n);
            println!("Data: {}", String::from_utf8_lossy(&buf[..n]).trim());
        }
        Err(e) => {
            println!("Timeout or error: {}", e);
        }
    }

    terminal.wait()?;
    println!();

    Ok(())
}

fn demo_process_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    println!("Demo 4: Process Lifecycle Management");
    println!("-------------------------------------");

    let mut terminal = TestTerminal::new(80, 24)?;

    // Spawn a long-running process
    let mut cmd = CommandBuilder::new("sleep");
    cmd.arg("10");
    terminal.spawn(cmd)?;
    println!("Spawned sleep process for 10 seconds");

    // Check if running
    if terminal.is_running() {
        println!("Process is running");
    }

    // Try to wait with a short timeout
    match terminal.wait_timeout(Duration::from_millis(500)) {
        Ok(status) => {
            println!("Process exited: {:?}", status);
        }
        Err(_) => {
            println!("Process still running after 500ms - killing it");
            terminal.kill()?;
            println!("Process terminated");
        }
    }

    // Verify it's not running
    if !terminal.is_running() {
        println!("Process is no longer running");
    }

    println!();

    Ok(())
}

fn demo_robust_write() -> Result<(), Box<dyn std::error::Error>> {
    println!("Demo 5: Robust Write Operations");
    println!("--------------------------------");

    let mut terminal = TestTerminal::new(80, 24)?;

    let cmd = CommandBuilder::new("cat");
    terminal.spawn(cmd)?;

    std::thread::sleep(Duration::from_millis(50));

    // Write with automatic EINTR retry
    let message = b"Line 1\nLine 2\nLine 3\n";
    terminal.write_all(message)?;
    println!("Wrote {} bytes (with EINTR handling)", message.len());

    // Give cat time to echo
    std::thread::sleep(Duration::from_millis(100));

    // Read back the echoed data
    let output = terminal.read_all()?;
    println!("Read back {} bytes", output.len());

    // Kill cat since it runs forever
    terminal.kill()?;
    println!("Cleaned up cat process\n");

    Ok(())
}
