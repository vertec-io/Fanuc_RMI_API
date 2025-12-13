//! Saved robot connections handlers.
//!
//! Handles CRUD operations for saved robot connection configurations.

use crate::api_types::*;
use crate::database::Database;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::info;

/// List all saved robot connections.
pub async fn list_robot_connections(db: Arc<Mutex<Database>>) -> ServerResponse {
    let db = db.lock().await;
    match db.list_robot_connections() {
        Ok(connections) => {
            let connections: Vec<RobotConnectionDto> = connections.iter().map(|c| RobotConnectionDto {
                id: c.id,
                name: c.name.clone(),
                description: c.description.clone(),
                ip_address: c.ip_address.clone(),
                port: c.port,
                default_speed: c.default_speed,
                default_speed_type: c.default_speed_type.clone(),
                default_term_type: c.default_term_type.clone(),
                default_w: c.default_w,
                default_p: c.default_p,
                default_r: c.default_r,
                default_cartesian_jog_speed: c.default_cartesian_jog_speed,
                default_cartesian_jog_step: c.default_cartesian_jog_step,
                default_joint_jog_speed: c.default_joint_jog_speed,
                default_joint_jog_step: c.default_joint_jog_step,
            }).collect();
            ServerResponse::RobotConnections { connections }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to list connections: {}", e) }
    }
}

/// Get a saved robot connection by ID.
pub async fn get_robot_connection(db: Arc<Mutex<Database>>, id: i64) -> ServerResponse {
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
                    default_speed: c.default_speed,
                    default_speed_type: c.default_speed_type,
                    default_term_type: c.default_term_type,
                    default_w: c.default_w,
                    default_p: c.default_p,
                    default_r: c.default_r,
                    default_cartesian_jog_speed: c.default_cartesian_jog_speed,
                    default_cartesian_jog_step: c.default_cartesian_jog_step,
                    default_joint_jog_speed: c.default_joint_jog_speed,
                    default_joint_jog_step: c.default_joint_jog_step,
                }
            }
        }
        Ok(None) => ServerResponse::Error { message: "Connection not found".to_string() },
        Err(e) => ServerResponse::Error { message: format!("Failed to get connection: {}", e) }
    }
}

