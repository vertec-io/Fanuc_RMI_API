# Issue Analysis - v0.5.0 Post-Release

**Date**: 2025-11-25  
**Status**: Critical Issues Identified

---

## 🔴 Issue 1: Simulator Response Format Mismatch

### Problem
Simulator sends responses that don't match our protocol types, causing deserialization errors:

```
[ERROR] Invalid JSON (data did not match any variant of untagged enum ResponsePacket): 
{"Command":"FRC_ReadCartesianPosition","Configuration":{...},"ErrorID":0,"Group":1,"Position":{...},"TimeTag":0}

[ERROR] Invalid JSON (data did not match any variant of untagged enum ResponsePacket): 
{"Command":"FRC_GetStatus","ErrorID":0,"NextSequenceID":1,...,"TPMode":1}
```

### Root Cause
**FrcGetStatusResponse** expects `Override` field (line 27 in frc_getstatus.rs):
```rust
#[serde(rename = "Override")]
pub override_value: u32,
```

**Simulator** doesn't send it (line 200-214 in sim/src/main.rs):
```rust
Some("FRC_GetStatus") => {
    json!({
        "Command": "FRC_GetStatus",
        "ErrorID": 0,
        "ServoReady": 1,
        "TPMode": 1,
        // ... missing "Override" field!
    })
}
```

### Solution
Add missing `Override` field to simulator responses.

---

## 🔴 Issue 2: Improper Initialization Sequence

### Problem
Real robot sometimes returns "RMICommandFailed" when `abort()` is called during startup. If abort fails, `initialize()` also fails.

### Observed Behavior
- Turn controller off/on → connect → abort() fails → initialize() fails
- Commenting out abort() → initialize() works

### Root Cause
**Current sequence** (web_server/src/main.rs lines 43-67):
```rust
driver.abort().await;  // May fail if robot not in right state!
driver.initialize().await;
```

**Problem**: We blindly call abort() without checking robot status first.

### Proper Sequence (from FANUC manual)
Need to research B-84184EN_02.pdf for correct sequence based on robot state.

**Likely correct sequence**:
1. Connect
2. **Get status first** - check if robot needs abort
3. If RMI already initialized → abort first
4. If RMI not initialized → skip abort
5. Initialize
6. Handle failures gracefully (disconnect/reconnect if needed)

---

## 🔴 Issue 3: WebSocket URL Change Doesn't Affect Robot Connection

### Problem
Changing "Robot IP" and "Robot Port" in UI settings does nothing - only WebSocket URL changes.

### Root Cause Analysis

**UI Side** (web_app/src/components/settings.rs lines 135-154):
```rust
on:click={
    let ws = ws.clone();
    move |_| {
        let new_ws_url = ws_url.get();
        // ... validation ...
        ws.reconnect(&new_ws_url);  // Only changes WebSocket URL!
        // robot_ip and robot_port are NEVER sent to backend!
    }
}
```

**Backend Side** (web_server/src/main.rs lines 19-38):
```rust
let driver_config = FanucDriverConfig {
    addr: "127.0.0.1".to_string(),  // HARDCODED!
    port: 16001,                     // HARDCODED!
    max_messages: 30,
    log_level: LogLevel::Error,
};

let driver = FanucDriver::connect(driver_config).await;
// Driver is created ONCE at startup, never recreated!
```

**Problem**: 
1. Robot address is hardcoded in web_server
2. UI changes only affect WebSocket connection (frontend ↔ web_server)
3. Robot connection (web_server ↔ robot) is never changed
4. No mechanism to send robot config changes from UI to backend

### Architecture Issue
```
[UI] --WebSocket--> [web_server] --TCP--> [Robot]
  ↑                      ↑                    ↑
  |                      |                    |
  Changes WS URL    Hardcoded config    Never changes!
```

---

## 🔴 Issue 4: No Environment Variable Support

### Problem
Initial robot address not loaded from environment variables, not propagated to UI.

### Current State
**Backend** (web_server/src/main.rs):
- Hardcoded: `addr: "127.0.0.1"`, `port: 16001`
- No env var support

**Frontend** (web_app/src/websocket.rs line 38):
- Hardcoded: `ws_url: store_value("ws://127.0.0.1:9000".to_string())`
- No env var support

### Expected Behavior
1. Backend reads `FANUC_ROBOT_ADDR` and `FANUC_ROBOT_PORT` env vars
2. Backend reads `WEBSOCKET_PORT` env var
3. Initial values propagated to UI on connection
4. UI displays current robot address in settings

---

## 📋 Solution Plan

### Priority 1: Fix Simulator (Immediate)
- Add `Override` field to FRC_GetStatus response
- Verify all response types match protocol definitions

### Priority 2: Smart Initialization (Critical)
- Research B-84184EN_02.pdf for proper sequence
- Implement `startup_sequence()` helper method:
  ```rust
  pub async fn startup_sequence(&self) -> Result<(), String> {
      // 1. Get status
      // 2. Check if abort needed
      // 3. Abort if needed (handle failures)
      // 4. Initialize
      // 5. Retry logic if needed
  }
  ```

### Priority 3: Dynamic Robot Connection (Important)
- Add message protocol for robot config changes
- Implement disconnect/reconnect logic in web_server
- Send robot config from UI to backend
- Handle connection state properly

### Priority 4: Environment Variables (Nice to have)
- Add env var support to web_server
- Propagate initial config to UI
- Document environment variables

---

## 🎯 Next Steps

1. Fix simulator response format
2. Research FANUC manual for initialization sequence
3. Implement smart initialization helper
4. Design robot config change protocol
5. Implement dynamic robot connection

