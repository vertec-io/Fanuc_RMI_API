# Coordinate Frames and Transformations Guide

**Advanced Guide for Coordinate Frame Management in FANUC RMI**

---

## Table of Contents

1. [Introduction](#introduction)
2. [Frame Transformation Basics](#frame-transformation-basics)
3. [Reading Frame Data from FANUC](#reading-frame-data-from-fanuc)
4. [Coordinate Transformation Math](#coordinate-transformation-math)
5. [Implementing Multi-Frame Display](#implementing-multi-frame-display)
6. [Practical Implementation Examples](#practical-implementation-examples)
7. [Testing and Validation](#testing-and-validation)

---

## Introduction

This guide explains how to work with multiple coordinate frames in the FANUC RMI system, including:

- Reading frame transformation data
- Converting positions between frames
- Displaying positions in multiple frames simultaneously
- Validating frame transformations

**Prerequisites**: Read [FANUC_ROBOTICS_FUNDAMENTALS.md](./FANUC_ROBOTICS_FUNDAMENTALS.md) first.

---

## Frame Transformation Basics

### What is a Frame Transformation?

A frame transformation describes how one coordinate frame is positioned and oriented relative to another.

```
World Frame (origin at robot base)
    ↓ Transformation: X=+1000, Y=+300, Z=+100, W=0, P=0, R=90
UFrame 3 (origin at fixture corner, rotated 90° around Z)
```

### Transformation Components

A FANUC frame transformation has 6 values (X, Y, Z, W, P, R):

- **X, Y, Z**: Translation (position offset in mm)
- **W, P, R**: Rotation (orientation in degrees)
  - **W**: Rotation around Z-axis (yaw)
  - **P**: Rotation around Y-axis (pitch)
  - **R**: Rotation around X-axis (roll)

### Rotation Order

FANUC uses **W-P-R** (Z-Y-X) Euler angle convention:
1. First rotate W degrees around Z-axis
2. Then rotate P degrees around Y-axis
3. Finally rotate R degrees around X-axis

---

## Reading Frame Data from FANUC

### FRC_ReadUFrameData Command

This command reads the transformation data for a specific UFrame.

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

This tells us: UFrame 3 is located at (1000, 300, 100) mm from World Frame, rotated 90° around X-axis.

### FRC_ReadUToolData Command

Similar to UFrame, but for tool transformations.

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

This tells us: UTool 1 (gripper) has TCP 150mm below the robot flange.

---

## Coordinate Transformation Math

### Simple Case: Translation Only (No Rotation)

If a frame has no rotation (W=0, P=0, R=0), transformation is simple addition:

```
UFrame 3: X=1000, Y=300, Z=100, W=0, P=0, R=0

Position in UFrame 3: (250, 40, 50)
Position in World Frame: (250+1000, 40+300, 50+100) = (1250, 340, 150)
```

**Formula**:
```
World_X = UFrame_X + Position_X
World_Y = UFrame_Y + Position_Y
World_Z = UFrame_Z + Position_Z
```

### Complex Case: Translation + Rotation

When rotation is involved, we need rotation matrices.

**Rotation Matrix for W (Z-axis)**:
```
Rz(W) = | cos(W)  -sin(W)   0 |
        | sin(W)   cos(W)   0 |
        |   0        0      1 |
```

**Rotation Matrix for P (Y-axis)**:
```
Ry(P) = | cos(P)   0   sin(P) |
        |   0      1     0     |
        |-sin(P)   0   cos(P) |
```

**Rotation Matrix for R (X-axis)**:
```
Rx(R) = | 1    0        0     |
        | 0  cos(R)  -sin(R)  |
        | 0  sin(R)   cos(R)  |
```

**Combined Rotation Matrix** (W-P-R order):
```
R_total = Rz(W) * Ry(P) * Rx(R)
```

**Full Transformation**:
```
World_Position = UFrame_Translation + (R_total * Local_Position)
```

### Rust Implementation Example

```rust
use nalgebra::{Matrix3, Vector3};

/// Convert degrees to radians
fn deg_to_rad(deg: f64) -> f64 {
    deg * std::f64::consts::PI / 180.0
}

/// Create rotation matrix for Z-axis (W)
fn rotation_z(w_deg: f64) -> Matrix3<f64> {
    let w = deg_to_rad(w_deg);
    Matrix3::new(
        w.cos(), -w.sin(), 0.0,
        w.sin(),  w.cos(), 0.0,
        0.0,      0.0,     1.0,
    )
}

/// Create rotation matrix for Y-axis (P)
fn rotation_y(p_deg: f64) -> Matrix3<f64> {
    let p = deg_to_rad(p_deg);
    Matrix3::new(
         p.cos(), 0.0, p.sin(),
         0.0,     1.0, 0.0,
        -p.sin(), 0.0, p.cos(),
    )
}

/// Create rotation matrix for X-axis (R)
fn rotation_x(r_deg: f64) -> Matrix3<f64> {
    let r = deg_to_rad(r_deg);
    Matrix3::new(
        1.0, 0.0,      0.0,
        0.0, r.cos(), -r.sin(),
        0.0, r.sin(),  r.cos(),
    )
}

/// Transform position from UFrame to World Frame
pub fn uframe_to_world(
    uframe_translation: Vector3<f64>,  // UFrame's X, Y, Z
    uframe_rotation: (f64, f64, f64),  // UFrame's W, P, R
    local_position: Vector3<f64>,      // Position in UFrame
) -> Vector3<f64> {
    let (w, p, r) = uframe_rotation;

    // Build combined rotation matrix (W-P-R order)
    let r_total = rotation_z(w) * rotation_y(p) * rotation_x(r);

    // Apply transformation
    uframe_translation + r_total * local_position
}

/// Transform position from World Frame to UFrame
pub fn world_to_uframe(
    uframe_translation: Vector3<f64>,
    uframe_rotation: (f64, f64, f64),
    world_position: Vector3<f64>,
) -> Vector3<f64> {
    let (w, p, r) = uframe_rotation;

    // Build combined rotation matrix
    let r_total = rotation_z(w) * rotation_y(p) * rotation_x(r);

    // Inverse transformation
    let r_inverse = r_total.transpose();  // For rotation matrices, transpose = inverse
    r_inverse * (world_position - uframe_translation)
}
```

---

## Implementing Multi-Frame Display

### Strategy

To display positions in multiple frames simultaneously:

1. **Read all UFrame transformations** at startup or when requested
2. **Cache the transformation data** (frames don't change often)
3. **Read current position** in active frame
4. **Transform to other frames** using cached data
5. **Display all frames** in UI

### Data Structure

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct FrameTransform {
    pub translation: Vector3<f64>,  // X, Y, Z
    pub rotation: (f64, f64, f64),  // W, P, R
}

pub struct FrameManager {
    /// Cached UFrame transformations (frame_number -> transform)
    uframes: HashMap<u8, FrameTransform>,

    /// Currently active UFrame number
    active_uframe: u8,
}

impl FrameManager {
    pub fn new() -> Self {
        Self {
            uframes: HashMap::new(),
            active_uframe: 0,
        }
    }

    /// Load all UFrame transformations from robot
    pub async fn load_all_uframes(&mut self, driver: &FanucDriver, max_frames: u8) -> Result<(), String> {
        for frame_num in 0..=max_frames {
            match driver.read_uframe_data(frame_num).await {
                Ok(response) => {
                    if response.error_id == 0 {
                        let transform = FrameTransform {
                            translation: Vector3::new(
                                response.frame.x,
                                response.frame.y,
                                response.frame.z,
                            ),
                            rotation: (
                                response.frame.w,
                                response.frame.p,
                                response.frame.r,
                            ),
                        };
                        self.uframes.insert(frame_num, transform);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to read UFrame {}: {}", frame_num, e);
                }
            }
        }
        Ok(())
    }

    /// Convert position from one frame to another
    pub fn convert_position(
        &self,
        position: Vector3<f64>,
        from_frame: u8,
        to_frame: u8,
    ) -> Option<Vector3<f64>> {
        if from_frame == to_frame {
            return Some(position);
        }

        // Get frame transformations
        let from_transform = self.uframes.get(&from_frame)?;
        let to_transform = self.uframes.get(&to_frame)?;

        // Convert from source frame to world frame
        let world_pos = uframe_to_world(
            from_transform.translation,
            from_transform.rotation,
            position,
        );

        // Convert from world frame to target frame
        Some(world_to_uframe(
            to_transform.translation,
            to_transform.rotation,
            world_pos,
        ))
    }

    /// Get position in all available frames
    pub fn get_all_frame_positions(
        &self,
        position: Vector3<f64>,
        current_frame: u8,
    ) -> HashMap<u8, Vector3<f64>> {
        let mut result = HashMap::new();

        for &frame_num in self.uframes.keys() {
            if let Some(converted) = self.convert_position(position, current_frame, frame_num) {
                result.insert(frame_num, converted);
            }
        }

        result
    }
}
```

---

## Practical Implementation Examples

### Example 1: Display Position in World Frame

```rust
// Read current position (in active UFrame)
let pos_response = driver.read_cartesian_position().await?;
let active_frame = pos_response.config.u_frame_number;
let position = Vector3::new(pos_response.pos.x, pos_response.pos.y, pos_response.pos.z);

// Convert to World Frame (UFrame 0)
let world_position = frame_manager.convert_position(position, active_frame, 0)
    .ok_or("Failed to convert to world frame")?;

println!("Position in UFrame {}: X={:.3}, Y={:.3}, Z={:.3}",
    active_frame, position.x, position.y, position.z);
println!("Position in World Frame: X={:.3}, Y={:.3}, Z={:.3}",
    world_position.x, world_position.y, world_position.z);
```

### Example 2: Display in All Frames

```rust
// Read current position
let pos_response = driver.read_cartesian_position().await?;
let active_frame = pos_response.config.u_frame_number;
let position = Vector3::new(pos_response.pos.x, pos_response.pos.y, pos_response.pos.z);

// Get positions in all frames
let all_positions = frame_manager.get_all_frame_positions(position, active_frame);

// Display
println!("Current position in all frames:");
for frame_num in 0..=9 {
    if let Some(pos) = all_positions.get(&frame_num) {
        let marker = if frame_num == active_frame { " (ACTIVE)" } else { "" };
        println!("  UFrame {}{}: X={:.3}, Y={:.3}, Z={:.3}",
            frame_num, marker, pos.x, pos.y, pos.z);
    }
}
```

### Example 3: UI Integration

```rust
// In web_server, send multi-frame data to UI
#[derive(Serialize)]
struct MultiFramePosition {
    active_frame: u8,
    positions: HashMap<u8, PositionData>,
}

#[derive(Serialize)]
struct PositionData {
    x: f64,
    y: f64,
    z: f64,
    w: f64,
    p: f64,
    r: f64,
}

// Build response
let pos_response = driver.read_cartesian_position().await?;
let active_frame = pos_response.config.u_frame_number;
let position = Vector3::new(pos_response.pos.x, pos_response.pos.y, pos_response.pos.z);

let all_positions = frame_manager.get_all_frame_positions(position, active_frame);

let mut positions_map = HashMap::new();
for (frame_num, pos) in all_positions {
    positions_map.insert(frame_num, PositionData {
        x: pos.x,
        y: pos.y,
        z: pos.z,
        w: pos_response.pos.w,  // Orientation stays same
        p: pos_response.pos.p,
        r: pos_response.pos.r,
    });
}

let multi_frame_data = MultiFramePosition {
    active_frame,
    positions: positions_map,
};

// Send to UI via WebSocket
send_to_ui(serde_json::to_string(&multi_frame_data)?);
```

---

## Testing and Validation

### Test 1: Identity Transformation

UFrame 0 should be identity (no transformation):

```rust
#[test]
fn test_uframe_0_identity() {
    let position = Vector3::new(100.0, 200.0, 300.0);
    let uframe_0 = FrameTransform {
        translation: Vector3::new(0.0, 0.0, 0.0),
        rotation: (0.0, 0.0, 0.0),
    };

    let world_pos = uframe_to_world(
        uframe_0.translation,
        uframe_0.rotation,
        position,
    );

    assert_eq!(world_pos, position);
}
```

### Test 2: Translation Only

```rust
#[test]
fn test_translation_only() {
    let local_pos = Vector3::new(250.0, 40.0, 50.0);
    let uframe_translation = Vector3::new(1000.0, 300.0, 100.0);
    let uframe_rotation = (0.0, 0.0, 0.0);

    let world_pos = uframe_to_world(uframe_translation, uframe_rotation, local_pos);

    assert!((world_pos.x - 1250.0).abs() < 0.001);
    assert!((world_pos.y - 340.0).abs() < 0.001);
    assert!((world_pos.z - 150.0).abs() < 0.001);
}
```

### Test 3: Round-Trip Conversion

```rust
#[test]
fn test_round_trip() {
    let original = Vector3::new(123.456, 789.012, 345.678);
    let uframe_translation = Vector3::new(1000.0, 500.0, 200.0);
    let uframe_rotation = (45.0, 30.0, 60.0);

    // Convert to world and back
    let world_pos = uframe_to_world(uframe_translation, uframe_rotation, original);
    let back_to_local = world_to_uframe(uframe_translation, uframe_rotation, world_pos);

    assert!((back_to_local.x - original.x).abs() < 0.001);
    assert!((back_to_local.y - original.y).abs() < 0.001);
    assert!((back_to_local.z - original.z).abs() < 0.001);
}
```

### Test 4: Validate Against Real Robot

```rust
// Manual test procedure:
// 1. Jog robot to known position in World Frame (e.g., X=500, Y=0, Z=400)
// 2. Read position in World Frame (UFrame 0)
// 3. Change to UFrame 3
// 4. Read position in UFrame 3
// 5. Use our transformation to convert UFrame 3 position to World
// 6. Compare with step 2 - should match within 0.1mm
```

---

## Summary

### Key Points

1. **Frame transformations** consist of translation (X,Y,Z) and rotation (W,P,R)
2. **Rotation order matters**: FANUC uses W-P-R (Z-Y-X) Euler angles
3. **Use rotation matrices** for accurate transformation with rotation
4. **Cache frame data**: UFrames don't change often, read once and reuse
5. **Validate thoroughly**: Test with real robot to ensure accuracy

### Implementation Checklist

- [ ] Add `nalgebra` dependency for matrix math
- [ ] Implement `FRC_ReadUFrameData` command in protocol
- [ ] Create `FrameManager` struct for caching transformations
- [ ] Implement transformation functions (`uframe_to_world`, `world_to_uframe`)
- [ ] Add multi-frame display to web_server
- [ ] Update UI to show positions in multiple frames
- [ ] Write unit tests for transformations
- [ ] Validate against real robot

---

## Next Steps

- See [RMI_COMMANDS_REFERENCE.md](./RMI_COMMANDS_REFERENCE.md) for frame-related command details
- Check [UI_UX_DESIGN.md](./UI_UX_DESIGN.md) for multi-frame display mockups
- Review [ARCHITECTURE_DESIGN.md](./ARCHITECTURE_DESIGN.md) for integration strategy

---

**Document Version**: 1.0
**Last Updated**: 2025-11-29
**Author**: FANUC RMI API Development Team



