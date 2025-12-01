//! I/O configuration handlers.

use crate::api_types::{IoDisplayConfigDto, ServerResponse};
use crate::database::Database;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Get I/O display configuration for a robot.
pub async fn get_io_config(
    db: Arc<Mutex<Database>>,
    robot_connection_id: i64,
) -> ServerResponse {
    let db = db.lock().await;
    match db.get_io_display_config(robot_connection_id) {
        Ok(configs) => {
            let dtos: Vec<IoDisplayConfigDto> = configs
                .into_iter()
                .map(|c| IoDisplayConfigDto {
                    io_type: c.io_type,
                    io_index: c.io_index,
                    display_name: c.display_name,
                    is_visible: c.is_visible,
                    display_order: c.display_order,
                })
                .collect();
            ServerResponse::IoConfig { configs: dtos }
        }
        Err(e) => ServerResponse::Error {
            message: format!("Failed to get I/O config: {}", e),
        },
    }
}

/// Update I/O display configuration.
pub async fn update_io_config(
    db: Arc<Mutex<Database>>,
    robot_connection_id: i64,
    io_type: String,
    io_index: i32,
    display_name: Option<String>,
    is_visible: bool,
    display_order: Option<i32>,
) -> ServerResponse {
    let db = db.lock().await;
    match db.upsert_io_display_config(
        robot_connection_id,
        &io_type,
        io_index,
        display_name.as_deref(),
        is_visible,
        display_order,
    ) {
        Ok(()) => ServerResponse::Success {
            message: format!("Updated {}[{}] config", io_type, io_index),
        },
        Err(e) => ServerResponse::Error {
            message: format!("Failed to update I/O config: {}", e),
        },
    }
}

