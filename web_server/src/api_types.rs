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

    /// Update program settings (start/end positions, move speed).
    #[serde(rename = "update_program_settings")]
    UpdateProgramSettings {
        program_id: i64,
        start_x: Option<f64>,
        start_y: Option<f64>,
        start_z: Option<f64>,
        end_x: Option<f64>,
        end_y: Option<f64>,
        end_z: Option<f64>,
        move_speed: Option<f64>,
    },

    /// Upload CSV content to a program.
    /// CSV contains generic waypoints (X, Y, Z, speed, etc.) without robot-specific configuration.
    /// Robot configuration is applied at execution time, not at upload time.
    #[serde(rename = "upload_csv")]
    UploadCsv {
        program_id: i64,
        csv_content: String,
        start_position: Option<StartPosition>,
    },

    // Program Execution
    /// Load a program into the executor (does not start execution)
    #[serde(rename = "load_program")]
    LoadProgram { program_id: i64 },

    /// Unload the current program from the executor
    #[serde(rename = "unload_program")]
    UnloadProgram,

    /// Start/resume execution of the loaded program
    #[serde(rename = "start_program")]
    StartProgram { program_id: i64 },

    #[serde(rename = "pause_program")]
    PauseProgram,

    #[serde(rename = "resume_program")]
    ResumeProgram,

    #[serde(rename = "stop_program")]
    StopProgram,

    /// Get current execution state (for client reconnection/sync)
    #[serde(rename = "get_execution_state")]
    GetExecutionState,

    // Robot Control Commands
    /// Abort current motion and clear motion queue
    #[serde(rename = "robot_abort")]
    RobotAbort,

    /// Reset robot controller (clears errors)
    #[serde(rename = "robot_reset")]
    RobotReset,

    /// Initialize robot controller
    #[serde(rename = "robot_initialize")]
    RobotInitialize { group_mask: Option<u8> },

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

    #[serde(rename = "connect_to_saved_robot")]
    ConnectToSavedRobot { connection_id: i64 },

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

    /// Update robot connection defaults - all fields are required (no global fallback).
    #[serde(rename = "update_robot_connection_defaults")]
    UpdateRobotConnectionDefaults {
        id: i64,
        default_speed: f64,
        default_term_type: String,
        default_uframe: i32,
        default_utool: i32,
        default_w: f64,
        default_p: f64,
        default_r: f64,
        // Robot arm configuration defaults
        default_front: i32,
        default_up: i32,
        default_left: i32,
        default_flip: i32,
        default_turn4: i32,
        default_turn5: i32,
        default_turn6: i32,
    },

    /// Update robot connection jog defaults.
    #[serde(rename = "update_robot_jog_defaults")]
    UpdateRobotJogDefaults {
        id: i64,
        cartesian_jog_speed: f64,
        cartesian_jog_step: f64,
        joint_jog_speed: f64,
        joint_jog_step: f64,
    },

    #[serde(rename = "delete_robot_connection")]
    DeleteRobotConnection { id: i64 },

    // Robot Configurations (Named configurations per robot)
    #[serde(rename = "list_robot_configurations")]
    ListRobotConfigurations { robot_connection_id: i64 },

    #[serde(rename = "get_robot_configuration")]
    GetRobotConfiguration { id: i64 },

    #[serde(rename = "create_robot_configuration")]
    CreateRobotConfiguration {
        robot_connection_id: i64,
        name: String,
        is_default: bool,
        u_frame_number: i32,
        u_tool_number: i32,
        front: i32,
        up: i32,
        left: i32,
        flip: i32,
        turn4: i32,
        turn5: i32,
        turn6: i32,
    },

    #[serde(rename = "update_robot_configuration")]
    UpdateRobotConfiguration {
        id: i64,
        name: String,
        is_default: bool,
        u_frame_number: i32,
        u_tool_number: i32,
        front: i32,
        up: i32,
        left: i32,
        flip: i32,
        turn4: i32,
        turn5: i32,
        turn6: i32,
    },

    #[serde(rename = "delete_robot_configuration")]
    DeleteRobotConfiguration { id: i64 },

    #[serde(rename = "set_default_robot_configuration")]
    SetDefaultRobotConfiguration { id: i64 },

    /// Get the current active configuration state (runtime, not persisted)
    #[serde(rename = "get_active_configuration")]
    GetActiveConfiguration,

    /// Load a saved configuration as the active configuration
    #[serde(rename = "load_configuration")]
    LoadConfiguration { configuration_id: i64 },

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

    // I/O Management - Digital
    #[serde(rename = "read_din")]
    ReadDin { port_number: u16 },

    #[serde(rename = "write_dout")]
    WriteDout { port_number: u16, port_value: bool },

    #[serde(rename = "read_din_batch")]
    ReadDinBatch { port_numbers: Vec<u16> },

    // I/O Management - Analog
    #[serde(rename = "read_ain")]
    ReadAin { port_number: u16 },

    #[serde(rename = "write_aout")]
    WriteAout { port_number: u16, port_value: f64 },

    // I/O Management - Group
    #[serde(rename = "read_gin")]
    ReadGin { port_number: u16 },

    #[serde(rename = "write_gout")]
    WriteGout { port_number: u16, port_value: u32 },

    // I/O Configuration
    /// Get I/O display configuration for a robot
    #[serde(rename = "get_io_config")]
    GetIoConfig { robot_connection_id: i64 },

    /// Update I/O display configuration
    #[serde(rename = "update_io_config")]
    UpdateIoConfig {
        robot_connection_id: i64,
        io_type: String,
        io_index: i32,
        display_name: Option<String>,
        is_visible: bool,
        display_order: Option<i32>,
    },

    // Control Locking
    /// Request control of the robot (only one client can control at a time)
    #[serde(rename = "request_control")]
    RequestControl,

    /// Release control of the robot
    #[serde(rename = "release_control")]
    ReleaseControl,

    /// Get current control status (who has control)
    #[serde(rename = "get_control_status")]
    GetControlStatus,
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
        /// Name of the saved connection if connected to a saved robot
        connection_name: Option<String>,
        /// ID of the saved connection if connected to a saved robot
        connection_id: Option<i64>,
        /// Whether the TP program is initialized and ready for motion commands
        tp_program_initialized: bool,
    },

    /// Response when connecting to a saved robot connection.
    /// Includes the effective settings (per-robot or global fallback).
    #[serde(rename = "robot_connected")]
    RobotConnected {
        connection_id: i64,
        connection_name: String,
        robot_addr: String,
        robot_port: u32,
        /// Effective settings (per-robot defaults or global fallback)
        effective_speed: f64,
        effective_term_type: String,
        effective_uframe: i32,
        effective_utool: i32,
        effective_w: f64,
        effective_p: f64,
        effective_r: f64,
    },

    /// Broadcast when robot connection is lost unexpectedly.
    #[serde(rename = "robot_disconnected")]
    RobotDisconnected {
        reason: String,
    },

    /// Broadcast when a robot protocol error occurs.
    #[serde(rename = "robot_error")]
    RobotError {
        error_type: String, // "protocol", "command", "communication"
        message: String,
        error_id: Option<i32>,
    },

    /// Response to robot control commands (abort, reset, initialize).
    #[serde(rename = "robot_command_result")]
    RobotCommandResult {
        command: String, // "abort", "reset", "initialize"
        success: bool,
        error_id: Option<i32>,
        message: Option<String>,
    },

    /// Broadcast when execution state changes (for multi-client sync).
    #[serde(rename = "execution_state_changed")]
    ExecutionStateChanged {
        state: String, // "idle", "running", "paused", "stopping", "completed", "error"
        program_id: Option<i64>,
        current_line: Option<usize>,
        total_lines: Option<usize>,
        message: Option<String>,
    },

    #[serde(rename = "robot_connections")]
    RobotConnections { connections: Vec<RobotConnectionDto> },

    #[serde(rename = "robot_connection")]
    RobotConnection { connection: RobotConnectionDto },

    // Robot Configuration responses
    #[serde(rename = "robot_configurations")]
    RobotConfigurations { configurations: Vec<RobotConfigurationDto> },

    #[serde(rename = "robot_configuration")]
    RobotConfigurationResponse { configuration: RobotConfigurationDto },

    #[serde(rename = "active_configuration")]
    ActiveConfigurationResponse {
        /// ID of the saved configuration this was loaded from (None = custom/unsaved)
        loaded_from_id: Option<i64>,
        /// Name of the loaded configuration
        loaded_from_name: Option<String>,
        /// Whether the active config has been modified from the loaded state
        modified: bool,
        /// Current UFrame number
        u_frame_number: i32,
        /// Current UTool number
        u_tool_number: i32,
        /// Arm configuration - Front(1)/Back(0)
        front: i32,
        /// Arm configuration - Up(1)/Down(0)
        up: i32,
        /// Arm configuration - Left(1)/Right(0)
        left: i32,
        /// Wrist configuration - Flip(1)/NoFlip(0)
        flip: i32,
        /// J4 turn number
        turn4: i32,
        /// J5 turn number
        turn5: i32,
        /// J6 turn number
        turn6: i32,
    },

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

    #[serde(rename = "ain_value")]
    AinValue { port_number: u16, port_value: f64 },

    #[serde(rename = "gin_value")]
    GinValue { port_number: u16, port_value: u32 },

    // I/O configuration responses
    #[serde(rename = "io_config")]
    IoConfig { configs: Vec<IoDisplayConfigDto> },

    // Control lock responses
    /// Control of the robot was acquired
    #[serde(rename = "control_acquired")]
    ControlAcquired,

    /// Control was released
    #[serde(rename = "control_released")]
    ControlReleased,

    /// Control request was denied (another client has control)
    #[serde(rename = "control_denied")]
    ControlDenied {
        holder_id: String, // UUID as string for JSON
        reason: String,
    },

    /// Control was lost (timeout, transfer, disconnect)
    #[serde(rename = "control_lost")]
    ControlLost { reason: String },

    /// Another client acquired control (notification to observers)
    #[serde(rename = "control_changed")]
    ControlChanged {
        /// New holder UUID (None if released)
        holder_id: Option<String>,
    },

    /// Current control status
    #[serde(rename = "control_status")]
    ControlStatus {
        /// Whether this client has control
        has_control: bool,
        /// Current holder UUID (if any)
        holder_id: Option<String>,
    },
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
/// All defaults are required (non-optional) - each robot has its own explicit settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotConnectionDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub ip_address: String,
    pub port: u32,
    // Per-robot defaults (required - no global fallback)
    pub default_speed: f64,
    pub default_term_type: String,
    pub default_uframe: i32,
    pub default_utool: i32,
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    // Robot arm configuration defaults (required)
    pub default_front: i32,
    pub default_up: i32,
    pub default_left: i32,
    pub default_flip: i32,
    pub default_turn4: i32,
    pub default_turn5: i32,
    pub default_turn6: i32,
    // Jog defaults
    pub default_cartesian_jog_speed: f64,
    pub default_cartesian_jog_step: f64,
    pub default_joint_jog_speed: f64,
    pub default_joint_jog_step: f64,
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

/// I/O display configuration DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoDisplayConfigDto {
    pub io_type: String,
    pub io_index: i32,
    pub display_name: Option<String>,
    pub is_visible: bool,
    pub display_order: Option<i32>,
}
