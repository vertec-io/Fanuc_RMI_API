# Sequence ID Migration Guide

## Overview

This guide explains the changes to sequence ID management in the FANUC RMI driver and provides migration strategies for existing code.

## What Changed

### Breaking Change: `send_command()` Return Type

**Before:**
```rust
pub fn send_command(&self, packet: SendPacket, priority: PacketPriority) -> Result<u32, String>
```
- Returned `u32` (sequence ID)
- **Problem:** Always returned `0` for Instructions because sequence IDs were assigned later

**After:**
```rust
pub fn send_command(&self, packet: SendPacket, priority: PacketPriority) -> Result<u64, String>
```
- Returns `u64` (correlation ID)
- Correlation ID can be used to track when the instruction is sent and get its actual sequence ID

### Why This Change Was Necessary

The driver assigns sequence IDs at **send time** (not queue insertion time) to ensure they are consecutive in the order packets are actually sent to the controller. This fixes critical issues with:

1. Priority queue reordering causing non-consecutive sequence IDs
2. Web app generating random sequence IDs
3. Starting sequence IDs not matching FANUC's expected value

However, this meant `send_command()` couldn't return the actual sequence ID synchronously. The correlation ID system solves this by allowing callers to track their requests and correlate them with the actual sequence IDs assigned later.

## New Features

### 1. Correlation ID System

Every call to `send_command()` now returns a unique correlation ID that can be used to track the request.

### 2. Sent Instruction Broadcast Channel

Subscribe to `driver.sent_instruction_tx` to receive notifications when instructions are assigned sequence IDs:

```rust
pub struct SentInstructionInfo {
    pub correlation_id: u64,  // Matches the return value from send_command()
    pub sequence_id: u32,     // The actual sequence ID assigned by the driver
    pub timestamp: Instant,   // When the instruction was sent
}
```

### 3. New Helper Functions

Three new convenience functions make it easy to work with the correlation ID system:

- `wait_on_instruction_completion(sequence_id: u32)` - Renamed from `wait_on_command_completion`
- `wait_on_correlation_completion(correlation_id: u64)` - Wait using correlation ID
- `send_and_wait_for_completion(packet, priority)` - Send and wait in one call

## Migration Strategies

### Strategy 1: Use `send_and_wait_for_completion()` (Simplest)

**Best for:** Code that sends an instruction and immediately waits for it to complete.

**Before:**
```rust
let command_id = driver.send_command(motion_command, PacketPriority::Standard)?;
driver.wait_on_command_completion(command_id).await;
```

**After:**
```rust
let sequence_id = driver.send_and_wait_for_completion(
    motion_command,
    PacketPriority::Standard
).await?;
// sequence_id is available if you need it for logging
```

**Pros:**
- Minimal code changes
- Clean and simple
- Returns the actual sequence ID

**Cons:**
- Blocks until completion (but this was the original behavior)

---

### Strategy 2: Use `wait_on_correlation_completion()` (Recommended)

**Best for:** Code that needs to send first, then wait later (e.g., spawning background tasks).

**Before:**
```rust
let command_id = driver.send_command(motion_command, PacketPriority::Standard)?;

let driver_clone = driver.clone();
tokio::spawn(async move {
    driver_clone.wait_on_command_completion(command_id).await;
    println!("Done!");
});
```

**After:**
```rust
let correlation_id = driver.send_command(motion_command, PacketPriority::Standard)?;

let driver_clone = driver.clone();
tokio::spawn(async move {
    match driver_clone.wait_on_correlation_completion(correlation_id).await {
        Ok(sequence_id) => println!("Instruction {} completed!", sequence_id),
        Err(e) => eprintln!("Error: {}", e),
    }
});
```

**Pros:**
- Flexible - send and wait are separate operations
- Works well with background tasks
- Returns the actual sequence ID

**Cons:**
- Slightly more verbose than Strategy 1

---

### Strategy 3: Manual Correlation (Advanced)

**Best for:** Code that needs fine-grained control over tracking multiple instructions.

