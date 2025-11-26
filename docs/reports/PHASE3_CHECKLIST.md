# Phase 3: Sixel Position Tracking - Implementation Checklist

**Status**: Ready for Implementation
**Priority**: P0 (Critical - MVP Blocker, Core Feature for dgx-pixels)
**Dependencies**: Phase 1 Complete (100%), Phase 2 In Progress (85%)
**Target Duration**: 2-3 weeks
**Assignee**: Implementation Agents

## Overview

Phase 3 delivers Sixel graphics position tracking and validation - the core MVP feature that enables dgx-pixels image preview testing. This phase transforms raw Sixel detection into comprehensive graphics validation.

### Goals

1. Parse Sixel raster attributes to extract pixel dimensions
2. Track Sixel graphics positions on the terminal screen
3. Validate Sixel graphics stay within designated bounds (preview areas)
4. Detect Sixel clearing on screen transitions
5. Enable comprehensive dgx-pixels image preview testing

### Success Criteria

- Can parse Sixel raster attributes ("1;1;100;50" → width=100px, height=50px)
- Can track Sixel position (row, col) when sequence starts
- Can validate Sixel graphics within preview area bounds
- Can detect Sixel clearing between screen states
- Can test dgx-pixels image preview scenarios
- All Sixel APIs documented with examples
- Integration tests cover real Sixel sequences

---

## Current State Analysis

### What's Already Working ✅

**Infrastructure (Phase 1)**:
- ✅ vtparse integration with DCS callbacks (dcs_hook, dcs_put, dcs_unhook)
- ✅ VTActor implementation in TerminalState
- ✅ Cursor position tracking via VTActor
- ✅ SixelRegion struct with fields: start_row, start_col, width, height, data
- ✅ sixel_regions() and has_sixel_at() accessors in ScreenState

**Partial Implementation**:
- 🔶 parse_raster_attributes() stub exists in screen.rs (lines 120-144)
  - Searches for '"' character to find raster attributes
  - Parses format "Pa;Pb;Ph;Pv" where Ph=width, Pv=height
  - Returns Option<(u32, u32)> for (width, height)
  - Currently returns (0, 0) as fallback
- 🔶 Sixel detection via dcs_hook() when mode == 'q' (0x71)
- 🔶 Sixel data accumulation in dcs_put()
- 🔶 Sixel region creation in dcs_unhook()

**Testing Foundation**:
- ✅ SixelSequence and SixelCapture types in src/sixel.rs
- ✅ Basic tests in tests/integration/sixel.rs
- ✅ Example program in examples/sixel_test.rs
- ✅ Real Sixel test data can be created

### What Needs Work 🚧

**Parsing Enhancement**:
- 🔶 parse_raster_attributes() needs robust parsing
  - Handle malformed sequences gracefully
  - Validate parameter ranges
  - Better error reporting
  - Support missing parameters (use defaults)

**Position Tracking**:
- 🔶 Cursor position capture is working but needs validation
- 🔶 Need cell calculation from pixel dimensions
- 🔶 Need bounds checking helpers

**Validation APIs**:
- 🔶 Harness methods need to be added:
  - assert_sixel_within_bounds()
  - get_sixel_at()
  - verify_sixel_cleared()
  - sixel_count()

**Testing**:
- 🔶 Need real Sixel sequence test data
- 🔶 Need integration tests with actual Sixel rendering
- 🔶 Need dgx-pixels scenario tests

---

## Task Breakdown

### 1. Sixel Raster Attribute Parsing

#### 1.1 Enhance parse_raster_attributes()

**Priority**: P0
**Estimated Effort**: 3-4 hours
**Location**: src/screen.rs:120-144

**Current Implementation Analysis**:
```rust
fn parse_raster_attributes(&self, data: &[u8]) -> Option<(u32, u32)> {
    let data_str = std::str::from_utf8(data).ok()?;

    // Find the raster attributes command starting with '"'
    if let Some(raster_start) = data_str.find('"') {
        let raster_part = &data_str[raster_start + 1..];

        // Parse format: Pa;Pb;Ph;Pv
        // We want Ph (width) and Pv (height)
        let parts: Vec<&str> = raster_part
            .split(|c: char| !c.is_ascii_digit() && c != ';')
            .filter(|s| !s.is_empty())
            .take(4)
            .collect();

        if parts.len() >= 4 {
            let width = parts[2].parse::<u32>().ok()?;
            let height = parts[3].parse::<u32>().ok()?;
            return Some((width, height));
        }
    }
    None
}
```

**Tasks**:
- [ ] Enhance existing implementation to handle edge cases
  - Handle missing raster attributes (no '"' character)
  - Handle incomplete parameter sequences (< 4 parameters)
  - Handle out-of-range values (0 or excessively large)
  - Handle malformed numbers (non-digit characters)
- [ ] Add default dimension handling
  - If no raster attributes: estimate from data size
  - Default to reasonable values (e.g., 100x100)
  - Document default behavior
- [ ] Validate parsed dimensions
  - Width and height should be > 0
  - Width and height should be < 10000 (sanity check)
  - Log warnings for suspicious values
- [ ] Add comprehensive unit tests
  - Test valid sequences: "1;1;100;50"
  - Test missing raster attributes (no '"')
  - Test incomplete parameters: "1;1;100" (only 3 params)
  - Test zero dimensions: "1;1;0;0"
  - Test huge dimensions: "1;1;99999;99999"
  - Test malformed data: "1;abc;100;50"
  - Test UTF-8 boundary issues
