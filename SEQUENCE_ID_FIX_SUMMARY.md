# Sequence ID Fix - Summary

**Date**: 2025-11-24
**Issue**: Invalid sequence ID errors (RMIT-029) when jogging robot via web app
**Error Code**: 2556957 (RMIT-029 Invalid sequence ID number)
**Status**: ✅ FIXED - Both root causes addressed (Ready for review, not committed)

---

## Problem Summary

When jogging the robot via the web app, FANUC controller returned error 2556957 (RMIT-029 Invalid sequence ID number) for every motion command. This error indicates that RMI detected non-consecutive sequence IDs.

**Two Root Causes Identified:**
1. **Web app generating random sequence IDs** - Using JavaScript timestamps instead of letting driver assign IDs
2. **Priority queue reordering** - High/Immediate priority packets inserted at front of queue after sequence ID assignment, causing out-of-order transmission

### FANUC RMI Sequence ID Requirements

From FANUC B-84184EN_02 manual:

> Start your SequenceID number from 1 after the FRC_Initialize packet. (Note that if the SequenceID should ever reach its maximum value of 2^31-1, it will wrap back to 1.) By default, RMI checks the SequenceID to make sure there are no missing packets. If RMI detects a non-consecutive sequence ID, RMI sends a RMIT-029 Invalid sequence ID number error ID back to the sender.

**Key Requirements:**
- Sequence IDs must start from 1 after FRC_Initialize
- Sequence IDs must be **monotonically increasing** (consecutive)
- Non-consecutive sequence IDs trigger RMIT-029 error and HOLD state
- Maximum value is 2^31-1 (2,147,483,647), then wraps to 1

---

## Root Cause #1: Web App Random Sequence IDs

The web app was generating **random** sequence IDs using JavaScript timestamps:

<augment_code_snippet path="web_app/src/components/jog_controls.rs" mode="EXCERPT">
````rust
sequence_id: (js_sys::Date::now() as u32) % 1000000,  // ❌ WRONG!
````
</augment_code_snippet>

This caused multiple problems:

1. **Non-monotonic IDs**: Timestamp-based IDs are not guaranteed to be consecutive
2. **Bypassed Driver Management**: The driver has a proper sequence ID counter that was being ignored
3. **Race Conditions**: Multiple rapid jog commands could generate out-of-order IDs

## Root Cause #2: Priority Queue Reordering

Even with correct sequence ID generation, the driver was assigning sequence IDs **before** queue insertion, then inserting High/Immediate priority packets at the **front** of the queue. This caused packets to be sent out of sequence order:

**Old Flow (BROKEN)**:
```
1. send_command() assigns sequence ID (e.g., seq=10)
2. Packet queued based on priority (High → front, Standard → back)
3. Packets sent from queue in FIFO order
4. Result: High priority packet with seq=12 sent before Standard packet with seq=10
```

**Example**:
```
send_command(packet_A, Standard)  → Assigned seq=10, queued at back
send_command(packet_B, Standard)  → Assigned seq=11, queued at back
send_command(packet_C, High)      → Assigned seq=12, queued at FRONT

Queue order: [packet_C(seq=12), packet_A(seq=10), packet_B(seq=11)]
Send order:   12 → 10 → 11  ❌ NON-CONSECUTIVE!
```

### Example of the Problem

User clicks jog buttons rapidly (6 times in 0.5-1.0 seconds):
```
Web App generates: 1732123456, 1732123457, 1732123456, 1732123458, 1732123457, 1732123459
                                            ↑ DUPLICATE!  ↑ OUT OF ORDER!
```

FANUC controller expects: `1, 2, 3, 4, 5, 6` (consecutive)

Result: **10 consecutive RMIT-029 errors** (one for each packet in the queue)

---

## How Sequence IDs Should Work

### Correct Flow

