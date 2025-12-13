//! Program-related DTOs.

use serde::{Deserialize, Serialize};

/// Optional start position for program execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Program summary info for listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInfo {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub instruction_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Full program detail including instructions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramDetail {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub instructions: Vec<InstructionDto>,
    // Start position (where robot moves before toolpath)
    pub start_x: Option<f64>,
    pub start_y: Option<f64>,
    pub start_z: Option<f64>,
    // End position (where robot moves after toolpath)
    pub end_x: Option<f64>,
    pub end_y: Option<f64>,
    pub end_z: Option<f64>,
    // Speed for moving to start/end positions
    pub move_speed: Option<f64>,
    // Timestamps
    pub created_at: String,
    pub updated_at: String,
}

/// Instruction DTO for client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionDto {
    pub line_number: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: Option<f64>,
    pub p: Option<f64>,
    pub r: Option<f64>,
    pub speed: Option<f64>,
    pub term_type: Option<String>,
    pub uframe: Option<i32>,
    pub utool: Option<i32>,
}

