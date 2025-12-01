//! API request handler for WebSocket messages.
//!
//! Processes client requests and returns server responses.

use crate::api_types::*;
use crate::database::{Database, ProgramInstruction};
use crate::program_parser::{parse_csv_string, ProgramDefaults};
use crate::program_executor::ProgramExecutor;
use crate::RobotConnection;
use fanuc_rmi::drivers::FanucDriver;
use fanuc_rmi::packets::{PacketPriority, SendPacket, DriverCommand};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, error, warn, debug};
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;

/// Type alias for the WebSocket sender
pub type WsSender = Arc<Mutex<futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    Message
>>>;

/// Handle a client API request and return a response.
pub async fn handle_request(
    request: ClientRequest,
    db: Arc<Mutex<Database>>,
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    ws_sender: Option<WsSender>,
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
) -> ServerResponse {
    match request {
        ClientRequest::ListPrograms => list_programs(db).await,
        ClientRequest::GetProgram { id } => get_program(db, id).await,
        ClientRequest::CreateProgram { name, description } => {
            create_program(db, &name, description.as_deref()).await
        }
        ClientRequest::DeleteProgram { id } => delete_program(db, id).await,
        ClientRequest::UploadCsv { program_id, csv_content, start_position } => {
            upload_csv(db, program_id, &csv_content, start_position).await
        }
        ClientRequest::GetSettings => get_settings(db).await,
        ClientRequest::UpdateSettings {
            default_w, default_p, default_r,
            default_speed, default_term_type,
            default_uframe, default_utool,
        } => {
            update_settings(
                db, default_w, default_p, default_r,
                default_speed, &default_term_type,
                default_uframe, default_utool,
            ).await
        }
        ClientRequest::ResetDatabase => reset_database(db).await,
        ClientRequest::StartProgram { program_id } => {
            start_program(db, driver, executor, program_id, ws_sender).await
        }
        ClientRequest::PauseProgram => {
            if let Some(driver) = driver {
                let packet = SendPacket::DriverCommand(DriverCommand::Pause);
                match driver.send_packet(packet, PacketPriority::Standard) {
                    Ok(_) => ServerResponse::Success { message: "Program paused".to_string() },
                    Err(e) => ServerResponse::Error { message: format!("Failed to pause: {}", e) }
                }
            } else {
                ServerResponse::Error { message: "Robot not connected".to_string() }
            }
        }
        ClientRequest::ResumeProgram => {
            if let Some(driver) = driver {
                let packet = SendPacket::DriverCommand(DriverCommand::Unpause);
                match driver.send_packet(packet, PacketPriority::Standard) {
                    Ok(_) => ServerResponse::Success { message: "Program resumed".to_string() },
                    Err(e) => ServerResponse::Error { message: format!("Failed to resume: {}", e) }
                }
            } else {
                ServerResponse::Error { message: "Robot not connected".to_string() }
            }
        }
        ClientRequest::StopProgram => {
            if let Some(driver) = driver {
                match driver.abort().await {
                    Ok(_) => ServerResponse::Success { message: "Program stopped".to_string() },
                    Err(e) => ServerResponse::Error { message: format!("Failed to stop: {}", e) }
                }
            } else {
                ServerResponse::Error { message: "Robot not connected".to_string() }
            }
        }
        ClientRequest::GetConnectionStatus => {
            get_connection_status(robot_connection).await
        }
        ClientRequest::ConnectRobot { robot_addr, robot_port } => {
            connect_robot(robot_connection, robot_addr, robot_port).await
        }
        ClientRequest::DisconnectRobot => {
            disconnect_robot(robot_connection).await
        }
        // Robot Connections (Saved Connections) CRUD
        ClientRequest::ListRobotConnections => {
            list_robot_connections(db).await
        }
        ClientRequest::GetRobotConnection { id } => {
            get_robot_connection(db, id).await
        }
        ClientRequest::CreateRobotConnection { name, description, ip_address, port } => {
            create_robot_connection(db, &name, description.as_deref(), &ip_address, port).await
        }
        ClientRequest::UpdateRobotConnection { id, name, description, ip_address, port } => {
            update_robot_connection(db, id, &name, description.as_deref(), &ip_address, port).await
        }
        ClientRequest::DeleteRobotConnection { id } => {
            delete_robot_connection(db, id).await
        }
    }
}