- [ ] Document raster attribute format in rustdoc
  - Explain Pan, Pad, Ph, Pv parameters
  - Show example sequences
  - Document fallback behavior

**Sixel Raster Attribute Format**:
```
DCS Pa ; Pad ; Ph ; Pv q <sixel data> ST

Where:
- Pa: Pixel aspect ratio (usually 1 or 2)
- Pad: Padding (usually 1)
- Ph: Horizontal pixel size (width)
- Pv: Vertical pixel size (height)

Example: ESC P 1 ; 1 ; 100 ; 50 q <data> ESC \
         Means: 100 pixels wide, 50 pixels tall
```

**Test Cases**:
```rust
#[test]
fn test_parse_raster_attributes_valid() {
    let data = b"\"1;1;100;50#0~";
    let (width, height) = parse_raster_attributes(data).unwrap();
    assert_eq!(width, 100);
    assert_eq!(height, 50);
}

#[test]
fn test_parse_raster_attributes_missing() {
    let data = b"#0~"; // No raster attributes
    let result = parse_raster_attributes(data);
    assert!(result.is_none() || result == Some((100, 100))); // Fallback
}

#[test]
fn test_parse_raster_attributes_incomplete() {
    let data = b"\"1;1;100"; // Only 3 parameters
    let result = parse_raster_attributes(data);
    assert!(result.is_none());
}
```

**Success Criteria**:
- Parses valid raster attributes correctly
- Handles missing/malformed sequences gracefully
- All unit tests pass
- Edge cases documented

---

#### 1.2 Pixel-to-Cell Conversion

**Priority**: P0
**Estimated Effort**: 2-3 hours
**Location**: src/screen.rs (new helper methods)

**Background**:
Terminals render graphics in character cells. Need to convert Sixel pixel dimensions to terminal cell dimensions for accurate bounds checking.

**Standard Conversion**:
- Vertical: 6 pixels per line (Sixel standard)
- Horizontal: 8-10 pixels per column (varies by terminal/font)
- For dgx-pixels testing: use configurable conversion ratios

**Tasks**:
- [ ] Add configuration for pixel-to-cell ratios
  - Add fields to TerminalState or config
  - Default: 6 pixels vertical, 8 pixels horizontal
  - Allow override for different terminals
- [ ] Implement conversion helper methods
  ```rust
  impl TerminalState {
      /// Convert pixel dimensions to cell dimensions
      fn pixels_to_cells(&self, width_px: u32, height_px: u32) -> (u16, u16) {
          let cols = ((width_px + self.h_pixels_per_cell - 1) / self.h_pixels_per_cell) as u16;
          let rows = ((height_px + self.v_pixels_per_cell - 1) / self.v_pixels_per_cell) as u16;
          (cols, rows)
      }
  }
  ```
- [ ] Update SixelRegion to store both pixel and cell dimensions
  ```rust
  pub struct SixelRegion {
      pub start_row: u16,
      pub start_col: u16,
      pub width: u32,      // pixels
      pub height: u32,     // pixels
      pub width_cells: u16,  // NEW: cell width
      pub height_cells: u16, // NEW: cell height
      pub data: Vec<u8>,
  }
  ```
- [ ] Update dcs_unhook() to calculate cell dimensions
- [ ] Add unit tests for conversion
  - Test exact divisions: 60 pixels / 6 = 10 rows
  - Test rounding up: 61 pixels / 6 = 11 rows (ceiling)
  - Test zero pixels (edge case)
  - Test large dimensions

**Example**:
```rust
// Sixel is 100x60 pixels
// Terminal: 6 pixels per row, 8 pixels per column
// Cell size: (100/8) x (60/6) = 13 columns x 10 rows (rounded up)
```

**Success Criteria**:
- Conversion methods work correctly
- SixelRegion includes cell dimensions
- Tests verify rounding behavior
- Configuration is flexible

---

### 2. Position Tracking Enhancement

#### 2.1 Validate Cursor Position Capture

**Priority**: P0
**Estimated Effort**: 2-3 hours
**Location**: src/screen.rs:173-214 (dcs_hook, dcs_unhook)

**Current Implementation**:
```rust
fn dcs_hook(&mut self, mode: u8, params: &[i64], ...) {
    if mode == b'q' {
        self.in_sixel_mode = true;
        self.current_sixel_data.clear();
        self.current_sixel_params = params.to_vec();
    }
}

fn dcs_unhook(&mut self) {
    if self.in_sixel_mode {
        let (width, height) = self
            .parse_raster_attributes(&self.current_sixel_data)
            .unwrap_or((0, 0));

        let region = SixelRegion {
            start_row: self.cursor_pos.0,  // ← Captured here
            start_col: self.cursor_pos.1,  // ← Captured here
            width,
            height,
            data: self.current_sixel_data.clone(),
        };
        self.sixel_regions.push(region);

        self.in_sixel_mode = false;
        // ...
    }
}
```

**Tasks**:
- [ ] Verify cursor position is captured before DCS processing
  - Test that cursor position is from *start* of Sixel, not end
  - Ensure cursor doesn't move during DCS processing
