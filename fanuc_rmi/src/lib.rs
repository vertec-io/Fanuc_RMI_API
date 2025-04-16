use serde::{Deserialize, Serialize};


pub mod drivers;

pub mod packets;
pub mod instructions;
pub mod commands;
pub mod communication;
pub mod errors;
pub use errors::*;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrameData {
    x: f32,
    y: f32,
    z: f32,
    w: f32,
    p: f32,
    r: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct Configuration {
    pub u_tool_number: u8,
    pub u_frame_number: u8,
    pub front: u8,
    pub up: u8,
    pub left: u8,
    pub flip: u8,
    pub turn4: u8,
    pub turn5: u8,
    pub turn6: u8,
}

impl Default for Configuration{
    fn default() -> Self {
        Self { u_tool_number: 1,
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

#[derive(Serialize, Deserialize, Debug, Clone,Copy, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
    pub p: f32,
    pub r: f32,
    pub ext1: f32,
    pub ext2: f32,
    pub ext3: f32,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: -100.0,
            w: 0.0,
            p: 90.0,
            r: 0.0,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct JointAngles {
    j1: f32,
    j2: f32,
    j3: f32,
    j4: f32,
    j5: f32,
    j6: f32,
    j7: f32,
    j8: f32,
    j9: f32,
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
    InchMin, // Speed in inches per second.
    Time, // Time in 0.1 second increments.
    #[serde(rename = "mSec")]
    MilliSeconds, // Time in milliseconds (0.001 seconds).
}
