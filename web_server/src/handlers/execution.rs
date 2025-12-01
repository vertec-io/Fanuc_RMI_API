//! Program execution handlers.
//!
//! Handles starting, pausing, resuming, and stopping program execution.

use crate::api_types::ServerResponse;
use crate::database::Database;
use crate::program_executor::ProgramExecutor;
use crate::handlers::WsSender;
use fanuc_rmi::drivers::FanucDriver;
use fanuc_rmi::packets::{PacketPriority, SendPacket, DriverCommand, SentInstructionInfo, ResponsePacket};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, error, warn, debug};
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;

/// Pause program execution.
pub async fn pause_program(driver: Option<Arc<FanucDriver>>) -> ServerResponse {
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

/// Resume program execution.
pub async fn resume_program(driver: Option<Arc<FanucDriver>>) -> ServerResponse {
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

/// Stop program execution.
pub async fn stop_program(driver: Option<Arc<FanucDriver>>) -> ServerResponse {
    if let Some(driver) = driver {
        match driver.abort().await {
            Ok(_) => ServerResponse::Success { message: "Program stopped".to_string() },
            Err(e) => ServerResponse::Error { message: format!("Failed to stop: {}", e) }
        }
    } else {
        ServerResponse::Error { message: "Robot not connected".to_string() }
    }
}

/// Start program execution.
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
    let sent_rx = driver.sent_instruction_tx.subscribe();
    let response_rx = driver.response_tx.subscribe();

    // Track request_id -> line number mapping
    let mut request_to_line: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    let sequence_to_line: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    let mut last_request_id: Option<u64> = None;
    #[allow(unused_variables)]
    let last_sequence_id: u32 = 0;

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

    for (i, packet) in instructions.into_iter().enumerate() {
        let line_number = i + 1;
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

    // Spawn progress tracking task
    if let (Some(ws_sender), Some(last_req_id)) = (ws_sender, last_request_id) {
        spawn_progress_tracker(
            sent_rx, response_rx, ws_sender, request_to_line, sequence_to_line,
            total_instructions, program_id, last_req_id,
        );
    }

    ServerResponse::ExecutionStarted {
        program_id,
        total_lines: total_instructions,
    }
}

/// Spawn a task to track program execution progress and send updates to the client.
fn spawn_progress_tracker(
    mut sent_rx: tokio::sync::broadcast::Receiver<SentInstructionInfo>,
    mut response_rx: tokio::sync::broadcast::Receiver<ResponsePacket>,
    ws_sender: WsSender,
    request_to_line: std::collections::HashMap<u64, usize>,
    sequence_to_line: std::collections::HashMap<u32, usize>,
    total_instructions: usize,
    program_id: i64,
    last_req_id: u64,
) {
    tokio::spawn(async move {
        let mut sequence_to_line = sequence_to_line;
        let mut pending_sent_notifications = request_to_line.len();
        let mut completed_count = 0;
        let mut highest_completed_line = 0usize;
        let mut pending_responses: Vec<(u32, u32)> = Vec::new();

        info!("Starting concurrent tracking: {} instructions, last_req_id: {}",
              total_instructions, last_req_id);

        loop {
            tokio::select! {
                sent_result = sent_rx.recv() => {
                    handle_sent_notification(
                        sent_result, &request_to_line, &mut sequence_to_line,
                        &mut pending_sent_notifications, &mut pending_responses,
                        &mut completed_count, &mut highest_completed_line,
                        total_instructions, program_id, &ws_sender,
                    ).await;

                    if completed_count >= total_instructions {
                        return;
                    }
                }

                response_result = response_rx.recv() => {
                    let should_exit = handle_response(
                        response_result, &sequence_to_line, &mut pending_responses,
                        &mut completed_count, &mut highest_completed_line,
                        total_instructions, program_id, &ws_sender,
                    ).await;

                    if should_exit {
                        return;
                    }
                }
            }
        }
    });
}


/// Handle a sent instruction notification.
async fn handle_sent_notification(
    sent_result: Result<SentInstructionInfo, tokio::sync::broadcast::error::RecvError>,
    request_to_line: &std::collections::HashMap<u64, usize>,
    sequence_to_line: &mut std::collections::HashMap<u32, usize>,
    pending_sent_notifications: &mut usize,
    pending_responses: &mut Vec<(u32, u32)>,
    completed_count: &mut usize,
    highest_completed_line: &mut usize,
    total_instructions: usize,
    program_id: i64,
    ws_sender: &WsSender,
) {
    match sent_result {
        Ok(sent_info) => {
            if let Some(line) = request_to_line.get(&sent_info.request_id) {
                sequence_to_line.insert(sent_info.sequence_id, *line);
                *pending_sent_notifications -= 1;
                debug!("Mapped seq_id {} to line {} (pending: {})",
                      sent_info.sequence_id, line, pending_sent_notifications);

                // Process any buffered responses
                let mut i = 0;
                while i < pending_responses.len() {
                    let (seq_id, error_id) = pending_responses[i];
                    if let Some(&line) = sequence_to_line.get(&seq_id) {
                        pending_responses.remove(i);
                        *completed_count += 1;

                        if line > *highest_completed_line {
                            *highest_completed_line = line;
                            info!("📍 Line {} completed (from buffer)", line);
                            send_progress_update(ws_sender, *highest_completed_line, total_instructions).await;
                        }

                        if error_id != 0 {
                            send_error_completion(ws_sender, program_id, line, error_id).await;
                            return;
                        }

                        if *completed_count >= total_instructions {
                            send_success_completion(ws_sender, program_id, total_instructions).await;
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
        Err(_) => {}
    }
}

/// Handle a response packet.
async fn handle_response(
    response_result: Result<ResponsePacket, tokio::sync::broadcast::error::RecvError>,
    sequence_to_line: &std::collections::HashMap<u32, usize>,
    pending_responses: &mut Vec<(u32, u32)>,
    completed_count: &mut usize,
    highest_completed_line: &mut usize,
    total_instructions: usize,
    program_id: i64,
    ws_sender: &WsSender,
) -> bool {
    match response_result {
        Ok(fanuc_rmi::packets::ResponsePacket::InstructionResponse(resp)) => {
            let seq_id = resp.get_sequence_id();
            let error_id = resp.get_error_id();

            if let Some(&line) = sequence_to_line.get(&seq_id) {
                *completed_count += 1;

                if line > *highest_completed_line {
                    *highest_completed_line = line;
                    info!("📍 Line {} completed (seq_id {})", line, seq_id);
                    send_progress_update(ws_sender, *highest_completed_line, total_instructions).await;
                }

                if error_id != 0 {
                    send_error_completion(ws_sender, program_id, line, error_id).await;
                    return true;
                }

                if *completed_count >= total_instructions {
                    send_success_completion(ws_sender, program_id, total_instructions).await;
                    return true;
                }
            } else {
                debug!("Buffering response for seq_id {} (mapping not yet available)", seq_id);
                pending_responses.push((seq_id, error_id));
            }
        }
        Ok(_) => {}
        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
            warn!("Response channel lagged by {} messages, continuing...", n);
        }
        Err(e) => {
            error!("Response channel closed: {}", e);
            if *highest_completed_line > 0 {
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
            return true;
        }
    }
    false
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

