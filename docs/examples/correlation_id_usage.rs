// Example: Using the Correlation ID System
// 
// This file demonstrates the three main patterns for using the correlation ID system
// to track instruction sequence IDs and responses.

use fanuc_rmi::{
    FanucDriver, FanucDriverConfig,
    packets::{SendPacket, PacketPriority, ResponsePacket, SentInstructionInfo},
    instructions::{Instruction, FrcLinearMotion},
    Configuration, Position, SpeedType, TermType,
};

// ============================================================================
// PATTERN 1: Simple - Send and Wait in One Call
// ============================================================================
// Best for: Code that sends an instruction and immediately waits for completion
async fn pattern_1_simple(driver: &FanucDriver) -> Result<(), String> {
    let instruction = create_example_instruction();
    
    // Send and wait in one call - returns the actual sequence ID
    let sequence_id = driver.send_and_wait_for_completion(
        SendPacket::Instruction(instruction),
        PacketPriority::Standard
    ).await?;
    
    println!("✅ Instruction {} completed successfully", sequence_id);
    Ok(())
}

// ============================================================================
// PATTERN 2: Flexible - Send Now, Wait Later
// ============================================================================
// Best for: Code that needs to send first, then wait later (e.g., background tasks)
async fn pattern_2_flexible(driver: &FanucDriver) -> Result<(), String> {
    let instruction = create_example_instruction();
    
    // Send and get correlation ID
    let correlation_id = driver.send_command(
        SendPacket::Instruction(instruction),
        PacketPriority::Standard
    )?;
    
    println!("📤 Sent instruction with correlation ID: {}", correlation_id);
    
    // Do other work here...
    
    // Later, wait for completion using correlation ID
    let sequence_id = driver.wait_on_correlation_completion(correlation_id).await?;
    
    println!("✅ Instruction {} completed successfully", sequence_id);
    Ok(())
}

// ============================================================================
// PATTERN 3: Advanced - Manual Correlation with Full Control
// ============================================================================
// Best for: Tracking multiple instructions, accessing full response data
async fn pattern_3_advanced(driver: &FanucDriver) -> Result<(), String> {
    // Subscribe to both sent notifications and responses
    let mut sent_rx = driver.sent_instruction_tx.subscribe();
    let mut response_rx = driver.response_tx.subscribe();
    
    let instruction = create_example_instruction();
    
    // Send and get correlation ID
    let correlation_id = driver.send_command(
        SendPacket::Instruction(instruction),
        PacketPriority::Standard
    )?;
    
    println!("📤 Sent instruction with correlation ID: {}", correlation_id);
    
    // Step 1: Wait for sent notification to get sequence ID
    let sequence_id = loop {
        match sent_rx.recv().await {
            Ok(sent_info) if sent_info.correlation_id == correlation_id => {
                println!("📨 Instruction sent with sequence ID: {}", sent_info.sequence_id);
                println!("   Timestamp: {:?}", sent_info.timestamp);
                break sent_info.sequence_id;
            }
            Ok(_) => continue, // Not our instruction
            Err(e) => return Err(format!("Failed to receive sent notification: {}", e)),
        }
    };
    
    // Step 2: Wait for response with matching sequence ID
    loop {
        match response_rx.recv().await {
            Ok(ResponsePacket::InstructionResponse(instr_resp)) => {
                if instr_resp.get_sequence_id() == sequence_id {
                    println!("✅ Received response for sequence {}: {:?}", sequence_id, instr_resp);
                    
                    // Check for errors
                    if instr_resp.get_error_id() != 0 {
                        return Err(format!("Instruction failed with error: {}", instr_resp.get_error_id()));
                    }
                    
                    break;
                }
            }
            Ok(_) => continue, // Not an instruction response or wrong sequence ID
            Err(e) => return Err(format!("Failed to receive response: {}", e)),
        }
    }
    
    Ok(())
}

// ============================================================================
// PATTERN 4: Batch Processing - Track Multiple Instructions
// ============================================================================
// Best for: Sending multiple instructions and tracking all of them
async fn pattern_4_batch(driver: &FanucDriver) -> Result<(), String> {
    use std::collections::HashMap;
    
    let mut sent_rx = driver.sent_instruction_tx.subscribe();
    let mut response_rx = driver.response_tx.subscribe();
    
    // Send multiple instructions
    let mut correlation_ids = Vec::new();
    for i in 0..5 {
        let instruction = create_example_instruction();
        let correlation_id = driver.send_command(
            SendPacket::Instruction(instruction),
            PacketPriority::Standard
        )?;
        correlation_ids.push(correlation_id);
        println!("📤 Sent instruction {} with correlation ID: {}", i, correlation_id);
    }
    
    // Track correlation_id -> sequence_id mapping
    let mut correlation_map: HashMap<u64, u32> = HashMap::new();
    let mut completed = 0;
    
    loop {
        tokio::select! {
            // Listen for sent notifications
            Ok(sent_info) = sent_rx.recv() => {
                if correlation_ids.contains(&sent_info.correlation_id) {
                    println!("📨 Correlation {} -> Sequence {}", 
                        sent_info.correlation_id, 
                        sent_info.sequence_id
                    );
                    correlation_map.insert(sent_info.correlation_id, sent_info.sequence_id);
                }
            }
            
            // Listen for responses
            Ok(ResponsePacket::InstructionResponse(instr_resp)) = response_rx.recv() => {
                let seq_id = instr_resp.get_sequence_id();
                
                // Find correlation ID for this sequence ID
                if let Some((corr_id, _)) = correlation_map.iter()
                    .find(|(_, &s)| s == seq_id) {
                    println!("✅ Instruction completed: correlation={}, sequence={}", 
                        corr_id, seq_id
                    );
                    completed += 1;
                    
                    if completed == correlation_ids.len() {
                        println!("🎉 All {} instructions completed!", completed);
                        break;
                    }
                }
            }
        }
    }
    
    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_example_instruction() -> Instruction {
    Instruction::FrcLinearMotion(FrcLinearMotion::new(
        0, // sequence_id will be assigned by driver
        Configuration {
            u_tool_number: 1,
            u_frame_number: 1,
            front: 1,
            up: 1,
            left: 0,
            flip: 0,
            turn4: 0,
            turn5: 0,
            turn6: 0,
        },
        Position {
            x: 100.0,
            y: 200.0,
            z: 300.0,
            w: 0.0,
            p: 0.0,
            r: 0.0,
        },
        SpeedType::MMSec,
        100.0,
        TermType::FINE,
        100,
    ))
}

// ============================================================================
// Main Example
// ============================================================================

#[tokio::main]
async fn main() -> Result<(), String> {
    // Connect to FANUC controller
    let config = FanucDriverConfig {
        addr: "192.168.1.100".to_string(),
        port: 18735,
    };
    
    let driver = FanucDriver::connect(config).await
        .map_err(|e| format!("Failed to connect: {:?}", e))?;
    
    println!("🤖 Connected to FANUC controller\n");
    
    // Run examples
    println!("=== Pattern 1: Simple ===");
    pattern_1_simple(&driver).await?;
    
    println!("\n=== Pattern 2: Flexible ===");
    pattern_2_flexible(&driver).await?;
    
    println!("\n=== Pattern 3: Advanced ===");
    pattern_3_advanced(&driver).await?;
    
    println!("\n=== Pattern 4: Batch ===");
    pattern_4_batch(&driver).await?;
    
    println!("\n✅ All examples completed successfully!");
    
    Ok(())
}

