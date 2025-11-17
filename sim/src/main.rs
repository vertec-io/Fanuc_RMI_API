use serde_json::json;
use std::error::Error;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::time::Duration;

mod kinematics;
mod robot_config;

use kinematics::CRXKinematics;

/// Simulator execution mode
#[derive(Clone, Debug, PartialEq)]
enum SimulatorMode {
    /// Immediate mode: Updates positions instantly when receiving motion commands
    /// Return packets are sent immediately after receiving the instruction
    Immediate,

    /// Realtime mode: Simulates actual robot controller behavior
    /// - Calculates motion duration based on distance and speed
    /// - Sends return packets only after instruction execution completes
    /// - Respects buffer limits (8 concurrent instructions, 200 instruction ring buffer)
    Realtime,
}

// Simulated robot state
#[derive(Clone, Debug)]
struct RobotState {
    joint_angles: [f32; 6],
    cartesian_position: [f32; 3],
    cartesian_orientation: [f32; 3],
    kinematics: CRXKinematics,
    mode: SimulatorMode,
    last_sequence_id: Arc<Mutex<u32>>, // Track the last completed sequence ID
}

impl Default for RobotState {
    fn default() -> Self {
        Self::new(SimulatorMode::Immediate)
    }
}

impl RobotState {
    fn new(mode: SimulatorMode) -> Self {
        let kinematics = CRXKinematics::default();
        // Start with a better initial configuration:
        // J2 = 45° (shoulder up), J3 = -90° (elbow bent)
        // This places the end effector at a comfortable mid-workspace position
        let j2_deg: f64 = 45.0;
        let j3_deg: f64 = -90.0;
        let joints_f64 = [
            0.0,                      // J1 = 0° (facing forward)
            j2_deg.to_radians(),      // J2 = 45° (shoulder up)
            j3_deg.to_radians(),      // J3 = -90° (elbow bent)
            0.0,                      // J4 = 0°
            0.0,                      // J5 = 0°
            0.0,                      // J6 = 0°
        ];
        let (pos, ori) = kinematics.forward_kinematics(&joints_f64);

        // Initial configuration: J2=45°, J3=-90° for mid-workspace position

        Self {
            joint_angles: [
                joints_f64[0] as f32,
                joints_f64[1] as f32,
                joints_f64[2] as f32,
                joints_f64[3] as f32,
                joints_f64[4] as f32,
                joints_f64[5] as f32,
            ],
            cartesian_position: [pos[0] as f32, pos[1] as f32, pos[2] as f32],
            cartesian_orientation: [ori[0] as f32, ori[1] as f32, ori[2] as f32],
            kinematics,
            mode,
            last_sequence_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Calculate motion duration in seconds based on distance and speed
    fn calculate_motion_duration(distance_mm: f64, speed_mm_per_sec: f64) -> f64 {
        if speed_mm_per_sec <= 0.0 {
            return 0.1; // Minimum duration
        }
        (distance_mm / speed_mm_per_sec).max(0.01) // At least 10ms
    }
}

// #[derive(Serialize, Deserialize, Debug)]
// struct ConnectResponse {
//     Communication: String,
//     PortNumber: Option<u16>,
//     MajorVersion: Option<u16>,
//     MinorVersion: Option<u16>,
// }

async fn handle_client(
    mut socket: TcpStream,
    new_port: Arc<Mutex<u16>>,
) -> Result<u16, Box<dyn Error + Send + Sync>> {
    let mut buffer = vec![0; 2048];
    let n = match socket.read(&mut buffer).await {
        Ok(n) => n,
        Err(e) => {
            eprintln!("Failed to read from socket: {}", e);
            return Err(Box::new(e));
        }
    };

    if n == 0 {
        return Ok(0);
    }

    let request = String::from_utf8_lossy(&buffer[..n]);
    let request_json: serde_json::Value = serde_json::from_str(&request)?;

    let response_json = match request_json["Communication"].as_str() {
        Some("FRC_Connect") => {
            let port = {
                let mut port_lock = new_port.lock().await;
                *port_lock += 1;
                *port_lock
            };
            println!("✓ Client connected, assigned port {}", port);

            json!({
                "Communication": "FRC_Connect",
                "ErrorID": 1,
                "PortNumber": port,
                "MajorVersion": 1,
                "MinorVersion": 0,
            })
        }
        _ => json!({
            "Error": "Unknown command"
        }),
    };

    let response = serde_json::to_string(&response_json)? + "\r\n";
    socket.write_all(response.as_bytes()).await?;

    if let Some(port) = response_json["PortNumber"].as_u64() {
        return Ok(port as u16);
    }

    Err("Failed to parse port number".into())
}

async fn handle_secondary_client(
    mut socket: TcpStream,
    robot_state: Arc<Mutex<RobotState>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut seq: u32 = 0;
    let mut buffer = vec![0; 1024];
    let mut temp_buffer = Vec::new();

    loop {
        let n = match socket.read(&mut buffer).await {
            Ok(n) => n,
            Err(e) => {
                eprintln!("Failed to read from socket: {}", e);
                return Err(Box::new(e));
            }
        };

        if n == 0 {
            break;
        }

        // Append new data to temp_buffer
        temp_buffer.extend_from_slice(&buffer[..n]);

        while let Some(pos) = temp_buffer.iter().position(|&x| x == b'\n') {
            // Split the buffer into the current message and the rest
            let request: Vec<u8> = temp_buffer.drain(..=pos).collect();
            // Remove the newline character
            let request = &request[..request.len() - 1];

            let request_str = String::from_utf8_lossy(request);

            let request_json: serde_json::Value = match serde_json::from_str(&request_str) {
                Ok(json) => json,
                Err(e) => {
                    eprintln!("Failed to parse JSON: {}", e);
                    continue;
                }
            };

            let mut response_json = match request_json["Command"].as_str() {
                Some("FRC_Initialize") => {
                    println!("📋 FRC_Initialize");
                    json!({
                        "Command": "FRC_Initialize",
                        "ErrorID": 0,
                        "GroupMask": 1
                    })
                }
                Some("FRC_GetStatus") => {
                    let state = robot_state.lock().await;
                    let next_seq = *state.last_sequence_id.lock().await + 1;
                    json!({
                        "Command": "FRC_GetStatus",
                        "ErrorID": 0,
                        "ServoReady": 1,
                        "TPMode": 1,
                        "RMIMotionStatus": 0,
                        "ProgramStatus": 0,
                        "SingleStepMode": 0,
                        "NumberUTool": 5,
                        "NextSequenceID": next_seq,
                        "NumberUFrame": 0
                    })
                },
                Some("FRC_ReadJointAngles") => {
                    let state = robot_state.lock().await;
                    println!("📐 FRC_ReadJointAngles: J1={:.2}° J2={:.2}° J3={:.2}° J4={:.2}° J5={:.2}° J6={:.2}°",
                        state.joint_angles[0].to_degrees(),
                        state.joint_angles[1].to_degrees(),
                        state.joint_angles[2].to_degrees(),
                        state.joint_angles[3].to_degrees(),
                        state.joint_angles[4].to_degrees(),
                        state.joint_angles[5].to_degrees());
                    json!({
                        "Command": "FRC_ReadJointAngles",
                        "ErrorID": 0,
                        "TimeTag": 0,
                        "JointAngles": {
                            "J1": state.joint_angles[0],
                            "J2": state.joint_angles[1],
                            "J3": state.joint_angles[2],
                            "J4": state.joint_angles[3],
                            "J5": state.joint_angles[4],
                            "J6": state.joint_angles[5],
                        },
                        "Group": 1
                    })
                },
                Some("FRC_ReadCartesianPosition") => {
                    let state = robot_state.lock().await;
                    println!("📍 FRC_ReadCartesianPosition: X={:.1} Y={:.1} Z={:.1} W={:.1}° P={:.1}° R={:.1}°",
                        state.cartesian_position[0],
                        state.cartesian_position[1],
                        state.cartesian_position[2],
                        state.cartesian_orientation[0].to_degrees(),
                        state.cartesian_orientation[1].to_degrees(),
                        state.cartesian_orientation[2].to_degrees());
                    json!({
                        "Command": "FRC_ReadCartesianPosition",
                        "ErrorID": 0,
                        "TimeTag": 0,
                        "Configuration": {
                            "F": 0,
                            "U": 0,
                            "T": 0,
                            "B1": 0,
                            "B2": 0,
                            "B3": 0,
                        },
                        "Position": {
                            "X": state.cartesian_position[0],
                            "Y": state.cartesian_position[1],
                            "Z": state.cartesian_position[2],
                            "W": state.cartesian_orientation[0],
                            "P": state.cartesian_orientation[1],
                            "R": state.cartesian_orientation[2],
                        },
                        "Group": 1
                    })
                },
                Some("FRC_LinearMotion") => json!({
                    "Status": "Motion started"
                }),
                Some("FRC_Abort") => {
                    println!("🛑 FRC_Abort");
                    json!({
                        "Command": "FRC_Abort",
                        "ErrorID": 0,
                    })
                }
                Some("FRC_Reset") => {
                    println!("🔄 FRC_Reset");
                    json!({
                        "Command": "FRC_Reset",
                        "ErrorID": 0,
                    })
                }
                Some("FRC_SetOverRide") => json!({
                    "Command": "FRC_SetOverRide",
                    "ErrorID": 0,
                }),
                _ => json!({}),
            };

            response_json = match request_json["Communication"].as_str() {
                Some("FRC_Disconnect") => {
                    println!("👋 FRC_Disconnect\n");
                    json!({
                        "Communication": "FRC_Disconnect",
                        "ErrorID": 0,
                    })
                }
                _ => response_json,
            };

            response_json = match request_json["Instruction"].as_str() {
                Some("FRC_LinearMotion") => json!({
                    "Instruction": "FRC_LinearMotion",
                    "ErrorID": 0,
                    "SequenceID": seq,
                }),
                Some("FRC_LinearRelative") => {
                    // Parse the Position from the instruction
                    if let Some(position) = request_json.get("Position") {
                        let dx = position["X"].as_f64().unwrap_or(0.0);
                        let dy = position["Y"].as_f64().unwrap_or(0.0);
                        let dz = position["Z"].as_f64().unwrap_or(0.0);

                        let speed = request_json.get("Speed").and_then(|v| v.as_f64()).unwrap_or(10.0);
                        let term_type = request_json.get("TermType").and_then(|v| v.as_str()).unwrap_or("FINE");
                        let term_value = request_json.get("TermValue").and_then(|v| v.as_u64()).unwrap_or(0);

                        // Update robot state with relative movement
                        let mut state = robot_state.lock().await;

                        // Calculate distance for timing
                        let distance = (dx * dx + dy * dy + dz * dz).sqrt();

                        // Update Cartesian position (happens immediately in both modes for visualization)
                        state.cartesian_position[0] += dx as f32;
                        state.cartesian_position[1] += dy as f32;
                        state.cartesian_position[2] += dz as f32;

                        println!("🎯 FRC_LinearRelative: ΔX={:+.1} ΔY={:+.1} ΔZ={:+.1} | Speed={:.1}mm/s | Term={} CNT={} | Pos=[{:.1}, {:.1}, {:.1}]",
                            dx, dy, dz, speed, term_type, term_value,
                            state.cartesian_position[0],
                            state.cartesian_position[1],
                            state.cartesian_position[2]);

                        // Calculate joint angles using inverse kinematics
                        let target_pos = [
                            state.cartesian_position[0] as f64,
                            state.cartesian_position[1] as f64,
                            state.cartesian_position[2] as f64,
                        ];

                        let current_joints = [
                            state.joint_angles[0] as f64,
                            state.joint_angles[1] as f64,
                            state.joint_angles[2] as f64,
                            state.joint_angles[3] as f64,
                            state.joint_angles[4] as f64,
                            state.joint_angles[5] as f64,
                        ];

                        let target_ori = Some([
                            state.cartesian_orientation[0] as f64,
                            state.cartesian_orientation[1] as f64,
                            state.cartesian_orientation[2] as f64,
                        ]);

                        if let Some(new_joints) = state.kinematics.inverse_kinematics(
                            &target_pos,
                            target_ori.as_ref(),
                            &current_joints,
                        ) {
                            // Update joint angles
                            state.joint_angles[0] = new_joints[0] as f32;
                            state.joint_angles[1] = new_joints[1] as f32;
                            state.joint_angles[2] = new_joints[2] as f32;
                            state.joint_angles[3] = new_joints[3] as f32;
                            state.joint_angles[4] = new_joints[4] as f32;
                            state.joint_angles[5] = new_joints[5] as f32;
                        } else {
                            eprintln!("⚠️  WARNING: IK solution not found for target position");
                        }

                        // Calculate delay based on mode
                        let delay_ms = if state.mode == SimulatorMode::Realtime {
                            let duration = RobotState::calculate_motion_duration(distance, speed);
                            println!("   ⏱️  Motion will take {:.2}s to complete", duration);
                            (duration * 1000.0) as u64 // Convert to milliseconds
                        } else {
                            0 // Immediate mode: no delay
                        };

                        let last_seq_id = Arc::clone(&state.last_sequence_id);
                        drop(state); // Release lock before sleeping

                        // Sleep to simulate motion execution time
                        if delay_ms > 0 {
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        }

                        // Update last completed sequence ID
                        *last_seq_id.lock().await = seq;
                    }

                    // Send response after motion completes (or immediately in immediate mode)
                    json!({
                        "Instruction": "FRC_LinearRelative",
                        "ErrorID": 0,
                        "SequenceID": seq
                    })
                },
                _ => response_json,
            };
            let response = serde_json::to_string(&response_json)? + "\r\n";
            socket.write_all(response.as_bytes()).await?;
            seq += 1;
        }
    }

    Ok(())
}

async fn start_secondary_server_with_listener(_port: u16, listener: TcpListener, mode: Arc<SimulatorMode>) -> Result<(), Box<dyn Error + Send + Sync>> {
    // Create shared robot state for this connection
    let robot_state = Arc::new(Mutex::new(RobotState::new((*mode).clone())));

    loop {
        let (socket, _) = match listener.accept().await {
            Ok((socket, addr)) => (socket, addr),
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue;
            }
        };

        let robot_state_clone = Arc::clone(&robot_state);
        tokio::spawn(async move {
            if let Err(e) = handle_secondary_client(socket, robot_state_clone).await {
                eprintln!("Error handling secondary client: {:?}", e);
            }
        });
    }
}

async fn start_server(port: u16, mode: SimulatorMode) -> Result<(), Box<dyn Error + Send + Sync>> {
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await?;
    println!("🤖 FANUC Simulator started on {}", addr);
    println!("   Waiting for connections...\n");

    let new_port = Arc::new(Mutex::new(port + 1));
    let sim_mode = Arc::new(mode);

    loop {
        let (socket, _) = match listener.accept().await {
            Ok((socket, addr)) => (socket, addr),
            Err(e) => {
                eprintln!("Failed to accept connection: {}", e);
                continue;
            }
        };

        let new_port = Arc::clone(&new_port);
        let sim_mode_clone = Arc::clone(&sim_mode);

        match handle_client(socket, new_port).await {
            Ok(port) if port != 0 => {
                // Start the secondary server and wait for it to be ready before continuing
                // This ensures the server is listening before the client tries to connect
                let secondary_addr = format!("0.0.0.0:{}", port);
                match TcpListener::bind(&secondary_addr).await {
                    Ok(secondary_listener) => {
                        tokio::spawn(async move {
                            start_secondary_server_with_listener(port, secondary_listener, sim_mode_clone).await
                        });
                    }
                    Err(e) => eprintln!("Failed to bind secondary server on port {}: {:?}", port, e),
                }
            }
            Ok(_) => {}
            Err(e) => eprintln!("Failed to handle client: {:?}", e),
        };
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    let mode = if args.len() > 1 && args[1] == "--realtime" {
        SimulatorMode::Realtime
    } else {
        SimulatorMode::Immediate
    };

    match mode {
        SimulatorMode::Immediate => {
            println!("🤖 Starting FANUC Simulator in IMMEDIATE mode");
            println!("   (Positions update instantly, return packets sent immediately)\n");
        }
        SimulatorMode::Realtime => {
            println!("🤖 Starting FANUC Simulator in REALTIME mode");
            println!("   (Simulates actual robot timing, return packets sent after execution)\n");
        }
    }

    start_server(16001, mode).await?;
    Ok(())
}
