//! Robot configuration handlers.
//!
//! Handles CRUD operations for named robot configurations and active configuration state.

use crate::api_types::{RobotConfigurationDto, ServerResponse};
use crate::database::Database;
use crate::session::ClientManager;
use crate::RobotConnection;
use fanuc_rmi::commands::FrcSetUFrameUTool;
use fanuc_rmi::packets::{Command, CommandResponse, PacketPriority, ResponsePacket, SendPacket};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, RwLock};
use tracing::warn;

/// Convert database RobotConfiguration to DTO.
fn to_dto(config: &crate::database::RobotConfiguration) -> RobotConfigurationDto {
    RobotConfigurationDto {
        id: config.id,
        robot_connection_id: config.robot_connection_id,
        name: config.name.clone(),
        is_default: config.is_default,
        u_frame_number: config.u_frame_number,
        u_tool_number: config.u_tool_number,
        front: config.front,
        up: config.up,
        left: config.left,
        flip: config.flip,
        turn4: config.turn4,
        turn5: config.turn5,
        turn6: config.turn6,
    }
}

/// List all configurations for a robot.
pub async fn list_robot_configurations(
    db: Arc<Mutex<Database>>,
    robot_connection_id: i64,
) -> ServerResponse {
    let db = db.lock().await;
    match db.list_robot_configurations(robot_connection_id) {
        Ok(configs) => ServerResponse::RobotConfigurations {
            configurations: configs.iter().map(to_dto).collect(),
        },
        Err(e) => ServerResponse::Error {
            message: format!("Failed to list configurations: {}", e),
        },
    }
}

/// Get a single configuration by ID.
pub async fn get_robot_configuration(db: Arc<Mutex<Database>>, id: i64) -> ServerResponse {
    let db = db.lock().await;
    match db.get_robot_configuration(id) {
        Ok(Some(config)) => ServerResponse::RobotConfigurationResponse {
            configuration: to_dto(&config),
        },
        Ok(None) => ServerResponse::Error {
            message: "Configuration not found".to_string(),
        },
        Err(e) => ServerResponse::Error {
            message: format!("Failed to get configuration: {}", e),
        },
    }
}

/// Create a new configuration.
#[allow(clippy::too_many_arguments)]
pub async fn create_robot_configuration(
    db: Arc<Mutex<Database>>,
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
) -> ServerResponse {
    let db = db.lock().await;
    match db.create_robot_configuration(
        robot_connection_id,
        &name,
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
    ) {
        Ok(id) => {
            // Fetch the created configuration to return it
            match db.get_robot_configuration(id) {
                Ok(Some(config)) => ServerResponse::RobotConfigurationResponse {
                    configuration: to_dto(&config),
                },
                _ => ServerResponse::Success {
                    message: format!("Configuration created with id {}", id),
                },
            }
        }
        Err(e) => ServerResponse::Error {
            message: format!("Failed to create configuration: {}", e),
        },
    }
}

/// Update an existing configuration.
#[allow(clippy::too_many_arguments)]
pub async fn update_robot_configuration(
    db: Arc<Mutex<Database>>,
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
) -> ServerResponse {
    let db = db.lock().await;
    match db.update_robot_configuration(
        id, &name, is_default, u_frame_number, u_tool_number,
        front, up, left, flip, turn4, turn5, turn6,
    ) {
        Ok(()) => ServerResponse::Success {
            message: "Configuration updated".to_string(),
        },
        Err(e) => ServerResponse::Error {
            message: format!("Failed to update configuration: {}", e),
        },
    }
}

/// Delete a configuration.
pub async fn delete_robot_configuration(db: Arc<Mutex<Database>>, id: i64) -> ServerResponse {
    let db = db.lock().await;
    match db.delete_robot_configuration(id) {
        Ok(()) => ServerResponse::Success {
            message: "Configuration deleted".to_string(),
        },
        Err(e) => ServerResponse::Error {
            message: format!("Failed to delete configuration: {}", e),
        },
    }
}

