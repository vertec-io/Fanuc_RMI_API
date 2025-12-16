use fanuc_rmi::dto::*;
use leptos::prelude::*;
use leptos::reactive::owner::LocalStorage;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{BinaryType, ErrorEvent, MessageEvent, WebSocket};

// Re-export shared API types from web_common
pub use web_common::{
    ClientRequest, ServerResponse,
    StartPosition, ProgramInfo, ProgramDetail,
    RobotConnectionDto, RobotConfigurationDto, NewRobotConfigurationDto,
    RobotSettingsDto, IoDisplayConfigDto, ChangeLogEntryDto,
};

/// Frame or Tool coordinate data (X, Y, Z, W, P, R)
/// (client-only type for local state management)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrameToolData {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
}

// ========== WebSocket Manager ==========

#[derive(Clone, Copy)]
pub struct WebSocketManager {
    pub connected: ReadSignal<bool>,
    set_connected: WriteSignal<bool>,
    /// WebSocket is attempting to connect
    pub ws_connecting: ReadSignal<bool>,
    pub set_ws_connecting: WriteSignal<bool>,
    pub position: ReadSignal<Option<(f64, f64, f64)>>,
    set_position: WriteSignal<Option<(f64, f64, f64)>>,
    /// Orientation data (W, P, R angles in degrees)
    pub orientation: ReadSignal<Option<(f64, f64, f64)>>,
    set_orientation: WriteSignal<Option<(f64, f64, f64)>>,
    pub joint_angles: ReadSignal<Option<[f32; 6]>>,
    set_joint_angles: WriteSignal<Option<[f32; 6]>>,
    pub status: ReadSignal<Option<RobotStatusData>>,
    set_status: WriteSignal<Option<RobotStatusData>>,
    pub motion_log: ReadSignal<Vec<String>>,
    set_motion_log: WriteSignal<Vec<String>>,
    pub error_log: ReadSignal<Vec<String>>,
    set_error_log: WriteSignal<Vec<String>>,
    // API response signals
    pub programs: ReadSignal<Vec<ProgramInfo>>,
    set_programs: WriteSignal<Vec<ProgramInfo>>,
    pub current_program: ReadSignal<Option<ProgramDetail>>,
    set_current_program: WriteSignal<Option<ProgramDetail>>,
    pub settings: ReadSignal<Option<RobotSettingsDto>>,
    set_settings: WriteSignal<Option<RobotSettingsDto>>,
    /// API message for toast notifications
    pub api_message: ReadSignal<Option<String>>,
    set_api_message: WriteSignal<Option<String>>,
    /// API error message (cleared on next successful response)
    pub api_error: ReadSignal<Option<String>>,
    set_api_error: WriteSignal<Option<String>>,
    /// Execution status for progress display
    pub execution_status: ReadSignal<Option<ExecutionStatusData>>,
    set_execution_status: WriteSignal<Option<ExecutionStatusData>>,
    // Program execution state
    pub program_running: ReadSignal<bool>,
    set_program_running: WriteSignal<bool>,
    /// Program is paused
    pub program_paused: ReadSignal<bool>,
    set_program_paused: WriteSignal<bool>,
    /// Currently loaded program ID (from server state)
    pub loaded_program_id: ReadSignal<Option<i64>>,
    set_loaded_program_id: WriteSignal<Option<i64>>,
    /// Program progress (completed_line, total_lines)
    pub program_progress: ReadSignal<Option<(usize, usize)>>,
    set_program_progress: WriteSignal<Option<(usize, usize)>>,
    pub executing_line: ReadSignal<Option<usize>>,  // The line currently being executed
    set_executing_line: WriteSignal<Option<usize>>,
    // Robot connection status
    pub robot_connected: ReadSignal<bool>,
    set_robot_connected: WriteSignal<bool>,
    /// Robot is attempting to connect
    pub robot_connecting: ReadSignal<bool>,
    pub set_robot_connecting: WriteSignal<bool>,
    pub robot_addr: ReadSignal<String>,
    set_robot_addr: WriteSignal<String>,
    /// Name of the currently connected robot (from saved connection)
    pub connected_robot_name: ReadSignal<Option<String>>,
    set_connected_robot_name: WriteSignal<Option<String>>,
    /// Whether the TP program is initialized and ready for motion commands.
    /// This must be true to send motion commands. False after abort/disconnect.
    pub tp_program_initialized: ReadSignal<bool>,
    set_tp_program_initialized: WriteSignal<bool>,
    // Saved robot connections
    pub robot_connections: ReadSignal<Vec<RobotConnectionDto>>,
    set_robot_connections: WriteSignal<Vec<RobotConnectionDto>>,
    // Currently active/selected connection
    pub active_connection_id: ReadSignal<Option<i64>>,
    set_active_connection_id: WriteSignal<Option<i64>>,
    // Frame/Tool data
    /// Active UFrame and UTool numbers from robot
    pub active_frame_tool: ReadSignal<Option<(u8, u8)>>,
    set_active_frame_tool: WriteSignal<Option<(u8, u8)>>,
    /// Frame data cache - indexed by frame number (0-9)
    pub frame_data: ReadSignal<HashMap<u8, FrameToolData>>,
    set_frame_data: WriteSignal<HashMap<u8, FrameToolData>>,
    /// Tool data cache - indexed by tool number (0-9)
    pub tool_data: ReadSignal<HashMap<u8, FrameToolData>>,
    set_tool_data: WriteSignal<HashMap<u8, FrameToolData>>,
    // I/O data
    /// Digital input values - indexed by port number
    pub din_values: ReadSignal<HashMap<u16, bool>>,
    set_din_values: WriteSignal<HashMap<u16, bool>>,
    /// Digital output values - indexed by port number
    pub dout_values: ReadSignal<HashMap<u16, bool>>,
    set_dout_values: WriteSignal<HashMap<u16, bool>>,
    /// Analog input values - indexed by port number
    pub ain_values: ReadSignal<HashMap<u16, f64>>,
    set_ain_values: WriteSignal<HashMap<u16, f64>>,
    /// Analog output values - indexed by port number
    pub aout_values: ReadSignal<HashMap<u16, f64>>,
    set_aout_values: WriteSignal<HashMap<u16, f64>>,
    /// Group input values - indexed by port number
    pub gin_values: ReadSignal<HashMap<u16, u32>>,
    set_gin_values: WriteSignal<HashMap<u16, u32>>,
    /// Group output values - indexed by port number
    pub gout_values: ReadSignal<HashMap<u16, u32>>,
    set_gout_values: WriteSignal<HashMap<u16, u32>>,
    /// I/O display configuration - keyed by (io_type, io_index)
    pub io_config: ReadSignal<HashMap<(String, i32), IoDisplayConfigDto>>,
    set_io_config: WriteSignal<HashMap<(String, i32), IoDisplayConfigDto>>,
    // Control lock state
    /// Whether this client has control of the robot
    pub has_control: ReadSignal<bool>,
    set_has_control: WriteSignal<bool>,
    // Active configuration state
    /// Active configuration for the connected robot
    pub active_configuration: ReadSignal<Option<ActiveConfigurationData>>,
    set_active_configuration: WriteSignal<Option<ActiveConfigurationData>>,
    /// List of saved robot configurations
    pub robot_configurations: ReadSignal<Vec<RobotConfigurationDto>>,
    set_robot_configurations: WriteSignal<Vec<RobotConfigurationDto>>,
    // Active jog settings (server-driven state)
    /// Active jog settings from server
    pub active_jog_settings: ReadSignal<Option<ActiveJogSettingsData>>,
    set_active_jog_settings: WriteSignal<Option<ActiveJogSettingsData>>,
    /// Console messages for chronological display
    pub console_messages: ReadSignal<Vec<ConsoleMessage>>,
    set_console_messages: WriteSignal<Vec<ConsoleMessage>>,
    ws: StoredValue<Option<WebSocket>, LocalStorage>,
    ws_url: StoredValue<String>,
}

/// Active jog settings data (client-side representation of server state)
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveJogSettingsData {
    pub cartesian_jog_speed: f64,
    pub cartesian_jog_step: f64,
    pub joint_jog_speed: f64,
    pub joint_jog_step: f64,
    pub rotation_jog_speed: f64,
    pub rotation_jog_step: f64,
}

/// Unified console message with timestamp and direction
#[derive(Clone, Debug)]
pub struct ConsoleMessage {
    /// Timestamp in HH:MM:SS.mmm format
    pub timestamp: String,
    /// Milliseconds since epoch for sorting
    pub timestamp_ms: u64,
    /// Message direction
    pub direction: MessageDirection,
    /// Message type
    pub msg_type: MessageType,
    /// Message content
    pub content: String,
    /// Optional sequence ID for matching requests/responses
    pub sequence_id: Option<u32>,
}

/// Direction of the message
#[derive(Clone, Debug, PartialEq)]
pub enum MessageDirection {
    /// Sent to robot (→)
    Sent,
    /// Received from robot (←)
    Received,
    /// System/status message (•)
    System,
}

