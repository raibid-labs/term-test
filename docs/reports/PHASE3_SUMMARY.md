# Phase 3 Architecture Design - Executive Summary

**Date**: 2025-11-21
**Status**: Design Complete, Ready for Implementation
**Priority**: P0 - Critical MVP Feature
**Estimated Duration**: 2-3 weeks (15 days)

---

## Mission Accomplished

Phase 3 Sixel Position Tracking architecture has been fully designed and is ready for Rust implementation agents.

---

## Deliverables Created

### 1. **PHASE3_CHECKLIST.md** - Comprehensive Implementation Guide
**Location**: `/home/beengud/raibid-labs/mimic/PHASE3_CHECKLIST.md`

**Contents**:
- 7 major implementation sections
- 50+ task breakdowns with code examples
- 3-week timeline with daily breakdown
- Week 1: Parsing & conversion
- Week 2: Validation APIs & fixtures
- Week 3: Documentation & polish

**Key Sections**:
1. Sixel Raster Attribute Parsing (enhance existing stub)
2. Pixel-to-Cell Conversion (new functionality)
3. Validation API Implementation (harness methods)
4. Test Data and Fixtures (real Sixel sequences)
5. dgx-pixels Integration Validation (E2E scenarios)
6. Documentation & Polish (rustdoc, guides, examples)
7. Testing & CI Integration (50+ tests)

### 2. **SIXEL_PARSING_STRATEGY.md** - Algorithm Design
**Location**: `/home/beengud/raibid-labs/mimic/docs/SIXEL_PARSING_STRATEGY.md`

**Contents**:
- Complete Sixel escape sequence format documentation
- Raster attribute parsing algorithm (Pa;Pad;Ph;Pv)
- Fallback dimension estimation strategy
- Pixel-to-cell conversion formulas (6 px/row, 8 px/col)
- Bounds checking mathematics
- Error handling strategy
- Test data generation approach

**Key Algorithms**:
- Parse: Extract width/height from "1;1;100;50" → (100px, 50px)
- Convert: (100px, 60px) → (13 cols, 10 rows) with ceiling rounding
- Validate: Clamp dimensions to (1, 10000) range
- Bounds: Check if (start + size) ≤ (area + area_size)

### 3. **PHASE3_VALIDATION_API.md** - API Specification
**Location**: `/home/beengud/raibid-labs/mimic/docs/PHASE3_VALIDATION_API.md`

**Contents**:
- Complete API surface for Sixel validation
- 4 new harness methods (assert_sixel_within_bounds, get_sixel_at, etc.)
- 5 new SixelCapture methods (sequences_overlapping, bounding_box, etc.)
- Updated SixelRegion with cell dimensions
- Usage patterns for dgx-pixels scenarios
- Error message formats
- Wait condition helpers

**Key APIs**:
```rust
// Harness methods
harness.assert_sixel_within_bounds(area)?;
harness.get_sixel_at(row, col)?;
harness.sixel_count();
harness.verify_sixel_cleared(&previous);

// SixelCapture methods
capture.sequences_overlapping(area);
capture.bounding_box();
capture.total_coverage();
```

### 4. **PHASE3_TEST_STRATEGY.md** - Comprehensive Testing Plan
**Location**: `/home/beengud/raibid-labs/mimic/docs/PHASE3_TEST_STRATEGY.md`

**Contents**:
- Test pyramid: 30 unit + 15 integration + 5 E2E tests
- Unit test cases for all parsing edge cases
- Integration test scenarios with real Sixel sequences
- dgx-pixels E2E validation scenarios
- Test fixture specifications (5 Sixel files)
- CI/CD integration strategy
- Coverage goals (>70% for Phase 3 code)
- Performance benchmarks (< 10µs per validation)

**Test Coverage**:
- Parsing: 10 unit tests (valid, missing, malformed, etc.)
- Conversion: 7 unit tests (rounding, edge cases)
- Bounds: 8 unit tests (within, outside, overlapping)
- Integration: 15 tests (detection, position, validation)
- E2E: 5 dgx-pixels scenarios (Gallery, transitions, thumbnails)

### 5. **ROADMAP_PHASE3_UPDATE.md** - ROADMAP Changes
**Location**: `/home/beengud/raibid-labs/mimic/docs/ROADMAP_PHASE3_UPDATE.md`

**Contents**:
- Phase 2 status update (85% complete)
- Phase 3 detailed section replacement
- Timeline updates
- Architecture document cross-references

---

## Current State Analysis

### What's Already Working ✅

