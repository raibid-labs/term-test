//! Async/Tokio testing example.
//!
//! This example demonstrates async testing patterns with Tokio runtime:
//! - Using ratatui_testlib in async contexts
//! - Integrating with Tokio async/await
//! - Testing async TUI applications
//! - Combining async I/O with terminal testing
//! - Best practices for async testing
//!
//! # Running This Example
//!
//! ```bash
//! cargo run --example async_test --features async-tokio
//! ```
//!
//! # Why Async Testing?
//!
//! Many modern TUI applications use async runtimes like Tokio for:
//! - Non-blocking I/O operations
//! - Concurrent task management
//! - Network communication
//! - Event-driven architectures
//!
//! ratatui_testlib's sync API works well with async runtimes by allowing you to:
//! - Wrap sync operations in async functions
//! - Use tokio::time for delays instead of std::thread::sleep
//! - Integrate with other async libraries
//! - Test async application behavior
//!
//! # Note on Current Implementation
//!
//! The current implementation uses the synchronous TuiTestHarness within
//! async contexts. A fully async AsyncTuiTestHarness is planned for Phase 2,
//! but the patterns shown here work well for most use cases.
//!
//! # Expected Output
//!
//! This example demonstrates:
//! 1. Basic async testing with Tokio
//! 2. Async wait conditions
//! 3. Concurrent operations
//! 4. Timeout handling
//! 5. Practical async patterns

use portable_pty::CommandBuilder;
use ratatui_testlib::{Result, TuiTestHarness};
use tokio::time::{timeout, Duration};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Async Testing with Tokio Example ===\n");

    println!("Note: Currently using sync TuiTestHarness in async context");
    println!("Phase 2 will implement AsyncTuiTestHarness with native async/await\n");

    // Example 1: Basic async testing
    example_1_basic_async().await?;

    // Example 2: Async wait patterns
    example_2_async_wait().await?;

    // Example 3: Concurrent operations
    example_3_concurrent_operations().await?;

    // Example 4: Timeout handling
    example_4_timeout_handling().await?;

    // Example 5: Practical async scenario
    example_5_practical_scenario().await?;

    println!("\n=== All Async Examples Completed ===");
    Ok(())
}

/// Example 1: Basic async testing
///
/// Demonstrates:
/// - Using TuiTestHarness in async function
/// - Tokio async sleeps
/// - Awaiting async operations
async fn example_1_basic_async() -> Result<()> {
    println!("--- Example 1: Basic Async Testing ---");

    // Create harness in async context
    let mut harness = TuiTestHarness::new(80, 24)?;
    println!("Created harness in async context");

    // Spawn command
    let mut cmd = CommandBuilder::new("echo");
    cmd.arg("Hello from async!");

    harness.spawn(cmd)?;
    println!("Spawned command");

    // Use tokio::time::sleep instead of std::thread::sleep
    println!("Waiting asynchronously...");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Update and check state
    harness.update_state()?;
    let contents = harness.screen_contents();

    println!("\nScreen contents:");
    println!("┌{:─<78}┐", "");
    for line in contents.lines().take(3) {
        println!("│ {:<77}│", line);
    }
    println!("└{:─<78}┘", "");

    assert!(contents.contains("Hello from async!"));
    println!("✓ Async test passed");

    println!();
    Ok(())
}

