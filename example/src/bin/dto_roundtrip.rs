// DTO roundtrip demonstration
// NOTE: See live_example() below for a connected-driver version of this flow.
// It sends a real instruction and roundtrips the response via DTO+bincode.


//
// This example shows how to:
// 1) Take a protocol ResponsePacket (the JSON/robot-wire format)
// 2) Convert it into a DTO ResponsePacket using fanuc_rmi::to_dto
// 3) Binary-encode it with bincode and then decode it back
// 4) Convert the DTO back into its protocol type
//
// In production, you'll typically receive protocol packets from the driver's
// broadcast channel (driver.response_tx.subscribe()) and then immediately
// convert to DTO and send over your app's binary channel.
//
// For a fully live demo, uncomment the connection code below and run against
// a controller/emulator that yields responses.

use fanuc_rmi::packets::{ResponsePacket, InstructionResponse};
use fanuc_rmi::instructions::FrcWaitTimeResponse;

#[tokio::main]
async fn main() {
    println!("DTO Roundtrip Demo\n");

    // --- Option A: Subscribe to the driver's broadcast channel (live system) ---
    // let cfg = fanuc_rmi::drivers::FanucDriverConfig {
    //     addr: "127.0.0.1".into(),
    //     port: 16001,
    //     max_messages: 100,
    // };
    // let driver = FanucDriver::connect(cfg).await.expect("connect");
    // let mut rx = driver.response_tx.subscribe();
    // let protocol_packet = rx.recv().await.expect("receive protocol packet");

    // --- Option B: Synthetic example packet (works without a controller) ---
    let protocol_packet = ResponsePacket::InstructionResponse(
        InstructionResponse::FrcWaitTime(FrcWaitTimeResponse {
            error_id: 0,
            sequence_id: 42,
        }),
    );

    println!("1) Protocol packet obtained: {:?}\n", protocol_packet);

    // 2) Convert protocol -> DTO (removes serde renaming/tagging for binary use)
    let dto_packet: fanuc_rmi::dto::ResponsePacket = protocol_packet.into();
    println!("2) Converted to DTO: {:?}\n", dto_packet);

    // 3) Binary encode & decode DTO
    let encoded = bincode::serialize(&dto_packet).expect("bincode serialize");
    println!("3) Bincode encoded ({} bytes)", encoded.len());

    let decoded: fanuc_rmi::dto::ResponsePacket = bincode::deserialize(&encoded).expect("bincode decode");
    println!("   Decoded DTO: {:?}\n", decoded);

    // 4) Convert DTO -> protocol (for consumers expecting JSON/protocol types)
    let protocol_back: ResponsePacket = decoded.into();
    println!("4) Converted DTO back to protocol: {:?}\n", protocol_back);

// LIVE version shows a full flow using a real driver connected to the simulator
// (cargo run -p sim in another terminal). It sends FrcWaitTime, waits for the
// protocol response, then performs DTO+bincode roundtrip and prints the result.
#[allow(dead_code)]
async fn live_example() {
    use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig};
    use fanuc_rmi::packets::{Instruction, PacketPriority, SendPacket};
    use fanuc_rmi::instructions::FrcWaitTime;
    use std::time::Duration;
    use fanuc_rmi::drivers::LogLevel;

    let cfg = FanucDriverConfig {
        addr: "127.0.0.1".into(),
        port: 16001,
        max_messages: 100,
        log_level: LogLevel::Info,
    };

    let driver = match FanucDriver::connect(cfg).await {
        Ok(d) => {
            println!("✓ Connected to controller/simulator");
            d
        }
        Err(e) => {
            println!("✗ Failed to connect: {}", e);
            println!("  Start the simulator with: cargo run -p sim");
            return;
        }
    };

    let mut rx = driver.response_tx.subscribe();

    // Initialize and wait for response
    match driver.initialize().await {
        Ok(response) => {
            if response.error_id == 0 {
                println!("✓ Initialize successful");
            } else {
                eprintln!("✗ Initialize failed with error: {}", response.error_id);
                return;
            }
        }
        Err(e) => {
            eprintln!("✗ Initialize error: {}", e);
            return;
        }
    }

    let wait = FrcWaitTime::new(123, 1.0);
    let pkt = SendPacket::Instruction(Instruction::FrcWaitTime(wait));
    println!("1) Sending FrcWaitTime(seq=123, time=1.0s)");
    if let Err(e) = driver.send_packet(pkt, PacketPriority::Standard) {
        println!("   Failed to send command: {}", e);
        return;
    }

    println!("2) Waiting for protocol response...");
    let protocol_packet = match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
        Ok(Ok(packet)) => {
            println!("   Received: {:?}", packet);
            packet
        }
        Ok(Err(e)) => {
            println!("   Channel error: {}", e);
            return;
        }
        Err(_) => {
            println!("   Timeout waiting for response");
            return;
        }
    };

    let dto_packet: fanuc_rmi::dto::ResponsePacket = protocol_packet.clone().into();
    println!("3) Converted to DTO: {:?}", dto_packet);

    let encoded = bincode::serialize(&dto_packet).expect("bincode serialize");
    println!("4) Bincode encoded ({} bytes)", encoded.len());

    let decoded: fanuc_rmi::dto::ResponsePacket = bincode::deserialize(&encoded).expect("bincode decode");
    println!("   Decoded DTO: {:?}", decoded);

    let protocol_back: fanuc_rmi::packets::ResponsePacket = decoded.into();
    println!("5) Back to protocol: {:?}", protocol_back);

    assert_eq!(protocol_packet, protocol_back);
    println!("✓ Live roundtrip successful");

    // Disconnect and wait for response
    match driver.disconnect().await {
        Ok(response) => {
            if response.error_id == 0 {
                println!("✓ Disconnect successful");
            } else {
                eprintln!("✗ Disconnect failed with error: {}", response.error_id);
            }
        }
        Err(e) => eprintln!("✗ Disconnect error: {}", e),
    }
    println!("✓ Disconnected");
}


    println!("Done.");
}

