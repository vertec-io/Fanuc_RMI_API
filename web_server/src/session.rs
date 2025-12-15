//! Robot session and client management.
//!
//! This module provides server-side state management for robot connections
//! and client sessions. The server is the source of truth for execution state.

use crate::api_types::ServerResponse;
use crate::program_executor::ProgramExecutor;
use futures_util::SinkExt;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};
use uuid::Uuid;

// ========== Control Lock ==========

/// Error when trying to acquire control.
#[derive(Debug, Clone)]
pub enum ControlError {
    /// Another client already has control
    AlreadyControlled {
        holder: Uuid,
        /// True if the client can request a transfer
        can_request_transfer: bool,
    },
    /// Control lock timed out (informational)
    TimedOut { previous_holder: Uuid },
}

/// Control lock for a robot - only one client can control at a time.
pub struct RobotControlLock {
    /// Client ID currently holding control (if any)
    holder: Option<Uuid>,
    /// When control was acquired
    acquired_at: Option<Instant>,
    /// Last activity time (for timeout)
    last_activity: Option<Instant>,
}

impl RobotControlLock {
    /// Inactivity timeout - release control after 10 minutes of no commands
    pub const INACTIVITY_TIMEOUT: Duration = Duration::from_secs(600);

    pub fn new() -> Self {
        Self {
            holder: None,
            acquired_at: None,
            last_activity: None,
        }
    }

    /// Get the current holder (if any).
    pub fn holder(&self) -> Option<Uuid> {
        self.holder
    }

    /// Check if a specific client has control.
    pub fn is_holder(&self, client_id: Uuid) -> bool {
        self.holder == Some(client_id)
    }

    /// Check if control has timed out due to inactivity.
    pub fn is_timed_out(&self) -> bool {
        if let Some(last) = self.last_activity {
            last.elapsed() > Self::INACTIVITY_TIMEOUT
        } else {
            false
        }
    }

    /// Try to acquire control.
    /// Returns Ok(()) if control was acquired, or Err with details if not.
    pub fn try_acquire(&mut self, client_id: Uuid) -> Result<Option<Uuid>, ControlError> {
        // If we already hold control, just update activity
        if self.holder == Some(client_id) {
            self.last_activity = Some(Instant::now());
            return Ok(None);
        }

        // Check if there's an existing holder
        if let Some(holder) = self.holder {
            // Check for timeout
            if self.is_timed_out() {
                let previous = self.holder.take();
                info!("Control lock timed out, releasing from {:?}", previous);
                // Fall through to acquire below
            } else {
                return Err(ControlError::AlreadyControlled {
                    holder,
                    can_request_transfer: true,
                });
            }
        }

        // Acquire control
        let previous = self.holder;
        self.holder = Some(client_id);
        self.acquired_at = Some(Instant::now());
        self.last_activity = Some(Instant::now());
        info!("Control acquired by {}", client_id);
        Ok(previous)
    }

    /// Update last activity time (called on commands from the holder).
    pub fn touch(&mut self, client_id: Uuid) -> bool {
        if self.holder == Some(client_id) {
            self.last_activity = Some(Instant::now());
            true
        } else {
            false
        }
    }

    /// Release control voluntarily.
    pub fn release(&mut self, client_id: Uuid) -> bool {
        if self.holder == Some(client_id) {
            self.holder = None;
            self.acquired_at = None;
            self.last_activity = None;
            info!("Control released by {}", client_id);
            true
        } else {
            false
        }
    }

    /// Force release (e.g., when client disconnects).
    pub fn force_release(&mut self) -> Option<Uuid> {
        let holder = self.holder.take();
        self.acquired_at = None;
        self.last_activity = None;
        if let Some(h) = holder {
            info!("Control force-released from {}", h);
        }
        holder
    }

    /// Transfer control to another client.
    pub fn transfer(&mut self, from: Uuid, to: Uuid) -> bool {
        if self.holder == Some(from) {
            self.holder = Some(to);
            self.acquired_at = Some(Instant::now());
            self.last_activity = Some(Instant::now());
            info!("Control transferred from {} to {}", from, to);
            true
        } else {
            false
        }
    }
}

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
    /// Parent session ID if this is a linked child (e.g., pop-out HMI window)
    pub parent_session: Option<Uuid>,
    /// Child session IDs linked to this session
    pub child_sessions: HashSet<Uuid>,
}

