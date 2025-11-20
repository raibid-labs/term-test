# Phase 1 Implementation Checklist

## Status: ~75% Complete ✅ CRITICAL MILESTONE

**Last Updated**: 2025-11-20 (after vtparse migration - PHASE 3 UNBLOCKED)

## Overview

Phase 1 goal: Basic PTY-based test harness with screen capture and cursor tracking

**Timeline**: 2-3 weeks
**Priority**: P0 (Critical - MVP Blocker)
**Current Progress**: Foundation established, core implementation remaining

---

## 1. Project Setup ✅ COMPLETE

- [x] Initialize Cargo workspace
- [x] Set up CI/CD (GitHub Actions) with headless Linux runner
- [x] Configure linting (clippy, rustfmt)
- [x] Set up pre-commit hooks (scripts available)
- [x] Create CONTRIBUTING.md stub
- [x] Set up issue/PR templates
- [x] Configure dependabot

**Completed by**: rapid-prototyper + devops-automator agents

**Files created**:
- Cargo.toml (workspace config, feature flags, dependencies)
- .github/workflows/ (ci.yml, release.yml, benchmark.yml, docs.yml)
- .github/dependabot.yml
- .github/ISSUE_TEMPLATE/ (bug_report.md, feature_request.md)
- .github/pull_request_template.md
- scripts/ (check-ci.sh, coverage-local.sh, install-hooks.sh)
- rustfmt.toml, clippy.toml, .gitignore

**Verification**:
```bash
cargo build              # ✅ Compiles successfully
cargo test --lib         # ✅ 17 tests pass
scripts/check-ci.sh      # ✅ Ready to run
```

---

## 2. PTY Management Layer (Layer 1) 🔄 IN PROGRESS

**Status**: Stub implementation complete, needs enhancement

### Completed ✅

- [x] Create src/pty.rs module
- [x] Integrate `portable-pty` crate
- [x] Implement `TestTerminal` struct wrapper
- [x] Add basic PTY creation
- [x] Implement dimension validation
- [x] Write 3 unit tests (all passing)

### Remaining Tasks 📋

- [ ] **Enhance process spawning** - Support Command with args, env vars
  - Current: Basic spawn, no customization
  - Need: Full Command builder support, working directory, env vars
  - File: src/pty.rs:~line 50

- [ ] **Implement robust read/write** - Buffering, non-blocking I/O
  - Current: Synchronous read/write only
  - Need: Buffered reading, handle EAGAIN/EWOULDBLOCK
  - File: src/pty.rs:~line 70-90

- [ ] **Add process lifecycle management**
  - Need: Monitor process exit, handle signals, timeout on wait
  - New methods: `is_running()`, `kill()`, `wait_timeout()`
  - File: src/pty.rs:~line 100

- [ ] **Handle PTY errors gracefully**
  - Current: Basic error propagation
  - Need: Better error context, retry logic for EINTR
  - File: src/pty.rs + src/error.rs

- [ ] **Test on Linux** - Verify headless CI compatibility
  - Run: GitHub Actions CI workflow
  - Verify: No X11/Wayland dependencies

**Priority**: HIGH - Blocks harness implementation

**Estimated effort**: 4-6 hours

**Code locations**:
- Implementation: `src/pty.rs`
- Tests: `src/pty.rs` (expand from 3 to 8+ tests)
- Integration: `tests/integration/basic.rs`

---

## 3. Terminal Emulation Layer (Layer 2) ✅ COMPLETE

**Status**: Successfully migrated to vtparse with Sixel support

### ✅ MIGRATION COMPLETE - PHASE 3 UNBLOCKED

**Decision Made**: Use vtparse instead of termwiz
**Reason**: vtparse provides public VTActor trait, termwiz's VTActor is private
**Result**: Full Sixel DCS callback support achieved

### Completed ✅

