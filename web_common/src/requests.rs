//! Client request types for WebSocket API.

use serde::{Deserialize, Serialize};
use fanuc_rmi::dto::FrameData;
use crate::{StartPosition, NewRobotConfigurationDto};

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
    #[serde(rename = "upload_csv")]
    UploadCsv {
        program_id: i64,
        csv_content: String,
        start_position: Option<StartPosition>,
    },

    // Program Execution
    #[serde(rename = "load_program")]
    LoadProgram { program_id: i64 },

    #[serde(rename = "unload_program")]
    UnloadProgram,

    #[serde(rename = "start_program")]
    StartProgram { program_id: i64 },

    #[serde(rename = "pause_program")]
    PauseProgram,

    #[serde(rename = "resume_program")]
    ResumeProgram,

    #[serde(rename = "stop_program")]
    StopProgram,

    #[serde(rename = "get_execution_state")]
    GetExecutionState,

    // Robot Control Commands
    #[serde(rename = "robot_abort")]
    RobotAbort,

    #[serde(rename = "robot_reset")]
    RobotReset,

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

    /// Create robot connection with configurations atomically.
    #[serde(rename = "create_robot_with_configurations")]
    CreateRobotWithConfigurations {
        name: String,
        description: Option<String>,
        ip_address: String,
        port: u32,
        default_speed: f64,
        default_speed_type: String,
        default_term_type: String,
        default_w: f64,
        default_p: f64,
        default_r: f64,
        default_cartesian_jog_speed: f64,
        default_cartesian_jog_step: f64,
        default_joint_jog_speed: f64,
        default_joint_jog_step: f64,
        configurations: Vec<NewRobotConfigurationDto>,
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
        default_speed: f64,
        default_speed_type: String,
        default_term_type: String,
        default_w: f64,
        default_p: f64,
        default_r: f64,
    },

    #[serde(rename = "update_robot_jog_defaults")]
    UpdateRobotJogDefaults {
        id: i64,
        cartesian_jog_speed: f64,
        cartesian_jog_step: f64,
        joint_jog_speed: f64,
        joint_jog_step: f64,
    },

    #[serde(rename = "update_jog_controls")]
    UpdateJogControls {
        cartesian_jog_speed: f64,
        cartesian_jog_step: f64,
        joint_jog_speed: f64,
        joint_jog_step: f64,
    },

    #[serde(rename = "apply_jog_settings")]
    ApplyJogSettings {
        cartesian_jog_speed: f64,
        cartesian_jog_step: f64,
        joint_jog_speed: f64,
        joint_jog_step: f64,
    },

    #[serde(rename = "save_current_configuration")]
    SaveCurrentConfiguration {
        configuration_name: Option<String>,
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

    #[serde(rename = "get_active_configuration")]
    GetActiveConfiguration,

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

    /// Write frame data - uses fanuc_rmi::dto::FrameData for the coordinate data
    #[serde(rename = "write_frame_data")]
    WriteFrameData {
        frame_number: u8,
        #[serde(flatten)]
        data: FrameData,
    },

    /// Write tool data - uses fanuc_rmi::dto::FrameData for the coordinate data
    #[serde(rename = "write_tool_data")]
    WriteToolData {
        tool_number: u8,
        #[serde(flatten)]
        data: FrameData,
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
    #[serde(rename = "get_io_config")]
    GetIoConfig { robot_connection_id: i64 },

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
    #[serde(rename = "request_control")]
    RequestControl,

    #[serde(rename = "release_control")]
    ReleaseControl,

    #[serde(rename = "get_control_status")]
    GetControlStatus,
}