- [ ] Add integration test with real Sixel sequence
  ```rust
  #[test]
  fn test_sixel_position_tracking() {
      let mut screen = ScreenState::new(80, 24);

      // Move cursor to (5, 10)
      screen.feed(b"\x1b[6;11H"); // CSI uses 1-based indexing

      // Render Sixel
      screen.feed(b"\x1bPq\"1;1;100;50#0~\x1b\\");

      let regions = screen.sixel_regions();
      assert_eq!(regions.len(), 1);
      assert_eq!(regions[0].start_row, 5); // 0-based
      assert_eq!(regions[0].start_col, 10);
  }
  ```
- [ ] Handle cursor position edge cases
  - Sixel at (0, 0) - top-left corner
  - Sixel at screen boundaries
  - Sixel that would overflow screen
- [ ] Document cursor position semantics
  - Position is where Sixel *starts*
  - 0-based indexing
  - Position may be outside screen if cursor was moved there

**Success Criteria**:
- Cursor position capture is verified correct
- Integration tests pass
- Edge cases handled
- Behavior documented

---

#### 2.2 Bounds Checking Helpers

**Priority**: P0
**Estimated Effort**: 3-4 hours
**Location**: src/sixel.rs (enhance existing types)

**Tasks**:
- [ ] Add bounds checking method to SixelRegion
  ```rust
  impl SixelRegion {
      /// Check if this region is completely within the given area (in cells)
      pub fn is_within_cells(&self, area: (u16, u16, u16, u16)) -> bool {
          let (area_row, area_col, area_width, area_height) = area;

          self.start_row >= area_row
              && self.start_col >= area_col
              && (self.start_row + self.height_cells) <= (area_row + area_height)
              && (self.start_col + self.width_cells) <= (area_col + area_width)
      }

      /// Check if this region overlaps with the given area (in cells)
      pub fn overlaps_cells(&self, area: (u16, u16, u16, u16)) -> bool {
          let (area_row, area_col, area_width, area_height) = area;

          !(self.start_row + self.height_cells <= area_row
              || self.start_col + self.width_cells <= area_col
              || self.start_row >= area_row + area_height
              || self.start_col >= area_col + area_width)
      }
  }
  ```
- [ ] Update SixelSequence to use cell-based checking
  - Keep existing pixel-based is_within() for compatibility
  - Add new is_within_cells() using cell dimensions
- [ ] Add comprehensive unit tests
  - Test region completely inside area
  - Test region completely outside area
  - Test region partially overlapping
  - Test region exactly matches area
  - Test edge cases (0-size regions, 0-size areas)
- [ ] Document coordinate system clearly
  - All positions in 0-based cells
  - Area format: (row, col, width, height)
  - Width/height are in cells, not pixels

**Test Cases**:
```rust
#[test]
fn test_sixel_within_cells() {
    let region = SixelRegion {
        start_row: 5,
        start_col: 10,
        width: 100,    // pixels
        height: 60,    // pixels
        width_cells: 13,  // 100/8 rounded up
        height_cells: 10, // 60/6
        data: vec![],
    };

    // Region is at (5,10) with size 13x10 cells
    // So it occupies rows 5-14, cols 10-22

    let preview_area = (0, 0, 30, 20);
    assert!(region.is_within_cells(preview_area));

    let small_area = (0, 0, 15, 10);
    assert!(!region.is_within_cells(small_area)); // Extends beyond
}
```

**Success Criteria**:
- Bounds checking methods work correctly
- Both pixel and cell-based checking available
- Tests cover all cases
- Documentation is clear

---

### 3. Validation API Implementation

#### 3.1 Harness Validation Methods

**Priority**: P0
**Estimated Effort**: 4-5 hours
**Location**: src/harness.rs (new methods)

