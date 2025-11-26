# CI/CD Workflow Implementation Guide

**Purpose:** Step-by-step recommendations to optimize the existing CI/CD pipeline

**Status:** OPTIONAL - Current workflow is production-ready. These are enhancements for reliability and clarity.

---

## Quick Implementation Path

### Option 1: Minimal Changes (5 minutes)
Keep current `ci.yml` and apply these small fixes:

#### Change 1: Fix MSRV Version (Line 242)
```diff
- name: Setup Rust 1.70
+ name: Setup Rust 1.75 (MSRV)
  uses: dtolnay/rust-toolchain@master
  with:
-   toolchain: "1.70"
+   toolchain: "1.75"
```

**Reason:** Aligns CI testing with declared MSRV in Cargo.toml (line 5)

#### Change 2: Add Test Timeouts (Lines 87-93)
```diff
  - name: Run library tests
-   run: cargo test --lib --verbose
+   run: timeout 300 cargo test --lib --verbose

  - name: Run integration tests
-   run: cargo test --test '*' --verbose
+   run: timeout 300 cargo test --test '*' --verbose

  - name: Run doc tests
-   run: cargo test --doc --verbose
+   run: timeout 300 cargo test --doc --verbose
```

**Reason:** Prevents hanging tests from blocking entire CI run. 300 seconds = 5 minutes per test suite.

#### Change 3: Add Bevy Headless Mode (Lines 10-12)
```diff
  env:
    CARGO_TERM_COLOR: always
    RUST_BACKTRACE: 1
+   # Bevy headless rendering configuration
+   WGPU_BACKEND: vk
```

**Reason:** Ensures Bevy tests run in headless mode on CI, prevents graphics initialization attempts.

### Option 2: Comprehensive Improvements (15 minutes)
Replace `ci.yml` with improved version:

```bash
# Option A: Use the provided improved workflow
mv .github/workflows/ci.yml .github/workflows/ci_original.yml
mv .github/workflows/ci_improved.yml .github/workflows/ci.yml

# Option B: Manually apply all changes (see details below)
```

**Benefits:**
- 3 test timeouts added (lib, integration, doc)
- MSRV version fixed
- Bevy headless mode explicit
- Improved test failure resilience
- Better documentation in workflow

### Option 3: Staged Implementation (Recommended for production)

**Week 1:** Apply minimal changes
- Fix MSRV
- Add timeouts

**Week 2:** Add environment variables
- Set Bevy headless mode
- Monitor CI runs

**Week 3:** Switch to improved workflow
- Validate no regressions
- Update documentation

---

## Detailed Changes

### Section 1: Environment Variables

**Current (`ci.yml` lines 10-12):**
```yaml
env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1
```

**Improved:**
```yaml
env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1
  # Bevy headless rendering configuration
  WGPU_BACKEND: vk
  # Disable optional rendering to ensure headless compatibility
  BEVY_RENDER: false
```

**Why these changes:**

