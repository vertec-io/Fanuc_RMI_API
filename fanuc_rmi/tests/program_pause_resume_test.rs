/// Integration tests for program_pause() and program_resume() functionality.
///
/// These tests require the simulator to be running in realtime mode:
///   cargo run -p sim -- --realtime
///
/// The tests verify:
/// 1. Basic program_pause/program_resume cycle
/// 2. In-flight instruction tracking and replay
/// 3. Queue preservation across pause/resume
/// 4. Multiple pause/resume cycles

use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig},
    packets::{SendPacket, PacketPriority, Instruction},
    instructions::FrcLinearMotion,
    Configuration, Position, SpeedType, TermType,
};
use std::time::Duration;
use tokio::time::timeout;

/// Default simulator address
const SIMULATOR_ADDR: &str = "127.0.0.1";
const SIMULATOR_PORT: u32 = 16001;

/// Helper to create a linear motion instruction
fn create_linear_motion(x: f64, y: f64, z: f64, speed: f64) -> Instruction {
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
            x,
            y,
            z,
            w: 0.0,
            p: 0.0,
            r: 0.0,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        },
        SpeedType::MMSec,
        speed,
        TermType::FINE,
        100,
    ))
}

/// Connect to the simulator
async fn connect_to_simulator() -> Result<FanucDriver, String> {
    let config = FanucDriverConfig {
        addr: SIMULATOR_ADDR.to_string(),
        port: SIMULATOR_PORT,
        ..Default::default()
    };

    FanucDriver::connect(config)
        .await
        .map_err(|e| format!("Failed to connect to simulator: {:?}", e))
}

/// Test basic program_pause followed by program_resume
#[tokio::test]
#[ignore] // Requires simulator to be running
async fn test_basic_program_pause_resume() {
    let driver = match connect_to_simulator().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Skipping test - simulator not available: {}", e);
            return;
        }
    };

    // Initialize the robot
    let init_result = driver.startup_sequence().await;
    assert!(init_result.is_ok(), "startup_sequence failed: {:?}", init_result);

    // Send a motion instruction
    let instruction = create_linear_motion(500.0, 0.0, 500.0, 50.0);
    let send_result = driver.send_packet(
        SendPacket::Instruction(instruction),
        PacketPriority::Standard,
    );
    assert!(send_result.is_ok(), "send_packet failed: {:?}", send_result);

    // Wait a bit for the instruction to be sent and start executing
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Program pause
    let pause_result = driver.program_pause().await;
    assert!(pause_result.is_ok(), "program_pause failed: {:?}", pause_result);

    // Wait a moment (simulating user jogging robot)
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Program resume
    let resume_result = driver.program_resume().await;
    assert!(resume_result.is_ok(), "program_resume failed: {:?}", resume_result);

    // Give time for any replayed instructions to complete
    tokio::time::sleep(Duration::from_millis(500)).await;

    println!("test_basic_program_pause_resume passed");
}

/// Test that in-flight instructions are correctly tracked and replayed
#[tokio::test]
#[ignore] // Requires simulator to be running
async fn test_in_flight_instruction_replay() {
    let driver = match connect_to_simulator().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Skipping test - simulator not available: {}", e);
            return;
        }
    };

    // Initialize the robot
    driver.startup_sequence().await.expect("startup_sequence failed");

    // Subscribe to completion responses
    let mut response_rx = driver.response_tx.subscribe();

    // Send multiple instructions quickly (they'll be in-flight)
    for i in 0..5 {
        let x = 400.0 + (i as f64 * 20.0);
        let instruction = create_linear_motion(x, 0.0, 500.0, 100.0);
        driver.send_packet(
            SendPacket::Instruction(instruction),
            PacketPriority::Standard,
        ).expect("send_packet failed");
    }

    // Wait a bit for some instructions to be sent (but not all completed in realtime mode)
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Program pause - this should capture any in-flight instructions
    driver.program_pause().await.expect("program_pause failed");

    // Program resume - should replay in-flight instructions
    driver.program_resume().await.expect("program_resume failed");

    // Wait for all instructions to complete (with timeout)
    let mut completed_count = 0;
    let result = timeout(Duration::from_secs(30), async {
        while completed_count < 5 {
            match response_rx.recv().await {
                Ok(fanuc_rmi::packets::ResponsePacket::InstructionResponse(_)) => {
                    completed_count += 1;
                    println!("Completed instruction {} of 5", completed_count);
                }
                Ok(_) => {} // Other response types
                Err(e) => {
                    eprintln!("Response channel error: {:?}", e);
                    break;
                }
            }
        }
    }).await;

    assert!(result.is_ok(), "Timeout waiting for instruction completions");
    assert_eq!(completed_count, 5, "Expected 5 completions, got {}", completed_count);

    println!("test_in_flight_instruction_replay passed");
}

