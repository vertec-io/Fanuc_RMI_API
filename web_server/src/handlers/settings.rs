//! Settings management handlers.
//!
//! Handles robot settings CRUD and database reset.

use crate::api_types::*;
use crate::database::Database;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Get robot settings.
pub async fn get_settings(db: Arc<Mutex<Database>>) -> ServerResponse {
    let db = db.lock().await;
    match db.get_robot_settings() {
        Ok(settings) => ServerResponse::Settings {
            settings: RobotSettingsDto {
                default_w: settings.default_w,
                default_p: settings.default_p,
                default_r: settings.default_r,
                default_speed: settings.default_speed,
                default_term_type: settings.default_term_type,
                default_uframe: settings.default_uframe,
                default_utool: settings.default_utool,
            }
        },
        Err(e) => ServerResponse::Error { message: format!("Failed to get settings: {}", e) }
    }
}

/// Update robot settings.
pub async fn update_settings(
    db: Arc<Mutex<Database>>,
    default_w: f64,
    default_p: f64,
    default_r: f64,
    default_speed: f64,
    default_term_type: &str,
    default_uframe: i32,
    default_utool: i32,
) -> ServerResponse {
    let db = db.lock().await;
    match db.update_robot_settings(
        default_w, default_p, default_r,
        default_speed, default_term_type,
        default_uframe, default_utool,
    ) {
        Ok(_) => {
            info!("Updated robot settings");
            ServerResponse::Success { message: "Settings updated".to_string() }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to update settings: {}", e) }
    }
}

/// Reset the database.
pub async fn reset_database(db: Arc<Mutex<Database>>) -> ServerResponse {
    let mut db = db.lock().await;
    match db.reset() {
        Ok(_) => {
            info!("Database reset");
            ServerResponse::Success { message: "Database reset successfully".to_string() }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to reset database: {}", e) }
    }
}

