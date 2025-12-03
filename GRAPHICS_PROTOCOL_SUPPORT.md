# Graphics Protocol Support

This document describes the unified graphics protocol detection system in `ratatui-testlib`, which supports Sixel, Kitty, and iTerm2 image protocols.

## Overview

The library now provides comprehensive support for testing TUI applications that use graphics protocols beyond just Sixel. The unified API in the `graphics` module allows you to:

- Detect and track graphics from multiple protocols simultaneously
- Filter graphics by protocol type
- Validate positioning and bounds
- Test graphics clearing on screen transitions
- Perform protocol-specific assertions

## Supported Protocols

### 1. Sixel (DCS-based)

**Escape Sequence:** `ESC P q ... ESC \`

Sixel is a bitmap graphics format supported by terminals like XTerm, WezTerm, and foot. Sequences are wrapped in DCS (Device Control String).

**Raster Attributes Format:** `"Pan;Pad;Ph;Pv`
- `Pan`, `Pad`: Pixel aspect ratio
- `Ph`: Horizontal pixel dimension (width)
- `Pv`: Vertical pixel dimension (height)

**Example:**
```
ESC P q "1;1;100;50 #0;2;100;100;100 #0~ ESC \
```

### 2. Kitty Graphics Protocol (APC-based)

**Escape Sequence:** `ESC _ G <control_data> ; <payload> ESC \`

Kitty's advanced graphics protocol uses APC (Application Program Command) sequences with key-value pairs.

**Control Data Format:**
- `w=<width>` - width in pixels
- `h=<height>` - height in pixels
- `c=<cols>` - width in terminal cells
- `r=<rows>` - height in terminal cells

**Example:**
```
ESC _ G w=200,h=100,f=24,a=T ; <base64_payload> ESC \
```

### 3. iTerm2 Inline Images (OSC-based)

**Escape Sequence:** `ESC ] 1337 ; File = <params> : <base64_data> BEL`

iTerm2 uses OSC (Operating System Command) sequences for inline images.

**Parameters:**
- `width=<n>` or `width=<n>px` - width
- `height=<n>` or `height=<n>px` - height
- `width=auto` or `height=auto` - automatic sizing
- `inline=<0|1>` - display inline
- `preserveAspectRatio=<0|1>` - preserve aspect ratio

**Example:**
```
ESC ] 1337;File=width=30;height=15:SGVsbG8= BEL
```

## API Documentation

### Core Types

#### `GraphicsProtocol`

Enum identifying the graphics protocol:

```rust
pub enum GraphicsProtocol {
    Sixel,   // DCS-based Sixel
    Kitty,   // APC-based Kitty graphics
    ITerm2,  // OSC 1337-based iTerm2 images
}
```

Methods:
- `name() -> &'static str` - Human-readable protocol name
- `escape_prefix() -> &'static str` - Escape sequence prefix

#### `GraphicsRegion`

Represents a captured graphics region:

```rust
pub struct GraphicsRegion {
    pub protocol: GraphicsProtocol,
    pub position: (u16, u16),           // (row, col) cursor position
    pub bounds: (u16, u16, u16, u16),   // (row, col, width, height) in cells
    pub raw_data: Vec<u8>,              // Raw escape sequence bytes
}
```

Methods:
- `is_within(area) -> bool` - Check if entirely within area
- `overlaps(area) -> bool` - Check if overlaps with area

#### `GraphicsCapture`

Collection of captured graphics with query methods:

```rust
pub struct GraphicsCapture {
    regions: Vec<GraphicsRegion>,
}
```

