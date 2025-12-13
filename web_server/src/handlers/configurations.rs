//! Robot configuration handlers.
//!
//! Handles CRUD operations for named robot configurations and active configuration state.

use crate::api_types::{ChangeLogEntryDto, RobotConfigurationDto, ServerResponse};
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

/// Convert changelog entries to DTOs.
fn changelog_to_dto(changelog: &[crate::ChangeLogEntry]) -> Vec<ChangeLogEntryDto> {
    changelog
        .iter()
        .map(|entry| ChangeLogEntryDto {
            field_name: entry.field_name.clone(),
            old_value: entry.old_value.clone(),
            new_value: entry.new_value.clone(),
        })
        .collect()
}

/// List all configurations for a robot.
pub async fn list_robot_configurations(
    db: Arc<Mutex<Database>>,
    robot_connection_id: i64,
) -> ServerResponse {
    let db = db.lock().await;
    match db.list_robot_configurations(robot_connection_id) {
        Ok(configs) => ServerResponse::RobotConfigurationList {
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
        changes_count: config.changes_count,
        change_log: changelog_to_dto(&config.change_log),
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

    // Get the saved connection for jog defaults
    let saved_conn = conn_guard.saved_connection.as_ref().ok_or_else(|| {
        ServerResponse::Error {
            message: "No saved connection found".to_string(),
        }
    });
    let saved_conn = match saved_conn {
        Ok(c) => c,
        Err(e) => return e,
    };

    conn_guard.active_configuration = crate::ActiveConfiguration::from_saved(&config, saved_conn);

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

    // Get the active configuration for broadcasting
    let active_config = &conn_guard.active_configuration;

    // Broadcast to all clients
    if let Some(ref client_manager) = client_manager {
        let frame_tool_response = ServerResponse::ActiveFrameTool { uframe, utool };
        client_manager.broadcast_all(&frame_tool_response).await;

        let config_response = ServerResponse::ActiveConfigurationResponse {
            loaded_from_id: active_config.loaded_from_id,
            loaded_from_name: active_config.loaded_from_name.clone(),
            changes_count: active_config.changes_count,
            change_log: changelog_to_dto(&active_config.change_log),
            u_frame_number: active_config.u_frame_number,
            u_tool_number: active_config.u_tool_number,
            front: active_config.front,
            up: active_config.up,
            left: active_config.left,
            flip: active_config.flip,
            turn4: active_config.turn4,
            turn5: active_config.turn5,
            turn6: active_config.turn6,
            default_cartesian_jog_speed: active_config.default_cartesian_jog_speed,
            default_cartesian_jog_step: active_config.default_cartesian_jog_step,
            default_joint_jog_speed: active_config.default_joint_jog_speed,
            default_joint_jog_step: active_config.default_joint_jog_step,
        };
        client_manager.broadcast_all(&config_response).await;
    }

    ServerResponse::ActiveConfigurationResponse {
        loaded_from_id: active_config.loaded_from_id,
        loaded_from_name: active_config.loaded_from_name.clone(),
        changes_count: active_config.changes_count,
        change_log: changelog_to_dto(&active_config.change_log),
        u_frame_number: active_config.u_frame_number,
        u_tool_number: active_config.u_tool_number,
        front: active_config.front,
        up: active_config.up,
        left: active_config.left,
        flip: active_config.flip,
        turn4: active_config.turn4,
        turn5: active_config.turn5,
        turn6: active_config.turn6,
        default_cartesian_jog_speed: active_config.default_cartesian_jog_speed,
        default_cartesian_jog_step: active_config.default_cartesian_jog_step,
        default_joint_jog_speed: active_config.default_joint_jog_speed,
        default_joint_jog_step: active_config.default_joint_jog_step,
    }
}

/// Save current configuration (active frame/tool/arm config + active jog settings) to database.
/// If configuration_name is provided, creates a new configuration.
/// Otherwise, updates the currently loaded configuration or creates a new one if none is loaded.
/// Also saves active jog settings to robot_connections table.
/// Resets changes_count to 0.
pub async fn save_current_configuration(
    db: Arc<Mutex<Database>>,
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    client_manager: Option<Arc<ClientManager>>,
    configuration_name: Option<String>,
) -> ServerResponse {
    let Some(conn) = robot_connection else {
        return ServerResponse::Error {
            message: "Not connected to robot".to_string(),
        };
    };

    let mut conn = conn.write().await;

    // Get the robot connection ID
    let robot_connection_id = match &conn.saved_connection {
        Some(saved_conn) => saved_conn.id,
        None => {
            return ServerResponse::Error {
                message: "No saved robot connection found".to_string(),
            };
        }
    };

    // Clone config and jog values we'll need later
    let u_frame_number = conn.active_configuration.u_frame_number;
    let u_tool_number = conn.active_configuration.u_tool_number;
    let front = conn.active_configuration.front;
    let up = conn.active_configuration.up;
    let left = conn.active_configuration.left;
    let flip = conn.active_configuration.flip;
    let turn4 = conn.active_configuration.turn4;
    let turn5 = conn.active_configuration.turn5;
    let turn6 = conn.active_configuration.turn6;
    // Save the DEFAULT jog settings (from active_configuration), not the active jog controls
    let default_cartesian_jog_speed = conn.active_configuration.default_cartesian_jog_speed;
    let default_cartesian_jog_step = conn.active_configuration.default_cartesian_jog_step;
    let default_joint_jog_speed = conn.active_configuration.default_joint_jog_speed;
    let default_joint_jog_step = conn.active_configuration.default_joint_jog_step;

    // Determine if we're creating a new configuration or updating existing
    let (config_id, config_name) = if let Some(name) = configuration_name {
        // Create new configuration with provided name
        (None, name)
    } else if let (Some(id), Some(name)) = (conn.active_configuration.loaded_from_id, &conn.active_configuration.loaded_from_name) {
        // Update existing loaded configuration
        (Some(id), name.clone())
    } else {
        // No configuration loaded and no name provided - error
        return ServerResponse::Error {
            message: "No configuration name provided and no configuration currently loaded".to_string(),
        };
    };

    // Save robot configuration to database
    let db_guard = db.lock().await;
    let saved_config_id = if let Some(id) = config_id {
        // Update existing configuration - preserve is_default flag
        // First, get the current is_default value
        let current_is_default = match db_guard.get_robot_configuration(id) {
            Ok(Some(config)) => config.is_default,
            Ok(None) => {
                return ServerResponse::Error {
                    message: "Configuration not found".to_string(),
                };
            }
            Err(e) => {
                return ServerResponse::Error {
                    message: format!("Failed to get configuration: {}", e),
                };
            }
        };

        // Update with preserved is_default flag
        if let Err(e) = db_guard.update_robot_configuration(
            id,
            &config_name,
            current_is_default, // Preserve the current is_default value
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
            return ServerResponse::Error {
                message: format!("Failed to update configuration: {}", e),
            };
        }
        id
    } else {
        // Create new configuration
        match db_guard.create_robot_configuration(
            robot_connection_id,
            &config_name,
            false, // is_default
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
            Ok(id) => id,
            Err(e) => {
                return ServerResponse::Error {
                    message: format!("Failed to create configuration: {}", e),
                };
            }
        }
    };

    // Save default jog settings to robot_connections table
    if let Err(e) = db_guard.update_robot_connection_jog_defaults(
        robot_connection_id,
        default_cartesian_jog_speed,
        default_cartesian_jog_step,
        default_joint_jog_speed,
        default_joint_jog_step,
    ) {
        return ServerResponse::Error {
            message: format!("Failed to save jog settings: {}", e),
        };
    }

    // Update active configuration state
    conn.active_configuration.loaded_from_id = Some(saved_config_id);
    conn.active_configuration.loaded_from_name = Some(config_name.clone());
    conn.active_configuration.changes_count = 0; // Reset counter

    // Update saved_connection with new jog defaults
    if let Some(ref mut saved_conn) = conn.saved_connection {
        saved_conn.default_cartesian_jog_speed = default_cartesian_jog_speed;
        saved_conn.default_cartesian_jog_step = default_cartesian_jog_step;
        saved_conn.default_joint_jog_speed = default_joint_jog_speed;
        saved_conn.default_joint_jog_step = default_joint_jog_step;
    }

    drop(db_guard);

    // Broadcast updated configuration to all clients
    if let Some(ref client_manager) = client_manager {
        let config_response = ServerResponse::ActiveConfigurationResponse {
            loaded_from_id: Some(saved_config_id),
            loaded_from_name: Some(config_name.clone()),
            changes_count: 0,
            change_log: Vec::new(),  // Clear changelog after saving
            u_frame_number,
            u_tool_number,
            front,
            up,
            left,
            flip,
            turn4,
            turn5,
            turn6,
            default_cartesian_jog_speed,
            default_cartesian_jog_step,
            default_joint_jog_speed,
            default_joint_jog_step,
        };
        client_manager.broadcast_all(&config_response).await;
    }

    ServerResponse::Success {
        message: format!("Configuration '{}' saved successfully", config_name),
    }
}
