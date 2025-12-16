//! Program executor with buffered streaming.
//!
//! Executes robot programs with:
//! - 5-instruction buffer (sends 5 ahead, streams as they complete)
//! - CNT termination for all instructions except the last
//! - FINE termination for the last instruction
//! - Progress tracking and status updates

use crate::database::{Database, Program, ProgramInstruction};
use crate::program_parser::ProgramDefaults;
use fanuc_rmi::packets::{SendPacket, Instruction};
use fanuc_rmi::instructions::FrcLinearMotion;
use fanuc_rmi::{TermType, SpeedType, Configuration, Position};
use std::collections::{VecDeque, HashMap};
use tracing::info;

/// Maximum instructions to send ahead (conservative: use 5 of 8 available slots).
pub const MAX_BUFFER: usize = 5;

/// Program execution state.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionState {
    /// No program loaded.
    Idle,
    /// Program is loaded but not yet running.
    Loaded {
        program_id: i64,
        total_lines: usize,
    },
    /// Program is running, sending instructions as buffer allows.
    Running {
        program_id: i64,
        total_lines: usize,
        last_completed: usize,
    },
    /// Program is paused (no new instructions sent, waiting for in-flight to complete or resume).
    Paused {
        program_id: i64,
        total_lines: usize,
        last_completed: usize,
    },
    /// Stopping: draining in-flight before transitioning to Idle.
    Stopping,
    /// Completed successfully.
    Completed { program_id: i64, total_lines: usize },
    /// Error occurred.
    Error { message: String },
}

/// Program executor manages program loading and buffered execution.
pub struct ProgramExecutor {
    /// Currently loaded program.
    loaded_program: Option<Program>,
    /// All instructions for the loaded program.
    all_instructions: Vec<ProgramInstruction>,
    /// Program defaults (from program or robot settings).
    defaults: ProgramDefaults,

    /// Current execution state.
    pub state: ExecutionState,
    /// Instructions waiting to be sent (line_number, packet).
    pending_queue: VecDeque<(usize, SendPacket)>,
    /// Instructions sent but not yet completed: request_id -> line_number.
    /// Updated to sequence_id -> line_number when SentInstructionInfo arrives.
    in_flight_by_request: HashMap<u64, usize>,
    /// Sequence ID to line number mapping (populated when instruction is actually sent).
    in_flight_by_sequence: HashMap<u32, usize>,
    /// Highest completed line number.
    completed_line: usize,
}

impl ProgramExecutor {
    /// Create a new program executor.
    pub fn new() -> Self {
        Self {
            loaded_program: None,
            all_instructions: Vec::new(),
            defaults: ProgramDefaults::default(),
            state: ExecutionState::Idle,
            pending_queue: VecDeque::new(),
            in_flight_by_request: HashMap::new(),
            in_flight_by_sequence: HashMap::new(),
            completed_line: 0,
        }
    }

