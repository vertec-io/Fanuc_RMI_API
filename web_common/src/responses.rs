//! Server response types for WebSocket API.

use serde::{Deserialize, Serialize};
use fanuc_rmi::dto::FrameData;
use crate::{
    ProgramInfo, ProgramDetail, RobotSettingsDto, RobotConnectionDto,
    RobotConfigurationDto, ChangeLogEntryDto, IoDisplayConfigDto,
};

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
        status: String,
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
        connection_name: Option<String>,
        connection_id: Option<i64>,
        tp_program_initialized: bool,
    },

    #[serde(rename = "robot_connected")]
    RobotConnected {
        connection_id: i64,
        connection_name: String,
        robot_addr: String,
        robot_port: u32,
        effective_speed: f64,
        effective_term_type: String,
        effective_uframe: i32,
        effective_utool: i32,
        effective_w: f64,
        effective_p: f64,
        effective_r: f64,
    },

    #[serde(rename = "robot_disconnected")]
    RobotDisconnected {
        reason: String,
    },

    #[serde(rename = "robot_error")]
    RobotError {
        error_type: String,
        message: String,
        error_id: Option<i32>,
        raw_data: Option<String>,
    },

    #[serde(rename = "robot_command_result")]
    RobotCommandResult {
        command: String,
        success: bool,
        error_id: Option<i32>,
        message: Option<String>,
    },

    #[serde(rename = "execution_state_changed")]
    ExecutionStateChanged {
        state: String,
        program_id: Option<i64>,
        current_line: Option<usize>,
        total_lines: Option<usize>,
        message: Option<String>,
    },

    #[serde(rename = "robot_connections")]
    RobotConnections { connections: Vec<RobotConnectionDto> },

    #[serde(rename = "robot_connection")]
    RobotConnection { connection: RobotConnectionDto },

    #[serde(rename = "robot_connection_created")]
    RobotConnectionCreated {
        id: i64,
        connection: RobotConnectionDto,
        configurations: Vec<RobotConfigurationDto>,
    },

    #[serde(rename = "robot_configuration_list")]
    RobotConfigurationList { configurations: Vec<RobotConfigurationDto> },

    #[serde(rename = "robot_configuration")]
    RobotConfigurationResponse { configuration: RobotConfigurationDto },

    #[serde(rename = "active_configuration")]
    ActiveConfigurationResponse {
        loaded_from_id: Option<i64>,
        loaded_from_name: Option<String>,
        changes_count: u32,
        change_log: Vec<ChangeLogEntryDto>,
        u_frame_number: i32,
        u_tool_number: i32,
        front: i32,
        up: i32,
        left: i32,
        flip: i32,
        turn4: i32,
        turn5: i32,
        turn6: i32,
        default_cartesian_jog_speed: f64,
        default_cartesian_jog_step: f64,
        default_joint_jog_speed: f64,
        default_joint_jog_step: f64,
    },

    #[serde(rename = "active_jog_settings")]
    ActiveJogSettings {
        cartesian_jog_speed: f64,
        cartesian_jog_step: f64,
        joint_jog_speed: f64,
        joint_jog_step: f64,
    },

    // Frame/Tool responses
    #[serde(rename = "active_frame_tool")]
    ActiveFrameTool { uframe: u8, utool: u8 },

    /// Frame data response - uses fanuc_rmi::dto::FrameData for the coordinate data
    #[serde(rename = "frame_data")]
    FrameDataResponse {
        frame_number: u8,
        #[serde(flatten)]
        data: FrameData,
    },

    /// Tool data response - uses fanuc_rmi::dto::FrameData for the coordinate data
    #[serde(rename = "tool_data")]
    ToolDataResponse {
        tool_number: u8,
        #[serde(flatten)]
        data: FrameData,
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
    #[serde(rename = "control_acquired")]
    ControlAcquired,

    #[serde(rename = "control_released")]
    ControlReleased,

    #[serde(rename = "control_denied")]
    ControlDenied {
        holder_id: String,
        reason: String,
    },

    #[serde(rename = "control_lost")]
    ControlLost { reason: String },

    #[serde(rename = "control_changed")]
    ControlChanged {
        holder_id: Option<String>,
    },

    #[serde(rename = "control_status")]
    ControlStatus {
        has_control: bool,
        holder_id: Option<String>,
    },
}

