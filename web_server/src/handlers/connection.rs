//! Robot connection handlers.
//!
//! Handles connecting, disconnecting, and checking status of the robot connection.

use crate::api_types::ServerResponse;
use crate::database::Database;
use crate::session::ClientManager;
use crate::RobotConnection;
use fanuc_rmi::commands::FrcSetUFrameUTool;
use fanuc_rmi::packets::{Command, CommandResponse, ResponsePacket, SendPacket, PacketPriority};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

/// Get the current robot connection status.
pub async fn get_connection_status(
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

/// Connect to a robot at the specified address and port.
pub async fn connect_robot(
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

/// Disconnect from the robot.
pub async fn disconnect_robot(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
) -> ServerResponse {
    if let Some(conn) = robot_connection {
        let mut conn = conn.write().await;
        conn.disconnect();
        conn.saved_connection = None; // Clear saved connection on disconnect
        info!("Disconnected from robot");
        ServerResponse::Success { message: "Disconnected from robot".to_string() }
    } else {
        ServerResponse::Error { message: "Robot connection manager not available".to_string() }
    }
}

/// Connect to a saved robot connection by ID.
/// Returns the effective settings (per-robot defaults or global fallback).
///
/// After successful connection:
/// 1. Sends FrcSetUFrameUTool to robot to set the default frame/tool
/// 2. Stores active_uframe/active_utool in server state
/// 3. Broadcasts ActiveFrameTool to all clients
pub async fn connect_to_saved_robot(
    db: Arc<Mutex<Database>>,
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    client_manager: Option<Arc<ClientManager>>,
    connection_id: i64,
) -> ServerResponse {
    // First, look up the saved connection
    let (saved_conn, global_settings) = {
        let db = db.lock().await;
        let conn = match db.get_robot_connection(connection_id) {
            Ok(Some(c)) => c,
            Ok(None) => return ServerResponse::Error { message: "Connection not found".to_string() },
            Err(e) => return ServerResponse::Error { message: format!("Database error: {}", e) },
        };
        let settings = match db.get_robot_settings() {
            Ok(s) => s,
            Err(e) => return ServerResponse::Error { message: format!("Failed to get settings: {}", e) },
        };
        (conn, settings)
    };

    // Connect to the robot
    let Some(conn) = robot_connection else {
        return ServerResponse::Error { message: "Robot connection manager not available".to_string() };
    };

    let mut conn_guard = conn.write().await;
    conn_guard.robot_addr = saved_conn.ip_address.clone();
    conn_guard.robot_port = saved_conn.port;

    match conn_guard.connect().await {
        Ok(()) => {
            info!("Successfully connected to saved robot '{}' at {}:{}",
                saved_conn.name, saved_conn.ip_address, saved_conn.port);

            // Calculate effective settings (per-robot or global fallback)
            let effective_speed = saved_conn.default_speed.unwrap_or(global_settings.default_speed);
            let effective_term_type = saved_conn.default_term_type.clone()
                .unwrap_or(global_settings.default_term_type.clone());
            let effective_uframe = saved_conn.default_uframe.unwrap_or(global_settings.default_uframe) as u8;
            let effective_utool = saved_conn.default_utool.unwrap_or(global_settings.default_utool) as u8;
            let effective_w = saved_conn.default_w.unwrap_or(global_settings.default_w);
            let effective_p = saved_conn.default_p.unwrap_or(global_settings.default_p);
            let effective_r = saved_conn.default_r.unwrap_or(global_settings.default_r);

            // Send FrcSetUFrameUTool to set the robot to default frame/tool
            if let Some(ref driver) = conn_guard.driver {
                let cmd = FrcSetUFrameUTool::new(None, effective_utool, effective_uframe);
                let packet = SendPacket::Command(Command::FrcSetUFrameUTool(cmd));

                let mut response_rx = driver.response_tx.subscribe();
                if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
                    warn!("Failed to send FrcSetUFrameUTool command: {}", e);
                } else {
                    // Wait for response (with timeout)
                    match tokio::time::timeout(Duration::from_secs(3), async {
                        while let Ok(response) = response_rx.recv().await {
                            if let ResponsePacket::CommandResponse(CommandResponse::FrcSetUFrameUTool(resp)) = response {
                                return Some(resp);
                            }
                        }
                        None
                    }).await {
                        Ok(Some(resp)) => {
                            if resp.error_id != 0 {
                                warn!("FrcSetUFrameUTool failed with error: {}", resp.error_id);
                            } else {
                                info!("Set robot to default UFrame={}, UTool={}", effective_uframe, effective_utool);
                            }
                        }
                        Ok(None) => warn!("No response to FrcSetUFrameUTool"),
                        Err(_) => warn!("Timeout waiting for FrcSetUFrameUTool response"),
                    }
                }
            }

            // Store active frame/tool in server state
            conn_guard.active_uframe = effective_uframe;
            conn_guard.active_utool = effective_utool;

            // Broadcast ActiveFrameTool to all clients
            if let Some(ref client_manager) = client_manager {
                let frame_tool_response = ServerResponse::ActiveFrameTool {
                    uframe: effective_uframe,
                    utool: effective_utool,
                };
                client_manager.broadcast_all(&frame_tool_response).await;
            }

            // Store the saved connection for configuration defaults
            conn_guard.saved_connection = Some(saved_conn.clone());

            ServerResponse::RobotConnected {
                connection_id: saved_conn.id,
                connection_name: saved_conn.name,
                robot_addr: saved_conn.ip_address,
                robot_port: saved_conn.port,
                effective_speed,
                effective_term_type,
                effective_uframe: effective_uframe as i32,
                effective_utool: effective_utool as i32,
                effective_w,
                effective_p,
                effective_r,
            }
        }
        Err(e) => {
            warn!("Failed to connect to saved robot '{}': {}", saved_conn.name, e);
            ServerResponse::Error {
                message: format!("Failed to connect to '{}': {}", saved_conn.name, e)
            }
        }
    }
}

