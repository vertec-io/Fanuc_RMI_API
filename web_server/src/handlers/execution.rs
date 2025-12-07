//! Program execution handlers.
//!
//! Handles starting, pausing, resuming, and stopping program execution.

use crate::api_types::ServerResponse;
use crate::database::Database;
use crate::program_executor::ProgramExecutor;
use crate::session::{ClientManager, execution_state_to_response};
use crate::RobotConnection;
use fanuc_rmi::drivers::FanucDriver;
use fanuc_rmi::packets::{PacketPriority, SendPacket, DriverCommand, SentInstructionInfo, ResponsePacket, Command};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, error, warn, debug};

/// Pause program execution.
///
/// This:
/// 1. Pauses the executor (stops sending new instructions from the buffer)
/// 2. Sends FRC_Pause to the robot controller (pauses current motion immediately)
/// 3. Pauses the driver's packet queue (stops sending any queued packets)
/// 4. Broadcasts state change to all connected clients
pub async fn pause_program(
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    client_manager: Option<Arc<ClientManager>>,
) -> ServerResponse {
    if let Some(driver) = driver {
        // Pause the executor first (stops buffered streaming)
        let state_response = if let Some(ref executor) = executor {
            let mut exec_guard = executor.lock().await;
            exec_guard.pause();
            info!("Executor paused");
            Some(execution_state_to_response(&exec_guard.get_state()))
        } else {
            None
        };

        // Broadcast state change to all clients
        if let (Some(client_manager), Some(state_response)) = (&client_manager, state_response) {
            client_manager.broadcast_all(&state_response).await;
        }

        // Send FRC_Pause to the robot to pause current motion immediately
        let pause_packet = SendPacket::Command(Command::FrcPause);
        if let Err(e) = driver.send_packet(pause_packet, PacketPriority::High) {
            return ServerResponse::Error { message: format!("Failed to send pause command: {}", e) };
        }

        // Pause the driver's packet queue to stop sending more instructions
        let driver_pause = SendPacket::DriverCommand(DriverCommand::Pause);
        match driver.send_packet(driver_pause, PacketPriority::High) {
            Ok(_) => {
                info!("Program paused: executor paused, FRC_Pause sent, driver queue paused");
                ServerResponse::Success { message: "Program paused".to_string() }
            },
            Err(e) => ServerResponse::Error { message: format!("Failed to pause driver: {}", e) }
        }
    } else {
        ServerResponse::Error { message: "Robot not connected".to_string() }
    }
}

/// Resume program execution.
///
/// This:
/// 1. Resumes the executor (allows sending more instructions from the buffer)
/// 2. Unpauses the driver's packet queue
/// 3. Sends FRC_Continue to the robot controller (resumes motion)
/// 4. Broadcasts state change to all connected clients
pub async fn resume_program(
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    client_manager: Option<Arc<ClientManager>>,
) -> ServerResponse {
    if let Some(driver) = driver {
        // Resume the executor (allows buffered streaming to continue)
        let state_response = if let Some(ref executor) = executor {
            let mut exec_guard = executor.lock().await;
            exec_guard.resume();
            info!("Executor resumed");
            Some(execution_state_to_response(&exec_guard.get_state()))
        } else {
            None
        };

        // Broadcast state change to all clients
        if let (Some(client_manager), Some(state_response)) = (&client_manager, state_response) {
            client_manager.broadcast_all(&state_response).await;
        }

        // Unpause the driver's packet queue
        let driver_unpause = SendPacket::DriverCommand(DriverCommand::Unpause);
        if let Err(e) = driver.send_packet(driver_unpause, PacketPriority::High) {
            return ServerResponse::Error { message: format!("Failed to unpause driver: {}", e) };
        }

        // Send FRC_Continue to the robot to resume motion
        let continue_packet = SendPacket::Command(Command::FrcContinue);
        match driver.send_packet(continue_packet, PacketPriority::High) {
            Ok(_) => {
                info!("Program resumed: executor resumed, driver queue unpaused, FRC_Continue sent");
                ServerResponse::Success { message: "Program resumed".to_string() }
            },
            Err(e) => ServerResponse::Error { message: format!("Failed to send continue command: {}", e) }
        }
    } else {
        ServerResponse::Error { message: "Robot not connected".to_string() }
    }
}

