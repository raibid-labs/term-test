//! Input-to-render latency measurement and timing utilities.
//!
//! This module provides comprehensive timing infrastructure for measuring and asserting
//! latency budgets in TUI applications. It enables testing that input→render latency
//! stays within acceptable bounds, which is critical for responsive user interfaces.
//!
//! # Overview
//!
//! The timing infrastructure consists of several components:
//!
//! - [`TimingRecorder`]: Records timestamped events for latency analysis
//! - [`TimingHooks`]: Trait for integrating timing into test harnesses
//! - [`LatencyProfile`]: Tracks input→render→frame latency stages
//! - [`LatencyStats`]: Statistical analysis of latency measurements
//!
//! # Example: Basic Timing
//!
//! ```rust
//! use std::time::Duration;
//!
//! use ratatui_testlib::timing::TimingRecorder;
//!
//! let mut recorder = TimingRecorder::new();
//!
//! // Record events
//! recorder.record_event("input_sent");
//! std::thread::sleep(Duration::from_millis(5));
//! recorder.record_event("render_start");
//! std::thread::sleep(Duration::from_millis(10));
//! recorder.record_event("render_end");
//!
//! // Measure latency
//! let latency = recorder
//!     .measure_latency("input_sent", "render_end")
//!     .unwrap();
//! assert!(latency < Duration::from_millis(20));
//! ```
//!
//! # Example: Input→Render Latency Profiling
//!
//! ```rust
//! use std::time::Duration;
//!
//! use ratatui_testlib::timing::LatencyProfile;
//!
//! let mut profile = LatencyProfile::new();
//!
//! // Track input event
//! profile.mark_input();
//! std::thread::sleep(Duration::from_millis(3));
//!
//! // Track render stages
//! profile.mark_render_start();
//! std::thread::sleep(Duration::from_millis(8));
//! profile.mark_render_end();
//! std::thread::sleep(Duration::from_millis(2));
//! profile.mark_frame_ready();
//!
//! // Analyze latency
//! assert!(profile.input_to_render().unwrap() < Duration::from_millis(15));
//! assert!(profile.total_latency().unwrap() < Duration::from_millis(20));
//! ```
//!
//! # Example: Assertion Helpers
//!
//! ```rust,no_run
//! use std::time::Duration;
//!
//! use ratatui_testlib::timing::TimingRecorder;
//!
//! # fn test() -> ratatui_testlib::Result<()> {
//! let mut recorder = TimingRecorder::new();
//! recorder.record_event("start");
//! // ... do work ...
//! recorder.record_event("end");
//!
//! // Assert latency is within budget
//! recorder.assert_latency_within("start", "end", Duration::from_millis(16))?;
//! # Ok(())
//! # }
//! ```

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

#[cfg(feature = "snapshot-insta")]
use serde::{Deserialize, Serialize};

use crate::error::{Result, TermTestError};

/// Records timestamped events for latency measurement.
///
/// This struct maintains a timeline of named events with their timestamps,
/// enabling measurement of time between any two recorded events.
///
/// # Features
///
/// - Record arbitrary named events with timestamps
/// - Measure latency between any two events
/// - Calculate statistics across multiple samples
/// - Serialize for snapshot testing (with `snapshot-insta` feature)
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use ratatui_testlib::timing::TimingRecorder;
///
/// let mut recorder = TimingRecorder::new();
///
/// recorder.record_event("input");
/// std::thread::sleep(Duration::from_millis(5));
/// recorder.record_event("render");
///
/// let latency = recorder.measure_latency("input", "render").unwrap();
/// assert!(latency >= Duration::from_millis(5));
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "snapshot-insta", derive(Serialize, Deserialize))]
pub struct TimingRecorder {
    /// Start time for relative timestamps (not serialized, recreated on deserialization)
    #[cfg_attr(feature = "snapshot-insta", serde(skip, default = "Instant::now"))]
    start_time: Instant,
    /// Recorded events with their timestamps
    events: HashMap<String, Vec<Duration>>,
    /// Event recording order for iteration
    event_order: Vec<String>,
}

