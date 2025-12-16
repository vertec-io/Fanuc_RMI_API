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
        // Get connection name and ID from saved_connection if available
        let (connection_name, connection_id) = if let Some(ref saved) = conn.saved_connection {
            (Some(saved.name.clone()), Some(saved.id))
        } else {
            (None, None)
        };
        ServerResponse::ConnectionStatus {
            connected: conn.connected,
            robot_addr: conn.robot_addr.clone(),
            robot_port: conn.robot_port,
            connection_name,
            connection_id,
            tp_program_initialized: conn.tp_program_initialized,
        }
    } else {
        ServerResponse::ConnectionStatus {
            connected: false,
            robot_addr: "unknown".to_string(),
            robot_port: 0,
            connection_name: None,
            connection_id: None,
            tp_program_initialized: false,
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
///
/// This sends FRC_Disconnect to the robot and waits for acknowledgment before dropping the driver.
/// Note: This does NOT clear saved_connection or active_configuration, so reconnection can work properly.
pub async fn disconnect_robot(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
) -> ServerResponse {
    if let Some(conn) = robot_connection {
        let mut conn = conn.write().await;
        conn.disconnect_async().await;
        // DO NOT clear saved_connection - keep it so reconnection works
        // DO NOT reset active_configuration - keep it so reconnection works
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
/// 1. Loads the default configuration (if one exists) or uses robot defaults
/// 2. Sends FrcSetUFrameUTool to robot to set the frame/tool
/// 3. Stores active configuration in server state
/// 4. Broadcasts ActiveFrameTool to all clients
pub async fn connect_to_saved_robot(
    db: Arc<Mutex<Database>>,
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    client_manager: Option<Arc<ClientManager>>,
    connection_id: i64,
) -> ServerResponse {
    // Look up the saved connection and default configuration
    let (saved_conn, default_config) = {
        let db = db.lock().await;
        let conn = match db.get_robot_connection(connection_id) {
            Ok(Some(c)) => c,
            Ok(None) => return ServerResponse::Error { message: "Connection not found".to_string() },
            Err(e) => return ServerResponse::Error { message: format!("Database error: {}", e) },
        };
        // Get the default configuration for this robot (REQUIRED)
        let config = match db.get_default_robot_configuration(connection_id) {
            Ok(Some(c)) => c,
            Ok(None) => {
                return ServerResponse::Error {
                    message: format!(
                        "Robot '{}' has no default configuration. Please create at least one configuration and mark it as default.",
                        conn.name
                    )
                };
            }
            Err(e) => {
                return ServerResponse::Error {
                    message: format!("Failed to get default configuration: {}", e)
                };
            }
        };
        (conn, config)
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

            // Initialize active configuration from default config
            info!("Loading default configuration '{}' for robot", default_config.name);
            conn_guard.active_configuration = crate::ActiveConfiguration::from_saved(&default_config, &saved_conn);

            // Initialize active jog settings from saved connection defaults
            // These are the "active jog controls" that can be changed independently from the defaults
            conn_guard.active_cartesian_jog_speed = saved_conn.default_cartesian_jog_speed;
            conn_guard.active_cartesian_jog_step = saved_conn.default_cartesian_jog_step;
            conn_guard.active_joint_jog_speed = saved_conn.default_joint_jog_speed;
            conn_guard.active_joint_jog_step = saved_conn.default_joint_jog_step;
            info!("Loaded jog defaults: cart_speed={}, cart_step={}, joint_speed={}, joint_step={}",
                conn_guard.active_cartesian_jog_speed, conn_guard.active_cartesian_jog_step,
                conn_guard.active_joint_jog_speed, conn_guard.active_joint_jog_step);

            // Get the frame/tool from active configuration
            let uframe = conn_guard.active_configuration.u_frame_number as u8;
            let utool = conn_guard.active_configuration.u_tool_number as u8;

            // Send FrcSetUFrameUTool to set the robot to the configured frame/tool
            if let Some(ref driver) = conn_guard.driver {
                let cmd = FrcSetUFrameUTool::new(None, utool, uframe);
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
                                info!("Set robot to UFrame={}, UTool={}", uframe, utool);
                            }
                        }
                        Ok(None) => warn!("No response to FrcSetUFrameUTool"),
                        Err(_) => warn!("Timeout waiting for FrcSetUFrameUTool response"),
                    }
                }
            }

            // Broadcast ActiveFrameTool, ActiveConfiguration, and ConnectionStatus to all clients
            if let Some(ref client_manager) = client_manager {
                let frame_tool_response = ServerResponse::ActiveFrameTool {
                    uframe,
                    utool,
                };
                client_manager.broadcast_all(&frame_tool_response).await;

                // Also broadcast the full active configuration
                let config = &conn_guard.active_configuration;
                let config_response = ServerResponse::ActiveConfigurationResponse {
                    loaded_from_id: config.loaded_from_id,
                    loaded_from_name: config.loaded_from_name.clone(),
                    changes_count: config.changes_count,
                    change_log: config.change_log.iter().map(|entry| crate::api_types::ChangeLogEntryDto {
                        field_name: entry.field_name.clone(),
                        old_value: entry.old_value.clone(),
                        new_value: entry.new_value.clone(),
                    }).collect(),
                    u_frame_number: config.u_frame_number,
                    u_tool_number: config.u_tool_number,
                    front: config.front,
                    up: config.up,
                    left: config.left,
                    flip: config.flip,
                    turn4: config.turn4,
                    turn5: config.turn5,
                    turn6: config.turn6,
                    default_cartesian_jog_speed: config.default_cartesian_jog_speed,
                    default_cartesian_jog_step: config.default_cartesian_jog_step,
                    default_joint_jog_speed: config.default_joint_jog_speed,
                    default_joint_jog_step: config.default_joint_jog_step,
                };
                client_manager.broadcast_all(&config_response).await;

                // Broadcast active jog settings
                let jog_response = ServerResponse::ActiveJogSettings {
                    cartesian_jog_speed: conn_guard.active_cartesian_jog_speed,
                    cartesian_jog_step: conn_guard.active_cartesian_jog_step,
                    joint_jog_speed: conn_guard.active_joint_jog_speed,
                    joint_jog_step: conn_guard.active_joint_jog_step,
                };
                client_manager.broadcast_all(&jog_response).await;

                // Broadcast connection status with tp_program_initialized flag
                let status_response = ServerResponse::ConnectionStatus {
                    connected: conn_guard.connected,
                    robot_addr: conn_guard.robot_addr.clone(),
                    robot_port: conn_guard.robot_port,
                    connection_name: Some(saved_conn.name.clone()),
                    connection_id: Some(saved_conn.id),
                    tp_program_initialized: conn_guard.tp_program_initialized,
                };
                client_manager.broadcast_all(&status_response).await;
            }

            // Store the saved connection for configuration defaults
            conn_guard.saved_connection = Some(saved_conn.clone());

            ServerResponse::RobotConnected {
                connection_id: saved_conn.id,
                connection_name: saved_conn.name.clone(),
                robot_addr: saved_conn.ip_address.clone(),
                robot_port: saved_conn.port,
                effective_speed: saved_conn.default_speed,
                effective_term_type: saved_conn.default_term_type,
                effective_uframe: conn_guard.active_configuration.u_frame_number,
                effective_utool: conn_guard.active_configuration.u_tool_number,
                effective_w: saved_conn.default_w,
                effective_p: saved_conn.default_p,
                effective_r: saved_conn.default_r,
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

