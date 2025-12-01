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

