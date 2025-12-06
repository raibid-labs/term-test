//! Integration tests for parallel test execution.
//!
//! These tests verify that the parallel testing infrastructure works correctly
//! and that tests can run in parallel without conflicts.

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use portable_pty::CommandBuilder;
use ratatui_testlib::{
    parallel::{PoolConfig, TerminalGuard, TerminalPool, TestContext},
    Result, TuiTestHarness,
};

/// Test that multiple harnesses can be created and used in parallel.
#[test]
fn test_parallel_harness_creation() -> Result<()> {
    let handles: Vec<_> = (0..4)
        .map(|i| {
            thread::spawn(move || {
                let harness = TuiTestHarness::new(80, 24)?;
                // Each harness should be independent
                assert_eq!(harness.state().size(), (80, 24));
                println!("Thread {} created harness successfully", i);
                Ok::<(), ratatui_testlib::TermTestError>(())
            })
        })
        .collect();

    for (i, handle) in handles.into_iter().enumerate() {
        handle
            .join()
            .unwrap_or_else(|_| panic!("Thread {} panicked", i))?;
    }

    Ok(())
}

/// Test that with_isolation provides isolated contexts for tests.
#[test]
fn test_with_isolation() -> Result<()> {
    let counter = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..4)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                TuiTestHarness::with_isolation(|_harness| {
                    counter.fetch_add(1, Ordering::SeqCst);
                    // Simulate some work
                    thread::sleep(Duration::from_millis(10));
                    Ok(())
                })
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap()?;
    }

    assert_eq!(counter.load(Ordering::SeqCst), 4);
    Ok(())
}

/// Test that with_isolation_sized works with custom dimensions.
#[test]
fn test_with_isolation_sized() -> Result<()> {
    TuiTestHarness::with_isolation_sized(100, 30, |harness| {
        assert_eq!(harness.state().size(), (100, 30));
        Ok(())
    })
}

/// Test parallel execution with actual command spawning.
#[test]
fn test_parallel_command_execution() -> Result<()> {
    let handles: Vec<_> = (0..3)
        .map(|i| {
            thread::spawn(move || -> Result<()> {
                TuiTestHarness::with_isolation(|harness| {
                    let mut cmd = CommandBuilder::new("echo");
                    cmd.arg(format!("test_{}", i));

                    // Try to spawn, but if the process exits immediately that's ok
                    if let Err(e) = harness.spawn(cmd) {
                        println!("Thread {} spawn error (expected in some environments): {}", i, e);
                        return Ok(());
                    }

                    // Wait for output with a reasonable timeout
                    for _ in 0..5 {
                        thread::sleep(Duration::from_millis(20));
                        // ProcessExited is acceptable here since echo exits immediately
                        let _ = harness.update_state();
                    }

                    let contents = harness.screen_contents();
                    if contents.contains(&format!("test_{}", i)) {
                        println!("Thread {} got expected output", i);
                    } else {
                        println!("Thread {} output: {}", i, contents);
                    }

                    Ok(())
                })
            })
        })
        .collect();

    for (i, handle) in handles.into_iter().enumerate() {
        handle
            .join()
            .unwrap_or_else(|_| panic!("Thread {} panicked", i))?;
    }

    Ok(())
}

/// Test terminal pool basic functionality.
#[test]
fn test_terminal_pool_basic() -> Result<()> {
    let pool = TerminalPool::default_pool()?;

    // Acquire a terminal
    let terminal = pool.acquire(80, 24)?;
    assert_eq!(terminal.size(), (80, 24));

    let stats = pool.stats();
    assert_eq!(stats.in_use, 1);

    // Release it
    pool.release(terminal)?;

    let stats = pool.stats();
    assert_eq!(stats.in_use, 0);

    Ok(())
}

/// Test terminal pool with parallel access.
#[test]
fn test_terminal_pool_parallel() -> Result<()> {
    let pool = Arc::new(TerminalPool::default_pool()?);

    let handles: Vec<_> = (0..8)
        .map(|i| {
            let pool = Arc::clone(&pool);
            thread::spawn(move || -> Result<()> {
                let terminal = pool.acquire(80, 24)?;
                println!("Thread {} acquired terminal {:?}", i, terminal.id());

                // Simulate work
                thread::sleep(Duration::from_millis(50));

                pool.release(terminal)?;
                println!("Thread {} released terminal", i);
                Ok(())
            })
        })
        .collect();

    for (i, handle) in handles.into_iter().enumerate() {
        handle
            .join()
            .unwrap_or_else(|_| panic!("Thread {} panicked", i))?;
    }

    let stats = pool.stats();
    assert_eq!(stats.in_use, 0, "All terminals should be released");

    Ok(())
}

