# Migration Guide: v0.4.x → v0.5.0 Naming Changes

**Date**: 2025-11-25  
**Target Version**: v0.5.0  
**Breaking Changes**: None (all old methods deprecated, not removed)

---

## Overview

Version 0.5.0 introduces clearer, more consistent naming across the FANUC RMI API. All old method names are **deprecated but still functional** - your code will continue to work with compiler warnings.

**Key Changes:**
- `send_command()` → `send_packet()` (more accurate - sends any packet type)
- Helper methods now return `Result<u64, String>` (request ID for tracking)
- `correlation_id` → `request_id` (industry standard terminology)
- `wait_on_correlation_completion()` → `wait_on_request_completion()` (clearer naming)

---

## Quick Migration Table

### Async Methods (Wait for Response)

| Old Method (Deprecated) | New Method | Return Type | Notes |
|------------------------|------------|-------------|-------|
| `abort()` (fire-and-forget) | `abort()` | `Result<FrcAbortResponse, String>` | **Now async** - waits for response |
| `initialize()` (fire-and-forget) | `initialize()` | `Result<FrcInitializeResponse, String>` | **Now async** - waits for response |
| `get_status()` (fire-and-forget) | `get_status()` | `Result<FrcGetStatusResponse, String>` | **Now async** - waits for response |
| `disconnect()` (fire-and-forget) | `disconnect()` | `Result<FrcDisconnectResponse, String>` | **Now async** - waits for response |

### Sync Methods (Fire-and-Forget with Request ID)

| Method | Return Type | Notes |
|--------|-------------|-------|
| `send_abort()` | `Result<u64, String>` | **New** - returns request_id immediately |
| `send_initialize()` | `Result<u64, String>` | **New** - returns request_id immediately |
| `send_get_status()` | `Result<u64, String>` | **New** - returns request_id immediately |
| `send_disconnect()` | `Result<u64, String>` | **New** - returns request_id immediately (async) |

### Other Changes

| Old Method | New Method | Notes |
|-----------|------------|-------|
| `send_command(packet, priority)` | `send_packet(packet, priority)` | More accurate name |
| `wait_on_correlation_completion(id)` | `wait_on_request_completion(id)` | "request_id" is industry standard |
| `wait_on_command_completion(seq)` | **REMOVED** | Use `wait_on_instruction_completion` |

### Terminology Changes ✅

| Old Term | New Term | Reason |
|----------|----------|--------|
| `correlation_id` | `request_id` | Industry standard (HTTP/2, gRPC, AWS SDK) |
| `CORRELATION_COUNTER` | `REQUEST_COUNTER` | Internal - matches new terminology |

### Methods Kept As-Is (No Change) ✅

| Method Name | Reason |
|------------|--------|
| `send_and_wait_for_completion(...)` | Clear and explicit - verbosity improves understanding |
| `wait_on_instruction_completion(seq)` | "instruction_completion" is clearer than alternatives |

---

## Detailed Migration Examples

### 1. Sending Packets

**Before (v0.4.x):**
```rust
use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig};
use fanuc_rmi::packets::{SendPacket, PacketPriority, Instruction};
use fanuc_rmi::instructions::FrcLinearRelative;

let driver = FanucDriver::connect(config).await?;

// Old: Misleading name (sends any packet type, not just commands)
let correlation_id = driver.send_command(
    SendPacket::Instruction(Instruction::FrcLinearRelative(instruction)),
    PacketPriority::Standard
)?;
```

**After (v0.5.0):**
```rust
use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig};
use fanuc_rmi::packets::{SendPacket, PacketPriority, Instruction};
use fanuc_rmi::instructions::FrcLinearRelative;

let driver = FanucDriver::connect(config).await?;

// New: Clear name - sends any packet type, returns request_id
let request_id = driver.send_packet(
    SendPacket::Instruction(Instruction::FrcLinearRelative(instruction)),
    PacketPriority::Standard
)?;
```

---

### 2. Helper Methods - Now Async with Response Handling!

**Before (v0.4.x):**
```rust
let driver = FanucDriver::connect(config).await?;

// Old: Fire-and-forget, no response handling
driver.abort();
driver.initialize();
driver.get_status();

// Had to manually sleep and hope it worked
tokio::time::sleep(Duration::from_millis(500)).await;
```

