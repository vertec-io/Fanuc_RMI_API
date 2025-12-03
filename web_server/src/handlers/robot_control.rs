//! Robot control handlers (abort, reset, initialize).

use std::sync::Arc;
use tracing::{info, error};
use fanuc_rmi::drivers::FanucDriver;
use tokio::sync::Mutex;

use crate::api_types::ServerResponse;
use crate::program_executor::ProgramExecutor;
use crate::session::{ClientManager, execution_state_to_response};

/// Abort current motion and clear motion queue.
/// 
/// This sends FRC_Abort to the robot and waits for confirmation.
/// Also stops any running program execution.
pub async fn robot_abort(
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    client_manager: Option<Arc<ClientManager>>,
) -> ServerResponse {
    let Some(driver) = driver else {
        return ServerResponse::RobotCommandResult {
            command: "abort".to_string(),
            success: false,
            error_id: None,
            message: Some("Not connected to robot".to_string()),
        };
    };

    // Stop the executor first
    if let Some(ref executor) = executor {
        let mut exec_guard = executor.lock().await;
        exec_guard.stop();
        exec_guard.clear_in_flight();
        info!("Executor stopped and in-flight cleared for abort");
    }

    // Send abort and wait for response
    match driver.abort().await {
        Ok(response) => {
            let error_id = response.error_id as i32;
            let success = error_id == 0;

            info!("Robot abort completed: error_id={}", error_id);

            // Broadcast execution state change
            if let Some(ref cm) = client_manager {
                if let Some(ref executor) = executor {
                    let exec_guard = executor.lock().await;
                    let state = exec_guard.get_state();
                    let state_response = execution_state_to_response(&state);
                    cm.broadcast_all(&state_response).await;
                }
            }

            ServerResponse::RobotCommandResult {
                command: "abort".to_string(),
                success,
                error_id: Some(error_id),
                message: if success { None } else { Some(format!("Abort returned error {}", error_id)) },
            }
        }
        Err(e) => {
            error!("Robot abort failed: {:?}", e);
            ServerResponse::RobotCommandResult {
                command: "abort".to_string(),
                success: false,
                error_id: None,
                message: Some(format!("Abort failed: {:?}", e)),
            }
        }
    }
}

/// Reset robot controller (clears errors).
/// 
/// This sends FRC_Reset to the robot and waits for confirmation.
pub async fn robot_reset(
    driver: Option<Arc<FanucDriver>>,
) -> ServerResponse {
    let Some(driver) = driver else {
        return ServerResponse::RobotCommandResult {
            command: "reset".to_string(),
            success: false,
            error_id: None,
            message: Some("Not connected to robot".to_string()),
        };
    };

    // Send reset and wait for response
    match driver.reset().await {
        Ok(response) => {
            let error_id = response.error_id as i32;
            let success = error_id == 0;

            info!("Robot reset completed: error_id={}", error_id);

            ServerResponse::RobotCommandResult {
                command: "reset".to_string(),
                success,
                error_id: Some(error_id),
                message: if success { None } else { Some(format!("Reset returned error {}", error_id)) },
            }
        }
        Err(e) => {
            error!("Robot reset failed: {:?}", e);
            ServerResponse::RobotCommandResult {
                command: "reset".to_string(),
                success: false,
                error_id: None,
                message: Some(format!("Reset failed: {:?}", e)),
            }
        }
    }
}

/// Initialize robot controller.
///
/// This sends FRC_Initialize to the robot and waits for confirmation.
/// Note: group_mask is currently ignored as the driver uses default (1).
pub async fn robot_initialize(
    driver: Option<Arc<FanucDriver>>,
    _group_mask: u8,
) -> ServerResponse {
    let Some(driver) = driver else {
        return ServerResponse::RobotCommandResult {
            command: "initialize".to_string(),
            success: false,
            error_id: None,
            message: Some("Not connected to robot".to_string()),
        };
    };

    // Send initialize and wait for response
    match driver.initialize().await {
        Ok(response) => {
            let error_id = response.error_id as i32;
            let success = error_id == 0;

            info!("Robot initialize completed: error_id={}", error_id);

            ServerResponse::RobotCommandResult {
                command: "initialize".to_string(),
                success,
                error_id: Some(error_id),
                message: if success { None } else { Some(format!("Initialize returned error {}", error_id)) },
            }
        }
        Err(e) => {
            error!("Robot initialize failed: {:?}", e);
            ServerResponse::RobotCommandResult {
                command: "initialize".to_string(),
                success: false,
                error_id: None,
                message: Some(format!("Initialize failed: {:?}", e)),
            }
        }
    }
}

