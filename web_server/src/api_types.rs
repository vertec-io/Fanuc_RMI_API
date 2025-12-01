//! WebSocket API types for program management and robot control.
//!
//! These types are used for client-server communication over WebSocket.
//! They are separate from the fanuc_rmi DTO types which handle robot protocol.

use serde::{Deserialize, Serialize};

/// Client requests to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
    // Program Management
    #[serde(rename = "list_programs")]
    ListPrograms,

    #[serde(rename = "get_program")]
    GetProgram { id: i64 },

    #[serde(rename = "create_program")]
    CreateProgram { name: String, description: Option<String> },

    #[serde(rename = "delete_program")]
    DeleteProgram { id: i64 },

    #[serde(rename = "upload_csv")]
    UploadCsv {
        program_id: i64,
        csv_content: String,
        start_position: Option<StartPosition>,
    },

    // Program Execution
    #[serde(rename = "start_program")]
    StartProgram { program_id: i64 },

    #[serde(rename = "pause_program")]
    PauseProgram,

    #[serde(rename = "resume_program")]
    ResumeProgram,

    #[serde(rename = "stop_program")]
    StopProgram,

    // Robot Settings
    #[serde(rename = "get_settings")]
    GetSettings,

    #[serde(rename = "update_settings")]
    UpdateSettings {
        default_w: f64,
        default_p: f64,
        default_r: f64,
        default_speed: f64,
        default_term_type: String,
        default_uframe: i32,
        default_utool: i32,
    },

    #[serde(rename = "reset_database")]
    ResetDatabase,

    // Connection Management
    #[serde(rename = "get_connection_status")]
    GetConnectionStatus,

    #[serde(rename = "connect_robot")]
    ConnectRobot { robot_addr: String, robot_port: u32 },

    #[serde(rename = "disconnect_robot")]
    DisconnectRobot,

    // Robot Connections (Saved Connections)
    #[serde(rename = "list_robot_connections")]
    ListRobotConnections,

    #[serde(rename = "get_robot_connection")]
    GetRobotConnection { id: i64 },

    #[serde(rename = "create_robot_connection")]
    CreateRobotConnection {
        name: String,
        description: Option<String>,
        ip_address: String,
        port: u32,
    },

    #[serde(rename = "update_robot_connection")]
    UpdateRobotConnection {
        id: i64,
        name: String,
        description: Option<String>,
        ip_address: String,
        port: u32,
    },

    #[serde(rename = "update_robot_connection_defaults")]
    UpdateRobotConnectionDefaults {
        id: i64,
        default_speed: Option<f64>,
        default_term_type: Option<String>,
        default_uframe: Option<i32>,
        default_utool: Option<i32>,
        default_w: Option<f64>,
        default_p: Option<f64>,
        default_r: Option<f64>,
    },

    #[serde(rename = "delete_robot_connection")]
    DeleteRobotConnection { id: i64 },

    // Frame/Tool Management
    #[serde(rename = "get_active_frame_tool")]
    GetActiveFrameTool,

    #[serde(rename = "set_active_frame_tool")]
    SetActiveFrameTool { uframe: u8, utool: u8 },

    #[serde(rename = "read_frame_data")]
    ReadFrameData { frame_number: u8 },

    #[serde(rename = "read_tool_data")]
    ReadToolData { tool_number: u8 },

    #[serde(rename = "write_frame_data")]
    WriteFrameData {
        frame_number: u8,
        x: f64,
        y: f64,
        z: f64,
        w: f64,
        p: f64,
        r: f64,
    },

    #[serde(rename = "write_tool_data")]
    WriteToolData {
        tool_number: u8,
        x: f64,
        y: f64,
        z: f64,
        w: f64,
        p: f64,
        r: f64,
    },

    // I/O Management
    #[serde(rename = "read_din")]
    ReadDin { port_number: u16 },

    #[serde(rename = "write_dout")]
    WriteDout { port_number: u16, port_value: bool },

    #[serde(rename = "read_din_batch")]
    ReadDinBatch { port_numbers: Vec<u16> },
}

/// Optional start position for program execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// Server responses to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerResponse {
    #[serde(rename = "success")]
    Success { message: String },

    #[serde(rename = "error")]
    Error { message: String },

    #[serde(rename = "programs")]
    Programs { programs: Vec<ProgramInfo> },

    #[serde(rename = "program")]
    Program { program: ProgramDetail },

    #[serde(rename = "settings")]
    Settings { settings: RobotSettingsDto },

    #[serde(rename = "execution_status")]
    ExecutionStatus {
        status: String, // "idle", "running", "paused", "error"
        current_line: Option<usize>,
        total_lines: Option<usize>,
        error: Option<String>,
    },

    #[serde(rename = "execution_started")]
    ExecutionStarted {
        program_id: i64,
        total_lines: usize,
    },

    #[serde(rename = "program_complete")]
    ProgramComplete {
        program_id: i64,
        success: bool,
        message: Option<String>,
    },

    #[serde(rename = "instruction_progress")]
    InstructionProgress {
        current_line: usize,
        total_lines: usize,
    },

    /// Sent when an instruction is being sent to the robot (started executing)
    #[serde(rename = "instruction_sent")]
    InstructionSent {
        current_line: usize,
        total_lines: usize,
    },

    #[serde(rename = "connection_status")]
    ConnectionStatus {
        connected: bool,
        robot_addr: String,
        robot_port: u32,
    },

    #[serde(rename = "robot_connections")]
    RobotConnections { connections: Vec<RobotConnectionDto> },

    #[serde(rename = "robot_connection")]
    RobotConnection { connection: RobotConnectionDto },

    // Frame/Tool responses
    #[serde(rename = "active_frame_tool")]
    ActiveFrameTool { uframe: u8, utool: u8 },

    #[serde(rename = "frame_data")]
    FrameData {
        frame_number: u8,
        x: f64,
        y: f64,
        z: f64,
        w: f64,
        p: f64,
        r: f64,
    },

    #[serde(rename = "tool_data")]
    ToolData {
        tool_number: u8,
        x: f64,
        y: f64,
        z: f64,
        w: f64,
        p: f64,
        r: f64,
    },

    // I/O responses
    #[serde(rename = "din_value")]
    DinValue { port_number: u16, port_value: bool },

    #[serde(rename = "din_batch")]
    DinBatch { values: Vec<(u16, bool)> },
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
    pub start_x: Option<f64>,
    pub start_y: Option<f64>,
    pub start_z: Option<f64>,
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
}

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

/// Robot connection DTO (for saved connections).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotConnectionDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub ip_address: String,
    pub port: u32,
    // Per-robot defaults (optional, falls back to global settings if None)
    pub default_speed: Option<f64>,
    pub default_term_type: Option<String>,
    pub default_uframe: Option<i32>,
    pub default_utool: Option<i32>,
    pub default_w: Option<f64>,
    pub default_p: Option<f64>,
    pub default_r: Option<f64>,
}