/// Create a new saved robot connection (DEPRECATED - use create_robot_with_configurations instead).
/// This creates a robot connection with default values but NO configurations.
/// The robot will not be connectable until at least one configuration is created.
pub async fn create_robot_connection(
    db: Arc<Mutex<Database>>,
    name: &str,
    description: Option<&str>,
    ip_address: &str,
    port: u32,
) -> ServerResponse {
    let db = db.lock().await;
    // Use sensible defaults for motion and jog parameters
    match db.create_robot_connection(
        name,
        description,
        ip_address,
        port,
        100.0,     // default_speed
        "mmSec",   // default_speed_type
        "CNT",     // default_term_type
        0.0,       // default_w
        0.0,       // default_p
        0.0,       // default_r
        10.0,      // default_cartesian_jog_speed (safer default)
        1.0,       // default_cartesian_jog_step (safer default)
        0.1,       // default_joint_jog_speed (safer default)
        0.25,      // default_joint_jog_step (safer default)
    ) {
        Ok(id) => {
            info!("Created robot connection: {} (id={})", name, id);
            ServerResponse::Success { message: format!("Connection '{}' created", name) }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to create connection: {}", e) }
    }
}

/// Update a saved robot connection.
pub async fn update_robot_connection(
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

/// Update robot connection motion defaults.
/// Motion parameters (speed, speed_type, term_type, w/p/r) only.
/// Frame/tool/arm config is managed via robot_configurations table.
pub async fn update_robot_connection_defaults(
    db: Arc<Mutex<Database>>,
    id: i64,
    default_speed: f64,
    default_speed_type: &str,
    default_term_type: &str,
    default_w: f64,
    default_p: f64,
    default_r: f64,
) -> ServerResponse {
    let db = db.lock().await;
    match db.update_robot_connection_defaults(
        id,
        default_speed,
        default_speed_type,
        default_term_type,
        default_w,
        default_p,
        default_r,
    ) {
        Ok(_) => {
            info!("Updated robot connection motion defaults for id={}", id);
            ServerResponse::Success { message: "Connection defaults updated".to_string() }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to update connection defaults: {}", e) }
    }
}

/// Update robot connection jog defaults (saves to database).
pub async fn update_robot_jog_defaults(
    db: Arc<Mutex<Database>>,
    id: i64,
    cartesian_jog_speed: f64,
    cartesian_jog_step: f64,
    joint_jog_speed: f64,
    joint_jog_step: f64,
) -> ServerResponse {
    let db = db.lock().await;
    match db.update_robot_connection_jog_defaults(id, cartesian_jog_speed, cartesian_jog_step, joint_jog_speed, joint_jog_step) {
        Ok(_) => {
            info!("Updated robot jog defaults for id={}", id);
            ServerResponse::Success { message: "Jog defaults updated".to_string() }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to update jog defaults: {}", e) }
    }
}

/// Update jog controls (from Control panel - updates active jog controls only, does NOT update defaults or increment changes_count).
/// This is called when the user changes jog settings from the jog controls in the Control tab.
pub async fn update_jog_controls(
    robot_connection: Option<Arc<RwLock<crate::RobotConnection>>>,
    client_manager: Option<Arc<crate::session::ClientManager>>,
    cartesian_jog_speed: f64,
    cartesian_jog_step: f64,
    joint_jog_speed: f64,
    joint_jog_step: f64,
) -> ServerResponse {
    let Some(conn) = robot_connection else {
        return ServerResponse::Error {
            message: "Not connected to robot".to_string(),
        };
    };

    let mut conn = conn.write().await;

    // Update active jog controls (NOT the defaults)
    conn.active_cartesian_jog_speed = cartesian_jog_speed;
    conn.active_cartesian_jog_step = cartesian_jog_step;
    conn.active_joint_jog_speed = joint_jog_speed;
    conn.active_joint_jog_step = joint_jog_step;

    info!("Updated jog controls: cart_speed={}, cart_step={}, joint_speed={}, joint_step={}",
        cartesian_jog_speed, cartesian_jog_step, joint_jog_speed, joint_jog_step);

    // Broadcast active jog settings to all clients
    if let Some(ref client_manager) = client_manager {
        let jog_response = ServerResponse::ActiveJogSettings {
            cartesian_jog_speed,
            cartesian_jog_step,
            joint_jog_speed,
            joint_jog_step,
        };
        client_manager.broadcast_all(&jog_response).await;
    }

    ServerResponse::Success {
        message: "Jog controls updated".to_string(),
    }
}

/// Apply jog defaults (from Configuration panel - updates active defaults AND active jog controls, increments changes_count).
/// This is called when the user clicks "Apply" in the Jog Defaults panel in the Configuration tab.
/// Does NOT save to database - use SaveCurrentConfiguration to persist changes.
pub async fn apply_jog_settings(
    robot_connection: Option<Arc<RwLock<crate::RobotConnection>>>,
    client_manager: Option<Arc<crate::session::ClientManager>>,
    cartesian_jog_speed: f64,
    cartesian_jog_step: f64,
    joint_jog_speed: f64,
    joint_jog_step: f64,
) -> ServerResponse {
    let Some(conn) = robot_connection else {
        return ServerResponse::Error {
            message: "Not connected to robot".to_string(),
        };
    };

    let mut conn = conn.write().await;

    // Capture old values from active defaults before updating
    let old_cart_speed = conn.active_configuration.default_cartesian_jog_speed;
    let old_cart_step = conn.active_configuration.default_cartesian_jog_step;
    let old_joint_speed = conn.active_configuration.default_joint_jog_speed;
    let old_joint_step = conn.active_configuration.default_joint_jog_step;

    // Track changes to changelog
    if old_cart_speed != cartesian_jog_speed {
        conn.active_configuration.change_log.push(crate::ChangeLogEntry {
            field_name: "Cartesian Jog Speed".to_string(),
            old_value: format!("{:.1}", old_cart_speed),
            new_value: format!("{:.1}", cartesian_jog_speed),
        });
    }
    if old_cart_step != cartesian_jog_step {
        conn.active_configuration.change_log.push(crate::ChangeLogEntry {
            field_name: "Cartesian Jog Step".to_string(),
            old_value: format!("{:.1}", old_cart_step),
            new_value: format!("{:.1}", cartesian_jog_step),
        });
    }
    if old_joint_speed != joint_jog_speed {
        conn.active_configuration.change_log.push(crate::ChangeLogEntry {
            field_name: "Joint Jog Speed".to_string(),
            old_value: format!("{:.1}", old_joint_speed),
            new_value: format!("{:.1}", joint_jog_speed),
        });
    }
    if old_joint_step != joint_jog_step {
        conn.active_configuration.change_log.push(crate::ChangeLogEntry {
            field_name: "Joint Jog Step".to_string(),
            old_value: format!("{:.1}", old_joint_step),
            new_value: format!("{:.1}", joint_jog_step),
        });
    }

    // Update active defaults (in active_configuration)
    conn.active_configuration.default_cartesian_jog_speed = cartesian_jog_speed;
    conn.active_configuration.default_cartesian_jog_step = cartesian_jog_step;
    conn.active_configuration.default_joint_jog_speed = joint_jog_speed;
    conn.active_configuration.default_joint_jog_step = joint_jog_step;

    // Also update active jog controls (so they match the new defaults)
    conn.active_cartesian_jog_speed = cartesian_jog_speed;
    conn.active_cartesian_jog_step = cartesian_jog_step;
    conn.active_joint_jog_speed = joint_jog_speed;
    conn.active_joint_jog_step = joint_jog_step;

    // Increment changes counter
    conn.active_configuration.changes_count += 1;

    info!("Applied jog defaults: cart_speed={}, cart_step={}, joint_speed={}, joint_step={}, changes_count={}",
        cartesian_jog_speed, cartesian_jog_step, joint_jog_speed, joint_jog_step, conn.active_configuration.changes_count);

    // Broadcast active jog settings to all clients
    if let Some(ref client_manager) = client_manager {
        let jog_response = ServerResponse::ActiveJogSettings {
            cartesian_jog_speed,
            cartesian_jog_step,
            joint_jog_speed,
            joint_jog_step,
        };
        client_manager.broadcast_all(&jog_response).await;

        // Also broadcast updated configuration with new changes_count and defaults
        let config = &conn.active_configuration;
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
    }

    ServerResponse::Success {
        message: "Jog defaults applied".to_string(),
    }
}

/// Delete a saved robot connection.
pub async fn delete_robot_connection(db: Arc<Mutex<Database>>, id: i64) -> ServerResponse {
    let db = db.lock().await;
    match db.delete_robot_connection(id) {
        Ok(_) => {
            info!("Deleted robot connection id={}", id);
            ServerResponse::Success { message: "Connection deleted".to_string() }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to delete connection: {}", e) }
    }
}

/// Create robot connection with configurations atomically.
/// This ensures at least one default configuration exists.
#[allow(clippy::too_many_arguments)]
pub async fn create_robot_with_configurations(
    db: Arc<Mutex<Database>>,
    name: &str,
    description: Option<&str>,
    ip_address: &str,
    port: u32,
    default_speed: f64,
    default_speed_type: &str,
    default_term_type: &str,
    default_w: f64,
    default_p: f64,
    default_r: f64,
    default_cartesian_jog_speed: f64,
    default_cartesian_jog_step: f64,
    default_joint_jog_speed: f64,
    default_joint_jog_step: f64,
    configurations: Vec<NewRobotConfigurationDto>,
) -> ServerResponse {
    // Validate: at least one configuration required
    if configurations.is_empty() {
        return ServerResponse::Error {
            message: "At least one configuration is required".to_string(),
        };
    }

    // Validate: exactly one default configuration
    let default_count = configurations.iter().filter(|c| c.is_default).count();
    if default_count == 0 {
        return ServerResponse::Error {
            message: "At least one configuration must be marked as default".to_string(),
        };
    }
    if default_count > 1 {
        return ServerResponse::Error {
            message: "Only one configuration can be marked as default".to_string(),
        };
    }

    let db = db.lock().await;
    let config_count = configurations.len();

    // Create robot connection
    let robot_id = match db.create_robot_connection(
        name,
        description,
        ip_address,
        port,
        default_speed,
        default_speed_type,
        default_term_type,
        default_w,
        default_p,
        default_r,
        default_cartesian_jog_speed,
        default_cartesian_jog_step,
        default_joint_jog_speed,
        default_joint_jog_step,
    ) {
        Ok(id) => id,
        Err(e) => {
            return ServerResponse::Error {
                message: format!("Failed to create robot connection: {}", e),
            }
        }
    };

    // Create all configurations
    for config in configurations {
        if let Err(e) = db.create_robot_configuration(
            robot_id,
            &config.name,
            config.is_default,
            config.u_frame_number,
            config.u_tool_number,
            config.front,
            config.up,
            config.left,
            config.flip,
            config.turn4,
            config.turn5,
            config.turn6,
        ) {
            // If configuration creation fails, delete the robot connection
            let _ = db.delete_robot_connection(robot_id);
            return ServerResponse::Error {
                message: format!("Failed to create configuration: {}", e),
            };
        }
    }

    info!("Created robot connection '{}' (id={}) with {} configurations", name, robot_id, config_count);

    // Fetch the created robot connection and configurations
    let connection = match db.get_robot_connection(robot_id) {
        Ok(Some(c)) => c,
        _ => {
            return ServerResponse::Error {
                message: "Failed to fetch created robot connection".to_string(),
            }
        }
    };

    let configurations = match db.list_robot_configurations(robot_id) {
        Ok(configs) => configs.into_iter().map(|cfg| RobotConfigurationDto {
            id: cfg.id,
            robot_connection_id: cfg.robot_connection_id,
            name: cfg.name,
            is_default: cfg.is_default,
            u_frame_number: cfg.u_frame_number,
            u_tool_number: cfg.u_tool_number,
            front: cfg.front,
            up: cfg.up,
            left: cfg.left,
            flip: cfg.flip,
            turn4: cfg.turn4,
            turn5: cfg.turn5,
            turn6: cfg.turn6,
        }).collect(),
        Err(e) => {
            return ServerResponse::Error {
                message: format!("Failed to fetch configurations: {}", e),
            }
        }
    };

    ServerResponse::RobotConnectionCreated {
        id: robot_id,
        connection: RobotConnectionDto {
            id: connection.id,
            name: connection.name,
            description: connection.description,
            ip_address: connection.ip_address,
            port: connection.port,
            default_speed: connection.default_speed,
            default_speed_type: connection.default_speed_type,
            default_term_type: connection.default_term_type,
            default_w: connection.default_w,
            default_p: connection.default_p,
            default_r: connection.default_r,
            default_cartesian_jog_speed: connection.default_cartesian_jog_speed,
            default_cartesian_jog_step: connection.default_cartesian_jog_step,
            default_joint_jog_speed: connection.default_joint_jog_speed,
            default_joint_jog_step: connection.default_joint_jog_step,
        },
        configurations,
    }
}