/// Test that internal queue is preserved across pause/resume
#[tokio::test]
#[ignore] // Requires simulator to be running
async fn test_queue_preservation() {
    let driver = match connect_to_simulator().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Skipping test - simulator not available: {}", e);
            return;
        }
    };

    // Initialize the robot
    driver.startup_sequence().await.expect("startup_sequence failed");

    // Subscribe to responses
    let mut response_rx = driver.response_tx.subscribe();

    // Send a slow-moving instruction that will take a while
    let slow_instruction = create_linear_motion(300.0, 0.0, 600.0, 10.0); // Very slow
    driver.send_packet(
        SendPacket::Instruction(slow_instruction),
        PacketPriority::Standard,
    ).expect("send first instruction");

    // Queue up more instructions behind it
    for i in 0..3 {
        let x = 350.0 + (i as f64 * 50.0);
        let instruction = create_linear_motion(x, 0.0, 500.0, 100.0);
        driver.send_packet(
            SendPacket::Instruction(instruction),
            PacketPriority::Standard,
        ).expect("send queued instruction");
    }

    // Wait briefly then pause
    tokio::time::sleep(Duration::from_millis(100)).await;
    driver.program_pause().await.expect("program_pause failed");

    // Wait (simulating operator intervention)
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Resume
    driver.program_resume().await.expect("program_resume failed");

    // All 4 instructions should eventually complete
    let mut completed = 0;
    let result = timeout(Duration::from_secs(60), async {
        while completed < 4 {
            match response_rx.recv().await {
                Ok(fanuc_rmi::packets::ResponsePacket::InstructionResponse(_)) => {
                    completed += 1;
                    println!("Completed instruction {} of 4", completed);
                }
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Response error: {:?}", e);
                    break;
                }
            }
        }
    }).await;

    assert!(result.is_ok(), "Timeout waiting for completions");
    assert_eq!(completed, 4, "Expected 4 completions after queue preserved");

    println!("test_queue_preservation passed");
}

/// Test multiple pause/resume cycles
#[tokio::test]
#[ignore] // Requires simulator to be running
async fn test_multiple_pause_resume_cycles() {
    let driver = match connect_to_simulator().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Skipping test - simulator not available: {}", e);
            return;
        }
    };

    // Initialize
    driver.startup_sequence().await.expect("startup_sequence failed");

    // Perform multiple pause/resume cycles
    for cycle in 0..3 {
        println!("Starting cycle {}", cycle);

        // Send an instruction
        let x = 400.0 + (cycle as f64 * 50.0);
        let instruction = create_linear_motion(x, 0.0, 500.0, 100.0);
        driver.send_packet(
            SendPacket::Instruction(instruction),
            PacketPriority::Standard,
        ).expect("send instruction");

        // Brief wait
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Pause
        driver.program_pause().await.expect("program_pause failed");

        // Wait
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Resume
        driver.program_resume().await.expect("program_resume failed");

        // Wait for motion to settle
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    println!("test_multiple_pause_resume_cycles passed");
}

/// Test that pause without prior initialization fails gracefully
#[tokio::test]
#[ignore] // Requires simulator to be running
async fn test_pause_without_running() {
    let driver = match connect_to_simulator().await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Skipping test - simulator not available: {}", e);
            return;
        }
    };

    // Try to pause without initializing - abort should still work
    // but the state might not be "running" for RMI
    let pause_result = driver.program_pause().await;

    // The pause might succeed (abort always works) or fail depending on RMI state
    // We just verify it doesn't panic
    println!("pause result (expected to vary): {:?}", pause_result);

    println!("test_pause_without_running passed");
}