**Example:**
```rust
use std::collections::HashMap;

// Subscribe to sent instruction notifications
let mut sent_rx = driver.sent_instruction_tx.subscribe();
let mut response_rx = driver.response_tx.subscribe();

// Send instruction and get correlation ID
let correlation_id = driver.send_command(packet, PacketPriority::Standard)?;

// Track correlation_id -> sequence_id mapping
let mut correlation_map: HashMap<u64, u32> = HashMap::new();

tokio::spawn(async move {
    loop {
        tokio::select! {
            // Listen for sent notifications
            Ok(sent_info) = sent_rx.recv() => {
                if sent_info.correlation_id == correlation_id {
                    println!("Instruction sent with sequence ID: {}", sent_info.sequence_id);
                    correlation_map.insert(sent_info.correlation_id, sent_info.sequence_id);
                }
            }

            // Listen for responses
            Ok(response) = response_rx.recv() => {
                if let ResponsePacket::InstructionResponse(ref instr_resp) = response {
                    let seq_id = instr_resp.get_sequence_id();

                    // Find correlation ID for this sequence ID
                    if let Some((corr_id, _)) = correlation_map.iter()
                        .find(|(_, &s)| s == seq_id) {
                        println!("Response for correlation {}: {:?}", corr_id, response);
                    }
                }
            }
        }
    }
});
```

**Pros:**
- Maximum control and flexibility
- Can track multiple instructions simultaneously
- Access to full response data

**Cons:**
- Most complex
- Requires manual state management

---

## Real-World Example: Meteorite Toolpath Execution

This example shows how to migrate the toolpath execution code from Meteorite.

### Before (Broken - returned 0):
```rust
let motion_command = SendPacket::Instruction(
    Instruction::FrcLinearMotion(FrcLinearMotion::new(
        0,  // sequence_id placeholder
        fanuc_config.0.clone(),
        convert_to_position(&point.point),
        SpeedType::MilliSeconds,
        point.move_time as f64 * 1000.0,
        point.term_type.clone(),
        100,
    ))
);

match rmi_driver.0.send_command(motion_command.clone(), PacketPriority::Standard) {
    Ok(command_id) if point.is_last_coast_point => {
        let driver_clone = rmi_driver.clone();
        let timeout_duration = std::time::Duration::from_secs_f32(point.move_time * 1.5);

        tokio_tasks_runtime.spawn_background_task(move |mut ctx| async move {
            let completion_result = tokio::time::timeout(
                timeout_duration,
                driver_clone.0.wait_on_command_completion(command_id),  // ❌ command_id was 0!
            ).await;

            ctx.run_on_main_thread(move |ctx| {
                match completion_result {
                    Ok(_) => ctx.world.write_message(
                        new_log_message("Refill coast points completed").debug()
                    ),
                    Err(_) => ctx.world.write_message(
                        new_log_message("Refill coast points timed out").debug()
                    ),
                };
            }).await;
        });
    }
    Ok(_) => {
        // Do nothing for non-final command
    }
    Err(e) => {
        return Err(format!("Failed to send motion command: {e}"));
    }
}
```

### After (Option 1 - Using `send_and_wait_for_completion`):
```rust
let motion_command = SendPacket::Instruction(
    Instruction::FrcLinearMotion(FrcLinearMotion::new(
        0,  // sequence_id will be assigned by driver
        fanuc_config.0.clone(),
        convert_to_position(&point.point),
        SpeedType::MilliSeconds,
        point.move_time as f64 * 1000.0,
        point.term_type.clone(),
        100,
    ))
);

if point.is_last_coast_point {
    let driver_clone = rmi_driver.clone();
    let timeout_duration = std::time::Duration::from_secs_f32(point.move_time * 1.5);

    tokio_tasks_runtime.spawn_background_task(move |mut ctx| async move {
        let completion_result = tokio::time::timeout(
            timeout_duration,
            driver_clone.0.send_and_wait_for_completion(
                motion_command.clone(),
                PacketPriority::Standard
            ),
        ).await;

        ctx.run_on_main_thread(move |ctx| {
            match completion_result {
                Ok(Ok(sequence_id)) => ctx.world.write_message(
                    new_log_message(format!("Refill coast points completed (seq {})", sequence_id)).debug()
                ),
                Ok(Err(e)) => ctx.world.write_message(
                    new_log_message(format!("Refill coast points failed: {}", e)).debug()
                ),
                Err(_) => ctx.world.write_message(
                    new_log_message("Refill coast points timed out").debug()
                ),
            };
        }).await;
    });
} else {
    // For non-final commands, just send without waiting
    rmi_driver.0.send_command(motion_command, PacketPriority::Standard)?;
}
```

