//! Settings and configuration DTOs.

use serde::{Deserialize, Serialize};

/// Robot settings DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotSettingsDto {
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    pub default_speed: f64,
    pub default_term_type: String,
    pub default_uframe: i32,
    pub default_utool: i32,
}

/// A single change entry in the changelog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeLogEntryDto {
    pub field_name: String,
    pub old_value: String,
    pub new_value: String,
}

/// I/O display configuration DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoDisplayConfigDto {
    pub io_type: String,
    pub io_index: i32,
    pub display_name: Option<String>,
    pub is_visible: bool,
    pub display_order: Option<i32>,
}

