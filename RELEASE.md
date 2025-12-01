# Release Guide for mimic

This document describes how to create releases and publish the `mimic` crate to crates.io.

## Prerequisites

Before creating a release, ensure:

1. **All tests pass**: `cargo test --all-features`
2. **Documentation builds**: `cargo doc --all-features --no-deps`
3. **Examples compile**: `cargo build --examples --all-features`
4. **CHANGELOG.md is updated** with changes for this version
5. **Version in Cargo.toml matches the release tag**
6. **CARGO_REGISTRY_TOKEN secret is configured** in GitHub repository settings

## Setting Up crates.io Token

To publish to crates.io, you need to set up the `CARGO_REGISTRY_TOKEN` secret:

1. **Get your crates.io API token**:
   - Log in to https://crates.io
   - Go to Account Settings → API Tokens
   - Click "New Token"
   - Give it a name like "mimic-releases"
   - Copy the token (you won't be able to see it again)

2. **Add the secret to GitHub**:
   ```bash
   gh secret set CARGO_REGISTRY_TOKEN
   # Paste your token when prompted
   ```

   Or manually via GitHub UI:
   - Go to repository Settings → Secrets and variables → Actions
   - Click "New repository secret"
   - Name: `CARGO_REGISTRY_TOKEN`
   - Value: Your crates.io token

## Release Process

### Option 1: Tag-based Release (Recommended)

1. **Update version in Cargo.toml**:
   ```toml
   [package]
   version = "0.1.0"  # Change to your target version
   ```

2. **Update CHANGELOG.md** with release notes

3. **Commit changes**:
   ```bash
   git add Cargo.toml CHANGELOG.md
   git commit -m "chore: prepare release v0.1.0"
   ```

4. **Create and push a tag**:
   ```bash
   git tag -a v0.1.0 -m "Release v0.1.0"
   git push origin main
   git push origin v0.1.0
   ```

5. **GitHub Actions will automatically**:
   - Run all tests
   - Create a GitHub release with changelog
   - Publish to crates.io
   - Build and deploy documentation to GitHub Pages

### Option 2: Manual Trigger

You can also trigger a release manually via GitHub Actions:

1. Go to the repository on GitHub
2. Click "Actions" → "Release" workflow
3. Click "Run workflow"
4. Enter the version (e.g., `v0.1.0`)
5. Click "Run workflow"

## Release Checklist

Before creating a release, verify:

- [ ] All tests pass: `cargo test --all-features`
- [ ] Clippy is happy: `cargo clippy --all-features -- -D warnings`
- [ ] Documentation builds: `cargo doc --all-features --no-deps`
- [ ] Examples compile: `cargo build --examples --all-features`
- [ ] Version in Cargo.toml matches release tag
- [ ] CHANGELOG.md is updated with release notes
- [ ] All open issues addressed in this release are documented
- [ ] README.md examples are up to date
- [ ] Breaking changes are documented (if any)
- [ ] Migration guide exists (if breaking changes)

## Versioning

`mimic` follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version for incompatible API changes
- **MINOR** version for new functionality (backward-compatible)
- **PATCH** version for bug fixes (backward-compatible)

### Pre-releases

For alpha, beta, or release candidate versions:

```bash
git tag -a v0.2.0-alpha.1 -m "Release v0.2.0-alpha.1"
git push origin v0.2.0-alpha.1
```

The workflow will automatically mark these as pre-releases.

## Post-Release

After a successful release:

1. **Announce the release**:
   - Post in Ratatui Discord
   - Tweet about it (if significant)
   - Update dgx-pixels to use the new version

2. **Monitor for issues**:
   - Watch GitHub issues
   - Check crates.io download stats
   - Monitor CI/CD for any failures

3. **Update project board**:
   - Close completed milestones
   - Create new milestone for next version

## Troubleshooting

### Release fails with "version mismatch"

Ensure the version in `Cargo.toml` matches the git tag:
- Tag `v0.1.0` requires Cargo.toml version `0.1.0` (without 'v' prefix)

### Release fails with "authentication required"

The `CARGO_REGISTRY_TOKEN` secret is missing or invalid:
```bash
gh secret set CARGO_REGISTRY_TOKEN
# Enter your crates.io API token
```

### Documentation deployment fails

Check that GitHub Pages is enabled:
- Repository Settings → Pages
- Source: "Deploy from a branch"
- Branch: "gh-pages"

### Tests fail on release

Always test before tagging:
```bash
cargo test --all-features
cargo clippy --all-features -- -D warnings
```

## Release Workflow Details

The `.github/workflows/release.yml` workflow consists of three jobs:

### 1. create-release
- Extracts version from tag
- Generates changelog from git commits
- Creates GitHub release (draft for pre-releases)

### 2. publish-crate
- Verifies version matches tag
- Runs full test suite
- Publishes to crates.io

### 3. build-docs
- Builds documentation with all features
- Creates index redirect
- Deploys to GitHub Pages (if configured)

## First Release Checklist

For the first v0.1.0 release:

- [ ] Set up CARGO_REGISTRY_TOKEN secret
- [ ] Verify crate name availability on crates.io
- [ ] Ensure all metadata in Cargo.toml is correct:
  - [ ] description
  - [ ] repository
  - [ ] license
  - [ ] keywords (max 5)
  - [ ] categories
- [ ] Add comprehensive README.md
- [ ] Include LICENSE file
- [ ] Verify all examples work
- [ ] Complete all documentation
- [ ] Close or address issues #1, #7, #8

## Additional Resources

- [crates.io Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [Cargo Manifest Format](https://doc.rust-lang.org/cargo/reference/manifest.html)
- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
