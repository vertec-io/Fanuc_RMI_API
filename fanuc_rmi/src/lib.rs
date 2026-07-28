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

/// Coordinate transformation utilities (nalgebra integration).
///
/// Enable with the `nalgebra-support` feature flag.
#[cfg(feature = "nalgebra-support")]
pub mod transforms;

// Re-export nalgebra when the feature is enabled
#[cfg(feature = "nalgebra-support")]
pub use nalgebra;
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

/// A FANUC motion-group selection bitmask.
///
/// FANUC controllers can be configured with multiple motion groups. Group 1 is
/// the robot arm; any additional coordinated axes — e.g. a **positioner**
/// (turntable / tilt-rotate table), a track/rail, or a second robot — are
/// configured as further motion groups (Group 2, Group 3, …). The RMI `GroupMask` field
/// (used by `FRC_Initialize`) is a bitmask where **bit N-1 selects group N**:
///
/// | Group | Bit  | Value |
/// |-------|------|-------|
/// | 1     | 0    | 0x01  |
/// | 2     | 1    | 0x02  |
/// | 3     | 2    | 0x04  |
/// | …     | …    | …     |
///
/// So Group 1 + Group 2 together is `0x03`.
///
/// # ⚠️ RMI limitation — read before using multi-group masks
///
/// Initializing with a multi-group mask (e.g. `GROUP_1 | GROUP_2`) tells the
/// controller which groups the RMI session *reserves*. It does **not** make the
/// RMI motion packets (`FRC_LinearMotion`, `FRC_JointMotion`, …) drive Group 2:
/// the RMI motion protocol is documented single-group and its motion packets
/// carry **no group field**, so coordinated motion is not reachable through them.
/// True coordinated / positioner motion must be driven by a controller-resident
/// TP/COORD program (see [`crate::instructions::FrcCall`]) parameterised through
/// registers / group I/O. Reading Group-2 joint angles for visualization *is*
/// supported (the read commands carry a group field). Use multi-group masks with
/// that architecture in mind.
///
/// `GroupMask` is `#[serde(transparent)]` over a `u8`, so it is wire-compatible
/// with the raw byte the controller expects and is reused as-is in the binary
/// DTO layer (no separate `GroupMaskDto`).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct GroupMask(u8);

impl GroupMask {
    /// Group 1 (the robot arm) — the RMI default.
    pub const GROUP_1: GroupMask = GroupMask(0b0000_0001);
    /// Group 2 — a second coordinated motion group (e.g. a positioner, track, or robot).
    pub const GROUP_2: GroupMask = GroupMask(0b0000_0010);
    /// Group 3.
    pub const GROUP_3: GroupMask = GroupMask(0b0000_0100);
    /// Group 4.
    pub const GROUP_4: GroupMask = GroupMask(0b0000_1000);
    /// Group 5.
    pub const GROUP_5: GroupMask = GroupMask(0b0001_0000);
    /// Group 6.
    pub const GROUP_6: GroupMask = GroupMask(0b0010_0000);
    /// Group 7.
    pub const GROUP_7: GroupMask = GroupMask(0b0100_0000);
    /// Group 8.
    pub const GROUP_8: GroupMask = GroupMask(0b1000_0000);

    /// An empty mask (no groups selected).
    pub const fn empty() -> Self {
        GroupMask(0)
    }

    /// Construct directly from the raw bitmask value.
    pub const fn from_bits(bits: u8) -> Self {
        GroupMask(bits)
    }

    /// The raw bitmask value, as sent on the wire.
    pub const fn bits(self) -> u8 {
        self.0
    }

    /// Build a mask selecting a single **1-based** group number (1..=8).
    ///
    /// Returns [`GroupMask::empty`] for `0` or values `> 8` (out of range).
    pub const fn from_group(group: u8) -> Self {
        if group == 0 || group > 8 {
            GroupMask::empty()
        } else {
            GroupMask(1u8 << (group - 1))
        }
    }

    /// Union of two masks (adds the groups selected by `other`).
    pub const fn with(self, other: GroupMask) -> Self {
        GroupMask(self.0 | other.0)
    }

    /// True if every group in `other` is also selected by `self`.
    pub const fn contains(self, other: GroupMask) -> bool {
        (self.0 & other.0) == other.0
    }

    /// True if a **1-based** group number is selected by this mask.
    pub const fn contains_group(self, group: u8) -> bool {
        self.contains(GroupMask::from_group(group))
    }

    /// True if more than one group bit is set (a coordinated/multi-group mask).
    pub const fn is_multi_group(self) -> bool {
        self.0.count_ones() > 1
    }

    /// Number of groups selected.
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// The **1-based** group numbers selected by this mask, ascending.
    pub fn groups(self) -> impl Iterator<Item = u8> {
        (1u8..=8).filter(move |g| self.contains_group(*g))
    }
}

