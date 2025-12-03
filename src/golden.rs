//! Golden file testing for visual regression detection.
//!
//! This module provides golden file (also called "snapshot" or "blessed file") functionality
//! for TUI applications. Golden files capture the expected terminal output, and tests can
//! compare current output against these saved baselines to detect visual regressions.

use std::{fs, path::PathBuf, time::SystemTime};

use similar::{ChangeTag, TextDiff};

use crate::{
    error::{Result, TermTestError},
    screen::ScreenState,
};

/// Default directory for golden files.
const DEFAULT_GOLDEN_DIR: &str = "tests/golden";

/// Header marker for golden file format.
const GOLDEN_HEADER_START: &str = "--- GOLDEN FILE ---";

/// Content marker for golden file format.
const GOLDEN_CONTENT_START: &str = "--- CONTENT ---";

/// Get the golden file directory from environment or use default.
pub fn get_golden_dir() -> PathBuf {
    std::env::var("GOLDEN_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_GOLDEN_DIR))
}

/// Check if golden files should be updated instead of compared.
pub fn should_update_goldens() -> bool {
    std::env::var("UPDATE_GOLDENS")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// Metadata for a golden file.
#[derive(Debug, Clone)]
pub struct GoldenMetadata {
    /// Name of the test that created this golden file.
    pub test_name: String,
    /// Terminal width in columns.
    pub width: u16,
    /// Terminal height in rows.
    pub height: u16,
    /// Timestamp when the golden file was created.
    pub timestamp: String,
}

impl GoldenMetadata {
    /// Create new metadata for a golden file.
    pub fn new(test_name: impl Into<String>, width: u16, height: u16) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| {
                let secs = d.as_secs();
                let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(secs as i64, 0);
                dt.map(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                    .unwrap_or_else(|| format!("{}s", secs))
            })
            .unwrap_or_else(|_| "unknown".to_string());

        Self {
            test_name: test_name.into(),
            width,
            height,
            timestamp,
        }
    }

    /// Serialize metadata to string format for golden file header.
    pub fn to_header(&self) -> String {
        format!(
            "{}\ntest: {}\nsize: {}x{}\ntimestamp: {}\n",
            GOLDEN_HEADER_START, self.test_name, self.width, self.height, self.timestamp
        )
    }

    /// Parse metadata from a golden file header.
    pub fn from_header(header: &str) -> Option<Self> {
        let lines: Vec<&str> = header.lines().collect();

        if lines.is_empty() || !lines[0].contains(GOLDEN_HEADER_START) {
            return None;
        }

        let mut test_name = String::new();
        let mut width = 0;
        let mut height = 0;
        let mut timestamp = String::new();

        for line in lines.iter().skip(1) {
            if let Some(value) = line.strip_prefix("test: ") {
                test_name = value.to_string();
            } else if let Some(value) = line.strip_prefix("size: ") {
                if let Some((w, h)) = value.split_once('x') {
                    width = w.parse().ok()?;
                    height = h.parse().ok()?;
                }
            } else if let Some(value) = line.strip_prefix("timestamp: ") {
                timestamp = value.to_string();
            }
        }

        Some(Self { test_name, width, height, timestamp })
    }
}

/// A golden file containing expected terminal output.
#[derive(Debug, Clone)]
pub struct GoldenFile {
    /// Metadata about the golden file.
    pub metadata: GoldenMetadata,
    /// The actual terminal content (with ANSI escape codes preserved).
    pub content: String,
}

impl GoldenFile {
    /// Create a new golden file from screen state.
    pub fn from_screen_state(test_name: impl Into<String>, state: &ScreenState) -> Self {
        let (width, height) = state.size();
        let metadata = GoldenMetadata::new(test_name, width, height);
        let content = state.contents();

        Self { metadata, content }
    }

    /// Serialize the golden file to a string.
    pub fn to_string(&self) -> String {
        format!("{}{}\n{}", self.metadata.to_header(), GOLDEN_CONTENT_START, self.content)
    }

    /// Parse a golden file from a string.
    pub fn from_string(content: &str) -> Result<Self> {
        let content_start = content.find(GOLDEN_CONTENT_START).ok_or_else(|| {
            TermTestError::Parse("Golden file missing content marker".to_string())
        })?;

        let header = &content[..content_start];
        let content_body = &content[content_start + GOLDEN_CONTENT_START.len()..];

        let metadata = GoldenMetadata::from_header(header).ok_or_else(|| {
            TermTestError::Parse("Failed to parse golden file header".to_string())
        })?;

        let content = content_body.trim_start_matches('\n').to_string();

        Ok(Self { metadata, content })
    }

    /// Save the golden file to disk.
    pub fn save(&self, name: &str) -> Result<PathBuf> {
        let golden_dir = get_golden_dir();
        fs::create_dir_all(&golden_dir)?;
        let path = golden_dir.join(format!("{}.golden.txt", name));
        fs::write(&path, self.to_string())?;
        Ok(path)
    }

    /// Load a golden file from disk.
    pub fn load(name: &str) -> Result<Self> {
        let golden_dir = get_golden_dir();
        let path = golden_dir.join(format!("{}.golden.txt", name));

        let content = fs::read_to_string(&path).map_err(|e| {
            TermTestError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read golden file '{}': {}", path.display(), e),
            ))
        })?;

        Self::from_string(&content)
    }

    /// Compare this golden file against current screen state.
    pub fn compare(&self, state: &ScreenState) -> Result<()> {
        let current_content = state.contents();

        if self.content == current_content {
            return Ok(());
        }

        let diff = generate_diff(&self.content, &current_content);

        Err(TermTestError::Parse(format!(
            "Golden file mismatch: {}\n{}",
            self.metadata.test_name, diff
        )))
    }
}

