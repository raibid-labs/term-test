//! Performance profiling and benchmarking utilities for Bevy TUI testing.
//!
//! This module provides tools for measuring rendering performance, profiling
//! update cycles, and ensuring TUI applications meet FPS targets (e.g., 60 FPS).
//!
//! # Overview
//!
//! The benchmarking utilities capture timing statistics for Bevy update cycles:
//!
//! - **Frame timing**: Measure individual frame durations
//! - **Percentile statistics**: p50, p95, p99 timing analysis
//! - **FPS validation**: Assert minimum FPS requirements
//! - **Profile reporting**: Detailed breakdown of update cycle performance
//!
//! # Example: Basic Benchmarking
//!
//! ```rust,no_run
//! # #[cfg(feature = "bevy")]
//! # {
//! use ratatui_testlib::BevyTuiTestHarness;
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! let mut harness = BevyTuiTestHarness::new()?;
//!
//! // Benchmark 1000 frames
//! let results = harness.benchmark_rendering(1000)?;
//!
//! println!("Average frame time: {:.2}ms", results.avg_frame_time_ms);
//! println!("p95 frame time: {:.2}ms", results.p95_ms);
//! println!("p99 frame time: {:.2}ms", results.p99_ms);
//!
//! // Assert 60 FPS target (16.67ms per frame)
//! assert!(results.avg_frame_time_ms < 16.67);
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! # Example: FPS Assertions
//!
//! ```rust,no_run
//! # #[cfg(feature = "bevy")]
//! # {
//! use ratatui_testlib::BevyTuiTestHarness;
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! let mut harness = BevyTuiTestHarness::new()?;
//!
//! // Run benchmark and assert FPS in one call
//! harness.assert_fps(60.0, 1000)?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! # Example: Single Frame Profiling
//!
//! ```rust,no_run
//! # #[cfg(feature = "bevy")]
//! # {
//! use ratatui_testlib::BevyTuiTestHarness;
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! let mut harness = BevyTuiTestHarness::new()?;
//!
//! // Profile a single update cycle
//! let profile = harness.profile_update_cycle()?;
//!
//! println!("Frame duration: {:.2}ms", profile.duration_ms);
//! println!("FPS equivalent: {:.2}", profile.fps_equivalent);
//! # Ok(())
//! # }
//! # }
//! ```

use std::{
    mem::size_of,
    time::{Duration, Instant},
};

use crate::error::Result;

/// Results from a rendering benchmark.
///
/// Contains timing statistics collected over multiple frames, including
/// average frame time and percentile distributions.
///
/// # Fields
///
/// - `iterations`: Number of frames benchmarked
/// - `total_duration_ms`: Total elapsed time for all frames
/// - `avg_frame_time_ms`: Mean frame duration
/// - `min_frame_time_ms`: Fastest frame
/// - `max_frame_time_ms`: Slowest frame
/// - `p50_ms`: 50th percentile (median) frame time
/// - `p95_ms`: 95th percentile frame time
/// - `p99_ms`: 99th percentile frame time
/// - `fps_avg`: Average frames per second (1000.0 / avg_frame_time_ms)
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
/// let results = harness.benchmark_rendering(1000)?;
///
/// // Check that 95% of frames meet 60 FPS target
/// assert!(results.p95_ms < 16.67, "95% of frames should be under 16.67ms for 60 FPS");
///
/// // Check that worst-case (p99) is acceptable
/// assert!(results.p99_ms < 33.33, "99% of frames should be under 33.33ms for 30 FPS");
/// # Ok(())
/// # }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    /// Number of frames benchmarked
    pub iterations: usize,
    /// Total elapsed time for all frames in milliseconds
    pub total_duration_ms: f64,
    /// Average frame time in milliseconds
    pub avg_frame_time_ms: f64,
    /// Minimum frame time in milliseconds
    pub min_frame_time_ms: f64,
    /// Maximum frame time in milliseconds
    pub max_frame_time_ms: f64,
    /// 50th percentile (median) frame time in milliseconds
    pub p50_ms: f64,
    /// 95th percentile frame time in milliseconds
    pub p95_ms: f64,
    /// 99th percentile frame time in milliseconds
    pub p99_ms: f64,
    /// Average frames per second
    pub fps_avg: f64,
}