**After (v0.5.0) - Option 1: Async Methods (Recommended)**
```rust
let driver = FanucDriver::connect(config).await?;

// New: Async methods that wait for responses
let abort_response = driver.abort().await?;
if abort_response.error_id == 0 {
    println!("✓ Abort successful");
}

let init_response = driver.initialize().await?;
if init_response.error_id == 0 {
    println!("✓ Initialize successful, group_mask: {}", init_response.group_mask);
}

let status = driver.get_status().await?;
println!("Servo ready: {}, Next seq: {}", status.servo_ready, status.next_sequence_id);
```

**After (v0.5.0) - Option 2: Fire-and-Forget with Request ID**
```rust
let driver = FanucDriver::connect(config).await?;

// New: Sync methods that return request_id immediately
let abort_id = driver.send_abort()?;
let init_id = driver.send_initialize()?;
let status_id = driver.send_get_status()?;

// Can track these requests manually via response_tx if needed
println!("Abort request ID: {}", abort_id);
```

**Migration Tip:** Use async methods for simple cases, `send_*()` for advanced concurrent usage.

---

### 3. Disconnect

**Before (v0.4.x):**
```rust
// Old: Fire-and-forget, no response
driver.disconnect().await;
```

**After (v0.5.0) - Option 1: Async with Response (Recommended)**
```rust
// New: Waits for disconnect confirmation
let response = driver.disconnect().await?;
if response.error_id == 0 {
    println!("✓ Disconnected successfully");
}
```

**After (v0.5.0) - Option 2: Fire-and-Forget with Request ID**
```rust
// New: Returns request_id immediately
let disconnect_id = driver.send_disconnect().await?;
```

---

### 4. Send and Wait Pattern - NO CHANGE ✅

**v0.4.x and v0.5.0 (Same):**
```rust
// Kept as-is: Clear and explicit name
let sequence_id = driver.send_and_wait_for_completion(
    SendPacket::Instruction(instruction),
    PacketPriority::Standard
).await?;
```

**Why no change?** The verbosity makes it crystal clear this is a blocking operation that waits for completion.

---

### 5. Waiting for Completion

**Before (v0.4.x):**
```rust
let correlation_id = driver.send_command(packet, priority)?;
let sequence_id = driver.wait_on_correlation_completion(correlation_id).await?;
driver.wait_on_instruction_completion(sequence_id).await;
```

**After (v0.5.0):**
```rust
// Updated: "request_id" is industry standard terminology
let request_id = driver.send_packet(packet, priority)?;
let sequence_id = driver.wait_on_request_completion(request_id).await?;
driver.wait_on_instruction_completion(sequence_id).await;
```

**Why the change?**
- "request_id" is industry standard (HTTP/2, gRPC, AWS SDK, database drivers)
- Clear mental model: "I send a request, I get a request ID back"
- "instruction_completion" kept as-is - clearer than alternatives like "sequence"

---

## Complete Example Migration

### Before (v0.4.x)

```rust
use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig},
    packets::{SendPacket, PacketPriority, Instruction},
    instructions::FrcLinearRelative,
    Configuration, Position, SpeedType, TermType,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = FanucDriverConfig {
        addr: "192.168.1.100".to_string(),
        port: 18735,
        max_messages: 30,
    };

    let driver = FanucDriver::connect(config).await?;

    // Initialize robot (fire-and-forget, no response handling)
    driver.abort();
    driver.initialize();
    tokio::time::sleep(Duration::from_millis(500)).await; // Manual delay
    
    // Send motion instruction
    let instruction = FrcLinearRelative::new(
        0,
        Configuration::default(),
        Position { x: 10.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0 },
        SpeedType::MMSec,
        30.0,
        TermType::FINE,
        1,
    );
    
    // Send and wait
    let sequence_id = driver.send_and_wait_for_completion(
        SendPacket::Instruction(Instruction::FrcLinearRelative(instruction)),
        PacketPriority::Standard
    ).await?;
    
    println!("Motion completed: sequence {}", sequence_id);
    
    // Disconnect
    driver.disconnect().await;
    
    Ok(())
}
```

### After (v0.5.0)