impl Default for GroupMask {
    /// The RMI default: Group 1 only.
    fn default() -> Self {
        GroupMask::GROUP_1
    }
}

impl core::fmt::Display for GroupMask {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "GroupMask(0x{:02x})", self.0)
    }
}

impl From<u8> for GroupMask {
    fn from(bits: u8) -> Self {
        GroupMask(bits)
    }
}

impl From<GroupMask> for u8 {
    fn from(mask: GroupMask) -> Self {
        mask.0
    }
}

impl core::ops::BitOr for GroupMask {
    type Output = GroupMask;
    fn bitor(self, rhs: GroupMask) -> GroupMask {
        GroupMask(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for GroupMask {
    fn bitor_assign(&mut self, rhs: GroupMask) {
        self.0 |= rhs.0;
    }
}

impl core::ops::BitAnd for GroupMask {
    type Output = GroupMask;
    fn bitand(self, rhs: GroupMask) -> GroupMask {
        GroupMask(self.0 & rhs.0)
    }
}


#[cfg_attr(feature = "DTO", mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FrameData {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
}

/// Robot configuration data structure
///
/// Corresponds to the "Configuration" object in FANUC RMI JSON packets.
/// Reference: FANUC B-84184EN_02 specification
///
/// # JSON Format
/// ```json
/// "Configuration": {
///     "UToolNumber": byteValue1,
///     "UFrameNumber": byteValue2,
///     "Front": byteValue3,
///     "Up": byteValue4,
///     "Left": byteValue5,
///     "Flip": byteValue6,
///     "Turn4": byteValue7,
///     "Turn5": byteValue8,
///     "Turn6": byteValue9
/// }
/// ```
///
/// # Note on signed types
/// The configuration fields use `i8` instead of `u8` because real FANUC robots
/// may return negative values in error states (e.g., ErrorID 2556941 returns
/// values like Left: -98, Flip: -32). Using signed types allows deserialization
/// to succeed even when the robot is in an error state.
#[cfg_attr(feature = "DTO", mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Configuration {
    /// User Tool Number (1-based index)
    /// Corresponds to "UToolNumber" in FANUC JSON
    /// Note: Can be negative in error states
    pub u_tool_number: i8,

    /// User Frame Number (1-based index)
    /// Corresponds to "UFrameNumber" in FANUC JSON
    /// Note: Can be negative in error states
    pub u_frame_number: i8,

    /// Front configuration bit
    /// Corresponds to "Front" in FANUC JSON
    pub front: i8,

    /// Up configuration bit
    /// Corresponds to "Up" in FANUC JSON
    pub up: i8,

    /// Left configuration bit
    /// Corresponds to "Left" in FANUC JSON
    pub left: i8,

    /// Flip configuration bit
    /// Corresponds to "Flip" in FANUC JSON
    pub flip: i8,

    /// Turn4 configuration bit (wrist axis 1)
    /// Corresponds to "Turn4" in FANUC JSON
    pub turn4: i8,

    /// Turn5 configuration bit (wrist axis 2)
    /// Corresponds to "Turn5" in FANUC JSON
    pub turn5: i8,

    /// Turn6 configuration bit (wrist axis 3)
    /// Corresponds to "Turn6" in FANUC JSON
    pub turn6: i8,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            u_tool_number: 1,
            u_frame_number: 1,
            front: 1,
            up: 1,
            left: 1,
            flip: 0,
            turn4: 0,
            turn5: 0,
            turn6: 0,
        }
    }
}

/// Represents a Cartesian position with orientation.
///
/// # Fields
/// - `x`, `y`, `z`: Required translation coordinates (mm)
/// - `w`, `p`, `r`: Optional rotation angles (degrees, W-P-R Euler convention)
/// - `ext1`, `ext2`, `ext3`: Optional external axis positions
///
/// # Serialization
/// All fields except `x`, `y`, `z` have `#[serde(default)]` and will default to 0.0
/// if not present during deserialization. This allows minimal CSV/JSON input.
#[cfg_attr(feature = "DTO", mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[serde(default)]
    pub w: f64,
    #[serde(default)]
    pub p: f64,
    #[serde(default)]
    pub r: f64,
    #[serde(default)]
    pub ext1: f64,
    #[serde(default)]
    pub ext2: f64,
    #[serde(default)]
    pub ext3: f64,
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
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
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
/// use fanuc_rmi::{instructions::FrcLinearRelative, TermType, SpeedType, Position, Configuration};
///
/// // FINE termination - robot stops precisely at target
/// let fine_move = FrcLinearRelative::new(
///     1,
///     Configuration::default(),
///     Position { x: 100.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0 },
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
///     Position { x: 200.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0 },
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