impl BenchmarkResults {
    /// Creates benchmark results from raw frame durations.
    ///
    /// This function calculates all statistics from the provided frame timings.
    ///
    /// # Arguments
    ///
    /// * `frame_times` - Vector of individual frame durations in milliseconds
    ///
    /// # Returns
    ///
    /// A `BenchmarkResults` struct with computed statistics.
    pub fn from_frame_times(frame_times: Vec<f64>) -> Self {
        let iterations = frame_times.len();
        let total_duration_ms: f64 = frame_times.iter().sum();
        let avg_frame_time_ms = if iterations > 0 {
            total_duration_ms / iterations as f64
        } else {
            0.0
        };

        let mut sorted_times = frame_times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min_frame_time_ms = sorted_times.first().copied().unwrap_or(0.0);
        let max_frame_time_ms = sorted_times.last().copied().unwrap_or(0.0);

        let p50_ms = percentile(&sorted_times, 50.0);
        let p95_ms = percentile(&sorted_times, 95.0);
        let p99_ms = percentile(&sorted_times, 99.0);

        let fps_avg = if avg_frame_time_ms > 0.0 {
            1000.0 / avg_frame_time_ms
        } else {
            0.0
        };

        Self {
            iterations,
            total_duration_ms,
            avg_frame_time_ms,
            min_frame_time_ms,
            max_frame_time_ms,
            p50_ms,
            p95_ms,
            p99_ms,
            fps_avg,
        }
    }

    /// Checks if the benchmark results meet a minimum FPS requirement.
    ///
    /// Uses the average FPS for the check.
    ///
    /// # Arguments
    ///
    /// * `min_fps` - Minimum required frames per second
    ///
    /// # Returns
    ///
    /// `true` if the average FPS meets or exceeds the requirement, `false` otherwise.
    pub fn meets_fps_requirement(&self, min_fps: f64) -> bool {
        self.fps_avg >= min_fps
    }

    /// Returns a formatted summary string for display or logging.
    ///
    /// # Example Output
    ///
    /// ```text
    /// Benchmark Results (1000 iterations):
    ///   Total Duration: 15234.56ms
    ///   Average FPS: 65.63
    ///   Frame Times:
    ///     Average: 15.23ms
    ///     Min: 12.34ms
    ///     Max: 25.67ms
    ///     p50 (median): 15.12ms
    ///     p95: 18.45ms
    ///     p99: 22.34ms
    /// ```
    pub fn summary(&self) -> String {
        format!(
            "Benchmark Results ({} iterations):\n\
             Total Duration: {:.2}ms\n\
             Average FPS: {:.2}\n\
             Frame Times:\n\
               Average: {:.2}ms\n\
               Min: {:.2}ms\n\
               Max: {:.2}ms\n\
               p50 (median): {:.2}ms\n\
               p95: {:.2}ms\n\
               p99: {:.2}ms",
            self.iterations,
            self.total_duration_ms,
            self.fps_avg,
            self.avg_frame_time_ms,
            self.min_frame_time_ms,
            self.max_frame_time_ms,
            self.p50_ms,
            self.p95_ms,
            self.p99_ms
        )
    }
}

/// Profile of a single frame update cycle.
///
/// Provides detailed timing information for a single Bevy update cycle.
///
/// # Fields
///
/// - `duration_ms`: Frame duration in milliseconds
/// - `fps_equivalent`: Equivalent FPS if sustained (1000.0 / duration_ms)
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
/// // Profile a single frame
/// let profile = harness.profile_update_cycle()?;
///
/// println!(
///     "Frame took {:.2}ms ({:.2} FPS equivalent)",
///     profile.duration_ms, profile.fps_equivalent
/// );
///
/// // Ensure single frame is under 60 FPS target
/// assert!(profile.duration_ms < 16.67);
/// # Ok(())
/// # }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct ProfileResults {
    /// Frame duration in milliseconds
    pub duration_ms: f64,
    /// Equivalent FPS if this frame time was sustained
    pub fps_equivalent: f64,
}