**Tasks**:
- [ ] Add assert_sixel_within_bounds() method
  ```rust
  impl TuiTestHarness {
      /// Assert that all Sixel graphics are within the specified area.
      ///
      /// # Arguments
      ///
      /// * `area` - Bounding area as (row, col, width, height) in cells
      ///
      /// # Errors
      ///
      /// Returns an error if any Sixel graphic extends outside the area.
      ///
      /// # Example
      ///
      /// ```rust
      /// # use term_test::TuiTestHarness;
      /// # fn test() -> term_test::Result<()> {
      /// let mut harness = TuiTestHarness::new(80, 24)?;
      /// // ... render Sixel graphics ...
      ///
      /// let preview_area = (5, 30, 45, 20); // Preview panel
      /// harness.assert_sixel_within_bounds(preview_area)?;
      /// # Ok(())
      /// # }
      /// ```
      pub fn assert_sixel_within_bounds(&self, area: (u16, u16, u16, u16)) -> Result<()> {
          let capture = SixelCapture::from_screen_state(&self.state);
          capture.assert_all_within(area)
      }
  }
  ```
- [ ] Add get_sixel_at() query method
  ```rust
  /// Get the Sixel region at the specified position.
  ///
  /// Returns the first Sixel region whose starting position matches
  /// the given coordinates.
  pub fn get_sixel_at(&self, row: u16, col: u16) -> Option<&SixelRegion> {
      self.state.sixel_regions()
          .iter()
          .find(|r| r.start_row == row && r.start_col == col)
  }
  ```
- [ ] Add verify_sixel_cleared() comparison method
  ```rust
  /// Verify that Sixel graphics have been cleared since the last check.
  ///
  /// This method compares the current Sixel state with a previous snapshot.
  /// Useful for verifying that graphics are properly cleared on screen
  /// transitions.
  ///
  /// # Arguments
  ///
  /// * `previous` - Previous SixelCapture snapshot
  ///
  /// # Returns
  ///
  /// `true` if the Sixel state has changed (cleared or modified)
  pub fn verify_sixel_cleared(&self, previous: &SixelCapture) -> bool {
      let current = SixelCapture::from_screen_state(&self.state);
      current.differs_from(previous)
  }
  ```
- [ ] Add sixel_count() convenience method
  ```rust
  /// Returns the number of Sixel graphics currently on screen.
  pub fn sixel_count(&self) -> usize {
      self.state.sixel_regions().len()
  }
  ```
- [ ] Add integration tests for all methods
  - Test assert_sixel_within_bounds with valid/invalid positions
  - Test get_sixel_at with existing/missing positions
  - Test verify_sixel_cleared across screen transitions
  - Test sixel_count with 0, 1, multiple graphics

**Example Usage**:
```rust
// dgx-pixels image preview testing
#[test]
fn test_dgx_pixels_preview() -> Result<()> {
    let mut harness = TuiTestHarness::new(120, 40)?;
    // ... spawn dgx-pixels, navigate to Gallery ...

    // Define preview area (right panel)
    let preview_area = (5, 70, 45, 30);

    // Wait for image to render
    harness.wait_for(|state| state.sixel_count() > 0)?;

    // Verify image is within preview area
    harness.assert_sixel_within_bounds(preview_area)?;

    // Navigate away
    harness.send_key(KeyCode::Esc)?;

    // Verify image is cleared
    let prev_capture = SixelCapture::from_screen_state(&harness.state);
    harness.wait_for_text("Main Menu")?;
    assert!(harness.verify_sixel_cleared(&prev_capture));

    Ok(())
}
```

**Success Criteria**:
- All validation methods implemented
- Integration tests pass
- Error messages are helpful
- Examples demonstrate usage

---

#### 3.2 Enhanced SixelCapture Methods

**Priority**: P1
**Estimated Effort**: 2-3 hours
**Location**: src/sixel.rs (enhance existing type)

**Tasks**:
- [ ] Add query methods to SixelCapture
  ```rust
  impl SixelCapture {
      /// Get all sequences that overlap with the specified area
      pub fn sequences_overlapping(&self, area: (u16, u16, u16, u16)) -> Vec<&SixelSequence> {
          self.sequences
              .iter()
              .filter(|seq| seq.overlaps(area))
              .collect()
      }

      /// Get sequences at specific row
      pub fn sequences_at_row(&self, row: u16) -> Vec<&SixelSequence> {
          self.sequences
              .iter()
              .filter(|seq| seq.position.0 == row)
              .collect()
      }

      /// Check if any sequences exist in the given area
      pub fn has_sequences_in(&self, area: (u16, u16, u16, u16)) -> bool {
          !self.sequences_in_area(area).is_empty()
      }
  }
  ```
- [ ] Add statistical methods
  ```rust
  /// Get total area covered by all Sixel graphics (in cells)
  pub fn total_coverage(&self) -> u32 {
      self.sequences
          .iter()
          .map(|seq| {
              let (_, _, w, h) = seq.bounds;
              w as u32 * h as u32
          })
          .sum()
  }

  /// Get bounding box containing all Sixel graphics
  pub fn bounding_box(&self) -> Option<(u16, u16, u16, u16)> {
      if self.sequences.is_empty() {
          return None;
      }

      let mut min_row = u16::MAX;
      let mut min_col = u16::MAX;
      let mut max_row = 0u16;
      let mut max_col = 0u16;

      for seq in &self.sequences {
          let (r, c, w, h) = seq.bounds;
          min_row = min_row.min(r);
          min_col = min_col.min(c);
          max_row = max_row.max(r + h);
          max_col = max_col.max(c + w);
      }

      Some((min_row, min_col, max_col - min_col, max_row - min_row))
  }
  ```
- [ ] Add unit tests for new methods
- [ ] Document use cases for each method

**Success Criteria**:
- Query methods implemented
- Statistical methods work
- Tests pass
- Documentation complete

---

### 4. Test Data and Fixtures

#### 4.1 Real Sixel Sequence Generation

**Priority**: P0
**Estimated Effort**: 3-4 hours
**Location**: tests/fixtures/sixel/ (new directory)

**Tasks**:
- [ ] Create tests/fixtures/sixel/ directory
- [ ] Generate or obtain real Sixel test data
  - Simple solid color rectangles (easy to verify)
  - Small images (10x10, 20x20 pixels)
  - Medium images (100x100 pixels)
  - Large images (500x500 pixels)
  - Images with various color palettes
- [ ] Create helper to generate Sixel sequences programmatically
  ```rust
  // tests/helpers/sixel_generator.rs
  pub fn generate_solid_sixel(width: u32, height: u32, color_rgb: (u8, u8, u8)) -> Vec<u8> {
      // Generate DCS P q ... ESC \ sequence
      // with solid color rectangle
  }

  pub fn generate_sixel_with_raster(width: u32, height: u32) -> Vec<u8> {
      // Generate sequence with proper raster attributes
  }
  ```
- [ ] Create test fixtures with known properties
  - tests/fixtures/sixel/red_100x50.sixel
  - tests/fixtures/sixel/blue_200x100.sixel
  - tests/fixtures/sixel/gradient_150x150.sixel
- [ ] Document fixture format and usage
  - README.md in fixtures directory
  - Describe each fixture
  - Show how to use in tests

**Sixel Format Reference**:
```
DCS Pa ; Pad ; Ph ; Pv q <sixel data> ST