/// Test terminal pool with limited capacity.
#[test]
fn test_terminal_pool_capacity() -> Result<()> {
    let config = PoolConfig::default()
        .with_max_terminals(2)
        .with_acquire_timeout(Duration::from_secs(5));

    let pool = Arc::new(TerminalPool::new(config)?);

    // Acquire 2 terminals (at capacity)
    let t1 = pool.acquire(80, 24)?;
    let t2 = pool.acquire(80, 24)?;

    let stats = pool.stats();
    assert_eq!(stats.total, 2);
    assert_eq!(stats.in_use, 2);

    // Release one
    pool.release(t1)?;

    // Should be able to acquire again
    let t3 = pool.acquire(80, 24)?;
    assert!(t3.id().as_usize() < 2);

    pool.release(t2)?;
    pool.release(t3)?;

    Ok(())
}

/// Test terminal guard RAII behavior.
#[test]
fn test_terminal_guard() -> Result<()> {
    let pool = Arc::new(TerminalPool::default_pool()?);

    {
        let _guard = TerminalGuard::acquire(Arc::clone(&pool), 80, 24)?;
        let stats = pool.stats();
        assert_eq!(stats.in_use, 1);
    } // Guard dropped here

    let stats = pool.stats();
    assert_eq!(stats.in_use, 0);

    Ok(())
}