impl Client {
    pub fn new(sender: WsSender) -> Self {
        Self {
            id: Uuid::new_v4(),
            sender,
            subscribed_robot: None,
            parent_session: None,
            child_sessions: HashSet::new(),
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

/// Manages all connected clients and the global control lock.
/// Note: Currently we support a single robot connection, so there's one global control lock.
/// When multi-robot support is added, this would move to per-RobotSession.
pub struct ClientManager {
    clients: RwLock<HashMap<Uuid, Client>>,
    control_lock: RwLock<RobotControlLock>,
}

impl ClientManager {
    pub fn new() -> Self {
        Self {
            clients: RwLock::new(HashMap::new()),
            control_lock: RwLock::new(RobotControlLock::new()),
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

    /// Unregister a client and release control if they held it.
    pub async fn unregister(&self, client_id: Uuid) {
        // First, release control if this client held it
        {
            let mut lock = self.control_lock.write().await;
            if lock.is_holder(client_id) {
                lock.force_release();
                info!("Control released due to client {} disconnect", client_id);
            }
        }

        // Then remove the client
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

    /// Send a response to a specific client.
    pub async fn send_to_client(&self, client_id: Uuid, response: &ServerResponse) {
        if let Some(client) = self.get(client_id).await {
            if let Err(e) = client.send(response).await {
                warn!("Failed to send to client {}: {}", client_id, e);
            }
        }
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

    // ========== Control Lock Methods ==========

    /// Try to acquire control of the robot.
    pub async fn try_acquire_control(&self, client_id: Uuid) -> Result<Option<Uuid>, ControlError> {
        let mut lock = self.control_lock.write().await;
        lock.try_acquire(client_id)
    }

    /// Release control of the robot.
    pub async fn release_control(&self, client_id: Uuid) -> bool {
        let mut lock = self.control_lock.write().await;
        lock.release(client_id)
    }

    /// Check if a client has control.
    pub async fn has_control(&self, client_id: Uuid) -> bool {
        let lock = self.control_lock.read().await;
        lock.is_holder(client_id)
    }

    /// Get the current control holder.
    pub async fn get_control_holder(&self) -> Option<Uuid> {
        let lock = self.control_lock.read().await;
        lock.holder()
    }

    /// Update activity timestamp for control holder.
    pub async fn touch_control(&self, client_id: Uuid) -> bool {
        let mut lock = self.control_lock.write().await;
        lock.touch(client_id)
    }

    /// Check for and release timed-out control.
    /// Returns the previous holder's UUID if control was released due to timeout.
    pub async fn check_control_timeout(&self) -> Option<Uuid> {
        let mut lock = self.control_lock.write().await;
        if lock.is_timed_out() {
            lock.force_release()
        } else {
            None
        }
    }

    // ========== Session Linking Methods (for HMI pop-out windows) ==========

    /// Link a child session to a parent session.
    /// The child inherits the parent's control authority.
    pub async fn link_child_session(&self, parent_id: Uuid, child_id: Uuid) -> Result<(), String> {
        let mut clients = self.clients.write().await;

        // Verify both sessions exist
        if !clients.contains_key(&parent_id) {
            return Err(format!("Parent session {} not found", parent_id));
        }
        if !clients.contains_key(&child_id) {
            return Err(format!("Child session {} not found", child_id));
        }

        // Check if child is already linked to another parent
        if let Some(child) = clients.get(&child_id) {
            if child.parent_session.is_some() {
                return Err("Child session is already linked to a parent".to_string());
            }
        }

        // Get parent's robot subscription first
        let parent_robot = clients.get(&parent_id).and_then(|p| p.subscribed_robot);

        // Link child to parent and inherit robot subscription
        if let Some(child) = clients.get_mut(&child_id) {
            child.parent_session = Some(parent_id);
            child.subscribed_robot = parent_robot;
        }

        // Add child to parent's child list
        if let Some(parent) = clients.get_mut(&parent_id) {
            parent.child_sessions.insert(child_id);
        }

        info!("Session {} linked as child of {}", child_id, parent_id);
        Ok(())
    }

    /// Unlink a child session from its parent.
    pub async fn unlink_child_session(&self, child_id: Uuid) -> Option<Uuid> {
        let mut clients = self.clients.write().await;

        let parent_id = clients.get(&child_id)?.parent_session?;

        // Remove from parent's child list
        if let Some(parent) = clients.get_mut(&parent_id) {
            parent.child_sessions.remove(&child_id);
        }

        // Clear child's parent reference
        if let Some(child) = clients.get_mut(&child_id) {
            child.parent_session = None;
        }

        info!("Session {} unlinked from parent {}", child_id, parent_id);
        Some(parent_id)
    }

    /// Check if a client has control (either directly or through parent).
    pub async fn has_control_or_inherited(&self, client_id: Uuid) -> bool {
        // Check direct control first
        if self.has_control(client_id).await {
            return true;
        }

        // Check if parent has control
        let clients = self.clients.read().await;
        if let Some(client) = clients.get(&client_id) {
            if let Some(parent_id) = client.parent_session {
                drop(clients); // Release read lock before checking control
                return self.has_control(parent_id).await;
            }
        }

        false
    }

    /// Get all child sessions for a parent.
    pub async fn get_child_sessions(&self, parent_id: Uuid) -> Vec<Uuid> {
        let clients = self.clients.read().await;
        clients.get(&parent_id)
            .map(|c| c.child_sessions.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get the parent session for a child.
    pub async fn get_parent_session(&self, child_id: Uuid) -> Option<Uuid> {
        let clients = self.clients.read().await;
        clients.get(&child_id)?.parent_session
    }
}

/// Robot session state - holds executor, control lock, and subscribed clients for a robot.
pub struct RobotSession {
    pub connection_id: i64,
    pub executor: Mutex<ProgramExecutor>,
    pub control_lock: RwLock<RobotControlLock>,
    pub subscribed_clients: RwLock<HashSet<Uuid>>,
}

impl RobotSession {
    pub fn new(connection_id: i64) -> Self {
        Self {
            connection_id,
            executor: Mutex::new(ProgramExecutor::new()),
            control_lock: RwLock::new(RobotControlLock::new()),
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

    /// Try to acquire control of this robot.
    pub async fn try_acquire_control(&self, client_id: Uuid) -> Result<Option<Uuid>, ControlError> {
        let mut lock = self.control_lock.write().await;
        lock.try_acquire(client_id)
    }

    /// Release control of this robot.
    pub async fn release_control(&self, client_id: Uuid) -> bool {
        let mut lock = self.control_lock.write().await;
        lock.release(client_id)
    }

    /// Check if client has control.
    pub async fn has_control(&self, client_id: Uuid) -> bool {
        let lock = self.control_lock.read().await;
        lock.is_holder(client_id)
    }

    /// Get current control holder.
    pub async fn control_holder(&self) -> Option<Uuid> {
        let lock = self.control_lock.read().await;
        lock.holder()
    }

    /// Update activity timestamp for control holder.
    pub async fn touch_control(&self, client_id: Uuid) -> bool {
        let mut lock = self.control_lock.write().await;
        lock.touch(client_id)
    }

    /// Force release control (e.g., on disconnect).
    pub async fn force_release_control(&self) -> Option<Uuid> {
        let mut lock = self.control_lock.write().await;
        lock.force_release()
    }

    /// Transfer control to another client.
    pub async fn transfer_control(&self, from: Uuid, to: Uuid) -> bool {
        let mut lock = self.control_lock.write().await;
        lock.transfer(from, to)
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
        ExecutionState::Loaded { program_id, total_lines } => ServerResponse::ExecutionStateChanged {
            state: "loaded".to_string(),
            program_id: Some(*program_id),
            current_line: Some(0),
            total_lines: Some(*total_lines),
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

