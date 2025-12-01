# FANUC Robotics Fundamentals

**A Comprehensive Guide for Developers New to Industrial Robotics**

---

## Table of Contents

1. [Introduction](#introduction)
2. [Coordinate Systems and Frames](#coordinate-systems-and-frames)
3. [User Frames (UFrames)](#user-frames-uframes)
4. [User Tools (UTools)](#user-tools-utools)
5. [Robot Groups](#robot-groups)
6. [Configuration Parameters](#configuration-parameters)
7. [Practical Examples](#practical-examples)
8. [Common Pitfalls](#common-pitfalls)

---

## Introduction

Industrial robots like FANUC systems operate in a complex 3D space with multiple coordinate systems. Understanding these concepts is crucial for:

- **Accurate positioning**: Knowing where the robot is and where it's going
- **Tool management**: Accounting for different end-effectors (grippers, welders, etc.)
- **Work cell setup**: Defining work areas relative to fixtures and parts
- **Multi-robot coordination**: Managing multiple robots in the same workspace

This guide explains the fundamental concepts used in FANUC robotics systems, specifically in the context of the Remote Motion Interface (RMI) API.

---

## Coordinate Systems and Frames

### What is a Coordinate Frame?

A **coordinate frame** (or coordinate system) is a reference point with three axes (X, Y, Z) that defines:
- **Origin**: The (0, 0, 0) point
- **Orientation**: The direction of the X, Y, and Z axes
- **Units**: Typically millimeters for position, degrees for rotation

### Why Multiple Frames?

In robotics, we use multiple coordinate frames because:

1. **Different perspectives**: The robot's base, the tool tip, and the workpiece all have different "viewpoints"
2. **Flexibility**: You can program relative to a part, not absolute robot coordinates
3. **Reusability**: Move a fixture, update one frame, and all programs still work

### Frame Hierarchy in FANUC Systems

```
World Frame (Base Frame)
    ├── Joint Frame (robot's internal joint angles)
    ├── User Frame 0 (typically same as World Frame)
    ├── User Frame 1
    ├── User Frame 2
    ├── ...
    └── User Frame 9
        └── Tool Center Point (TCP)
            ├── User Tool 0 (robot flange)
            ├── User Tool 1
            ├── ...
            └── User Tool 10
```

---

## User Frames (UFrames)

### What is a User Frame?

A **User Frame** (UFrame) is a custom coordinate system that you define relative to the robot's World Frame. It represents:
- A workpiece location
- A fixture position
- A pallet corner
- Any reference point in the work cell

### Why Use User Frames?

**Example Scenario**: You have a welding robot that welds parts on a fixture.

**Without User Frames**:
```
Move to X=1250.5, Y=340.2, Z=150.0  (absolute world coordinates)
```
❌ If you move the fixture 100mm to the right, you must reprogram EVERY position!

**With User Frames**:
```
Set UFrame 1 = Fixture corner (X=1000, Y=300, Z=100)
Move to X=250.5, Y=40.2, Z=50.0 (relative to UFrame 1)
```
✅ Move the fixture? Just update UFrame 1 definition. All programs still work!

### UFrame Numbering

- **UFrame 0**: Typically the same as World Frame (robot base)
- **UFrame 1-9**: Custom frames you define

### How Positions are Reported

When you read a robot's position using `FRC_ReadCartesianPosition`:
- The position is **always reported in the currently active UFrame**
- The response tells you which UFrame was active
- If UFrame 3 is active, position (10, 20, 30) means "10mm X, 20mm Y, 30mm Z from UFrame 3's origin"

---

## User Tools (UTools)

### What is a User Tool?

A **User Tool** (UTool) defines the geometry of the end-effector attached to the robot. It specifies:
- **Tool Center Point (TCP)**: The functional point of the tool (e.g., welding tip, gripper center)
- **Tool orientation**: How the tool is rotated relative to the robot flange

### Why Use User Tools?

**Example**: A robot with a gripper vs. a welding torch.

**Without User Tools**:
```
Robot flange position: X=500, Y=200, Z=300
Gripper tip is 150mm below flange
You must calculate: X=500, Y=200, Z=150 for every move
```

**With User Tools**:
```
Define UTool 1: Gripper (TCP offset: X=0, Y=0, Z=-150)
Command: Move to X=500, Y=200, Z=300
Robot automatically positions gripper tip at that location!
```

### UTool Numbering

- **UTool 0**: Robot flange (no tool offset)
- **UTool 1-10**: Custom tools you define

---

## Robot Groups

### What is a Group?

A **Group** represents an independently controllable set of robot axes. In FANUC systems:

- **Single robot**: Typically Group 1 (most common)
- **Multi-robot system**: Up to 8 groups (e.g., Group 1 = Robot A, Group 2 = Robot B)
- **Robot + positioner**: Group 1 = robot, Group 2 = turntable/positioner

### Why Groups Matter

**Example**: A welding cell with a robot and a rotating positioner.

```
Group 1: 6-axis robot arm
Group 2: 2-axis positioner (rotation + tilt)
```

You can command both groups to move simultaneously for coordinated motion.

### Group in RMI Commands

Most RMI commands have an optional `Group` parameter:
- **Default**: Group 1 (if not specified)
- **Multi-group systems**: Must specify which group you're controlling

---

## Configuration Parameters

### What is Configuration?

The **Configuration** describes the robot's joint arrangement to reach a Cartesian position. For a 6-axis robot, there can be multiple joint solutions for the same XYZ position.

### Configuration Fields

From `FRC_ReadCartesianPosition` response:

```json
"Configuration": {
    "UToolNumber": 4,      // Active tool
    "UFrameNumber": 3,     // Active frame
    "Front": 1,            // Arm in front (1) or back (0)
    "Up": 1,               // Elbow up (1) or down (0)
    "Left": 0,             // Wrist left (0) or right (1)
    "Flip": 1,             // Wrist flipped (1) or not (0)
    "Turn4": 0,            // J4 turn count
    "Turn5": 0,            // J5 turn count
    "Turn6": 0             // J6 turn count
}
```

### Why Configuration Matters

**Same position, different configurations**:

```
Position: X=500, Y=0, Z=400

Configuration A: Front=1, Up=1, Left=0 (elbow up, arm in front)
Configuration B: Front=1, Up=0, Left=0 (elbow down, arm in front)
```

Both reach the same point, but the robot looks completely different! This affects:
- **Collision avoidance**: One config might hit obstacles
- **Joint limits**: One config might be near limits
- **Singularities**: Some configs are more stable

---

## Practical Examples

### Example 1: Reading Position in Different Frames

**Scenario**: Robot at a fixed location, but we read position in different frames.

```
World Frame (UFrame 0):
    Position: X=1250.5, Y=340.2, Z=150.0

UFrame 3 (offset from World by X=1000, Y=300, Z=100):
    Position: X=250.5, Y=40.2, Z=50.0

Same physical location, different numbers!
```

**In RMI**:
```rust
// Read position (returns position in currently active frame)
let pos = driver.read_cartesian_position().await?;

// Response shows which frame was active
println!("UFrame: {}", pos.config.u_frame_number);  // e.g., 3
println!("Position: X={}, Y={}, Z={}", pos.pos.x, pos.pos.y, pos.pos.z);
// Output: Position: X=250.5, Y=40.2, Z=50.0 (in UFrame 3)
```

### Example 2: Changing Active Frame

**Scenario**: You want to work relative to a different fixture.

```rust
// Check current frame
let status = driver.get_status().await?;
println!("Current UFrame: {}", status.number_uframe);

// Change to UFrame 5 (fixture B)
driver.set_uframe_utool(5, 1).await?;  // UFrame 5, UTool 1

// Now all positions are relative to UFrame 5
let pos = driver.read_cartesian_position().await?;
// Position is now in UFrame 5 coordinates
```

⚠️ **Warning**: Changing the active frame affects motion commands too!

### Example 3: Multi-Tool Setup

**Scenario**: Robot with gripper (UTool 1) and camera (UTool 2).

```rust
// Use gripper
driver.set_uframe_utool(1, 1).await?;  // UFrame 1, UTool 1 (gripper)
driver.linear_motion(x, y, z, w, p, r, ...).await?;  // Gripper tip goes to position

// Switch to camera
driver.set_uframe_utool(1, 2).await?;  // UFrame 1, UTool 2 (camera)
driver.linear_motion(x, y, z, w, p, r, ...).await?;  // Camera goes to same position
// Robot moves differently because camera has different TCP offset!
```

---

## Common Pitfalls

### Pitfall 1: Comparing Positions in Different Frames

❌ **Wrong**:
```rust
// Read position in UFrame 3
let pos1 = driver.read_cartesian_position().await?;  // UFrame 3 active

// Change to UFrame 0
driver.set_uframe_utool(0, 1).await?;

// Read position in UFrame 0
let pos2 = driver.read_cartesian_position().await?;  // UFrame 0 active

// Compare
if pos1.pos.x != pos2.pos.x {
    println!("Robot moved!");  // FALSE! Just different frames!
}
```

✅ **Correct**: Always compare positions in the same frame, or transform coordinates.

### Pitfall 2: Changing Frame During Motion

❌ **Wrong**:
```rust
// Start motion
driver.linear_motion(x1, y1, z1, ...).await?;
driver.linear_motion(x2, y2, z2, ...).await?;

// Change frame while moving
driver.set_uframe_utool(5, 1).await?;  // ERROR! Robot is moving!
```

✅ **Correct**: Only change frames when robot is stopped (`RMIMotionStatus == 0`).

### Pitfall 3: Forgetting Which Frame is Active

❌ **Wrong**:
```rust
// Assume we're in World Frame
driver.linear_motion(1000.0, 500.0, 200.0, ...).await?;
// But UFrame 3 is active! Robot goes to wrong location!
```

✅ **Correct**: Always check active frame before commanding motion:
```rust
let status = driver.get_status().await?;
println!("Active UFrame: {}", status.number_uframe);
```

### Pitfall 4: Teach Pendant vs. RMI Frame Mismatch

❌ **Problem**: Teach Pendant displays position in World Frame, but RMI reads in UFrame 3.

```
TP shows:  X=1250.5, Y=340.2, Z=150.0  (World Frame)
UI shows:  X=250.5, Y=40.2, Z=50.0     (UFrame 3)
```

Users think the robot is in the wrong position!

✅ **Solution**:
1. Display which frame is active in the UI
2. Allow users to change active frame to match TP
3. Or, transform coordinates to World Frame for display

---

## Summary

| Concept | Purpose | Example |
|---------|---------|---------|
| **World Frame** | Robot's base coordinate system | Origin at robot base |
| **User Frame (UFrame)** | Custom work coordinate system | Fixture corner, pallet position |
| **User Tool (UTool)** | Tool geometry and TCP | Gripper tip, welding torch |
| **Group** | Independent robot/axis set | Robot 1, Robot 2, Positioner |
| **Configuration** | Joint arrangement for position | Elbow up/down, wrist flip |

### Key Takeaways

1. **Positions are frame-relative**: X=100 in UFrame 3 ≠ X=100 in World Frame
2. **Active frame affects everything**: Both position reading AND motion commands
3. **Check before changing**: Don't change frames while robot is moving
4. **Display frame info**: Always show users which frame is active
5. **Transform when needed**: Convert between frames for comparison/display

---

## Next Steps

- Read [COORDINATE_FRAMES_GUIDE.md](./COORDINATE_FRAMES_GUIDE.md) for coordinate transformation math
- See [RMI_COMMANDS_REFERENCE.md](./RMI_COMMANDS_REFERENCE.md) for all frame-related commands
- Check [UI_UX_DESIGN.md](./UI_UX_DESIGN.md) for how we display this in the web interface

---

**Document Version**: 1.0
**Last Updated**: 2025-11-29
**Author**: FANUC RMI API Development Team



