//! Robot connection handlers.
//!
//! Handles connecting, disconnecting, and checking status of the robot connection.

use crate::api_types::ServerResponse;
use crate::RobotConnection;
use std::sync::Arc;
use tokio::sync::RwLock;
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

