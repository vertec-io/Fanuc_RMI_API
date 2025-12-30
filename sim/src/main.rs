use serde_json::json;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, mpsc, RwLock};
use tokio::time::Duration;
use fanuc_rmi::{
    commands::*,
    packets::{CommandResponse, CommunicationResponse, InstructionResponse, FrcConnectResponse, FrcDisconnectResponse},
    instructions::{FrcLinearMotion, FrcLinearMotionResponse, FrcLinearRelative, FrcLinearRelativeResponse, FrcJointMotion, FrcJointMotionResponse, FrcJointRelativeJRep, FrcJointRelativeJRepResponse},
    FrameData, Configuration, Position, JointAngles,
};

mod kinematics;
mod robot_config;

use kinematics::CRXKinematics;

/// Helper to serialize a CommandResponse to JSON
fn serialize_response(response: CommandResponse) -> serde_json::Value {
    serde_json::to_value(&response).unwrap_or_else(|e| {
        eprintln!("Failed to serialize response: {}", e);
        json!({"ErrorID": 9999})
    })
}

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



/// Error code for invalid sequence ID (from FANUC RMI documentation)
const ERROR_INVALID_SEQUENCE_ID: u32 = 2556957;

// Simulated robot state - now using RwLock for concurrent read access
#[derive(Clone, Debug)]
struct RobotState {
    joint_angles: [f32; 6],
    cartesian_position: [f32; 3],
    cartesian_orientation: [f32; 3],
    kinematics: CRXKinematics,
    mode: SimulatorMode,
    last_sequence_id: u32, // Track the last completed sequence ID
    expected_next_sequence_id: u32, // Track the expected next sequence ID (for validation)
    // Frame/Tool state
    active_uframe: u8,
    active_utool: u8,
    uframes: [FrameData; 10],
    utools: [FrameData; 10],
    // I/O state
    din: [bool; 256],  // Digital inputs (simulated)
    dout: [bool; 256], // Digital outputs
    ain: [f64; 256],   // Analog inputs (simulated)
    aout: [f64; 256],  // Analog outputs
    gin: [u32; 256],   // Group inputs (simulated)
    gout: [u32; 256],  // Group outputs
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
            expected_next_sequence_id: 1, // Start expecting sequence ID 1
            // Initialize Frame/Tool state
            active_uframe: 0,
            active_utool: 0,
            uframes: [
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
            ],
            utools: [
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
                FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 },
            ],
            // Initialize I/O state
            din: [false; 256],
            dout: [false; 256],
            ain: [0.0; 256],
            aout: [0.0; 256],
            gin: [0; 256],
            gout: [0; 256],
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

            let response = CommunicationResponse::FrcConnect(FrcConnectResponse {
                error_id: 1,
                port_number: port as u32,
                major_version: 1,
                minor_version: 0,
            });
            serde_json::to_value(&response).unwrap_or_else(|e| {
                eprintln!("Failed to serialize FRC_Connect response: {}", e);
                serde_json::json!({"Communication": "FRC_Connect", "ErrorID": 1, "PortNumber": port, "MajorVersion": 1, "MinorVersion": 0})
            })
        }
        _ => {
            eprintln!("Unknown communication command in handshake");
            serde_json::json!({"Error": "Unknown command"})
        }
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
                            let cmd: FrcInitialize = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcInitialize { group_mask: 1 });

                            // Reset sequence tracking on initialize
                            {
                                let mut state = robot_state.lock().await;
                                state.last_sequence_id = 0;
                                state.expected_next_sequence_id = 1;
                                eprintln!("🔄 Sequence counter reset: expected_next=1");
                            }
                            let response = CommandResponse::FrcInitialize(FrcInitializeResponse {
                                error_id: 0,
                                group_mask: cmd.group_mask as u16,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_GetStatus") => {
                            let state = robot_state.lock().await;
                            // Use expected_next_sequence_id for NextSequenceID
                            let next_seq = state.expected_next_sequence_id;
                            let override_val = executor_control.get_speed_override();
                            let paused = if executor_control.is_paused() { 1 } else { 0 };
                            // Per FANUC documentation B-84184EN/02:
                            // TPMode: 0 = teach pendant disabled (RMI works), 1 = teach pendant enabled (RMI blocked)
                            // NumberUTool: Number of user tools available (10 for CRX-30iA)
                            // NumberUFrame: Number of user frames available (9 for CRX-30iA)
                            let response = CommandResponse::FrcGetStatus(FrcGetStatusResponse {
                                error_id: 0,
                                servo_ready: 1,
                                tp_mode: 0, // 0 = TP disabled, RMI can work
                                rmi_motion_status: paused, // 0=running, 1=paused
                                program_status: 0,
                                single_step_mode: 0,
                                number_utool: 10, // Number of user tools available (CRX-30iA)
                                number_uframe: 9, // Number of user frames available (CRX-30iA)
                                next_sequence_id: next_seq,
                                override_value: override_val as u32,
                            });
                            serialize_response(response)
                        },
                        Some("FRC_ReadJointAngles") => {
                            let cmd: FrcReadJointAngles = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcReadJointAngles { group: 1 });
                            let state = robot_state.lock().await;
                            let response = CommandResponse::FrcReadJointAngles(FrcReadJointAnglesResponse {
                                error_id: 0,
                                time_tag: 0,
                                joint_angles: JointAngles {
                                    j1: state.joint_angles[0],
                                    j2: state.joint_angles[1],
                                    j3: state.joint_angles[2],
                                    j4: state.joint_angles[3],
                                    j5: state.joint_angles[4],
                                    j6: state.joint_angles[5],
                                    j7: 0.0,
                                    j8: 0.0,
                                    j9: 0.0,
                                },
                                group: cmd.group,
                            });
                            serialize_response(response)
                        },
                        Some("FRC_ReadCartesianPosition") => {
                            let cmd: FrcReadCartesianPosition = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcReadCartesianPosition { group: 1 });
                            let state = robot_state.lock().await;
                            let response = CommandResponse::FrcReadCartesianPosition(FrcReadCartesianPositionResponse {
                                error_id: 0,
                                time_tag: 0,
                                config: Configuration {
                                    u_tool_number: state.active_utool as i8,
                                    u_frame_number: state.active_uframe as i8,
                                    front: 1,
                                    up: 1,
                                    left: 1,
                                    flip: 0,
                                    turn4: 0,
                                    turn5: 0,
                                    turn6: 0,
                                },
                                pos: Position {
                                    x: state.cartesian_position[0] as f64,
                                    y: state.cartesian_position[1] as f64,
                                    z: state.cartesian_position[2] as f64,
                                    w: state.cartesian_orientation[0] as f64,
                                    p: state.cartesian_orientation[1] as f64,
                                    r: state.cartesian_orientation[2] as f64,
                                    ext1: 0.0,
                                    ext2: 0.0,
                                    ext3: 0.0,
                                },
                                group: cmd.group,
                            });
                            serialize_response(response)
                        },
                        Some("FRC_Abort") => {
                            println!("🛑 FRC_Abort - signaling motion executor to abort immediately");
                            executor_control.request_abort();
                            // Also unpause if paused, so abort takes effect
                            executor_control.unpause();
                            let response = CommandResponse::FrcAbort(FrcAbortResponse {
                                error_id: 0,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_Pause") => {
                            println!("⏸️ FRC_Pause - pausing motion executor");
                            executor_control.pause();
                            let response = CommandResponse::FrcPause(FrcPauseResponse {
                                error_id: 0,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_Continue") => {
                            println!("▶️ FRC_Continue - resuming motion executor");
                            executor_control.unpause();
                            let response = CommandResponse::FrcContinue(FrcContinueResponse {
                                error_id: 0,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_Reset") => {
                            println!("🔄 FRC_Reset");
                            // Reset also clears abort/pause state
                            executor_control.clear_abort();
                            executor_control.unpause();
                            let response = CommandResponse::FrcReset(FrcResetResponse {
                                error_id: 0,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_SetOverRide") => {
                            let cmd: FrcSetOverRide = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcSetOverRide { value: 100 });
                            executor_control.set_speed_override(cmd.value);
                            println!("⚡ FRC_SetOverRide: {}%", cmd.value);
                            let response = CommandResponse::FrcSetOverRide(FrcSetOverRideResponse {
                                error_id: 0,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_GetUFrameUTool") => {
                            let cmd: FrcGetUFrameUTool = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcGetUFrameUTool { group: 1 });
                            let state = robot_state.lock().await;
                            let response = CommandResponse::FrcGetUFrameUTool(FrcGetUFrameUToolResponse {
                                error_id: 0,
                                u_frame_number: state.active_uframe,
                                u_tool_number: state.active_utool,
                                group: cmd.group as u16,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_SetUFrameUTool") => {
                            let cmd: FrcSetUFrameUTool = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcSetUFrameUTool { u_frame_number: 0, u_tool_number: 0, group: 1 });
                            let mut state = robot_state.lock().await;
                            state.active_uframe = cmd.u_frame_number;
                            state.active_utool = cmd.u_tool_number;
                            println!("🔧 FRC_SetUFrameUTool: UFrame={}, UTool={}", cmd.u_frame_number, cmd.u_tool_number);
                            let response = CommandResponse::FrcSetUFrameUTool(FrcSetUFrameUToolResponse {
                                error_id: 0,
                                group: cmd.group as u16,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_ReadUFrameData") => {
                            // Deserialize the command properly
                            let cmd: FrcReadUFrameData = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcReadUFrameData { frame_number: 0, group: 1 });

                            // REAL ROBOT BEHAVIOR:
                            // - Frame 0 (world frame) CANNOT be read - robot never responds (timeout)
                            // - Frames 1-9 can be read successfully
                            // - Frame 10+ don't exist (would return error on real robot)
                            //
                            // We simulate the timeout by simply not sending a response for frame 0
                            if cmd.frame_number == 0 {
                                eprintln!("⚠️ FRC_ReadUFrameData: Frame 0 requested - simulating timeout (real robot behavior)");
                                // Don't send any response - this will cause a timeout on the client
                                serde_json::json!({})  // Return empty to skip response
                            } else {
                                let state = robot_state.lock().await;
                                let frame_num = cmd.frame_number as usize;
                                let frame = state.uframes.get(frame_num).cloned().unwrap_or(FrameData {
                                    x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0
                                });

                                let response = CommandResponse::FrcReadUFrameData(FrcReadUFrameDataResponse {
                                    error_id: 0,
                                    frame_number: cmd.frame_number as u8,
                                    group: cmd.group,
                                    frame: FrameData {
                                        x: frame.x,
                                        y: frame.y,
                                        z: frame.z,
                                        w: frame.w,
                                        p: frame.p,
                                        r: frame.r,
                                    },
                                });
                                serialize_response(response)
                            }
                        }
                        Some("FRC_ReadUToolData") => {
                            // Deserialize the command properly
                            let cmd: FrcReadUToolData = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcReadUToolData { tool_number: 0, group: 1 });

                            // REAL ROBOT BEHAVIOR:
                            // - Tool 0 does NOT exist - returns Unknown error 2556950
                            // - Tools 1-10 are valid and can be read
                            // - Tool 11+ don't exist (would return error on real robot)
                            if cmd.tool_number == 0 {
                                eprintln!("⚠️ FRC_ReadUToolData: Tool 0 requested - returning Unknown error (real robot behavior)");
                                let response = CommandResponse::Unknown(FrcUnknownResponse {
                                    error_id: 2556950,  // Same error as real robot
                                });
                                serialize_response(response)
                            } else {
                                let state = robot_state.lock().await;
                                let tool_num = cmd.tool_number as usize;
                                let tool = state.utools.get(tool_num).cloned().unwrap_or(FrameData {
                                    x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0
                                });

                                let response = CommandResponse::FrcReadUToolData(FrcReadUToolDataResponse {
                                    error_id: 0,
                                    tool_number: cmd.tool_number as u8,
                                    group: cmd.group,
                                    frame: FrameData {
                                        x: tool.x,
                                        y: tool.y,
                                        z: tool.z,
                                        w: tool.w,
                                        p: tool.p,
                                        r: tool.r,
                                    },
                                });
                                serialize_response(response)
                            }
                        }
                        Some("FRC_WriteUFrameData") => {
                            let cmd: FrcWriteUFrameData = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcWriteUFrameData {
                                    frame_number: 0,
                                    group: 1,
                                    frame: FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 }
                                });
                            let mut state = robot_state.lock().await;
                            let frame_num = cmd.frame_number as usize;
                            if frame_num < 10 {
                                state.uframes[frame_num] = FrameData {
                                    x: cmd.frame.x,
                                    y: cmd.frame.y,
                                    z: cmd.frame.z,
                                    w: cmd.frame.w,
                                    p: cmd.frame.p,
                                    r: cmd.frame.r,
                                };
                                println!("📝 FRC_WriteUFrameData: UFrame {} updated", frame_num);
                            }
                            let response = CommandResponse::FrcWriteUFrameData(FrcWriteUFrameDataResponse {
                                error_id: 0,
                                group: cmd.group,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_WriteUToolData") => {
                            let cmd: FrcWriteUToolData = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcWriteUToolData {
                                    tool_number: 0,
                                    group: 1,
                                    frame: FrameData { x: 0.0, y: 0.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0 }
                                });
                            let mut state = robot_state.lock().await;
                            let tool_num = cmd.tool_number as usize;
                            if tool_num < 10 {
                                state.utools[tool_num] = FrameData {
                                    x: cmd.frame.x,
                                    y: cmd.frame.y,
                                    z: cmd.frame.z,
                                    w: cmd.frame.w,
                                    p: cmd.frame.p,
                                    r: cmd.frame.r,
                                };
                                println!("📝 FRC_WriteUToolData: UTool {} updated", tool_num);
                            }
                            let response = CommandResponse::FrcWriteUToolData(FrcWriteUToolDataResponse {
                                error_id: 0,
                                group: cmd.group,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_ReadDIN") => {
                            let cmd: FrcReadDIN = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcReadDIN { port_number: 0 });
                            let state = robot_state.lock().await;
                            let port_num = cmd.port_number as usize;
                            let port_value = if port_num < 256 { state.din[port_num] } else { false };
                            println!("📥 FRC_ReadDIN: Port {} = {}", port_num, if port_value { "ON" } else { "OFF" });
                            let response = CommandResponse::FrcReadDIN(FrcReadDINResponse {
                                error_id: 0,
                                port_number: cmd.port_number,
                                port_value: if port_value { 1 } else { 0 },
                            });
                            serialize_response(response)
                        }
                        Some("FRC_WriteDOUT") => {
                            let cmd: FrcWriteDOUT = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcWriteDOUT { port_number: 0, port_value: 0 });
                            let mut state = robot_state.lock().await;
                            let port_num = cmd.port_number as usize;
                            let port_value = cmd.port_value != 0;
                            if port_num < 256 {
                                state.dout[port_num] = port_value;
                            }
                            println!("📤 FRC_WriteDOUT: Port {} = {}", port_num, if port_value { "ON" } else { "OFF" });
                            let response = CommandResponse::FrcWriteDOUT(FrcWriteDOUTResponse {
                                error_id: 0,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_ReadAIN") => {
                            let cmd: FrcReadAIN = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcReadAIN { port_number: 0 });
                            let state = robot_state.lock().await;
                            let port_num = cmd.port_number as usize;
                            let port_value = if port_num < 256 { state.ain[port_num] } else { 0.0 };
                            println!("📥 FRC_ReadAIN: Port {} = {:.2}", port_num, port_value);
                            let response = CommandResponse::FrcReadAIN(FrcReadAINResponse {
                                error_id: 0,
                                port_number: cmd.port_number,
                                port_value,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_WriteAOUT") => {
                            let cmd: FrcWriteAOUT = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcWriteAOUT { port_number: 0, port_value: 0.0 });
                            let mut state = robot_state.lock().await;
                            let port_num = cmd.port_number as usize;
                            if port_num < 256 {
                                state.aout[port_num] = cmd.port_value;
                            }
                            println!("📤 FRC_WriteAOUT: Port {} = {:.2}", port_num, cmd.port_value);
                            let response = CommandResponse::FrcWriteAOUT(FrcWriteAOUTResponse {
                                error_id: 0,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_ReadGIN") => {
                            let cmd: FrcReadGIN = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcReadGIN { port_number: 0 });
                            let state = robot_state.lock().await;
                            let port_num = cmd.port_number as usize;
                            let port_value = if port_num < 256 { state.gin[port_num] } else { 0 };
                            println!("📥 FRC_ReadGIN: Port {} = {}", port_num, port_value);
                            let response = CommandResponse::FrcReadGIN(FrcReadGINResponse {
                                error_id: 0,
                                port_number: cmd.port_number,
                                port_value,
                            });
                            serialize_response(response)
                        }
                        Some("FRC_WriteGOUT") => {
                            let cmd: FrcWriteGOUT = serde_json::from_value(request_json.clone())
                                .unwrap_or(FrcWriteGOUT { port_number: 0, port_value: 0 });
                            let mut state = robot_state.lock().await;
                            let port_num = cmd.port_number as usize;
                            if port_num < 256 {
                                state.gout[port_num] = cmd.port_value;
                            }
                            println!("📤 FRC_WriteGOUT: Port {} = {}", port_num, cmd.port_value);
                            let response = CommandResponse::FrcWriteGOUT(FrcWriteGOUTResponse {
                                error_id: 0,
                            });
                            serialize_response(response)
                        }
                        _ => {
                            // Unknown command - return proper Unknown response
                            eprintln!("⚠️ Unknown command: {:?}", request_json.get("Command"));
                            let response = CommandResponse::Unknown(FrcUnknownResponse {
                                error_id: 2556950,  // InvalidTextString error (same as real robot)
                            });
                            serialize_response(response)
                        }
                    };

                    response_json = match request_json["Communication"].as_str() {
                        Some("FRC_Disconnect") => {
                            println!("👋 FRC_Disconnect\n");
                            let response = CommunicationResponse::FrcDisconnect(FrcDisconnectResponse {
                                error_id: 0,
                            });
                            serde_json::to_value(&response).unwrap_or_else(|e| {
                                eprintln!("Failed to serialize FRC_Disconnect response: {}", e);
                                json!({"Communication": "FRC_Disconnect", "ErrorID": 0})
                            })
                        }
                        _ => response_json,
                    };

                    // Extract SequenceID from instruction requests (if present)
                    if let Some(seq_id) = request_json.get("SequenceID").and_then(|v| v.as_u64()) {
                        seq = seq_id as u32;
                    }

                    // Validate sequence ID for motion instructions
                    let is_motion_instruction = matches!(
                        request_json["Instruction"].as_str(),
                        Some("FRC_LinearMotion") | Some("FRC_LinearRelative") | Some("FRC_JointMotion") | Some("FRC_JointRelativeJRep")
                    );

                    if is_motion_instruction {
                        let mut state = robot_state.lock().await;
                        let expected = state.expected_next_sequence_id;

                        if seq != expected {
                            eprintln!("❌ Sequence ID mismatch: received {} but expected {}", seq, expected);
                            // Return a generic error response for invalid sequence ID
                            // We use FrcLinearMotionResponse as a generic instruction error response
                            let error_response = InstructionResponse::FrcLinearMotion(FrcLinearMotionResponse {
                                error_id: ERROR_INVALID_SEQUENCE_ID,
                                sequence_id: seq,
                            });
                            let error_json = serde_json::to_value(&error_response).unwrap_or_else(|e| {
                                eprintln!("Failed to serialize error response: {}", e);
                                serde_json::json!({"Instruction": "FRC_LinearMotion", "ErrorID": ERROR_INVALID_SEQUENCE_ID, "SequenceID": seq})
                            });
                            let response = serde_json::to_string(&error_json)? + "\r\n";
                            socket.write_all(response.as_bytes()).await?;
                            continue; // Skip processing this instruction
                        }

                        // Increment expected sequence ID for next instruction
                        state.expected_next_sequence_id = seq + 1;
                        eprintln!("✓ Sequence ID {} validated, next expected: {}", seq, state.expected_next_sequence_id);
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

                            let response = InstructionResponse::FrcLinearMotion(FrcLinearMotionResponse {
                                error_id: 0,
                                sequence_id: seq,
                            });
                            serde_json::to_value(&response).unwrap_or_else(|e| {
                                eprintln!("Failed to serialize FRC_LinearMotion response: {}", e);
                                serde_json::json!({"Instruction": "FRC_LinearMotion", "ErrorID": 0, "SequenceID": seq})
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

                            let response = InstructionResponse::FrcLinearRelative(FrcLinearRelativeResponse {
                                error_id: 0,
                                sequence_id: seq,
                            });
                            serde_json::to_value(&response).unwrap_or_else(|e| {
                                eprintln!("Failed to serialize FRC_LinearRelative response: {}", e);
                                serde_json::json!({"Instruction": "FRC_LinearRelative", "ErrorID": 0, "SequenceID": seq})
                            })
                        }
                        Some("FRC_JointRelativeJRep") => {
                            // Parse the JointAngles from the instruction (relative joint motion)
                            if let Some(joint_angles) = request_json.get("JointAngles") {
                                let dj1 = joint_angles["J1"].as_f64().unwrap_or(0.0);
                                let dj2 = joint_angles["J2"].as_f64().unwrap_or(0.0);
                                let dj3 = joint_angles["J3"].as_f64().unwrap_or(0.0);
                                let dj4 = joint_angles["J4"].as_f64().unwrap_or(0.0);
                                let dj5 = joint_angles["J5"].as_f64().unwrap_or(0.0);
                                let dj6 = joint_angles["J6"].as_f64().unwrap_or(0.0);

                                let speed = request_json.get("Speed").and_then(|v| v.as_f64()).unwrap_or(10.0);
                                let term_type = request_json.get("TermType").and_then(|v| v.as_str()).unwrap_or("FINE").to_string();
                                let term_value = request_json.get("TermValue").and_then(|v| v.as_u64()).unwrap_or(0);

                                // Get current joint angles and apply delta
                                {
                                    let mut state = robot_state.lock().await;
                                    let new_j1 = state.joint_angles[0] as f64 + dj1;
                                    let new_j2 = state.joint_angles[1] as f64 + dj2;
                                    let new_j3 = state.joint_angles[2] as f64 + dj3;
                                    let new_j4 = state.joint_angles[3] as f64 + dj4;
                                    let new_j5 = state.joint_angles[4] as f64 + dj5;
                                    let new_j6 = state.joint_angles[5] as f64 + dj6;

                                    // Update joint angles immediately (for immediate mode)
                                    state.joint_angles[0] = new_j1 as f32;
                                    state.joint_angles[1] = new_j2 as f32;
                                    state.joint_angles[2] = new_j3 as f32;
                                    state.joint_angles[3] = new_j4 as f32;
                                    state.joint_angles[4] = new_j5 as f32;
                                    state.joint_angles[5] = new_j6 as f32;

                                    // Update Cartesian position using forward kinematics
                                    let joints_rad = [new_j1, new_j2, new_j3, new_j4, new_j5, new_j6];
                                    let (pos, ori) = state.kinematics.forward_kinematics(&joints_rad);
                                        state.cartesian_position[0] = pos[0] as f32;
                                        state.cartesian_position[1] = pos[1] as f32;
                                        state.cartesian_position[2] = pos[2] as f32;
                                        state.cartesian_orientation[0] = ori[0] as f32;
                                        state.cartesian_orientation[1] = ori[1] as f32;
                                        state.cartesian_orientation[2] = ori[2] as f32;
                                };

                                println!("🎯 FRC_JointRelativeJRep: ΔJ1={:+.2}° ΔJ2={:+.2}° ΔJ3={:+.2}° ΔJ4={:+.2}° ΔJ5={:+.2}° ΔJ6={:+.2}° | Speed={:.1}°/s | Term={} CNT={} | seq={}",
                                    dj1.to_degrees(), dj2.to_degrees(), dj3.to_degrees(), dj4.to_degrees(), dj5.to_degrees(), dj6.to_degrees(), speed, term_type, term_value, seq);
                            }

                            let response = InstructionResponse::FrcJointRelativeJRep(FrcJointRelativeJRepResponse {
                                error_id: 0,
                                sequence_id: seq,
                            });
                            serde_json::to_value(&response).unwrap_or_else(|e| {
                                eprintln!("Failed to serialize FRC_JointRelativeJRep response: {}", e);
                                serde_json::json!({"Instruction": "FRC_JointRelativeJRep", "ErrorID": 0, "SequenceID": seq})
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

                // Create the appropriate InstructionResponse based on instruction type
                let response_enum = match motion_response.instruction_type.as_str() {
                    "FRC_LinearMotion" => InstructionResponse::FrcLinearMotion(FrcLinearMotionResponse {
                        error_id: 0,
                        sequence_id: motion_response.seq_id,
                    }),
                    "FRC_LinearRelative" => InstructionResponse::FrcLinearRelative(FrcLinearRelativeResponse {
                        error_id: 0,
                        sequence_id: motion_response.seq_id,
                    }),
                    "FRC_JointRelativeJRep" => InstructionResponse::FrcJointRelativeJRep(FrcJointRelativeJRepResponse {
                        error_id: 0,
                        sequence_id: motion_response.seq_id,
                    }),
                    _ => {
                        eprintln!("⚠️ Unknown instruction type: {}", motion_response.instruction_type);
                        InstructionResponse::FrcLinearMotion(FrcLinearMotionResponse {
                            error_id: 0,
                            sequence_id: motion_response.seq_id,
                        })
                    }
                };

                let response_json = serde_json::to_value(&response_enum).unwrap_or_else(|e| {
                    eprintln!("Failed to serialize motion response: {}", e);
                    serde_json::json!({"Instruction": motion_response.instruction_type, "ErrorID": 0, "SequenceID": motion_response.seq_id})
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