/// Get the current robot connection status
async fn get_connection_status(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
) -> ServerResponse {
    if let Some(conn) = robot_connection {
        let conn = conn.read().await;
        ServerResponse::ConnectionStatus {
            connected: conn.connected,
            robot_addr: conn.robot_addr.clone(),
            robot_port: conn.robot_port,
        }
    } else {
        ServerResponse::ConnectionStatus {
            connected: false,
            robot_addr: "unknown".to_string(),
            robot_port: 0,
        }
    }
}

/// Connect to a robot
async fn connect_robot(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    robot_addr: String,
    robot_port: u32,
) -> ServerResponse {
    if let Some(conn) = robot_connection {
        let mut conn = conn.write().await;

        // Update address and port
        conn.robot_addr = robot_addr.clone();
        conn.robot_port = robot_port;

        // Attempt to connect
        match conn.connect().await {
            Ok(()) => {
                info!("Successfully connected to robot at {}:{}", robot_addr, robot_port);
                ServerResponse::Success {
                    message: format!("Connected to robot at {}:{}", robot_addr, robot_port)
                }
            }
            Err(e) => {
                warn!("Failed to connect to robot: {}", e);
                ServerResponse::Error {
                    message: format!("Failed to connect: {}", e)
                }
            }
        }
    } else {
        ServerResponse::Error { message: "Robot connection manager not available".to_string() }
    }
}

/// Disconnect from the robot
async fn disconnect_robot(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
) -> ServerResponse {
    if let Some(conn) = robot_connection {
        let mut conn = conn.write().await;
        conn.disconnect();
        info!("Disconnected from robot");
        ServerResponse::Success { message: "Disconnected from robot".to_string() }
    } else {
        ServerResponse::Error { message: "Robot connection manager not available".to_string() }
    }
}