    /// Load a program from the database and prepare for execution.
    ///
    /// # Arguments
    /// * `db` - Database connection
    /// * `program_id` - ID of the program to load
    /// * `active_config` - Optional active configuration for arm configuration (front, up, left, flip, turn4, turn5, turn6)
    /// * `default_speed_type` - Default speed type from robot connection (mmSec, InchMin, Time, mSec)
    pub fn load_program(
        &mut self,
        db: &Database,
        program_id: i64,
        active_config: Option<&crate::ActiveConfiguration>,
        default_speed_type: &str,
    ) -> Result<(), String> {
        let program = db.get_program(program_id)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Program {} not found", program_id))?;

        let instructions = db.get_instructions(program_id)
            .map_err(|e| format!("Failed to load instructions: {}", e))?;

        if instructions.is_empty() {
            return Err("Program has no instructions".to_string());
        }

        // Set defaults from program, with active configuration for arm configuration and frame/tool
        // Priority for uframe/utool:
        // 1. Program default (if specified)
        // 2. Active robot configuration (if available)
        // 3. Fallback to 1 (FANUC uses 1-based indexing)
        let active_uframe = active_config.map(|c| c.u_frame_number);
        let active_utool = active_config.map(|c| c.u_tool_number);

        self.defaults = ProgramDefaults {
            w: program.default_w,
            p: program.default_p,
            r: program.default_r,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
            speed: program.default_speed.unwrap_or(100.0),
            speed_type: default_speed_type.to_string(),
            term_type: program.default_term_type.clone(),
            term_value: program.default_term_value,
            // Use program defaults, fall back to active robot configuration, then to 1
            uframe: program.default_uframe.or(active_uframe),
            utool: program.default_utool.or(active_utool),
            // Use active configuration for arm configuration
            front: active_config.map(|c| c.front),
            up: active_config.map(|c| c.up),
            left: active_config.map(|c| c.left),
            flip: active_config.map(|c| c.flip),
            turn4: active_config.map(|c| c.turn4),
            turn5: active_config.map(|c| c.turn5),
            turn6: active_config.map(|c| c.turn6),
        };

        // Build pending queue with all instructions
        let total = instructions.len();
        self.pending_queue.clear();

        // Add approach move (start position) if defined
        // Line 0 is used for approach move so program instructions start at line 1
        if let (Some(start_x), Some(start_y), Some(start_z)) = (program.start_x, program.start_y, program.start_z) {
            let approach_packet = self.build_approach_retreat_packet(
                &program,
                start_x, start_y, start_z,
                program.start_w, program.start_p, program.start_r,
                0, // Line 0 for approach
                false, // Not last instruction - use CNT
            );
            self.pending_queue.push_back((0, approach_packet));
            info!("Added approach move to ({:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}) at speed {:.0}",
                  start_x, start_y, start_z,
                  program.start_w.unwrap_or(program.default_w),
                  program.start_p.unwrap_or(program.default_p),
                  program.start_r.unwrap_or(program.default_r),
                  program.move_speed.unwrap_or(100.0));
        }

        // Add program instructions (lines 1 through N)
        let has_retreat = program.end_x.is_some() && program.end_y.is_some() && program.end_z.is_some();
        for (i, instr) in instructions.iter().enumerate() {
            let line_number = i + 1;
            // If there's a retreat move, the last program instruction is NOT the last overall
            let is_last_overall = !has_retreat && (i == total - 1);
            let packet = self.build_motion_packet(instr, is_last_overall);
            self.pending_queue.push_back((line_number, packet));
        }

        // Add retreat move (end position) if defined
        // Use line total+1 for retreat move
        if let (Some(end_x), Some(end_y), Some(end_z)) = (program.end_x, program.end_y, program.end_z) {
            let retreat_packet = self.build_approach_retreat_packet(
                &program,
                end_x, end_y, end_z,
                program.end_w, program.end_p, program.end_r,
                total + 1, // Line after last instruction
                true, // Last instruction - use FINE
            );
            self.pending_queue.push_back((total + 1, retreat_packet));
            info!("Added retreat move to ({:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}) at speed {:.0}",
                  end_x, end_y, end_z,
                  program.end_w.unwrap_or(program.default_w),
                  program.end_p.unwrap_or(program.default_p),
                  program.end_r.unwrap_or(program.default_r),
                  program.move_speed.unwrap_or(100.0));
        }

        // Calculate total lines including approach/retreat
        let has_approach = program.start_x.is_some() && program.start_y.is_some() && program.start_z.is_some();
        let total_with_extras = instructions.len() + (if has_approach { 1 } else { 0 }) + (if has_retreat { 1 } else { 0 });

        self.loaded_program = Some(program);
        self.all_instructions = instructions.clone();
        self.state = ExecutionState::Loaded {
            program_id,
            total_lines: total_with_extras,
        };
        self.in_flight_by_request.clear();
        self.in_flight_by_sequence.clear();
        self.completed_line = 0;

        Ok(())
    }

    /// Reset the executor to idle state.
    pub fn reset(&mut self) {
        self.loaded_program = None;
        self.all_instructions.clear();
        self.pending_queue.clear();
        self.in_flight_by_request.clear();
        self.in_flight_by_sequence.clear();
        self.state = ExecutionState::Idle;
        self.completed_line = 0;
    }

    /// Get the current execution state.
    pub fn get_state(&self) -> &ExecutionState {
        &self.state
    }

    /// Get the loaded program.
    pub fn loaded_program(&self) -> Option<&Program> {
        self.loaded_program.as_ref()
    }

    /// Get the total number of instructions (including approach/retreat moves).
    pub fn total_instructions(&self) -> usize {
        match &self.state {
            ExecutionState::Loaded { total_lines, .. } => *total_lines,
            ExecutionState::Running { total_lines, .. } => *total_lines,
            ExecutionState::Paused { total_lines, .. } => *total_lines,
            ExecutionState::Completed { total_lines, .. } => *total_lines,
            _ => self.all_instructions.len(),
        }
    }

    /// Get the highest completed line number.
    pub fn completed_line(&self) -> usize {
        self.completed_line
    }

    /// Get the number of in-flight instructions.
    pub fn in_flight_count(&self) -> usize {
        self.in_flight_by_sequence.len()
    }

    /// Check if there are more instructions to send.
    pub fn has_pending(&self) -> bool {
        !self.pending_queue.is_empty()
    }

    /// Start execution (transition from Loaded to Running).
    pub fn start(&mut self) {
        if let ExecutionState::Loaded { program_id, total_lines } = self.state {
            self.state = ExecutionState::Running {
                program_id,
                total_lines,
                last_completed: 0,
            };
        }
    }

