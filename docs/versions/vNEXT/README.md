# ratatui-testlib vNEXT - Unreleased Features

This directory contains documentation for features and changes planned for the next release.

## Planned Features

### Async Redraw Support
- Enhanced async integration for triggering and waiting on redraws
- Better control over render timing in async contexts

### Time-Travel Snapshotting
- Ability to capture and restore terminal state at specific points
- Debugging aid for reproducing specific states
- Integration with snapshot testing workflows

### Layout Diff Visualization
- Visual comparison of layout changes between test runs
- Highlight differences in widget positioning
- Assist with regression detection

### Widget Fixtures
- Pre-built test fixtures for common widgets:
  - Tables with sample data
  - Lists with various configurations
  - Popups and modal dialogs
  - Navigation patterns

### Event Scripting DSL
- Domain-specific language for describing user interaction sequences
- Reusable event patterns
- Support for complex input scenarios

### Contract Testing
- Test against multiple ratatui versions
- Ensure backward compatibility
- CI matrix testing support

### Enhanced Documentation
- Cookbook examples per host project:
  - scarab - file manager patterns
  - scarab-nav - navigation patterns
  - tolaria - MTG deck manager patterns
  - sparky - electric vehicle patterns
- Version-specific API guides
- Migration guides between versions

### Examples Gallery
- Golden snapshot examples
- Common testing patterns
- Integration examples for each feature flag

## Version Management

When these features are released, this directory will be:
1. Copied to `docs/versions/v{VERSION}/`
2. Updated with actual implementation details
3. Reset for the next development cycle

## Contributing

To add documentation for a planned feature:
1. Create a new markdown file in this directory
2. Follow the structure defined in `docs/STRUCTURE.md`
3. Include code examples even if they're aspirational
4. Mark clearly what's implemented vs. planned

## Status

**Current Status**: Planning and initial implementation
**Target Release**: TBD
**Tracking Issues**: #36 (Roadmap), and future feature-specific issues