/// Type of console message
#[derive(Clone, Debug, PartialEq)]
pub enum MessageType {
    /// Command sent to robot
    Command,
    /// Response from robot (success)
    Response,
    /// Error response
    Error,
    /// Status update
    Status,
    /// Configuration change
    Config,
}

#[derive(Clone, Debug)]
pub struct RobotStatusData {
    pub servo_ready: i8,
    pub tp_mode: i8,
    pub motion_status: i8,
    pub speed_override: u32,
}

/// Execution status data for progress display
#[derive(Clone, Debug)]
pub struct ExecutionStatusData {
    pub status: String,
    /// Current line from execution status (prefer program_progress signal for progress tracking)
    #[allow(dead_code)]
    pub current_line: Option<usize>,
    /// Total lines from execution status (prefer program_progress signal for progress tracking)
    #[allow(dead_code)]
    pub total_lines: Option<usize>,
    pub error: Option<String>,
}

/// Active configuration data for display in sidebar
#[derive(Clone, Debug, Default)]
pub struct ActiveConfigurationData {
    /// ID of the saved configuration this was loaded from (None = custom/unsaved)
    pub loaded_from_id: Option<i64>,
    /// Name of the loaded configuration
    pub loaded_from_name: Option<String>,
    /// Number of configuration changes applied since loading (0 = unmodified)
    pub changes_count: u32,
    /// Changelog tracking all changes since loading
    pub change_log: Vec<ChangeLogEntryDto>,
    /// Current UFrame number
    pub u_frame_number: i32,
    /// Current UTool number
    pub u_tool_number: i32,
    /// Arm configuration - Front(1)/Back(0)
    pub front: i32,
    /// Arm configuration - Up(1)/Down(0)
    pub up: i32,
    /// Arm configuration - Left(1)/Right(0)
    pub left: i32,
    /// Wrist configuration - Flip(1)/NoFlip(0)
    pub flip: i32,
    /// J4 turn number
    pub turn4: i32,
    /// J5 turn number
    pub turn5: i32,
    /// J6 turn number
    pub turn6: i32,
    /// Active default jog settings (applied but not yet saved to database)
    pub default_cartesian_jog_speed: f64,
    pub default_cartesian_jog_step: f64,
    pub default_joint_jog_speed: f64,
    pub default_joint_jog_step: f64,
    pub default_rotation_jog_speed: f64,
    pub default_rotation_jog_step: f64,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (connected, set_connected) = signal(false);
        let (ws_connecting, set_ws_connecting) = signal(false);
        let (position, set_position) = signal(None);
        let (orientation, set_orientation) = signal(None);
        let (joint_angles, set_joint_angles) = signal(None);
        let (status, set_status) = signal(None);
        let (motion_log, set_motion_log) = signal(Vec::new());
        let (error_log, set_error_log) = signal(Vec::new());
        // API signals
        let (programs, set_programs) = signal(Vec::new());
        let (current_program, set_current_program) = signal(None);
        let (settings, set_settings) = signal(None);
        let (api_message, set_api_message) = signal(None);
        let (api_error, set_api_error) = signal::<Option<String>>(None);
        let (execution_status, set_execution_status) = signal(None);
        // Program execution state
        let (program_running, set_program_running) = signal(false);
        let (program_paused, set_program_paused) = signal(false);
        let (loaded_program_id, set_loaded_program_id) = signal::<Option<i64>>(None);
        let (program_progress, set_program_progress) = signal(None);
        let (executing_line, set_executing_line) = signal(None);
        // Robot connection status
        let (robot_connected, set_robot_connected) = signal(false);
        let (robot_connecting, set_robot_connecting) = signal(false);
        let (robot_addr, set_robot_addr) = signal("127.0.0.1:16001".to_string());
        let (connected_robot_name, set_connected_robot_name) = signal::<Option<String>>(None);
        let (tp_program_initialized, set_tp_program_initialized) = signal(false);
        // Saved robot connections
        let (robot_connections, set_robot_connections) = signal(Vec::new());
        // Currently active/selected connection
        let (active_connection_id, set_active_connection_id) = signal::<Option<i64>>(None);
        // Frame/Tool data
        let (active_frame_tool, set_active_frame_tool) = signal::<Option<(u8, u8)>>(None);
        let (frame_data, set_frame_data) = signal::<HashMap<u8, FrameToolData>>(HashMap::new());
        let (tool_data, set_tool_data) = signal::<HashMap<u8, FrameToolData>>(HashMap::new());
        // I/O data
        let (din_values, set_din_values) = signal::<HashMap<u16, bool>>(HashMap::new());
        let (dout_values, set_dout_values) = signal::<HashMap<u16, bool>>(HashMap::new());
        let (ain_values, set_ain_values) = signal::<HashMap<u16, f64>>(HashMap::new());
        let (aout_values, set_aout_values) = signal::<HashMap<u16, f64>>(HashMap::new());
        let (gin_values, set_gin_values) = signal::<HashMap<u16, u32>>(HashMap::new());
        let (gout_values, set_gout_values) = signal::<HashMap<u16, u32>>(HashMap::new());
        let (io_config, set_io_config) = signal::<HashMap<(String, i32), IoDisplayConfigDto>>(HashMap::new());
        // Control lock state
        let (has_control, set_has_control) = signal(false);
        // Active configuration state
        let (active_configuration, set_active_configuration) = signal::<Option<ActiveConfigurationData>>(None);
        let (robot_configurations, set_robot_configurations) = signal::<Vec<RobotConfigurationDto>>(Vec::new());
        // Active jog settings (server-driven state)
        let (active_jog_settings, set_active_jog_settings) = signal::<Option<ActiveJogSettingsData>>(None);
        // Console messages
        let (console_messages, set_console_messages) = signal::<Vec<ConsoleMessage>>(Vec::new());
        let ws: StoredValue<Option<WebSocket>, LocalStorage> = StoredValue::new_local(None);
        let ws_url = StoredValue::new("ws://127.0.0.1:9000".to_string());

        let manager = Self {
            connected,
            set_connected,
            ws_connecting,
            set_ws_connecting,
            position,
            set_position,
            orientation,
            set_orientation,
            joint_angles,
            set_joint_angles,
            status,
            set_status,
            motion_log,
            set_motion_log,
            error_log,
            set_error_log,
            programs,
            set_programs,
            current_program,
            set_current_program,
            settings,
            set_settings,
            api_message,
            set_api_message,
            api_error,
            set_api_error,
            execution_status,
            set_execution_status,
            program_running,
            set_program_running,
            program_paused,
            set_program_paused,
            loaded_program_id,
            set_loaded_program_id,
            program_progress,
            set_program_progress,
            executing_line,
            set_executing_line,
            robot_connected,
            set_robot_connected,
            robot_connecting,
            set_robot_connecting,
            robot_addr,
            set_robot_addr,
            connected_robot_name,
            set_connected_robot_name,
            tp_program_initialized,
            set_tp_program_initialized,
            robot_connections,
            set_robot_connections,
            active_connection_id,
            set_active_connection_id,
            active_frame_tool,
            set_active_frame_tool,
            frame_data,
            set_frame_data,
            tool_data,
            set_tool_data,
            din_values,
            set_din_values,
            dout_values,
            set_dout_values,
            ain_values,
            set_ain_values,
            aout_values,
            set_aout_values,
            gin_values,
            set_gin_values,
            gout_values,
            set_gout_values,
            io_config,
            set_io_config,
            has_control,
            set_has_control,
            active_configuration,
            set_active_configuration,
            robot_configurations,
            set_robot_configurations,
            active_jog_settings,
            set_active_jog_settings,
            console_messages,
            set_console_messages,
            ws,
            ws_url,
        };