    /// Pause execution (stop sending new instructions).
    pub fn pause(&mut self) {
        if let ExecutionState::Running { program_id, total_lines, last_completed } = self.state {
            self.state = ExecutionState::Paused {
                program_id,
                total_lines,
                last_completed,
            };
        }
    }

    /// Resume execution (continue sending instructions).
    pub fn resume(&mut self) {
        if let ExecutionState::Paused { program_id, total_lines, last_completed } = self.state {
            self.state = ExecutionState::Running {
                program_id,
                total_lines,
                last_completed,
            };
        }
    }

    /// Stop execution (clear queues, transition to Stopping then Idle).
    pub fn stop(&mut self) {
        self.pending_queue.clear();
        self.state = ExecutionState::Stopping;
    }

    /// Clear in-flight tracking (called after abort completes).
    pub fn clear_in_flight(&mut self) {
        self.in_flight_by_request.clear();
        self.in_flight_by_sequence.clear();
        self.state = ExecutionState::Idle;
    }

    /// Get the next batch of instructions to send (up to MAX_BUFFER - in_flight).
    /// Returns Vec of (line_number, packet, request_id placeholder).
    pub fn get_next_batch(&mut self) -> Vec<(usize, SendPacket)> {
        let can_send = MAX_BUFFER.saturating_sub(self.in_flight_by_sequence.len());
        let mut batch = Vec::new();

        for _ in 0..can_send {
            if let Some((line, packet)) = self.pending_queue.pop_front() {
                batch.push((line, packet));
            } else {
                break;
            }
        }

        batch
    }

    /// Record that an instruction was sent (by request_id).
    pub fn record_sent(&mut self, request_id: u64, line_number: usize) {
        self.in_flight_by_request.insert(request_id, line_number);
    }

    /// Map request_id to sequence_id when SentInstructionInfo arrives.
    pub fn map_sequence(&mut self, request_id: u64, sequence_id: u32) {
        if let Some(line) = self.in_flight_by_request.remove(&request_id) {
            self.in_flight_by_sequence.insert(sequence_id, line);
        }
    }

    /// Handle instruction completion by sequence_id.
    /// Returns the line number if found, and updates state.
    pub fn handle_completion(&mut self, sequence_id: u32) -> Option<usize> {
        if let Some(line) = self.in_flight_by_sequence.remove(&sequence_id) {
            self.completed_line = self.completed_line.max(line);

            // Update state with new completed line
            match &mut self.state {
                ExecutionState::Running { last_completed, .. } => {
                    *last_completed = self.completed_line;
                }
                ExecutionState::Paused { last_completed, .. } => {
                    *last_completed = self.completed_line;
                }
                _ => {}
            }

            // Check for completion
            if self.pending_queue.is_empty() && self.in_flight_by_sequence.is_empty() {
                if let ExecutionState::Running { program_id, total_lines, .. } = self.state {
                    self.state = ExecutionState::Completed { program_id, total_lines };
                }
            }

            Some(line)
        } else {
            None
        }
    }

    /// Check if execution is complete.
    pub fn is_complete(&self) -> bool {
        matches!(self.state, ExecutionState::Completed { .. })
    }

    /// Check if execution is running (not paused, not stopped).
    pub fn is_running(&self) -> bool {
        matches!(self.state, ExecutionState::Running { .. })
    }

    /// Get all motion packets for the loaded program (legacy method for compatibility).
    pub fn get_all_packets(&self) -> Vec<SendPacket> {
        let total = self.all_instructions.len();
        self.all_instructions.iter().enumerate().map(|(i, instr)| {
            self.build_motion_packet(instr, i == total - 1)
        }).collect()
    }