Example solid red 100x50:
ESC P 1 ; 1 ; 100 ; 50 q " 1 ; 1 ; 100 ; 50 # 0 ; 2 ; 100 ; 0 ; 0 # 0 ~~~~~~~~...~ ESC \

Where:
- DCS = ESC P (0x1b 0x50)
- Raster attributes: "1;1;100;50
- Color definition: #0;2;100;0;0 (red)
- Sixel data: #0~~~~~~... (fill with color 0)
- ST = ESC \ (0x1b 0x5c)
```

**Test Helper**:
```rust
#[cfg(test)]
mod fixtures {
    use std::path::PathBuf;

    pub fn sixel_fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("sixel")
            .join(name)
    }

    pub fn load_sixel_fixture(name: &str) -> Vec<u8> {
        std::fs::read(sixel_fixture_path(name))
            .expect("Failed to load fixture")
    }
}
```

**Success Criteria**:
- Test fixtures directory created
- Multiple Sixel sequences available
- Helper functions work
- Documentation explains usage

---

#### 4.2 Integration Test Suite

**Priority**: P0
**Estimated Effort**: 5-6 hours
**Location**: tests/integration/sixel.rs (expand)

**Tasks**:
- [ ] Expand tests/integration/sixel.rs with real sequences
  ```rust
  #[test]
  fn test_sixel_parsing_with_raster_attributes() -> Result<()> {
      let mut screen = ScreenState::new(80, 24);

      // Feed complete Sixel sequence with raster attributes
      screen.feed(b"\x1b[10;10H"); // Position cursor
      screen.feed(b"\x1bPq\"1;1;100;50#0;2;100;0;0#0~~~~~~\x1b\\");

      let regions = screen.sixel_regions();
      assert_eq!(regions.len(), 1);

      let region = &regions[0];
      assert_eq!(region.start_row, 9); // 0-based
      assert_eq!(region.start_col, 9);
      assert_eq!(region.width, 100);
      assert_eq!(region.height, 50);

      Ok(())
  }
  ```
- [ ] Test position tracking accuracy
  ```rust
  #[test]
  fn test_multiple_sixel_positions() -> Result<()> {
      let mut screen = ScreenState::new(80, 24);

      // Render Sixel at (5, 10)
      screen.feed(b"\x1b[6;11H\x1bPq\"1;1;50;30#0~\x1b\\");

      // Render Sixel at (15, 40)
      screen.feed(b"\x1b[16;41H\x1bPq\"1;1;60;40#0~\x1b\\");

      let regions = screen.sixel_regions();
      assert_eq!(regions.len(), 2);

      assert_eq!(regions[0].start_row, 5);
      assert_eq!(regions[0].start_col, 10);

      assert_eq!(regions[1].start_row, 15);
      assert_eq!(regions[1].start_col, 40);

      Ok(())
  }
  ```
- [ ] Test bounds validation
  ```rust
  #[test]
  fn test_sixel_bounds_validation() -> Result<()> {
      let mut screen = ScreenState::new(100, 40);

      // Define preview area
      let preview_area = (5, 30, 45, 20);

      // Render Sixel within preview
      screen.feed(b"\x1b[10;35H\x1bPq\"1;1;200;100#0~\x1b\\");

      let capture = SixelCapture::from_screen_state(&screen);
      assert!(capture.assert_all_within(preview_area).is_ok());

      Ok(())
  }
  ```
- [ ] Test clearing detection
  ```rust
  #[test]
  fn test_sixel_clearing() -> Result<()> {
      let mut screen1 = ScreenState::new(80, 24);
      screen1.feed(b"\x1bPq\"1;1;100;50#0~\x1b\\");
      let capture1 = SixelCapture::from_screen_state(&screen1);

      let screen2 = ScreenState::new(80, 24);
      let capture2 = SixelCapture::from_screen_state(&screen2);

      assert!(capture1.differs_from(&capture2));
      assert_eq!(capture1.sequences().len(), 1);
      assert_eq!(capture2.sequences().len(), 0);

      Ok(())
  }
  ```
- [ ] Test edge cases
  - Sixel at screen boundaries
  - Sixel with missing raster attributes
  - Malformed Sixel sequences
  - Very large Sixel graphics
  - Multiple overlapping Sixel graphics

**Success Criteria**:
- All integration tests pass
- Real Sixel sequences tested
- Edge cases covered
- Tests are reliable and fast

---

### 5. dgx-pixels Integration Validation

#### 5.1 dgx-pixels Preview Scenario Tests

**Priority**: P0 (MVP Requirement)
**Estimated Effort**: 4-5 hours
**Location**: tests/integration/dgx_pixels_scenarios.rs (new)

**Tasks**:
- [ ] Create dgx-pixels-specific integration tests
- [ ] Test Gallery screen preview area
  ```rust
  #[test]
  fn test_dgx_pixels_gallery_preview() -> Result<()> {
      // Simulate dgx-pixels Gallery screen layout:
      // - Terminal: 120x40
      // - Left sidebar: columns 0-29
      // - Preview panel: columns 30-99, rows 5-35

      let mut harness = TuiTestHarness::new(120, 40)?;

      // Simulate rendering in preview area
      harness.send_text("\x1b[10;40H")?; // Position in preview
      harness.send_text("\x1bPq\"1;1;400;300#0;2;0;100;200#0~~~~~~\x1b\\")?;
      harness.update_state()?;

      // Verify Sixel is within preview bounds
      let preview_area = (5, 30, 70, 30);
      harness.assert_sixel_within_bounds(preview_area)?;

      // Verify no Sixel in sidebar
      let sidebar_area = (0, 0, 30, 40);
      let capture = SixelCapture::from_screen_state(&harness.state());
      assert!(capture.sequences_in_area(sidebar_area).is_empty());

      Ok(())
  }
  ```
- [ ] Test screen transition clearing
  ```rust
  #[test]
  fn test_dgx_pixels_screen_transition() -> Result<()> {
      let mut harness = TuiTestHarness::new(120, 40)?;

      // Gallery screen with image
      harness.send_text("\x1bPq\"1;1;400;300#0~\x1b\\")?;
      harness.update_state()?;
      let gallery_capture = SixelCapture::from_screen_state(&harness.state());
      assert_eq!(gallery_capture.sequences().len(), 1);

      // Transition to different screen (clear screen)
      harness.send_text("\x1b[2J")?; // Clear screen
      harness.update_state()?;
      let new_capture = SixelCapture::from_screen_state(&harness.state());

      // Note: Sixel regions persist in our tracking until explicit clear
      // This might need adjustment based on requirements
      assert!(gallery_capture.differs_from(&new_capture));

      Ok(())
  }
  ```
- [ ] Test multiple image scenario
  ```rust
  #[test]
  fn test_dgx_pixels_multiple_images() -> Result<()> {
      // Test scenario where multiple thumbnails appear
      let mut harness = TuiTestHarness::new(120, 40)?;

      // Render 3 thumbnail images in gallery grid
      for i in 0..3 {
          let row = 10 + (i * 10);
          harness.send_text(&format!("\x1b[{};40H", row + 1))?;
          harness.send_text("\x1bPq\"1;1;80;60#0~\x1b\\")?;
      }
      harness.update_state()?;

      assert_eq!(harness.sixel_count(), 3);

      // Verify all within bounds
      let preview_area = (5, 30, 70, 35);
      harness.assert_sixel_within_bounds(preview_area)?;

      Ok(())
  }
  ```
- [ ] Document dgx-pixels testing patterns
  - Create docs/DGX_PIXELS_TESTING.md
  - Explain screen layouts
  - Show preview area definitions
  - Provide example test code

**dgx-pixels Screen Layouts**:
```
Gallery Screen (120x40 terminal):
┌─────────────────────────────┬──────────────────────────────────────────────┐
│                             │                                              │
│   Sidebar (0-29)            │   Preview Area (30-99)                       │
│   - Image list              │   - Large preview image                      │
│   - Navigation              │   - Centered in panel                        │
│                             │   - Should stay within bounds                │
│                             │                                              │
└─────────────────────────────┴──────────────────────────────────────────────┘

