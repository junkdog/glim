# Changelog

## glim 0.2.0 - 2025-07-21

### Added
- GitLab service integration
- Vim key bindings support
- Project search filtering
- Structured logging with tracing framework
- Visual effects system with EffectRegistry
- Animation toggle configuration option

### Changed
- Refactored GitLab client architecture
- Updated to use CompactString for better performance
- Migrated to color-eyre for error handling
- Updated tachyonfx and ratatui dependencies
- Log files now saved to OS-appropriate cache directories (Linux: ~/.cache/glim, macOS: ~/Library/Caches/glim, Windows: %LOCALAPPDATA%\glim\cache)

### Fixed
- Fixed application panic issues

### Removed
- Removed internal log widget (replaced with proper logging)

## glim 0.1.0 - 2024-10-05

Initial release.