impl TimingRecorder {
    /// Creates a new timing recorder.
    ///
    /// The recorder's start time is set to the current instant.
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            events: HashMap::new(),
            event_order: Vec::new(),
        }
    }

    /// Resets the recorder, clearing all recorded events.
    ///
    /// The start time is reset to the current instant.
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.events.clear();
        self.event_order.clear();
    }

    /// Records a timestamp for the named event.
    ///
    /// Multiple recordings of the same event name create multiple samples,
    /// which can be used for statistical analysis.
    ///
    /// # Arguments
    ///
    /// * `event_name` - Name of the event to record
    ///
    /// # Example
    ///
    /// ```rust
    /// use ratatui_testlib::timing::TimingRecorder;
    ///
    /// let mut recorder = TimingRecorder::new();
    /// recorder.record_event("frame_start");
    /// // ... rendering work ...
    /// recorder.record_event("frame_end");
    /// ```
    pub fn record_event(&mut self, event_name: &str) {
        let elapsed = self.start_time.elapsed();
        let name = event_name.to_string();

        if !self.events.contains_key(&name) {
            self.event_order.push(name.clone());
        }

        self.events
            .entry(name)
            .or_insert_with(Vec::new)
            .push(elapsed);
    }

    /// Measures the time between two recorded events.
    ///
    /// If multiple samples exist for either event, the most recent sample is used.
    ///
    /// # Arguments
    ///
    /// * `start_event` - Name of the starting event
    /// * `end_event` - Name of the ending event
    ///
    /// # Returns
    ///
    /// `Some(Duration)` if both events exist, `None` otherwise.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    ///
    /// use ratatui_testlib::timing::TimingRecorder;
    ///
    /// let mut recorder = TimingRecorder::new();
    /// recorder.record_event("input");
    /// std::thread::sleep(Duration::from_millis(10));
    /// recorder.record_event("output");
    ///
    /// let latency = recorder.measure_latency("input", "output").unwrap();
    /// assert!(latency >= Duration::from_millis(10));
    /// ```
    pub fn measure_latency(&self, start_event: &str, end_event: &str) -> Option<Duration> {
        let start_time = self.events.get(start_event)?.last()?;
        let end_time = self.events.get(end_event)?.last()?;

        if end_time >= start_time {
            Some(*end_time - *start_time)
        } else {
            None
        }
    }

    /// Gets all timestamps for a specific event.
    ///
    /// # Arguments
    ///
    /// * `event_name` - Name of the event to query
    ///
    /// # Returns
    ///
    /// A slice of all recorded timestamps for this event, or an empty slice if the event doesn't
    /// exist.
    pub fn get_event_times(&self, event_name: &str) -> &[Duration] {
        self.events
            .get(event_name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Calculates statistics for latency between two events across multiple samples.
    ///
    /// This method pairs up samples in order (first start with first end, etc.)
    /// and computes statistics across all pairs.
    ///
    /// # Arguments
    ///
    /// * `start_event` - Name of the starting event
    /// * `end_event` - Name of the ending event
    ///
    /// # Returns
    ///
    /// `Some(LatencyStats)` if both events have samples, `None` otherwise.
    pub fn latency_stats(&self, start_event: &str, end_event: &str) -> Option<LatencyStats> {
        let start_times = self.events.get(start_event)?;
        let end_times = self.events.get(end_event)?;

        if start_times.is_empty() || end_times.is_empty() {
            return None;
        }

        let samples: Vec<Duration> = start_times
            .iter()
            .zip(end_times.iter())
            .filter_map(|(start, end)| {
                if end >= start {
                    Some(*end - *start)
                } else {
                    None
                }
            })
            .collect();

        if samples.is_empty() {
            return None;
        }

        Some(LatencyStats::from_samples(samples))
    }

    /// Asserts that latency between two events is within a budget.
    ///
    /// Uses the most recent sample of each event for the measurement.
    ///
    /// # Arguments
    ///
    /// * `start_event` - Name of the starting event
    /// * `end_event` - Name of the ending event
    /// * `budget` - Maximum allowed latency
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Either event doesn't exist
    /// - The measured latency exceeds the budget
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use std::time::Duration;
    ///
    /// use ratatui_testlib::timing::TimingRecorder;
    ///
    /// # fn test() -> ratatui_testlib::Result<()> {
    /// let mut recorder = TimingRecorder::new();
    /// recorder.record_event("start");
    /// // ... work ...
    /// recorder.record_event("end");
    ///
    /// // Assert completes within 16.67ms (60 FPS)
    /// recorder.assert_latency_within("start", "end", Duration::from_micros(16670))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn assert_latency_within(
        &self,
        start_event: &str,
        end_event: &str,
        budget: Duration,
    ) -> Result<()> {
        let latency = self
            .measure_latency(start_event, end_event)
            .ok_or_else(|| {
                TermTestError::Timing(format!(
                    "Cannot measure latency: missing event '{}' or '{}'",
                    start_event, end_event
                ))
            })?;

        if latency > budget {
            return Err(TermTestError::Timing(format!(
                "Latency budget exceeded: {} -> {} took {:.3}ms, budget was {:.3}ms",
                start_event,
                end_event,
                latency.as_secs_f64() * 1000.0,
                budget.as_secs_f64() * 1000.0
            )));
        }

        Ok(())
    }

    /// Returns an iterator over all recorded event names in recording order.
    pub fn event_names(&self) -> impl Iterator<Item = &str> {
        self.event_order.iter().map(|s| s.as_str())
    }

    /// Returns the number of unique event names recorded.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Returns the total number of samples across all events.
    pub fn sample_count(&self) -> usize {
        self.events.values().map(|v| v.len()).sum()
    }
}