```rust
use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig},
    packets::{SendPacket, PacketPriority, Instruction},
    instructions::FrcLinearRelative,
    Configuration, Position, SpeedType, TermType,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = FanucDriverConfig {
        addr: "192.168.1.100".to_string(),
        port: 18735,
        max_messages: 30,
    };

    let driver = FanucDriver::connect(config).await?;

    // Initialize robot with response handling
    let abort_response = driver.abort().await?;
    if abort_response.error_id != 0 {
        return Err(format!("Abort failed: {}", abort_response.error_id).into());
    }

    let init_response = driver.initialize().await?;
    if init_response.error_id != 0 {
        return Err(format!("Initialize failed: {}", init_response.error_id).into());
    }
    
    // Send motion instruction
    let instruction = FrcLinearRelative::new(
        0,
        Configuration::default(),
        Position { x: 10.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0 },
        SpeedType::MMSec,
        30.0,
        TermType::FINE,
        1,
    );
    
    // Send and wait (name unchanged - kept for clarity)
    let sequence_id = driver.send_and_wait_for_completion(
        SendPacket::Instruction(Instruction::FrcLinearRelative(instruction)),
        PacketPriority::Standard
    ).await?;
    
    println!("Motion completed: sequence {}", sequence_id);
    
    // Disconnect with response handling
    let disconnect_response = driver.disconnect().await?;
    if disconnect_response.error_id == 0 {
        println!("✓ Disconnected successfully");
    }
    
    Ok(())
}
```

---

## Why These Changes?

### 1. `send_command()` → `send_packet()`

**Problem:** The name `send_command()` was misleading because it sends **any** packet type:
- `Communication` (FrcConnect, FrcDisconnect)
- `Command` (FrcAbort, FrcInitialize, FrcGetStatus)
- `Instruction` (FrcLinearRelative, FrcJointMotion, etc.)

**Solution:** `send_packet()` accurately describes what the method does.

### 2. Helper Methods Now Async with Response Handling

**Problem:** Methods like `abort()`, `initialize()` were fire-and-forget with no way to:
- Know if the command succeeded
- Handle FANUC error codes
- Avoid arbitrary sleep delays

**Solution:**
- **Async methods** (`abort()`, `initialize()`, `get_status()`, `disconnect()`) now wait for responses with 5-second timeout
- **Sync methods** (`send_abort()`, `send_initialize()`, etc.) return request_id for advanced usage
- Proper error handling with FANUC error codes

### 3. Terminology: `correlation_id` → `request_id`

**Problem:** "correlation_id" is unusual terminology not commonly used in other codebases.

**Solution:** Renamed to "request_id" - industry standard used by HTTP/2, gRPC, AWS SDK, database drivers.

**Benefits:**
- Clear mental model: "I send a request, I get a request ID back"
- Familiar to all developers
- Matches the pattern (making requests to FANUC controller)

---

## Backward Compatibility

✅ **All old methods still work** - they're deprecated, not removed  
✅ **No breaking changes** - your code will compile with warnings  
✅ **Easy migration** - just follow compiler warnings  
✅ **Gradual migration** - update at your own pace  

Old methods will be **removed in v1.0.0**, giving you plenty of time to migrate.

---

## Migration Checklist

- [ ] Replace `send_command()` with `send_packet()`
- [ ] Update `abort()` calls - now async, returns `Result<FrcAbortResponse, String>`
- [ ] Update `initialize()` calls - now async, returns `Result<FrcInitializeResponse, String>`
- [ ] Update `get_status()` calls - now async, returns `Result<FrcGetStatusResponse, String>`
- [ ] Update `disconnect()` calls - now async, returns `Result<FrcDisconnectResponse, String>`
- [ ] Remove manual `sleep()` calls after abort/initialize/disconnect
- [ ] Add proper error handling for FANUC error codes (check `error_id` field)
- [ ] Replace `correlation_id` with `request_id` (variable names)
- [ ] Replace `wait_on_correlation_completion()` with `wait_on_request_completion()`
- [ ] Remove any `wait_on_command_completion()` calls (use `wait_on_instruction_completion()`)
- [ ] Test your application
- [ ] Remove `#[allow(deprecated)]` attributes once migration is complete

**Note:** `send_and_wait_for_completion()` and `wait_on_instruction_completion()` are **NOT** changing - keep using them as-is!

**Advanced Usage:** If you need fire-and-forget behavior or concurrent command sending, use the new `send_*()` methods instead of the async methods.

---

## Need Help?

- See examples in `example/` directory for updated usage
- Check `docs/examples/correlation_id_usage.rs` for patterns
- Read `docs/SEQUENCE_ID_MIGRATION_GUIDE.md` for correlation ID system details