/// Stop program execution.
///
/// This:
/// 1. Stops the executor (clears pending queue)
/// 2. Sends FRC_Abort to the robot controller (aborts current motion)
/// 3. Clears in-flight tracking
/// 4. Auto-reinitializes the TP program (allows immediate motion commands)
/// 5. Broadcasts state change to all connected clients
pub async fn stop_program(
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    client_manager: Option<Arc<ClientManager>>,
) -> ServerResponse {
    if let Some(driver) = driver {
        // Stop the executor (clears pending queue)
        if let Some(ref executor) = executor {
            let mut exec_guard = executor.lock().await;
            exec_guard.stop();
            info!("Executor stopped, pending queue cleared");
        }

        // Send FRC_Abort to the robot
        match driver.abort().await {
            Ok(_) => {
                // Clear in-flight tracking after abort completes
                let state_response = if let Some(ref executor) = executor {
                    let mut exec_guard = executor.lock().await;
                    exec_guard.clear_in_flight();
                    info!("In-flight tracking cleared");
                    Some(execution_state_to_response(&exec_guard.get_state()))
                } else {
                    None
                };

                // Broadcast state change to all clients
                if let (Some(client_manager), Some(state_response)) = (&client_manager, state_response) {
                    client_manager.broadcast_all(&state_response).await;
                }

                // Auto-reinitialize TP program after abort
                if let Some(ref conn) = robot_connection {
                    info!("Auto-reinitializing TP program after stop...");
                    let mut conn = conn.write().await;
                    match conn.reinitialize_tp().await {
                        Ok(()) => {
                            info!("TP program auto-reinitialized successfully after stop");
                            // Broadcast updated connection status with tp_program_initialized = true
                            if let Some(ref cm) = client_manager {
                                let status = ServerResponse::ConnectionStatus {
                                    connected: conn.connected,
                                    robot_addr: conn.robot_addr.clone(),
                                    robot_port: conn.robot_port,
                                    connection_name: conn.saved_connection.as_ref().map(|s| s.name.clone()),
                                    connection_id: conn.saved_connection.as_ref().map(|s| s.id),
                                    tp_program_initialized: conn.tp_program_initialized,
                                };
                                cm.broadcast_all(&status).await;
                            }
                        }
                        Err(e) => {
                            // Re-initialization failed - leave tp_program_initialized as false
                            info!("Auto-reinitialize failed after stop: {}. Manual initialization required.", e);
                            // Broadcast updated connection status with tp_program_initialized = false
                            if let Some(ref cm) = client_manager {
                                let status = ServerResponse::ConnectionStatus {
                                    connected: conn.connected,
                                    robot_addr: conn.robot_addr.clone(),
                                    robot_port: conn.robot_port,
                                    connection_name: conn.saved_connection.as_ref().map(|s| s.name.clone()),
                                    connection_id: conn.saved_connection.as_ref().map(|s| s.id),
                                    tp_program_initialized: conn.tp_program_initialized,
                                };
                                cm.broadcast_all(&status).await;
                            }
                        }
                    }
                }

                ServerResponse::Success { message: "Program stopped".to_string() }
            },
            Err(e) => ServerResponse::Error { message: format!("Failed to stop: {}", e) }
        }
    } else {
        ServerResponse::Error { message: "Robot not connected".to_string() }
    }
}

/// Get current execution state.
///
/// Used for client reconnection/sync - returns the current state of the executor
/// so the client can restore its UI to the correct state.
pub async fn get_execution_state(
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
) -> ServerResponse {
    if let Some(executor) = executor {
        let exec_guard = executor.lock().await;
        execution_state_to_response(&exec_guard.get_state())
    } else {
        // No executor means idle state
        ServerResponse::ExecutionStateChanged {
            state: "idle".to_string(),
            program_id: None,
            current_line: None,
            total_lines: None,
            message: None,
        }
    }
}

