//! Frame and Tool management handlers.
//!
//! Handles reading and writing user frames (UFrame) and user tools (UTool)
//! on the FANUC robot controller.

use crate::api_types::ServerResponse;
use crate::RobotConnection;
use fanuc_rmi::commands::{
    FrcGetUFrameUTool, FrcReadUFrameData, FrcReadUToolData, FrcSetUFrameUTool,
    FrcWriteUFrameData, FrcWriteUToolData,
};
use fanuc_rmi::packets::{Command, CommandResponse, ResponsePacket, SendPacket, PacketPriority};
use fanuc_rmi::FrameData;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Get the currently active UFrame and UTool numbers.
pub async fn get_active_frame_tool(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
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

    // Send FrcGetUFrameUTool command
    let cmd = FrcGetUFrameUTool::new(None);
    let packet = SendPacket::Command(Command::FrcGetUFrameUTool(cmd));

    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            if let ResponsePacket::CommandResponse(CommandResponse::FrcGetUFrameUTool(resp)) =
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
            ServerResponse::ActiveFrameTool {
                uframe: resp.u_frame_number,
                utool: resp.u_tool_number,
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

/// Set the active UFrame and UTool numbers.
pub async fn set_active_frame_tool(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    uframe: u8,
    utool: u8,
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
            ServerResponse::Success {
                message: format!("Set UFrame={}, UTool={}", uframe, utool),
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

    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
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
            ServerResponse::FrameData {
                frame_number: resp.u_frame_number as u8,
                x: resp.frame.x,
                y: resp.frame.y,
                z: resp.frame.z,
                w: resp.frame.w,
                p: resp.frame.p,
                r: resp.frame.r,
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

    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
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
            ServerResponse::ToolData {
                tool_number: resp.utool_number,
                x: resp.frame.x,
                y: resp.frame.y,
                z: resp.frame.z,
                w: resp.frame.w,
                p: resp.frame.p,
                r: resp.frame.r,
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

/// Write UFrame data for a specific frame number.
#[allow(clippy::too_many_arguments)]
pub async fn write_frame_data(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    frame_number: u8,
    x: f64,
    y: f64,
    z: f64,
    w: f64,
    p: f64,
    r: f64,
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
    let frame = FrameData { x, y, z, w, p, r };
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
#[allow(clippy::too_many_arguments)]
pub async fn write_tool_data(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    tool_number: u8,
    x: f64,
    y: f64,
    z: f64,
    w: f64,
    p: f64,
    r: f64,
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
    let frame = FrameData { x, y, z, w, p, r };
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
