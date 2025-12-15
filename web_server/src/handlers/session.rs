//! Session linking handlers for HMI pop-out windows.
//!
//! These handlers manage parent/child session relationships,
//! allowing pop-out HMI windows to inherit control authority
//! from their parent session.

use std::sync::Arc;
use uuid::Uuid;
use crate::api_types::ServerResponse;
use crate::session::ClientManager;

/// Link this session as a child of a parent session.
pub async fn link_child_session(
    client_manager: Option<Arc<ClientManager>>,
    client_id: Option<Uuid>,
    parent_session_id: String,
) -> ServerResponse {
    let Some(manager) = client_manager else {
        return ServerResponse::Error {
            message: "Client manager not available".to_string(),
        };
    };
    
    let Some(child_id) = client_id else {
        return ServerResponse::Error {
            message: "Client ID not available".to_string(),
        };
    };
    
    let Ok(parent_id) = Uuid::parse_str(&parent_session_id) else {
        return ServerResponse::Error {
            message: format!("Invalid parent session ID: {}", parent_session_id),
        };
    };
    
    match manager.link_child_session(parent_id, child_id).await {
        Ok(()) => ServerResponse::SessionLinked {
            parent_session_id,
        },
        Err(e) => ServerResponse::Error {
            message: format!("Failed to link session: {}", e),
        },
    }
}

/// Unlink this session from its parent.
pub async fn unlink_child_session(
    client_manager: Option<Arc<ClientManager>>,
    client_id: Option<Uuid>,
) -> ServerResponse {
    let Some(manager) = client_manager else {
        return ServerResponse::Error {
            message: "Client manager not available".to_string(),
        };
    };
    
    let Some(child_id) = client_id else {
        return ServerResponse::Error {
            message: "Client ID not available".to_string(),
        };
    };
    
    manager.unlink_child_session(child_id).await;
    ServerResponse::SessionUnlinked
}

/// Get session info for this client.
pub async fn get_session_info(
    client_manager: Option<Arc<ClientManager>>,
    client_id: Option<Uuid>,
) -> ServerResponse {
    let Some(manager) = client_manager else {
        return ServerResponse::Error {
            message: "Client manager not available".to_string(),
        };
    };
    
    let Some(id) = client_id else {
        return ServerResponse::Error {
            message: "Client ID not available".to_string(),
        };
    };
    
    let Some(client) = manager.get(id).await else {
        return ServerResponse::Error {
            message: "Client not found".to_string(),
        };
    };
    
    let has_control = manager.has_control_or_inherited(id).await;
    let child_ids = manager.get_child_sessions(id).await;
    
    ServerResponse::SessionInfo {
        session_id: id.to_string(),
        robot_connection_id: client.subscribed_robot,
        has_control,
        parent_session_id: client.parent_session.map(|p| p.to_string()),
        child_session_ids: child_ids.iter().map(|c| c.to_string()).collect(),
    }
}