/// Start program execution
async fn start_program(
    db: Arc<Mutex<Database>>,
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    program_id: i64,
    ws_sender: Option<WsSender>,
) -> ServerResponse {
    let driver = match driver {
        Some(d) => d,
        None => return ServerResponse::Error { message: "Driver not available".to_string() }
    };

    let executor = match executor {
        Some(e) => e,
        None => return ServerResponse::Error { message: "Executor not available".to_string() }
    };

    // Load program into executor
    {
        let db_guard = db.lock().await;
        let mut exec_guard = executor.lock().await;
        if let Err(e) = exec_guard.load_program(&db_guard, program_id) {
            return ServerResponse::Error { message: format!("Failed to load program: {}", e) };
        }
    }

    // Get instructions and send them
    let instructions = {
        let exec_guard = executor.lock().await;
        exec_guard.get_all_packets()
    };

    let total_instructions = instructions.len();
    info!("Starting program {} with {} instructions", program_id, total_instructions);

    // Subscribe to sent instruction notifications BEFORE sending any instructions
    // This is critical - broadcast channels only deliver to existing subscribers
    let mut sent_rx = driver.sent_instruction_tx.subscribe();
    // Subscribe to response packets to track instruction completions
    let mut response_rx = driver.response_tx.subscribe();

    // Track request_id -> line number mapping
    let mut request_to_line: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    // Also build sequence_to_line as we receive sent notifications
    let mut sequence_to_line: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut last_request_id: Option<u64> = None;
    let mut last_sequence_id: u32 = 0;

    // Send first InstructionSent to UI to indicate program is starting
    // (highlight line 1 as the first instruction being executed)
    if let Some(ref ws_sender) = ws_sender {
        let sent_msg = ServerResponse::InstructionSent {
            current_line: 1,
            total_lines: total_instructions,
        };
        if let Ok(json) = serde_json::to_string(&sent_msg) {
            let mut sender = ws_sender.lock().await;
            let _ = sender.send(Message::Text(json.into())).await;
        }
    }

    for (i, packet) in instructions.into_iter().enumerate() {
        let line_number = i + 1; // 1-indexed line numbers
        info!("Sending instruction {}: {:?}", line_number, packet);

        match driver.send_packet(packet, PacketPriority::Standard) {
            Ok(request_id) => {
                info!("Instruction {} sent successfully (request_id: {})", line_number, request_id);
                request_to_line.insert(request_id, line_number);
                last_request_id = Some(request_id);
            }
            Err(e) => {
                error!("Failed to send instruction {}: {}", line_number, e);
                return ServerResponse::Error { message: format!("Failed to send instruction: {}", e) };
            }
        }
    }

    // Spawn a task to track progress and send updates
    if let (Some(ws_sender), Some(last_req_id)) = (ws_sender, last_request_id) {
        tokio::spawn(async move {
            // Process both SentInstructionInfo and InstructionResponse concurrently
            // This ensures we can track progress in real-time as instructions complete
            let mut sequence_to_line = sequence_to_line;
            let mut last_sequence_id = last_sequence_id;
            let mut pending_sent_notifications = request_to_line.len();
            let mut completed_count = 0;
            let mut highest_completed_line = 0usize;
            // Buffer responses that arrive before we have the sequence_id mapping
            let mut pending_responses: Vec<(u32, u32)> = Vec::new(); // (seq_id, error_id)

            info!("Starting concurrent tracking: {} instructions, last_req_id: {}",
                  total_instructions, last_req_id);

            loop {
                tokio::select! {
                    // Handle SentInstructionInfo - builds sequence_id -> line mapping
                    sent_result = sent_rx.recv() => {
                        match sent_result {
                            Ok(sent_info) => {
                                if let Some(line) = request_to_line.get(&sent_info.request_id) {
                                    sequence_to_line.insert(sent_info.sequence_id, *line);
                                    pending_sent_notifications -= 1;
                                    debug!("Mapped seq_id {} to line {} (pending: {})",
                                          sent_info.sequence_id, line, pending_sent_notifications);
                                    if sent_info.request_id == last_req_id {
                                        last_sequence_id = sent_info.sequence_id;
                                    }

                                    // Check if any buffered responses can now be processed
                                    let mut i = 0;
                                    while i < pending_responses.len() {
                                        let (seq_id, error_id) = pending_responses[i];
                                        if let Some(&line) = sequence_to_line.get(&seq_id) {
                                            pending_responses.remove(i);
                                            completed_count += 1;

                                            // Only send progress update if this is a new highest line
                                            if line > highest_completed_line {
                                                highest_completed_line = line;
                                                info!("📍 Line {} completed (from buffer)", line);

                                                // Send progress update for completed line
                                                let progress = ServerResponse::InstructionProgress {
                                                    current_line: highest_completed_line,
                                                    total_lines: total_instructions,
                                                };
                                                if let Ok(json) = serde_json::to_string(&progress) {
                                                    let mut sender = ws_sender.lock().await;
                                                    let _ = sender.send(Message::Text(json)).await;
                                                }

                                                // Send InstructionSent for the NEXT line (if there is one)
                                                let next_line = highest_completed_line + 1;
                                                if next_line <= total_instructions {
                                                    let sent_msg = ServerResponse::InstructionSent {
                                                        current_line: next_line,
                                                        total_lines: total_instructions,
                                                    };
                                                    if let Ok(json) = serde_json::to_string(&sent_msg) {
                                                        let mut sender = ws_sender.lock().await;
                                                        let _ = sender.send(Message::Text(json)).await;
                                                    }
                                                }
                                            }

                                            if error_id != 0 {
                                                error!("Instruction {} failed with error {}", line, error_id);
                                                let response = ServerResponse::ProgramComplete {
                                                    program_id,
                                                    success: false,
                                                    message: Some(format!("Error at line {}: error_id {}", line, error_id)),
                                                };
                                                if let Ok(json) = serde_json::to_string(&response) {
                                                    let mut sender = ws_sender.lock().await;
                                                    let _ = sender.send(Message::Text(json)).await;
                                                }
                                                return;
                                            }

                                            if completed_count >= total_instructions {
                                                info!("Program {} completed successfully ({} instructions)", program_id, completed_count);
                                                let response = ServerResponse::ProgramComplete {
                                                    program_id,
                                                    success: true,
                                                    message: Some(format!("Completed {} instructions", total_instructions)),
                                                };
                                                if let Ok(json) = serde_json::to_string(&response) {
                                                    let mut sender = ws_sender.lock().await;
                                                    let _ = sender.send(Message::Text(json)).await;
                                                }
                                                return;
                                            }
                                        } else {
                                            i += 1;
                                        }
                                    }
                                }
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                warn!("Sent notification channel lagged by {} messages", n);
                            }
                            Err(_) => {
                                // Channel closed, continue with response processing
                            }
                        }
                    }

                    // Handle InstructionResponse - track completions
                    response_result = response_rx.recv() => {
                        match response_result {
                            Ok(fanuc_rmi::packets::ResponsePacket::InstructionResponse(resp)) => {
                                let seq_id = resp.get_sequence_id();
                                let error_id = resp.get_error_id();

                                if let Some(&line) = sequence_to_line.get(&seq_id) {
                                    completed_count += 1;

                                    // Only send progress update if this is a new highest line
                                    if line > highest_completed_line {
                                        highest_completed_line = line;
                                        info!("📍 Line {} completed (seq_id {})", line, seq_id);

                                        // Send progress update for completed line
                                        let progress = ServerResponse::InstructionProgress {
                                            current_line: highest_completed_line,
                                            total_lines: total_instructions,
                                        };
                                        if let Ok(json) = serde_json::to_string(&progress) {
                                            let mut sender = ws_sender.lock().await;
                                            let _ = sender.send(Message::Text(json)).await;
                                        }

                                        // Send InstructionSent for the NEXT line (if there is one)
                                        // This tells the UI to highlight the next instruction being executed
                                        let next_line = highest_completed_line + 1;
                                        if next_line <= total_instructions {
                                            let sent_msg = ServerResponse::InstructionSent {
                                                current_line: next_line,
                                                total_lines: total_instructions,
                                            };
                                            if let Ok(json) = serde_json::to_string(&sent_msg) {
                                                let mut sender = ws_sender.lock().await;
                                                let _ = sender.send(Message::Text(json)).await;
                                            }
                                        }
                                    }

                                    if error_id != 0 {
                                        error!("Instruction {} failed with error {}", line, error_id);
                                        let response = ServerResponse::ProgramComplete {
                                            program_id,
                                            success: false,
                                            message: Some(format!("Error at line {}: error_id {}", line, error_id)),
                                        };
                                        if let Ok(json) = serde_json::to_string(&response) {
                                            let mut sender = ws_sender.lock().await;
                                            let _ = sender.send(Message::Text(json)).await;
                                        }
                                        return;
                                    }

                                    if completed_count >= total_instructions {
                                        info!("Program {} completed successfully ({} instructions)", program_id, completed_count);
                                        let response = ServerResponse::ProgramComplete {
                                            program_id,
                                            success: true,
                                            message: Some(format!("Completed {} instructions", total_instructions)),
                                        };
                                        if let Ok(json) = serde_json::to_string(&response) {
                                            let mut sender = ws_sender.lock().await;
                                            let _ = sender.send(Message::Text(json)).await;
                                        }
                                        return;
                                    }
                                } else {
                                    // Response arrived before we have the mapping - buffer it
                                    debug!("Buffering response for seq_id {} (mapping not yet available)", seq_id);
                                    pending_responses.push((seq_id, error_id));
                                }
                            }
                            Ok(_) => {
                                // Non-instruction response, ignore
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                warn!("Response channel lagged by {} messages, continuing...", n);
                            }
                            Err(e) => {
                                error!("Response channel closed: {}", e);
                                // Send completion anyway since we might have just missed the final message
                                if highest_completed_line > 0 {
                                    let response = ServerResponse::ProgramComplete {
                                        program_id,
                                        success: true,
                                        message: Some(format!("Completed (tracked {} of {} instructions)", completed_count, total_instructions)),
                                    };
                                    if let Ok(json) = serde_json::to_string(&response) {
                                        let mut sender = ws_sender.lock().await;
                                        let _ = sender.send(Message::Text(json)).await;
                                    }
                                }
                                return;
                            }
                        }
                    }
                }
            }
        });
    }

    ServerResponse::ExecutionStarted {
        program_id,
        total_lines: total_instructions,
    }
}