impl Default for TimingRecorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistical analysis of latency measurements.
///
/// Provides min, max, mean, and percentile statistics for a collection
/// of latency samples.
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use ratatui_testlib::timing::LatencyStats;
///
/// let samples = vec![
///     Duration::from_millis(10),
///     Duration::from_millis(15),
///     Duration::from_millis(12),
///     Duration::from_millis(20),
///     Duration::from_millis(11),
/// ];
///
/// let stats = LatencyStats::from_samples(samples);
/// println!("Mean: {:.2}ms", stats.mean.as_secs_f64() * 1000.0);
/// println!("p95: {:.2}ms", stats.p95.as_secs_f64() * 1000.0);
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "snapshot-insta", derive(Serialize, Deserialize))]
pub struct LatencyStats {
    /// Number of samples
    pub count: usize,
    /// Minimum latency
    pub min: Duration,
    /// Maximum latency
    pub max: Duration,
    /// Mean (average) latency
    pub mean: Duration,
    /// Median (50th percentile) latency
    pub median: Duration,
    /// 95th percentile latency
    pub p95: Duration,
    /// 99th percentile latency
    pub p99: Duration,
}

impl LatencyStats {
    /// Creates latency statistics from a collection of samples.
    ///
    /// # Arguments
    ///
    /// * `samples` - Vector of latency measurements
    ///
    /// # Panics
    ///
    /// Panics if `samples` is empty.
    pub fn from_samples(mut samples: Vec<Duration>) -> Self {
        assert!(!samples.is_empty(), "Cannot compute stats from empty samples");

        samples.sort();
        let count = samples.len();

        let min = samples[0];
        let max = samples[count - 1];

        let total: Duration = samples.iter().sum();
        let mean = total / count as u32;

        let median = percentile(&samples, 50.0);
        let p95 = percentile(&samples, 95.0);
        let p99 = percentile(&samples, 99.0);

        Self { count, min, max, mean, median, p95, p99 }
    }

    /// Returns a formatted summary string.
    ///
    /// # Example Output
    ///
    /// ```text
    /// Latency Statistics (100 samples):
    ///   Min: 8.23ms
    ///   Max: 25.67ms
    ///   Mean: 15.34ms
    ///   Median: 15.12ms
    ///   p95: 22.45ms
    ///   p99: 24.89ms
    /// ```
    pub fn summary(&self) -> String {
        format!(
            "Latency Statistics ({} samples):\n\
             Min: {:.2}ms\n\
             Max: {:.2}ms\n\
             Mean: {:.2}ms\n\
             Median: {:.2}ms\n\
             p95: {:.2}ms\n\
             p99: {:.2}ms",
            self.count,
            self.min.as_secs_f64() * 1000.0,
            self.max.as_secs_f64() * 1000.0,
            self.mean.as_secs_f64() * 1000.0,
            self.median.as_secs_f64() * 1000.0,
            self.p95.as_secs_f64() * 1000.0,
            self.p99.as_secs_f64() * 1000.0
        )
    }
}

