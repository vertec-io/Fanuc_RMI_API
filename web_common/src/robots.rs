//! Robot connection and configuration DTOs.

use serde::{Deserialize, Serialize};

/// Robot connection DTO (for saved connections).
/// Motion defaults (speed, term_type, w/p/r) and jog defaults are stored here.
/// Frame/tool/arm configuration is stored in robot_configurations table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RobotConnectionDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub ip_address: String,
    pub port: u32,
    // Motion defaults (required - no global fallback)
    pub default_speed: f64,
    pub default_speed_type: String,  // mmSec, InchMin, Time, mSec
    pub default_term_type: String,
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    // Jog defaults
    pub default_cartesian_jog_speed: f64,
    pub default_cartesian_jog_step: f64,
    pub default_joint_jog_speed: f64,
    pub default_joint_jog_step: f64,
    pub default_rotation_jog_speed: f64,
    pub default_rotation_jog_step: f64,
}

/// Robot configuration DTO (named configurations per robot).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotConfigurationDto {
    pub id: i64,
    pub robot_connection_id: i64,
    pub name: String,
    pub is_default: bool,
    pub u_frame_number: i32,
    pub u_tool_number: i32,
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
}

/// New robot configuration DTO (for creating configurations without ID).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewRobotConfigurationDto {
    pub name: String,
    pub is_default: bool,
    pub u_frame_number: i32,
    pub u_tool_number: i32,
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
}

