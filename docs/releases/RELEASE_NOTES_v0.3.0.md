# Release Notes - v0.3.0

**Release Date**: 2025-11-13  
**Repository**: vertec-io/Fanuc_RMI_API  
**Tag**: v0.3.0  
**Commit**: 0f5984e

---

## 🎉 Major Release: ExtractInner Trait, DTO Enums, and Comprehensive Documentation

This release adds powerful new ergonomic features and extensive documentation to make working with Fanuc RMI responses easier and more type-safe.

---

## ✨ New Features

### 1. Generic `ExtractInner<T>` Trait

A new generic trait for type-safe extraction of specific response types from response enums:

```rust
use fanuc_rmi::extract::ExtractInner;
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::commands::FrcReadJointAnglesResponse;

fn process_response(response: CommandResponse) {
    if let Some(angles) = response.extract_inner::<FrcReadJointAnglesResponse>() {
        println!("Joint angles: {:?}", angles.joint_angles);
    }
}
```

**Benefits**:
- Type-safe extraction without manual pattern matching
- Works with generic code
- Compile-time type checking
- Zero runtime overhead

### 2. DTO Enum Re-exports

Clean re-exports of DTO enums for better ergonomics:

```rust
// Before
use fanuc_rmi::packets::CommandResponseDto;

// After (cleaner!)
use fanuc_rmi::dto::CommandResponse;
```

**Re-exported enums**:
- `Command` (was `CommandDto`)
- `CommandResponse` (was `CommandResponseDto`)
- `Instruction` (was `InstructionDto`)
- `InstructionResponse` (was `InstructionResponseDto`)
- `Communication` (was `CommunicationDto`)
- `CommunicationResponse` (was `CommunicationResponseDto`)

### 3. Comprehensive Documentation (2,672+ lines)

#### Architecture Documentation
- **`docs/architecture/protocol_dto_system.md`** (444 lines)
  - Complete explanation of protocol vs DTO types
  - Auto-generation with `#[mirror_dto]` macro
  - Type hierarchies and usage patterns
  
- **`docs/architecture/message_relay_patterns.md`** (308 lines)
  - Three-tier relay architecture
  - Dual message bus pattern (internal + network)
  - Performance and complexity trade-offs

#### Examples and Guides
- **`docs/examples/basic_usage.md`** (376 lines)
  - Pattern matching examples
  - ExtractInner trait usage
  - Generic functions
  - Network serialization

#### Reference Implementations
- **`docs/reference_implementations/bevy_ecs_three_tier_relay.md`** (1,000+ lines)
  - Complete Bevy ECS three-tier relay implementation
  - Bevy 0.17 and eventwork 1.1 integration
  - Network transport examples
  - Copy-paste ready code

#### Documentation Hub
- **`docs/README.md`** (195 lines)
  - Central documentation hub
  - Quick start guide
  - Documentation structure overview

---

## 🔧 Code Changes

### New Files
- `fanuc_rmi/src/extract.rs` - ExtractInner trait and implementations
- `fanuc_rmi/tests/extract_inner_test.rs` - Comprehensive test suite
- `docs/` - Complete documentation structure (7 files)

### Modified Files
- `fanuc_rmi/src/dto/mod.rs` - Added DTO enum re-exports
- `fanuc_rmi/src/lib.rs` - Added extract module
- `fanuc_rmi/src/packets/*.rs` - Added ExtractInner implementations
- `fanuc_rmi/Cargo.toml` - Version bump to 0.3.0

---

## 📦 Dependencies

Documentation updated to reference:
- **Bevy 0.17.2** (requires Rust nightly 1.88.0+)
- **eventwork 1.1.2** (with automatic message registration)
- **bincode 1.3.3**
- **serde 1.0.190**

---

## 🚀 Migration Guide

### From v0.2.x to v0.3.0

**No breaking changes!** All changes are additive.

#### Optional: Use ExtractInner for cleaner code

**Before:**
```rust
match response {
    CommandResponse::FrcReadJointAngles(angles) => {
        // process angles
    }
    _ => {}
}
```

**After:**
```rust
if let Some(angles) = response.extract_inner::<FrcReadJointAnglesResponse>() {
    // process angles
}
```

#### Optional: Use cleaner DTO imports

**Before:**
```rust
use fanuc_rmi::packets::CommandResponseDto;
```

**After:**
```rust
use fanuc_rmi::dto::CommandResponse;
```

---

## 📊 Statistics

- **16 files changed**
- **3,222 insertions**, 3 deletions
- **2,672+ lines of documentation**
- **100% test coverage** for ExtractInner trait
- **Zero breaking changes**

---

## 🔗 Links

- **Repository**: https://github.com/vertec-io/Fanuc_RMI_API
- **Tag**: https://github.com/vertec-io/Fanuc_RMI_API/releases/tag/v0.3.0
- **Documentation**: See `docs/README.md` for complete documentation index

---

## 🙏 Acknowledgments

This release focuses on developer ergonomics and comprehensive documentation to make the Fanuc RMI library easier to use and integrate into real-world applications.

