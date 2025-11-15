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