impl ProfileResults {
    /// Creates profile results from a duration.
    ///
    /// # Arguments
    ///
    /// * `duration` - Frame duration
    ///
    /// # Returns
    ///
    /// A `ProfileResults` struct with computed metrics.
    pub fn from_duration(duration: Duration) -> Self {
        let duration_ms = duration.as_secs_f64() * 1000.0;
        let fps_equivalent = if duration_ms > 0.0 {
            1000.0 / duration_ms
        } else {
            0.0
        };

        Self { duration_ms, fps_equivalent }
    }

    /// Returns a formatted summary string.
    ///
    /// # Example Output
    ///
    /// ```text
    /// Frame Profile:
    ///   Duration: 15.23ms
    ///   FPS Equivalent: 65.63
    /// ```
    pub fn summary(&self) -> String {
        format!(
            "Frame Profile:\n  Duration: {:.2}ms\n  FPS Equivalent: {:.2}",
            self.duration_ms, self.fps_equivalent
        )
    }
}

/// Results from memory profiling.
///
/// Provides estimated memory usage for a test harness, including the screen
/// buffer, Sixel regions, and recording buffer (if active).
///
/// # Memory Estimation
///
/// This struct provides size estimates, not precise heap measurements:
///
/// - **Screen buffer**: `width * height * size_of::<Cell>()`
/// - **Sixel regions**: Sum of all Sixel data sizes
/// - **Recording buffer**: Size of recorded events (if recording is active)
///
/// These are conservative estimates useful for detecting memory regressions
/// in tests, not exact heap profiling.
///
/// # Example
///
/// ```rust,no_run
/// # #[cfg(feature = "bevy")]
/// # {
/// use ratatui_testlib::TuiTestHarness;
///
/// # fn test() -> ratatui_testlib::Result<()> {
/// let harness = TuiTestHarness::new(80, 24)?;
///
/// // Get memory usage estimate
/// let memory = harness.memory_usage();
/// println!("Current: {} bytes", memory.current_bytes);
/// println!("Peak: {} bytes", memory.peak_bytes);
///
/// // Assert memory stays under limit (e.g., 1MB)
/// harness.assert_memory_under(1_000_000)?;
/// # Ok(())
/// # }
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryResults {
    /// Current estimated memory usage in bytes.
    pub current_bytes: usize,
    /// Peak estimated memory usage in bytes.
    ///
    /// Note: In the current implementation, this is the same as current_bytes
    /// since we don't track historical peak values. Future versions may add
    /// peak tracking across multiple measurements.
    pub peak_bytes: usize,
}

impl MemoryResults {
    /// Creates memory results from byte counts.
    ///
    /// # Arguments
    ///
    /// * `current_bytes` - Current memory usage estimate
    /// * `peak_bytes` - Peak memory usage estimate
    pub fn new(current_bytes: usize, peak_bytes: usize) -> Self {
        Self { current_bytes, peak_bytes }
    }

    /// Returns a formatted summary string.
    ///
    /// # Example Output
    ///
    /// ```text
    /// Memory Usage:
    ///   Current: 123,456 bytes (120.56 KB)
    ///   Peak: 150,000 bytes (146.48 KB)
    /// ```
    pub fn summary(&self) -> String {
        format!(
            "Memory Usage:\n  Current: {} bytes ({:.2} KB)\n  Peak: {} bytes ({:.2} KB)",
            self.current_bytes,
            self.current_bytes as f64 / 1024.0,
            self.peak_bytes,
            self.peak_bytes as f64 / 1024.0
        )
    }
}

/// Calculates a percentile from sorted data.
///
/// Uses linear interpolation for percentiles that fall between samples.
///
/// # Arguments
///
/// * `sorted_data` - Sorted slice of values
/// * `percentile` - Percentile to calculate (0.0 to 100.0)
///
/// # Returns
///
/// The value at the specified percentile, or 0.0 if data is empty.
fn percentile(sorted_data: &[f64], percentile: f64) -> f64 {
    if sorted_data.is_empty() {
        return 0.0;
    }

    let index = (percentile / 100.0) * (sorted_data.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        sorted_data[lower]
    } else {
        // Linear interpolation
        let weight = index - lower as f64;
        sorted_data[lower] * (1.0 - weight) + sorted_data[upper] * weight
    }
}