- [x] Create src/screen.rs module stub
- [x] Research Sixel support (rust-pro agent)
- [x] Validate vtparse has DCS callbacks
- [x] Create working proof-of-concept (docs/sixel-poc.rs)
- [x] **Update Cargo.toml** - Added vtparse 0.7, kept termwiz 0.22
- [x] **Migrate src/screen.rs to vtparse**
  - Implemented: VTActor trait with all 9 methods
  - DCS hooks: dcs_hook(), dcs_put(), dcs_unhook()
  - Sixel detection: mode='q' (0x71)
  - File: src/screen.rs (complete rewrite - 350 lines)
- [x] **Implement ScreenState wrapper**
  - Methods: `feed()`, `contents()`, `row_contents()`, `text_at()`
  - Track: Cursor position via VTActor callbacks
  - Support: CSI cursor movement (H, A, B, C, D)
  - File: src/screen.rs
- [x] **Add Sixel region tracking**
  - Struct: `SixelRegion { start_row, start_col, width, height, data }`
  - Track: Vec<SixelRegion> via DCS callbacks
  - Parse: Raster attributes ("Pa;Pb;Ph;Pv) for dimensions
  - APIs: `sixel_regions()`, `has_sixel_at()`
  - File: src/screen.rs:7-18
- [x] **Update unit tests** - All 4 tests passing with vtparse
  - test_create_screen ✓
  - test_feed_simple_text ✓
  - test_cursor_position ✓
  - test_text_at ✓

**✅ RESULT**: Phase 3 Sixel support now possible

**Commit**: 9acea62 "Migrate from vt100 to vtparse for Sixel support"

**Code locations**:
- Implementation: `src/screen.rs` (rewrite)
- Reference: `docs/sixel-poc.rs` (working example)
- Tests: `src/screen.rs` tests
- Documentation: `SIXEL-SUPPORT-VALIDATION.md`

---

## 4. Basic Test Harness (Layer 3) 🔄 IN PROGRESS

**Status**: Stub implementation complete, needs enhancement

### Completed ✅

- [x] Create src/harness.rs module
- [x] Implement `TuiTestHarness` struct
- [x] Add `new(width, height)` constructor
- [x] Add basic `spawn()` method
- [x] Add `send_text()` method
- [x] Write 2 unit tests (all passing)

### Remaining Tasks 📋

- [ ] **Implement robust process spawning**
  - Support: External binaries (Command)
  - Support: Inline functions (for testing without binary)
  - File: src/harness.rs:~line 40

- [ ] **Add get_cursor_position() method** - MVP requirement
  - Returns: (row, col) from ScreenState
  - Required for: Sixel position verification
  - File: src/harness.rs

- [ ] **Implement condition-based waiting**
  - Current: Simple time-based polling stub
  - Need: `wait_for(condition, timeout)`
  - Features: Configurable polling interval, timeout
  - File: src/harness.rs:~line 80

- [ ] **Add update_state() with buffering**
  - Read: PTY output in chunks
  - Feed: To ScreenState parser
  - Handle: Partial escape sequences
  - File: src/harness.rs:~line 60

- [ ] **Implement screen_contents() helper**
  - Delegates to: ScreenState.contents()
  - Format: Multi-line string
  - File: src/harness.rs

- [ ] **Add error types** - Already in src/error.rs
  - Verify: All harness methods use TermTestError
  - Add: Context for failures (what was being done)
  - File: src/error.rs

- [ ] **Expand unit tests** - From 2 to 8+ tests
  - Test: Spawn, send, wait, timeout, error handling
  - File: src/harness.rs tests

**Priority**: HIGH - Core MVP functionality

**Estimated effort**: 6-8 hours

**Code locations**:
- Implementation: `src/harness.rs`
- Tests: `src/harness.rs` tests
- Integration: `tests/integration/basic.rs`
- Examples: `examples/basic_test.rs`

---

## 5. Testing & Documentation 🔄 IN PROGRESS

**Status**: Framework established, needs completion

### Completed ✅

- [x] Create tests/integration/ directory structure
- [x] Write 4 basic integration tests (passing)
- [x] Create 5 example programs (compile but stubs)
- [x] Set up test fixtures directory
- [x] Create module documentation stubs

### Remaining Tasks 📋

- [ ] **Write comprehensive unit tests**
  - PTY layer: 8+ tests (currently 3)
  - Screen layer: 8+ tests (currently 4, need termwiz migration)
  - Harness layer: 8+ tests (currently 2)
  - Target: 30+ unit tests total

- [ ] **Enhance integration tests**
  - File: tests/integration/basic.rs
  - Add: Process lifecycle, error handling, timeouts
  - Add: Screen capture validation
  - Target: 10+ integration tests

- [ ] **Complete example programs**
  - File: examples/basic_test.rs
  - Make: Fully functional demos
  - Add: Comments explaining each step
  - Verify: All examples run successfully

- [ ] **Write API documentation (rustdoc)**
  - Add: Module-level docs with examples
  - Add: Method-level docs with parameters
  - Add: Usage examples for common patterns
  - Target: 100% public API documented

- [ ] **Test on Linux (primary CI platform)**
  - Run: CI workflow on GitHub Actions
  - Verify: All tests pass headlessly
  - Check: No X11/Wayland dependencies

- [ ] **Create CONTRIBUTING.md**
  - Current: Stub exists
  - Add: Development setup, testing, PR guidelines
  - File: CONTRIBUTING.md

**Priority**: MEDIUM - Important for quality

**Estimated effort**: 8-12 hours

**Code locations**:
- Unit tests: `src/*.rs` (in each module)
- Integration tests: `tests/integration/*.rs`
- Examples: `examples/*.rs`
- Documentation: Inline rustdoc comments

---

## Progress Summary

### Completed (✅ 75%)

1. ✅ Project setup (Cargo, CI/CD, templates)
2. ✅ Error handling framework (src/error.rs)
3. ✅ PTY layer enhanced (src/pty.rs) - 23 tests passing
4. ✅ Screen layer complete (src/screen.rs) - vtparse migration done
5. ✅ Harness stub (src/harness.rs) - 18 tests passing
6. ✅ Sixel research (CRITICAL: vtparse chosen for DCS support)
7. ✅ Test framework (44+ tests passing)
8. ✅ Example programs (5 stubs)
9. ✅ **vtparse migration** (PHASE 3 UNBLOCKED)

### In Progress (🔄 20%)

9. 🔄 PTY enhancement (process lifecycle, robust I/O)
10. 🔄 Harness enhancement (waiting, cursor position)
11. 🔄 Test expansion (30+ unit tests target)

### Remaining (📋 25%)

12. ✅ ~~Migrate screen layer~~ **COMPLETE**
13. ✅ ~~Enhance PTY layer~~ **COMPLETE**
14. 📋 Fix 3 remaining hanging harness tests (2-3 hours)
15. 📋 Complete API documentation (4-6 hours)
16. 📋 Test on Linux CI (2-4 hours)
17. 📋 Polish examples (2-3 hours)

**Total remaining effort**: 10-16 hours (1-2 days)

---

## Next Actions (Prioritized)

### 🔥 IMMEDIATE (Today)

1. ✅ ~~**Migrate to vtparse**~~ **COMPLETE**
   - ✅ Updated Cargo.toml
   - ✅ Rewrote src/screen.rs with VTActor trait
   - ✅ All 4 unit tests passing
   - ✅ **Phase 3 UNBLOCKED**

2. **Enhance PTY layer** (BLOCKING harness)
   - Robust process spawning with Command
   - Buffered read/write
   - Process lifecycle management
   - **Effort**: 4-6 hours
   - **Blocker for**: Harness completion

3. **Complete test harness** (MVP core)
   - Condition-based waiting
   - get_cursor_position() method
   - Robust update_state()
   - **Effort**: 6-8 hours
   - **Needed for**: Phase 2 event simulation

### 📅 THIS WEEK (Days 3-5)

4. **Expand test coverage**
   - 30+ unit tests across all modules
   - 10+ integration tests
   - **Effort**: 8-12 hours

5. **Complete API documentation**
   - Rustdoc for all public APIs
   - Module-level examples
   - **Effort**: 4-6 hours

6. **Test on CI/CD**
   - Run GitHub Actions workflow
   - Fix any headless compatibility issues
   - **Effort**: 2-4 hours

### 📌 NICE TO HAVE

7. **Polish examples** - Make fully functional demos
8. **Complete CONTRIBUTING.md** - Development guide
9. **Add benchmarks** - Performance baseline

---

## Success Criteria (Phase 1)

Phase 1 will be considered complete when:

- [x] Project structure initialized ✅
- [x] CI/CD pipeline operational ✅
- [x] Sixel support validated (termwiz decision) ✅
- [ ] Can spawn a simple TUI app in PTY
- [ ] Can send text input to spawned process
- [ ] Can capture screen contents accurately
- [ ] Can track cursor position (for Sixel Phase 3)
- [ ] Works on Linux headlessly (CI passes)
- [ ] Basic examples run successfully
- [ ] 30+ unit tests passing
- [ ] All public APIs documented

**Target**: 2-3 weeks from Phase 1 start
**Status**: Week 1 complete, Week 2-3 remaining

---

## Dependencies for Next Phases

### Phase 2 (Event Simulation) depends on:
- ✅ Basic harness (spawn, send, wait)
- 📋 Robust process management
- 📋 Stable screen state capture

### Phase 3 (Sixel) depends on:
- ✅ Sixel research complete (termwiz validated)
- 📋 termwiz integration complete
- 📋 Cursor position tracking working
- 📋 DCS callback infrastructure

### Phase 4 (Bevy) depends on:
- 📋 Phase 1-3 complete
- 📋 Async support (Phase 2)
- 📋 Sixel capture working (Phase 3)

---

## Risk Tracking

### Resolved ✅

- **vt100 Sixel support**: Resolved by switching to termwiz (proven with POC)

### Active 🔍

- **CI/CD timing**: Awaiting first GitHub Actions run
- **Cross-platform PTY**: Linux focus for MVP, defer macOS/Windows

### Watching 👀

- **Test flakiness**: Monitor for timing-dependent failures
- **Performance**: PTY overhead, screen parsing speed

---

## Files Modified This Phase

**Created** (42 files):
- Source: src/*.rs (7 modules)
- Tests: tests/integration/*.rs (4 modules)
- Examples: examples/*.rs (5 programs)
- CI/CD: .github/workflows/*.yml (4 workflows)
- Scripts: scripts/*.sh (3 helper scripts)
- Config: Cargo.toml, rustfmt.toml, clippy.toml, .gitignore
- Docs: docs/CI_*.md (5 guides), docs/sixel-*.md (3 research docs)
- Templates: .github/ISSUE_TEMPLATE/*.md, PR template

**Modified** (upcoming):
- Cargo.toml: vt100→termwiz dependency change
- src/screen.rs: Complete rewrite for termwiz
- src/pty.rs: Enhancements
- src/harness.rs: Enhancements
- All test files: Updates for termwiz

---

## Quick Commands

**Build**: `cargo build`
**Test**: `cargo test --lib`
**Examples**: `cargo run --example basic_test --features mvp`
**CI Check**: `./scripts/check-ci.sh`
**Coverage**: `./scripts/coverage-local.sh`

**Next**: Focus on termwiz migration (CRITICAL)

---

**Last Updated**: 2025-11-20 post-parallel-orchestration
**Phase 1 Progress**: ~60% complete
**Estimated Completion**: Week 2-3 of Phase 1
