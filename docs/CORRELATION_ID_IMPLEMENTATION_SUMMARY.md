# Correlation ID System Implementation Summary

## Overview

This document summarizes the implementation of the correlation ID system for the FANUC RMI driver, which solves the sequence ID traceability problem while maintaining all critical sequence ID fixes.

## Problem Statement

The driver assigns sequence IDs to instructions at **send time** (not queue insertion time) to ensure they are consecutive in the order packets are actually sent to the controller. This was necessary to fix:

1. ✅ Priority queue reordering causing non-consecutive sequence IDs
2. ✅ Web app generating random sequence IDs  
3. ✅ Starting sequence IDs not matching FANUC's expected value

However, this meant `send_command()` couldn't return the actual sequence ID synchronously, breaking traceability for client code that needed to correlate requests with responses.

## Solution: Correlation ID System

We implemented a correlation ID system that allows callers to:
1. Get a unique correlation ID immediately when calling `send_command()` (sync)
2. Subscribe to a broadcast channel to receive notifications when instructions are sent
3. Correlate their requests (via correlation ID) with actual sequence IDs and responses

## Implementation Details

### 1. New Types

#### `SentInstructionInfo` (fanuc_rmi/src/packets/instruction.rs)
```rust
pub struct SentInstructionInfo {
    pub correlation_id: u64,  // Matches return from send_command()
    pub sequence_id: u32,     // Actual sequence ID assigned by driver
    pub timestamp: Instant,   // When instruction was sent
}
```

### 2. Modified Types

#### `DriverPacket` (fanuc_rmi/src/drivers/driver.rs)
Added `correlation_id: u64` field to track packets through the queue.

#### `FanucDriver` (fanuc_rmi/src/drivers/driver.rs)
Added `sent_instruction_tx: broadcast::Sender<SentInstructionInfo>` field.

### 3. Modified Functions

#### `send_command()` - Return Type Changed
**Before:** `Result<u32, String>` (returned 0 for instructions)  
**After:** `Result<u64, String>` (returns unique correlation ID)

Implementation:
- Uses atomic counter `CORRELATION_COUNTER` to generate unique IDs
- Attaches correlation ID to `DriverPacket`
- Returns correlation ID immediately (sync)

#### `send_queue_to_controller()` - Emits Sent Notifications
After assigning sequence ID to an instruction:
```rust
let _ = self.sent_instruction_tx.send(SentInstructionInfo {
    correlation_id: driver_packet.correlation_id,
    sequence_id: current_id,
    timestamp: Instant::now(),
});
```

### 4. New Helper Functions

#### `wait_on_instruction_completion(sequence_id: u32)`
Renamed from `wait_on_command_completion` for clarity. Waits for an instruction with the given sequence ID to complete.

#### `wait_on_correlation_completion(correlation_id: u64) -> Result<u32, String>`
Convenience function that:
1. Subscribes to `sent_instruction_tx`
2. Waits for the instruction with matching correlation ID to be sent
3. Gets the sequence ID
4. Waits for completion
5. Returns the sequence ID

#### `send_and_wait_for_completion(packet, priority) -> Result<u32, String>`
One-shot convenience function that sends and waits in a single call.

### 5. Deprecated Functions

#### `wait_on_command_completion()`
Marked as deprecated, calls `wait_on_instruction_completion()` internally.

## Files Modified

1. **fanuc_rmi/src/packets/instruction.rs**
   - Added `SentInstructionInfo` struct

2. **fanuc_rmi/src/drivers/driver.rs**
   - Added `CORRELATION_COUNTER` static
   - Added `correlation_id` field to `DriverPacket`
   - Added `sent_instruction_tx` field to `FanucDriver`
   - Modified `send_command()` to return `u64` correlation ID
   - Modified `send_queue_to_controller()` to emit sent notifications
   - Added `wait_on_instruction_completion()`
   - Added `wait_on_correlation_completion()`
   - Added `send_and_wait_for_completion()`
   - Deprecated `wait_on_command_completion()`

3. **docs/SEQUENCE_ID_MIGRATION_GUIDE.md** (NEW)
   - Comprehensive migration guide for client code
   - Three migration strategies with examples
   - Real-world Meteorite example
   - API reference
   - Troubleshooting section

4. **docs/CORRELATION_ID_IMPLEMENTATION_SUMMARY.md** (NEW - this file)
   - Technical implementation summary

## Usage Examples

### Simple: Send and Wait
```rust
let sequence_id = driver.send_and_wait_for_completion(
    SendPacket::Instruction(instruction),
    PacketPriority::Standard
).await?;
```

### Flexible: Send Now, Wait Later
```rust
let correlation_id = driver.send_command(packet, PacketPriority::Standard)?;

// Later, in async context:
let sequence_id = driver.wait_on_correlation_completion(correlation_id).await?;
```

### Advanced: Manual Tracking
```rust
let mut sent_rx = driver.sent_instruction_tx.subscribe();
let correlation_id = driver.send_command(packet, PacketPriority::Standard)?;

// Listen for sent notification
while let Ok(sent_info) = sent_rx.recv().await {
    if sent_info.correlation_id == correlation_id {
        println!("Sent with sequence ID: {}", sent_info.sequence_id);
        break;
    }
}
```

## Testing

All packages compile successfully:
- ✅ `cargo check -p fanuc_rmi`
- ✅ `cargo build -p fanuc_rmi`
- ✅ `cargo test -p fanuc_rmi --lib`
- ✅ `cargo check -p web_server`

## Migration Path

See `docs/SEQUENCE_ID_MIGRATION_GUIDE.md` for detailed migration instructions.

**Key Points:**
- Breaking change: `send_command()` return type changed from `u32` to `u64`
- Old code that ignored the return value will get type errors
- Three migration strategies provided (simple, flexible, advanced)
- Backward compatibility: deprecated `wait_on_command_completion()` still works

## Benefits

1. ✅ **Maintains all sequence ID fixes** - Consecutive IDs in send order
2. ✅ **Full traceability** - Can correlate requests → sequence IDs → responses
3. ✅ **Sync-compatible** - `send_command()` returns immediately
4. ✅ **Flexible** - Three usage patterns for different needs
5. ✅ **Type-safe** - Correlation IDs are `u64`, sequence IDs are `u32`
6. ✅ **Non-blocking** - Broadcast channel doesn't block sender
7. ✅ **Multiple subscribers** - Many listeners can track sent instructions