### After (Option 2 - Using `wait_on_correlation_completion`):
```rust
let motion_command = SendPacket::Instruction(
    Instruction::FrcLinearMotion(FrcLinearMotion::new(
        0,  // sequence_id will be assigned by driver
        fanuc_config.0.clone(),
        convert_to_position(&point.point),
        SpeedType::MilliSeconds,
        point.move_time as f64 * 1000.0,
        point.term_type.clone(),
        100,
    ))
);

match rmi_driver.0.send_command(motion_command.clone(), PacketPriority::Standard) {
    Ok(correlation_id) if point.is_last_coast_point => {
        let driver_clone = rmi_driver.clone();
        let timeout_duration = std::time::Duration::from_secs_f32(point.move_time * 1.5);

        tokio_tasks_runtime.spawn_background_task(move |mut ctx| async move {
            let completion_result = tokio::time::timeout(
                timeout_duration,
                driver_clone.0.wait_on_correlation_completion(correlation_id),  // ✅ Real correlation ID
            ).await;

            ctx.run_on_main_thread(move |ctx| {
                match completion_result {
                    Ok(Ok(sequence_id)) => ctx.world.write_message(
                        new_log_message(format!("Refill coast points completed (seq {})", sequence_id)).debug()
                    ),
                    Ok(Err(e)) => ctx.world.write_message(
                        new_log_message(format!("Refill coast points failed: {}", e)).debug()
                    ),
                    Err(_) => ctx.world.write_message(
                        new_log_message("Refill coast points timed out").debug()
                    ),
                };
            }).await;
        });
    }
    Ok(_) => {
        // Do nothing for non-final command
    }
    Err(e) => {
        return Err(format!("Failed to send motion command: {e}"));
    }
}
```

---

## API Reference

### `send_command(packet, priority) -> Result<u64, String>`

Sends a packet to the FANUC controller and returns a correlation ID.

**Parameters:**
- `packet: SendPacket` - The packet to send (Communication, Command, or Instruction)
- `priority: PacketPriority` - Queue priority (Low, Standard, High, Immediate, Termination)

**Returns:**
- `Ok(correlation_id)` - Unique correlation ID for tracking this request
- `Err(String)` - Error message if packet could not be queued

**Example:**
```rust
let correlation_id = driver.send_command(
    SendPacket::Instruction(instruction),
    PacketPriority::Standard
)?;
```

---

### `wait_on_instruction_completion(sequence_id: u32)`

Waits for an instruction with the given sequence ID to complete.

**Parameters:**
- `sequence_id: u32` - The sequence ID to wait for

**Behavior:**
- Polls the completed packet channel every 10ms
- Returns when `sequence_id >= target` or an error occurs
- Prints error message if `error_id != 0`

**Example:**
```rust
driver.wait_on_instruction_completion(42).await;
```

---

### `wait_on_correlation_completion(correlation_id: u64) -> Result<u32, String>`

Waits for an instruction to complete using its correlation ID.

**Parameters:**
- `correlation_id: u64` - The correlation ID from `send_command()`

**Returns:**
- `Ok(sequence_id)` - The sequence ID that was assigned to the instruction
- `Err(String)` - Error if sent notification was not received