/// Load a program into the executor without starting execution.
///
/// Loads the program from the database into the executor's pending queue.
/// The program is ready to run but won't start until start_program is called.
/// Broadcasts the "loaded" state to all connected clients.
pub async fn load_program(
    db: Arc<Mutex<Database>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    program_id: i64,
    robot_connection: Option<Arc<tokio::sync::RwLock<crate::RobotConnection>>>,
    client_manager: Option<Arc<ClientManager>>,
) -> ServerResponse {
    let executor = match executor {
        Some(e) => e,
        None => return ServerResponse::Error { message: "Executor not available".to_string() }
    };

    // Get robot connection defaults if available
    let robot_defaults = if let Some(ref conn) = robot_connection {
        let conn_guard = conn.read().await;
        conn_guard.saved_connection.clone()
    } else {
        None
    };

    // Load program into executor
    let state_response = {
        let db_guard = db.lock().await;
        let mut exec_guard = executor.lock().await;
        if let Err(e) = exec_guard.load_program(&db_guard, program_id, robot_defaults.as_ref()) {
            return ServerResponse::Error { message: format!("Failed to load program: {}", e) };
        }
        execution_state_to_response(&exec_guard.get_state())
    };

    info!("Loaded program {} into executor", program_id);

    // Broadcast state change to all clients
    if let Some(ref client_manager) = client_manager {
        client_manager.broadcast_all(&state_response).await;
    }

    ServerResponse::Success { message: format!("Program {} loaded", program_id) }
}

/// Unload the current program from the executor.
///
/// Stops any running execution and clears the executor state.
/// Broadcasts the "idle" state to all connected clients.
pub async fn unload_program(
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    client_manager: Option<Arc<ClientManager>>,
) -> ServerResponse {
    // If program is running, stop it first
    if let Some(ref driver) = driver {
        // Send stop command to robot
        let stop_packet = SendPacket::Command(Command::FrcAbort);
        let _ = driver.send_packet(stop_packet, PacketPriority::High);

        // Resume driver (in case it was paused)
        let unpause = SendPacket::DriverCommand(DriverCommand::Unpause);
        let _ = driver.send_packet(unpause, PacketPriority::High);
    }

    // Reset the executor
    let state_response = if let Some(ref executor) = executor {
        let mut exec_guard = executor.lock().await;
        exec_guard.reset();
        info!("Executor reset - program unloaded");
        Some(execution_state_to_response(&exec_guard.get_state()))
    } else {
        None
    };

    // Broadcast state change to all clients
    if let (Some(client_manager), Some(state_response)) = (&client_manager, state_response) {
        client_manager.broadcast_all(&state_response).await;
    }

    ServerResponse::Success { message: "Program unloaded".to_string() }
}