        manager.connect();
        manager
    }

    fn connect(&self) {
        let url = self.ws_url.get_value();
        let ws = match WebSocket::new(&url) {
            Ok(ws) => ws,
            Err(e) => {
                log::error!("Failed to create WebSocket: {:?}", e);
                return;
            }
        };
        ws.set_binary_type(BinaryType::Arraybuffer);

        let set_connected = self.set_connected;
        let set_ws_connecting = self.set_ws_connecting;
        let set_position = self.set_position;
        let set_orientation = self.set_orientation;
        let set_joint_angles = self.set_joint_angles;
        let set_status = self.set_status;
        let set_motion_log = self.set_motion_log;
        let set_error_log = self.set_error_log;
        let set_programs = self.set_programs;
        let set_current_program = self.set_current_program;
        let set_settings = self.set_settings;
        let set_api_message = self.set_api_message;
        let set_api_error = self.set_api_error;
        let set_execution_status = self.set_execution_status;
        let set_program_running = self.set_program_running;
        let set_program_paused = self.set_program_paused;
        let set_loaded_program_id = self.set_loaded_program_id;
        let set_program_progress = self.set_program_progress;
        let set_executing_line = self.set_executing_line;
        let set_robot_connected = self.set_robot_connected;
        let set_robot_connecting = self.set_robot_connecting;
        let set_robot_addr = self.set_robot_addr;
        let set_connected_robot_name = self.set_connected_robot_name;
        let set_tp_program_initialized = self.set_tp_program_initialized;
        let set_robot_connections = self.set_robot_connections;
        let set_active_connection_id = self.set_active_connection_id;
        let set_active_frame_tool = self.set_active_frame_tool;
        let set_frame_data = self.set_frame_data;
        let set_tool_data = self.set_tool_data;
        let set_din_values = self.set_din_values;
        // Output values are now updated from server broadcasts (not optimistically)
        let set_dout_values = self.set_dout_values;
        let set_ain_values = self.set_ain_values;
        let set_aout_values = self.set_aout_values;
        let set_gin_values = self.set_gin_values;
        let set_gout_values = self.set_gout_values;
        let set_io_config = self.set_io_config;
        let set_has_control = self.set_has_control;
        let set_active_configuration = self.set_active_configuration;
        let set_robot_configurations = self.set_robot_configurations;
        let set_active_jog_settings = self.set_active_jog_settings;
        let set_console_messages = self.set_console_messages;

        // On open
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            set_connected.set(true);
            set_ws_connecting.set(false);
            log::info!("WebSocket connected");
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        // On message - handles both binary (robot protocol) and text (API JSON)
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            // Handle binary messages (robot protocol via bincode)
            if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                let bytes = uint8_array.to_vec();

                if let Ok(response) = bincode::deserialize::<ResponsePacket>(&bytes) {
                    match response {
                        ResponsePacket::InstructionResponse(resp) => {
                            let (seq_id, error_id) = get_response_ids(&resp);
                            let resp_name = get_instruction_response_name(&resp);

                            // Log to console
                            add_console_msg(
                                set_console_messages,
                                MessageDirection::Received,
                                if error_id != 0 { MessageType::Error } else { MessageType::Response },
                                if error_id != 0 {
                                    format!("{} error_id={}", resp_name, error_id)
                                } else {
                                    format!("{} completed", resp_name)
                                },
                                Some(seq_id),
                            );

                            if error_id != 0 {
                                set_error_log.update(|log| {
                                    log.push(format!("Motion error: Seq:{} Err:{}", seq_id, error_id));
                                    if log.len() > 10 {
                                        log.remove(0);
                                    }
                                });
                            } else {
                                let msg = format_instruction_response(&resp, seq_id);
                                set_motion_log.update(|log| {
                                    log.push(msg);
                                    if log.len() > 50 {
                                        log.remove(0);
                                    }
                                });
                            }
                        }
                        ResponsePacket::CommandResponse(resp) => match resp {
                            CommandResponse::FrcReadCartesianPosition(r) => {
                                if r.error_id == 0 {
                                    set_position.set(Some((
                                        r.pos.x as f64,
                                        r.pos.y as f64,
                                        r.pos.z as f64,
                                    )));
                                    set_orientation.set(Some((
                                        r.pos.w as f64,
                                        r.pos.p as f64,
                                        r.pos.r as f64,
                                    )));
                                }
                            }
                            CommandResponse::FrcReadJointAngles(r) => {
                                if r.error_id == 0 {
                                    set_joint_angles.set(Some([
                                        r.joint_angles.j1,
                                        r.joint_angles.j2,
                                        r.joint_angles.j3,
                                        r.joint_angles.j4,
                                        r.joint_angles.j5,
                                        r.joint_angles.j6,
                                    ]));
                                }
                            }
                            CommandResponse::FrcGetStatus(s) => {
                                if s.error_id == 0 {
                                    set_status.set(Some(RobotStatusData {
                                        servo_ready: s.servo_ready,
                                        tp_mode: s.tp_mode,
                                        motion_status: s.rmi_motion_status,
                                        speed_override: s.override_value,
                                    }));
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                } else {
                    log::error!("Failed to deserialize binary response (length: {} bytes)", bytes.len());
                    // Add parse error to console with hex dump for debugging
                    let hex_dump = if bytes.len() <= 64 {
                        bytes.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")
                    } else {
                        format!("{}... ({} bytes total)",
                            bytes[..64].iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" "),
                            bytes.len())
                    };
                    add_console_msg(
                        set_console_messages,
                        MessageDirection::Received,
                        MessageType::Error,
                        format!("Failed to deserialize binary response. Hex: {}", hex_dump),
                        None,
                    );
                }
            }
            // Handle text messages (API JSON responses)
            else if let Ok(text) = e.data().dyn_into::<js_sys::JsString>() {
                let text_str: String = text.into();
                if let Ok(response) = serde_json::from_str::<ServerResponse>(&text_str) {
                    match response {
                        ServerResponse::Success { message } => {
                            log::info!("API Success: {}", message);
                            set_api_message.set(Some(message));
                            set_api_error.set(None); // Clear error on success
                        }
                        ServerResponse::Error { message } => {
                            log::error!("API Error: {}", message);
                            set_api_message.set(Some(format!("Error: {}", message)));
                            set_api_error.set(Some(message)); // Set error signal
                            // Clear connecting states on error
                            set_robot_connecting.set(false);
                        }
                        ServerResponse::Programs { programs } => {
                            log::info!("Received {} programs", programs.len());
                            set_programs.set(programs);
                        }
                        ServerResponse::Program { program } => {
                            log::info!("Received program: {}", program.name);
                            set_current_program.set(Some(program));
                        }
                        ServerResponse::Settings { settings } => {
                            log::info!("Received settings");
                            set_settings.set(Some(settings));
                        }
                        ServerResponse::ExecutionStatus { status, current_line, total_lines, error } => {
                            log::info!("Execution status: {}", status);
                            set_execution_status.set(Some(ExecutionStatusData {
                                status,
                                current_line,
                                total_lines,
                                error,
                            }));
                        }
                        ServerResponse::ExecutionStarted { program_id, total_lines } => {
                            log::info!("Program {} started with {} lines", program_id, total_lines);
                            set_program_running.set(true);
                            set_program_progress.set(Some((0, total_lines)));
                        }
                        ServerResponse::ProgramComplete { program_id, success, message } => {
                            log::info!("Program {} complete: success={}, message={:?}", program_id, success, message);
                            set_program_running.set(false);
                            set_program_progress.set(None);
                            set_executing_line.set(None);
                            if let Some(msg) = message {
                                set_api_message.set(Some(msg));
                            }
                        }
                        ServerResponse::InstructionProgress { current_line, total_lines } => {
                            log::debug!("Progress: {}/{}", current_line, total_lines);
                            set_program_progress.set(Some((current_line, total_lines)));
                        }
                        ServerResponse::InstructionSent { current_line, total_lines } => {
                            log::debug!("Executing: {}/{}", current_line, total_lines);
                            set_executing_line.set(Some(current_line));
                        }
                        ServerResponse::ConnectionStatus { connected, robot_addr, robot_port, connection_name, connection_id, tp_program_initialized } => {
                            log::info!("Robot connection status: connected={}, addr={}:{}, name={:?}, tp_initialized={}", connected, robot_addr, robot_port, connection_name, tp_program_initialized);
                            set_robot_connected.set(connected);
                            set_robot_addr.set(format!("{}:{}", robot_addr, robot_port));
                            set_tp_program_initialized.set(tp_program_initialized);
                            // Set connection name and ID from status (for page refresh)
                            if connected {
                                if let Some(name) = connection_name {
                                    set_connected_robot_name.set(Some(name));
                                }
                                if let Some(id) = connection_id {
                                    set_active_connection_id.set(Some(id));
                                }
                            } else {
                                // Clear connection name if disconnected
                                set_connected_robot_name.set(None);
                                set_active_connection_id.set(None);
                            }
                        }
                        ServerResponse::RobotConnected {
                            connection_id,
                            connection_name,
                            robot_addr,
                            robot_port,
                            effective_speed,
                            effective_term_type,
                            effective_uframe,
                            effective_utool,
                            effective_w,
                            effective_p,
                            effective_r,
                        } => {
                            log::info!("Connected to saved robot '{}' at {}:{}", connection_name, robot_addr, robot_port);
                            log::info!("Effective settings: speed={}, term={}, uframe={}, utool={}, wpr=({},{},{})",
                                effective_speed, effective_term_type, effective_uframe, effective_utool,
                                effective_w, effective_p, effective_r);
                            // Clear connecting state
                            set_robot_connecting.set(false);
                            set_robot_connected.set(true);
                            set_robot_addr.set(format!("{}:{}", robot_addr, robot_port));
                            // Set the active connection ID and name
                            set_active_connection_id.set(Some(connection_id));
                            set_connected_robot_name.set(Some(connection_name.clone()));
                            // Update settings with effective values from the saved connection
                            set_settings.set(Some(RobotSettingsDto {
                                default_w: effective_w,
                                default_p: effective_p,
                                default_r: effective_r,
                                default_speed: effective_speed,
                                default_term_type: effective_term_type,
                                default_uframe: effective_uframe,
                                default_utool: effective_utool,
                            }));
                            set_api_message.set(Some(format!("Connected to '{}'", connection_name)));
                        }
                        ServerResponse::RobotConnections { connections } => {
                            log::info!("Received {} robot connections", connections.len());
                            set_robot_connections.set(connections);
                        }
                        ServerResponse::RobotConnection { connection } => {
                            log::info!("Received robot connection: {}", connection.name);
                            // Could update a single connection in the list if needed
                        }
                        ServerResponse::RobotConnectionCreated { id, connection, configurations } => {
                            log::info!("Robot connection created: id={}, name={}, {} configurations",
                                id, connection.name, configurations.len());
                            // Refresh the robot connections list
                            // The wizard will handle closing itself via timeout
                            set_api_message.set(Some(format!("Robot '{}' created successfully", connection.name)));
                        }
                        ServerResponse::ActiveFrameTool { uframe, utool } => {
                            log::info!("Active frame/tool: UFrame={}, UTool={}", uframe, utool);
                            set_active_frame_tool.set(Some((uframe, utool)));
                        }
                        ServerResponse::FrameDataResponse { frame_number, data } => {
                            log::debug!("Frame {} data: ({:.3}, {:.3}, {:.3}, {:.2}, {:.2}, {:.2})",
                                frame_number, data.x, data.y, data.z, data.w, data.p, data.r);
                            set_frame_data.update(|map| {
                                map.insert(frame_number, FrameToolData {
                                    x: data.x, y: data.y, z: data.z,
                                    w: data.w, p: data.p, r: data.r,
                                });
                            });
                        }
                        ServerResponse::ToolDataResponse { tool_number, data } => {
                            log::debug!("Tool {} data: ({:.3}, {:.3}, {:.3}, {:.2}, {:.2}, {:.2})",
                                tool_number, data.x, data.y, data.z, data.w, data.p, data.r);
                            set_tool_data.update(|map| {
                                map.insert(tool_number, FrameToolData {
                                    x: data.x, y: data.y, z: data.z,
                                    w: data.w, p: data.p, r: data.r,
                                });
                            });
                        }
                        ServerResponse::DinValue { port_number, port_value } => {
                            log::debug!("DIN[{}] = {}", port_number, if port_value { "ON" } else { "OFF" });
                            set_din_values.update(|map| {
                                map.insert(port_number, port_value);
                            });
                        }
                        ServerResponse::DinBatch { values } => {
                            log::debug!("DIN batch: {} values", values.len());
                            set_din_values.update(|map| {
                                for (port, value) in values {
                                    map.insert(port, value);
                                }
                            });
                        }
                        ServerResponse::AinValue { port_number, port_value } => {
                            log::debug!("AIN[{}] = {:.3}", port_number, port_value);
                            set_ain_values.update(|map| {
                                map.insert(port_number, port_value);
                            });
                        }
                        ServerResponse::GinValue { port_number, port_value } => {
                            log::debug!("GIN[{}] = {}", port_number, port_value);
                            set_gin_values.update(|map| {
                                map.insert(port_number, port_value);
                            });
                        }
                        // Output values - broadcast from server after successful write
                        ServerResponse::DoutValue { port_number, port_value } => {
                            log::debug!("DOUT[{}] = {} (confirmed)", port_number, if port_value { "ON" } else { "OFF" });
                            set_dout_values.update(|map| {
                                map.insert(port_number, port_value);
                            });
                        }
                        ServerResponse::AoutValue { port_number, port_value } => {
                            log::debug!("AOUT[{}] = {:.3} (confirmed)", port_number, port_value);
                            set_aout_values.update(|map| {
                                map.insert(port_number, port_value);
                            });
                        }
                        ServerResponse::GoutValue { port_number, port_value } => {
                            log::debug!("GOUT[{}] = {} (confirmed)", port_number, port_value);
                            set_gout_values.update(|map| {
                                map.insert(port_number, port_value);
                            });
                        }
                        ServerResponse::IoConfig { configs } => {
                            log::debug!("Received I/O config: {} entries", configs.len());
                            set_io_config.update(|map| {
                                map.clear();
                                for cfg in configs {
                                    map.insert((cfg.io_type.clone(), cfg.io_index), cfg);
                                }
                            });
                        }
                        ServerResponse::ExecutionStateChanged { state, program_id, current_line, total_lines, message } => {
                            log::info!("Execution state changed: {} (program={:?}, line={:?}/{:?})", state, program_id, current_line, total_lines);
                            // Update loaded program ID if provided
                            set_loaded_program_id.set(program_id);
                            // Update execution status based on broadcast state
                            match state.as_str() {
                                "loaded" => {
                                    // Program is loaded but not running yet
                                    set_program_running.set(false);
                                    set_program_paused.set(false);
                                    if let (Some(line), Some(total)) = (current_line, total_lines) {
                                        set_program_progress.set(Some((line, total)));
                                    }
                                    set_executing_line.set(Some(0));
                                    if let Some(msg) = message {
                                        set_api_message.set(Some(msg));
                                    }
                                }
                                "running" => {
                                    set_program_running.set(true);
                                    set_program_paused.set(false);
                                    if let (Some(line), Some(total)) = (current_line, total_lines) {
                                        set_program_progress.set(Some((line, total)));
                                    }
                                }
                                "paused" => {
                                    set_program_running.set(true); // Still running, just paused
                                    set_program_paused.set(true);
                                    if let (Some(line), Some(total)) = (current_line, total_lines) {
                                        set_program_progress.set(Some((line, total)));
                                    }
                                    if let Some(msg) = message {
                                        set_api_message.set(Some(msg));
                                    }
                                }
                                "idle" | "completed" | "stopping" => {
                                    set_program_running.set(false);
                                    set_program_paused.set(false);
                                    set_program_progress.set(None);
                                    set_executing_line.set(None);
                                    if let Some(msg) = message {
                                        set_api_message.set(Some(msg));
                                    }
                                }
                                "error" => {
                                    set_program_running.set(false);
                                    set_program_paused.set(false);
                                    set_program_progress.set(None);
                                    set_executing_line.set(None);
                                    if let Some(msg) = message {
                                        set_api_error.set(Some(msg));
                                    }
                                }
                                _ => {
                                    log::warn!("Unknown execution state: {}", state);
                                }
                            }
                        }
                        // Control lock responses
                        ServerResponse::ControlAcquired => {
                            log::info!("Control acquired");
                            set_has_control.set(true);
                            set_api_message.set(Some("You now have control of the robot".to_string()));
                        }
                        ServerResponse::ControlReleased => {
                            log::info!("Control released");
                            set_has_control.set(false);
                            set_api_message.set(Some("Control released".to_string()));
                        }
                        ServerResponse::ControlDenied { holder_id, reason } => {
                            log::warn!("Control denied: {} (holder: {})", reason, holder_id);
                            set_has_control.set(false);
                            set_api_error.set(Some(format!("Control denied: {}", reason)));
                        }
                        ServerResponse::ControlLost { reason } => {
                            log::warn!("Control lost: {}", reason);
                            set_has_control.set(false);
                            set_api_message.set(Some(format!("Control lost: {}", reason)));
                        }
                        ServerResponse::ControlChanged { holder_id } => {
                            log::info!("Control changed: holder={:?}", holder_id);
                            // This is a broadcast to all clients - check if we still have control
                            // The holder_id is a UUID string, we don't have our own ID to compare
                            // So we rely on ControlAcquired/ControlLost for our own state
                        }
                        ServerResponse::ControlStatus { has_control, holder_id } => {
                            log::info!("Control status: has_control={}, holder={:?}", has_control, holder_id);
                            set_has_control.set(has_control);
                        }
                        ServerResponse::RobotDisconnected { reason } => {
                            log::warn!("Robot disconnected: {}", reason);
                            // Update connection state
                            set_robot_connected.set(false);
                            set_robot_connecting.set(false);
                            set_connected_robot_name.set(None);
                            set_active_connection_id.set(None);
                            // NOTE: Do NOT clear has_control here - the user should maintain
                            // control of the server even when the robot disconnects.
                            // This allows them to connect to a different robot without
                            // having to re-acquire control.
                            // Clear robot data
                            set_position.set(None);
                            set_status.set(None);
                            set_joint_angles.set(None);
                            // Show error toast
                            set_api_error.set(Some(format!("Robot disconnected: {}", reason)));
                        }
                        ServerResponse::RobotError { error_type, message, error_id, raw_data } => {
                            log::error!("Robot error ({}): {} (error_id: {:?})", error_type, message, error_id);
                            if let Some(ref raw) = raw_data {
                                log::error!("Raw data that failed to parse: {}", raw);
                            }
                            // Add to error log
                            set_error_log.update(|log| {
                                let error_msg = if let Some(ref raw) = raw_data {
                                    // Include raw data for protocol errors
                                    if let Some(id) = error_id {
                                        format!("[{}] {} (ErrorID: {}) | Raw: {}", error_type, message, id, raw)
                                    } else {
                                        format!("[{}] {} | Raw: {}", error_type, message, raw)
                                    }
                                } else if let Some(id) = error_id {
                                    format!("[{}] {} (ErrorID: {})", error_type, message, id)
                                } else {
                                    format!("[{}] {}", error_type, message)
                                };
                                log.push(error_msg);
                                if log.len() > 20 {
                                    log.remove(0);
                                }
                            });
                            // Also show as toast for critical errors
                            if error_type == "protocol" || error_type == "communication" {
                                let toast_msg = if let Some(ref raw) = raw_data {
                                    format!("Robot error: {} | Raw: {}", message, raw)
                                } else {
                                    format!("Robot error: {}", message)
                                };
                                set_api_error.set(Some(toast_msg));
                            }
                        }
                        ServerResponse::RobotCommandResult { command, success, error_id, message } => {
                            log::info!("Robot command result: {} success={} error_id={:?}", command, success, error_id);
                            // Add to motion log (command results are similar to motion feedback)
                            set_motion_log.update(|log| {
                                let msg = if success {
                                    format!("✓ {} completed", command)
                                } else if let Some(ref err_msg) = message {
                                    format!("✗ {} failed: {}", command, err_msg)
                                } else if let Some(id) = error_id {
                                    format!("✗ {} failed (ErrorID: {})", command, id)
                                } else {
                                    format!("✗ {} failed", command)
                                };
                                log.push(msg);
                                if log.len() > 20 {
                                    log.remove(0);
                                }
                            });
                            // Show toast for failures
                            if !success {
                                let err_msg = message.unwrap_or_else(|| {
                                    if let Some(id) = error_id {
                                        format!("{} failed (ErrorID: {})", command, id)
                                    } else {
                                        format!("{} failed", command)
                                    }
                                });
                                set_api_error.set(Some(err_msg));
                            } else {
                                // Show success message
                                set_api_message.set(Some(format!("{} completed", command)));
                            }
                        }
                        ServerResponse::ActiveConfigurationResponse {
                            loaded_from_id,
                            loaded_from_name,
                            changes_count,
                            change_log,
                            u_frame_number,
                            u_tool_number,
                            front,
                            up,
                            left,
                            flip,
                            turn4,
                            turn5,
                            turn6,
                            default_cartesian_jog_speed,
                            default_cartesian_jog_step,
                            default_joint_jog_speed,
                            default_joint_jog_step,
                            default_rotation_jog_speed,
                            default_rotation_jog_step,
                        } => {
                            log::info!("Received active configuration: {:?}", loaded_from_name);
                            set_active_configuration.set(Some(ActiveConfigurationData {
                                loaded_from_id,
                                loaded_from_name,
                                changes_count,
                                change_log,
                                u_frame_number,
                                u_tool_number,
                                front,
                                up,
                                left,
                                flip,
                                turn4,
                                turn5,
                                turn6,
                                default_cartesian_jog_speed,
                                default_cartesian_jog_step,
                                default_joint_jog_speed,
                                default_joint_jog_step,
                                default_rotation_jog_speed,
                                default_rotation_jog_step,
                            }));
                        }
                        ServerResponse::ActiveJogSettings {
                            cartesian_jog_speed,
                            cartesian_jog_step,
                            joint_jog_speed,
                            joint_jog_step,
                            rotation_jog_speed,
                            rotation_jog_step,
                        } => {
                            log::info!("Received active jog settings: cart_speed={}, cart_step={}, joint_speed={}, joint_step={}, rot_speed={}, rot_step={}",
                                cartesian_jog_speed, cartesian_jog_step, joint_jog_speed, joint_jog_step, rotation_jog_speed, rotation_jog_step);
                            set_active_jog_settings.set(Some(ActiveJogSettingsData {
                                cartesian_jog_speed,
                                cartesian_jog_step,
                                joint_jog_speed,
                                joint_jog_step,
                                rotation_jog_speed,
                                rotation_jog_step,
                            }));
                        }
                        ServerResponse::RobotConfigurationList { configurations } => {
                            log::info!("Received {} robot configurations", configurations.len());
                            set_robot_configurations.set(configurations);
                        }
                        ServerResponse::RobotConfigurationResponse { configuration } => {
                            log::info!("Received robot configuration: {}", configuration.name);
                            // Update the configuration in the list if it exists
                            set_robot_configurations.update(|configs| {
                                if let Some(pos) = configs.iter().position(|c| c.id == configuration.id) {
                                    configs[pos] = configuration.clone();
                                } else {
                                    configs.push(configuration.clone());
                                }
                            });
                        }
                    }
                } else {
                    log::error!("Failed to parse API response: {}", text_str);
                    // Add parse error to console with raw JSON for debugging
                    add_console_msg(
                        set_console_messages,
                        MessageDirection::Received,
                        MessageType::Error,
                        format!("Failed to parse JSON response. Raw data: {}", text_str),
                        None,
                    );
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // On error
        let set_api_error_err = self.set_api_error;
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            log::error!("WebSocket error: {:?}", e);
            set_api_error_err.set(Some("WebSocket connection error".to_string()));
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        // On close - critical for detecting disconnects
        let set_connected_close = self.set_connected;
        let set_robot_connected_close = self.set_robot_connected;
        let set_has_control_close = self.set_has_control;
        let set_api_error_close = self.set_api_error;
        let set_position_close = self.set_position;
        let set_status_close = self.set_status;
        let set_joint_angles_close = self.set_joint_angles;
        let onclose_callback = Closure::wrap(Box::new(move |e: web_sys::CloseEvent| {
            log::warn!("WebSocket closed: code={}, reason={}", e.code(), e.reason());

            // Immediately update connection state
            set_connected_close.set(false);
            set_robot_connected_close.set(false);
            set_has_control_close.set(false);

            // Clear robot data
            set_position_close.set(None);
            set_status_close.set(None);
            set_joint_angles_close.set(None);

            // Show error toast based on close code
            let error_msg = if e.code() == 1000 {
                "WebSocket connection closed normally".to_string()
            } else if e.code() == 1006 {
                "WebSocket connection lost - server may be down".to_string()
            } else {
                format!("WebSocket disconnected: {} (code {})", e.reason(), e.code())
            };
            set_api_error_close.set(Some(error_msg));
        }) as Box<dyn FnMut(web_sys::CloseEvent)>);
        ws.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        onclose_callback.forget();

        self.ws.set_value(Some(ws));

        // Request initial state after connection is established
        // Use a small delay to ensure the WebSocket is fully ready
        let ws_ref = self.ws.get_value();
        if let Some(ws) = ws_ref {
            // Request robot connection status
            if let Ok(json) = serde_json::to_string(&ClientRequest::GetConnectionStatus) {
                let _ = ws.send_with_str(&json);
            }
            // Request control status
            if let Ok(json) = serde_json::to_string(&ClientRequest::GetControlStatus) {
                let _ = ws.send_with_str(&json);
            }
            // Request execution state (what program is loaded and running)
            if let Ok(json) = serde_json::to_string(&ClientRequest::GetExecutionState) {
                let _ = ws.send_with_str(&json);
            }
            // Request active configuration (if robot is connected)
            if let Ok(json) = serde_json::to_string(&ClientRequest::GetActiveConfiguration) {
                let _ = ws.send_with_str(&json);
            }
        }
    }

    /// Send a robot protocol command (binary/bincode)
    pub fn send_command(&self, packet: SendPacket) {
        // Log the command to console (skip polling commands to avoid flooding)
        let (name, seq_id) = Self::get_packet_info(&packet);
        if !Self::is_polling_command(&name) {
            self.log_command_sent(&name, seq_id);
        }

        if let Some(ws) = self.ws.get_value() {
            if let Ok(binary) = bincode::serialize(&packet) {
                let _ = ws.send_with_u8_array(&binary);
            }
        }
    }

    /// Check if a command is a polling command (should not be logged)
    fn is_polling_command(name: &str) -> bool {
        matches!(
            name,
            "FRC_GetStatus"
                | "FRC_ReadCartesianPosition"
                | "FRC_ReadJointAngles"
                | "FRC_ReadDIN"
                | "FRC_ReadAIN"
                | "FRC_ReadGIN"
        )
    }

    /// Extract command name and sequence ID from a SendPacket
    fn get_packet_info(packet: &SendPacket) -> (String, Option<u32>) {
        match packet {
            SendPacket::Communication(comm) => {
                let name = match comm {
                    Communication::FrcConnect => "FRC_Connect",
                    Communication::FrcDisconnect => "FRC_Disconnect",
                    Communication::FrcTerminate => "FRC_Terminate",
                    Communication::FrcSystemFault => "FRC_SystemFault",
                };
                (name.to_string(), None)
            }
            SendPacket::Command(cmd) => {
                let name = match cmd {
                    Command::FrcInitialize(_) => "FRC_Initialize",
                    Command::FrcAbort => "FRC_Abort",
                    Command::FrcPause => "FRC_Pause",
                    Command::FrcReadError(_) => "FRC_ReadError",
                    Command::FrcContinue => "FRC_Continue",
                    Command::FrcSetUFrameUTool(_) => "FRC_SetUFrameUTool",
                    Command::FrcReadPositionRegister(_) => "FRC_ReadPositionRegister",
                    Command::FrcWritePositionRegister(_) => "FRC_WritePositionRegister",
                    Command::FrcGetUFrameUTool(_) => "FRC_GetUFrameUTool",
                    Command::FrcGetStatus => "FRC_GetStatus",
                    Command::FrcReadUFrameData(_) => "FRC_ReadUFrameData",
                    Command::FrcReadUToolData(_) => "FRC_ReadUToolData",
                    Command::FrcWriteUFrameData(_) => "FRC_WriteUFrameData",
                    Command::FrcWriteUToolData(_) => "FRC_WriteUToolData",
                    Command::FrcSetOverRide(_) => "FRC_SetOverRide",
                    Command::FrcReset => "FRC_Reset",
                    Command::FrcReadDIN(_) => "FRC_ReadDIN",
                    Command::FrcWriteDOUT(_) => "FRC_WriteDOUT",
                    Command::FrcReadAIN(_) => "FRC_ReadAIN",
                    Command::FrcWriteAOUT(_) => "FRC_WriteAOUT",
                    Command::FrcReadGIN(_) => "FRC_ReadGIN",
                    Command::FrcWriteGOUT(_) => "FRC_WriteGOUT",
                    Command::FrcReadCartesianPosition(_) => "FRC_ReadCartesianPosition",
                    Command::FrcReadJointAngles(_) => "FRC_ReadJointAngles",
                    Command::FrcReadTCPSpeed => "FRC_ReadTCPSpeed",
                };
                (name.to_string(), None)
            }
            SendPacket::Instruction(instr) => {
                let (name, seq_id) = match instr {
                    Instruction::FrcWaitDIN(i) => ("FRC_WaitDIN", i.sequence_id),
                    Instruction::FrcSetUFrame(i) => ("FRC_SetUFrame", i.sequence_id),
                    Instruction::FrcSetUTool(i) => ("FRC_SetUTool", i.sequence_id),
                    Instruction::FrcWaitTime(i) => ("FRC_WaitTime", i.sequence_id),
                    Instruction::FrcSetPayLoad(i) => ("FRC_SetPayLoad", i.sequence_id),
                    Instruction::FrcCall(i) => ("FRC_Call", i.sequence_id),
                    Instruction::FrcLinearMotion(i) => ("FRC_LinearMotion", i.sequence_id),
                    Instruction::FrcLinearRelative(i) => ("FRC_LinearRelative", i.sequence_id),
                    Instruction::FrcLinearRelativeJRep(i) => ("FRC_LinearRelativeJRep", i.sequence_id),
                    Instruction::FrcJointMotion(i) => ("FRC_JointMotion", i.sequence_id),
                    Instruction::FrcJointRelative(i) => ("FRC_JointRelative", i.sequence_id),
                    Instruction::FrcCircularMotion(i) => ("FRC_CircularMotion", i.sequence_id),
                    Instruction::FrcCircularRelative(i) => ("FRC_CircularRelative", i.sequence_id),
                    Instruction::FrcJointMotionJRep(i) => ("FRC_JointMotionJRep", i.sequence_id),
                    Instruction::FrcJointRelativeJRep(i) => ("FRC_JointRelativeJRep", i.sequence_id),
                    Instruction::FrcLinearMotionJRep(i) => ("FRC_LinearMotionJRep", i.sequence_id),
                };
                (name.to_string(), Some(seq_id))
            }
            SendPacket::DriverCommand(_) => ("DriverCommand".to_string(), None),
        }
    }

    /// Send an API request (JSON text)
    pub fn send_api_request(&self, request: ClientRequest) {
        if let Some(ws) = self.ws.get_value() {
            if let Ok(json) = serde_json::to_string(&request) {
                let _ = ws.send_with_str(&json);
            }
        }
    }

    // ========== API Request Helpers ==========

    /// Request list of all programs
    pub fn list_programs(&self) {
        self.send_api_request(ClientRequest::ListPrograms);
    }

    /// Request a specific program by ID
    pub fn get_program(&self, id: i64) {
        self.send_api_request(ClientRequest::GetProgram { id });
    }

    /// Create a new program
    pub fn create_program(&self, name: String, description: Option<String>) {
        self.send_api_request(ClientRequest::CreateProgram { name, description });
    }

    /// Delete a program
    pub fn delete_program(&self, id: i64) {
        self.send_api_request(ClientRequest::DeleteProgram { id });
    }

    /// Update program settings (start/end positions with orientation, move speed, termination defaults).
    #[allow(clippy::too_many_arguments)]
    pub fn update_program_settings(
        &self,
        program_id: i64,
        start_x: Option<f64>,
        start_y: Option<f64>,
        start_z: Option<f64>,
        start_w: Option<f64>,
        start_p: Option<f64>,
        start_r: Option<f64>,
        end_x: Option<f64>,
        end_y: Option<f64>,
        end_z: Option<f64>,
        end_w: Option<f64>,
        end_p: Option<f64>,
        end_r: Option<f64>,
        move_speed: Option<f64>,
        default_term_type: Option<String>,
        default_term_value: Option<u8>,
    ) {
        self.send_api_request(ClientRequest::UpdateProgramSettings {
            program_id,
            start_x,
            start_y,
            start_z,
            start_w,
            start_p,
            start_r,
            end_x,
            end_y,
            end_z,
            end_w,
            end_p,
            end_r,
            move_speed,
            default_term_type,
            default_term_value,
        });
    }

    /// Upload CSV content to a program.
    ///
    /// CSV contains generic waypoints. Robot-specific configuration is applied
    /// at execution time, not at upload time.
    pub fn upload_csv(&self, program_id: i64, csv_content: String, start_position: Option<StartPosition>) {
        self.send_api_request(ClientRequest::UploadCsv {
            program_id,
            csv_content,
            start_position,
        });
    }

    /// Load a program into the executor (without starting execution)
    pub fn load_program(&self, program_id: i64) {
        self.send_api_request(ClientRequest::LoadProgram { program_id });
    }

    /// Unload the current program from the executor
    pub fn unload_program(&self) {
        self.send_api_request(ClientRequest::UnloadProgram);
    }

    /// Start program execution (loads and starts)
    pub fn start_program(&self, program_id: i64) {
        self.send_api_request(ClientRequest::StartProgram { program_id });
    }

    /// Pause program execution
    pub fn pause_program(&self) {
        self.send_api_request(ClientRequest::PauseProgram);
    }

    /// Resume program execution
    pub fn resume_program(&self) {
        self.send_api_request(ClientRequest::ResumeProgram);
    }

    /// Stop program execution
    pub fn stop_program(&self) {
        self.send_api_request(ClientRequest::StopProgram);
    }

    /// Get current execution state (for reconnection/sync)
    pub fn get_execution_state(&self) {
        self.send_api_request(ClientRequest::GetExecutionState);
    }

    /// Get robot settings
    pub fn get_settings(&self) {
        self.send_api_request(ClientRequest::GetSettings);
    }

    /// Update robot settings
    pub fn update_settings(
        &self,
        default_w: f64,
        default_p: f64,
        default_r: f64,
        default_speed: f64,
        default_term_type: String,
        default_uframe: i32,
        default_utool: i32,
    ) {
        self.send_api_request(ClientRequest::UpdateSettings {
            default_w,
            default_p,
            default_r,
            default_speed,
            default_term_type,
            default_uframe,
            default_utool,
        });
    }

    /// Reset database (dangerous!) - deletes all programs, settings, and connections
    pub fn reset_database(&self) {
        self.send_api_request(ClientRequest::ResetDatabase);
    }

    /// Reconnect WebSocket to a new URL
    pub fn reconnect(&self, new_url: &str) {
        // Close existing connection
        if let Some(ws) = self.ws.get_value() {
            let _ = ws.close();
        }

        // Update URL
        self.ws_url.set_value(new_url.to_string());

        // Set disconnected and connecting
        self.set_connected.set(false);
        self.set_ws_connecting.set(true);

        // Clear data
        self.set_position.set(None);
        self.set_status.set(None);
        self.set_programs.set(Vec::new());
        self.set_current_program.set(None);
        self.set_settings.set(None);

        // Reconnect
        self.connect();
    }

    /// Get robot connection status from server
    pub fn get_connection_status(&self) {
        self.send_api_request(ClientRequest::GetConnectionStatus);
    }

    /// Connect to robot at specified address
    pub fn connect_robot(&self, robot_addr: &str, robot_port: u32) {
        self.set_robot_connecting.set(true);
        self.send_api_request(ClientRequest::ConnectRobot {
            robot_addr: robot_addr.to_string(),
            robot_port,
        });
    }

    /// Connect to a saved robot connection by ID.
    /// This will load the per-robot defaults and apply them.
    pub fn connect_to_saved_robot(&self, connection_id: i64) {
        self.set_robot_connecting.set(true);
        self.send_api_request(ClientRequest::ConnectToSavedRobot { connection_id });
    }

    /// Disconnect from robot
    pub fn disconnect_robot(&self) {
        self.send_api_request(ClientRequest::DisconnectRobot);
    }

    /// Clear the motion log
    pub fn clear_motion_log(&self) {
        self.set_motion_log.set(Vec::new());
    }

    /// Clear the error log
    pub fn clear_error_log(&self) {
        self.set_error_log.set(Vec::new());
    }

    /// Add a console message
    pub fn add_console_message(&self, msg: ConsoleMessage) {
        self.set_console_messages.update(|msgs| {
            msgs.push(msg);
            // Keep only last 100 messages
            if msgs.len() > 100 {
                msgs.remove(0);
            }
        });
    }

    /// Clear all console messages
    pub fn clear_console_messages(&self) {
        self.set_console_messages.set(Vec::new());
    }

    /// Helper to get current timestamp string and ms
    fn get_timestamp() -> (String, u64) {
        let now = js_sys::Date::new_0();
        let hours = now.get_hours();
        let minutes = now.get_minutes();
        let seconds = now.get_seconds();
        let millis = now.get_milliseconds();
        let timestamp = format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis);
        let timestamp_ms = now.get_time() as u64;
        (timestamp, timestamp_ms)
    }

    /// Log a sent command to console
    pub fn log_command_sent(&self, command_name: &str, sequence_id: Option<u32>) {
        let (timestamp, timestamp_ms) = Self::get_timestamp();
        self.add_console_message(ConsoleMessage {
            timestamp,
            timestamp_ms,
            direction: MessageDirection::Sent,
            msg_type: MessageType::Command,
            content: command_name.to_string(),
            sequence_id,
        });
    }

    /// Log a received response to console
    pub fn log_response_received(&self, response_name: &str, sequence_id: Option<u32>, is_error: bool) {
        let (timestamp, timestamp_ms) = Self::get_timestamp();
        self.add_console_message(ConsoleMessage {
            timestamp,
            timestamp_ms,
            direction: MessageDirection::Received,
            msg_type: if is_error { MessageType::Error } else { MessageType::Response },
            content: response_name.to_string(),
            sequence_id,
        });
    }

    /// Log a system/status message to console
    pub fn log_system_message(&self, message: &str) {
        let (timestamp, timestamp_ms) = Self::get_timestamp();
        self.add_console_message(ConsoleMessage {
            timestamp,
            timestamp_ms,
            direction: MessageDirection::System,
            msg_type: MessageType::Status,
            content: message.to_string(),
            sequence_id: None,
        });
    }

    /// Clear the API error
    pub fn clear_api_error(&self) {
        self.set_api_error.set(None);
    }

    /// Clear the API message (toast notification)
    pub fn clear_api_message(&self) {
        self.set_api_message.set(None);
    }

    /// Set an API message (toast notification)
    pub fn set_message(&self, message: String) {
        self.set_api_message.set(Some(message));
    }

    /// Clear the current program (close it)
    pub fn clear_current_program(&self) {
        self.set_current_program.set(None);
    }

    // ========== Robot Control Commands ==========

    /// Abort current motion and clear motion queue
    pub fn robot_abort(&self) {
        self.send_api_request(ClientRequest::RobotAbort);
    }

    /// Reset robot controller (clears errors)
    pub fn robot_reset(&self) {
        self.send_api_request(ClientRequest::RobotReset);
    }

    /// Initialize robot controller
    pub fn robot_initialize(&self, group_mask: Option<u8>) {
        self.send_api_request(ClientRequest::RobotInitialize { group_mask });
    }

    // ========== Robot Connections (Saved Connections) ==========

    /// List all saved robot connections
    pub fn list_robot_connections(&self) {
        self.send_api_request(ClientRequest::ListRobotConnections);
    }

    /// Get a specific robot connection by ID (reserved for connection details view)
    #[allow(dead_code)]
    pub fn get_robot_connection(&self, id: i64) {
        self.send_api_request(ClientRequest::GetRobotConnection { id });
    }

    /// Create a new saved robot connection (DEPRECATED - use create_robot_with_configurations)
    pub fn create_robot_connection(&self, name: String, description: Option<String>, ip_address: String, port: u32) {
        self.send_api_request(ClientRequest::CreateRobotConnection {
            name,
            description,
            ip_address,
            port,
        });
    }

    /// Create a new robot connection with configurations atomically
    #[allow(clippy::too_many_arguments)]
    pub fn create_robot_with_configurations(
        &self,
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
        default_rotation_jog_speed: f64,
        default_rotation_jog_step: f64,
        configurations: Vec<NewRobotConfigurationDto>,
    ) {
        self.send_api_request(ClientRequest::CreateRobotWithConfigurations {
            name,
            description,
            ip_address,
            port,
            default_speed,
            default_speed_type,
            default_term_type,
            default_w,
            default_p,
            default_r,
            default_cartesian_jog_speed,
            default_cartesian_jog_step,
            default_joint_jog_speed,
            default_joint_jog_step,
            default_rotation_jog_speed,
            default_rotation_jog_step,
            configurations,
        });
    }

    /// Update an existing robot connection
    pub fn update_robot_connection(&self, id: i64, name: String, description: Option<String>, ip_address: String, port: u32) {
        self.send_api_request(ClientRequest::UpdateRobotConnection {
            id,
            name,
            description,
            ip_address,
            port,
        });
    }

    /// Update robot connection motion defaults.
    /// Motion parameters (speed, speed_type, term_type, w/p/r) only.
    /// Frame/tool/arm config is managed via robot_configurations table.
    pub fn update_robot_connection_defaults(
        &self,
        id: i64,
        default_speed: f64,
        default_speed_type: String,
        default_term_type: String,
        default_w: f64,
        default_p: f64,
        default_r: f64,
    ) {
        self.send_api_request(ClientRequest::UpdateRobotConnectionDefaults {
            id,
            default_speed,
            default_speed_type,
            default_term_type,
            default_w,
            default_p,
            default_r,
        });
    }

    /// Delete a saved robot connection
    pub fn delete_robot_connection(&self, id: i64) {
        self.send_api_request(ClientRequest::DeleteRobotConnection { id });
    }

    /// Update robot connection jog defaults (saves to database)
    pub fn update_robot_jog_defaults(
        &self,
        id: i64,
        cartesian_jog_speed: f64,
        cartesian_jog_step: f64,
        joint_jog_speed: f64,
        joint_jog_step: f64,
        rotation_jog_speed: f64,
        rotation_jog_step: f64,
    ) {
        self.send_api_request(ClientRequest::UpdateRobotJogDefaults {
            id,
            cartesian_jog_speed,
            cartesian_jog_step,
            joint_jog_speed,
            joint_jog_step,
            rotation_jog_speed,
            rotation_jog_step,
        });
    }

    /// Update jog controls (from Control panel - updates active jog controls only, does NOT update defaults or increment changes_count)
    pub fn update_jog_controls(
        &self,
        cartesian_jog_speed: f64,
        cartesian_jog_step: f64,
        joint_jog_speed: f64,
        joint_jog_step: f64,
        rotation_jog_speed: f64,
        rotation_jog_step: f64,
    ) {
        self.send_api_request(ClientRequest::UpdateJogControls {
            cartesian_jog_speed,
            cartesian_jog_step,
            joint_jog_speed,
            joint_jog_step,
            rotation_jog_speed,
            rotation_jog_step,
        });
    }

    /// Apply jog defaults (from Configuration panel - updates active defaults AND active jog controls, increments changes_count, does NOT save to database)
    pub fn apply_jog_settings(
        &self,
        cartesian_jog_speed: f64,
        cartesian_jog_step: f64,
        joint_jog_speed: f64,
        joint_jog_step: f64,
        rotation_jog_speed: f64,
        rotation_jog_step: f64,
    ) {
        self.send_api_request(ClientRequest::ApplyJogSettings {
            cartesian_jog_speed,
            cartesian_jog_step,
            joint_jog_speed,
            joint_jog_step,
            rotation_jog_speed,
            rotation_jog_step,
        });
    }

    /// Save current configuration (active frame/tool/arm config + active jog settings) to database.
    /// If configuration_name is provided, creates a new configuration.
    /// Otherwise, updates the currently loaded configuration.
    pub fn save_current_configuration(&self, configuration_name: Option<String>) {
        self.send_api_request(ClientRequest::SaveCurrentConfiguration {
            configuration_name,
        });
    }

    /// Set the active/selected connection ID
    pub fn set_active_connection(&self, id: Option<i64>) {
        self.set_active_connection_id.set(id);
    }

    // ========== Frame/Tool Management ==========

    /// Get the currently active UFrame and UTool numbers
    pub fn get_active_frame_tool(&self) {
        self.send_api_request(ClientRequest::GetActiveFrameTool);
    }

    /// Set the active UFrame and UTool numbers
    pub fn set_active_frame_tool(&self, uframe: u8, utool: u8) {
        self.send_api_request(ClientRequest::SetActiveFrameTool { uframe, utool });
    }

    /// Read UFrame data for a specific frame number
    pub fn read_frame_data(&self, frame_number: u8) {
        self.send_api_request(ClientRequest::ReadFrameData { frame_number });
    }

    /// Read UTool data for a specific tool number
    pub fn read_tool_data(&self, tool_number: u8) {
        self.send_api_request(ClientRequest::ReadToolData { tool_number });
    }

    /// Write UFrame data for a specific frame number
    /// Note: Currently unused but exposed as public API for future frame editing UI
    #[allow(dead_code)]
    pub fn write_frame_data(&self, frame_number: u8, data: fanuc_rmi::dto::FrameData) {
        self.send_api_request(ClientRequest::WriteFrameData {
            frame_number,
            data,
        });
    }

    /// Write UTool data for a specific tool number
    /// Note: Currently unused but exposed as public API for future tool editing UI
    #[allow(dead_code)]
    pub fn write_tool_data(&self, tool_number: u8, data: fanuc_rmi::dto::FrameData) {
        self.send_api_request(ClientRequest::WriteToolData {
            tool_number,
            data,
        });
    }

    // ========== I/O Management ==========

    /// Read a single digital input port
    pub fn read_din(&self, port_number: u16) {
        self.send_api_request(ClientRequest::ReadDin { port_number });
    }

    /// Write a digital output port
    pub fn write_dout(&self, port_number: u16, port_value: bool) {
        self.send_api_request(ClientRequest::WriteDout { port_number, port_value });
    }

    /// Read multiple digital input ports at once
    pub fn read_din_batch(&self, port_numbers: Vec<u16>) {
        self.send_api_request(ClientRequest::ReadDinBatch { port_numbers });
    }

    /// Clear the cached I/O values
    pub fn clear_io_cache(&self) {
        self.set_din_values.set(std::collections::HashMap::new());
        self.set_dout_values.set(std::collections::HashMap::new());
        self.set_ain_values.set(std::collections::HashMap::new());
        self.set_aout_values.set(std::collections::HashMap::new());
        self.set_gin_values.set(std::collections::HashMap::new());
        self.set_gout_values.set(std::collections::HashMap::new());
    }

    /// Update a single DOUT value in the local cache (for optimistic updates)
    pub fn update_dout_cache(&self, port: u16, value: bool) {
        self.set_dout_values.update(|map| {
            map.insert(port, value);
        });
    }

    // ========== Analog I/O ==========

    /// Read a single analog input port
    pub fn read_ain(&self, port_number: u16) {
        self.send_api_request(ClientRequest::ReadAin { port_number });
    }

    /// Write an analog output port
    pub fn write_aout(&self, port_number: u16, port_value: f64) {
        self.send_api_request(ClientRequest::WriteAout { port_number, port_value });
    }

    /// Update a single AOUT value in the local cache (for optimistic updates)
    pub fn update_aout_cache(&self, port: u16, value: f64) {
        self.set_aout_values.update(|map| {
            map.insert(port, value);
        });
    }

    // ========== Group I/O ==========

    /// Read a single group input port
    pub fn read_gin(&self, port_number: u16) {
        self.send_api_request(ClientRequest::ReadGin { port_number });
    }

    /// Write a group output port
    pub fn write_gout(&self, port_number: u16, port_value: u32) {
        self.send_api_request(ClientRequest::WriteGout { port_number, port_value });
    }

    /// Update a single GOUT value in the local cache (for optimistic updates)
    pub fn update_gout_cache(&self, port: u16, value: u32) {
        self.set_gout_values.update(|map| {
            map.insert(port, value);
        });
    }

    // ========== I/O Configuration ==========

    /// Get I/O display configuration for a robot
    pub fn get_io_config(&self, robot_connection_id: i64) {
        self.send_api_request(ClientRequest::GetIoConfig { robot_connection_id });
    }

    /// Update I/O display configuration
    pub fn update_io_config(
        &self,
        robot_connection_id: i64,
        io_type: String,
        io_index: i32,
        display_name: Option<String>,
        is_visible: bool,
        display_order: Option<i32>,
    ) {
        self.send_api_request(ClientRequest::UpdateIoConfig {
            robot_connection_id,
            io_type,
            io_index,
            display_name,
            is_visible,
            display_order,
        });
    }

    // ========== Control Lock ==========

    /// Request control of the robot
    pub fn request_control(&self) {
        self.send_api_request(ClientRequest::RequestControl);
    }

    /// Release control of the robot
    pub fn release_control(&self) {
        self.send_api_request(ClientRequest::ReleaseControl);
    }

    /// Get current control status
    pub fn get_control_status(&self) {
        self.send_api_request(ClientRequest::GetControlStatus);
    }

    /// Get the currently active robot connection (if any)
    pub fn get_active_connection(&self) -> Option<RobotConnectionDto> {
        let active_id = self.active_connection_id.get_untracked();
        let connections = self.robot_connections.get_untracked();
        active_id.and_then(|id| connections.into_iter().find(|c| c.id == id))
    }

    // ========== Robot Configurations ==========

    /// List all configurations for a robot
    pub fn list_robot_configurations(&self, robot_connection_id: i64) {
        self.send_api_request(ClientRequest::ListRobotConfigurations { robot_connection_id });
    }

    /// Get the current active configuration
    pub fn get_active_configuration(&self) {
        self.send_api_request(ClientRequest::GetActiveConfiguration);
    }

    /// Load a saved configuration as active
    pub fn load_configuration(&self, configuration_id: i64) {
        self.send_api_request(ClientRequest::LoadConfiguration { configuration_id });
    }

    /// Create a new robot configuration
    #[allow(clippy::too_many_arguments)]
    pub fn create_robot_configuration(
        &self,
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
    ) {
        self.send_api_request(ClientRequest::CreateRobotConfiguration {
            robot_connection_id,
            name,
            is_default,
            u_frame_number,
            u_tool_number,
            front,
            up,
            left,
            flip,
            turn4,
            turn5,
            turn6,
        });
    }

    /// Update an existing robot configuration
    #[allow(clippy::too_many_arguments)]
    pub fn update_robot_configuration(
        &self,
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
    ) {
        self.send_api_request(ClientRequest::UpdateRobotConfiguration {
            id,
            name,
            is_default,
            u_frame_number,
            u_tool_number,
            front,
            up,
            left,
            flip,
            turn4,
            turn5,
            turn6,
        });
    }

    /// Delete a robot configuration
    pub fn delete_robot_configuration(&self, id: i64) {
        self.send_api_request(ClientRequest::DeleteRobotConfiguration { id });
    }

    /// Set a configuration as the default for its robot
    pub fn set_default_robot_configuration(&self, id: i64) {
        self.send_api_request(ClientRequest::SetDefaultRobotConfiguration { id });
    }
}

fn get_response_ids(resp: &InstructionResponse) -> (u32, u32) {
    match resp {
        InstructionResponse::FrcLinearRelative(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcLinearRelativeJRep(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcJointMotion(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcJointRelative(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcJointRelativeJRep(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcLinearMotion(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcCircularMotion(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcWaitTime(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcSetUFrame(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcSetUTool(r) => (r.sequence_id, r.error_id),
        _ => (0, 0),
    }
}

/// Get instruction response name for console logging
fn get_instruction_response_name(resp: &InstructionResponse) -> &'static str {
    match resp {
        InstructionResponse::FrcWaitDIN(_) => "FRC_WaitDIN",
        InstructionResponse::FrcSetUFrame(_) => "FRC_SetUFrame",
        InstructionResponse::FrcSetUTool(_) => "FRC_SetUTool",
        InstructionResponse::FrcWaitTime(_) => "FRC_WaitTime",
        InstructionResponse::FrcSetPayLoad(_) => "FRC_SetPayLoad",
        InstructionResponse::FrcCall(_) => "FRC_Call",
        InstructionResponse::FrcLinearMotion(_) => "FRC_LinearMotion",
        InstructionResponse::FrcLinearRelative(_) => "FRC_LinearRelative",
        InstructionResponse::FrcLinearRelativeJRep(_) => "FRC_LinearRelativeJRep",
        InstructionResponse::FrcJointMotion(_) => "FRC_JointMotion",
        InstructionResponse::FrcJointRelative(_) => "FRC_JointRelative",
        InstructionResponse::FrcCircularMotion(_) => "FRC_CircularMotion",
        InstructionResponse::FrcCircularRelative(_) => "FRC_CircularRelative",
        InstructionResponse::FrcJointMotionJRep(_) => "FRC_JointMotionJRep",
        InstructionResponse::FrcJointRelativeJRep(_) => "FRC_JointRelativeJRep",
        InstructionResponse::FrcLinearMotionJRep(_) => "FRC_LinearMotionJRep",
    }
}

/// Add a console message to the signal
fn add_console_msg(
    set_console_messages: WriteSignal<Vec<ConsoleMessage>>,
    direction: MessageDirection,
    msg_type: MessageType,
    content: String,
    sequence_id: Option<u32>,
) {
    let now = js_sys::Date::new_0();
    let hours = now.get_hours();
    let minutes = now.get_minutes();
    let seconds = now.get_seconds();
    let millis = now.get_milliseconds();
    let timestamp = format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis);
    let timestamp_ms = now.get_time() as u64;

    set_console_messages.update(|msgs| {
        msgs.push(ConsoleMessage {
            timestamp,
            timestamp_ms,
            direction,
            msg_type,
            content,
            sequence_id,
        });
        // Keep only last 100 messages
        if msgs.len() > 100 {
            msgs.remove(0);
        }
    });
}

fn format_instruction_response(resp: &InstructionResponse, seq_id: u32) -> String {
    match resp {
        InstructionResponse::FrcLinearRelative(_) => {
            format!("✓ Linear move completed (Seq:{})", seq_id)
        }
        InstructionResponse::FrcJointMotion(_) => {
            format!("✓ Joint move completed (Seq:{})", seq_id)
        }
        InstructionResponse::FrcWaitTime(_) => {
            format!("✓ Wait completed (Seq:{})", seq_id)
        }
        _ => format!("✓ Motion completed (Seq:{})", seq_id),
    }
}
