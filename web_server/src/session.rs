//! Robot session and client management.
//!
//! This module provides server-side state management for robot connections
//! and client sessions. The server is the source of truth for execution state.

use crate::api_types::ServerResponse;
use crate::program_executor::ProgramExecutor;
use futures_util::SinkExt;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};
use uuid::Uuid;

/// Type alias for WebSocket sender
pub type WsSender = Arc<Mutex<futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    Message
>>>;

/// A connected client with its WebSocket sender.
#[derive(Clone)]
pub struct Client {
    pub id: Uuid,
    pub sender: WsSender,
    /// The robot connection ID this client is subscribed to (if any)
    pub subscribed_robot: Option<i64>,
}

impl Client {
    pub fn new(sender: WsSender) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender,
            subscribed_robot: None,
        }
    }

    /// Send a response to this client.
    pub async fn send(&self, response: &ServerResponse) -> Result<(), String> {
        let json = serde_json::to_string(response)
            .map_err(|e| format!("Serialization error: {}", e))?;
        let mut sender = self.sender.lock().await;
        sender.send(Message::Text(json)).await
            .map_err(|e| format!("Send error: {}", e))
    }
}

/// Manages all connected clients.
pub struct ClientManager {
    clients: RwLock<HashMap<Uuid, Client>>,
}

impl ClientManager {
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new client and return its ID.
    pub async fn register(&self, sender: WsSender) -> Uuid {
        let client = Client::new(sender);
        let id = client.id;
        let mut clients = self.clients.write().await;
        clients.insert(id, client);
        info!("Client {} registered ({} total)", id, clients.len());
        id
    }

    /// Unregister a client.
    pub async fn unregister(&self, client_id: Uuid) {
        let mut clients = self.clients.write().await;
        if clients.remove(&client_id).is_some() {
            info!("Client {} unregistered ({} remaining)", client_id, clients.len());
        }
    }

    /// Get a client by ID.
    pub async fn get(&self, client_id: Uuid) -> Option<Client> {
        let clients = self.clients.read().await;
        clients.get(&client_id).cloned()
    }

    /// Subscribe a client to a robot connection.
    pub async fn subscribe_to_robot(&self, client_id: Uuid, robot_connection_id: i64) {
        let mut clients = self.clients.write().await;
        if let Some(client) = clients.get_mut(&client_id) {
            client.subscribed_robot = Some(robot_connection_id);
            info!("Client {} subscribed to robot {}", client_id, robot_connection_id);
        }
    }

    /// Get all clients subscribed to a robot.
    pub async fn get_subscribers(&self, robot_connection_id: i64) -> Vec<Client> {
        let clients = self.clients.read().await;
        clients.values()
            .filter(|c| c.subscribed_robot == Some(robot_connection_id))
            .cloned()
            .collect()
    }

    /// Broadcast a response to all clients subscribed to a robot.
    pub async fn broadcast_to_robot(&self, robot_connection_id: i64, response: &ServerResponse) {
        let subscribers = self.get_subscribers(robot_connection_id).await;
        for client in subscribers {
            if let Err(e) = client.send(response).await {
                warn!("Failed to send to client {}: {}", client.id, e);
            }
        }
    }

    /// Broadcast a response to all connected clients.
    pub async fn broadcast_all(&self, response: &ServerResponse) {
        let clients = self.clients.read().await;
        for client in clients.values() {
            if let Err(e) = client.send(response).await {
                warn!("Failed to send to client {}: {}", client.id, e);
            }
        }
    }
}

/// Robot session state - holds executor and subscribed clients for a robot.
pub struct RobotSession {
    pub connection_id: i64,
    pub executor: Mutex<ProgramExecutor>,
    pub subscribed_clients: RwLock<HashSet<Uuid>>,
}

impl RobotSession {
    pub fn new(connection_id: i64) -> Self {
        Self {
            connection_id,
            executor: Mutex::new(ProgramExecutor::new()),
            subscribed_clients: RwLock::new(HashSet::new()),
        }
    }

    /// Subscribe a client to this robot session.
    pub async fn subscribe(&self, client_id: Uuid) {
        let mut clients = self.subscribed_clients.write().await;
        clients.insert(client_id);
        info!("Client {} subscribed to robot session {}", client_id, self.connection_id);
    }

    /// Unsubscribe a client from this robot session.
    pub async fn unsubscribe(&self, client_id: Uuid) {
        let mut clients = self.subscribed_clients.write().await;
        clients.remove(&client_id);
        info!("Client {} unsubscribed from robot session {}", client_id, self.connection_id);
    }
}

/// Convert ExecutionState to a ServerResponse for broadcasting.
pub fn execution_state_to_response(state: &crate::program_executor::ExecutionState) -> ServerResponse {
    use crate::program_executor::ExecutionState;

    match state {
        ExecutionState::Idle => ServerResponse::ExecutionStateChanged {
            state: "idle".to_string(),
            program_id: None,
            current_line: None,
            total_lines: None,
            message: None,
        },
        ExecutionState::Running { program_id, total_lines, last_completed } => ServerResponse::ExecutionStateChanged {
            state: "running".to_string(),
            program_id: Some(*program_id),
            current_line: Some(*last_completed),
            total_lines: Some(*total_lines),
            message: None,
        },
        ExecutionState::Paused { program_id, total_lines, last_completed } => ServerResponse::ExecutionStateChanged {
            state: "paused".to_string(),
            program_id: Some(*program_id),
            current_line: Some(*last_completed),
            total_lines: Some(*total_lines),
            message: None,
        },
        ExecutionState::Stopping => ServerResponse::ExecutionStateChanged {
            state: "stopping".to_string(),
            program_id: None,
            current_line: None,
            total_lines: None,
            message: None,
        },
        ExecutionState::Completed { program_id, total_lines } => ServerResponse::ExecutionStateChanged {
            state: "completed".to_string(),
            program_id: Some(*program_id),
            current_line: Some(*total_lines),
            total_lines: Some(*total_lines),
            message: None,
        },
        ExecutionState::Error { message } => ServerResponse::ExecutionStateChanged {
            state: "error".to_string(),
            program_id: None,
            current_line: None,
            total_lines: None,
            message: Some(message.clone()),
        },
    }
}

