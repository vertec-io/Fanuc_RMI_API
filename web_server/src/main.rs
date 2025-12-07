// WebSocket server that bridges between Fanuc driver and web clients
// Run with: cargo run -p web_server

mod api_types;
mod database;
mod handlers;
mod program_executor;
mod program_parser;
mod session;

use handlers::handle_request;
use api_types::{ClientRequest, ServerResponse};
use database::Database;
use program_executor::ProgramExecutor;
use session::ClientManager;
use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig, LogLevel},
    dto,
    packets::PacketPriority,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{info, warn, error};

/// Active configuration state (runtime, not persisted).
/// Tracks which configuration is loaded and whether it has been modified.
#[derive(Debug, Clone)]
pub struct ActiveConfiguration {
    /// ID of the saved configuration this was loaded from (None = custom/unsaved)
    pub loaded_from_id: Option<i64>,
    /// Name of the loaded configuration
    pub loaded_from_name: Option<String>,
    /// Whether the active config has been modified from the loaded state
    pub modified: bool,
    /// Current UFrame number
    pub u_frame_number: i32,
    /// Current UTool number
    pub u_tool_number: i32,
    /// Arm configuration - Front(1)/Back(0)
    pub front: i32,
    /// Arm configuration - Up(1)/Down(0)
    pub up: i32,
    /// Arm configuration - Left(1)/Right(0)
    pub left: i32,
    /// Wrist configuration - Flip(1)/NoFlip(0)
    pub flip: i32,
    /// J4 turn number
    pub turn4: i32,
    /// J5 turn number
    pub turn5: i32,
    /// J6 turn number
    pub turn6: i32,
}

impl Default for ActiveConfiguration {
    fn default() -> Self {
        Self {
            loaded_from_id: None,
            loaded_from_name: None,
            modified: false,
            u_frame_number: 0,
            u_tool_number: 1,
            front: 1,  // Front
            up: 1,     // Up
            left: 0,   // Right
            flip: 0,   // NoFlip
            turn4: 0,
            turn5: 0,
            turn6: 0,
        }
    }
}

impl ActiveConfiguration {
    /// Create from a saved RobotConfiguration
    pub fn from_saved(config: &database::RobotConfiguration) -> Self {
        Self {
            loaded_from_id: Some(config.id),
            loaded_from_name: Some(config.name.clone()),
            modified: false,
            u_frame_number: config.u_frame_number,
            u_tool_number: config.u_tool_number,
            front: config.front,
            up: config.up,
            left: config.left,
            flip: config.flip,
            turn4: config.turn4,
            turn5: config.turn5,
            turn6: config.turn6,
        }
    }

    /// Create from robot connection defaults (when no saved config exists)
    pub fn from_robot_defaults(conn: &database::RobotConnection) -> Self {
        Self {
            loaded_from_id: None,
            loaded_from_name: Some("Robot Defaults".to_string()),
            modified: false,
            u_frame_number: conn.default_uframe,
            u_tool_number: conn.default_utool,
            front: conn.default_front,
            up: conn.default_up,
            left: conn.default_left,
            flip: conn.default_flip,
            turn4: conn.default_turn4,
            turn5: conn.default_turn5,
            turn6: conn.default_turn6,
        }
    }
}

/// Shared robot connection state
pub struct RobotConnection {
    pub driver: Option<Arc<FanucDriver>>,
    pub connected: bool,
    pub robot_addr: String,
    pub robot_port: u32,
    /// Saved robot connection configuration from database (for defaults)
    pub saved_connection: Option<database::RobotConnection>,
    /// Active configuration state (runtime, not persisted)
    pub active_configuration: ActiveConfiguration,
    /// Whether the TP program is initialized (FRC_Initialize was successful)
    /// This must be true to send motion commands. It becomes false after:
    /// - FRC_Abort is called
    /// - Robot disconnects
    /// - Stop program is called
    pub tp_program_initialized: bool,
}

impl RobotConnection {
    pub fn new(robot_addr: String, robot_port: u32) -> Self {
        Self {
            driver: None,
            connected: false,
            robot_addr,
            robot_port,
            saved_connection: None,
            active_configuration: ActiveConfiguration::default(),
            tp_program_initialized: false,
        }
    }

    /// Get the currently active UFrame number
    pub fn active_uframe(&self) -> u8 {
        self.active_configuration.u_frame_number as u8
    }