/// Calculates a percentile from sorted duration data.
///
/// Uses linear interpolation for percentiles that fall between samples.
///
/// # Arguments
///
/// * `sorted_data` - Sorted slice of durations
/// * `percentile` - Percentile to calculate (0.0 to 100.0)
///
/// # Returns
///
/// The duration at the specified percentile.
///
/// # Panics
///
/// Panics if `sorted_data` is empty.
fn percentile(sorted_data: &[Duration], percentile: f64) -> Duration {
    assert!(!sorted_data.is_empty(), "Cannot compute percentile from empty data");

    let index = (percentile / 100.0) * (sorted_data.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;

    if lower == upper {
        sorted_data[lower]
    } else {
        // Linear interpolation
        let weight = index - lower as f64;
        let lower_dur = sorted_data[lower].as_secs_f64();
        let upper_dur = sorted_data[upper].as_secs_f64();
        let interpolated = lower_dur * (1.0 - weight) + upper_dur * weight;
        Duration::from_secs_f64(interpolated)
    }
}

/// Profile for tracking input→render→frame latency stages.
///
/// This struct tracks the complete pipeline from user input to visible frame,
/// with timestamps for each stage of the rendering process.
///
/// # Latency Stages
///
/// 1. **Input**: User input received (key press, mouse event, etc.)
/// 2. **Render Start**: Application begins processing input and updating state
/// 3. **Render End**: Frame rendering completed
/// 4. **Frame Ready**: Frame fully prepared and ready for display
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use ratatui_testlib::timing::LatencyProfile;
///
/// let mut profile = LatencyProfile::new();
///
/// // User presses a key
/// profile.mark_input();
///
/// // Application starts processing
/// profile.mark_render_start();
///
/// // Frame rendering completes
/// profile.mark_render_end();
///
/// // Frame is ready for display
/// profile.mark_frame_ready();
///
/// // Analyze latency
/// let input_to_render = profile.input_to_render().unwrap();
/// let total = profile.total_latency().unwrap();
///
/// println!("Input→Render: {:.2}ms", input_to_render.as_secs_f64() * 1000.0);
/// println!("Total Latency: {:.2}ms", total.as_secs_f64() * 1000.0);
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "snapshot-insta", derive(Serialize, Deserialize))]
pub struct LatencyProfile {
    /// Timestamp when input was received (not serialized)
    #[cfg_attr(feature = "snapshot-insta", serde(skip))]
    pub input_timestamp: Option<Instant>,
    /// Timestamp when rendering started (not serialized)
    #[cfg_attr(feature = "snapshot-insta", serde(skip))]
    pub render_start: Option<Instant>,
    /// Timestamp when rendering completed (not serialized)
    #[cfg_attr(feature = "snapshot-insta", serde(skip))]
    pub render_end: Option<Instant>,
    /// Timestamp when frame is ready for display (not serialized)
    #[cfg_attr(feature = "snapshot-insta", serde(skip))]
    pub frame_ready: Option<Instant>,
}

impl LatencyProfile {
    /// Creates a new latency profile with no recorded timestamps.
    pub fn new() -> Self {
        Self {
            input_timestamp: None,
            render_start: None,
            render_end: None,
            frame_ready: None,
        }
    }

    /// Resets all timestamps.
    pub fn reset(&mut self) {
        self.input_timestamp = None;
        self.render_start = None;
        self.render_end = None;
        self.frame_ready = None;
    }

    /// Records the input timestamp.
    pub fn mark_input(&mut self) {
        self.input_timestamp = Some(Instant::now());
    }

    /// Records the render start timestamp.
    pub fn mark_render_start(&mut self) {
        self.render_start = Some(Instant::now());
    }

    /// Records the render end timestamp.
    pub fn mark_render_end(&mut self) {
        self.render_end = Some(Instant::now());
    }

