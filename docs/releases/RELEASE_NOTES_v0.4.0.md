# Release Notes - v0.4.0

**Release Date**: 2025-11-25  
**Repository**: vertec-io/Fanuc_RMI_API  
**Tag**: v0.4.0

---

## 🎉 Major Release: Correlation ID System & Position Precision Fix

This release introduces a new correlation ID system for tracking requests/responses and fixes critical precision issues with position data. **This release contains breaking changes** - please read the migration guide carefully.

---

## ⚠️ Breaking Changes

### 1. Position Data Type Change (f32 → f64)

**Impact**: HIGH - Affects all code using `Position` or `FrameData` structs

**What Changed**:
- All position fields changed from `f32` to `f64` (9 fields in `Position`, 6 fields in `FrameData`)
- Provides sub-millimeter precision (15-16 decimal digits vs 7 decimal digits)
- Eliminates JSON serialization precision loss

**Migration**:
```rust
// Before (v0.3.0)
let pos = Position {
    x: 100.0_f32,
    y: 200.0_f32,
    z: 300.0_f32,
    // ...
};

// After (v0.4.0)
let pos = Position {
    x: 100.0,  // f64 is default for float literals
    y: 200.0,
    z: 300.0,
    // ...
};

// Remove any `as f32` casts when working with Position
// Before: position.x as f32
// After:  position.x  (already f64)
```

**Why**: f32 precision loss caused UI to show slightly different values than FANUC Teach Pendant. Values like `12345.6789` were stored as `12345.6787` (loss of ~0.0002mm).

**Documentation**: [Position Precision Fix](../POSITION_PRECISION_FIX.md)

---

### 2. Correlation ID System

**Impact**: MEDIUM - Changes `send_command()` return type

**What Changed**:
- `send_command()` now returns `Result<u64, String>` (correlation ID) instead of `Result<u32, String>` (sequence ID)
- Sequence IDs are now assigned at send time (not queue insertion time)
- New helper functions for waiting on completion

**Migration**:

**Option 1: Simple (Recommended for most cases)**
```rust
// Before (v0.3.0) - BROKEN! Returns 0
let seq_id = driver.send_command(packet, priority)?;
driver.wait_on_command_completion(seq_id).await;

// After (v0.4.0) - Use helper function
let seq_id = driver.send_and_wait_for_completion(packet, priority).await?;
println!("Completed with sequence ID: {}", seq_id);
```

**Option 2: Flexible (Send now, wait later)**
```rust
// After (v0.4.0)
let correlation_id = driver.send_command(packet, priority)?;
// ... do other work ...
let seq_id = driver.wait_on_correlation_completion(correlation_id).await?;
```

**Option 3: Manual (Full control)**
```rust
// After (v0.4.0)
let correlation_id = driver.send_command(packet, priority)?;
let mut rx = driver.subscribe_sent_instructions();

while let Ok(info) = rx.recv().await {
    if info.correlation_id == correlation_id {
        println!("Sent with sequence ID: {}", info.sequence_id);
        break;
    }
}
```

**Why**: Previous implementation returned sequence ID which is now assigned asynchronously right before sending which broke traceability. Correlation IDs allow tracking requests across async boundaries. This fix was to address queue priority reordering when pushing to the front of a queue after a sequence ID had already been assigned.

**Documentation**: [Sequence ID Migration Guide](../SEQUENCE_ID_MIGRATION_GUIDE.md)

---

## ✨ New Features

### 1. Correlation ID Tracking System

Track requests and responses across async boundaries:

```rust
// Send command and get correlation ID
let correlation_id = driver.send_command(motion_packet, PacketPriority::Standard)?;

// Wait for it to be sent and get sequence ID
let sequence_id = driver.wait_on_correlation_completion(correlation_id).await?;

// Or use the all-in-one helper
let sequence_id = driver.send_and_wait_for_completion(motion_packet, priority).await?;
```

**Benefits**:
- Synchronous code can track async operations
- Full traceability from send → sequence ID → completion

**New Functions**:
- `send_and_wait_for_completion()` - Send and wait in one call
- `wait_on_correlation_completion()` - Wait for correlation ID to be sent
- `wait_on_instruction_completion()` - Wait for sequence ID to complete (renamed from `wait_on_command_completion`)
- `subscribe_sent_instructions()` - Subscribe to sent instruction notifications

---

### 2. High-Precision Position Data

Position data now uses `f64` for all coordinate fields:

```rust
pub struct Position {
    pub x: f64,     // Was f32
    pub y: f64,     // Was f32
    pub z: f64,     // Was f32
    pub w: f64,     // Was f32
    pub p: f64,     // Was f32
    pub r: f64,     // Was f32
    pub ext1: f64,  // Was f32
    pub ext2: f64,  // Was f32
    pub ext3: f64,  // Was f32
}
```

**Benefits**:
- Perfect JSON roundtrip accuracy
- UI displays exact values matching Teach Pendant
- Sub-millimeter precision maintained
- No more rounding errors

**Test**: Run `cargo test -p fanuc_rmi --test position_precision_test` to verify

---

### 3. Improved Sequence ID Management

- Sequence IDs now assigned at send time (not queue insertion time)
- Fixes non-consecutive sequence ID errors when using priority queue
- Reads `NextSequenceID` from FANUC controller on first `FRC_GetStatus`
- Proper handling of sequence ID wraparound

---

### 4. Web App Improvements

- Fixed WASM build issues (removed duplicate bin/lib targets)
- Updated to use `f64` for position data
- Removed unnecessary type casts
- Added `#[wasm_bindgen(start)]` entry point

---

## 🐛 Bug Fixes

### Fixed: Invalid Sequence ID Errors (RMIT-029)

**Issue**: FANUC controller returned error 2556957 (Invalid sequence ID number)

**Root Causes Fixed**:
1. ✅ Web app sending random sequence IDs (timestamp-based)
2. ✅ Priority queue reordering after sequence ID assignment
3. ✅ Missing `NextSequenceID` field - driver starting from 1 instead of controller's value

**Solution**: 
- Web app now sends `sequence_id: 0` (driver assigns)
- Sequence IDs assigned at send time (after queue reordering)
- Driver reads and uses `NextSequenceID` from controller

---

### Fixed: Position Display Precision Loss

**Issue**: Web UI showed slightly different values than FANUC Teach Pendant

**Root Cause**: f32 precision limited to ~7 decimal digits

**Solution**: Changed to f64 (15-16 decimal digits)

**Example**:
- Before: `12345.6789` → `12345.6787` (loss of 0.0002mm)
- After: `12345.6789` → `12345.6789` (perfect accuracy)

---

### Fixed: Web App Build Issues

**Issue**: Trunk build failed with "found more than one target artifact"

**Root Cause**: Both lib and bin targets with same name

**Solution**: 
- Removed `src/main.rs`
- Consolidated entry point in `src/lib.rs` with `#[wasm_bindgen(start)]`
- Updated `index.html` with proper trunk configuration

---

## 📚 New Documentation

- **[Sequence ID Migration Guide](../SEQUENCE_ID_MIGRATION_GUIDE.md)** - Complete migration guide with 3 patterns
- **[Correlation ID Implementation Summary](../CORRELATION_ID_IMPLEMENTATION_SUMMARY.md)** - Technical details
- **[Correlation ID Usage Examples](../examples/correlation_id_usage.rs)** - 4 complete working examples
- **[Position Precision Fix](../POSITION_PRECISION_FIX.md)** - Detailed explanation of f32→f64 change
- **[Position Precision Summary](../POSITION_PRECISION_SUMMARY.md)** - Quick reference
- **[Documentation Reorganization](../DOCUMENTATION_REORGANIZATION.md)** - New docs structure

---

## 🔧 Technical Changes

### Core Library (`fanuc_rmi`)

**Modified Files**:
- `src/lib.rs` - Position/FrameData f32→f64
- `src/drivers/driver.rs` - Correlation ID system
- `src/packets/instruction.rs` - SentInstructionInfo struct

**New Types**:
```rust
pub struct SentInstructionInfo {
    pub correlation_id: u64,
    pub sequence_id: u32,
    pub timestamp: std::time::Instant,
}
```

**New Fields**:
- `FrcGetStatusResponse.next_sequence_id: Option<u32>`
- `DriverPacket.correlation_id: u64`
- `FanucDriver.sent_instruction_tx: broadcast::Sender<SentInstructionInfo>`

### Examples

**Modified Files**:
- `example/src/bin/jog_client.rs` - Removed f32 casts
- `example/src/bin/jog_client_tui.rs` - Removed f32 casts

### Web App

**Modified Files**:
- `web_app/src/components/jog_controls.rs` - f32→f64 parameters
- `web_app/src/lib.rs` - Added wasm_bindgen entry point
- `web_app/index.html` - Updated trunk configuration

