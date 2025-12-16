//! Frame and Tool management handlers.
//!
//! Handles reading and writing user frames (UFrame) and user tools (UTool)
//! on the FANUC robot controller.

use crate::api_types::ServerResponse;
use crate::session::ClientManager;
use crate::RobotConnection;
use fanuc_rmi::commands::{
    FrcReadUFrameData, FrcReadUToolData, FrcSetUFrameUTool,
    FrcWriteUFrameData, FrcWriteUToolData,
};
use fanuc_rmi::packets::{Command, CommandResponse, ResponsePacket, SendPacket, PacketPriority};
use fanuc_rmi::FrameData;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Get the currently active UFrame and UTool numbers.
///
/// Returns the server-side state (which is synchronized with the robot on connection
/// and when changed via SetActiveFrameTool). This is faster than querying the robot
/// and ensures all clients see consistent values.
pub async fn get_active_frame_tool(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
) -> ServerResponse {
    let Some(conn) = robot_connection else {
        return ServerResponse::Error {
            message: "Not connected to robot".to_string(),
        };
    };

    let conn = conn.read().await;
    if !conn.connected {
        return ServerResponse::Error {
            message: "Robot driver not initialized".to_string(),
        };
    }

    // Return server-side state (synchronized with robot on connection and changes)
    ServerResponse::ActiveFrameTool {
        uframe: conn.active_uframe(),
        utool: conn.active_utool(),
    }
}

/// Set the active UFrame and UTool numbers.
///
/// This function:
/// 1. Sends the command to the robot
/// 2. Updates server-side state (active_uframe/active_utool)
/// 3. Broadcasts the change to all connected clients
pub async fn set_active_frame_tool(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    client_manager: Option<Arc<ClientManager>>,
    uframe: u8,
    utool: u8,
) -> ServerResponse {
    let Some(conn) = robot_connection else {
        return ServerResponse::Error {
            message: "Not connected to robot".to_string(),
        };
    };

    // Need write lock to update server state after success
    let mut conn = conn.write().await;
    let Some(ref driver) = conn.driver else {
        return ServerResponse::Error {
            message: "Robot driver not initialized".to_string(),
        };
    };

    // Send FrcSetUFrameUTool command
    let cmd = FrcSetUFrameUTool::new(None, utool, uframe);
    let packet = SendPacket::Command(Command::FrcSetUFrameUTool(cmd));

    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            if let ResponsePacket::CommandResponse(CommandResponse::FrcSetUFrameUTool(resp)) =
                response
            {
                return Some(resp);
            }
        }
        None
    })
    .await
    {
        Ok(Some(resp)) => {
            if resp.error_id != 0 {
                return ServerResponse::Error {
                    message: format!("Robot error: {}", resp.error_id),
                };
            }

            // Track changes to changelog
            let old_uframe = conn.active_configuration.u_frame_number;
            let old_utool = conn.active_configuration.u_tool_number;

            if old_uframe != uframe as i32 {
                conn.active_configuration.change_log.push(crate::ChangeLogEntry {
                    field_name: "UFrame".to_string(),
                    old_value: format!("{}", old_uframe),
                    new_value: format!("{}", uframe),
                });
            }
            if old_utool != utool as i32 {
                conn.active_configuration.change_log.push(crate::ChangeLogEntry {
                    field_name: "UTool".to_string(),
                    old_value: format!("{}", old_utool),
                    new_value: format!("{}", utool),
                });
            }

            // Update server-side state
            conn.active_configuration.u_frame_number = uframe as i32;
            conn.active_configuration.u_tool_number = utool as i32;
            conn.active_configuration.changes_count += 1;  // Increment counter

            // Broadcast to all clients
            let broadcast_response = ServerResponse::ActiveFrameTool { uframe, utool };
            if let Some(ref client_manager) = client_manager {
                client_manager.broadcast_all(&broadcast_response).await;

                // Also broadcast the full active configuration with changes count
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
                    default_rotation_jog_speed: config.default_rotation_jog_speed,
                    default_rotation_jog_step: config.default_rotation_jog_step,
                };
                client_manager.broadcast_all(&config_response).await;
            }

            broadcast_response
        }
        Ok(None) => ServerResponse::Error {
            message: "No response received".to_string(),
        },
        Err(_) => ServerResponse::Error {
            message: "Timeout waiting for response".to_string(),
        },
    }
}

/// Read UFrame data for a specific frame number.
pub async fn read_frame_data(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    frame_number: u8,
) -> ServerResponse {
    let Some(conn) = robot_connection else {
        return ServerResponse::Error {
            message: "Not connected to robot".to_string(),
        };
    };

    let conn = conn.read().await;
    let Some(ref driver) = conn.driver else {
        return ServerResponse::Error {
            message: "Robot driver not initialized".to_string(),
        };
    };

    // Send FrcReadUFrameData command
    let cmd = FrcReadUFrameData::new(None, frame_number as i8);
    let packet = SendPacket::Command(Command::FrcReadUFrameData(cmd));

    info!("Sending FRC_ReadUFrameData command for frame {}", frame_number);
    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            // Log Unknown responses
            if let ResponsePacket::CommandResponse(CommandResponse::Unknown(ref unknown)) = response {
                warn!("Received Unknown response while waiting for FRC_ReadUFrameData: error_id={}", unknown.error_id);
            }

            if let ResponsePacket::CommandResponse(CommandResponse::FrcReadUFrameData(resp)) =
                response
            {
                return Some(resp);
            }
        }
        None
    })
    .await
    {
        Ok(Some(resp)) => {
            if resp.error_id != 0 {
                return ServerResponse::Error {
                    message: format!("Robot error: {}", resp.error_id),
                };
            }
            ServerResponse::FrameDataResponse {
                frame_number: resp.frame_number as u8,
                data: resp.frame.into(),
            }
        }
        Ok(None) => {
            error!("No response received for FRC_ReadUFrameData (frame {})", frame_number);
            ServerResponse::Error {
                message: "No response received".to_string(),
            }
        }
        Err(_) => {
            error!("Timeout waiting for FRC_ReadUFrameData response (frame {})", frame_number);
            ServerResponse::Error {
                message: format!("Timeout waiting for FRC_ReadUFrameData response (frame {})", frame_number),
            }
        }
    }
}