    /// Records the frame ready timestamp.
    pub fn mark_frame_ready(&mut self) {
        self.frame_ready = Some(Instant::now());
    }

    /// Measures latency from input to render completion.
    ///
    /// # Returns
    ///
    /// `Some(Duration)` if both timestamps are recorded, `None` otherwise.
    pub fn input_to_render(&self) -> Option<Duration> {
        let input = self.input_timestamp?;
        let render = self.render_end?;
        Some(render.duration_since(input))
    }

    /// Measures total latency from input to frame ready.
    ///
    /// # Returns
    ///
    /// `Some(Duration)` if both timestamps are recorded, `None` otherwise.
    pub fn total_latency(&self) -> Option<Duration> {
        let input = self.input_timestamp?;
        let ready = self.frame_ready?;
        Some(ready.duration_since(input))
    }

    /// Measures rendering time (render start to render end).
    ///
    /// # Returns
    ///
    /// `Some(Duration)` if both timestamps are recorded, `None` otherwise.
    pub fn render_duration(&self) -> Option<Duration> {
        let start = self.render_start?;
        let end = self.render_end?;
        Some(end.duration_since(start))
    }

    /// Measures post-render time (render end to frame ready).
    ///
    /// # Returns
    ///
    /// `Some(Duration)` if both timestamps are recorded, `None` otherwise.
    pub fn post_render_duration(&self) -> Option<Duration> {
        let end = self.render_end?;
        let ready = self.frame_ready?;
        Some(ready.duration_since(end))
    }

