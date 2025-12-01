# FANUC RMI Commands Reference

**Comprehensive Reference for All Remote Motion Interface Commands**

---

## Table of Contents

1. [Command Categories](#command-categories)
2. [Communication Packets](#communication-packets)
3. [Robot Control Commands](#robot-control-commands)
4. [Status and Information Commands](#status-and-information-commands)
5. [Frame and Tool Commands](#frame-and-tool-commands)
6. [Motion Instructions](#motion-instructions)
7. [I/O Commands](#io-commands)
8. [Advanced Commands](#advanced-commands)
9. [Command Availability Matrix](#command-availability-matrix)

---

## Command Categories

FANUC RMI commands are organized into three main categories:

### 1. Communication Packets
Manage connection lifecycle between client and robot controller.

### 2. Command Packets
Immediate actions that don't add instructions to the TP program (e.g., read status, change settings).

### 3. Instruction Packets
Add motion/logic instructions to the RMI_MOVE TP program for execution.

---

## Communication Packets

### FRC_Connect

**Purpose**: Establish connection to robot controller

**When to use**: First command after TCP connection established

**Request**:
```json
{"Communication": "FRC_Connect"}
```

**Response**:
```json
{"Communication": "FRC_Connect", "ErrorID": 0}
```

**Rust API**:
```rust
driver.connect().await?;
```

**Notes**:
- Must be first command sent
- Connection times out after 60 minutes of inactivity
- Only one client can connect at a time

---

### FRC_Disconnect

**Purpose**: Gracefully close connection to robot controller

**When to use**: Before closing TCP connection, or when done with robot

**Request**:
```json
{"Communication": "FRC_Disconnect"}
```

**Response**:
```json
{"Communication": "FRC_Disconnect", "ErrorID": 0}
```

**Rust API**:
```rust
driver.disconnect().await?;
```

**Notes**:
- Automatically aborts RMI_MOVE program if running
- Releases robot for other clients
- Always disconnect gracefully when possible

---

### FRC_Terminate

**Purpose**: Robot-initiated connection termination (timeout)

**When to use**: You don't send this - robot sends it to you

**Packet from Robot**:
```json
{"Communication": "FRC_Terminate"}
```

**Notes**:
- Sent by robot after 60 minutes of inactivity
- No response expected
- Must reconnect with FRC_Connect

---

### FRC_SystemFault

**Purpose**: Robot-initiated fault notification

**When to use**: You don't send this - robot sends it to you

**Packet from Robot**:
```json
{"Communication": "FRC_SystemFault", "SequenceID": 42}
```

**Notes**:
- Sent when robot encounters error during instruction execution
- SequenceID indicates which instruction caused the fault
- Check robot teach pendant for error details

---

## Robot Control Commands

### FRC_Initialize

**Purpose**: Start RMI session and create RMI_MOVE TP program

**When to use**: After FRC_Connect, before sending motion instructions

**Prerequisites**:
- Robot in AUTO mode (teach pendant disabled)
- Servo ready (no errors)
- RMI_MOVE program NOT selected on TP

**Request**:
```json
{"Command": "FRC_Initialize", "GroupMask": 1}
```

**Response**:
```json
{"Command": "FRC_Initialize", "ErrorID": 0, "GroupMask": 1}
```

**Rust API**:
```rust
let response = driver.initialize().await?;
if response.error_id != 0 {
    eprintln!("Initialize failed: {}", response.error_id);
}
```

**Common Errors**:
- **7015**: RMI_MOVE program is selected on TP (select different program)
- **7001**: Robot not in AUTO mode
- **7002**: Servo not ready

---

### FRC_Abort

**Purpose**: Stop RMI_MOVE program and clear instruction buffer

**When to use**: 
- Emergency stop
- Cancel queued instructions
- Before disconnecting

**Prerequisites**:
- RMI must be running (RMIMotionStatus != 0)

**Request**:
```json
{"Command": "FRC_Abort"}
```

**Response**:
```json
{"Command": "FRC_Abort", "ErrorID": 0}
```

**Rust API**:
```rust
driver.abort().await?;
```

**Notes**:
- Only works if RMI_MOVE is running
- Calling abort when RMI not running returns error
- Use `startup_sequence()` for smart initialization

---

### FRC_Continue

**Purpose**: Resume RMI_MOVE program after pause

**When to use**: After robot was paused (e.g., by teach pendant)

**Request**:
```json
{"Command": "FRC_Continue"}
```

**Response**:
```json
{"Command": "FRC_Continue", "ErrorID": 0}
```

**Rust API**:
```rust
driver.continue_motion().await?;
```

---

### FRC_Reset

**Purpose**: Reset robot controller errors

**When to use**: After robot fault, to clear error state

**Availability**: V9.30P/19+ or V9.40P/18+

**Request**:
```json
{"Command": "FRC_Reset"}
```

**Response**:
```json
{"Command": "FRC_Reset", "ErrorID": 0}
```

**Notes**:
- Not available on all controller versions
- Check version before using

---

## Status and Information Commands

### FRC_GetStatus

**Purpose**: Read robot controller status

**When to use**: 
- Check if robot is ready
- Monitor RMI state
- Get next sequence ID

**Request**:
```json
{"Command": "FRC_GetStatus"}
```

**Response**:
```json
{
    "Command": "FRC_GetStatus",
    "ErrorID": 0,
    "ServoReady": 1,
    "TPMode": 1,
    "RMIMotionStatus": 0,
    "ProgramStatus": 0,
    "SingleStepMode": 0,
    "NumberUTool": 10,
    "NumberUFrame": 9,
    "NextSequenceID": 1,
    "Override": 100
}
```

**Rust API**:
```rust
let status = driver.get_status().await?;
println!("Servo ready: {}", status.servo_ready);
println!("RMI running: {}", status.rmi_motion_status);
println!("Next sequence ID: {}", status.next_sequence_id);
```

**Response Fields**:
- **ServoReady**: 1 = ready, 0 = not ready (errors present)
- **TPMode**: 1 = AUTO mode, 0 = MANUAL mode
- **RMIMotionStatus**: 0 = not running, 1 = running, 2 = paused
- **ProgramStatus**: 0 = not running, 1 = running, 2 = paused
- **SingleStepMode**: 1 = enabled, 0 = disabled
- **NumberUTool**: Number of available user tools (0-10)
- **NumberUFrame**: Number of available user frames (0-9)
- **NextSequenceID**: Next sequence ID to use for instructions
- **Override**: Speed override percentage (0-100)

---

### FRC_ReadCartesianPosition

**Purpose**: Read current robot TCP position in Cartesian coordinates

**When to use**:
- Display robot position in UI
- Record waypoints
- Verify robot location

**Request**:
```json
{"Command": "FRC_ReadCartesianPosition", "Group": 1}
```

**Response**:
```json
{
    "Command": "FRC_ReadCartesianPosition",
    "ErrorID": 0,
    "TimeTag": 441803,
    "Group": 1,
    "Configuration": {
        "UToolNumber": 4,
        "UFrameNumber": 3,
        "Front": 1,
        "Up": 1,
        "Left": 0,
        "Flip": 1,
        "Turn4": 0,
        "Turn5": 0,
        "Turn6": 0
    },
    "Position": {
        "X": 6.022,
        "Y": -35.690,
        "Z": 26.048,
        "W": 0.002327,
        "P": 0.002024,
        "R": 179.999,
        "Ext1": 0.0,
        "Ext2": 0.0,
        "Ext3": 0.0
    }
}
```

**Rust API**:
```rust
let pos = driver.read_cartesian_position().await?;
println!("Position in UFrame {}: X={:.3}, Y={:.3}, Z={:.3}",
    pos.config.u_frame_number, pos.pos.x, pos.pos.y, pos.pos.z);
```

**Important Notes**:
- **Position is in the currently active UFrame**, not World Frame!
- Check `Configuration.UFrameNumber` to know which frame
- To get World Frame position, either:
  - Set active frame to 0 first, OR
  - Read UFrame data and transform coordinates

---

### FRC_ReadJointAngles

**Purpose**: Read current robot joint angles

**When to use**:
- Monitor joint positions
- Check for joint limits
- Record joint configurations

**Request**:
```json
{"Command": "FRC_ReadJointAngles", "Group": 1}
```

**Response**:
```json
{
    "Command": "FRC_ReadJointAngles",
    "ErrorID": 0,
    "TimeTag": 123456,
    "Group": 1,
    "JointAngles": {
        "J1": 45.123,
        "J2": -30.456,
        "J3": 60.789,
        "J4": 0.000,
        "J5": 90.000,
        "J6": 180.000,
        "J7": 0.000,
        "J8": 0.000,
        "J9": 0.000
    }
}
```

**Rust API**:
```rust
let joints = driver.read_joint_angles().await?;
println!("J1={:.3}, J2={:.3}, J3={:.3}",
    joints.joint_angles.j1, joints.joint_angles.j2, joints.joint_angles.j3);
```

---

### FRC_ReadTCPSpeed

**Purpose**: Read current tool center point speed

**When to use**: Monitor robot motion speed

**Request**:
```json
{"Command": "FRC_ReadTCPSpeed", "Group": 1}
```

**Response**:
```json
{
    "Command": "FRC_ReadTCPSpeed",
    "ErrorID": 0,
    "TCPSpeed": 125.5,
    "Group": 1
}
```

**Notes**:
- Speed in mm/s
- Returns 0 when robot is stopped

---

## Frame and Tool Commands

### FRC_GetUFrameUTool

**Purpose**: Get currently active UFrame and UTool numbers

**When to use**: Check which frame/tool is active before motion

**Request**:
```json
{"Command": "FRC_GetUFrameUTool", "Group": 1}
```

**Response**:
```json
{
    "Command": "FRC_GetUFrameUTool",
    "ErrorID": 0,
    "UFrameNumber": 3,
    "UToolNumber": 1,
    "Group": 1
}
```

**Rust API**:
```rust
let frame_tool = driver.get_uframe_utool().await?;
println!("Active UFrame: {}, UTool: {}",
    frame_tool.u_frame_number, frame_tool.u_tool_number);
```

---

### FRC_SetUFrameUTool

**Purpose**: Set active UFrame and UTool numbers

**When to use**:
- Change coordinate frame for motion
- Switch between tools
- Match teach pendant frame

**⚠️ WARNING**: Do NOT call while robot is moving!

**Request**:
```json
{
    "Command": "FRC_SetUFrameUTool",
    "UFrameNumber": 0,
    "UToolNumber": 1,
    "Group": 1
}
```

**Response**:
```json
{
    "Command": "FRC_SetUFrameUTool",
    "ErrorID": 0,
    "Group": 1
}
```

**Rust API**:
```rust
// Check robot is stopped first
let status = driver.get_status().await?;
if status.rmi_motion_status == 0 {
    driver.set_uframe_utool(0, 1).await?;  // UFrame 0, UTool 1
} else {
    eprintln!("Cannot change frame while robot is moving!");
}
```

**Safety Notes**:
- Affects ALL subsequent motion commands
- Can cause frame mismatch errors if changed during motion
- Always verify robot is stopped first

---

### FRC_ReadUFrameData

**Purpose**: Read transformation data for a specific UFrame

**When to use**:
- Get frame transformation for coordinate conversion
- Display frame information in UI
- Validate frame setup

**Request**:
```json
{
    "Command": "FRC_ReadUFrameData",
    "FrameNumber": 3,
    "Group": 1
}
```

**Response**:
```json
{
    "Command": "FRC_ReadUFrameData",
    "ErrorID": 0,
    "UFrameNumber": 3,
    "Frame": {
        "X": 1000.0,
        "Y": 300.0,
        "Z": 100.0,
        "W": 0.0,
        "P": 0.0,
        "R": 90.0
    },
    "Group": 1
}
```

**Rust API**:
```rust
let uframe = driver.read_uframe_data(3).await?;
println!("UFrame 3: X={}, Y={}, Z={}, W={}, P={}, R={}",
    uframe.frame.x, uframe.frame.y, uframe.frame.z,
    uframe.frame.w, uframe.frame.p, uframe.frame.r);
```

**Use Case**: Coordinate transformation (see [COORDINATE_FRAMES_GUIDE.md](./COORDINATE_FRAMES_GUIDE.md))

---

### FRC_WriteUFrameData

**Purpose**: Set transformation data for a specific UFrame

**When to use**:
- Define new work coordinate system
- Update fixture location
- Calibrate frame

**⚠️ WARNING**: Do NOT call while robot is moving!

**Request**:
```json
{
    "Command": "FRC_WriteUFrameData",
    "FrameNumber": 3,
    "Frame": {
        "X": 1000.0,
        "Y": 300.0,
        "Z": 100.0,
        "W": 0.0,
        "P": 0.0,
        "R": 90.0
    },
    "Group": 1
}
```

**Response**:
```json
{
    "Command": "FRC_WriteUFrameData",
    "ErrorID": 0,
    "Group": 1
}
```

**Safety Notes**:
- Can cause unexpected motion if changed during operation
- Verify robot is stopped before calling
- Document frame definitions for your work cell

---

### FRC_ReadUToolData

**Purpose**: Read transformation data for a specific UTool

**When to use**: Get tool TCP offset and orientation

**Request**:
```json
{
    "Command": "FRC_ReadUToolData",
    "ToolNumber": 1,
    "Group": 1
}
```

**Response**:
```json
{
    "Command": "FRC_ReadUToolData",
    "ErrorID": 0,
    "UToolNumber": 1,
    "Frame": {
        "X": 0.0,
        "Y": 0.0,
        "Z": 150.0,
        "W": 0.0,
        "P": 0.0,
        "R": 0.0
    },
    "Group": 1
}
```

**Interpretation**: UTool 1 has TCP 150mm below robot flange (Z=-150)

---

### FRC_WriteUToolData

**Purpose**: Set transformation data for a specific UTool

**When to use**: Define tool geometry and TCP

**⚠️ WARNING**: Do NOT call while robot is moving!

**Request**:
```json
{
    "Command": "FRC_WriteUToolData",
    "ToolNumber": 1,
    "Frame": {
        "X": 0.0,
        "Y": 0.0,
        "Z": 150.0,
        "W": 0.0,
        "P": 0.0,
        "R": 0.0
    },
    "Group": 1
}
```

**Response**:
```json
{
    "Command": "FRC_WriteUToolData",
    "ErrorID": 0,
    "Group": 1
}
```

---

## Motion Instructions

Motion instructions are added to the RMI_MOVE TP program buffer and executed in sequence.

### Key Concepts

- **Sequence IDs**: Must be consecutive and monotonically increasing
- **Buffer**: Robot can hold ~200 instructions
- **Execution**: Instructions execute in order, responses sent when complete
- **CNT vs FINE**: CNT blends motion, FINE stops at point

### FRC_LinearMotion

**Purpose**: Add linear motion instruction (straight line path)

**When to use**: Move TCP in straight line to target position

**Request**:
```json
{
    "Instruction": "FRC_LinearMotion",
    "SequenceID": 1,
    "Configuration": {
        "UToolNumber": 1,
        "UFrameNumber": 0,
        "Front": 1,
        "Up": 1,
        "Left": 0,
        "Flip": 0,
        "Turn4": 0,
        "Turn5": 0,
        "Turn6": 0
    },
    "Position": {
        "X": 500.0,
        "Y": 0.0,
        "Z": 400.0,
        "W": 0.0,
        "P": 0.0,
        "R": 180.0,
        "Ext1": 0.0,
        "Ext2": 0.0,
        "Ext3": 0.0
    },
    "SpeedType": "mm/sec",
    "Speed": 100,
    "TermType": "FINE",
    "TermValue": 0,
    "Group": 1
}
```

**Response** (when instruction completes):
```json
{
    "Instruction": "FRC_LinearMotion",
    "ErrorID": 0,
    "SequenceID": 1
}
```

**Rust API**:
```rust
let request_id = driver.linear_motion(
    500.0, 0.0, 400.0,  // X, Y, Z
    0.0, 0.0, 180.0,    // W, P, R
    100,                // Speed (mm/sec)
    "FINE",             // Term type
    0,                  // Term value
).await?;

// Wait for completion
let response = driver.wait_for_instruction_completion(request_id).await?;
```

**Speed Types**:
- `"mm/sec"`: Linear speed in mm/s
- `"sec"`: Time to complete motion in seconds
- `"cm/min"`: Linear speed in cm/min

**Term Types**:
- `"FINE"`: Stop at point (TermValue = 0)
- `"CNT"`: Blend through point (TermValue = 0-100, corner rounding percentage)

---

### FRC_LinearRelative

**Purpose**: Add linear motion relative to current position

**When to use**: Move incrementally from current location

**Request**:
```json
{
    "Instruction": "FRC_LinearRelative",
    "SequenceID": 2,
    "Position": {
        "X": 10.0,   // Move +10mm in X
        "Y": 0.0,
        "Z": 0.0,
        "W": 0.0,
        "P": 0.0,
        "R": 0.0,
        "Ext1": 0.0,
        "Ext2": 0.0,
        "Ext3": 0.0
    },
    "SpeedType": "mm/sec",
    "Speed": 50,
    "TermType": "FINE",
    "TermValue": 0,
    "Group": 1
}
```

**Rust API**:
```rust
// Move 10mm in +X direction
driver.linear_relative(10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 50, "FINE", 0).await?;
```

---

### FRC_JointMotion

**Purpose**: Add joint motion instruction (fastest path, not straight line)

**When to use**:
- Move to position quickly
- Avoid obstacles
- Return to home position

**Request**:
```json
{
    "Instruction": "FRC_JointMotion",
    "SequenceID": 3,
    "Configuration": {...},
    "Position": {...},
    "SpeedType": "%",
    "Speed": 50,
    "TermType": "FINE",
    "TermValue": 0,
    "Group": 1
}
```

**Speed Types for Joint Motion**:
- `"%"`: Percentage of maximum joint speed (1-100)

---

### FRC_CircularMotion

**Purpose**: Add circular arc motion instruction

**When to use**: Move in circular path (e.g., welding, grinding)

**Request**:
```json
{
    "Instruction": "FRC_CircularMotion",
    "SequenceID": 4,
    "ViaConfiguration": {...},
    "ViaPosition": {...},  // Intermediate point on arc
    "TargetConfiguration": {...},
    "TargetPosition": {...},  // End point of arc
    "SpeedType": "mm/sec",
    "Speed": 100,
    "TermType": "CNT",
    "TermValue": 50,
    "Group": 1
}
```

**Notes**:
- Requires 3 points: current position, via point, target point
- Arc passes through all 3 points

---

## I/O Commands

### FRC_ReadDIN

**Purpose**: Read digital input port value

**Request**:
```json
{
    "Command": "FRC_ReadDIN",
    "PortNumber": 1
}
```

**Response**:
```json
{
    "Command": "FRC_ReadDIN",
    "ErrorID": 0,
    "PortNumber": 1,
    "PortValue": 1
}
```

**Rust API**:
```rust
let din = driver.read_din(1).await?;
println!("DI[1] = {}", din.port_value);
```

---

### FRC_WriteDOUT

**Purpose**: Write digital output port value

**Request**:
```json
{
    "Command": "FRC_WriteDOUT",
    "PortNumber": 1,
    "PortValue": "ON"
}
```

**Response**:
```json
{
    "Command": "FRC_WriteDOUT",
    "ErrorID": 0
}
```

**Rust API**:
```rust
driver.write_dout(1, true).await?;  // Turn on DO[1]
driver.write_dout(1, false).await?; // Turn off DO[1]
```

---

## Advanced Commands

### FRC_SetOverride

**Purpose**: Change program speed override

**When to use**: Slow down or speed up robot motion

**Request**:
```json
{
    "Command": "FRC_SetOverride",
    "OverrideValue": 50
}
```

**Response**:
```json
{
    "Command": "FRC_SetOverride",
    "ErrorID": 0
}
```

**Notes**:
- Value: 0-100 (percentage)
- Affects all motion speeds
- Takes effect immediately

---

### FRC_ReadPositionRegister / FRC_WritePositionRegister

**Purpose**: Read/write position register data

**When to use**:
- Store waypoints
- Share positions between programs
- Dynamic position calculation

**Read Request**:
```json
{
    "Command": "FRC_ReadPositionRegister",
    "RegisterNumber": 1,
    "Group": 1
}
```

**Write Request**:
```json
{
    "Command": "FRC_WritePositionRegister",
    "RegisterNumber": 1,
    "Configuration": {...},
    "Position": {...},
    "Group": 1
}
```

---

## Command Availability Matrix

| Command | Requires Connection | Requires Initialize | Can Use During Motion |
|---------|-------------------|-------------------|---------------------|
| FRC_Connect | No | No | N/A |
| FRC_Disconnect | Yes | No | Yes |
| FRC_Initialize | Yes | No | No |
| FRC_Abort | Yes | Yes | Yes |
| FRC_Continue | Yes | Yes | No |
| FRC_GetStatus | Yes | No | Yes |
| FRC_ReadCartesianPosition | Yes | No | Yes |
| FRC_ReadJointAngles | Yes | No | Yes |
| FRC_SetUFrameUTool | Yes | No | **NO** ⚠️ |
| FRC_ReadUFrameData | Yes | No | Yes |
| FRC_WriteUFrameData | Yes | No | **NO** ⚠️ |
| FRC_ReadUToolData | Yes | No | Yes |
| FRC_WriteUToolData | Yes | No | **NO** ⚠️ |
| FRC_LinearMotion | Yes | Yes | Yes (queued) |
| FRC_JointMotion | Yes | Yes | Yes (queued) |
| FRC_ReadDIN | Yes | No | Yes |
| FRC_WriteDOUT | Yes | No | Yes |
| FRC_SetOverride | Yes | No | Yes |

---

## Summary

### Command Workflow

```
1. FRC_Connect
2. FRC_GetStatus (check robot ready)
3. FRC_Initialize (or startup_sequence())
4. FRC_SetUFrameUTool (optional - set coordinate frame)
5. FRC_LinearMotion / FRC_JointMotion (send instructions)
6. FRC_Abort (when done)
7. FRC_Disconnect
```

### Best Practices

1. **Always check status** before sending commands
2. **Use startup_sequence()** instead of blind abort/initialize
3. **Never change frames during motion**
4. **Monitor sequence IDs** to ensure consecutive ordering
5. **Handle errors gracefully** - check ErrorID in all responses
6. **Disconnect cleanly** when done

---

## Next Steps

- See [FANUC_ROBOTICS_FUNDAMENTALS.md](./FANUC_ROBOTICS_FUNDAMENTALS.md) for concepts
- Check [COORDINATE_FRAMES_GUIDE.md](./COORDINATE_FRAMES_GUIDE.md) for frame transformations
- Review [UI_UX_DESIGN.md](./UI_UX_DESIGN.md) for how commands are exposed in UI

---

**Document Version**: 1.0
**Last Updated**: 2025-11-29
**Author**: FANUC RMI API Development Team