Preview Area Bounds: (5, 30, 70, 30)
```

**Success Criteria**:
- dgx-pixels scenarios tested
- Preview area validation works
- Screen transitions tested
- Documentation complete

---

### 6. Documentation & Polish

#### 6.1 API Documentation

**Priority**: P0
**Estimated Effort**: 3-4 hours

**Tasks**:
- [ ] Complete rustdoc for all Phase 3 APIs
  - parse_raster_attributes()
  - Pixel-to-cell conversion methods
  - SixelRegion fields and methods
  - Harness validation methods
  - SixelCapture enhancements
- [ ] Add comprehensive examples to rustdoc
  - Show complete usage patterns
  - Include dgx-pixels scenarios
  - Link to test fixtures
- [ ] Update module-level documentation
  - src/sixel.rs overview
  - src/screen.rs Sixel section
  - src/harness.rs Sixel methods
- [ ] Run `cargo doc` and verify output
  - Check all links work
  - Verify examples compile
  - Fix any warnings

**Success Criteria**:
- 100% API documentation
- All examples compile
- No rustdoc warnings
- Documentation is helpful

---

#### 6.2 User Guide

**Priority**: P1
**Estimated Effort**: 4-5 hours
**Location**: docs/SIXEL_TESTING.md (new)

**Tasks**:
- [ ] Create comprehensive Sixel testing guide
  - What is Sixel?
  - Why test Sixel graphics?
  - How mimic tracks Sixel
  - Position tracking explained
  - Bounds validation patterns
- [ ] Document coordinate systems
  - 0-based row/col positions
  - Pixel vs cell dimensions
  - Area format: (row, col, width, height)
- [ ] Provide code examples
  - Basic Sixel detection
  - Position verification
  - Bounds checking
  - Screen transition testing
  - dgx-pixels patterns
- [ ] Add troubleshooting section
  - Sixel not detected → check DCS
  - Wrong position → verify cursor tracking
  - Bounds check failing → debug area definition
  - Graphics persisting → understand clearing behavior
- [ ] Create quick reference
  - Table of all Sixel APIs
  - Common patterns
  - Error messages and solutions

**Documentation Structure**:
```markdown
# Sixel Graphics Testing Guide

