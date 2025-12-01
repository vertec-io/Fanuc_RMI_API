//! Control lock handlers.
//!
//! Manages which client has control of the robot. Only one client
//! can control the robot at a time; others can observe.

use crate::api_types::ServerResponse;
use crate::session::{ClientManager, ControlError};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Request control of the robot.
pub async fn request_control(
    client_manager: Option<Arc<ClientManager>>,
    client_id: Option<Uuid>,
) -> ServerResponse {
    let client_manager = match client_manager {
        Some(cm) => cm,
        None => return ServerResponse::Error { 
            message: "Client manager not available".to_string() 
        },
    };

    let client_id = match client_id {
        Some(id) => id,
        None => return ServerResponse::Error { 
            message: "Client ID not available".to_string() 
        },
    };

    // Try to acquire control
    let result = client_manager.try_acquire_control(client_id).await;

    match result {
        Ok(previous_holder) => {
            // Notify previous holder if there was one (timeout case)
            if let Some(prev) = previous_holder {
                let lost_response = ServerResponse::ControlLost {
                    reason: "Control timed out due to inactivity".to_string(),
                };
                client_manager.send_to_client(prev, &lost_response).await;
            }

            // Broadcast control change to all clients
            let changed_response = ServerResponse::ControlChanged {
                holder_id: Some(client_id.to_string()),
            };
            client_manager.broadcast_all(&changed_response).await;

            info!("Client {} acquired control", client_id);
            ServerResponse::ControlAcquired
        }
        Err(ControlError::AlreadyControlled { holder, .. }) => {
            ServerResponse::ControlDenied {
                holder_id: holder.to_string(),
                reason: "Another client already has control".to_string(),
            }
        }
        Err(ControlError::TimedOut { previous_holder }) => {
            // This shouldn't happen normally as try_acquire handles timeout internally
            info!("Control timed out from {}", previous_holder);
            ServerResponse::ControlAcquired
        }
    }
}

/// Release control of the robot.
pub async fn release_control(
    client_manager: Option<Arc<ClientManager>>,
    client_id: Option<Uuid>,
) -> ServerResponse {
    let client_manager = match client_manager {
        Some(cm) => cm,
        None => return ServerResponse::Error { 
            message: "Client manager not available".to_string() 
        },
    };

    let client_id = match client_id {
        Some(id) => id,
        None => return ServerResponse::Error { 
            message: "Client ID not available".to_string() 
        },
    };

    if client_manager.release_control(client_id).await {
        // Broadcast control change to all clients
        let changed_response = ServerResponse::ControlChanged {
            holder_id: None,
        };
        client_manager.broadcast_all(&changed_response).await;

        info!("Client {} released control", client_id);
        ServerResponse::ControlReleased
    } else {
        ServerResponse::Error {
            message: "You don't have control to release".to_string(),
        }
    }
}

/// Get current control status.
pub async fn get_control_status(
    client_manager: Option<Arc<ClientManager>>,
    client_id: Option<Uuid>,
) -> ServerResponse {
    let client_manager = match client_manager {
        Some(cm) => cm,
        None => return ServerResponse::Error { 
            message: "Client manager not available".to_string() 
        },
    };

    let holder = client_manager.get_control_holder().await;
    let has_control = client_id.map_or(false, |id| holder == Some(id));

    ServerResponse::ControlStatus {
        has_control,
        holder_id: holder.map(|h| h.to_string()),
    }
}

