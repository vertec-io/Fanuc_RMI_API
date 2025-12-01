//! Robot connection handlers.
//!
//! Handles connecting, disconnecting, and checking status of the robot connection.

use crate::api_types::ServerResponse;
use crate::database::Database;
use crate::RobotConnection;
use std::sync::Arc;
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
        info!("Disconnected from robot");
        ServerResponse::Success { message: "Disconnected from robot".to_string() }
    } else {
        ServerResponse::Error { message: "Robot connection manager not available".to_string() }
    }
}

/// Connect to a saved robot connection by ID.
/// Returns the effective settings (per-robot defaults or global fallback).
pub async fn connect_to_saved_robot(
    db: Arc<Mutex<Database>>,
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
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
    if let Some(conn) = robot_connection {
        let mut conn = conn.write().await;
        conn.robot_addr = saved_conn.ip_address.clone();
        conn.robot_port = saved_conn.port;

        match conn.connect().await {
            Ok(()) => {
                info!("Successfully connected to saved robot '{}' at {}:{}",
                    saved_conn.name, saved_conn.ip_address, saved_conn.port);

                // Calculate effective settings (per-robot or global fallback)
                let effective_speed = saved_conn.default_speed.unwrap_or(global_settings.default_speed);
                let effective_term_type = saved_conn.default_term_type.clone()
                    .unwrap_or(global_settings.default_term_type.clone());
                let effective_uframe = saved_conn.default_uframe.unwrap_or(global_settings.default_uframe);
                let effective_utool = saved_conn.default_utool.unwrap_or(global_settings.default_utool);
                let effective_w = saved_conn.default_w.unwrap_or(global_settings.default_w);
                let effective_p = saved_conn.default_p.unwrap_or(global_settings.default_p);
                let effective_r = saved_conn.default_r.unwrap_or(global_settings.default_r);

                ServerResponse::RobotConnected {
                    connection_id: saved_conn.id,
                    connection_name: saved_conn.name,
                    robot_addr: saved_conn.ip_address,
                    robot_port: saved_conn.port,
                    effective_speed,
                    effective_term_type,
                    effective_uframe,
                    effective_utool,
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
    } else {
        ServerResponse::Error { message: "Robot connection manager not available".to_string() }
    }
}