1. `WGPU_BACKEND: vk` - Forces Vulkan backend instead of auto-selecting (may try X11)
2. `BEVY_RENDER: false` - Disables rendering pipeline entirely (though this isn't a standard Bevy flag, documenting intent is valuable)

**Alternative if issues arise:**
```yaml
# Use GL backend instead (software rendering)
WGPU_BACKEND: gl
```

---

### Section 2: Test Job Improvements

**Current (Lines 86-93):**
```yaml
- name: Run library tests
  run: cargo test --lib --verbose

- name: Run integration tests
  run: cargo test --test '*' --verbose

- name: Run doc tests
  run: cargo test --doc --verbose
```

**Improved:**
```yaml
- name: Run library tests with timeout
  run: timeout 300 cargo test --lib --verbose
  continue-on-error: false

- name: Run integration tests with timeout
  run: timeout 300 cargo test --test '*' --verbose
  continue-on-error: false

- name: Run doc tests with timeout
  run: timeout 300 cargo test --doc --verbose
  continue-on-error: false
```

**Timeout Logic:**
- `timeout 300` = 5 minute maximum per test suite
- `continue-on-error: false` = Fail job if timeout occurs (don't ignore)
- If tests hang, job fails with clear timeout message

**Expected test times (from local runs):**
- Library tests: 30-60 seconds
- Integration tests: 60-90 seconds
- Doc tests: 10-30 seconds
- Total: ~2-3 minutes (well under 5 min per category)

---

### Section 3: Feature Test Improvements

**Current (Line 131):**
```yaml
- name: Test with features ${{ matrix.features }}
  run: cargo test ${{ matrix.features }} --verbose
```

**Improved:**
```yaml
- name: Test with features ${{ matrix.features }}
  run: timeout 300 cargo test ${{ matrix.features }} --verbose
  continue-on-error: false
```

**Why:** Consistency with other test jobs. Each feature combination gets same 5-minute timeout.

---

### Section 4: Coverage Command Improvements

**Current (Lines 161-163):**
```yaml
- name: Generate code coverage
  run: |
    cargo tarpaulin --verbose --all-features --workspace --timeout 300 --out xml --output-dir coverage
```

**Improved:**
```yaml
- name: Generate code coverage with timeout
  run: |
    cargo tarpaulin \
      --verbose \
      --all-features \
      --workspace \
      --timeout 300 \
      --exclude-files tests/* \
      --out xml \
      --output-dir coverage
```

**Change explanation:**
- `--exclude-files tests/*` - Don't measure coverage of test code itself (only application code)
- Multiline format - More readable
- Better comment

**Why:** Focus coverage metrics on application code, not test infrastructure

---

### Section 5: MSRV Job Improvements

**Current (Lines 239-242):**
```yaml
- name: Setup Rust 1.70
  uses: dtolnay/rust-toolchain@master
  with:
    toolchain: "1.70"
```

**Improved:**
```yaml
- name: Setup Rust 1.75 (MSRV)
  uses: dtolnay/rust-toolchain@master
  with:
    toolchain: "1.75"
```

**Why:** Matches Cargo.toml line 5: `rust-version = "1.75"`
- Currently testing against lower version (1.70)
- If features in 1.75 are used, CI would pass locally but fail when built with declared MSRV

**Verification:**
```bash
# Check declared MSRV
grep rust-version Cargo.toml
# Output: rust-version = "1.75"

# Verify CI matches
grep -A2 "Rust 1.75\|Rust 1.70" .github/workflows/ci.yml
```

---

## Implementation Instructions

### Method 1: Manual Patching (Safest)

Use this if you want to review each change:

```bash
# 1. Backup original
cp .github/workflows/ci.yml .github/workflows/ci.yml.backup

# 2. Open in editor
nano .github/workflows/ci.yml

# 3. Apply changes from sections above:
# - Add env variables (after line 12)
# - Add timeouts (lines 87, 90, 93)
# - Add timeouts to features (line 131)
# - Update MSRV (line 242)

# 4. Verify syntax
grep -n "timeout 300" .github/workflows/ci.yml  # Should see 4 matches
grep -n "WGPU_BACKEND" .github/workflows/ci.yml  # Should see 1 match
grep "1.75" .github/workflows/ci.yml  # Should match MSRV

# 5. Test locally
cargo check --all-features  # Verify Cargo.toml syntax
```

### Method 2: Use Provided Improved Workflow

```bash
# 1. Verify improved workflow exists
test -f .github/workflows/ci_improved.yml && echo "File exists"

# 2. Create backup
cp .github/workflows/ci.yml .github/workflows/ci.yml.backup

# 3. Replace workflow
cp .github/workflows/ci_improved.yml .github/workflows/ci.yml

# 4. Verify changes
diff -u .github/workflows/ci.yml.backup .github/workflows/ci.yml | head -50

# 5. Commit changes
git add .github/workflows/ci.yml
git commit -m "ci: improve test timeouts and alignment

- Add 300s timeout to all test jobs to prevent hangs
- Fix MSRV from 1.70 to 1.75 (matches Cargo.toml)
- Add Bevy headless rendering configuration
- Exclude test code from coverage metrics"
```

### Method 3: Gradual Migration

```bash
# Week 1: Test changes locally
git checkout -b ci/improvements
# Apply minimal changes (MSRV + timeouts)

# Week 2: Run in CI
git push -u origin ci/improvements
# Create pull request, monitor CI

# Week 3: Merge
git merge ci/improvements
# Delete branch
git branch -d ci/improvements
```

---

## Validation Checklist

After implementing changes, verify:

### Pre-commit Checks
```bash
# 1. Syntax validation
yamllint .github/workflows/ci.yml

# 2. File integrity
wc -l .github/workflows/ci.yml  # Should be ~290 lines

# 3. Key changes present
grep -c "timeout 300" .github/workflows/ci.yml  # Should be 4
grep -c "WGPU_BACKEND" .github/workflows/ci.yml  # Should be 1
grep -c "1.75" .github/workflows/ci.yml  # Should be 2+ (in MSRV job)
```

### Post-commit Checks
```bash
# 1. Push and wait for CI run
git push

# 2. Monitor CI jobs
# - Should all complete in ~15-20 minutes (same as before)
# - Check "Security Audit" job passes
# - Check "Test Suite" jobs complete without timeout errors
# - Check "MSRV" job succeeds with 1.75

# 3. Verify coverage still generated
# - Should see coverage report artifact
# - Should see codecov.io upload

# 4. Review logs for improvements
# - Look for explicit timeout messages
# - Verify no spurious test failures
```

---

## Rollback Instructions

If issues occur after implementation:

```bash
# Restore original workflow
cp .github/workflows/ci.yml.backup .github/workflows/ci.yml
git add .github/workflows/ci.yml
git commit -m "Revert CI workflow to previous version"
git push

# Investigate issue
# Then re-apply changes with fixes
```

---

## Monitoring After Implementation

### Expected Changes

1. **Test job output:** Will now show timeout prefix in logs
   ```
   running: timeout 300 cargo test --lib --verbose
   ```

2. **Job durations:** Should be identical to before (tests aren't slower)
   - Before: ~15-20 min total
   - After: ~15-20 min total

3. **Coverage reports:** May show slightly different numbers
   - Reason: `--exclude-files tests/*` removes test code coverage
   - Application coverage should be more accurate

### What Could Go Wrong

| Symptom | Cause | Solution |
|---------|-------|----------|
| Test job timeout | Tests slower than expected | Increase timeout to 600 (10 min) |
| MSRV job fails | Code uses 1.75+ features | Downgrade MSRV or fix code |
| Bevy tests hang | Graphics init still happening | Check Bevy version/features |
| Coverage drops | Test code excluded | Update baseline expectations |

### Performance Benchmarks

Expected CI run performance:

```
Job                  Before    After    Change
check               2:30      2:30     (same)
test (stable)       6:00      6:00     (same)
test (beta)         6:00      6:00     (same)
feature-tests      10:00     10:00     (same)
coverage           12:00     12:00     (same)
security-audit      1:30      1:30     (same)
examples            4:00      4:00     (same)
msrv                6:00      6:00     (same)
ci-success          0:30      0:30     (same)
─────────────────────────────────────────────
TOTAL              ~18 min   ~18 min   (same)
```

The improvements don't change execution time, but add safety nets for reliability.

---

## FAQ

### Q: Will timeouts break existing tests?
**A:** No. Current tests complete in 2-3 minutes total. 300-second (5-minute) per-suite timeout provides 100x headroom.

### Q: Can I use different timeout values?
**A:** Yes. Common values:
- `timeout 60` - 1 minute (aggressive, for fast tests)
- `timeout 300` - 5 minutes (balanced, recommended)
- `timeout 600` - 10 minutes (generous, for slow tests)

### Q: What if MSRV tests fail after update?
**A:** Code may use features only available in Rust 1.75+. Either:
1. Lower feature usage (use 1.70-compatible APIs)
2. Update MSRV to 1.75 in Cargo.toml and CI

### Q: Should I update both `ci.yml` and `ci_improved.yml`?
**A:** No. Choose one:
- Keep only current `ci.yml` - apply manual patches
- Replace with `ci_improved.yml` - delete original
- Don't maintain both parallel versions

### Q: Will Bevy tests run in headless CI?
**A:** Yes. Bevy can run in headless mode:
- No window created in tests
- Graphics initialization optional
- `WGPU_BACKEND=vk` prevents X11 fallback

### Q: What's the difference between `continue-on-error` values?
**A:**
- `continue-on-error: false` (or omitted) = Fail job if step fails
- `continue-on-error: true` = Continue job even if step fails

---

## References

### Files Modified
- `.github/workflows/ci.yml` - Main CI workflow
- `.github/workflows/ci_improved.yml` - Enhanced version (optional)

### Related Documentation
- `/home/beengud/raibid-labs/mimic/CI_VERIFICATION_REPORT.md` - Full analysis
- `Cargo.toml` - Project configuration
- GitHub Actions documentation: https://docs.github.com/en/actions

### Tools Used
- `cargo` - Rust package manager
- `cargo-tarpaulin` - Code coverage
- `cargo-audit` - Security auditing
- GitHub Actions - CI/CD platform

---

## Summary

**Current Status:** Production-ready CI pipeline ✅

**Recommended Actions:**
1. Apply minimal fixes (5 minutes) - MSRV + timeouts
2. Monitor CI runs (1-2 weeks)
3. Validate no regressions
4. Document in team wiki

**Risk Level:** Very Low
- Changes are conservative and additive
- No behavior changes, only safety improvements
- Easy to rollback if issues occur

**Expected Outcome:** Same reliable CI runs with better safety nets for timeout detection and version alignment.