/// Read UTool data for a specific tool number.
pub async fn read_tool_data(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    tool_number: u8,
) -> ServerResponse {
    let Some(conn) = robot_connection else {
        return ServerResponse::Error {
            message: "Not connected to robot".to_string(),
        };
    };

    let conn = conn.read().await;
    let Some(ref driver) = conn.driver else {
        return ServerResponse::Error {
            message: "Robot driver not initialized".to_string(),
        };
    };

    // Send FrcReadUToolData command
    let cmd = FrcReadUToolData::new(None, tool_number as i8);
    let packet = SendPacket::Command(Command::FrcReadUToolData(cmd));

    info!("Sending FRC_ReadUToolData command for tool {}", tool_number);
    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            // Log Unknown responses
            if let ResponsePacket::CommandResponse(CommandResponse::Unknown(ref unknown)) = response {
                warn!("Received Unknown response while waiting for FRC_ReadUToolData: error_id={}", unknown.error_id);
            }

            if let ResponsePacket::CommandResponse(CommandResponse::FrcReadUToolData(resp)) =
                response
            {
                return Some(resp);
            }
        }
        None
    })
    .await
    {
        Ok(Some(resp)) => {
            if resp.error_id != 0 {
                return ServerResponse::Error {
                    message: format!("Robot error: {}", resp.error_id),
                };
            }
            ServerResponse::ToolDataResponse {
                tool_number: resp.tool_number,
                data: resp.frame.into(),
            }
        }
        Ok(None) => {
            error!("No response received for FRC_ReadUToolData (tool {})", tool_number);
            ServerResponse::Error {
                message: "No response received".to_string(),
            }
        }
        Err(_) => {
            error!("Timeout waiting for FRC_ReadUToolData response (tool {})", tool_number);
            ServerResponse::Error {
                message: format!("Timeout waiting for FRC_ReadUToolData response (tool {})", tool_number),
            }
        }
    }
}

/// Write UFrame data for a specific frame number.
pub async fn write_frame_data(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    frame_number: u8,
    frame: FrameData,
) -> ServerResponse {
    let Some(conn) = robot_connection else {
        return ServerResponse::Error {
            message: "Not connected to robot".to_string(),
        };
    };

    let conn = conn.read().await;
    let Some(ref driver) = conn.driver else {
        return ServerResponse::Error {
            message: "Robot driver not initialized".to_string(),
        };
    };

    // Send FrcWriteUFrameData command
    let cmd = FrcWriteUFrameData::new(None, frame_number as i8, frame);
    let packet = SendPacket::Command(Command::FrcWriteUFrameData(cmd));

    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteUFrameData(resp)) =
                response
            {
                return Some(resp);
            }
        }
        None
    })
    .await
    {
        Ok(Some(resp)) => {
            if resp.error_id != 0 {
                return ServerResponse::Error {
                    message: format!("Robot error: {}", resp.error_id),
                };
            }
            ServerResponse::Success {
                message: format!("Wrote UFrame {} data", frame_number),
            }
        }
        Ok(None) => ServerResponse::Error {
            message: "No response received".to_string(),
        },
        Err(_) => ServerResponse::Error {
            message: "Timeout waiting for response".to_string(),
        },
    }
}

/// Write UTool data for a specific tool number.
pub async fn write_tool_data(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    tool_number: u8,
    frame: FrameData,
) -> ServerResponse {
    let Some(conn) = robot_connection else {
        return ServerResponse::Error {
            message: "Not connected to robot".to_string(),
        };
    };

    let conn = conn.read().await;
    let Some(ref driver) = conn.driver else {
        return ServerResponse::Error {
            message: "Robot driver not initialized".to_string(),
        };
    };

    // Send FrcWriteUToolData command
    let cmd = FrcWriteUToolData::new(None, tool_number as i8, frame);
    let packet = SendPacket::Command(Command::FrcWriteUToolData(cmd));

    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteUToolData(resp)) =
                response
            {
                return Some(resp);
            }
        }
        None
    })
    .await
    {
        Ok(Some(_resp)) => {
            // FrcWriteUToolData response is the command itself echoed back
            // Check if it was successful by verifying we got a response
            ServerResponse::Success {
                message: format!("Wrote UTool {} data", tool_number),
            }
        }
        Ok(None) => ServerResponse::Error {
            message: "No response received".to_string(),
        },
        Err(_) => ServerResponse::Error {
            message: "Timeout waiting for response".to_string(),
        },
    }
}
