# Release Notes

Official release notes for Fanuc RMI API versions.

---

## Releases

### [v0.4.0](RELEASE_NOTES_v0.4.0.md) - 2025-11-25 ⭐ **LATEST**

**Major Release: Correlation ID System & Position Precision Fix**

**Breaking Changes:**
- 💥 Position/FrameData fields changed from f32 to f64
- 💥 `send_command()` returns correlation ID (u64) instead of sequence ID (u32)

**New Features:**
- ✨ Correlation ID system for request/response tracking
- ✨ Helper functions: `send_and_wait_for_completion()`, `wait_on_correlation_completion()`
- ✨ High-precision position data (f64) - sub-millimeter accuracy
- ✨ Comprehensive migration documentation

**Bug Fixes:**
- 🐛 Fixed invalid sequence ID errors (RMIT-029)
- 🐛 Fixed position display precision loss
- 🐛 Fixed web app trunk build issues

**Migration Required:** See [Sequence ID Migration Guide](../SEQUENCE_ID_MIGRATION_GUIDE.md)

---

### [v0.3.0](RELEASE_NOTES_v0.3.0.md) - 2025-11-13

**Major Release: ExtractInner Trait, DTO Enums, and Comprehensive Documentation**

**New Features:**
- ✨ Generic `ExtractInner<T>` trait for type-safe response extraction
- ✨ DTO enum re-exports for cleaner imports
- ✨ Comprehensive documentation system
- ✨ Three-tier message relay architecture

**Breaking Changes:**
- None (fully backward compatible)

**Documentation:**
- Added architecture guides
- Added reference implementations
- Added framework-agnostic examples

---

### [v0.2.0 Web App](RELEASE_NOTES_v0.2.0_WEB_APP.md) - 2025-11-16

**Web Application Redesign**

**New Features:**
- 🎨 Complete UI/UX redesign with dark mode aesthetic
- 🎨 Clean, minimalistic design inspired by industrial control software
- 🎨 Improved typography with Inter font family
- 🎨 Cyan accent color (#00d9ff) for professional look
- 🎨 Custom scrollbar styling

**Technical Improvements:**
- Updated all component styles
- Improved accessibility with better contrast
- Optimized CSS for performance
- Responsive grid layout

---

## Upcoming Releases

### v0.5.0 (Planned)

**Planned Features:**
- Additional robot model support
- Enhanced error recovery
- Performance optimizations
- Extended kinematics support

---

## Version History

| Version | Date | Highlights |
|---------|------|------------|
| v0.4.0 | 2025-11-25 | Correlation ID system, f32→f64 precision fix, bug fixes |
| v0.3.0 | 2025-11-13 | ExtractInner trait, DTO system, documentation |
| v0.2.0 | 2025-11-16 | Web app redesign, dark mode UI |
| v0.1.0 | 2025-XX-XX | Initial release |

---

## Release Process

1. **Version Bump**: Update version in all `Cargo.toml` files
2. **Changelog**: Create release notes in this directory
3. **Documentation**: Update main README and docs
4. **Testing**: Run full test suite
5. **Tag**: Create git tag `vX.Y.Z`
6. **Publish**: Publish to crates.io (when ready)

---

## Semantic Versioning

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR** (X.0.0): Breaking changes
- **MINOR** (0.X.0): New features, backward compatible
- **PATCH** (0.0.X): Bug fixes, backward compatible

---

## Migration Guides

For breaking changes, see:
- [Sequence ID Migration Guide](../SEQUENCE_ID_MIGRATION_GUIDE.md) - v0.3.0+ correlation ID system
- [Position Precision Fix](../POSITION_PRECISION_FIX.md) - f32 → f64 changes

---

## Contributing

When creating a new release:

1. Create a new `RELEASE_NOTES_vX.Y.Z.md` file
2. Follow the existing format
3. Include:
   - Release date
   - New features
   - Breaking changes
   - Bug fixes
   - Migration notes (if applicable)
4. Update this README with the new version
5. Update the main README.md