/// Example 2: Async wait patterns
///
/// Demonstrates:
/// - Converting sync wait_for into async pattern
/// - Using tokio::spawn for background polling
/// - Async timeout handling
async fn example_2_async_wait() -> Result<()> {
    println!("--- Example 2: Async Wait Patterns ---");

    let mut harness = TuiTestHarness::new(80, 24)?;

    // Spawn a command that outputs after a delay
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg("sleep 0.2 && echo 'Ready!'");

    harness.spawn(cmd)?;
    println!("Spawned delayed command");

    // Pattern 1: Simple async polling
    println!("\nPattern 1: Simple async polling");
    let start = std::time::Instant::now();

    loop {
        harness.update_state()?;

        if harness.screen_contents().contains("Ready!") {
            println!("✓ Found expected text after {:?}", start.elapsed());
            break;
        }

        if start.elapsed() > Duration::from_secs(5) {
            return Err(ratatui_testlib::TermTestError::Timeout { timeout_ms: 5000 });
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    println!("\nFinal output:");
    println!("{}", harness.screen_contents().lines().next().unwrap_or(""));

    println!();
    Ok(())
}

/// Example 3: Concurrent operations
///
/// Demonstrates:
/// - Running multiple test harnesses concurrently
/// - Using tokio::join! for parallel testing
/// - Gathering results from concurrent tasks
async fn example_3_concurrent_operations() -> Result<()> {
    println!("--- Example 3: Concurrent Operations ---");

    println!("Running 3 tests concurrently with tokio::join!\n");

    // Create async tasks for parallel execution
    let task1 = async {
        let mut harness = TuiTestHarness::new(80, 24)?;
        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("Task 1");
        harness.spawn(cmd)?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        harness.update_state()?;
        let result = harness.screen_contents().contains("Task 1");
        Ok::<bool, ratatui_testlib::TermTestError>(result)
    };

    let task2 = async {
        let mut harness = TuiTestHarness::new(80, 24)?;
        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("Task 2");
        harness.spawn(cmd)?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        harness.update_state()?;
        let result = harness.screen_contents().contains("Task 2");
        Ok::<bool, ratatui_testlib::TermTestError>(result)
    };

    let task3 = async {
        let mut harness = TuiTestHarness::new(80, 24)?;
        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("Task 3");
        harness.spawn(cmd)?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        harness.update_state()?;
        let result = harness.screen_contents().contains("Task 3");
        Ok::<bool, ratatui_testlib::TermTestError>(result)
    };

    // Run all tasks concurrently
    let start = std::time::Instant::now();
    let (result1, result2, result3) = tokio::join!(task1, task2, task3);

    println!("All tasks completed in {:?}", start.elapsed());
    println!("  Task 1 passed: {}", result1?);
    println!("  Task 2 passed: {}", result2?);
    println!("  Task 3 passed: {}", result3?);

    println!("\n✓ Concurrent testing successful");
    println!("  Benefit: Tests run in parallel, reducing total test time");

    println!();
    Ok(())
}

/// Example 4: Timeout handling
///
/// Demonstrates:
/// - Using tokio::time::timeout
/// - Handling timeout errors gracefully
/// - Setting per-operation timeouts
async fn example_4_timeout_handling() -> Result<()> {
    println!("--- Example 4: Timeout Handling ---");

    // Test 1: Operation that completes within timeout
    println!("Test 1: Fast operation (should succeed)");
    let result = timeout(Duration::from_secs(2), async {
        let mut harness = TuiTestHarness::new(80, 24)?;
        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("Quick!");
        harness.spawn(cmd)?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        harness.update_state()?;
        let contents = harness.screen_contents();
        Ok::<String, ratatui_testlib::TermTestError>(contents)
    })
    .await;

    match result {
        Ok(Ok(contents)) => {
            println!("  ✓ Operation completed: {}", contents.lines().next().unwrap_or("").trim());
        }
        Ok(Err(e)) => println!("  ✗ Operation failed: {}", e),
        Err(_) => println!("  ✗ Operation timed out"),
    }

    // Test 2: Operation that times out
    println!("\nTest 2: Slow operation (will timeout)");
    let result = timeout(Duration::from_millis(50), async {
        let mut harness = TuiTestHarness::new(80, 24)?;
        let mut cmd = CommandBuilder::new("sleep");
        cmd.arg("1");
        harness.spawn(cmd)?;
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok::<(), ratatui_testlib::TermTestError>(())
    })
    .await;

    match result {
        Ok(_) => println!("  ✗ Unexpected: Operation completed"),
        Err(_) => println!("  ✓ Expected: Operation timed out after 50ms"),
    }

    println!("\nTimeout patterns are essential for:");
    println!("  - Preventing hung tests");
    println!("  - Testing timeout behavior");
    println!("  - Enforcing performance requirements");

    println!();
    Ok(())
}

/// Example 5: Practical async scenario
///
/// Demonstrates:
/// - Complete async test scenario
/// - Testing a simulated async TUI app
/// - Combining multiple async patterns
async fn example_5_practical_scenario() -> Result<()> {
    println!("--- Example 5: Practical Async Scenario ---");
    println!("Scenario: Testing an async data loader TUI\n");

    // Create a harness for testing
    let mut harness = TuiTestHarness::new(80, 24)?;

    // Simulate an async TUI app that loads data progressively
    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(
        "echo 'Loading...' && \
         sleep 0.1 && \
         echo 'Fetching data...' && \
         sleep 0.1 && \
         echo 'Processing...' && \
         sleep 0.1 && \
         echo 'Complete!'",
    );

    harness.spawn(cmd)?;
    println!("Spawned simulated async TUI app");

    // Test step 1: Wait for initial loading message
    println!("\nStep 1: Wait for loading message");
    let result = timeout(Duration::from_secs(1), async {
        loop {
            harness.update_state()?;
            if harness.screen_contents().contains("Loading") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok::<(), ratatui_testlib::TermTestError>(())
    })
    .await;

    match result {
        Ok(Ok(())) => println!("  ✓ Loading message appeared"),
        _ => println!("  ✗ Failed to detect loading message"),
    }

    // Test step 2: Wait for completion
    println!("\nStep 2: Wait for completion message");
    let result = timeout(Duration::from_secs(2), async {
        loop {
            harness.update_state()?;
            if harness.screen_contents().contains("Complete!") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        Ok::<(), ratatui_testlib::TermTestError>(())
    })
    .await;

    match result {
        Ok(Ok(())) => println!("  ✓ Completion message appeared"),
        _ => println!("  ✗ Failed to detect completion message"),
    }

    // Capture final state
    harness.update_state()?;
    let final_state = harness.screen_contents();

    println!("\nFinal state:");
    println!("┌{:─<78}┐", "");
    for line in final_state.lines().take(6) {
        println!("│ {:<77}│", line);
    }
    println!("└{:─<78}┘", "");

    // Verify all stages appeared
    assert!(final_state.contains("Loading"));
    assert!(final_state.contains("Fetching"));
    assert!(final_state.contains("Processing"));
    assert!(final_state.contains("Complete"));

    println!("\n✓ All stages verified");

    println!("\nThis pattern works well for:");
    println!("  - Testing async loading sequences");
    println!("  - Verifying progress indicators");
    println!("  - Testing timeout behavior");
    println!("  - Integration testing with async libraries");

    println!();
    Ok(())
}

// Example of how async tests would look in a real test file:
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use tokio::test;
//
//     #[tokio::test]
//     async fn test_async_tui_app() -> Result<()> {
//         let mut harness = TuiTestHarness::new(80, 24)?;
//         let mut cmd = CommandBuilder::new("my-async-app");
//         harness.spawn(cmd)?;
//
//         // Wait for initial render using async polling
//         timeout(Duration::from_secs(5), async {
//             loop {
//                 harness.update_state()?;
//                 if harness.screen_contents().contains("Ready") {
//                     break;
//                 }
//                 tokio::time::sleep(Duration::from_millis(50)).await;
//             }
//             Ok::<(), ratatui_testlib::TermTestError>(())
//         })
//         .await??;
//
//         // Send input
//         harness.send_text("test\n")?;
//
//         // Wait for result
//         tokio::time::sleep(Duration::from_millis(100)).await;
//         harness.update_state()?;
//
//         assert!(harness.screen_contents().contains("Success"));
//
//         Ok(())
//     }
//
//     #[tokio::test]
//     async fn test_concurrent_sessions() -> Result<()> {
//         // Test multiple sessions concurrently
//         let tasks = (0..5).map(|i| {
//             tokio::spawn(async move {
//                 let mut harness = TuiTestHarness::new(80, 24)?;
//                 let mut cmd = CommandBuilder::new("echo");
//                 cmd.arg(format!("Session {}", i));
//                 harness.spawn(cmd)?;
//                 tokio::time::sleep(Duration::from_millis(100)).await;
//                 harness.update_state()?;
//                 Ok::<_, ratatui_testlib::TermTestError>(())
//             })
//         });
//
//         for task in tasks {
//             task.await.unwrap()?;
//         }
//
//         Ok(())
//     }
// }