/// Set a configuration as the default for its robot.
pub async fn set_default_robot_configuration(db: Arc<Mutex<Database>>, id: i64) -> ServerResponse {
    let db = db.lock().await;
    match db.set_default_robot_configuration(id) {
        Ok(()) => ServerResponse::Success {
            message: "Default configuration set".to_string(),
        },
        Err(e) => ServerResponse::Error {
            message: format!("Failed to set default configuration: {}", e),
        },
    }
}

/// Get the current active configuration state.
pub async fn get_active_configuration(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
) -> ServerResponse {
    let Some(conn) = robot_connection else {
        return ServerResponse::Error {
            message: "Not connected to robot".to_string(),
        };
    };

    let conn = conn.read().await;
    let config = &conn.active_configuration;

    ServerResponse::ActiveConfigurationResponse {
        loaded_from_id: config.loaded_from_id,
        loaded_from_name: config.loaded_from_name.clone(),
        modified: config.modified,
        u_frame_number: config.u_frame_number,
        u_tool_number: config.u_tool_number,
        front: config.front,
        up: config.up,
        left: config.left,
        flip: config.flip,
        turn4: config.turn4,
        turn5: config.turn5,
        turn6: config.turn6,
    }
}

/// Load a saved configuration as the active configuration.
/// This also sends FrcSetUFrameUTool to the robot and broadcasts to all clients.
pub async fn load_configuration(
    db: Arc<Mutex<Database>>,
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    client_manager: Option<Arc<ClientManager>>,
    configuration_id: i64,
) -> ServerResponse {
    let Some(conn) = robot_connection else {
        return ServerResponse::Error {
            message: "Not connected to robot".to_string(),
        };
    };

    // Get the configuration from database
    let config = {
        let db = db.lock().await;
        match db.get_robot_configuration(configuration_id) {
            Ok(Some(c)) => c,
            Ok(None) => {
                return ServerResponse::Error {
                    message: "Configuration not found".to_string(),
                }
            }
            Err(e) => {
                return ServerResponse::Error {
                    message: format!("Failed to get configuration: {}", e),
                }
            }
        }
    };

    let uframe = config.u_frame_number as u8;
    let utool = config.u_tool_number as u8;

    // Update the active configuration and send command to robot
    let mut conn_guard = conn.write().await;
    conn_guard.active_configuration = crate::ActiveConfiguration::from_saved(&config);

    // Send FrcSetUFrameUTool to robot
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
                    }
                }
                Ok(None) => warn!("No response to FrcSetUFrameUTool"),
                Err(_) => warn!("Timeout waiting for FrcSetUFrameUTool response"),
            }
        }
    }

    // Broadcast to all clients
    if let Some(ref client_manager) = client_manager {
        let frame_tool_response = ServerResponse::ActiveFrameTool { uframe, utool };
        client_manager.broadcast_all(&frame_tool_response).await;

        let config_response = ServerResponse::ActiveConfigurationResponse {
            loaded_from_id: Some(config.id),
            loaded_from_name: Some(config.name.clone()),
            modified: false,
            u_frame_number: config.u_frame_number,
            u_tool_number: config.u_tool_number,
            front: config.front,
            up: config.up,
            left: config.left,
            flip: config.flip,
            turn4: config.turn4,
            turn5: config.turn5,
            turn6: config.turn6,
        };
        client_manager.broadcast_all(&config_response).await;
    }

    ServerResponse::ActiveConfigurationResponse {
        loaded_from_id: Some(config.id),
        loaded_from_name: Some(config.name),
        modified: false,
        u_frame_number: config.u_frame_number,
        u_tool_number: config.u_tool_number,
        front: config.front,
        up: config.up,
        left: config.left,
        flip: config.flip,
        turn4: config.turn4,
        turn5: config.turn5,
        turn6: config.turn6,
    }
}