    /// Get the currently active UTool number
    pub fn active_utool(&self) -> u8 {
        self.active_configuration.u_tool_number as u8
    }

    pub async fn connect(&mut self) -> Result<(), String> {
        // Disconnect from existing robot first if connected
        if self.connected {
            info!("Disconnecting from current robot before connecting to new one");
            self.disconnect();
            // Give the old connection time to clean up
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        let driver_config = FanucDriverConfig {
            addr: self.robot_addr.clone(),
            port: self.robot_port,
            max_messages: 30,
            log_level: LogLevel::Debug,
        };

        info!("Connecting to robot at {}:{}", driver_config.addr, driver_config.port);
        match FanucDriver::connect(driver_config).await {
            Ok(d) => {
                info!("✓ Connected to robot");
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                // Smart initialization - checks status first, only aborts if needed
                match d.startup_sequence().await {
                    Ok(()) => {
                        info!("✓ Robot initialization complete");
                        self.driver = Some(Arc::new(d));
                        self.connected = true;
                        self.tp_program_initialized = true;
                        Ok(())
                    }
                    Err(e) => {
                        warn!("⚠ Robot initialization failed: {}", e);
                        // Still connect, but warn that initialization failed
                        self.driver = Some(Arc::new(d));
                        self.connected = true;
                        self.tp_program_initialized = false; // Not initialized - cannot send motions
                        Ok(())
                    }
                }
            }
            Err(e) => {
                error!("✗ Failed to connect: {}", e);
                self.connected = false;
                self.driver = None;
                Err(format!("Failed to connect: {}", e))
            }
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(ref driver) = self.driver {
            info!("Disconnecting from robot at {}:{}", self.robot_addr, self.robot_port);
            // The driver will clean up its connections when dropped
            drop(driver.clone());
        }
        self.driver = None;
        self.connected = false;
        self.tp_program_initialized = false;
    }

    /// Re-initialize the TP program after an abort.
    /// This should be called after FRC_Abort to allow motion commands again.
    pub async fn reinitialize_tp(&mut self) -> Result<(), String> {
        if !self.connected {
            return Err("Not connected to robot".to_string());
        }

        let driver = self.driver.as_ref().ok_or("No driver available")?;

        info!("Re-initializing TP program...");
        match driver.initialize().await {
            Ok(response) => {
                if response.error_id == 0 {
                    info!("✓ TP program re-initialized successfully");
                    self.tp_program_initialized = true;
                    Ok(())
                } else {
                    let msg = format!("Initialize failed with error: {}", response.error_id);
                    warn!("{}", msg);
                    self.tp_program_initialized = false;
                    Err(msg)
                }
            }
            Err(e) => {
                let msg = format!("Failed to initialize: {}", e);
                warn!("{}", msg);
                self.tp_program_initialized = false;
                Err(msg)
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Initialize database
    let db_path = std::env::var("FANUC_DB_PATH")
        .unwrap_or_else(|_| Database::DEFAULT_PATH.to_string());

    let db = match Database::new(&db_path) {
        Ok(db) => {
            info!("✓ Database initialized at {}", db_path);
            Arc::new(tokio::sync::Mutex::new(db))
        }
        Err(e) => {
            error!("✗ Failed to initialize database: {}", e);
            return;
        }
    };

    // Load configuration from environment variables with defaults
    // Note: FANUC_ROBOT_ADDR and FANUC_ROBOT_PORT are only used as defaults for the
    // RobotConnection struct. The server does NOT auto-connect on startup.
    // Users must explicitly connect via the UI by selecting a saved robot connection.
    let robot_addr = std::env::var("FANUC_ROBOT_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
    let robot_port = std::env::var("FANUC_ROBOT_PORT")
        .ok()
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(16001);
    let websocket_port = std::env::var("WEBSOCKET_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(9000);

    // Create robot connection in disconnected state
    // Users must explicitly connect via the UI by selecting a saved robot connection
    let robot_connection = Arc::new(RwLock::new(RobotConnection::new(robot_addr.clone(), robot_port)));
    info!("Robot connection initialized (not connected - use UI to connect)");

    let executor = Arc::new(tokio::sync::Mutex::new(ProgramExecutor::new()));
    let client_manager = Arc::new(ClientManager::new());
    let (broadcast_tx, _) = broadcast::channel::<Vec<u8>>(100);
    let broadcast_tx = Arc::new(broadcast_tx);

    // Start response broadcast task - forwards robot responses to all WebSocket clients
    let robot_connection_clone = Arc::clone(&robot_connection);
    let broadcast_tx_clone = Arc::clone(&broadcast_tx);
    let client_manager_broadcast = Arc::clone(&client_manager);
    let executor_broadcast = Arc::clone(&executor);
    tokio::spawn(async move {
        // Track which driver we're currently subscribed to (by its channel address)
        let mut current_driver_id: Option<usize> = None;

        loop {
            // Get current driver
            let driver_opt = {
                let conn = robot_connection_clone.read().await;
                conn.driver.clone()
            };

            if let Some(driver) = driver_opt {
                // Check if this is a different driver than we were subscribed to
                let driver_id = Arc::as_ptr(&driver) as usize;

                if current_driver_id != Some(driver_id) {
                    // New driver - subscribe to its response channel
                    info!("Subscribing to new robot driver response channel");
                    current_driver_id = Some(driver_id);
                }

                let mut response_rx = driver.response_tx.subscribe();

                // Broadcast responses, but periodically check if driver changed
                loop {
                    // Use select to either receive a message or timeout to check for driver change
                    tokio::select! {
                        result = response_rx.recv() => {
                            match result {
                                Ok(response) => {
                                    let dto_response: dto::ResponsePacket = response.into();
                                    if let Ok(binary) = bincode::serialize(&dto_response) {
                                        let _ = broadcast_tx_clone.send(binary);
                                    }
                                }
                                Err(broadcast::error::RecvError::Closed) => {
                                    warn!("Driver response channel closed - robot disconnected");
                                    current_driver_id = None;
                                    break;
                                }
                                Err(broadcast::error::RecvError::Lagged(n)) => {
                                    warn!("Lagged {} messages", n);
                                }
                            }
                        }
                        _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                            // Periodically check if the driver has changed
                            let new_driver_opt = {
                                let conn = robot_connection_clone.read().await;
                                conn.driver.clone()
                            };

                            match new_driver_opt {
                                Some(new_driver) => {
                                    let new_id = Arc::as_ptr(&new_driver) as usize;
                                    if Some(new_id) != current_driver_id {
                                        info!("Robot driver changed - resubscribing to new channel");
                                        break; // Exit inner loop to resubscribe
                                    }
                                }
                                None => {
                                    info!("Robot driver disconnected");
                                    current_driver_id = None;
                                    break;
                                }
                            }
                        }
                    }
                }

                // Mark as disconnected if driver channel closed (not just switched)
                if current_driver_id.is_none() {
                    let mut conn = robot_connection_clone.write().await;
                    conn.connected = false;

                    // Unload any running program - it's no longer valid
                    {
                        let mut exec = executor_broadcast.lock().await;
                        if exec.is_running() {
                            exec.stop();
                            warn!("Stopped running program due to robot disconnect");
                        }
                        exec.reset();
                        warn!("Reset executor due to robot disconnect");
                    }

                    // Broadcast robot disconnected to all clients
                    let disconnect_response = ServerResponse::RobotDisconnected {
                        reason: "Robot connection lost".to_string(),
                    };
                    client_manager_broadcast.broadcast_all(&disconnect_response).await;

                    // Broadcast execution state change (program unloaded)
                    let state_response = ServerResponse::ExecutionStateChanged {
                        state: "idle".to_string(),
                        program_id: None,
                        current_line: None,
                        total_lines: None,
                        message: Some("Program unloaded due to robot disconnect".to_string()),
                    };
                    client_manager_broadcast.broadcast_all(&state_response).await;
                    warn!("Broadcasted RobotDisconnected and ExecutionStateChanged to all clients");
                }
            } else {
                current_driver_id = None;
            }

            // Wait before trying again
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    // Start error broadcast task - forwards protocol errors to all WebSocket clients
    let robot_connection_error = Arc::clone(&robot_connection);
    let client_manager_error = Arc::clone(&client_manager);
    tokio::spawn(async move {
        let mut current_driver_id: Option<usize> = None;

        loop {
            let driver_opt = {
                let conn = robot_connection_error.read().await;
                conn.driver.clone()
            };

            if let Some(driver) = driver_opt {
                let driver_id = Arc::as_ptr(&driver) as usize;

                if current_driver_id != Some(driver_id) {
                    info!("Subscribing to new robot driver error channel");
                    current_driver_id = Some(driver_id);
                }

                let mut error_rx = driver.error_tx.subscribe();

                loop {
                    tokio::select! {
                        result = error_rx.recv() => {
                            match result {
                                Ok(protocol_error) => {
                                    warn!("Protocol error: {} - {}", protocol_error.error_type, protocol_error.message);
                                    let response = ServerResponse::RobotError {
                                        error_type: protocol_error.error_type,
                                        message: protocol_error.message,
                                        error_id: None,
                                    };
                                    client_manager_error.broadcast_all(&response).await;
                                }
                                Err(broadcast::error::RecvError::Closed) => {
                                    current_driver_id = None;
                                    break;
                                }
                                Err(broadcast::error::RecvError::Lagged(n)) => {
                                    warn!("Error channel lagged {} messages", n);
                                }
                            }
                        }
                        _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                            // Check if driver changed
                            let new_driver_opt = {
                                let conn = robot_connection_error.read().await;
                                conn.driver.clone()
                            };
                            match new_driver_opt {
                                Some(new_driver) => {
                                    let new_id = Arc::as_ptr(&new_driver) as usize;
                                    if Some(new_id) != current_driver_id {
                                        break;
                                    }
                                }
                                None => {
                                    current_driver_id = None;
                                    break;
                                }
                            }
                        }
                    }
                }
            } else {
                current_driver_id = None;
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    // Periodic status polling task - uses High priority so polling interleaves with motion commands
    let robot_connection_clone = Arc::clone(&robot_connection);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            let driver_opt = {
                let conn = robot_connection_clone.read().await;
                conn.driver.clone()
            };

            if let Some(driver) = driver_opt {
                // Use High priority so these get pushed to front of queue, interleaving with motion commands
                // Note: Commands (not Instructions) don't consume the 8-slot instruction buffer
                let packet: fanuc_rmi::packets::SendPacket = dto::SendPacket::Command(dto::Command::FrcReadCartesianPosition(
                    dto::FrcReadCartesianPosition { group: 1 }
                )).into();
                let _ = driver.send_packet(packet, PacketPriority::High);

                let packet: fanuc_rmi::packets::SendPacket = dto::SendPacket::Command(dto::Command::FrcReadJointAngles(
                    dto::FrcReadJointAngles { group: 1 }
                )).into();
                let _ = driver.send_packet(packet, PacketPriority::High);

                let packet: fanuc_rmi::packets::SendPacket = dto::SendPacket::Command(dto::Command::FrcGetStatus).into();
                let _ = driver.send_packet(packet, PacketPriority::High);
            }
        }
    });

    // Control lock timeout checker - runs every 30 seconds
    let client_manager_timeout = Arc::clone(&client_manager);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            // Check if control has timed out
            if let Some(timed_out_client) = client_manager_timeout.check_control_timeout().await {
                info!("Control lock timed out for client {}", timed_out_client);
                // Notify the timed-out client that they lost control
                let response = ServerResponse::ControlLost {
                    reason: "Control released due to inactivity timeout (10 minutes)".to_string(),
                };
                client_manager_timeout.send_to_client(timed_out_client, &response).await;
                // Notify all clients that control changed
                let changed_response = ServerResponse::ControlChanged { holder_id: None };
                client_manager_timeout.broadcast_all(&changed_response).await;
            }
        }
    });

    // Start WebSocket server
    let websocket_addr = format!("0.0.0.0:{}", websocket_port);
    let ws_listener = tokio::net::TcpListener::bind(&websocket_addr).await.unwrap();
    info!("🚀 WebSocket server listening on ws://{}", websocket_addr);
    info!("   No robot connected - use UI to connect to a saved robot connection");
    info!("   Environment: WEBSOCKET_PORT={}", websocket_port);

    while let Ok((stream, addr)) = ws_listener.accept().await {
        info!("New WebSocket connection from {}", addr);
        let robot_connection = Arc::clone(&robot_connection);
        let db = Arc::clone(&db);
        let executor = Arc::clone(&executor);
        let client_manager = Arc::clone(&client_manager);
        let broadcast_rx = broadcast_tx.subscribe();

        tokio::spawn(handle_connection(stream, robot_connection, db, executor, client_manager, broadcast_rx));
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    robot_connection: Arc<RwLock<RobotConnection>>,
    db: Arc<tokio::sync::Mutex<Database>>,
    executor: Arc<tokio::sync::Mutex<ProgramExecutor>>,
    client_manager: Arc<ClientManager>,
    mut broadcast_rx: broadcast::Receiver<Vec<u8>>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed: {}", e);
            return;
        }
    };

    let (ws_sender, mut ws_receiver) = ws_stream.split();
    let ws_sender = Arc::new(tokio::sync::Mutex::new(ws_sender));

    // Register this client with the client manager
    let client_id = client_manager.register(Arc::clone(&ws_sender)).await;
    info!("Client {} connected", client_id);

    // Task to forward broadcast messages to this client
    let ws_sender_clone = Arc::clone(&ws_sender);
    let send_task = tokio::spawn(async move {
        while let Ok(binary) = broadcast_rx.recv().await {
            let mut sender = ws_sender_clone.lock().await;
            if sender.send(Message::Binary(binary)).await.is_err() {
                break;
            }
        }
    });

    // Task to handle incoming messages from client
    let ws_sender_clone = Arc::clone(&ws_sender);
    let robot_connection_clone = Arc::clone(&robot_connection);
    let client_manager_clone = Arc::clone(&client_manager);
    let client_id_for_recv = client_id; // Copy for recv_task
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    // Binary = Robot protocol (bincode-encoded DTO)
                    // Requires control to send robot commands
                    let has_control = client_manager_clone.has_control(client_id_for_recv).await;
                    if !has_control {
                        warn!("Client {} tried to send robot command without control", client_id_for_recv);
                        // Send error response back to client
                        let error_response = ServerResponse::Error {
                            message: "You must have control to send robot commands. Request control first.".to_string(),
                        };
                        let error_json = serde_json::to_string(&error_response).unwrap_or_default();
                        let mut sender = ws_sender_clone.lock().await;
                        let _ = sender.send(Message::Text(error_json)).await;
                        continue;
                    }

                    // Touch activity to reset control timeout
                    client_manager_clone.touch_control(client_id_for_recv).await;

                    if let Ok(dto_packet) = bincode::deserialize::<dto::SendPacket>(&data) {
                        info!("Received robot command from client: {:?}", dto_packet);
                        let driver_opt = {
                            let conn = robot_connection_clone.read().await;
                            conn.driver.clone()
                        };
                        if let Some(driver) = driver_opt {
                            let packet: fanuc_rmi::packets::SendPacket = dto_packet.into();
                            let _ = driver.send_packet(packet, PacketPriority::Standard);
                        } else {
                            warn!("Robot not connected - cannot send command");
                            // Send error response back to client
                            let error_response = ServerResponse::Error {
                                message: "Robot not connected".to_string(),
                            };
                            let error_json = serde_json::to_string(&error_response).unwrap_or_default();
                            let mut sender = ws_sender_clone.lock().await;
                            let _ = sender.send(Message::Text(error_json)).await;
                        }
                    } else {
                        warn!("Failed to deserialize binary packet from client");
                    }
                }
                Ok(Message::Text(text)) => {
                    // Text = API request (JSON)
                    match serde_json::from_str::<ClientRequest>(&text) {
                        Ok(request) => {
                            info!("Received API request: {:?}", request);
                            // Get driver if connected
                            let driver_opt = {
                                let conn = robot_connection_clone.read().await;
                                conn.driver.clone()
                            };
                            let response = handle_request(
                                request,
                                Arc::clone(&db),
                                driver_opt,
                                Some(Arc::clone(&executor)),
                                Some(Arc::clone(&robot_connection_clone)),
                                Some(Arc::clone(&client_manager_clone)),
                                Some(client_id_for_recv),
                            ).await;
                            let response_json = serde_json::to_string(&response).unwrap_or_else(|e| {
                                format!(r#"{{"type":"error","message":"Serialization error: {}"}}"#, e)
                            });
                            let mut sender = ws_sender_clone.lock().await;
                            if sender.send(Message::Text(response_json)).await.is_err() {
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("Failed to parse API request: {} - {}", e, text);
                            let error_response = ServerResponse::Error {
                                message: format!("Invalid request: {}", e)
                            };
                            let response_json = serde_json::to_string(&error_response).unwrap();
                            let mut sender = ws_sender_clone.lock().await;
                            let _ = sender.send(Message::Text(response_json)).await;
                        }
                    }
                }
                Ok(Message::Close(_)) => break,
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Wait for either task to finish
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    // Unregister client when connection closes
    client_manager.unregister(client_id).await;
    info!("WebSocket connection closed for client {}", client_id);
}