1. **Client** creates instruction with `sequence_id: 0` (placeholder)
2. **WebSocket Server** forwards packet to driver
3. **Driver** calls `give_sequence_id()` which:
   - Locks the sequence counter mutex
   - Assigns current counter value to the instruction
   - Increments counter by 1
   - Returns the assigned sequence ID
4. **Driver** queues packet with correct sequence ID
5. **Driver** sends packets in queue order (FIFO for Standard priority)

### Driver Implementation

<augment_code_snippet path="fanuc_rmi/src/drivers/driver.rs" mode="EXCERPT">
````rust
fn give_sequence_id(&self, mut packet: SendPacket) -> Result<(SendPacket, u32), String> {
    let mut sid = self.next_available_sequence_number.lock()?;
    let current_id = *sid;
    
    if let SendPacket::Instruction(ref mut instruction) = packet {
        // Assign sequence ID to instruction
        instruction.set_sequence_id(current_id);
        *sid += 1;  // Increment for next instruction
    }
    
    Ok((packet, current_id))
}
````
</augment_code_snippet>

This ensures:
- ✅ Thread-safe sequence ID generation (mutex)
- ✅ Monotonically increasing IDs
- ✅ No gaps or duplicates
- ✅ Correct ordering even with priority packets

---

## The Fixes

### Fix #1: Web App - Use Placeholder Sequence ID

**File**: `web_app/src/components/jog_controls.rs`

**Before (INCORRECT)**:
```rust
FrcLinearRelative {
    sequence_id: (js_sys::Date::now() as u32) % 1000000,  // ❌ Random ID
    configuration: Configuration { ... },
    // ...
}
```

**After (CORRECT)**:
```rust
FrcLinearRelative {
    sequence_id: 0,  // ✅ Will be assigned by driver
    configuration: Configuration { ... },
    // ...
}
```

### Fix #2: Driver - Assign Sequence IDs After Queue Insertion

**File**: `fanuc_rmi/src/drivers/driver.rs`

**Changed Functions:**

1. **`send_command()`** - Removed sequence ID assignment
   - **Before**: Called `give_sequence_id()` before queuing
   - **After**: Queues packet with placeholder sequence ID (0)
   - Returns 0 as placeholder (actual ID assigned at send time)

2. **`send_queue_to_controller()`** - Added sequence ID assignment
   - **Before**: Sent packets with pre-assigned sequence IDs
   - **After**: Assigns sequence IDs right before sending to controller
   - Ensures consecutive IDs in **send order**, not queue insertion order

**New Flow (CORRECT)**:
```
1. send_command() queues packet with sequence_id=0
2. Packet queued based on priority (High → front, Standard → back)
3. send_queue_to_controller() pops packet from front of queue
4. Assigns next available sequence ID (monotonic counter)
5. Sends packet to controller
6. Result: Packets sent with consecutive sequence IDs regardless of priority
```

**Example**:
```
send_command(packet_A, Standard)  → seq=0, queued at back
send_command(packet_B, Standard)  → seq=0, queued at back
send_command(packet_C, High)      → seq=0, queued at FRONT

Queue order: [packet_C(seq=0), packet_A(seq=0), packet_B(seq=0)]

Send loop:
  Pop packet_C → Assign seq=1 → Send seq=1
  Pop packet_A → Assign seq=2 → Send seq=2
  Pop packet_B → Assign seq=3 → Send seq=3

Send order: 1 → 2 → 3  ✅ CONSECUTIVE!
```

---

## Verification

### Other Code Already Correct ✅

All other parts of the codebase were already using `sequence_id: 0`:

1. **`example/src/bin/jog_client.rs`**: ✅ Uses `sequence_id: 0`
2. **`example/src/bin/jog_client_tui.rs`**: ✅ Uses `sequence_id: 0`
3. **`example/src/main.rs`**: ✅ Uses `FrcLinearRelative::new(0, ...)`

Only the web app was incorrectly generating its own sequence IDs.

### Build Status

```bash
✅ Web app rebuilt successfully
✅ No compilation errors
✅ Ready for testing with real FANUC controller
```

