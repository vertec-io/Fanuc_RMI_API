# Release Notes: v0.5.0

**Release Date**: 2025-11-25  
**Type**: Minor Release (API Improvements)  
**Breaking Changes**: None (deprecations only)

---

## 🎯 Overview

Version 0.5.0 introduces significant API improvements focused on **clarity, consistency, and better async patterns**. All changes are backward compatible through deprecation warnings.

**Key Improvements:**
- ✨ **Async Command Methods** - `abort()`, `initialize()`, `get_status()`, `disconnect()` now wait for responses
- ✨ **Proper Error Handling** - Access FANUC error codes directly from responses
- ✨ **Industry Standard Terminology** - `correlation_id` → `request_id`
- ✨ **Clearer Method Names** - `send_command()` → `send_packet()`
- 🐛 **No More Arbitrary Sleeps** - Async methods eliminate manual delay guessing

---

## 🚀 New Features

### 1. Async Command Methods with Response Handling

**What Changed:**
- `abort()`, `initialize()`, `get_status()`, `disconnect()` are now **async** and **wait for responses**
- Return actual response types with FANUC error codes
- 5-second timeout prevents hanging forever

**Before (v0.4.x):**
```rust
driver.abort();
driver.initialize();
tokio::time::sleep(Duration::from_millis(500)).await; // Guessing!
```

**After (v0.5.0):**
```rust
let init_response = driver.initialize().await?;
if init_response.error_id == 0 {
    println!("✓ Initialize successful");
} else {
    eprintln!("✗ Initialize failed: {}", init_response.error_id);
}
```

**Benefits:**
- ✅ Proper error handling with FANUC error codes
- ✅ No arbitrary sleep delays
- ✅ Know exactly when commands complete
- ✅ Cleaner, more idiomatic async Rust

### 2. New Fire-and-Forget Methods

For advanced users who need fire-and-forget behavior or concurrent command sending:

- `send_abort()` → Returns `Result<u64, String>` (request_id)
- `send_initialize()` → Returns `Result<u64, String>` (request_id)
- `send_get_status()` → Returns `Result<u64, String>` (request_id)
- `send_disconnect()` → Returns `Result<u64, String>` (request_id)

**Use Case:** When you need to send multiple commands concurrently and track them manually via `response_tx`.

### 3. Industry Standard Terminology

**Changed:**
- `correlation_id` → `request_id` (matches HTTP/2, gRPC, AWS SDK, database drivers)
- `wait_on_correlation_completion()` → `wait_on_request_completion()`

**Why:** "request_id" is universally understood - clear mental model of "I send a request, I get a request ID back."

### 4. Clearer Method Names

**Changed:**
- `send_command()` → `send_packet()` (more accurate - sends ANY packet type, not just Commands)

**Why:** The old name was misleading since it sends Communication, Command, AND Instruction packets.

---

## 📋 Deprecations

All old methods still work but emit compiler warnings:

| Deprecated Method | Replacement | Notes |
|------------------|-------------|-------|
| `send_command()` | `send_packet()` | More accurate name |
| `wait_on_correlation_completion()` | `wait_on_request_completion()` | Industry standard terminology |

**Removal Timeline:** Deprecated methods will be removed in v1.0.0.

---

## 🔧 Migration Guide

See [NAMING_MIGRATION_GUIDE_v0.5.0.md](../NAMING_MIGRATION_GUIDE_v0.5.0.md) for complete migration instructions.

**Quick Migration:**

1. **Update async command calls:**
   ```rust
   // Old
   driver.initialize();
   tokio::time::sleep(Duration::from_millis(500)).await;
   
   // New
   driver.initialize().await?;
   ```

2. **Add error handling:**
   ```rust
   let response = driver.initialize().await?;
   if response.error_id != 0 {
       return Err(format!("Initialize failed: {}", response.error_id).into());
   }
   ```

3. **Update terminology:**
   ```rust
   // Old
   let correlation_id = driver.send_command(packet, priority)?;
   let seq = driver.wait_on_correlation_completion(correlation_id).await?;
   
   // New
   let request_id = driver.send_packet(packet, priority)?;
   let seq = driver.wait_on_request_completion(request_id).await?;
   ```

---

## ✅ Backward Compatibility

- ✅ **No breaking changes** - all old methods still work
- ✅ **Deprecation warnings** guide migration
- ✅ **Gradual migration** - update at your own pace
- ✅ **Examples updated** - see `example/` directory for new patterns

---

## 📦 Updated Examples

All examples updated to use new async methods:
- `example/src/main.rs` - Basic usage with response handling
- `example/src/bin/dto_roundtrip.rs` - DTO serialization example
- `example/src/bin/jog_client.rs` - Interactive jogging client
- `example/src/bin/jog_client_tui.rs` - TUI jogging interface
- `web_server/src/main.rs` - WebSocket server (no more sleep delays!)

---

## 🐛 Bug Fixes

- Fixed web_server using arbitrary sleep delays - now uses proper async methods
- Improved error handling across all examples

---

## 📚 Documentation Updates

- Updated [NAMING_MIGRATION_GUIDE_v0.5.0.md](../NAMING_MIGRATION_GUIDE_v0.5.0.md)
- Updated [README.md](../../readme.md) with new async patterns
- Updated all code examples

---

## 🙏 Acknowledgments

Thanks to the community for feedback on API ergonomics and async patterns!

---

## 📝 Full Changelog

### Added
- Async methods: `abort()`, `initialize()`, `get_status()`, `disconnect()` with response handling
- Fire-and-forget methods: `send_abort()`, `send_initialize()`, `send_get_status()`, `send_disconnect()`
- 5-second timeout on async command methods
- Comprehensive documentation with examples

### Changed
- `correlation_id` → `request_id` (terminology)
- `wait_on_correlation_completion()` → `wait_on_request_completion()`
- `send_command()` → `send_packet()`

### Deprecated
- `send_command()` - use `send_packet()`
- `wait_on_correlation_completion()` - use `wait_on_request_completion()`

### Removed
- None (all changes are backward compatible)

---

**Upgrade Command:**
```toml
[dependencies]
fanuc_rmi = "0.5"
```

**Questions?** See [Migration Guide](../NAMING_MIGRATION_GUIDE_v0.5.0.md) or open an issue.

