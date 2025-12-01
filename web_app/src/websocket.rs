use fanuc_rmi::dto::*;
use leptos::prelude::*;
use leptos::reactive::owner::LocalStorage;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{BinaryType, ErrorEvent, MessageEvent, WebSocket};

// ========== API Types (matching web_server/src/api_types.rs) ==========

/// Client requests to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientRequest {
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

    #[serde(rename = "start_program")]
    StartProgram { program_id: i64 },

    #[serde(rename = "pause_program")]
    PauseProgram,

    #[serde(rename = "resume_program")]
    ResumeProgram,

    #[serde(rename = "stop_program")]
    StopProgram,

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

    #[serde(rename = "delete_robot_connection")]
    DeleteRobotConnection { id: i64 },
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotConnectionDto {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub ip_address: String,
    pub port: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInfo {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub instruction_count: i64,
    pub created_at: String,
    pub updated_at: String,
}

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

// ========== WebSocket Manager ==========

#[derive(Clone, Copy)]
pub struct WebSocketManager {
    pub connected: ReadSignal<bool>,
    set_connected: WriteSignal<bool>,
    pub position: ReadSignal<Option<(f64, f64, f64)>>,
    set_position: WriteSignal<Option<(f64, f64, f64)>>,
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
    pub api_message: ReadSignal<Option<String>>,
    set_api_message: WriteSignal<Option<String>>,
    /// API error message (cleared on next successful response)
    pub api_error: ReadSignal<Option<String>>,
    set_api_error: WriteSignal<Option<String>>,
    pub execution_status: ReadSignal<Option<ExecutionStatusData>>,
    set_execution_status: WriteSignal<Option<ExecutionStatusData>>,
    // Program execution state
    pub program_running: ReadSignal<bool>,
    set_program_running: WriteSignal<bool>,
    pub program_progress: ReadSignal<Option<(usize, usize)>>, // (completed_line, total_lines)
    set_program_progress: WriteSignal<Option<(usize, usize)>>,
    pub executing_line: ReadSignal<Option<usize>>,  // The line currently being executed
    set_executing_line: WriteSignal<Option<usize>>,
    // Robot connection status
    pub robot_connected: ReadSignal<bool>,
    set_robot_connected: WriteSignal<bool>,
    pub robot_addr: ReadSignal<String>,
    set_robot_addr: WriteSignal<String>,
    // Saved robot connections
    pub robot_connections: ReadSignal<Vec<RobotConnectionDto>>,
    set_robot_connections: WriteSignal<Vec<RobotConnectionDto>>,
    // Currently active/selected connection
    pub active_connection_id: ReadSignal<Option<i64>>,
    set_active_connection_id: WriteSignal<Option<i64>>,
    ws: StoredValue<Option<WebSocket>, LocalStorage>,
    ws_url: StoredValue<String>,
}

#[derive(Clone, Debug)]
pub struct RobotStatusData {
    pub servo_ready: i8,
    pub tp_mode: i8,
    pub motion_status: i8,
}

#[derive(Clone, Debug)]
pub struct ExecutionStatusData {
    pub status: String,
    pub current_line: Option<usize>,
    pub total_lines: Option<usize>,
    pub error: Option<String>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (connected, set_connected) = signal(false);
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
        let (program_progress, set_program_progress) = signal(None);
        let (executing_line, set_executing_line) = signal(None);
        // Robot connection status
        let (robot_connected, set_robot_connected) = signal(false);
        let (robot_addr, set_robot_addr) = signal("127.0.0.1:16001".to_string());
        // Saved robot connections
        let (robot_connections, set_robot_connections) = signal(Vec::new());
        // Currently active/selected connection
        let (active_connection_id, set_active_connection_id) = signal::<Option<i64>>(None);
        let ws: StoredValue<Option<WebSocket>, LocalStorage> = StoredValue::new_local(None);
        let ws_url = StoredValue::new("ws://127.0.0.1:9000".to_string());

        let manager = Self {
            connected,
            set_connected,
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
            program_progress,
            set_program_progress,
            executing_line,
            set_executing_line,
            robot_connected,
            set_robot_connected,
            robot_addr,
            set_robot_addr,
            robot_connections,
            set_robot_connections,
            active_connection_id,
            set_active_connection_id,
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
        let set_program_progress = self.set_program_progress;
        let set_executing_line = self.set_executing_line;
        let set_robot_connected = self.set_robot_connected;
        let set_robot_addr = self.set_robot_addr;
        let set_robot_connections = self.set_robot_connections;

        // On open
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            set_connected.set(true);
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
                                    }));
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                } else {
                    log::error!("Failed to deserialize binary response");
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
                        ServerResponse::ConnectionStatus { connected, robot_addr, robot_port } => {
                            log::info!("Robot connection status: connected={}, addr={}:{}", connected, robot_addr, robot_port);
                            set_robot_connected.set(connected);
                            set_robot_addr.set(format!("{}:{}", robot_addr, robot_port));
                        }
                        ServerResponse::RobotConnections { connections } => {
                            log::info!("Received {} robot connections", connections.len());
                            set_robot_connections.set(connections);
                        }
                        ServerResponse::RobotConnection { connection } => {
                            log::info!("Received robot connection: {}", connection.name);
                            // Could update a single connection in the list if needed
                        }
                    }
                } else {
                    log::error!("Failed to parse API response: {}", text_str);
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // On error
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            log::error!("WebSocket error: {:?}", e);
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        self.ws.set_value(Some(ws));
    }

    /// Send a robot protocol command (binary/bincode)
    pub fn send_command(&self, packet: SendPacket) {
        if let Some(ws) = self.ws.get_value() {
            if let Ok(binary) = bincode::serialize(&packet) {
                let _ = ws.send_with_u8_array(&binary);
            }
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

    /// Upload CSV content to a program
    pub fn upload_csv(&self, program_id: i64, csv_content: String, start_position: Option<StartPosition>) {
        self.send_api_request(ClientRequest::UploadCsv {
            program_id,
            csv_content,
            start_position,
        });
    }

    /// Start program execution
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

    /// Reset database (dangerous!)
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

        // Set disconnected
        self.set_connected.set(false);

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
        self.send_api_request(ClientRequest::ConnectRobot {
            robot_addr: robot_addr.to_string(),
            robot_port,
        });
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

    /// Clear the API error
    pub fn clear_api_error(&self) {
        self.set_api_error.set(None);
    }

    /// Clear the current program (close it)
    pub fn clear_current_program(&self) {
        self.set_current_program.set(None);
    }

    // ========== Robot Connections (Saved Connections) ==========

    /// List all saved robot connections
    pub fn list_robot_connections(&self) {
        self.send_api_request(ClientRequest::ListRobotConnections);
    }

    /// Get a specific robot connection by ID
    pub fn get_robot_connection(&self, id: i64) {
        self.send_api_request(ClientRequest::GetRobotConnection { id });
    }

    /// Create a new saved robot connection
    pub fn create_robot_connection(&self, name: String, description: Option<String>, ip_address: String, port: u32) {
        self.send_api_request(ClientRequest::CreateRobotConnection {
            name,
            description,
            ip_address,
            port,
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

    /// Delete a saved robot connection
    pub fn delete_robot_connection(&self, id: i64) {
        self.send_api_request(ClientRequest::DeleteRobotConnection { id });
    }

    /// Set the active/selected connection ID
    pub fn set_active_connection(&self, id: Option<i64>) {
        self.set_active_connection_id.set(id);
    }
}

fn get_response_ids(resp: &InstructionResponse) -> (u32, u32) {
    match resp {
        InstructionResponse::FrcLinearRelative(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcJointMotion(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcWaitTime(r) => (r.sequence_id, r.error_id),
        _ => (0, 0),
    }
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
