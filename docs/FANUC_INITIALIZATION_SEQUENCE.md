# FANUC RMI Initialization Sequence

**Source**: B-84184EN_02.pdf Section 2.3.1  
**Date**: 2025-11-25

---

## 📋 Proper Initialization Sequence

### According to FANUC Manual

**Before sending FRC_Initialize**, ensure the robot controller is in the following state:

1. ✅ The teach pendant is disabled and the controller is in **AUTO mode**
2. ✅ The controller is **ready to run** (no servo or other errors)
3. ✅ The selected TP program is **NOT RMI_MOVE**

### Recommended Sequence

```
1. FRC_Connect
   ↓
2. FRC_GetStatus (check current state)
   ↓
3. Decision based on status:
   
   If RMI_MOVE is running (RMIMotionStatus != 0):
      → FRC_Abort first
      → Wait for response
      → Then FRC_Initialize
   
   If RMI_MOVE is not running (RMIMotionStatus == 0):
      → Skip abort
      → FRC_Initialize directly
   
   If errors present (ServoReady != 1):
      → Handle errors first
      → May need FRC_Reset
      → Then retry initialization
```

---

## 🔴 Common Errors

### Error: RMI Command Failed on Abort

**Cause**: Calling `FRC_Abort` when RMI_MOVE is not running

**Solution**: Check status first using `FRC_GetStatus`

**From Manual**:
> "Sending the abort packet to the robot controller allows you to abort the current running RMI_MOVE TP program."

**Key Point**: Abort only works if RMI_MOVE is actually running!

### Error: Initialize Failed (ErrorID = 7015)

**Cause (per manual)**: RMI_MOVE program is selected on teach pendant.

**Not the only cause.** Reproduced on an R-30iB (COMET1) with every documented
precondition satisfied — `ServoReady: 1`, `TPMode: 0`, `RMIMotionStatus: 0`,
`FRC_ReadError` reporting `"No Error"` — after a cold reboot, with RMI_MOVE
deselected on the pendant. Verified live that it survives `FRC_Abort`,
`FRC_Reset`, and abort+reset, on both GroupMask `0b01` and `0b11`. No RMI
command clears it. See `probe/FINDINGS-7015.md` and reproduce with
`cargo run -p fanuc_probe --bin init_diag -- --addr <ip>:16001`.

**Solution** (from manual):
> "Press the SELECT button on the TP, choose another program besides RMI_MOVE from the program list, then press the ENTER button on the TP. Next, resend the FRC_Initialize command."

---

## 📊 FRC_GetStatus Response Fields

```rust
pub struct FrcGetStatusResponse {
    pub error_id: u32,
    pub servo_ready: i8,        // 1 = ready, 0 = not ready
    pub tp_mode: i8,             // 0 = teach pendant DISABLED (required by RMI), 1 = enabled
    pub rmi_motion_status: i8,   // 0 = not running, != 0 = running
    pub program_status: i8,      // 1 = aborted
    pub single_step_mode: i8,    // 1 = single-step mode
    pub number_utool: i8,
    pub number_uframe: i8,
    pub next_sequence_id: u32,
    pub override_value: u32,
}
```

**Key Fields for Initialization**:
- `servo_ready`: Must be 1 (controller ready)
- `tp_mode`: Must be **0** — the teach pendant must be DISABLED (B-84184EN/02). 1 means the pendant is enabled and RMI will not run.
- `rmi_motion_status`: 0 = can initialize, != 0 = need abort first
- `program_status`: 1 = RMI_MOVE is aborted

---

## 🎯 Smart Initialization Logic

```rust
pub async fn startup_sequence(&self) -> Result<(), String> {
    // Step 1: Get current status
    let status = self.get_status().await?;
    
    if status.error_id != 0 {
        return Err(format!("Get status failed: {}", status.error_id));
    }
    
    // Step 2: Check if controller is ready
    if status.servo_ready != 1 {
        return Err("Controller not ready (servo errors)".to_string());
    }
    
    if status.tp_mode != 0 {
        return Err("Teach pendant is enabled; RMI requires it disabled".to_string());
    }
    
    // Step 3: Abort if RMI is already running
    if status.rmi_motion_status != 0 {
        log::info!("RMI already running, aborting first...");
        let abort_response = self.abort().await?;
        if abort_response.error_id != 0 {
            return Err(format!("Abort failed: {}", abort_response.error_id));
        }
    }
    
    // Step 4: Initialize
    let init_response = self.initialize().await?;
    if init_response.error_id != 0 {
        return Err(format!("Initialize failed: {}", init_response.error_id));
    }
    
    Ok(())
}
```

---

## ⚠️ Important Notes from Manual

### 1. Always End Session Properly

> "Please always end your RMI session with either an FRC_Abort or FRC_Disconnect packet. This will ensure you can execute other TP programs after the RMI session."

### 2. Wait for Responses

> "It takes the robot controller some time to initialize the system to accept the TP program instructions. Please wait until you have received the return packet before sending the next packet to the controller."

**Our async methods handle this automatically with 5-second timeout!**

### 3. Abort is Required After Initialize

> "If the FRC_Initialize command is executed successfully, you will have to send an FRC_Abort command to terminate the RMI program in order for another TP program to run."

---

## 🔧 Error Recovery

### Recoverable Errors

For some warning errors:
1. Send `FRC_Reset` packet
2. Wait for response
3. Send `FRC_Continue` to resume

### Non-Recoverable Errors

For critical errors:
1. Send `FRC_Abort` to terminate RMI_MOVE
2. Send `FRC_Initialize` to create new program
3. Re-add required instructions

---

## 📝 Implementation Plan

### Phase 1: Add startup_sequence() method
- Implement smart initialization logic
- Check status before abort
- Handle errors gracefully

### Phase 2: Update web_server
- Replace blind abort/initialize with startup_sequence()
- Add proper error logging
- Handle initialization failures

### Phase 3: Add retry logic (optional)
- Retry on transient errors
- Exponential backoff
- Maximum retry count