/// Start program execution with buffered streaming.
///
/// This sends instructions in batches of up to 5 (MAX_BUFFER), waiting for
/// completions before sending more. This matches FANUC RMI's buffer behavior
/// and allows for proper pause/stop handling.
/// 5. Broadcasts state change to all connected clients
pub async fn start_program(
    db: Arc<Mutex<Database>>,
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    program_id: i64,
    robot_connection: Option<Arc<tokio::sync::RwLock<crate::RobotConnection>>>,
    client_manager: Option<Arc<ClientManager>>,
) -> ServerResponse {
    let driver = match driver {
        Some(d) => d,
        None => return ServerResponse::Error { message: "Driver not available".to_string() }
    };

    let executor = match executor {
        Some(e) => e,
        None => return ServerResponse::Error { message: "Executor not available".to_string() }
    };

    // Get robot connection defaults if available
    let robot_defaults = if let Some(ref conn) = robot_connection {
        let conn_guard = conn.read().await;
        conn_guard.saved_connection.clone()
    } else {
        None
    };

    // Load program into executor, then start it
    let (total_instructions, state_response) = {
        let db_guard = db.lock().await;
        let mut exec_guard = executor.lock().await;
        if let Err(e) = exec_guard.load_program(&db_guard, program_id, robot_defaults.as_ref()) {
            return ServerResponse::Error { message: format!("Failed to load program: {}", e) };
        }
        exec_guard.start(); // Transitions from Loaded to Running
        let total = exec_guard.total_instructions();
        let state = execution_state_to_response(&exec_guard.get_state());
        (total, state)
    };

    // Broadcast state change to all clients
    if let Some(ref client_manager) = client_manager {
        client_manager.broadcast_all(&state_response).await;
    }

    info!("Starting buffered execution of program {} with {} instructions", program_id, total_instructions);

    // Subscribe to notifications BEFORE sending any instructions
    let sent_rx = driver.sent_instruction_tx.subscribe();
    let response_rx = driver.response_tx.subscribe();

    // Send initial batch
    let initial_batch = {
        let mut exec_guard = executor.lock().await;
        exec_guard.get_next_batch()
    };

    for (line_number, packet) in initial_batch {
        match driver.send_packet(packet, PacketPriority::Standard) {
            Ok(request_id) => {
                let mut exec_guard = executor.lock().await;
                exec_guard.record_sent(request_id, line_number);
                info!("Sent instruction {} (request_id: {})", line_number, request_id);
            }
            Err(e) => {
                error!("Failed to send instruction {}: {}", line_number, e);
                let mut exec_guard = executor.lock().await;
                exec_guard.reset();
                return ServerResponse::Error { message: format!("Failed to send instruction: {}", e) };
            }
        }
    }

    // Send first InstructionSent to all clients
    if let Some(ref client_manager) = client_manager {
        let sent_msg = ServerResponse::InstructionSent {
            current_line: 1,
            total_lines: total_instructions,
        };
        client_manager.broadcast_all(&sent_msg).await;
    }

    // Spawn buffered execution task (broadcasts progress to all clients)
    if let Some(client_manager) = client_manager {
        spawn_buffered_executor(
            driver, executor, sent_rx, response_rx, client_manager,
            total_instructions, program_id,
        );
    }

    ServerResponse::ExecutionStarted {
        program_id,
        total_lines: total_instructions,
    }
}