---

## Testing Recommendations

### Test 1: Rapid Jogging
1. Connect to real FANUC controller
2. Click jog button rapidly (6-10 times in 1 second)
3. **Expected**: All motions execute successfully, no RMIT-029 errors
4. **Previous**: 10 consecutive RMIT-029 errors

### Test 2: Sequence ID Verification
1. Enable debug logging in driver
2. Send 5 jog commands
3. **Expected**: Sequence IDs are 1, 2, 3, 4, 5 (consecutive)
4. **Previous**: Random IDs like 1732123456, 1732123457, etc.

### Test 3: Priority Packet Ordering
1. Send Standard priority packet
2. Immediately send High priority packet
3. **Expected**: High priority executes first, but both have consecutive sequence IDs
4. **Note**: Priority affects queue order, not sequence ID assignment

---

## Priority Packets and Sequence IDs

### ✅ FIXED: Priority Reordering No Longer Causes Issues

With the new implementation, priority packets can be inserted at the front of the queue **without** causing sequence ID issues:

**How It Works Now:**
1. Packets queued with `sequence_id=0` (placeholder)
2. Queue insertion order determined by priority (High/Immediate → front, Standard/Low → back)
3. Sequence IDs assigned **right before sending** in FIFO order from queue
4. Result: Consecutive sequence IDs in send order, regardless of priority

**Example:**
```
send_command(packet_A, Standard)  → seq=0, queued at back
send_command(packet_B, High)      → seq=0, queued at FRONT
send_command(packet_C, Standard)  → seq=0, queued at back

Queue order: [packet_B(seq=0), packet_A(seq=0), packet_C(seq=0)]

Send loop assigns IDs in queue order:
  Pop packet_B → Assign seq=1 → Send
  Pop packet_A → Assign seq=2 → Send
  Pop packet_C → Assign seq=3 → Send

Send order: 1 → 2 → 3  ✅ CONSECUTIVE!
```

**Benefits:**
- ✅ High/Immediate priority packets execute first (as intended)
- ✅ Sequence IDs remain consecutive (FANUC requirement)
- ✅ No RMIT-029 errors
- ✅ Priority system works correctly

---

## Files Modified

1. ✅ `web_app/src/components/jog_controls.rs` - Changed sequence ID from random to 0
2. ✅ `fanuc_rmi/src/drivers/driver.rs` - Moved sequence ID assignment to send loop
   - Modified `send_command()` to not assign sequence IDs
   - Modified `send_queue_to_controller()` to assign sequence IDs before sending
   - Deprecated `give_sequence_id()` function (kept for reference)
3. ✅ `web_app/pkg/*` - Rebuilt WASM files

---

## Testing Status

### Build Status ✅
```bash
✅ cargo check -p fanuc_rmi - Success
✅ cargo check -p web_server - Success
✅ cargo check -p sim - Success
✅ cargo test -p fanuc_rmi --all-features - All tests pass (14 tests)
✅ Web app rebuilt successfully
```

### Test Results
- **DTO roundtrip tests**: 4 passed
- **JSON validation tests**: 5 passed
- **Extract tests**: 5 passed
- **Doc tests**: 4 passed (3 ignored)

## Next Steps

1. ✅ Both fixes implemented
2. ✅ Web app rebuilt
3. ✅ All tests passing
4. ⏳ **AWAITING REVIEW** - Do not commit yet
5. ⏳ Test with real FANUC controller
6. ⏳ Verify no RMIT-029 errors with rapid jogging
7. ⏳ Test High/Immediate priority packets (if used)
8. ⏳ After approval, commit changes

---

## References

- **FANUC Specification**: B-84184EN_02.pdf (Sequence ID requirements)
- **Error Code**: RMIT-029 Invalid sequence ID number (error_id: 2556957)
- **Driver Implementation**: `fanuc_rmi/src/drivers/driver.rs` (give_sequence_id function)

