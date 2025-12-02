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

/// Shared robot connection state
pub struct RobotConnection {
    pub driver: Option<Arc<FanucDriver>>,
    pub connected: bool,
    pub robot_addr: String,
    pub robot_port: u32,
    /// Saved robot connection configuration from database (for defaults)
    pub saved_connection: Option<database::RobotConnection>,
    /// Currently active UFrame number on the robot
    pub active_uframe: u8,
    /// Currently active UTool number on the robot
    pub active_utool: u8,
}

impl RobotConnection {
    pub fn new(robot_addr: String, robot_port: u32) -> Self {
        Self {
            driver: None,
            connected: false,
            robot_addr,
            robot_port,
            saved_connection: None,
            active_uframe: 0,
            active_utool: 1,
        }
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
            log_level: LogLevel::Error,
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
                        Ok(())
                    }
                    Err(e) => {
                        warn!("⚠ Robot initialization failed: {}", e);
                        // Still connect, but warn that initialization failed
                        self.driver = Some(Arc::new(d));
                        self.connected = true;
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
    let robot_addr = std::env::var("FANUC_ROBOT_ADDR").unwrap_or_else(|_| "127.0.0.1".to_string());
    let robot_port = std::env::var("FANUC_ROBOT_PORT")
        .ok()
        .and_then(|p| p.parse::<u32>().ok())
        .unwrap_or(16001);
    let websocket_port = std::env::var("WEBSOCKET_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(9000);

    // Create robot connection (may or may not connect on startup)
    let robot_connection = Arc::new(RwLock::new(RobotConnection::new(robot_addr.clone(), robot_port)));

    // Try to connect to robot, but don't exit if it fails
    {
        let mut conn = robot_connection.write().await;
        if let Err(e) = conn.connect().await {
            warn!("⚠ Could not connect to robot on startup: {}", e);
            warn!("  Web server will continue running. Connect via UI when robot is available.");
            warn!("  Or start the simulator: cargo run -p sim -- --realtime");
        }
    }

    let executor = Arc::new(tokio::sync::Mutex::new(ProgramExecutor::new()));
    let client_manager = Arc::new(ClientManager::new());
    let (broadcast_tx, _) = broadcast::channel::<Vec<u8>>(100);
    let broadcast_tx = Arc::new(broadcast_tx);

    // Start response broadcast task - forwards robot responses to all WebSocket clients
    let robot_connection_clone = Arc::clone(&robot_connection);
    let broadcast_tx_clone = Arc::clone(&broadcast_tx);
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
                }
            } else {
                current_driver_id = None;
            }

            // Wait before trying again
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
    info!("   Environment variables:");
    info!("   - FANUC_ROBOT_ADDR={}", robot_addr);
    info!("   - FANUC_ROBOT_PORT={}", robot_port);
    info!("   - WEBSOCKET_PORT={}", websocket_port);

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