/// Trait for benchmarking support on test harnesses.
///
/// This trait provides performance profiling methods for Bevy test harnesses.
/// It's implemented by both [`BevyTuiTestHarness`] and [`HeadlessBevyRunner`].
pub trait BenchmarkableHarness {
    /// Runs a single Bevy update cycle.
    fn tick_once(&mut self) -> Result<()>;

    /// Benchmarks rendering performance over multiple iterations.
    ///
    /// This method runs the specified number of Bevy update cycles and
    /// collects timing statistics for each frame.
    ///
    /// # Arguments
    ///
    /// * `iterations` - Number of frames to benchmark
    ///
    /// # Returns
    ///
    /// A [`BenchmarkResults`] struct containing timing statistics.
    ///
    /// # Errors
    ///
    /// Returns an error if any update cycle fails.
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
    /// // Benchmark 1000 frames
    /// let results = harness.benchmark_rendering(1000)?;
    ///
    /// // Check 60 FPS target
    /// assert!(results.avg_frame_time_ms < 16.67);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    fn benchmark_rendering(&mut self, iterations: usize) -> Result<BenchmarkResults> {
        let mut frame_times = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let start = Instant::now();
            self.tick_once()?;
            let duration = start.elapsed();
            frame_times.push(duration.as_secs_f64() * 1000.0);
        }

        Ok(BenchmarkResults::from_frame_times(frame_times))
    }

    /// Profiles a single update cycle.
    ///
    /// This method measures the duration of a single Bevy update cycle,
    /// providing detailed timing information.
    ///
    /// # Returns
    ///
    /// A [`ProfileResults`] struct with timing metrics.
    ///
    /// # Errors
    ///
    /// Returns an error if the update cycle fails.
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
    /// let profile = harness.profile_update_cycle()?;
    /// println!("Frame duration: {:.2}ms", profile.duration_ms);
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    fn profile_update_cycle(&mut self) -> Result<ProfileResults> {
        let start = Instant::now();
        self.tick_once()?;
        let duration = start.elapsed();

        Ok(ProfileResults::from_duration(duration))
    }

    /// Asserts that the average FPS meets a minimum requirement.
    ///
    /// This is a convenience method that runs a benchmark and checks the
    /// average FPS in one call.
    ///
    /// # Arguments
    ///
    /// * `min_fps` - Minimum required frames per second
    /// * `iterations` - Number of frames to benchmark (default: 1000)
    ///
    /// # Returns
    ///
    /// The benchmark results if the FPS requirement is met.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any update cycle fails
    /// - The average FPS is below the minimum requirement
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
    /// // Assert 60 FPS over 1000 frames
    /// harness.assert_fps(60.0, 1000)?;
    /// # Ok(())
    /// # }
    /// # }
    /// ```
    fn assert_fps(&mut self, min_fps: f64, iterations: usize) -> Result<BenchmarkResults> {
        let results = self.benchmark_rendering(iterations)?;

        if !results.meets_fps_requirement(min_fps) {
            return Err(crate::error::TermTestError::Bevy(format!(
                "FPS requirement not met: average {:.2} FPS < minimum {:.2} FPS\n{}",
                results.fps_avg,
                min_fps,
                results.summary()
            )));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_calculation() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

        assert_eq!(percentile(&data, 0.0), 1.0);
        assert_eq!(percentile(&data, 50.0), 5.5);
        assert_eq!(percentile(&data, 100.0), 10.0);
    }

    #[test]
    fn test_percentile_empty() {
        let data: Vec<f64> = vec![];
        assert_eq!(percentile(&data, 50.0), 0.0);
    }

    #[test]
    fn test_benchmark_results_from_frame_times() {
        let frame_times = vec![10.0, 15.0, 20.0, 25.0, 30.0];
        let results = BenchmarkResults::from_frame_times(frame_times);

        assert_eq!(results.iterations, 5);
        assert_eq!(results.total_duration_ms, 100.0);
        assert_eq!(results.avg_frame_time_ms, 20.0);
        assert_eq!(results.min_frame_time_ms, 10.0);
        assert_eq!(results.max_frame_time_ms, 30.0);
        assert_eq!(results.p50_ms, 20.0);
        assert_eq!(results.fps_avg, 50.0); // 1000ms / 20ms = 50 FPS
    }

    #[test]
    fn test_benchmark_results_fps_requirement() {
        let frame_times = vec![10.0, 10.0, 10.0]; // 100 FPS
        let results = BenchmarkResults::from_frame_times(frame_times);

        assert!(results.meets_fps_requirement(60.0));
        assert!(results.meets_fps_requirement(100.0));
        assert!(!results.meets_fps_requirement(120.0));
    }

    #[test]
    fn test_profile_results_from_duration() {
        let duration = Duration::from_millis(16); // ~60 FPS
        let profile = ProfileResults::from_duration(duration);

        assert!((profile.duration_ms - 16.0).abs() < 0.1);
        assert!((profile.fps_equivalent - 62.5).abs() < 0.1);
    }

    #[test]
    fn test_benchmark_results_summary() {
        let frame_times = vec![10.0, 15.0, 20.0];
        let results = BenchmarkResults::from_frame_times(frame_times);
        let summary = results.summary();

        assert!(summary.contains("3 iterations"));
        assert!(summary.contains("Average FPS"));
        assert!(summary.contains("p50"));
        assert!(summary.contains("p95"));
        assert!(summary.contains("p99"));
    }

    #[test]
    fn test_profile_results_summary() {
        let duration = Duration::from_millis(20);
        let profile = ProfileResults::from_duration(duration);
        let summary = profile.summary();

        assert!(summary.contains("Frame Profile"));
        assert!(summary.contains("Duration"));
        assert!(summary.contains("FPS Equivalent"));
    }

    #[test]
    fn test_percentile_interpolation() {
        let data = vec![1.0, 2.0, 3.0, 4.0];

        // p25 should be 1.75 (between 1.0 and 2.0)
        let p25 = percentile(&data, 25.0);
        assert!((p25 - 1.75).abs() < 0.01);
    }

    #[test]
    fn test_benchmark_results_60fps_target() {
        // Simulate 60 FPS target (16.67ms per frame)
        let frame_times = vec![15.0, 16.0, 17.0, 16.5, 16.2];
        let results = BenchmarkResults::from_frame_times(frame_times);

        assert!(results.meets_fps_requirement(60.0));
        assert!(results.avg_frame_time_ms < 16.67);
    }

    #[test]
    fn test_memory_results_new() {
        let memory = MemoryResults::new(1024, 2048);

        assert_eq!(memory.current_bytes, 1024);
        assert_eq!(memory.peak_bytes, 2048);
    }

    #[test]
    fn test_memory_results_equality() {
        let memory1 = MemoryResults::new(1024, 1024);
        let memory2 = MemoryResults::new(1024, 1024);
        let memory3 = MemoryResults::new(2048, 2048);

        assert_eq!(memory1, memory2);
        assert_ne!(memory1, memory3);
    }

    #[test]
    fn test_memory_results_summary() {
        let memory = MemoryResults::new(123_456, 150_000);
        let summary = memory.summary();

        assert!(summary.contains("Memory Usage"));
        assert!(summary.contains("123456")); // Numbers are not comma-formatted
        assert!(summary.contains("150000"));
        assert!(summary.contains("KB"));
    }

    #[test]
    fn test_memory_results_zero() {
        let memory = MemoryResults::new(0, 0);
        let summary = memory.summary();

        assert_eq!(memory.current_bytes, 0);
        assert_eq!(memory.peak_bytes, 0);
        assert!(summary.contains("0 bytes"));
    }

    #[test]
    fn test_memory_results_large_values() {
        // Test with ~10 MB
        let memory = MemoryResults::new(10_485_760, 10_485_760);

        assert_eq!(memory.current_bytes, 10_485_760);
        assert_eq!(memory.peak_bytes, 10_485_760);

        let summary = memory.summary();
        // Should show ~10240 KB
        assert!(summary.contains("10240.00 KB"));
    }
}
