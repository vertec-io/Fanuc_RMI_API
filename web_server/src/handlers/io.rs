//! I/O handlers for reading/writing digital, analog, and group I/O.

use crate::api_types::ServerResponse;
use crate::RobotConnection;
use fanuc_rmi::commands::{
    FrcReadAIN, FrcReadDIN, FrcReadGIN, FrcWriteAOUT, FrcWriteDOUT, FrcWriteGOUT,
};
use fanuc_rmi::packets::{Command, CommandResponse, PacketPriority, ResponsePacket, SendPacket};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

/// Read a digital input port.
pub async fn read_din(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    port_number: u16,
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

    let packet = SendPacket::Command(Command::FrcReadDIN(FrcReadDIN { port_number }));

    info!("Sending FRC_ReadDIN command for port {}", port_number);
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
                warn!("Received Unknown response while waiting for FRC_ReadDIN: error_id={}", unknown.error_id);
            }

            if let ResponsePacket::CommandResponse(CommandResponse::FrcReadDIN(resp)) = response {
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
            ServerResponse::DinValue {
                port_number: resp.port_number,
                port_value: resp.port_value != 0,
            }
        }
        Ok(None) => {
            error!("No response received for FRC_ReadDIN (port {})", port_number);
            ServerResponse::Error {
                message: "No response received".to_string(),
            }
        }
        Err(_) => {
            error!("Timeout waiting for FRC_ReadDIN response (port {})", port_number);
            ServerResponse::Error {
                message: format!("Timeout waiting for FRC_ReadDIN response (port {})", port_number),
            }
        }
    }
}

/// Write to a digital output port.
pub async fn write_dout(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    port_number: u16,
    port_value: bool,
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

    let packet = SendPacket::Command(Command::FrcWriteDOUT(FrcWriteDOUT {
        port_number,
        port_value: if port_value { 1 } else { 0 },
    }));

    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteDOUT(resp)) = response {
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
            info!("DOUT[{}] set to {} successfully", port_number, if port_value { "ON" } else { "OFF" });
            // Return the new value - this will be broadcast to all clients
            ServerResponse::DoutValue { port_number, port_value }
        }
        Ok(None) => ServerResponse::Error {
            message: "No response received".to_string(),
        },
        Err(_) => ServerResponse::Error {
            message: "Timeout waiting for response".to_string(),
        },
    }
}

/// Read multiple digital inputs (batch operation).
pub async fn read_din_batch(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    port_numbers: Vec<u16>,
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

    let mut results = Vec::new();
    for port_number in port_numbers.iter().copied() {
        let packet = SendPacket::Command(Command::FrcReadDIN(FrcReadDIN { port_number }));

        let mut response_rx = driver.response_tx.subscribe();
        if driver.send_packet(packet, PacketPriority::Standard).is_err() {
            continue;
        }

        // Wait for response with short timeout
        if let Ok(Ok(Some(resp))) = tokio::time::timeout(Duration::from_millis(500), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(CommandResponse::FrcReadDIN(resp)) = response
                {
                    return Ok(Some(resp));
                }
            }
            Ok::<_, ()>(None)
        })
        .await
        {
            if resp.error_id == 0 {
                results.push((port_number, resp.port_value != 0));
            }
        }
    }

    ServerResponse::DinBatch { values: results }
}

// ========== Analog I/O ==========

/// Read an analog input port.
pub async fn read_ain(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    port_number: u16,
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

    let packet = SendPacket::Command(Command::FrcReadAIN(FrcReadAIN { port_number }));

    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            if let ResponsePacket::CommandResponse(CommandResponse::FrcReadAIN(resp)) = response {
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
            ServerResponse::AinValue {
                port_number: resp.port_number,
                port_value: resp.port_value,
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

/// Write to an analog output port.
pub async fn write_aout(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    port_number: u16,
    port_value: f64,
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

    let packet = SendPacket::Command(Command::FrcWriteAOUT(FrcWriteAOUT {
        port_number,
        port_value,
    }));

    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteAOUT(resp)) = response {
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
            info!("AOUT[{}] set to {:.2} successfully", port_number, port_value);
            // Return the new value - this will be broadcast to all clients
            ServerResponse::AoutValue { port_number, port_value }
        }
        Ok(None) => ServerResponse::Error {
            message: "No response received".to_string(),
        },
        Err(_) => ServerResponse::Error {
            message: "Timeout waiting for response".to_string(),
        },
    }
}

// ========== Group I/O ==========

/// Read a group input port.
pub async fn read_gin(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    port_number: u16,
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

    let packet = SendPacket::Command(Command::FrcReadGIN(FrcReadGIN { port_number }));

    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            if let ResponsePacket::CommandResponse(CommandResponse::FrcReadGIN(resp)) = response {
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
            ServerResponse::GinValue {
                port_number: resp.port_number,
                port_value: resp.port_value,
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

/// Write to a group output port.
pub async fn write_gout(
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    port_number: u16,
    port_value: u32,
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

    let packet = SendPacket::Command(Command::FrcWriteGOUT(FrcWriteGOUT {
        port_number,
        port_value,
    }));

    let mut response_rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        return ServerResponse::Error {
            message: format!("Failed to send command: {}", e),
        };
    }

    // Wait for response
    match tokio::time::timeout(Duration::from_secs(5), async {
        while let Ok(response) = response_rx.recv().await {
            if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteGOUT(resp)) = response {
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
            info!("GOUT[{}] set to {} successfully", port_number, port_value);
            // Return the new value - this will be broadcast to all clients
            ServerResponse::GoutValue { port_number, port_value }
        }
        Ok(None) => ServerResponse::Error {
            message: "No response received".to_string(),
        },
        Err(_) => ServerResponse::Error {
            message: "Timeout waiting for response".to_string(),
        },
    }
}
