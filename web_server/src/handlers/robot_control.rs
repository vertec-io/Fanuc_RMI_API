//! Robot control handlers (abort, reset, initialize).

use std::sync::Arc;
use tracing::{info, error};
use fanuc_rmi::drivers::FanucDriver;
use tokio::sync::{Mutex, RwLock};

use crate::api_types::ServerResponse;
use crate::program_executor::ProgramExecutor;
use crate::session::{ClientManager, execution_state_to_response};
use crate::RobotConnection;

/// Abort current motion and clear motion queue.
///
/// This sends FRC_Abort to the robot and waits for confirmation.
/// Also stops any running program execution.
///
/// After a successful abort, this function automatically re-initializes the TP program
/// by calling FRC_Initialize. This allows motion commands to be sent immediately
/// without requiring a manual re-initialization.
pub async fn robot_abort(
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
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

            // Auto-reinitialize after successful abort
            if success {
                if let Some(ref conn) = robot_connection {
                    info!("Auto-reinitializing TP program after abort...");
                    let mut conn = conn.write().await;
                    match conn.reinitialize_tp().await {
                        Ok(()) => {
                            info!("TP program auto-reinitialized successfully after abort");
                            // Broadcast updated connection status with tp_program_initialized = true
                            if let Some(ref cm) = client_manager {
                                let status = ServerResponse::ConnectionStatus {
                                    connected: conn.connected,
                                    robot_addr: conn.robot_addr.clone(),
                                    robot_port: conn.robot_port,
                                    connection_name: conn.saved_connection.as_ref().map(|s| s.name.clone()),
                                    connection_id: conn.saved_connection.as_ref().map(|s| s.id),
                                    tp_program_initialized: conn.tp_program_initialized,
                                };
                                cm.broadcast_all(&status).await;
                            }
                        }
                        Err(e) => {
                            // Re-initialization failed - leave tp_program_initialized as false
                            info!("Auto-reinitialize failed after abort: {}. Manual initialization required.", e);
                            // Broadcast updated connection status with tp_program_initialized = false
                            if let Some(ref cm) = client_manager {
                                let status = ServerResponse::ConnectionStatus {
                                    connected: conn.connected,
                                    robot_addr: conn.robot_addr.clone(),
                                    robot_port: conn.robot_port,
                                    connection_name: conn.saved_connection.as_ref().map(|s| s.name.clone()),
                                    connection_id: conn.saved_connection.as_ref().map(|s| s.id),
                                    tp_program_initialized: conn.tp_program_initialized,
                                };
                                cm.broadcast_all(&status).await;
                            }
                        }
                    }
                }
            } else {
                // Abort failed - mark TP as not initialized
                if let Some(ref conn) = robot_connection {
                    let mut conn = conn.write().await;
                    conn.tp_program_initialized = false;
                    if let Some(ref cm) = client_manager {
                        let status = ServerResponse::ConnectionStatus {
                            connected: conn.connected,
                            robot_addr: conn.robot_addr.clone(),
                            robot_port: conn.robot_port,
                            connection_name: conn.saved_connection.as_ref().map(|s| s.name.clone()),
                            connection_id: conn.saved_connection.as_ref().map(|s| s.id),
                            tp_program_initialized: conn.tp_program_initialized,
                        };
                        cm.broadcast_all(&status).await;
                    }
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
/// On success, sets `tp_program_initialized` to true, allowing motion commands.
/// Note: group_mask is currently ignored as the driver uses default (1).
pub async fn robot_initialize(
    driver: Option<Arc<FanucDriver>>,
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    client_manager: Option<Arc<ClientManager>>,
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

            // Mark TP program as initialized on success
            if success {
                if let Some(ref conn) = robot_connection {
                    let mut conn = conn.write().await;
                    conn.tp_program_initialized = true;
                    info!("TP program marked as initialized");

                    // Broadcast updated connection status
                    if let Some(ref cm) = client_manager {
                        let status = ServerResponse::ConnectionStatus {
                            connected: conn.connected,
                            robot_addr: conn.robot_addr.clone(),
                            robot_port: conn.robot_port,
                            connection_name: conn.saved_connection.as_ref().map(|s| s.name.clone()),
                            connection_id: conn.saved_connection.as_ref().map(|s| s.id),
                            tp_program_initialized: conn.tp_program_initialized,
                        };
                        cm.broadcast_all(&status).await;
                    }
                }
            }

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

