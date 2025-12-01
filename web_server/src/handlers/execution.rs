//! Program execution handlers.
//!
//! Handles starting, pausing, resuming, and stopping program execution.

use crate::api_types::ServerResponse;
use crate::database::Database;
use crate::program_executor::ProgramExecutor;
use crate::handlers::WsSender;
use fanuc_rmi::drivers::FanucDriver;
use fanuc_rmi::packets::{PacketPriority, SendPacket, DriverCommand, SentInstructionInfo, ResponsePacket, Command};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error, warn, debug};
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;

/// Pause program execution.
///
/// This:
/// 1. Pauses the executor (stops sending new instructions from the buffer)
/// 2. Sends FRC_Pause to the robot controller (pauses current motion immediately)
/// 3. Pauses the driver's packet queue (stops sending any queued packets)
pub async fn pause_program(
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
) -> ServerResponse {
    if let Some(driver) = driver {
        // Pause the executor first (stops buffered streaming)
        if let Some(executor) = executor {
            let mut exec_guard = executor.lock().await;
            exec_guard.pause();
            info!("Executor paused");
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
pub async fn resume_program(
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
) -> ServerResponse {
    if let Some(driver) = driver {
        // Resume the executor (allows buffered streaming to continue)
        if let Some(executor) = executor {
            let mut exec_guard = executor.lock().await;
            exec_guard.resume();
            info!("Executor resumed");
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
pub async fn stop_program(
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
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
                if let Some(executor) = executor {
                    let mut exec_guard = executor.lock().await;
                    exec_guard.clear_in_flight();
                    info!("In-flight tracking cleared");
                }
                ServerResponse::Success { message: "Program stopped".to_string() }
            },
            Err(e) => ServerResponse::Error { message: format!("Failed to stop: {}", e) }
        }
    } else {
        ServerResponse::Error { message: "Robot not connected".to_string() }
    }
}

/// Start program execution with buffered streaming.
///
/// This sends instructions in batches of up to 5 (MAX_BUFFER), waiting for
/// completions before sending more. This matches FANUC RMI's buffer behavior
/// and allows for proper pause/stop handling.
pub async fn start_program(
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
    let total_instructions = {
        let db_guard = db.lock().await;
        let mut exec_guard = executor.lock().await;
        if let Err(e) = exec_guard.load_program(&db_guard, program_id) {
            return ServerResponse::Error { message: format!("Failed to load program: {}", e) };
        }
        exec_guard.start(program_id);
        exec_guard.total_instructions()
    };

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

    // Send first InstructionSent to UI
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

    // Spawn buffered execution task
    if let Some(ws_sender) = ws_sender {
        spawn_buffered_executor(
            driver, executor, sent_rx, response_rx, ws_sender,
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
/// 3. Sends progress updates to the client
/// 4. Handles completion/error states
fn spawn_buffered_executor(
    driver: Arc<FanucDriver>,
    executor: Arc<Mutex<ProgramExecutor>>,
    mut sent_rx: tokio::sync::broadcast::Receiver<SentInstructionInfo>,
    mut response_rx: tokio::sync::broadcast::Receiver<ResponsePacket>,
    ws_sender: WsSender,
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

                                // Send progress update
                                send_progress_update(&ws_sender, line, total_instructions).await;

                                // Check for error
                                if error_id != 0 {
                                    error!("Instruction {} failed with error {}", line, error_id);
                                    let mut exec_guard = executor.lock().await;
                                    exec_guard.reset();
                                    send_error_completion(&ws_sender, program_id, line, error_id).await;
                                    return;
                                }

                                // Check for completion
                                if is_complete {
                                    info!("Program {} completed successfully", program_id);
                                    send_success_completion(&ws_sender, program_id, total_instructions).await;
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
                                                send_error_completion(&ws_sender, program_id, line_number, 999).await;
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
/// Send a progress update to the client.
async fn send_progress_update(ws_sender: &WsSender, current_line: usize, total_lines: usize) {
    let progress = ServerResponse::InstructionProgress {
        current_line,
        total_lines,
    };
    if let Ok(json) = serde_json::to_string(&progress) {
        let mut sender = ws_sender.lock().await;
        let _ = sender.send(Message::Text(json)).await;
    }

    // Send InstructionSent for the next line
    let next_line = current_line + 1;
    if next_line <= total_lines {
        let sent_msg = ServerResponse::InstructionSent {
            current_line: next_line,
            total_lines,
        };
        if let Ok(json) = serde_json::to_string(&sent_msg) {
            let mut sender = ws_sender.lock().await;
            let _ = sender.send(Message::Text(json)).await;
        }
    }
}

/// Send an error completion message.
async fn send_error_completion(ws_sender: &WsSender, program_id: i64, line: usize, error_id: u32) {
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
}

/// Send a success completion message.
async fn send_success_completion(ws_sender: &WsSender, program_id: i64, total_instructions: usize) {
    info!("Program {} completed successfully ({} instructions)", program_id, total_instructions);
    let response = ServerResponse::ProgramComplete {
        program_id,
        success: true,
        message: Some(format!("Completed {} instructions", total_instructions)),
    };
    if let Ok(json) = serde_json::to_string(&response) {
        let mut sender = ws_sender.lock().await;
        let _ = sender.send(Message::Text(json)).await;
    }
}