/// Generate a unified diff between expected and actual content.
pub fn generate_diff(expected: &str, actual: &str) -> String {
    let diff = TextDiff::from_lines(expected, actual);

    let mut output = String::new();
    output.push_str("--- expected (golden)\n");
    output.push_str("+++ actual\n");

    let mut line_num = 1;

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        if idx > 0 {
            output.push_str("...\n");
        }

        for op in group {
            let old_start = op.old_range().start + 1;
            let old_len = op.old_range().len();
            let new_start = op.new_range().start + 1;
            let new_len = op.new_range().len();

            output.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                old_start, old_len, new_start, new_len
            ));

            for change in diff.iter_changes(op) {
                let (sign, style_prefix) = match change.tag() {
                    ChangeTag::Delete => ("-", "\x1b[31m"),
                    ChangeTag::Insert => ("+", "\x1b[32m"),
                    ChangeTag::Equal => (" ", ""),
                };

                if change.tag() != ChangeTag::Equal {
                    output.push_str(&format!(
                        "{}{:>4} {} {}",
                        style_prefix,
                        line_num,
                        sign,
                        change.value()
                    ));
                    if !change.value().ends_with('\n') {
                        output.push('\n');
                    }
                    output.push_str("\x1b[0m");
                } else {
                    output.push_str(&format!("{:>4} {} {}", line_num, sign, change.value()));
                    if !change.value().ends_with('\n') {
                        output.push('\n');
                    }
                }

                line_num += 1;
            }
        }
    }

    output
}

/// Save the current screen state as a golden file.
pub fn save_golden(name: &str, state: &ScreenState) -> Result<PathBuf> {
    let golden = GoldenFile::from_screen_state(name, state);
    golden.save(name)
}

/// Compare current screen state against a golden file.
pub fn assert_matches_golden(name: &str, state: &ScreenState) -> Result<()> {
    if should_update_goldens() {
        let path = save_golden(name, state)?;
        eprintln!("Updated golden file: {}", path.display());
        Ok(())
    } else {
        let golden = GoldenFile::load(name)?;
        golden.compare(state)
    }
}

/// Update a golden file with new content.
pub fn update_golden(name: &str, state: &ScreenState) -> Result<PathBuf> {
    save_golden(name, state)
}