## Overview
- What is Sixel?
- Terminal graphics testing challenges
- How mimic solves these

## Getting Started
- Basic Sixel detection
- Position tracking example
- First bounds check test

## Core Concepts
- Coordinate systems (0-based cells)
- Pixel vs cell dimensions
- Raster attributes parsing
- Position capture timing

## API Reference
- ScreenState methods
- SixelCapture methods
- Harness validation methods

## Testing Patterns
- Single image preview
- Multiple thumbnails
- Screen transitions
- Clearing validation

## dgx-pixels Integration
- Screen layouts
- Preview area definitions
- Example tests

## Troubleshooting
- Common issues and solutions
```

**Success Criteria**:
- Guide is comprehensive
- Examples are clear
- Troubleshooting helps
- dgx-pixels patterns documented

---

#### 6.3 Example Programs

**Priority**: P1
**Estimated Effort**: 3-4 hours
**Location**: examples/sixel_test.rs (expand)

**Tasks**:
- [ ] Expand examples/sixel_test.rs with real scenarios
  - Add real Sixel sequence rendering
  - Show position tracking
  - Demonstrate bounds validation
  - Show clearing detection
- [ ] Create examples/dgx_pixels_preview.rs
  - Simulate dgx-pixels Gallery layout
  - Show preview area validation
  - Demonstrate screen transitions
  - Include comments explaining each step
- [ ] Update existing Sixel examples with Phase 3 features
- [ ] Verify all examples run successfully
- [ ] Add examples to README

**Example Structure**:
```rust
//! examples/dgx_pixels_preview.rs
//!
//! Demonstrates Sixel testing for dgx-pixels Gallery screen.

use term_test::{Result, TuiTestHarness, SixelCapture};

fn main() -> Result<()> {
    println!("=== dgx-pixels Gallery Preview Testing ===\n");

    // Example 1: Basic preview area validation
    example_preview_area_validation()?;

    // Example 2: Multiple images
    example_multiple_thumbnails()?;

    // Example 3: Screen transitions
    example_screen_transition()?;

    println!("\n=== All Examples Complete ===");
    Ok(())
}

fn example_preview_area_validation() -> Result<()> {
    println!("--- Example 1: Preview Area Validation ---");

    let mut harness = TuiTestHarness::new(120, 40)?;

    // Define dgx-pixels Gallery preview area
    let preview_area = (5, 30, 70, 30);
    println!("Preview area: {:?}", preview_area);

    // Simulate image rendering in preview
    harness.send_text("\x1b[10;40H")?; // Position in preview
    harness.send_text("\x1bPq\"1;1;400;300#0~\x1b\\")?;
    harness.update_state()?;

    // Validate bounds
    match harness.assert_sixel_within_bounds(preview_area) {
        Ok(()) => println!("✓ Image is within preview area"),
        Err(e) => println!("✗ Validation failed: {}", e),
    }

    Ok(())
}
```

**Success Criteria**:
- Examples demonstrate all features
- Examples run without errors
- Comments explain concepts
- README includes examples

---

### 7. Testing & CI Integration

#### 7.1 Comprehensive Test Suite

**Priority**: P0
**Estimated Effort**: 3-4 hours

**Tasks**:
- [ ] Ensure all unit tests pass
  - Run `cargo test --lib`
  - Fix any failures
  - Verify coverage
- [ ] Ensure all integration tests pass
  - Run `cargo test --test '*'`
  - Verify Sixel-specific tests
  - Check dgx-pixels scenarios
- [ ] Add Sixel tests to CI pipeline
  - Update .github/workflows/ci.yml
  - Run Sixel tests separately
  - Check for flakiness
- [ ] Performance testing
  - Benchmark Sixel parsing
  - Benchmark position tracking
  - Ensure overhead is minimal
- [ ] Test on Linux headlessly
  - Verify no display requirements
  - Check CI passes

**CI Configuration**:
```yaml
# .github/workflows/ci.yml
- name: Test Sixel Support
  run: cargo test --lib sixel

- name: Test Sixel Integration
  run: cargo test --test sixel

- name: Test dgx-pixels Scenarios
  run: cargo test --test dgx_pixels_scenarios
