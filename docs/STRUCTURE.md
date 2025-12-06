# Documentation Structure

This document describes the organization and required sections for ratatui-testlib documentation.

## Directory Layout

```
docs/
├── STRUCTURE.md           # This file - documentation organization guide
├── README.md              # Overview of documentation contents
├── ARCHITECTURE.md        # Library architecture and design decisions
├── versions/              # Version-specific documentation
│   ├── vNEXT/            # Unreleased features and changes
│   ├── v0.2.0/           # Version 0.2.0 documentation (future)
│   └── v0.1.0/           # Version 0.1.0 documentation (future)
└── reports/              # Implementation reports and summaries
```

## Required Documentation Sections

### 1. Introduction (README.md)
- Project overview
- Key features
- Quick start examples
- Links to detailed documentation

### 2. Installation
- Cargo.toml configuration
- Feature flags explanation
- Platform-specific requirements

### 3. Usage Guides
Located in version-specific directories:
- Basic PTY testing
- Sixel graphics testing
- Bevy ECS integration
- Async testing patterns
- Snapshot testing workflows

### 4. API Documentation
- Generated via `cargo doc`
- Inline code examples
- Link from README to docs.rs

### 5. Architecture (ARCHITECTURE.md)
- System design
- Component relationships
- Technology choices
- Extension points

### 6. Changelog
- Root-level CHANGELOG.md
- Follows [Keep a Changelog](https://keepachangelog.com/) format
- Links to version-specific docs

## Versioning Policy

### Version-Specific Documentation

Each release should have a corresponding directory under `docs/versions/`:
- `vNEXT/` - Documentation for unreleased features
- `v{MAJOR}.{MINOR}.{PATCH}/` - Documentation frozen at release time

### When to Version Docs

- **Major releases**: Create new version directory, archive previous
- **Minor releases**: Update existing version directory
- **Patch releases**: Update existing version directory

### Migration Process

When cutting a release:
1. Copy `docs/versions/vNEXT/` to `docs/versions/v{VERSION}/`
2. Update version references in the new directory
3. Clear or reset `docs/versions/vNEXT/` for next development cycle
4. Update README.md to reference latest version docs

## Content Guidelines

### User-Facing Documentation
- Clear, concise language
- Working code examples
- Step-by-step tutorials
- Troubleshooting guides

### Implementation Reports
- Keep in `docs/reports/` directory
- Reference from main docs as needed
- Include context, decisions, and outcomes
- Maintain for historical reference

### What NOT to Include
- AI conversation logs (keep in local notes only)
- Temporary implementation notes
- Personal development notes
- Duplicate content across versions

## CI Documentation Checks

The following checks should be enforced in CI:

1. **Link Validation**
   - All internal links resolve correctly
   - No broken external links

2. **Structure Validation**
   - Required files exist (README.md, ARCHITECTURE.md, etc.)
   - Version directories follow naming convention

3. **Code Example Validation**
   - All code examples compile (where possible)
   - Examples use current API

4. **Changelog Validation**
   - CHANGELOG.md follows standard format
   - Version numbers are consistent

## Maintenance

### Regular Tasks
- Review and update examples with each release
- Prune outdated implementation reports
- Update architecture docs when design changes
- Validate all links quarterly

### Housekeeping
- Remove redundant content
- Consolidate overlapping guides
- Update screenshots/examples to match current UI
- Archive superseded version docs (keep 2-3 recent versions)

## Questions?

For questions about documentation structure or contributing to docs, see [CONTRIBUTING.md](../CONTRIBUTING.md).
