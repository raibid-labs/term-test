//! Demonstration of async/await support and enhanced wait operations.
//!
//! This example shows:
//! - Basic async wait operations
//! - WaitBuilder with custom timeouts and callbacks
//! - Waiting for multiple conditions
//! - Progress callbacks for long-running operations
//! - Hierarchical timeouts
//!
//! Run with:
//! ```bash
//! cargo run --example async_wait_demo --features async-tokio
//! ```

#[cfg(feature = "async-tokio")]
use std::time::Duration;

#[cfg(feature = "async-tokio")]
use portable_pty::CommandBuilder;
#[cfg(feature = "async-tokio")]
use ratatui_testlib::{AsyncTuiTestHarness, WaitResult};

#[cfg(feature = "async-tokio")]
#[tokio::main]
async fn main() -> ratatui_testlib::Result<()> {
    println!("=== Async Wait Demo ===\n");

    // Example 1: Basic async wait
    println!("1. Basic async wait for text...");
    {
        let mut harness = AsyncTuiTestHarness::new(80, 24).await?;

        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("Hello from async!");
        harness.spawn(cmd).await?;

        harness.wait_for_text("Hello").await?;
        println!("   ✓ Found 'Hello' in output\n");
    }

    // Example 2: Async wait with custom timeout
    println!("2. Async wait with custom timeout...");
    {
        let mut harness = AsyncTuiTestHarness::new(80, 24).await?;

        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("Quick response");
        harness.spawn(cmd).await?;

        harness
            .wait_for_async(|state| state.contains("Quick"))
            .timeout(Duration::from_secs(2))
            .execute()
            .await?;

        println!("   ✓ Found text within 2 second timeout\n");
    }

    // Example 3: Wait for any of multiple conditions
    println!("3. Wait for any of multiple conditions...");
    {
        let mut harness = AsyncTuiTestHarness::new(80, 24).await?;

        #[cfg(unix)]
        {
            let mut cmd = CommandBuilder::new("sh");
            cmd.arg("-c");
            cmd.arg("echo 'Operation completed successfully'");
            harness.spawn(cmd).await?;
        }

        #[cfg(windows)]
        {
            let mut cmd = CommandBuilder::new("cmd");
            cmd.arg("/C");
            cmd.arg("echo Operation completed successfully");
            harness.spawn(cmd).await?;
        }

        let result = harness
            .wait_for_any_async()
            .add_condition(|state| state.contains("failed"))
            .add_condition(|state| state.contains("completed"))
            .add_condition(|state| state.contains("timeout"))
            .timeout(Duration::from_secs(5))
            .execute()
            .await?;

        match result {
            WaitResult::Condition(0) => println!("   → Operation failed"),
            WaitResult::Condition(1) => println!("   ✓ Operation completed successfully"),
            WaitResult::Condition(2) => println!("   → Operation timed out"),
            WaitResult::Timeout(_) => println!("   → No condition matched within timeout"),
            _ => {}
        }
        println!();
    }

    // Example 4: Custom poll interval for responsiveness
    println!("4. Fast polling for quick responses...");
    {
        let mut harness = AsyncTuiTestHarness::new(80, 24).await?;

        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("Fast!");
        harness.spawn(cmd).await?;

        let start = std::time::Instant::now();
        harness
            .wait_for_async(|state| state.contains("Fast"))
            .timeout(Duration::from_secs(1))
            .poll_interval(Duration::from_millis(10))
            .execute()
            .await?;

        let elapsed = start.elapsed();
        println!("   ✓ Found text in {:?} (with 10ms polling)\n", elapsed);
    }

    // Example 5: Demonstration of timeout behavior
    println!("5. Demonstrating timeout behavior...");
    {
        let mut harness = AsyncTuiTestHarness::new(80, 24).await?;

        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("Something");
        harness.spawn(cmd).await?;

        let result = harness
            .wait_for_async(|state| state.contains("NeverAppears"))
            .timeout(Duration::from_millis(500))
            .execute()
            .await;

        match result {
            Err(ratatui_testlib::TermTestError::Timeout { timeout_ms }) => {
                println!("   ✓ Correctly timed out after {}ms", timeout_ms);
            }
            Err(ratatui_testlib::TermTestError::ProcessExited) => {
                println!("   ✓ Process exited before timeout (also acceptable)");
            }
            Ok(_) => {
                println!("   ✗ Unexpected success");
            }
            Err(e) => {
                println!("   ✗ Unexpected error: {}", e);
            }
        }
        println!();
    }

    // Example 6: Sequential async operations
    // println!("6. Sequential async operations...");
    // {
    // let mut harness = AsyncTuiTestHarness::new(80, 24).await?;
    //
    // #[cfg(unix)]
    // {
    // let mut cmd = CommandBuilder::new("sh");
    // cmd.arg("-c");
    // cmd.arg("echo 'Step 1' && sleep 1.0 && echo 'Step 2'");
    // harness.spawn(cmd).await?;
    // }
    //
    // #[cfg(windows)]
    // {
    // let mut cmd = CommandBuilder::new("cmd");
    // cmd.arg("/C");
    // cmd.arg("echo Step 1 && timeout /t 0 /nobreak > nul && echo Step 2");
    // harness.spawn(cmd).await?;
    // }
    //
    // Wait for each step in sequence
    // harness.wait_for_text("Step 1").await?;
    // println!("   ✓ Completed Step 1");
    //
    // match harness.wait_for_text("Step 2").await {
    // Ok(_) => println!("   ✓ Completed Step 2\n"),
    // Err(e) => {
    // println!("   ✗ Failed to find Step 2: {}", e);
    // println!("     Screen content:\n{}", harness.screen_contents().await);
    // return Err(e);
    // }
    // }
    // }

    // Example 7: Using async in concurrent tasks
    println!("7. Concurrent async tasks...");
    {
        use tokio::time::timeout;

        let task1 = async {
            let mut harness = AsyncTuiTestHarness::new(80, 24).await?;
            let mut cmd = CommandBuilder::new("echo");
            cmd.arg("Task 1");
            harness.spawn(cmd).await?;
            harness.wait_for_text("Task 1").await?;
            Ok::<_, ratatui_testlib::TermTestError>(())
        };

        let task2 = async {
            let mut harness = AsyncTuiTestHarness::new(80, 24).await?;
            let mut cmd = CommandBuilder::new("echo");
            cmd.arg("Task 2");
            harness.spawn(cmd).await?;
            harness.wait_for_text("Task 2").await?;
            Ok::<_, ratatui_testlib::TermTestError>(())
        };

        // Run both tasks concurrently with a timeout
        let result =
            timeout(Duration::from_secs(5), async { tokio::try_join!(task1, task2) }).await;

        match result {
            Ok(Ok(_)) => println!("   ✓ Both tasks completed successfully"),
            Ok(Err(e)) => println!("   ✗ Task failed: {}", e),
            Err(_) => println!("   ✗ Tasks timed out"),
        }
        println!();
    }

    println!("=== Demo Complete ===");
    Ok(())
}

#[cfg(not(feature = "async-tokio"))]
fn main() {
    eprintln!("This example requires the 'async-tokio' feature.");
    eprintln!("Run with: cargo run --example async_wait_demo --features async-tokio");
    std::process::exit(1);
}
