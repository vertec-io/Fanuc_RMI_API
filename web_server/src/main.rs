// WebSocket server that bridges between Fanuc driver and web clients
// Run with: cargo run --manifest-path web_app/Cargo_server.toml

use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig},
    dto,
    packets::PacketPriority,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{info, warn, error};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Connect to robot/simulator
    let driver_config = FanucDriverConfig {
        addr: "127.0.0.1".to_string(),
        port: 16001,
        max_messages: 30,
    };

    info!("Connecting to robot at {}:{}", driver_config.addr, driver_config.port);
    let driver = match FanucDriver::connect(driver_config).await {
        Ok(d) => {
            info!("✓ Connected to robot");
            d
        }
        Err(e) => {
            error!("✗ Failed to connect: {}", e);
            error!("  Make sure the simulator is running: cargo run -p sim -- --realtime");
            return;
        }
    };

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    driver.abort();
    driver.initialize();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let driver = Arc::new(driver);
    let (broadcast_tx, _) = broadcast::channel::<Vec<u8>>(100);
    let broadcast_tx = Arc::new(broadcast_tx);

    // Subscribe to driver responses and broadcast to all web clients
    let mut response_rx = driver.response_tx.subscribe();
    let broadcast_tx_clone = Arc::clone(&broadcast_tx);
    tokio::spawn(async move {
        while let Ok(response) = response_rx.recv().await {
            // Convert protocol response to DTO
            let dto_response: dto::ResponsePacket = response.into();
            
            // Serialize to binary
            if let Ok(binary) = bincode::serialize(&dto_response) {
                let _ = broadcast_tx_clone.send(binary);
            }
        }
    });

    // Periodic status polling
    let driver_clone = Arc::clone(&driver);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            let packet: fanuc_rmi::packets::SendPacket = dto::SendPacket::Command(dto::Command::FrcReadCartesianPosition(
                dto::FrcReadCartesianPosition { group: 1 }
            )).into();
            let _ = driver_clone.send_command(packet, PacketPriority::Low);

            let packet: fanuc_rmi::packets::SendPacket = dto::SendPacket::Command(dto::Command::FrcGetStatus).into();
            let _ = driver_clone.send_command(packet, PacketPriority::Low);
        }
    });

    // Start WebSocket server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:9000").await.unwrap();
    info!("🚀 WebSocket server listening on ws://127.0.0.1:9000");

    while let Ok((stream, addr)) = listener.accept().await {
        info!("New WebSocket connection from {}", addr);
        let driver = Arc::clone(&driver);
        let broadcast_rx = broadcast_tx.subscribe();
        
        tokio::spawn(handle_connection(stream, driver, broadcast_rx));
    }
}

async fn handle_connection(
    stream: tokio::net::TcpStream,
    driver: Arc<FanucDriver>,
    mut broadcast_rx: broadcast::Receiver<Vec<u8>>,
) {
    let ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            error!("WebSocket handshake failed: {}", e);
            return;
        }
    };

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Task to forward broadcast messages to this client
    let send_task = tokio::spawn(async move {
        while let Ok(binary) = broadcast_rx.recv().await {
            if ws_sender.send(Message::Binary(binary)).await.is_err() {
                break;
            }
        }
    });

    // Task to handle incoming messages from client
    let recv_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(Message::Binary(data)) => {
                    // Deserialize command from client
                    if let Ok(dto_packet) = bincode::deserialize::<dto::SendPacket>(&data) {
                        info!("Received command from client: {:?}", dto_packet);
                        let packet: fanuc_rmi::packets::SendPacket = dto_packet.into();
                        let _ = driver.send_command(packet, PacketPriority::Standard);
                    } else {
                        warn!("Failed to deserialize packet from client");
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

    info!("WebSocket connection closed");
}