**Deleted Files**:
- `web_app/src/main.rs` - Consolidated into lib.rs

### Tests

**New Files**:
- `fanuc_rmi/tests/position_precision_test.rs` - Precision validation tests

---

## 📊 Version Compatibility

### Minimum Rust Version

- **Rust 1.70+** (unchanged)

### Feature Flags

- `driver` - Async driver with tokio (default)
- `DTO` - DTO type generation for network serialization
- `logging` - Logging support (default)

### Dependencies

No new dependencies added. All changes use existing dependencies.

---

## 🚀 Upgrade Guide

### Step 1: Update Cargo.toml

```toml
[dependencies]
fanuc_rmi = "0.4"  # Was 0.3
```

### Step 2: Fix Position Type Issues

Run `cargo check` and fix any type errors:

```rust
// Remove f32 casts
- let dist = distance as f32;
+ let dist = distance;  // Already f64

// Update function signatures if needed
- fn process(x: f32, y: f32, z: f32) { }
+ fn process(x: f64, y: f64, z: f64) { }
```

### Step 3: Update send_command() Usage

Choose one of the three patterns from the migration guide:

```rust
// Pattern 1: Simple (recommended)
let seq_id = driver.send_and_wait_for_completion(packet, priority).await?;

// Pattern 2: Flexible
let correlation_id = driver.send_command(packet, priority)?;
let seq_id = driver.wait_on_correlation_completion(correlation_id).await?;

// Pattern 3: Manual (advanced)
let correlation_id = driver.send_command(packet, priority)?;
let mut rx = driver.subscribe_sent_instructions();
// ... handle notifications ...
```

### Step 4: Test Thoroughly

```bash
# Run tests
cargo test

# Test with simulator
cargo run -p sim -- --realtime
cargo run -p example --bin jog_client

# Test web app
cargo run -p web_server
cd web_app && trunk serve
```

---

## 📖 Migration Resources

- **[Sequence ID Migration Guide](../SEQUENCE_ID_MIGRATION_GUIDE.md)** - Detailed migration instructions
- **[Correlation ID Examples](../examples/correlation_id_usage.rs)** - Working code examples
- **[Position Precision Fix](../POSITION_PRECISION_FIX.md)** - f32→f64 details

---

## 🙏 Acknowledgments

Special thanks to:
- FANUC Corporation for RMI specification (B-84184EN_02)
- All contributors who reported issues and tested fixes
- Research paper authors for CRX kinematics reference

---

## 📝 Full Changelog

### Added
- ✨ Correlation ID system for request/response tracking
- ✨ `send_and_wait_for_completion()` helper function
- ✨ `wait_on_correlation_completion()` function
- ✨ `wait_on_instruction_completion()` function (renamed)
- ✨ `subscribe_sent_instructions()` function
- ✨ `SentInstructionInfo` struct
- ✨ `next_sequence_id` field in `FrcGetStatusResponse`
- ✨ Position precision test suite
- 📚 Comprehensive migration documentation

### Changed
- 💥 **BREAKING**: `Position` fields from f32 to f64 (9 fields)
- 💥 **BREAKING**: `FrameData` fields from f32 to f64 (6 fields)
- 💥 **BREAKING**: `send_command()` returns `u64` (correlation ID) instead of `u32`
- 🔧 Sequence IDs assigned at send time (not queue insertion time)
- 🔧 Driver reads `NextSequenceID` from controller
- 🔧 Web app sends `sequence_id: 0` (driver assigns)
- 📝 Renamed `wait_on_command_completion()` to `wait_on_instruction_completion()`

### Fixed
- 🐛 Invalid sequence ID errors (RMIT-029, error code 2556957)
- 🐛 Position display precision loss in UI
- 🐛 Non-consecutive sequence IDs from priority queue reordering
- 🐛 Web app trunk build issues
- 🐛 Missing `NextSequenceID` field

### Deprecated
- ⚠️ `wait_on_command_completion()` - Use `wait_on_instruction_completion()` instead

### Removed
- 🗑️ `web_app/src/main.rs` - Consolidated into lib.rs

---

## 🔗 Links

- **Repository**: https://github.com/vertec-io/Fanuc_RMI_API
- **Documentation**: [docs/README.md](../README.md)
- **Issues**: https://github.com/vertec-io/Fanuc_RMI_API/issues

---

## Next Release (v0.5.0 - Planned)

Potential features for next release:
- Additional robot model support
- Enhanced error recovery
- Performance optimizations
- Extended kinematics support in simulator