**Example:**
```rust
let correlation_id = driver.send_command(packet, PacketPriority::Standard)?;
let sequence_id = driver.wait_on_correlation_completion(correlation_id).await?;
println!("Instruction {} completed", sequence_id);
```

---

### `send_and_wait_for_completion(packet, priority) -> Result<u32, String>`

Sends an instruction and waits for it to complete in one call.

**Parameters:**
- `packet: SendPacket` - The packet to send
- `priority: PacketPriority` - Queue priority

**Returns:**
- `Ok(sequence_id)` - The sequence ID that was assigned
- `Err(String)` - Error if send or wait failed

**Example:**
```rust
let sequence_id = driver.send_and_wait_for_completion(
    SendPacket::Instruction(instruction),
    PacketPriority::Standard
).await?;
```

---

### `sent_instruction_tx: broadcast::Sender<SentInstructionInfo>`

Broadcast channel that emits notifications when instructions are sent.

**Subscribe:**
```rust
let mut sent_rx = driver.sent_instruction_tx.subscribe();
```

**Receive:**
```rust
while let Ok(sent_info) = sent_rx.recv().await {
    println!("Correlation {} -> Sequence {}",
        sent_info.correlation_id,
        sent_info.sequence_id
    );
}
```

**SentInstructionInfo fields:**
- `correlation_id: u64` - Matches return value from `send_command()`
- `sequence_id: u32` - Actual sequence ID assigned by driver
- `timestamp: Instant` - When the instruction was sent

---

## Deprecated Functions

### `wait_on_command_completion(packet_number_to_wait_for: u32)`

**Status:** Deprecated since v0.1.0

**Replacement:** Use `wait_on_instruction_completion()` instead

This function is kept for backward compatibility but will be removed in a future version. It simply calls `wait_on_instruction_completion()` internally.

---

## Summary of Changes

| What | Before | After |
|------|--------|-------|
| **Return type** | `Result<u32, String>` | `Result<u64, String>` |
| **Return value** | `0` (placeholder) | Unique correlation ID |
| **Tracking** | Not possible | Subscribe to `sent_instruction_tx` |
| **Wait function** | `wait_on_command_completion()` | `wait_on_instruction_completion()` |
| **New helpers** | None | `wait_on_correlation_completion()`<br>`send_and_wait_for_completion()` |

---

## Troubleshooting

### "I'm getting type errors about u32 vs u64"

**Problem:** Your code expects `send_command()` to return `u32` but it now returns `u64`.

**Solution:** Update your variable types or use one of the helper functions:
```rust
// Old:
let command_id: u32 = driver.send_command(...)?;

// New Option 1:
let correlation_id: u64 = driver.send_command(...)?;

// New Option 2:
let sequence_id: u32 = driver.send_and_wait_for_completion(...).await?;
```

---

### "My code was using the sequence ID immediately"

**Problem:** You were using the return value from `send_command()` as a sequence ID, but it was always `0`.

**Solution:** This was already broken! Use one of these approaches:
1. Use `send_and_wait_for_completion()` to get the real sequence ID
2. Use `wait_on_correlation_completion()` to convert correlation ID to sequence ID
3. Subscribe to `sent_instruction_tx` to receive sequence IDs as they're assigned

---

### "I need to track multiple instructions at once"

**Problem:** You're sending multiple instructions and need to track all of them.

**Solution:** Use Strategy 3 (Manual Correlation) or create a helper struct:
```rust
struct InstructionTracker {
    pending: HashMap<u64, oneshot::Sender<u32>>,
}

impl InstructionTracker {
    async fn track(&mut self, driver: &FanucDriver, correlation_id: u64) -> u32 {
        let (tx, rx) = oneshot::channel();
        self.pending.insert(correlation_id, tx);

        // Background task listens to sent_instruction_tx and resolves pending requests
        rx.await.unwrap()
    }
}
```

---

## Questions?

If you have questions about migrating your code, please:
1. Review the examples in this guide
2. Check the API reference section
3. Look at the real-world Meteorite example
4. Open an issue on GitHub with your specific use case