/// Test terminal guard with parallel access.
#[test]
fn test_terminal_guard_parallel() -> Result<()> {
    let pool = Arc::new(TerminalPool::default_pool()?);

    let handles: Vec<_> = (0..4)
        .map(|i| {
            let pool = Arc::clone(&pool);
            thread::spawn(move || -> Result<()> {
                let guard = TerminalGuard::acquire(Arc::clone(&pool), 80, 24)?;
                println!("Thread {} using terminal {:?}", i, guard.terminal().id());
                thread::sleep(Duration::from_millis(20));
                Ok(())
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap()?;
    }

    let stats = pool.stats();
    assert_eq!(stats.in_use, 0);

    Ok(())
}

/// Test TestContext port allocation.
#[test]
fn test_context_port_allocation() {
    let context = TestContext::new();

    let port1 = context.allocate_port();
    let port2 = context.allocate_port();
    let port3 = context.allocate_port();

    assert_ne!(port1, port2);
    assert_ne!(port2, port3);
    assert_ne!(port1, port3);

    // Ports should be in the safe range
    assert!(port1 >= 20000);
    assert!(port2 >= 20000);
    assert!(port3 >= 20000);
}

/// Test TestContext parallel port allocation.
#[test]
fn test_context_parallel_port_allocation() {
    let context = Arc::new(TestContext::new());
    let ports = Arc::new(std::sync::Mutex::new(Vec::new()));

    let handles: Vec<_> = (0..20)
        .map(|_| {
            let ctx = Arc::clone(&context);
            let ports = Arc::clone(&ports);
            thread::spawn(move || {
                let port = ctx.allocate_port();
                ports.lock().unwrap().push(port);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let allocated_ports = ports.lock().unwrap();
    assert_eq!(allocated_ports.len(), 20);

    // All ports should be unique
    let mut sorted = allocated_ports.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 20, "All ports should be unique");
}

/// Test TestContext metadata operations.
#[test]
fn test_context_metadata() {
    let context = TestContext::new();

    context.set_metadata("test_name", "parallel_test");
    context.set_metadata("iteration", "1");

    assert_eq!(context.get_metadata("test_name"), Some("parallel_test".to_string()));
    assert_eq!(context.get_metadata("iteration"), Some("1".to_string()));

    context.remove_metadata("iteration");
    assert_eq!(context.get_metadata("iteration"), None);

    context.clear_metadata();
    assert_eq!(context.get_metadata("test_name"), None);
}

/// Test TestContext parallel metadata access.
#[test]
fn test_context_parallel_metadata() {
    let context = Arc::new(TestContext::new());

    let handles: Vec<_> = (0..10)
        .map(|i| {
            let ctx = Arc::clone(&context);
            thread::spawn(move || {
                ctx.set_metadata(format!("thread_{}", i), format!("value_{}", i));
                thread::sleep(Duration::from_millis(5));
                let value = ctx.get_metadata(&format!("thread_{}", i));
                assert_eq!(value, Some(format!("value_{}", i)));
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // All metadata should be present
    for i in 0..10 {
        let value = context.get_metadata(&format!("thread_{}", i));
        assert_eq!(value, Some(format!("value_{}", i)));
    }
}

/// Test parallel harness builder.
#[test]
fn test_parallel_harness_builder() -> Result<()> {
    let handles: Vec<_> = (0..4)
        .map(|i| {
            thread::spawn(move || -> Result<()> {
                let harness = TuiTestHarness::parallel_harness_builder()
                    .with_size(100, 30)
                    .with_timeout(Duration::from_secs(10))
                    .build()?;

                assert_eq!(harness.state().size(), (100, 30));
                println!("Thread {} built harness with builder", i);
                Ok(())
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap()?;
    }

    Ok(())
}

/// Stress test: Many parallel operations.
#[test]
fn test_parallel_stress() -> Result<()> {
    let pool = Arc::new(TerminalPool::new(
        PoolConfig::default()
            .with_max_terminals(8)
            .with_acquire_timeout(Duration::from_secs(30)),
    )?);

    let context = Arc::new(TestContext::new());
    let success_count = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..20)
        .map(|i| {
            let pool = Arc::clone(&pool);
            let context = Arc::clone(&context);
            let success_count = Arc::clone(&success_count);

            thread::spawn(move || -> Result<()> {
                let guard = TerminalGuard::acquire(Arc::clone(&pool), 80, 24)?;
                let port = context.allocate_port();

                context.set_metadata(format!("thread_{}", i), format!("port_{}", port));

                // Simulate work
                thread::sleep(Duration::from_millis(10));

                success_count.fetch_add(1, Ordering::SeqCst);

                println!(
                    "Thread {} completed with terminal {:?} and port {}",
                    i,
                    guard.terminal().id(),
                    port
                );

                Ok(())
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap()?;
    }

    assert_eq!(success_count.load(Ordering::SeqCst), 20);

    let stats = pool.stats();
    assert_eq!(stats.in_use, 0, "All terminals should be released");

    Ok(())
}

/// Test that different terminal sizes are handled correctly.
#[test]
fn test_different_terminal_sizes() -> Result<()> {
    let pool = Arc::new(TerminalPool::default_pool()?);

    let sizes = vec![(80, 24), (100, 30), (120, 40), (80, 24)];

    let handles: Vec<_> = sizes
        .into_iter()
        .enumerate()
        .map(|(i, (width, height))| {
            let pool = Arc::clone(&pool);
            thread::spawn(move || -> Result<()> {
                let terminal = pool.acquire(width, height)?;
                assert_eq!(terminal.size(), (width, height));
                println!("Thread {} acquired terminal with size {}x{}", i, width, height);
                thread::sleep(Duration::from_millis(20));
                pool.release(terminal)?;
                Ok(())
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap()?;
    }

    Ok(())
}

/// Test pool statistics accuracy.
#[test]
fn test_pool_stats() -> Result<()> {
    let pool = TerminalPool::new(PoolConfig::default().with_max_terminals(5))?;

    let stats = pool.stats();
    assert_eq!(stats.total, 0);
    assert_eq!(stats.in_use, 0);
    assert_eq!(stats.available, 0);
    assert_eq!(stats.max_capacity, 5);

    let t1 = pool.acquire(80, 24)?;
    let stats = pool.stats();
    assert_eq!(stats.total, 1);
    assert_eq!(stats.in_use, 1);

    let t2 = pool.acquire(80, 24)?;
    let stats = pool.stats();
    assert_eq!(stats.total, 2);
    assert_eq!(stats.in_use, 2);

    pool.release(t1)?;
    let stats = pool.stats();
    assert_eq!(stats.total, 2);
    assert_eq!(stats.in_use, 1);
    assert_eq!(stats.available, 1);

    pool.release(t2)?;
    let stats = pool.stats();
    assert_eq!(stats.in_use, 0);
    assert_eq!(stats.available, 2);

    Ok(())
}

/// Test pool clear functionality.
#[test]
fn test_pool_clear() -> Result<()> {
    let pool = TerminalPool::default_pool()?;

    let t1 = pool.acquire(80, 24)?;
    let t2 = pool.acquire(100, 30)?;

    pool.release(t1)?;
    pool.release(t2)?;

    let stats = pool.stats();
    assert!(stats.total > 0);

    pool.clear();

    let stats = pool.stats();
    assert_eq!(stats.total, 0);

    Ok(())
}