```

**Success Criteria**:
- All tests pass
- CI pipeline green
- No flaky tests
- Performance acceptable

---

#### 7.2 Coverage & Quality Gates

**Priority**: P1
**Estimated Effort**: 2-3 hours

**Tasks**:
- [ ] Run coverage analysis
  - Use cargo-tarpaulin or similar
  - Focus on Phase 3 code
  - Target: >70% coverage
- [ ] Review code quality
  - Run clippy with strict settings
  - Fix all warnings
  - Apply rustfmt
- [ ] Security audit
  - Check for unsafe code
  - Validate input parsing security
  - Review error handling
- [ ] Documentation quality
  - Verify all public APIs documented
  - Check for broken links
  - Validate examples compile

**Quality Checklist**:
- [ ] All clippy warnings resolved
- [ ] Code formatted with rustfmt
- [ ] No unsafe code without justification
- [ ] Input validation comprehensive
- [ ] Error messages actionable
- [ ] Documentation complete

**Success Criteria**:
- Code quality high
- Coverage >70%
- No security issues
- Documentation complete

---

## Timeline Estimate

### Week 1 (Days 1-5): Core Parsing & Tracking

**Days 1-2**: Raster attribute parsing
- Enhance parse_raster_attributes()
- Add unit tests
- Handle edge cases

**Day 3**: Position tracking
- Validate cursor position capture
- Add integration tests
- Fix any issues

**Days 4-5**: Pixel-to-cell conversion
- Implement conversion helpers
- Update SixelRegion structure
- Add comprehensive tests

### Week 2 (Days 6-10): Validation APIs & Testing

**Days 6-7**: Harness validation methods
- Implement assert_sixel_within_bounds()
- Add query methods
- Write integration tests

**Day 8**: Test fixtures
- Generate Sixel test data
- Create fixture helpers
- Organize test resources

**Days 9-10**: Integration tests
- Expand sixel.rs tests
- Add dgx-pixels scenarios
- Verify all tests pass

### Week 3 (Days 11-15): Documentation & Polish

**Days 11-12**: Documentation
- Complete API rustdoc
- Write user guide
- Update examples

**Day 13**: dgx-pixels validation
- Test all scenarios
- Verify requirements met
- Document patterns

**Days 14-15**: CI & polish
- Integrate with CI pipeline
- Run quality checks
- Fix any remaining issues

**Total**: 15 days (3 weeks)

---

## Dependencies

### Internal (Phase 1 & 2)
- ✅ ScreenState with vtparse (Phase 1)
- ✅ DCS callback infrastructure (Phase 1)
- ✅ Cursor position tracking (Phase 1)
- ✅ TuiTestHarness (Phase 1)
- 🔶 Event simulation (Phase 2) - for triggering Sixel rendering
- 🔶 Wait conditions (Phase 2) - for detecting Sixel appearance

### External
- ✅ vtparse = "0.7" (already integrated)
- ✅ portable-pty = "0.8" (already integrated)
- ✅ No new dependencies needed

---

## Risk Assessment

### High Risk

**Risk**: Raster attribute parsing edge cases
- **Mitigation**: Comprehensive test suite, graceful fallbacks
- **Fallback**: Manual dimension configuration API

**Risk**: Terminal-specific pixel-to-cell ratios
- **Mitigation**: Configurable conversion ratios, defaults work for common terminals
- **Fallback**: Document per-terminal settings

### Medium Risk

**Risk**: Sixel clearing detection complexity
- **Mitigation**: Clear API semantics, document expected behavior
- **Fallback**: Provide manual clear tracking methods

**Risk**: Test fixture generation
- **Mitigation**: Use simple solid colors, document format
- **Fallback**: Accept any valid Sixel sequences

### Low Risk

**Risk**: Position tracking accuracy
- **Status**: Already implemented and working
- **Mitigation**: Integration tests verify correctness

---

## Acceptance Criteria

Phase 3 is complete when:

1. **Parsing**
   - [ ] Raster attributes parsed correctly
   - [ ] Pixel-to-cell conversion works
   - [ ] Edge cases handled gracefully
   - [ ] All parsing tests pass

2. **Position Tracking**
   - [ ] Cursor position captured at Sixel start
   - [ ] Cell dimensions calculated
   - [ ] Multiple Sixel graphics tracked
   - [ ] Position tests pass

3. **Validation APIs**
   - [ ] assert_sixel_within_bounds() works
   - [ ] Query methods implemented
   - [ ] Clearing detection works
   - [ ] All validation tests pass

4. **Testing**
   - [ ] Real Sixel sequences tested
   - [ ] Integration tests comprehensive
   - [ ] dgx-pixels scenarios covered
   - [ ] All tests pass in CI

5. **Documentation**
   - [ ] All APIs documented
   - [ ] User guide complete
   - [ ] Examples demonstrate features
   - [ ] dgx-pixels patterns documented

6. **dgx-pixels Integration**
   - [ ] Gallery preview tested
   - [ ] Screen transitions tested
   - [ ] Multiple images tested
   - [ ] Can prevent real bugs

---

## Next Phase Preview

**Phase 4: Bevy ECS Integration**

After Phase 3, we'll implement:
- Bevy App wrapper for TUI testing
- ECS query support (entities, components, resources)
- Update cycle control (frame-by-frame execution)
- bevy_ratatui plugin integration

Phase 4 depends on Phase 3 for:
- Sixel validation in Bevy rendering
- Position tracking for bevy_ratatui widgets
- Complete TUI testing foundation

---

## Resources

### Sixel Specification
- DEC Sixel Graphics: https://www.vt100.net/docs/vt3xx-gp/chapter14.html
- Wikipedia Sixel: https://en.wikipedia.org/wiki/Sixel
- libsixel documentation: https://github.com/saitoha/libsixel

### Related Code
- vtparse DCS handling: https://docs.rs/vtparse/
- image2sixel tools: https://github.com/saitoha/libsixel
- Sixel terminal support: https://www.arewesixelyet.com/

### Testing References
- dgx-pixels requirements: (internal docs)
- Terminal graphics testing: (Phase 0 research)

---

**Document Version**: 1.0
**Last Updated**: 2025-11-21
**Status**: Ready for Implementation
**Estimated Duration**: 2-3 weeks (15 days)
**Priority**: P0 - Critical MVP Feature
