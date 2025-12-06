# Branch Protection Configuration

This document describes the branch protection rules that should be configured for the `main` branch.

## Why Branch Protection?

Branch protection ensures code quality and prevents accidental direct pushes to the main branch. All changes must go through pull requests with required reviews and CI checks.

## Configuration Steps

To enable branch protection, a repository administrator should:

1. Go to `Settings` → `Branches` in the GitHub repository
2. Under "Branch protection rules", click "Add rule"
3. Apply the following settings for the `main` branch:

### Protection Settings for `main` Branch

#### General Settings
- **Branch name pattern**: `main`
- **Require a pull request before merging**: ✅ Enabled
  - **Require approvals**: ✅ Enabled (1 approval minimum)
  - **Dismiss stale pull request approvals when new commits are pushed**: ✅ Enabled
  - **Require review from Code Owners**: ✅ Enabled

#### Status Checks
- **Require status checks to pass before merging**: ✅ Enabled
- **Require branches to be up to date before merging**: ✅ Enabled

**Required status checks** (from CI workflow):
- `Quick Check` (formatting and clippy)
- `Test Suite` (stable)
- `Documentation Check`
- `Security Audit`
- `Build Examples`
- `Headless Mode Tests`
- `Minimum Rust Version`
- `CI Success` (summary job)

#### Additional Settings
- **Require conversation resolution before merging**: ✅ Enabled
- **Require signed commits**: ⚠️ Optional (recommended for enhanced security)
- **Require linear history**: ⚠️ Optional (prevents merge commits)
- **Allow force pushes**: ❌ Disabled
- **Allow deletions**: ❌ Disabled

#### Rules Applied to Administrators
- **Include administrators**: ⚠️ Optional (recommended: ✅ Enabled for consistency)

## Verification

After enabling branch protection, verify it works:

1. Try to push directly to `main`:
   ```bash
   git checkout main
   git commit --allow-empty -m "test"
   git push origin main
   ```
   Expected: This should be rejected with an error about branch protection.

2. Create a PR without passing CI:
   - The merge button should be disabled
   - Status checks should show as required

3. Create a PR without approval:
   - Even if CI passes, merge should be blocked
   - At least one approval from a CODEOWNER should be required

## CODEOWNERS Integration

Branch protection works with `.github/CODEOWNERS` to enforce reviews:

- When files matching CODEOWNERS patterns are modified, designated reviewers are automatically requested
- At least one approval from a code owner is required to merge
- This ensures critical files (CI workflows, Cargo.toml, etc.) get proper review

## Bypass Permissions

In rare cases, protection rules may need to be bypassed:

- Only repository administrators can bypass protection rules
- Use "Include administrators" setting to enforce rules even for admins
- Document any bypasses in the PR or commit message

## Troubleshooting

### Can't merge even though CI passed

Check:
- Are all required status checks green?
- Do you have the required number of approvals?
- Are there unresolved conversations?
- Is the branch up to date with `main`?

### Status check not appearing

Ensure:
- The status check name matches exactly (case-sensitive)
- The CI workflow has run at least once on the PR
- The workflow is configured to run on pull requests

### Code owner approval not working

Verify:
- `.github/CODEOWNERS` file exists and is valid
- File paths in CODEOWNERS use correct syntax
- Code owners have permission to review

## Related Documentation

- [GitHub Branch Protection Rules](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches)
- [About Code Owners](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners)
- [docs/RELEASE.md](./RELEASE.md) - Release process documentation

## Implementation Status

**Current Status**: Branch protection rules documented, awaiting administrator configuration

**Action Required**: Repository administrator should apply these settings via GitHub UI

**Verification**: After configuration, test by attempting direct push to main (should be rejected)