    /// Returns a formatted summary string.
    ///
    /// # Example Output
    ///
    /// ```text
    /// Latency Profile:
    ///   Input → Render: 12.34ms
    ///   Render Duration: 8.56ms
    ///   Post-Render: 2.10ms
    ///   Total Latency: 14.44ms
    /// ```
    pub fn summary(&self) -> String {
        let input_render = self
            .input_to_render()
            .map(|d| format!("{:.2}ms", d.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "N/A".to_string());

        let render = self
            .render_duration()
            .map(|d| format!("{:.2}ms", d.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "N/A".to_string());

        let post_render = self
            .post_render_duration()
            .map(|d| format!("{:.2}ms", d.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "N/A".to_string());

        let total = self
            .total_latency()
            .map(|d| format!("{:.2}ms", d.as_secs_f64() * 1000.0))
            .unwrap_or_else(|| "N/A".to_string());

        format!(
            "Latency Profile:\n\
             Input → Render: {}\n\
             Render Duration: {}\n\
             Post-Render: {}\n\
             Total Latency: {}",
            input_render, render, post_render, total
        )
    }
}

impl Default for LatencyProfile {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for test harnesses that support timing hooks.
///
/// This trait provides methods for recording and measuring latency in test
/// harnesses, enabling performance testing and latency budget validation.
///
/// # Example Implementation
///
/// ```rust,ignore
/// impl TimingHooks for TuiTestHarness {
///     fn record_event(&mut self, event_name: &str) {
///         self.timing_recorder.record_event(event_name);
///     }
///
///     fn measure_latency(&self, start_event: &str, end_event: &str) -> Option<Duration> {
///         self.timing_recorder.measure_latency(start_event, end_event)
///     }
///
///     fn get_timings(&self) -> &TimingRecorder {
///         &self.timing_recorder
///     }
///
///     fn assert_latency_within(&self, start: &str, end: &str, budget: Duration) -> Result<()> {
///         self.timing_recorder.assert_latency_within(start, end, budget)
///     }
/// }
/// ```
pub trait TimingHooks {
    /// Records a timestamp for the named event.
    ///
    /// # Arguments
    ///
    /// * `event_name` - Name of the event to record
    fn record_event(&mut self, event_name: &str);

    /// Measures time between two recorded events.
    ///
    /// # Arguments
    ///
    /// * `start_event` - Name of the starting event
    /// * `end_event` - Name of the ending event
    ///
    /// # Returns
    ///
    /// `Some(Duration)` if both events exist, `None` otherwise.
    fn measure_latency(&self, start_event: &str, end_event: &str) -> Option<Duration>;

    /// Gets the timing recorder for direct access.
    fn get_timings(&self) -> &TimingRecorder;

    /// Asserts that latency between two events is within a budget.
    ///
    /// # Arguments
    ///
    /// * `start_event` - Name of the starting event
    /// * `end_event` - Name of the ending event
    /// * `budget` - Maximum allowed latency
    ///
    /// # Errors
    ///
    /// Returns an error if either event doesn't exist or if the latency exceeds the budget.
    fn assert_latency_within(
        &self,
        start_event: &str,
        end_event: &str,
        budget: Duration,
    ) -> Result<()>;
}

/// Helper function to calculate frame time budget from target FPS.
///
/// # Arguments
///
/// * `fps_target` - Target frames per second
///
/// # Returns
///
/// The maximum frame time allowed to achieve the target FPS.
///
/// # Example
///
/// ```rust
/// use std::time::Duration;
///
/// use ratatui_testlib::timing::fps_to_frame_budget;
///
/// // 60 FPS = 16.67ms per frame
/// let budget = fps_to_frame_budget(60.0);
/// assert!((budget.as_secs_f64() - 0.016666).abs() < 0.0001);
/// ```
pub fn fps_to_frame_budget(fps_target: f64) -> Duration {
    assert!(fps_target > 0.0, "FPS target must be positive");
    Duration::from_secs_f64(1.0 / fps_target)
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn test_timing_recorder_basic() {
        let mut recorder = TimingRecorder::new();

        recorder.record_event("start");
        thread::sleep(Duration::from_millis(10));
        recorder.record_event("end");

        let latency = recorder.measure_latency("start", "end");
        assert!(latency.is_some());
        assert!(latency.unwrap() >= Duration::from_millis(10));
    }

    #[test]
    fn test_timing_recorder_multiple_samples() {
        let mut recorder = TimingRecorder::new();

        recorder.record_event("event1");
        thread::sleep(Duration::from_millis(5));
        recorder.record_event("event1");

        let times = recorder.get_event_times("event1");
        assert_eq!(times.len(), 2);
        assert!(times[1] > times[0]);
    }

    #[test]
    fn test_timing_recorder_missing_event() {
        let recorder = TimingRecorder::new();
        let latency = recorder.measure_latency("start", "end");
        assert!(latency.is_none());
    }

    #[test]
    fn test_timing_recorder_reset() {
        let mut recorder = TimingRecorder::new();
        recorder.record_event("event1");
        recorder.reset();

        assert_eq!(recorder.event_count(), 0);
        assert_eq!(recorder.sample_count(), 0);
    }

    #[test]
    fn test_timing_recorder_assert_latency_within() {
        let mut recorder = TimingRecorder::new();
        recorder.record_event("start");
        thread::sleep(Duration::from_millis(5));
        recorder.record_event("end");

        // Should pass
        assert!(recorder
            .assert_latency_within("start", "end", Duration::from_millis(100))
            .is_ok());

        // Should fail
        assert!(recorder
            .assert_latency_within("start", "end", Duration::from_millis(1))
            .is_err());
    }

    #[test]
    fn test_latency_stats() {
        let samples = vec![
            Duration::from_millis(10),
            Duration::from_millis(15),
            Duration::from_millis(12),
            Duration::from_millis(20),
            Duration::from_millis(11),
        ];

        let stats = LatencyStats::from_samples(samples);

        assert_eq!(stats.count, 5);
        assert_eq!(stats.min, Duration::from_millis(10));
        assert_eq!(stats.max, Duration::from_millis(20));
        assert!(stats.mean >= Duration::from_millis(13));
        assert!(stats.mean <= Duration::from_millis(14));
    }

    #[test]
    fn test_latency_stats_summary() {
        let samples = vec![Duration::from_millis(10), Duration::from_millis(20)];
        let stats = LatencyStats::from_samples(samples);
        let summary = stats.summary();

        assert!(summary.contains("2 samples"));
        assert!(summary.contains("Min:"));
        assert!(summary.contains("Max:"));
    }

    #[test]
    fn test_latency_profile() {
        let mut profile = LatencyProfile::new();

        profile.mark_input();
        thread::sleep(Duration::from_millis(5));
        profile.mark_render_start();
        thread::sleep(Duration::from_millis(5));
        profile.mark_render_end();
        thread::sleep(Duration::from_millis(5));
        profile.mark_frame_ready();

        assert!(profile.input_to_render().is_some());
        assert!(profile.total_latency().is_some());
        assert!(profile.render_duration().is_some());
        assert!(profile.post_render_duration().is_some());

        let total = profile.total_latency().unwrap();
        assert!(total >= Duration::from_millis(15));
    }

    #[test]
    fn test_latency_profile_partial() {
        let mut profile = LatencyProfile::new();

        profile.mark_input();
        profile.mark_render_end();

        assert!(profile.input_to_render().is_some());
        assert!(profile.total_latency().is_none()); // No frame_ready
        assert!(profile.render_duration().is_none()); // No render_start
    }

    #[test]
    fn test_latency_profile_reset() {
        let mut profile = LatencyProfile::new();
        profile.mark_input();
        profile.mark_render_end();

        profile.reset();

        assert!(profile.input_to_render().is_none());
        assert!(profile.total_latency().is_none());
    }

    #[test]
    fn test_latency_profile_summary() {
        let mut profile = LatencyProfile::new();
        profile.mark_input();
        profile.mark_render_end();

        let summary = profile.summary();
        assert!(summary.contains("Latency Profile"));
        assert!(summary.contains("Input → Render"));
    }

    #[test]
    fn test_fps_to_frame_budget() {
        // 60 FPS = 16.67ms
        let budget = fps_to_frame_budget(60.0);
        assert!((budget.as_secs_f64() * 1000.0 - 16.666).abs() < 0.1);

        // 30 FPS = 33.33ms
        let budget = fps_to_frame_budget(30.0);
        assert!((budget.as_secs_f64() * 1000.0 - 33.333).abs() < 0.1);

        // 120 FPS = 8.33ms
        let budget = fps_to_frame_budget(120.0);
        assert!((budget.as_secs_f64() * 1000.0 - 8.333).abs() < 0.1);
    }

    #[test]
    fn test_percentile_calculation() {
        let data = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(50),
        ];

        let p50 = percentile(&data, 50.0);
        assert_eq!(p50, Duration::from_millis(30));

        let p0 = percentile(&data, 0.0);
        assert_eq!(p0, Duration::from_millis(10));

        let p100 = percentile(&data, 100.0);
        assert_eq!(p100, Duration::from_millis(50));
    }

    #[test]
    fn test_percentile_interpolation() {
        let data = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
        ];

        // p25 should be between 10 and 20 (approximately 17.5ms based on linear interpolation)
        let p25 = percentile(&data, 25.0);
        assert!(p25 >= Duration::from_millis(15));
        assert!(p25 <= Duration::from_millis(18));
    }

    #[test]
    fn test_timing_recorder_latency_stats() {
        let mut recorder = TimingRecorder::new();

        // Record multiple input→render cycles
        for _ in 0..5 {
            recorder.record_event("input");
            thread::sleep(Duration::from_millis(10));
            recorder.record_event("render");
        }

        let stats = recorder.latency_stats("input", "render");
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert_eq!(stats.count, 5);
        assert!(stats.mean >= Duration::from_millis(10));
    }

    #[test]
    fn test_timing_recorder_event_order() {
        let mut recorder = TimingRecorder::new();

        recorder.record_event("event3");
        recorder.record_event("event1");
        recorder.record_event("event2");

        let names: Vec<&str> = recorder.event_names().collect();
        assert_eq!(names, vec!["event3", "event1", "event2"]);
    }

    #[test]
    #[should_panic(expected = "Cannot compute stats from empty samples")]
    fn test_latency_stats_empty_panic() {
        let samples: Vec<Duration> = vec![];
        LatencyStats::from_samples(samples);
    }

    #[test]
    #[should_panic(expected = "Cannot compute percentile from empty data")]
    fn test_percentile_empty_panic() {
        let data: Vec<Duration> = vec![];
        percentile(&data, 50.0);
    }

    #[test]
    #[should_panic(expected = "FPS target must be positive")]
    fn test_fps_to_frame_budget_zero() {
        fps_to_frame_budget(0.0);
    }
}