async fn list_programs(db: Arc<Mutex<Database>>) -> ServerResponse {
    let db = db.lock().await;
    match db.list_programs() {
        Ok(programs) => {
            let program_infos: Vec<ProgramInfo> = programs.iter().map(|p| {
                let count = db.instruction_count(p.id).unwrap_or(0);
                ProgramInfo {
                    id: p.id,
                    name: p.name.clone(),
                    description: p.description.clone(),
                    instruction_count: count,
                    created_at: p.created_at.clone(),
                    updated_at: p.updated_at.clone(),
                }
            }).collect();
            ServerResponse::Programs { programs: program_infos }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to list programs: {}", e) }
    }
}

async fn get_program(db: Arc<Mutex<Database>>, id: i64) -> ServerResponse {
    let db = db.lock().await;
    match db.get_program(id) {
        Ok(Some(program)) => {
            let instructions = db.get_instructions(id).unwrap_or_default();
            let instruction_dtos: Vec<InstructionDto> = instructions.iter().map(|i| {
                InstructionDto {
                    line_number: i.line_number,
                    x: i.x,
                    y: i.y,
                    z: i.z,
                    w: i.w,
                    p: i.p,
                    r: i.r,
                    speed: i.speed,
                    term_type: i.term_type.clone(),
                }
            }).collect();
            ServerResponse::Program {
                program: ProgramDetail {
                    id: program.id,
                    name: program.name,
                    description: program.description,
                    instructions: instruction_dtos,
                    start_x: program.start_x,
                    start_y: program.start_y,
                    start_z: program.start_z,
                }
            }
        }
        Ok(None) => ServerResponse::Error { message: "Program not found".to_string() },
        Err(e) => ServerResponse::Error { message: format!("Failed to get program: {}", e) }
    }
}

async fn create_program(db: Arc<Mutex<Database>>, name: &str, description: Option<&str>) -> ServerResponse {
    let db = db.lock().await;
    match db.create_program(name, description) {
        Ok(id) => {
            info!("Created program '{}' with id {}", name, id);
            ServerResponse::Success { message: format!("Created program with id {}", id) }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to create program: {}", e) }
    }
}

async fn delete_program(db: Arc<Mutex<Database>>, id: i64) -> ServerResponse {
    let db = db.lock().await;
    match db.delete_program(id) {
        Ok(_) => {
            info!("Deleted program {}", id);
            ServerResponse::Success { message: "Program deleted".to_string() }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to delete program: {}", e) }
    }
}

async fn upload_csv(
    db: Arc<Mutex<Database>>,
    program_id: i64,
    csv_content: &str,
    start_position: Option<StartPosition>,
) -> ServerResponse {
    let db = db.lock().await;
    
    // Get robot settings for defaults
    let settings = match db.get_robot_settings() {
        Ok(s) => s,
        Err(e) => return ServerResponse::Error { 
            message: format!("Failed to get robot settings: {}", e) 
        }
    };
    
    let defaults = ProgramDefaults {
        w: settings.default_w,
        p: settings.default_p,
        r: settings.default_r,
        ext1: 0.0,
        ext2: 0.0,
        ext3: 0.0,
        speed: settings.default_speed,
        term_type: settings.default_term_type.clone(),
        uframe: Some(settings.default_uframe),
        utool: Some(settings.default_utool),
    };

    // Parse CSV
    let instructions = match parse_csv_string(csv_content, &defaults) {
        Ok(instrs) => instrs,
        Err(e) => return ServerResponse::Error {
            message: format!("Failed to parse CSV: {:?}", e)
        }
    };

    // Clear existing instructions
    if let Err(e) = db.clear_instructions(program_id) {
        return ServerResponse::Error {
            message: format!("Failed to clear existing instructions: {}", e)
        };
    }

    // Add new instructions
    for instr in &instructions {
        let db_instr = ProgramInstruction {
            id: 0,
            program_id,
            line_number: instr.line_number,
            x: instr.x,
            y: instr.y,
            z: instr.z,
            w: instr.w,
            p: instr.p,
            r: instr.r,
            ext1: instr.ext1,
            ext2: instr.ext2,
            ext3: instr.ext3,
            speed: instr.speed,
            term_type: instr.term_type.clone(),
            uframe: instr.uframe,
            utool: instr.utool,
        };
        if let Err(e) = db.add_instruction(program_id, &db_instr) {
            return ServerResponse::Error {
                message: format!("Failed to add instruction: {}", e)
            };
        }
    }

    // Update start position if provided (use first instruction as default if not)
    let (start_x, start_y, start_z) = if let Some(start) = start_position {
        (Some(start.x), Some(start.y), Some(start.z))
    } else if let Some(first) = instructions.first() {
        (Some(first.x), Some(first.y), Some(first.z))
    } else {
        (Some(0.0), Some(0.0), Some(0.0))
    };

    // Update program with start position and defaults from robot settings
    if let Ok(Some(prog)) = db.get_program(program_id) {
        let _ = db.update_program(
            program_id,
            &prog.name,
            prog.description.as_deref(),
            settings.default_w,
            settings.default_p,
            settings.default_r,
            Some(settings.default_speed),
            &settings.default_term_type,
            Some(settings.default_uframe),
            Some(settings.default_utool),
            start_x,
            start_y,
            start_z,
        );
    }

    info!("Uploaded {} instructions to program {}", instructions.len(), program_id);
    ServerResponse::Success {
        message: format!("Uploaded {} instructions", instructions.len())
    }
}

async fn get_settings(db: Arc<Mutex<Database>>) -> ServerResponse {
    let db = db.lock().await;
    match db.get_robot_settings() {
        Ok(settings) => ServerResponse::Settings {
            settings: RobotSettingsDto {
                default_w: settings.default_w,
                default_p: settings.default_p,
                default_r: settings.default_r,
                default_speed: settings.default_speed,
                default_term_type: settings.default_term_type,
                default_uframe: settings.default_uframe,
                default_utool: settings.default_utool,
            }
        },
        Err(e) => ServerResponse::Error { message: format!("Failed to get settings: {}", e) }
    }
}

async fn update_settings(
    db: Arc<Mutex<Database>>,
    default_w: f64,
    default_p: f64,
    default_r: f64,
    default_speed: f64,
    default_term_type: &str,
    default_uframe: i32,
    default_utool: i32,
) -> ServerResponse {
    let db = db.lock().await;
    match db.update_robot_settings(
        default_w, default_p, default_r,
        default_speed, default_term_type,
        default_uframe, default_utool,
    ) {
        Ok(_) => {
            info!("Updated robot settings");
            ServerResponse::Success { message: "Settings updated".to_string() }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to update settings: {}", e) }
    }
}

async fn reset_database(db: Arc<Mutex<Database>>) -> ServerResponse {
    let mut db = db.lock().await;
    match db.reset() {
        Ok(_) => {
            info!("Database reset");
            ServerResponse::Success { message: "Database reset successfully".to_string() }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to reset database: {}", e) }
    }
}

// ========== Robot Connections CRUD ==========

async fn list_robot_connections(db: Arc<Mutex<Database>>) -> ServerResponse {
    let db = db.lock().await;
    match db.list_robot_connections() {
        Ok(connections) => {
            let connections: Vec<RobotConnectionDto> = connections.iter().map(|c| RobotConnectionDto {
                id: c.id,
                name: c.name.clone(),
                description: c.description.clone(),
                ip_address: c.ip_address.clone(),
                port: c.port,
            }).collect();
            ServerResponse::RobotConnections { connections }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to list connections: {}", e) }
    }
}

async fn get_robot_connection(db: Arc<Mutex<Database>>, id: i64) -> ServerResponse {
    let db = db.lock().await;
    match db.get_robot_connection(id) {
        Ok(Some(c)) => {
            ServerResponse::RobotConnection {
                connection: RobotConnectionDto {
                    id: c.id,
                    name: c.name,
                    description: c.description,
                    ip_address: c.ip_address,
                    port: c.port,
                }
            }
        }
        Ok(None) => ServerResponse::Error { message: "Connection not found".to_string() },
        Err(e) => ServerResponse::Error { message: format!("Failed to get connection: {}", e) }
    }
}

async fn create_robot_connection(
    db: Arc<Mutex<Database>>,
    name: &str,
    description: Option<&str>,
    ip_address: &str,
    port: u32,
) -> ServerResponse {
    let db = db.lock().await;
    match db.create_robot_connection(name, description, ip_address, port) {
        Ok(id) => {
            info!("Created robot connection: {} (id={})", name, id);
            ServerResponse::Success { message: format!("Connection '{}' created", name) }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to create connection: {}", e) }
    }
}

async fn update_robot_connection(
    db: Arc<Mutex<Database>>,
    id: i64,
    name: &str,
    description: Option<&str>,
    ip_address: &str,
    port: u32,
) -> ServerResponse {
    let db = db.lock().await;
    match db.update_robot_connection(id, name, description, ip_address, port) {
        Ok(_) => {
            info!("Updated robot connection: {} (id={})", name, id);
            ServerResponse::Success { message: format!("Connection '{}' updated", name) }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to update connection: {}", e) }
    }
}

async fn delete_robot_connection(db: Arc<Mutex<Database>>, id: i64) -> ServerResponse {
    let db = db.lock().await;
    match db.delete_robot_connection(id) {
        Ok(_) => {
            info!("Deleted robot connection id={}", id);
            ServerResponse::Success { message: "Connection deleted".to_string() }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to delete connection: {}", e) }
    }
}