/// Spawn a task that manages buffered execution.
///
/// This task:
/// 1. Maps request_ids to sequence_ids when SentInstructionInfo arrives
/// 2. Handles instruction completions and sends more instructions
/// 3. Broadcasts progress updates to all connected clients
/// 4. Handles completion/error states
fn spawn_buffered_executor(
    driver: Arc<FanucDriver>,
    executor: Arc<Mutex<ProgramExecutor>>,
    mut sent_rx: tokio::sync::broadcast::Receiver<SentInstructionInfo>,
    mut response_rx: tokio::sync::broadcast::Receiver<ResponsePacket>,
    client_manager: Arc<ClientManager>,
    total_instructions: usize,
    program_id: i64,
) {
    tokio::spawn(async move {
        info!("Buffered executor started for program {}", program_id);

        loop {
            tokio::select! {
                // Handle SentInstructionInfo (map request_id -> sequence_id)
                sent_result = sent_rx.recv() => {
                    match sent_result {
                        Ok(sent_info) => {
                            let mut exec_guard = executor.lock().await;
                            exec_guard.map_sequence(sent_info.request_id, sent_info.sequence_id);
                            debug!("Mapped request {} -> sequence {}", sent_info.request_id, sent_info.sequence_id);
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            warn!("Sent notification channel lagged by {} messages", n);
                        }
                        Err(_) => {
                            // Channel closed
                            break;
                        }
                    }
                }

                // Handle instruction completions
                response_result = response_rx.recv() => {
                    match response_result {
                        Ok(ResponsePacket::InstructionResponse(resp)) => {
                            let seq_id = resp.get_sequence_id();
                            let error_id = resp.get_error_id();

                            let (completed_line, is_complete, is_running) = {
                                let mut exec_guard = executor.lock().await;
                                let line = exec_guard.handle_completion(seq_id);
                                (line, exec_guard.is_complete(), exec_guard.is_running())
                            };

                            if let Some(line) = completed_line {
                                info!("📍 Line {} completed (seq_id {})", line, seq_id);

                                // Broadcast progress update to all clients
                                broadcast_progress_update(&client_manager, line, total_instructions).await;

                                // Check for error
                                if error_id != 0 {
                                    error!("Instruction {} failed with error {}", line, error_id);
                                    let mut exec_guard = executor.lock().await;
                                    exec_guard.reset();
                                    broadcast_error_completion(&client_manager, program_id, line, error_id).await;
                                    return;
                                }

                                // Check for completion
                                if is_complete {
                                    info!("Program {} completed successfully", program_id);
                                    broadcast_success_completion(&client_manager, program_id, total_instructions).await;
                                    return;
                                }

                                // Send more instructions if running
                                if is_running {
                                    let next_batch = {
                                        let mut exec_guard = executor.lock().await;
                                        exec_guard.get_next_batch()
                                    };

                                    for (line_number, packet) in next_batch {
                                        match driver.send_packet(packet, PacketPriority::Standard) {
                                            Ok(request_id) => {
                                                let mut exec_guard = executor.lock().await;
                                                exec_guard.record_sent(request_id, line_number);
                                                info!("Sent instruction {} (request_id: {})", line_number, request_id);
                                            }
                                            Err(e) => {
                                                error!("Failed to send instruction {}: {}", line_number, e);
                                                let mut exec_guard = executor.lock().await;
                                                exec_guard.reset();
                                                broadcast_error_completion(&client_manager, program_id, line_number, 999).await;
                                                return;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Ok(_) => {
                            // Other response types (command responses, etc.)
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            warn!("Response channel lagged by {} messages", n);
                        }
                        Err(_) => {
                            // Channel closed
                            break;
                        }
                    }
                }
            }

            // Check if executor was stopped externally
            {
                let exec_guard = executor.lock().await;
                if matches!(exec_guard.get_state(), crate::program_executor::ExecutionState::Idle | crate::program_executor::ExecutionState::Stopping) {
                    info!("Executor stopped, exiting buffered executor task");
                    break;
                }
            }
        }
    });
}
/// Broadcast a progress update to all connected clients.
async fn broadcast_progress_update(client_manager: &ClientManager, current_line: usize, total_lines: usize) {
    let progress = ServerResponse::InstructionProgress {
        current_line,
        total_lines,
    };
    client_manager.broadcast_all(&progress).await;

    // Broadcast InstructionSent for the next line
    let next_line = current_line + 1;
    if next_line <= total_lines {
        let sent_msg = ServerResponse::InstructionSent {
            current_line: next_line,
            total_lines,
        };
        client_manager.broadcast_all(&sent_msg).await;
    }
}

/// Broadcast an error completion message to all connected clients.
async fn broadcast_error_completion(client_manager: &ClientManager, program_id: i64, line: usize, error_id: u32) {
    error!("Instruction {} failed with error {}", line, error_id);
    let response = ServerResponse::ProgramComplete {
        program_id,
        success: false,
        message: Some(format!("Error at line {}: error_id {}", line, error_id)),
    };
    client_manager.broadcast_all(&response).await;
}

/// Broadcast a success completion message to all connected clients.
async fn broadcast_success_completion(client_manager: &ClientManager, program_id: i64, total_instructions: usize) {
    info!("Program {} completed successfully ({} instructions)", program_id, total_instructions);
    let response = ServerResponse::ProgramComplete {
        program_id,
        success: true,
        message: Some(format!("Completed {} instructions", total_instructions)),
    };
    client_manager.broadcast_all(&response).await;
}

