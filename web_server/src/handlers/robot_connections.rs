//! Saved robot connections handlers.
//!
//! Handles CRUD operations for saved robot connection configurations.

use crate::api_types::*;
use crate::database::Database;
use std::sync::Arc;
use tokio::sync::Mutex;
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
                default_term_type: c.default_term_type.clone(),
                default_uframe: c.default_uframe,
                default_utool: c.default_utool,
                default_w: c.default_w,
                default_p: c.default_p,
                default_r: c.default_r,
                default_front: c.default_front,
                default_up: c.default_up,
                default_left: c.default_left,
                default_flip: c.default_flip,
                default_turn4: c.default_turn4,
                default_turn5: c.default_turn5,
                default_turn6: c.default_turn6,
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
                    default_term_type: c.default_term_type,
                    default_uframe: c.default_uframe,
                    default_utool: c.default_utool,
                    default_w: c.default_w,
                    default_p: c.default_p,
                    default_r: c.default_r,
                    default_front: c.default_front,
                    default_up: c.default_up,
                    default_left: c.default_left,
                    default_flip: c.default_flip,
                    default_turn4: c.default_turn4,
                    default_turn5: c.default_turn5,
                    default_turn6: c.default_turn6,
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

/// Create a new saved robot connection.
pub async fn create_robot_connection(
    db: Arc<Mutex<Database>>,
    name: &str,
    description: Option<&str>,
    ip_address: &str,
    port: u32,
) -> ServerResponse {
    let db = db.lock().await;
    match db.create_robot_connection(name, description, ip_address, port) {
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

/// Update robot connection defaults (per-robot settings).
/// All parameters are required (non-optional) - each robot has explicit settings.
#[allow(clippy::too_many_arguments)]
pub async fn update_robot_connection_defaults(
    db: Arc<Mutex<Database>>,
    id: i64,
    default_speed: f64,
    default_term_type: &str,
    default_uframe: i32,
    default_utool: i32,
    default_w: f64,
    default_p: f64,
    default_r: f64,
    default_front: i32,
    default_up: i32,
    default_left: i32,
    default_flip: i32,
    default_turn4: i32,
    default_turn5: i32,
    default_turn6: i32,
) -> ServerResponse {
    let db = db.lock().await;
    match db.update_robot_connection_defaults(
        id,
        default_speed,
        default_term_type,
        default_uframe,
        default_utool,
        default_w,
        default_p,
        default_r,
        default_front,
        default_up,
        default_left,
        default_flip,
        default_turn4,
        default_turn5,
        default_turn6,
    ) {
        Ok(_) => {
            info!("Updated robot connection defaults for id={}", id);
            ServerResponse::Success { message: "Connection defaults updated".to_string() }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to update connection defaults: {}", e) }
    }
}

/// Update robot connection jog defaults.
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
