# Historical Fixes

This directory contains documentation of past issues and their solutions. These documents are kept for historical reference and to help understand the evolution of the codebase.

## ⚠️ Important Note

**These documents describe issues that have already been fixed.** They are maintained for:
- Historical reference
- Understanding past design decisions
- Learning from previous bugs
- Tracking the evolution of the codebase

For current documentation, see the main [docs/](../) directory.

---

## Documents

### Configuration & Protocol Fixes

**[Configuration Fix Summary](CONFIGURATION_FIX_SUMMARY.md)** (2025-11-20)
- **Issue**: Configuration struct broke FANUC RMI API compatibility
- **Root Cause**: Incorrect JSON serialization and missing required fields
- **Fix**: Restored correct PascalCase field names and all required fields
- **Status**: ✅ Fixed and merged

### Sequence ID Issues

**[Final Sequence ID Fix](FINAL_SEQUENCE_ID_FIX.md)** (2025-11-24)
- **Issue**: RMIT-029 Invalid Sequence ID errors (error code 2556957)
- **Root Causes**: 
  1. Web app sending random sequence IDs
  2. Priority queue reordering after sequence ID assignment
  3. Missing NextSequenceID field
- **Fix**: All three root causes addressed
- **Status**: ✅ Fixed - Superseded by correlation ID system (see [SEQUENCE_ID_MIGRATION_GUIDE.md](../SEQUENCE_ID_MIGRATION_GUIDE.md))

**[Sequence ID Fix Summary](SEQUENCE_ID_FIX_SUMMARY.md)** (2025-11-24)
- **Issue**: Initial investigation of sequence ID errors
- **Root Causes**: Web app random IDs and priority queue reordering
- **Status**: ✅ Fixed - See FINAL_SEQUENCE_ID_FIX.md for complete solution

### Simulator & Kinematics

**[Jog Functionality Fix](JOG_FUNCTIONALITY_FIX.md)**
- **Issue**: Simulator was stateless - didn't update position after jog commands
- **Root Cause**: FRC_LinearRelative handler not updating robot_state
- **Fix**: Added state tracking to simulator
- **Status**: ✅ Fixed

**[Full IK Implementation Summary](FULL_IK_IMPLEMENTATION_SUMMARY.md)**
- **Objective**: Implement 7-step geometric IK algorithm from research paper
- **Status**: Partial implementation (60% complete)
- **Note**: Research implementation, not production-ready

**[Kinematics Update Summary](KINEMATICS_UPDATE_SUMMARY.md)**
- **Issue**: Kinematics using CRX-30iA approximation instead of accurate CRX-10iA parameters
- **Fix**: Updated DH parameters to match research paper
- **Status**: ✅ Fixed

---

## Timeline

| Date | Document | Issue |
|------|----------|-------|
| 2025-11-20 | Configuration Fix | Configuration struct compatibility |
| 2025-11-24 | Sequence ID Fixes | RMIT-029 Invalid Sequence ID errors |
| 2025-11-24 | Jog Functionality | Simulator state tracking |
| 2025-11-XX | Kinematics Update | CRX-10iA parameter accuracy |
| 2025-11-XX | IK Implementation | Geometric IK solver |

---

## Related Current Documentation

These historical fixes led to current features and documentation:

- **Sequence ID Fixes** → [Correlation ID System](../CORRELATION_ID_IMPLEMENTATION_SUMMARY.md) and [Migration Guide](../SEQUENCE_ID_MIGRATION_GUIDE.md)
- **Configuration Fix** → Proper Configuration struct in core library
- **Kinematics Updates** → [Robot Configuration](../ROBOT_CONFIGURATION.md)
- **Simulator Fixes** → Working simulator in `sim/` package

---

## How to Use These Documents

1. **Learning**: Understand how specific bugs were diagnosed and fixed
2. **Reference**: See what issues have already been addressed
3. **Context**: Understand why certain design decisions were made
4. **Debugging**: If similar issues arise, see how they were solved before

**Do not use these as current documentation** - they describe past states of the codebase that may no longer be accurate.

---

## Contributing

When adding new historical fix documentation:

1. Include the date and issue description
2. Explain the root cause clearly
3. Document the solution
4. Mark the status (Fixed/Partial/Superseded)
5. Link to related current documentation if applicable
6. Update this README with a summary entry


