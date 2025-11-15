use serde::{Deserialize, Serialize};

// Extract module must be declared first so the macro is available to other modules
#[macro_use]
mod extract;
pub use extract::ExtractInner;

pub mod drivers;

pub mod packets;
pub mod instructions;
pub mod commands;
pub mod communication;
pub mod errors;
pub use errors::*;
/// Binary-friendly Data Transfer Objects (DTOs) for application networking.
///
/// The `dto` module contains 1:1 mirrored types without serde renaming/tagging
/// for compact, unambiguous binary serialization (e.g., with `bincode`).
/// Use `fanuc_rmi::protocol` for the JSON/robot protocol types, and
/// `fanuc_rmi::dto` for your app's binary wire. Variant and field order in DTOs
/// affect binary compatibility; prefer additive changes at the end and avoid
/// reordering existing items.
#[cfg(feature = "DTO")]
pub mod dto;
#[cfg(feature = "DTO")]
pub use fanuc_rmi_macros::mirror_dto;


/// JSON protocol types used to communicate with the FANUC controller.
/// These retain serde renaming/tagging to match the controller's wire format.
pub mod protocol {
    pub use super::*;
}


#[cfg_attr(feature = "DTO", mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrameData {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub p: f32,
    pub r: f32,
}

#[cfg_attr(feature = "DTO", mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Configuration {
    #[serde(rename = "F")]
    pub front: u8,
    #[serde(rename = "U")]
    pub up: u8,
    #[serde(rename = "T")]
    pub left: u8,
    #[serde(rename = "B1")]
    pub turn4: u8,
    #[serde(rename = "B2")]
    pub turn5: u8,
    #[serde(rename = "B3")]
    pub turn6: u8,
}

impl Default for Configuration{
    fn default() -> Self {
        Self {
               front: 1,
               up: 1,
               left: 1,
               turn4: 0,
               turn5: 0,
               turn6: 0,
            }
    }
}

#[cfg_attr(feature = "DTO", mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone,Copy, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub p: f32,
    pub r: f32,
    #[serde(default)]
    pub ext1: f32,
    #[serde(default)]
    pub ext2: f32,
    #[serde(default)]
    pub ext3: f32,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
            p: 0.0,
            r: 0.0,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        }
    }
}

#[cfg_attr(feature = "DTO", mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct JointAngles {
    pub j1: f32,
    pub j2: f32,
    pub j3: f32,
    pub j4: f32,
    pub j5: f32,
    pub j6: f32,
    #[serde(default)]
    pub j7: f32,
    #[serde(default)]
    pub j8: f32,
    #[serde(default)]
    pub j9: f32,
}


/// Represents the termination type for robot motion instructions.
///
/// The termination type controls how the robot behaves when reaching the end of a motion instruction,
/// particularly how it transitions between consecutive moves.
///
/// # Variants
///
/// * `FINE` - Robot comes to a complete stop at the target position (precise positioning)
/// * `CNT` - Continuous motion that blends smoothly into the next move (corner rounding)
/// * `CR` - Corner rounding (requires Advanced Constant Path option)
///
/// # TermValue (CNT Smoothness)
///
/// When using `CNT` termination type, the `term_value` field (1-100) controls the corner blending behavior:
///
/// * **CNT100** - Maximum smoothness, largest corner radius, minimal slowdown
///   - Robot maintains high speed through corners
///   - Larger deviation from programmed path at corners
///   - Best for high-speed operations where precision at corners is less critical
///
/// * **CNT50** - Medium blending
///   - Balanced between speed and accuracy
///   - Moderate corner radius
///
/// * **CNT1** - Tight corners, robot slows down significantly
///   - Robot stays very close to programmed path
///   - More deceleration/acceleration at corners
///   - Best when path accuracy is critical
///
/// # Important: CNT Motion Execution Behavior
///
/// **Critical Rule**: A motion instruction with CNT termination type **will not execute** until the
/// next motion instruction arrives. This is because the robot controller needs to know the next move
/// to plan the blending trajectory correctly.
///
/// **Implications**:
/// - Always ensure the last motion instruction uses `FINE` termination type
/// - If the last instruction is CNT, it will never execute (robot will wait indefinitely)
/// - For RMI version 5+: Setting the `NoBlend` flag allows CNT moves to execute without waiting
///
/// # Buffer System
///
/// The FANUC RMI system has specific buffer limits:
/// - **Ring Buffer Size**: 200 instructions maximum
/// - **Concurrent Send Limit**: 8 instructions can be sent at a time
/// - **Execution Dependency**: Instruction N+8 must wait for instruction N to complete before being accepted
///
/// When the 201st instruction is sent, it wraps around to the beginning of the ring buffer.
///
/// # Examples
///
/// ```rust
/// use fanuc_rmi::{FrcLinearRelative, TermType, SpeedType, Position, Configuration};
///
/// // FINE termination - robot stops precisely at target
/// let fine_move = FrcLinearRelative::new(
///     1,
///     Configuration::default(),
///     Position { x: 100.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0, e1: 0.0, e2: 0.0, e3: 0.0 },
///     SpeedType::MMSec,
///     50.0,
///     TermType::FINE,
///     1, // term_value ignored for FINE
/// );
///
/// // CNT termination - smooth blending (requires next move to execute!)
/// let cnt_move = FrcLinearRelative::new(
///     2,
///     Configuration::default(),
///     Position { x: 200.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0, e1: 0.0, e2: 0.0, e3: 0.0 },
///     SpeedType::MMSec,
///     50.0,
///     TermType::CNT,
///     100, // Maximum smoothness
/// );
/// ```
///
/// # See Also
///
/// * FANUC RMI Documentation Section 2.4: "TEACH PENDANT PROGRAM INSTRUCTION PACKETS"
/// * Motion instruction packets: `FrcLinearRelative`, `FrcLinearMotion`, `FrcJointMotion`, etc.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum TermType {
    FINE,
    CNT, // Continuous motion
    CR,  // CR with a value from 1 to 100
}


/// Represents different types of speed measurements.
///
/// This enum provides various units of speed that can be used
/// to specify movement or duration in different contexts.
///
/// # Variants
///
/// * `MMSec` - Represents speed in millimeters per second (mm/sec).
/// * `InchMin` - Represents speed in inches per second.
/// * `Time` - Represents time in 0.1 second increments.
/// * `MilliSeconds` - Represents time in milliseconds (0.001 seconds).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum SpeedType {
    #[serde(rename = "mmSec")]
    MMSec, // Speed in millimeters per second (mm/sec).
    #[serde(rename = "InchMin")]
    InchMin, // Speed in inches per second.
    #[serde(rename = "Time")]
    Time, // Time in 0.1 second increments.
    #[serde(rename = "mSec")]
    MilliSeconds, // Time in milliseconds (0.001 seconds).
}

