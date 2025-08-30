// Run with: cargo run -p example --features DTO --example dto_roundtrip
// Note: Requires a running robot or an emulator that provides Fanuc responses.

use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig};
use fanuc_rmi::packets::ResponsePacket;

#[tokio::main]
async fn main() {
    println!("Starting DTO roundtrip example...\n");

    let cfg = FanucDriverConfig { addr: "127.0.0.1".into(), port: 16001, max_messages: 100 };
    let driver = match FanucDriver::connect(cfg).await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to connect: {e}");
            return;
        }
    };

    let mut rx = driver.response_tx.subscribe();

    println!("Waiting for a response from the driver...\n");
    match rx.recv().await {
        Ok(ResponsePacket::InstructionResponse(pkt)) => {
            println!("Received protocol packet: InstructionResponse");
            println!("  sequence_id={} error_id={}", pkt.get_sequence_id(), pkt.get_error_id());
        }
        Ok(ResponsePacket::CommandResponse(_)) => {
            println!("Received protocol packet: CommandResponse");
        }
        Ok(ResponsePacket::CommunicationResponse(_)) => {
            println!("Received protocol packet: CommunicationResponse");
        }
        Err(e) => {
            eprintln!("Channel error: {e}");
            return;
        }
    }

    // For demonstration, we convert the last packet we received again via subscribe().
    // In a real app, you would convert and forward each packet as it arrives.
    if let Ok(protocol_packet) = rx.recv().await {
        println!("\n--- DTO Conversion + Bincode Roundtrip ---");
        // Convert protocol -> DTO
        let dto_packet = fanuc_rmi::to_dto(protocol_packet);
        println!("Converted to DTO: type preserved, tags/renames removed");

        // Binary encode the DTO
        let encoded = bincode::serialize(&dto_packet).expect("bincode serialize");
        println!("Encoded {} bytes via bincode", encoded.len());

        // Binary decode the DTO
        let decoded: fanuc_rmi::dto::ResponsePacket = bincode::deserialize(&encoded).expect("bincode deserialize");
        println!("Decoded DTO successfully\n");

        // If needed on the other side: DTO -> protocol
        let protocol_back: ResponsePacket = decoded.into();
        println!("Converted DTO back into protocol type");

        // Show a couple of details to confirm behavior
        match protocol_back {
            ResponsePacket::InstructionResponse(pkt) => {
                println!("Back to protocol: InstructionResponse seq={} err={}", pkt.get_sequence_id(), pkt.get_error_id());
            }
            ResponsePacket::CommandResponse(_) => println!("Back to protocol: CommandResponse"),
            ResponsePacket::CommunicationResponse(_) => println!("Back to protocol: CommunicationResponse"),
        }
        println!("--- end ---");
    }
}