    /// Build a motion instruction packet from a program instruction.
    fn build_motion_packet(&self, instruction: &ProgramInstruction, is_last: bool) -> SendPacket {
        // Use instruction values or fall back to defaults
        let w = instruction.w.unwrap_or(self.defaults.w);
        let p = instruction.p.unwrap_or(self.defaults.p);
        let r = instruction.r.unwrap_or(self.defaults.r);
        let ext1 = instruction.ext1.unwrap_or(self.defaults.ext1);
        let ext2 = instruction.ext2.unwrap_or(self.defaults.ext2);
        let ext3 = instruction.ext3.unwrap_or(self.defaults.ext3);
        let speed = instruction.speed.unwrap_or(self.defaults.speed);

        // Parse speed_type from instruction or use defaults
        let speed_type_str = instruction.speed_type.as_deref().unwrap_or(&self.defaults.speed_type);
        let speed_type = match speed_type_str {
            "mmSec" => SpeedType::MMSec,
            "InchMin" => SpeedType::InchMin,
            "Time" => SpeedType::Time,
            "mSec" => SpeedType::MilliSeconds,
            _ => SpeedType::MMSec,  // Fallback to mmSec if invalid
        };

        // Use FINE for last instruction, otherwise use instruction's term_type or program default
        let term_type = if is_last {
            TermType::FINE
        } else {
            match instruction.term_type.as_deref().unwrap_or(&self.defaults.term_type) {
                "FINE" => TermType::FINE,
                _ => TermType::CNT,
            }
        };

        // Determine term_value:
        // 1. Use instruction's term_value if specified
        // 2. Fall back to program's default_term_value
        // 3. Fall back to sensible default: 100 for CNT (max smoothness), 0 for FINE
        let term_value = instruction.term_value
            .or(self.defaults.term_value)
            .unwrap_or_else(|| {
                match term_type {
                    TermType::CNT => 100,  // Maximum smoothness for CNT
                    TermType::FINE => 0,   // FINE doesn't use term_value
                    TermType::CR => 0,     // CR uses different semantics
                }
            });

        let position = Position {
            x: instruction.x,
            y: instruction.y,
            z: instruction.z,
            w,
            p,
            r,
            ext1,
            ext2,
            ext3,
        };

        // Build configuration with uframe/utool and robot arm configuration defaults
        // FANUC uses 1-based indexing for frames/tools, so default to 1 (not 0)
        let uframe = instruction.uframe.unwrap_or(self.defaults.uframe.unwrap_or(1)) as i8;
        let utool = instruction.utool.unwrap_or(self.defaults.utool.unwrap_or(1)) as i8;
        let configuration = Configuration {
            u_tool_number: utool,
            u_frame_number: uframe,
            front: self.defaults.front.unwrap_or(1) as i8,  // Default: Front
            up: self.defaults.up.unwrap_or(1) as i8,        // Default: Up
            left: self.defaults.left.unwrap_or(0) as i8,    // Default: Right
            flip: self.defaults.flip.unwrap_or(0) as i8,    // Default: NoFlip
            turn4: self.defaults.turn4.unwrap_or(0) as i8,
            turn5: self.defaults.turn5.unwrap_or(0) as i8,
            turn6: self.defaults.turn6.unwrap_or(0) as i8,
        };

        let motion = FrcLinearMotion::new(
            instruction.line_number as u32,
            configuration,
            position,
            speed_type,
            speed,
            term_type,
            term_value,
        );

        SendPacket::Instruction(Instruction::FrcLinearMotion(motion))
    }

    /// Build an approach or retreat motion packet (for start/end positions).
    ///
    /// Uses the program's move_speed. Orientation (W, P, R) uses provided values if set,
    /// otherwise falls back to program defaults.
    #[allow(clippy::too_many_arguments)]
    fn build_approach_retreat_packet(
        &self,
        program: &Program,
        x: f64,
        y: f64,
        z: f64,
        w: Option<f64>,
        p: Option<f64>,
        r: Option<f64>,
        line_number: usize,
        is_last: bool,
    ) -> SendPacket {
        // Use provided orientation or fall back to program defaults
        let w = w.unwrap_or(program.default_w);
        let p = p.unwrap_or(program.default_p);
        let r = r.unwrap_or(program.default_r);

        // Use move_speed or default to 100 mm/s
        let speed = program.move_speed.unwrap_or(100.0);

        // Always use mmSec for approach/retreat moves
        let speed_type = SpeedType::MMSec;

        // Use FINE for last move (retreat), CNT for approach
        let (term_type, term_value) = if is_last {
            (TermType::FINE, 0)
        } else {
            // Use program's default term_value for approach, default to 100 for CNT
            let tv = program.default_term_value.unwrap_or(100);
            (TermType::CNT, tv)
        };

        let position = Position {
            x,
            y,
            z,
            w,
            p,
            r,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        };

        // Build configuration with uframe/utool and robot arm configuration defaults
        // FANUC uses 1-based indexing for frames/tools, so default to 1 (not 0)
        let uframe = self.defaults.uframe.unwrap_or(1) as i8;
        let utool = self.defaults.utool.unwrap_or(1) as i8;
        let configuration = Configuration {
            u_tool_number: utool,
            u_frame_number: uframe,
            front: self.defaults.front.unwrap_or(1) as i8,
            up: self.defaults.up.unwrap_or(1) as i8,
            left: self.defaults.left.unwrap_or(0) as i8,
            flip: self.defaults.flip.unwrap_or(0) as i8,
            turn4: self.defaults.turn4.unwrap_or(0) as i8,
            turn5: self.defaults.turn5.unwrap_or(0) as i8,
            turn6: self.defaults.turn6.unwrap_or(0) as i8,
        };

        let motion = FrcLinearMotion::new(
            line_number as u32,
            configuration,
            position,
            speed_type,
            speed,
            term_type,
            term_value,
        );

        SendPacket::Instruction(Instruction::FrcLinearMotion(motion))
    }
}

