use serde_json::json;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc, RwLock};
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

/// Motion command that can be queued for execution
#[derive(Debug)]
struct MotionCommand {
    seq_id: u32,
    target_pos: [f64; 3],
    target_ori: [f64; 3],
    speed: f64,
    term_type: String,
    term_value: u64,
    is_relative: bool,
    instruction_type: String,
}

/// Response to send back after motion completes
#[derive(Debug)]
struct MotionResponse {
    seq_id: u32,
    instruction_type: String,
}

/// Motion executor control signals - allows immediate pause/abort
#[derive(Debug)]
struct MotionExecutorControl {
    /// When true, motion interpolation is paused (checked every 50ms during motion)
    paused: AtomicBool,
    /// When true, abort current motion and clear queue
    abort_requested: AtomicBool,
    /// Speed override percentage (0-100), affects motion duration
    speed_override: AtomicU8,
}

impl Default for MotionExecutorControl {
    fn default() -> Self {
        Self {
            paused: AtomicBool::new(false),
            abort_requested: AtomicBool::new(false),
            speed_override: AtomicU8::new(100),
        }
    }
}

impl MotionExecutorControl {
    fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    fn unpause(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    fn request_abort(&self) {
        self.abort_requested.store(true, Ordering::SeqCst);
    }

    fn clear_abort(&self) {
        self.abort_requested.store(false, Ordering::SeqCst);
    }

    fn is_abort_requested(&self) -> bool {
        self.abort_requested.load(Ordering::SeqCst)
    }

    fn set_speed_override(&self, percent: u8) {
        self.speed_override.store(percent.min(100), Ordering::SeqCst);
    }

    fn get_speed_override(&self) -> u8 {
        self.speed_override.load(Ordering::SeqCst)
    }
}

/// Frame/Tool coordinate data (X, Y, Z, W, P, R)
#[derive(Clone, Debug, Default)]
struct FrameToolData {
    x: f64,
    y: f64,
    z: f64,
    w: f64,
    p: f64,
    r: f64,
}

// Simulated robot state - now using RwLock for concurrent read access
#[derive(Clone, Debug)]
struct RobotState {
    joint_angles: [f32; 6],
    cartesian_position: [f32; 3],
    cartesian_orientation: [f32; 3],
    kinematics: CRXKinematics,
    mode: SimulatorMode,
    last_sequence_id: u32, // Track the last completed sequence ID
    // Frame/Tool state
    active_uframe: u8,
    active_utool: u8,
    uframes: [FrameToolData; 10],
    utools: [FrameToolData; 10],
    // I/O state
    din: [bool; 256],  // Digital inputs (simulated)
    dout: [bool; 256], // Digital outputs
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
            last_sequence_id: 0,
            // Initialize Frame/Tool state
            active_uframe: 0,
            active_utool: 0,
            uframes: Default::default(),
            utools: Default::default(),
            // Initialize I/O state
            din: [false; 256],
            dout: [false; 256],
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

/// Shared state wrapper with RwLock for concurrent read access
struct SharedRobotState {
    state: RwLock<RobotState>,
    response_tx: mpsc::Sender<MotionResponse>,
}

async fn handle_secondary_client(
    mut socket: TcpStream,
    robot_state: Arc<Mutex<RobotState>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut seq: u32 = 0; // Default, will be overwritten by each request's SequenceID
    let mut buffer = vec![0; 1024];
    let mut temp_buffer = Vec::new();

    // Create a channel for motion responses (completed motions -> socket writer)
    let (response_tx, mut response_rx) = mpsc::channel::<MotionResponse>(100);

    // Create a channel for motion commands (command receiver -> motion executor)
    let (motion_tx, mut motion_rx) = mpsc::channel::<MotionCommand>(200);

    // Create shared motion executor control for pause/abort/speed override
    let executor_control = Arc::new(MotionExecutorControl::default());

    // Spawn a single motion executor task that processes motions SEQUENTIALLY
    let robot_state_for_executor = Arc::clone(&robot_state);
    let response_tx_for_executor = response_tx.clone();
    let control_for_executor = Arc::clone(&executor_control);
    tokio::spawn(async move {
        'motion_loop: while let Some(cmd) = motion_rx.recv().await {
            // Check for abort BEFORE starting motion
            if control_for_executor.is_abort_requested() {
                eprintln!("🛑 Abort detected before motion {}, clearing queue", cmd.seq_id);
                // Drain remaining commands from the queue
                while motion_rx.try_recv().is_ok() {}
                control_for_executor.clear_abort();
                continue 'motion_loop;
            }

            // Get current position for interpolation
            let (start_x, start_y, start_z, start_w, start_p, start_r, current_joints, mode) = {
                let state = robot_state_for_executor.lock().await;
                (
                    state.cartesian_position[0] as f64,
                    state.cartesian_position[1] as f64,
                    state.cartesian_position[2] as f64,
                    state.cartesian_orientation[0] as f64,
                    state.cartesian_orientation[1] as f64,
                    state.cartesian_orientation[2] as f64,
                    [
                        state.joint_angles[0] as f64,
                        state.joint_angles[1] as f64,
                        state.joint_angles[2] as f64,
                        state.joint_angles[3] as f64,
                        state.joint_angles[4] as f64,
                        state.joint_angles[5] as f64,
                    ],
                    state.mode.clone(),
                )
            };

            // For relative motion, target_pos contains the delta; for absolute, it's the target
            let (target_x, target_y, target_z, target_w, target_p, target_r) = if cmd.is_relative {
                (
                    start_x + cmd.target_pos[0],
                    start_y + cmd.target_pos[1],
                    start_z + cmd.target_pos[2],
                    start_w,  // Keep current orientation for relative moves
                    start_p,
                    start_r,
                )
            } else {
                (
                    cmd.target_pos[0],
                    cmd.target_pos[1],
                    cmd.target_pos[2],
                    cmd.target_ori[0],
                    cmd.target_ori[1],
                    cmd.target_ori[2],
                )
            };

            // Calculate distance and duration
            let dx = target_x - start_x;
            let dy = target_y - start_y;
            let dz = target_z - start_z;
            let distance = (dx * dx + dy * dy + dz * dz).sqrt();

            // Apply speed override to motion speed
            let speed_override = control_for_executor.get_speed_override() as f64 / 100.0;
            let effective_speed = cmd.speed * speed_override.max(0.01); // Minimum 1% to avoid division by zero

            eprintln!("🏃 Executing motion {} ({}) | dist={:.1}mm | speed={:.1}mm/s ({}% override)",
                cmd.seq_id, cmd.instruction_type, distance, effective_speed, (speed_override * 100.0) as u8);

            let delay_ms = if mode == SimulatorMode::Realtime {
                let duration = RobotState::calculate_motion_duration(distance, effective_speed);
                (duration * 1000.0) as u64
            } else {
                0
            };

            // Execute motion with incremental position updates
            let mut motion_aborted = false;
            if delay_ms > 0 {
                let update_interval_ms = 50u64;
                let total_steps = (delay_ms / update_interval_ms).max(1);

                for step in 1..=total_steps {
                    // Check for abort DURING motion interpolation
                    if control_for_executor.is_abort_requested() {
                        eprintln!("🛑 Abort detected during motion {} at step {}/{}", cmd.seq_id, step, total_steps);
                        // Drain remaining commands
                        while motion_rx.try_recv().is_ok() {}
                        control_for_executor.clear_abort();
                        motion_aborted = true;
                        break;
                    }

                    // Check for pause - wait while paused
                    while control_for_executor.is_paused() {
                        // Check for abort while paused
                        if control_for_executor.is_abort_requested() {
                            eprintln!("🛑 Abort detected while paused during motion {}", cmd.seq_id);
                            while motion_rx.try_recv().is_ok() {}
                            control_for_executor.clear_abort();
                            motion_aborted = true;
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }

                    if motion_aborted {
                        break;
                    }

                    let t = step as f64 / total_steps as f64;

                    // Linear interpolation
                    let current_x = start_x + (target_x - start_x) * t;
                    let current_y = start_y + (target_y - start_y) * t;
                    let current_z = start_z + (target_z - start_z) * t;
                    let current_w = start_w + (target_w - start_w) * t;
                    let current_p = start_p + (target_p - start_p) * t;
                    let current_r = start_r + (target_r - start_r) * t;

                    // Update robot state
                    {
                        let mut state = robot_state_for_executor.lock().await;
                        state.cartesian_position[0] = current_x as f32;
                        state.cartesian_position[1] = current_y as f32;
                        state.cartesian_position[2] = current_z as f32;
                        state.cartesian_orientation[0] = current_w as f32;
                        state.cartesian_orientation[1] = current_p as f32;
                        state.cartesian_orientation[2] = current_r as f32;

                        // Calculate joint angles using inverse kinematics
                        let target_pos = [current_x, current_y, current_z];
                        let target_ori = Some([current_w, current_p, current_r]);

                        if let Some(new_joints) = state.kinematics.inverse_kinematics(
                            &target_pos,
                            target_ori.as_ref(),
                            &current_joints,
                        ) {
                            state.joint_angles[0] = new_joints[0] as f32;
                            state.joint_angles[1] = new_joints[1] as f32;
                            state.joint_angles[2] = new_joints[2] as f32;
                            state.joint_angles[3] = new_joints[3] as f32;
                            state.joint_angles[4] = new_joints[4] as f32;
                            state.joint_angles[5] = new_joints[5] as f32;
                        }
                    }

                    tokio::time::sleep(Duration::from_millis(update_interval_ms)).await;
                }
            } else {
                // Instant mode - just set final position
                let mut state = robot_state_for_executor.lock().await;
                state.cartesian_position[0] = target_x as f32;
                state.cartesian_position[1] = target_y as f32;
                state.cartesian_position[2] = target_z as f32;
                state.cartesian_orientation[0] = target_w as f32;
                state.cartesian_orientation[1] = target_p as f32;
                state.cartesian_orientation[2] = target_r as f32;

                let target_pos = [target_x, target_y, target_z];
                let target_ori = Some([target_w, target_p, target_r]);

                if let Some(new_joints) = state.kinematics.inverse_kinematics(
                    &target_pos,
                    target_ori.as_ref(),
                    &current_joints,
                ) {
                    state.joint_angles[0] = new_joints[0] as f32;
                    state.joint_angles[1] = new_joints[1] as f32;
                    state.joint_angles[2] = new_joints[2] as f32;
                    state.joint_angles[3] = new_joints[3] as f32;
                    state.joint_angles[4] = new_joints[4] as f32;
                    state.joint_angles[5] = new_joints[5] as f32;
                }
            }

            // Skip response if motion was aborted
            if motion_aborted {
                continue 'motion_loop;
            }

            // Update last sequence ID
            {
                let mut state = robot_state_for_executor.lock().await;
                state.last_sequence_id = cmd.seq_id;
            }

            // Send response back - motion is complete
            eprintln!("✅ Motion {} complete, sending response", cmd.seq_id);
            let _ = response_tx_for_executor.send(MotionResponse {
                seq_id: cmd.seq_id,
                instruction_type: cmd.instruction_type,
            }).await;
        }
        eprintln!("Motion executor task ended");
    });

    // motion_tx is used to queue commands to the executor
    let motion_tx = Arc::new(motion_tx);
    // response_tx was moved to the executor task, response_rx is used below
    // executor_control is used to signal pause/abort from command handlers

    loop {
        tokio::select! {
            // Check for incoming data
            read_result = socket.read(&mut buffer) => {
                let n = match read_result {
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
                            let next_seq = state.last_sequence_id + 1;
                            let override_val = executor_control.get_speed_override();
                            let paused = if executor_control.is_paused() { 1 } else { 0 };
                            json!({
                                "Command": "FRC_GetStatus",
                                "ErrorID": 0,
                                "ServoReady": 1,
                                "TPMode": 1,
                                "RMIMotionStatus": paused, // 0=running, 1=paused
                                "ProgramStatus": 0,
                                "SingleStepMode": 0,
                                "NumberUTool": 5,
                                "NextSequenceID": next_seq,
                                "NumberUFrame": 0,
                                "Override": override_val
                            })
                        },
                        Some("FRC_ReadJointAngles") => {
                            let state = robot_state.lock().await;
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
                            json!({
                                "Command": "FRC_ReadCartesianPosition",
                                "ErrorID": 0,
                                "TimeTag": 0,
                                "Configuration": {
                                    "UToolNumber": 1,
                                    "UFrameNumber": 1,
                                    "Front": 1,
                                    "Up": 1,
                                    "Left": 1,
                                    "Flip": 0,
                                    "Turn4": 0,
                                    "Turn5": 0,
                                    "Turn6": 0,
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
                            println!("🛑 FRC_Abort - signaling motion executor to abort immediately");
                            executor_control.request_abort();
                            // Also unpause if paused, so abort takes effect
                            executor_control.unpause();
                            json!({
                                "Command": "FRC_Abort",
                                "ErrorID": 0,
                            })
                        }
                        Some("FRC_Pause") => {
                            println!("⏸️ FRC_Pause - pausing motion executor");
                            executor_control.pause();
                            json!({
                                "Command": "FRC_Pause",
                                "ErrorID": 0,
                            })
                        }
                        Some("FRC_Continue") => {
                            println!("▶️ FRC_Continue - resuming motion executor");
                            executor_control.unpause();
                            json!({
                                "Command": "FRC_Continue",
                                "ErrorID": 0,
                            })
                        }
                        Some("FRC_Reset") => {
                            println!("🔄 FRC_Reset");
                            // Reset also clears abort/pause state
                            executor_control.clear_abort();
                            executor_control.unpause();
                            json!({
                                "Command": "FRC_Reset",
                                "ErrorID": 0,
                            })
                        }
                        Some("FRC_SetOverRide") => {
                            // The struct uses "Value" field (serde rename)
                            let override_val = request_json["Value"].as_u64().unwrap_or(100) as u8;
                            executor_control.set_speed_override(override_val);
                            println!("⚡ FRC_SetOverRide: {}%", override_val);
                            json!({
                                "Command": "FRC_SetOverRide",
                                "ErrorID": 0,
                            })
                        }
                        Some("FRC_GetUFrameUTool") => {
                            let state = robot_state.lock().await;
                            json!({
                                "Command": "FRC_GetUFrameUTool",
                                "ErrorID": 0,
                                "UFrameNumber": state.active_uframe,
                                "UToolNumber": state.active_utool,
                                "Group": 1
                            })
                        }
                        Some("FRC_SetUFrameUTool") => {
                            let mut state = robot_state.lock().await;
                            let uframe = request_json["UFrameNumber"].as_u64().unwrap_or(0) as u8;
                            let utool = request_json["UToolNumber"].as_u64().unwrap_or(0) as u8;
                            state.active_uframe = uframe;
                            state.active_utool = utool;
                            println!("🔧 FRC_SetUFrameUTool: UFrame={}, UTool={}", uframe, utool);
                            json!({
                                "Command": "FRC_SetUFrameUTool",
                                "ErrorID": 0,
                                "Group": 1
                            })
                        }
                        Some("FRC_ReadUFrameData") => {
                            let state = robot_state.lock().await;
                            // Request uses "FrameNumber", response uses "UFrameNumber"
                            let frame_num = request_json["FrameNumber"].as_u64().unwrap_or(0) as usize;
                            let frame = state.uframes.get(frame_num).cloned().unwrap_or_default();
                            json!({
                                "Command": "FRC_ReadUFrameData",
                                "ErrorID": 0,
                                "UFrameNumber": frame_num,
                                "Group": 1,
                                "Frame": {
                                    "x": frame.x,
                                    "y": frame.y,
                                    "z": frame.z,
                                    "w": frame.w,
                                    "p": frame.p,
                                    "r": frame.r
                                }
                            })
                        }
                        Some("FRC_ReadUToolData") => {
                            let state = robot_state.lock().await;
                            // Request uses "FrameNumber", response uses "UToolNumber"
                            let tool_num = request_json["FrameNumber"].as_u64().unwrap_or(0) as usize;
                            let tool = state.utools.get(tool_num).cloned().unwrap_or_default();
                            json!({
                                "Command": "FRC_ReadUToolData",
                                "ErrorID": 0,
                                "UToolNumber": tool_num,
                                "Group": 1,
                                "Frame": {
                                    "x": tool.x,
                                    "y": tool.y,
                                    "z": tool.z,
                                    "w": tool.w,
                                    "p": tool.p,
                                    "r": tool.r
                                }
                            })
                        }
                        Some("FRC_WriteUFrameData") => {
                            let mut state = robot_state.lock().await;
                            let frame_num = request_json["FrameNumber"].as_u64().unwrap_or(0) as usize;
                            if frame_num < 10 {
                                if let Some(frame_obj) = request_json.get("Frame") {
                                    state.uframes[frame_num] = FrameToolData {
                                        x: frame_obj["x"].as_f64().unwrap_or(0.0),
                                        y: frame_obj["y"].as_f64().unwrap_or(0.0),
                                        z: frame_obj["z"].as_f64().unwrap_or(0.0),
                                        w: frame_obj["w"].as_f64().unwrap_or(0.0),
                                        p: frame_obj["p"].as_f64().unwrap_or(0.0),
                                        r: frame_obj["r"].as_f64().unwrap_or(0.0),
                                    };
                                    println!("📝 FRC_WriteUFrameData: UFrame {} updated", frame_num);
                                }
                            }
                            json!({
                                "Command": "FRC_WriteUFrameData",
                                "ErrorID": 0,
                                "Group": 1
                            })
                        }
                        Some("FRC_WriteUToolData") => {
                            let mut state = robot_state.lock().await;
                            let tool_num = request_json["ToolNumber"].as_u64().unwrap_or(0) as usize;
                            if tool_num < 10 {
                                if let Some(frame_obj) = request_json.get("Frame") {
                                    state.utools[tool_num] = FrameToolData {
                                        x: frame_obj["x"].as_f64().unwrap_or(0.0),
                                        y: frame_obj["y"].as_f64().unwrap_or(0.0),
                                        z: frame_obj["z"].as_f64().unwrap_or(0.0),
                                        w: frame_obj["w"].as_f64().unwrap_or(0.0),
                                        p: frame_obj["p"].as_f64().unwrap_or(0.0),
                                        r: frame_obj["r"].as_f64().unwrap_or(0.0),
                                    };
                                    println!("📝 FRC_WriteUToolData: UTool {} updated", tool_num);
                                }
                            }
                            json!({
                                "Command": "FRC_WriteUToolData",
                                "ErrorID": 0,
                                "Group": 1
                            })
                        }
                        Some("FRC_ReadDIN") => {
                            let state = robot_state.lock().await;
                            let port_num = request_json["PortNumber"].as_u64().unwrap_or(0) as usize;
                            let port_value = if port_num < 256 { state.din[port_num] } else { false };
                            println!("📥 FRC_ReadDIN: Port {} = {}", port_num, if port_value { "ON" } else { "OFF" });
                            json!({
                                "Command": "FRC_ReadDIN",
                                "ErrorID": 0,
                                "PortNumber": port_num,
                                "PortValue": if port_value { 1 } else { 0 }
                            })
                        }
                        Some("FRC_WriteDOUT") => {
                            let mut state = robot_state.lock().await;
                            let port_num = request_json["PortNumber"].as_u64().unwrap_or(0) as usize;
                            let port_value = request_json["PortValue"].as_u64().unwrap_or(0) != 0;
                            if port_num < 256 {
                                state.dout[port_num] = port_value;
                            }
                            println!("📤 FRC_WriteDOUT: Port {} = {}", port_num, if port_value { "ON" } else { "OFF" });
                            json!({
                                "Command": "FRC_WriteDOUT",
                                "ErrorID": 0
                            })
                        }
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

                    // Extract SequenceID from instruction requests (if present)
                    if let Some(seq_id) = request_json.get("SequenceID").and_then(|v| v.as_u64()) {
                        seq = seq_id as u32;
                    }

                    // Handle motion instructions asynchronously
                    response_json = match request_json["Instruction"].as_str() {
                        Some("FRC_LinearMotion") => {
                            // Parse the Position from the instruction (absolute position)
                            if let Some(position) = request_json.get("Position") {
                                let target_x = position["X"].as_f64().unwrap_or(0.0);
                                let target_y = position["Y"].as_f64().unwrap_or(0.0);
                                let target_z = position["Z"].as_f64().unwrap_or(0.0);
                                let target_w = position["W"].as_f64().unwrap_or(0.0);
                                let target_p = position["P"].as_f64().unwrap_or(0.0);
                                let target_r = position["R"].as_f64().unwrap_or(0.0);

                                let speed = request_json.get("Speed").and_then(|v| v.as_f64()).unwrap_or(100.0);
                                let term_type = request_json.get("TermType").and_then(|v| v.as_str()).unwrap_or("FINE").to_string();
                                let term_value = request_json.get("TermValue").and_then(|v| v.as_u64()).unwrap_or(0);

                                // Get mode for logging
                                let mode = {
                                    let state = robot_state.lock().await;
                                    state.mode.clone()
                                };

                                println!("🎯 FRC_LinearMotion: X={:.1} Y={:.1} Z={:.1} | Speed={:.1}mm/s | Term={} CNT={} | seq={}",
                                    target_x, target_y, target_z, speed, term_type, term_value, seq);

                                // Queue the motion command for sequential execution
                                let cmd = MotionCommand {
                                    seq_id: seq,
                                    target_pos: [target_x, target_y, target_z],
                                    target_ori: [target_w, target_p, target_r],
                                    speed,
                                    term_type,
                                    term_value,
                                    is_relative: false,
                                    instruction_type: "FRC_LinearMotion".to_string(),
                                };

                                if let Err(e) = motion_tx.send(cmd).await {
                                    eprintln!("❌ Failed to queue motion {}: {}", seq, e);
                                }

                                // In realtime mode, don't send immediate response - wait for motion completion
                                if mode == SimulatorMode::Realtime {
                                    continue; // Don't send response now, will be sent when motion completes
                                }
                            }

                            json!({
                                "Instruction": "FRC_LinearMotion",
                                "ErrorID": 0,
                                "SequenceID": seq,
                            })
                        }
                        Some("FRC_LinearRelative") => {
                            // Parse the Position from the instruction (relative offset)
                            if let Some(position) = request_json.get("Position") {
                                let dx = position["X"].as_f64().unwrap_or(0.0);
                                let dy = position["Y"].as_f64().unwrap_or(0.0);
                                let dz = position["Z"].as_f64().unwrap_or(0.0);

                                let speed = request_json.get("Speed").and_then(|v| v.as_f64()).unwrap_or(10.0);
                                let term_type = request_json.get("TermType").and_then(|v| v.as_str()).unwrap_or("FINE").to_string();
                                let term_value = request_json.get("TermValue").and_then(|v| v.as_u64()).unwrap_or(0);

                                // Get mode for logging
                                let mode = {
                                    let state = robot_state.lock().await;
                                    state.mode.clone()
                                };

                                println!("🎯 FRC_LinearRelative: ΔX={:+.1} ΔY={:+.1} ΔZ={:+.1} | Speed={:.1}mm/s | Term={} CNT={} | seq={}",
                                    dx, dy, dz, speed, term_type, term_value, seq);

                                // Queue the motion command - use is_relative=true to indicate this is a relative move
                                // The executor will add the offset to the current position at execution time
                                let cmd = MotionCommand {
                                    seq_id: seq,
                                    target_pos: [dx, dy, dz],  // Store the delta values
                                    target_ori: [0.0, 0.0, 0.0],  // No orientation change for relative
                                    speed,
                                    term_type,
                                    term_value,
                                    is_relative: true,
                                    instruction_type: "FRC_LinearRelative".to_string(),
                                };

                                if let Err(e) = motion_tx.send(cmd).await {
                                    eprintln!("❌ Failed to queue relative motion {}: {}", seq, e);
                                }

                                // In realtime mode, don't send immediate response
                                if mode == SimulatorMode::Realtime {
                                    continue;
                                }
                            }

                            json!({
                                "Instruction": "FRC_LinearRelative",
                                "ErrorID": 0,
                                "SequenceID": seq
                            })
                        }
                        _ => response_json,
                    };
                    let response = serde_json::to_string(&response_json)? + "\r\n";
                    socket.write_all(response.as_bytes()).await?;
                    seq += 1;
                }
            }
            // Check for motion responses to send back
            Some(motion_response) = response_rx.recv() => {
                eprintln!("📨 Received response from channel: seq_id={}", motion_response.seq_id);
                let response_json = json!({
                    "Instruction": motion_response.instruction_type,
                    "ErrorID": 0,
                    "SequenceID": motion_response.seq_id,
                });
                let response = serde_json::to_string(&response_json)? + "\r\n";
                eprintln!("📬 Sending to client: {}", response.trim());
                socket.write_all(response.as_bytes()).await?;
            }
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