**Phase 1 Infrastructure (100% Complete)**:
- ✅ vtparse integration with DCS callbacks
- ✅ VTActor implementation (dcs_hook, dcs_put, dcs_unhook)
- ✅ Sixel detection (mode == 'q')
- ✅ Cursor position tracking
- ✅ SixelRegion struct with basic fields
- ✅ sixel_regions() accessor in ScreenState
- ✅ SixelSequence and SixelCapture types
- ✅ Basic unit and integration tests

**Phase 2 Progress (85% Complete)**:
- ✅ KeyCode enum and Modifiers bitflags
- ✅ Escape sequence generation
- ✅ send_key() and send_keys() methods
- ✅ Enhanced wait conditions
- 🔶 Async support pending (doesn't block Phase 3)

### What Needs Implementation 🔶

**Parsing Enhancement**:
- 🔶 Enhance parse_raster_attributes() (stub exists at src/screen.rs:120-144)
  - Add fallback defaults
  - Validate dimensions
  - Handle malformed sequences
  - 2-3 hours estimated

**Position Tracking**:
- 🔶 Add pixel-to-cell conversion
  - Implement pixels_to_cells() helper
  - Update SixelRegion with width_cells, height_cells
  - 2-3 hours estimated

**Validation APIs**:
- 🔶 Add harness methods (4 methods, 4-5 hours)
- 🔶 Enhance SixelCapture (5 methods, 2-3 hours)
- 🔶 Add bounds checking to SixelRegion (3-4 hours)

**Testing**:
- 🔶 Create test fixtures (5 Sixel files, 3-4 hours)
- 🔶 Write unit tests (30 tests, 6-8 hours)
- 🔶 Write integration tests (15 tests, 5-6 hours)
- 🔶 Write E2E tests (5 tests, 4-5 hours)

**Documentation**:
- 🔶 Complete rustdoc (3-4 hours)
- 🔶 Write user guide (4-5 hours)
- 🔶 Update examples (3-4 hours)

---

## Dependencies & Blockers

### Internal Dependencies

**Phase 1 (All Complete)** ✅:
- ✅ ScreenState with vtparse
- ✅ DCS callback infrastructure
- ✅ Cursor position tracking
- ✅ TuiTestHarness
- ✅ Basic Sixel types

**Phase 2 (Mostly Complete)** 🔶:
- ✅ Event simulation (send_key) - Phase 3 can use this
- ✅ Wait conditions - Phase 3 needs this
- 🔶 Async support - NOT required for Phase 3 (independent)

**Status**: Phase 3 can start immediately without waiting for Phase 2 async

### External Dependencies

**All Present** ✅:
- ✅ vtparse = "0.7" (already integrated)
- ✅ portable-pty = "0.8" (already integrated)
- ✅ No new crate dependencies needed

### Identified Blockers

**NONE** ✅

All blockers have been resolved:
- ✅ Sixel support validated (vtparse DCS)
- ✅ Cursor tracking implemented
- ✅ Types defined (SixelRegion, SixelCapture)
- ✅ Test infrastructure ready
- ✅ Architecture fully designed

### Risk Assessment

**Overall Risk**: ✅ **LOW**

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Raster attribute parsing edge cases | Medium | Medium | Comprehensive test suite, graceful fallbacks |
| Terminal-specific pixel ratios | Low | Medium | Configurable conversion, document defaults |
| Test fixture generation | Low | Low | Simple solid colors, document format |
| Performance concerns | Low | Low | Benchmarks, optimize if needed |

**All risks have mitigation strategies in place.**

---

## Implementation Path

### Week 1: Core Parsing & Tracking (Days 1-5)

**Monday-Tuesday**: Raster attribute parsing
- Enhance parse_raster_attributes()
- Add fallback defaults
- Validate dimensions
- Write 10 unit tests

**Wednesday**: Pixel-to-cell conversion
- Implement pixels_to_cells()
- Add configuration
- Write 7 unit tests

**Thursday-Friday**: Bounds checking
- Add is_within_cells(), overlaps_cells()
- Update SixelRegion structure
- Write 8 unit tests

**Milestone**: Parsing and tracking foundation complete

### Week 2: APIs & Testing (Days 6-10)

**Monday-Tuesday**: Validation APIs
- Implement 4 harness methods
- Implement 5 SixelCapture methods
- Write integration tests

**Wednesday**: Test fixtures
- Create 5 Sixel test files
- Implement fixture helpers
- Document usage

**Thursday-Friday**: Integration tests
- Expand sixel.rs tests (15 tests)
- Test with real sequences
- Verify all APIs

**Milestone**: All APIs implemented and tested

### Week 3: dgx-pixels & Polish (Days 11-15)

**Monday-Tuesday**: dgx-pixels validation
- Create dgx_pixels_scenarios.rs
- Implement 5 E2E tests
- Verify requirements met

**Wednesday-Thursday**: Documentation
- Complete rustdoc (100%)
- Write SIXEL_TESTING.md guide
- Update examples

**Friday**: CI & polish
- Integrate with CI pipeline
- Run quality checks
- Performance benchmarks
- Final bug fixes

**Milestone**: Phase 3 complete and shipped

---

## Success Metrics

### Technical Metrics

**Code Quality**:
- [ ] All unit tests pass (>30 tests)
- [ ] All integration tests pass (>15 tests)
- [ ] All E2E tests pass (>5 dgx-pixels scenarios)
- [ ] Code coverage >70% for Phase 3 code
- [ ] All clippy warnings resolved
- [ ] Performance < 10µs per validation

**Documentation**:
- [ ] 100% API rustdoc coverage
- [ ] User guide complete (SIXEL_TESTING.md)
- [ ] All examples working
- [ ] dgx-pixels patterns documented

**Integration**:
- [ ] CI pipeline green
- [ ] No flaky tests
- [ ] Benchmarks meet targets
- [ ] Examples in README

### MVP Requirements

**dgx-pixels Testing Capabilities**:
- [ ] Can detect Sixel graphics in terminal output
- [ ] Can verify Sixel position (row, col)
- [ ] Can verify Sixel within preview area bounds
- [ ] Can detect Sixel overflow outside bounds
- [ ] Can detect Sixel clearing on screen transitions
- [ ] Can test Gallery preview area
- [ ] Can test multiple thumbnail images
- [ ] Can prevent real Sixel positioning bugs

**Developer Experience**:
- [ ] APIs are intuitive and ergonomic
- [ ] Error messages are actionable
- [ ] Examples cover common patterns
- [ ] Documentation is clear and helpful
- [ ] Tests are fast and reliable

---

## Next Actions

### For Studio Producer

1. **Review this summary** - Validate architecture completeness
2. **Approve implementation** - Confirm ready for Rust agents
3. **Assign implementation agents** - Match tasks to specialists
4. **Set up coordination** - Establish checkpoints and reviews

### For Implementation Agents

1. **Read PHASE3_CHECKLIST.md** - Your detailed implementation guide
2. **Review architecture docs** - Understand parsing, APIs, testing
3. **Start with Week 1 tasks** - Parsing and conversion foundation
4. **Follow test-driven approach** - Write tests, then implementation
5. **Track progress** - Update checklist as tasks complete

### Immediate First Steps

**Day 1 Morning**: Begin parsing enhancement
- File: `src/screen.rs`
- Method: `parse_raster_attributes()` (lines 120-144)
- Goal: Add fallback defaults and validation
- Tests: Create test file with 10 parsing tests
- Duration: 3-4 hours

**Day 1 Afternoon**: Pixel-to-cell conversion
- File: `src/screen.rs` (new helper methods)
- Goal: Implement `pixels_to_cells()` with configuration
- Tests: 7 conversion unit tests
- Duration: 2-3 hours

**Day 2**: Bounds checking and region updates
- Files: `src/screen.rs`, `src/sixel.rs`
- Goal: Add cell dimensions and bounds methods
- Tests: 8 bounds checking tests
- Duration: 3-4 hours

---

## Key Design Decisions

### 1. Graceful Degradation
**Decision**: Use fallback defaults (100x100) for missing raster attributes
**Rationale**: Some Sixel sequences may not include dimensions, tests should still work
**Impact**: More robust parsing, better error handling

### 2. Cell-Based Validation
**Decision**: Validate using terminal cells, not pixels
**Rationale**: Terminal layout is in cells, more intuitive for developers
**Impact**: Need pixel-to-cell conversion, but clearer API

### 3. Ceiling Rounding
**Decision**: Round up when converting pixels to cells
**Rationale**: Ensures full graphic is accounted for in bounds checking
**Impact**: Conservative bounds checking, won't miss overflows

### 4. Comprehensive Test Coverage
**Decision**: 50+ tests across unit/integration/E2E levels
**Rationale**: Sixel parsing has many edge cases, MVP feature must be reliable
**Impact**: Higher initial effort, but robust implementation

### 5. Real Test Fixtures
**Decision**: Generate actual Sixel files for testing
**Rationale**: Validates full pipeline with real data
**Impact**: More realistic tests, catches parsing issues

---

## Coordination Notes

### Parallel Work Opportunities

**Can work in parallel**:
- Parsing enhancement (src/screen.rs)
- Validation APIs (src/harness.rs, src/sixel.rs)
- Test fixtures (tests/fixtures/sixel/)
- Documentation (docs/, examples/)

**Must be sequential**:
1. Parsing → Conversion → Validation APIs
2. Fixtures → Integration tests → E2E tests
3. Implementation → Documentation

### Handoff Points

**Week 1 → Week 2**: Parsing foundation complete
- Deliverable: Working parse_raster_attributes() and pixels_to_cells()
- Validation: 25 unit tests passing
- Enables: API implementation

**Week 2 → Week 3**: APIs and fixtures ready
- Deliverable: All validation APIs implemented
- Validation: 40 tests passing
- Enables: dgx-pixels scenarios

**Week 3 → Phase 4**: Phase 3 complete
- Deliverable: All tests passing, docs complete
- Validation: 50+ tests, >70% coverage, CI green
- Enables: Bevy ECS integration

---

## Resources for Implementation

### Documentation References

**Architecture Documents**:
- /home/beengud/raibid-labs/mimic/PHASE3_CHECKLIST.md
- /home/beengud/raibid-labs/mimic/docs/SIXEL_PARSING_STRATEGY.md
- /home/beengud/raibid-labs/mimic/docs/PHASE3_VALIDATION_API.md
- /home/beengud/raibid-labs/mimic/docs/PHASE3_TEST_STRATEGY.md

**Existing Code**:
- /home/beengud/raibid-labs/mimic/src/sixel.rs (SixelSequence, SixelCapture)
- /home/beengud/raibid-labs/mimic/src/screen.rs (ScreenState, SixelRegion, parse stub)
- /home/beengud/raibid-labs/mimic/examples/sixel_test.rs (usage example)
- /home/beengud/raibid-labs/mimic/tests/integration/sixel.rs (basic tests)

**External References**:
- DEC Sixel Specification: https://www.vt100.net/docs/vt3xx-gp/chapter14.html
- vtparse documentation: https://docs.rs/vtparse/
- libsixel: https://github.com/saitoha/libsixel

### Code Locations

**Files to Modify**:
- src/screen.rs - Enhance parsing, add conversion
- src/harness.rs - Add validation methods
- src/sixel.rs - Add query methods to SixelCapture
- src/error.rs - Ensure SixelValidation error exists

**Files to Create**:
- tests/fixtures/sixel/README.md
- tests/fixtures/sixel/*.sixel (5 files)
- tests/helpers/sixel_fixtures.rs
- tests/integration/dgx_pixels_scenarios.rs
- docs/SIXEL_TESTING.md
- examples/dgx_pixels_preview.rs
- benches/sixel_benchmarks.rs

---

## Final Checklist

### Architecture Design ✅
- [x] Implementation checklist created (PHASE3_CHECKLIST.md)
- [x] Parsing strategy documented (SIXEL_PARSING_STRATEGY.md)
- [x] API specification complete (PHASE3_VALIDATION_API.md)
- [x] Test strategy designed (PHASE3_TEST_STRATEGY.md)
- [x] ROADMAP updated (ROADMAP_PHASE3_UPDATE.md)
- [x] Dependencies identified (this document)
- [x] Blockers resolved (NONE)
- [x] Timeline estimated (2-3 weeks)
- [x] Risk assessment complete (LOW risk)

### Ready for Implementation ✅
- [x] All Phase 1 dependencies met
- [x] Phase 2 dependencies met (async not needed)
- [x] No external blockers
- [x] Infrastructure in place
- [x] Design decisions made
- [x] Implementation path clear
- [x] Test strategy defined
- [x] Success metrics established

### Coordination Ready ✅
- [x] Task breakdown complete
- [x] Work packages defined
- [x] Parallel work identified
- [x] Handoff points clear
- [x] Resources documented
- [x] Code locations specified

---

## Conclusion

Phase 3 Sixel Position Tracking is **fully designed and ready for implementation**.

**Status**: 🎯 Ready to Start
**Risk**: ✅ Low
**Blockers**: None
**Dependencies**: All met
**Timeline**: 2-3 weeks (15 days)
**Estimated Effort**: 60-80 hours total

The architecture provides:
- Complete implementation guide (PHASE3_CHECKLIST.md)
- Detailed algorithms (SIXEL_PARSING_STRATEGY.md)
- Full API specification (PHASE3_VALIDATION_API.md)
- Comprehensive test plan (PHASE3_TEST_STRATEGY.md)
- Clear success metrics
- Week-by-week timeline

**Recommendation**: Begin implementation immediately.

---

**Document Version**: 1.0
**Date**: 2025-11-21
**Prepared by**: Studio Producer Coordinator
**Status**: Final - Ready for Handoff
