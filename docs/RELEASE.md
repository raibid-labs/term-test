# Release Process

This document describes the release process for ratatui-testlib, including workflow details, branch protection, and step-by-step instructions.

## Release Workflow Overview

The release process is automated via GitHub Actions (`.github/workflows/release.yml`) and triggered by pushing a version tag.

### Workflow Outputs

When a release is created, the following artifacts are produced:

1. **GitHub Release**
   - Created automatically with changelog
   - Includes auto-generated release notes from commits
   - Tagged with version number (e.g., `v0.2.0`)
   - Marked as prerelease for alpha/beta/rc versions

2. **Crates.io Publication**
   - Package published to crates.io registry
   - Validates version matches tag before publishing
   - Runs full test suite before publishing

3. **Documentation**
   - API documentation built with `cargo doc`
   - Deployed to GitHub Pages
   - Accessible at project GitHub Pages URL

### Workflow Triggers

The release workflow can be triggered in two ways:

1. **Tag Push** (Recommended)
   ```bash
   git tag v0.2.0
   git push origin v0.2.0
   ```

2. **Manual Workflow Dispatch**
   - Via GitHub Actions UI
   - Requires version input (e.g., `v0.2.0`)
   - Useful for re-running failed releases

## Branch Protection

The `main` branch is protected with the following rules:

### Required Checks

Before merging to `main`, pull requests must pass:
- ✅ All CI checks (formatting, clippy, tests)
- ✅ Documentation validation
- ✅ Security audit
- ✅ Feature flag tests
- ✅ Headless mode tests
- ✅ MSRV compatibility check

### Protection Rules

- ❌ Direct pushes to `main` are disabled
- ✅ Pull requests are required
- ✅ At least one approval required (configured via CODEOWNERS)
- ✅ Dismiss stale approvals on new commits
- ✅ Require branches to be up to date before merging

### CODEOWNERS

The following files require review from designated owners:

- `.github/workflows/*` - CI/CD workflows
- `Cargo.toml` - Package configuration
- `docs/RELEASE.md` - Release documentation
- `CHANGELOG.md` - Version history

See `.github/CODEOWNERS` for current reviewer assignments.

## Release Steps

Follow these steps to create a new release:

### 1. Prepare the Release

```bash
# Ensure you're on main and up to date
git checkout main
git pull origin main

# Create a release branch
git checkout -b release/v0.2.0
```

### 2. Update Version Numbers

Update version in `Cargo.toml`:

```toml
[package]
name = "ratatui-testlib"
version = "0.2.0"  # Update this
```

### 3. Update CHANGELOG.md

Add a new version section following [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
## [0.2.0] - 2025-12-05

### Added
- New feature X
- New feature Y

### Changed
- Updated behavior Z

### Fixed
- Bug fix A
```

### 4. Update Version Documentation

If this is a minor or major release:

```bash
# Copy vNEXT to versioned directory
cp -r docs/versions/vNEXT docs/versions/v0.2.0

# Update version references in the new directory
# Reset vNEXT for next development cycle
```

### 5. Commit and Create PR

```bash
# Stage changes
git add Cargo.toml CHANGELOG.md docs/

# Commit with conventional commit message
git commit -m "chore: Prepare release v0.2.0"

# Push and create PR
git push origin release/v0.2.0
gh pr create --title "Release v0.2.0" --body "Prepare release v0.2.0"
```

### 6. Get Approval and Merge

- Wait for CI to pass
- Get required approvals from CODEOWNERS
- Merge PR to main

### 7. Tag and Push

```bash
# Switch back to main and pull merged changes
git checkout main
git pull origin main

# Create and push tag
git tag -a v0.2.0 -m "Release version 0.2.0"
git push origin v0.2.0
```

### 8. Monitor Release Workflow

Watch the GitHub Actions workflow:
- Go to Actions tab in GitHub
- Find the "Release" workflow run
- Monitor each job (create-release, publish-crate, build-docs)

### 9. Verify Release

Check that all artifacts were created:

1. **GitHub Release**: Visit `https://github.com/raibid-labs/ratatui-testlib/releases`
   - Verify release exists with correct version
   - Check release notes are populated

2. **Crates.io**: Visit `https://crates.io/crates/ratatui-testlib`
   - Verify new version is published
   - Check that documentation link works

3. **GitHub Pages**: Visit project docs URL
   - Verify documentation is updated
   - Check that version matches release

## Troubleshooting

### Release Workflow Failed

If the release workflow fails:

1. Check the workflow logs in GitHub Actions
2. Fix the issue locally
3. If the tag wasn't created on crates.io yet:
   ```bash
   # Delete the tag locally and remotely
   git tag -d v0.2.0
   git push origin :refs/tags/v0.2.0

   # Fix the issue, commit, and try again
   ```

### Version Mismatch Error

If you see "Version mismatch: Cargo.toml has X but tag is Y":

- Ensure Cargo.toml version matches the tag (without 'v' prefix)
- Tag `v0.2.0` should match `version = "0.2.0"` in Cargo.toml

### crates.io Publication Failed

Common issues:
- Missing or expired `CARGO_REGISTRY_TOKEN` secret
- Version already published (crates.io doesn't allow overwrites)
- Tests failed during publication

### Documentation Deployment Failed

Check:
- GitHub Pages is enabled in repository settings
- `gh-pages` branch exists
- Workflow has correct permissions

## Post-Release

After a successful release:

1. **Announce the Release**
   - Update project README if needed
   - Notify users via appropriate channels

2. **Create Next Milestone**
   - In GitHub Issues, create next version milestone
   - Move any remaining issues to new milestone

3. **Archive Old Version Docs**
   - Keep 2-3 recent version docs
   - Remove very old version directories

## Security Notes

- Never commit the `CARGO_REGISTRY_TOKEN` to version control
- Token is stored as GitHub secret
- Token can be regenerated at https://crates.io/me
- Follow principle of least privilege for GitHub tokens

## Versioning Policy

We follow [Semantic Versioning](https://semver.org/) (SemVer):

- **MAJOR**: Incompatible API changes
- **MINOR**: New functionality, backward compatible
- **PATCH**: Bug fixes, backward compatible

Pre-release versions:
- `v0.2.0-alpha.1` - Alpha release
- `v0.2.0-beta.1` - Beta release
- `v0.2.0-rc.1` - Release candidate

## Related Documentation

- [CHANGELOG.md](../CHANGELOG.md) - Version history
- [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution guidelines
- [docs/STRUCTURE.md](./STRUCTURE.md) - Documentation versioning

## Questions?

For questions about the release process, please:
- Open an issue in the repository
- Contact the maintainers listed in CODEOWNERS