Methods:
- `new() -> Self` - Create empty capture
- `from_screen_state(screen: &ScreenState) -> Self` - Extract from screen state
- `regions() -> &[GraphicsRegion]` - Get all regions
- `is_empty() -> bool` - Check if empty
- `regions_in_area(area) -> Vec<&GraphicsRegion>` - Filter by area
- `regions_outside_area(area) -> Vec<&GraphicsRegion>` - Inverse filter
- `by_protocol(protocol) -> Vec<&GraphicsRegion>` - Filter by protocol
- `count_by_protocol(protocol) -> usize` - Count by protocol
- `assert_all_within(area) -> Result<()>` - Validate positioning
- `assert_protocol_exists(protocol) -> Result<()>` - Validate presence
- `differs_from(&other) -> bool` - Compare captures

### Screen State Extensions

The `ScreenState` struct now tracks all three protocols:

```rust
impl ScreenState {
    // Sixel regions
    pub fn sixel_regions(&self) -> &[SixelRegion];
    pub fn sixel_regions_mut(&mut self) -> &mut Vec<SixelRegion>;

    // Kitty regions
    pub fn kitty_regions(&self) -> &[KittyRegion];
    pub fn kitty_regions_mut(&mut self) -> &mut Vec<KittyRegion>;

    // iTerm2 regions
    pub fn iterm2_regions(&self) -> &[ITerm2Region];
    pub fn iterm2_regions_mut(&mut self) -> &mut Vec<ITerm2Region>;
}
```

### Backwards Compatibility

The existing `sixel` module continues to work unchanged. It now internally uses the unified graphics system:

```rust
use ratatui_testlib::sixel::{SixelCapture, SixelSequence};

// Still works as before
let capture = SixelCapture::from_screen_state(&screen);
let sequences = capture.sequences();
```

For new code, prefer the unified API:

```rust
use ratatui_testlib::graphics::{GraphicsCapture, GraphicsProtocol};

let capture = GraphicsCapture::from_screen_state(&screen);
let sixel_graphics = capture.by_protocol(GraphicsProtocol::Sixel);
```

## Usage Examples

### Basic Detection

```rust
use ratatui_testlib::graphics::{GraphicsCapture, GraphicsProtocol};
use ratatui_testlib::ScreenState;

let screen = ScreenState::new(80, 24);
// ... render graphics ...

let capture = GraphicsCapture::from_screen_state(&screen);

// Count by protocol
let sixel_count = capture.count_by_protocol(GraphicsProtocol::Sixel);
let kitty_count = capture.count_by_protocol(GraphicsProtocol::Kitty);
let iterm2_count = capture.count_by_protocol(GraphicsProtocol::ITerm2);

println!("Detected: {} Sixel, {} Kitty, {} iTerm2",
         sixel_count, kitty_count, iterm2_count);
```

### Protocol-Specific Filtering

```rust
// Get only Kitty graphics
let kitty_graphics = capture.by_protocol(GraphicsProtocol::Kitty);
for region in kitty_graphics {
    println!("Kitty graphic at ({}, {}), size {}x{}",
             region.position.0, region.position.1,
             region.bounds.2, region.bounds.3);
}
```

### Area-Based Validation

```rust
// Define a preview area (row, col, width, height)
let preview_area = (5, 5, 30, 15);

// Check all graphics are within bounds
capture.assert_all_within(preview_area)?;

// Or get specific lists
let inside = capture.regions_in_area(preview_area);
let outside = capture.regions_outside_area(preview_area);

assert_eq!(outside.len(), 0, "No graphics should be outside preview");
```

### Protocol Existence Checks

```rust
// Verify specific protocols are being used
capture.assert_protocol_exists(GraphicsProtocol::Sixel)?;

// Or check without panicking
if capture.count_by_protocol(GraphicsProtocol::Kitty) > 0 {
    println!("Kitty graphics detected!");
}
```

### Testing Graphics Clearing

```rust
let capture_before = GraphicsCapture::from_screen_state(&screen);

// ... trigger screen transition ...

let capture_after = GraphicsCapture::from_screen_state(&screen);

// Verify graphics were cleared
assert!(capture_before.differs_from(&capture_after),
        "Graphics should change on transition");
```

### Mixed Protocol Testing

```rust
// Test app that uses multiple protocols
let capture = GraphicsCapture::from_screen_state(&screen);

// Check each protocol separately
for protocol in [GraphicsProtocol::Sixel,
                 GraphicsProtocol::Kitty,
                 GraphicsProtocol::ITerm2] {
    let regions = capture.by_protocol(protocol);
    println!("{}: {} graphics", protocol.name(), regions.len());

    for region in regions {
        assert!(region.is_within(screen_bounds),
                "{} graphic outside bounds", protocol.name());
    }
}
```

## Implementation Details

### Terminal State Tracking

The `TerminalState` (internal to `ScreenState`) now maintains separate vectors for each protocol:

```rust
struct TerminalState {
    sixel_regions: Vec<SixelRegion>,
    kitty_regions: Vec<KittyRegion>,
    iterm2_regions: Vec<ITerm2Region>,
    // ... other fields
}
```

### VTActor Implementation

The vtparse callbacks handle each protocol:

- **DCS sequences** (`dcs_hook`, `dcs_put`, `dcs_unhook`) - Sixel detection
- **APC sequences** (`apc_dispatch`) - Kitty graphics detection
- **OSC sequences** (`osc_dispatch`) - iTerm2 inline image detection

### Dimension Parsing

Each protocol has custom dimension parsing:

- **Sixel**: `parse_raster_attributes()` - extracts Ph, Pv from raster attributes
- **Kitty**: `parse_kitty_dimensions()` - extracts w=, h= from control data
- **iTerm2**: `parse_iterm2_dimensions()` - extracts width=, height= from params

### Cell Conversion

Graphics dimensions are converted to terminal cells:

- **Sixel/Kitty**: 8 pixels/column, 6 pixels/row (standard ratios)
- **iTerm2**: Dimensions already in cells (no conversion needed)

## Testing

The `graphics` module includes comprehensive tests:

- Protocol identification and formatting
- Region bounds checking and overlap detection
- Capture filtering and protocol-specific queries
- Screen state integration
- Mixed protocol handling
- Area-based validation

Run tests with:

```bash
cargo test --lib --features sixel graphics::
```

## Examples

See `examples/graphics_detection.rs` for a complete demonstration:

```bash
cargo run --example graphics_detection --features sixel
```

This example shows:
- Creating mock graphics regions
- Protocol-specific filtering
- Area-based validation
- Overlap detection
- Existence assertions

## Migration Guide

### From Sixel-Only Code

**Before:**
```rust
use ratatui_testlib::sixel::{SixelCapture, SixelSequence};

let capture = SixelCapture::from_screen_state(&screen);
let sequences = capture.sequences();
capture.assert_all_within(preview_area)?;
```

**After (unified API):**
```rust
use ratatui_testlib::graphics::{GraphicsCapture, GraphicsProtocol};

let capture = GraphicsCapture::from_screen_state(&screen);
let sixel_regions = capture.by_protocol(GraphicsProtocol::Sixel);
capture.assert_all_within(preview_area)?;
```

The old API still works for backwards compatibility, but new code should use the unified API for better protocol support.

## Future Enhancements

Potential future additions:

1. **ReGIS Protocol** - DEC ReGIS vector graphics
2. **Unicode Halfblocks** - Text-based graphics using block elements
3. **DRCS** - Dynamically Redefinable Character Sets
4. **Actual Graphics Detection** - Parse real graphics from PTY output (not just mock regions)
5. **Image Comparison** - Decode and compare actual image content
6. **Performance Metrics** - Track graphics rendering performance

## References

- [Sixel Graphics](https://en.wikipedia.org/wiki/Sixel)
- [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/)
- [iTerm2 Inline Images](https://iterm2.com/documentation-images.html)
- [Are We Sixel Yet?](https://www.arewesixelyet.com/)
- [VT100 Control Sequences](https://vt100.net/docs/vt510-rm/)
