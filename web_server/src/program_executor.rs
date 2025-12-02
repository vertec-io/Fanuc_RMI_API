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
    /// * `robot_defaults` - Optional robot connection defaults for configuration (front, up, left, flip, turn4, turn5, turn6)
    pub fn load_program(
        &mut self,
        db: &Database,
        program_id: i64,
        robot_defaults: Option<&crate::database::RobotConnection>,
    ) -> Result<(), String> {
        let program = db.get_program(program_id)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Program {} not found", program_id))?;

        let instructions = db.get_instructions(program_id)
            .map_err(|e| format!("Failed to load instructions: {}", e))?;

        if instructions.is_empty() {
            return Err("Program has no instructions".to_string());
        }

        // Set defaults from program, with robot connection defaults for configuration
        self.defaults = ProgramDefaults {
            w: program.default_w,
            p: program.default_p,
            r: program.default_r,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
            speed: program.default_speed.unwrap_or(100.0),
            term_type: program.default_term_type.clone(),
            uframe: program.default_uframe,
            utool: program.default_utool,
            // Use robot connection defaults for configuration if available
            front: robot_defaults.and_then(|r| r.default_front),
            up: robot_defaults.and_then(|r| r.default_up),
            left: robot_defaults.and_then(|r| r.default_left),
            flip: robot_defaults.and_then(|r| r.default_flip),
            turn4: robot_defaults.and_then(|r| r.default_turn4),
            turn5: robot_defaults.and_then(|r| r.default_turn5),
            turn6: robot_defaults.and_then(|r| r.default_turn6),
        };

        // Build pending queue with all instructions
        let total = instructions.len();
        self.pending_queue.clear();
        for (i, instr) in instructions.iter().enumerate() {
            let line_number = i + 1;
            let is_last = i == total - 1;
            let packet = self.build_motion_packet(instr, is_last);
            self.pending_queue.push_back((line_number, packet));
        }

        self.loaded_program = Some(program);
        self.all_instructions = instructions.clone();
        self.state = ExecutionState::Loaded {
            program_id,
            total_lines: instructions.len(),
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

    /// Get the total number of instructions.
    pub fn total_instructions(&self) -> usize {
        self.all_instructions.len()
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

        // Use FINE for last instruction, otherwise CNT
        let term_type = if is_last {
            TermType::FINE
        } else {
            match instruction.term_type.as_deref().unwrap_or(&self.defaults.term_type) {
                "FINE" => TermType::FINE,
                _ => TermType::CNT,
            }
        };

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
        let uframe = instruction.uframe.unwrap_or(self.defaults.uframe.unwrap_or(0)) as u8;
        let utool = instruction.utool.unwrap_or(self.defaults.utool.unwrap_or(0)) as u8;
        let configuration = Configuration {
            u_tool_number: utool,
            u_frame_number: uframe,
            front: self.defaults.front.unwrap_or(1) as u8,  // Default: Front
            up: self.defaults.up.unwrap_or(1) as u8,        // Default: Up
            left: self.defaults.left.unwrap_or(0) as u8,    // Default: Right
            flip: self.defaults.flip.unwrap_or(0) as u8,    // Default: NoFlip
            turn4: self.defaults.turn4.unwrap_or(0) as u8,
            turn5: self.defaults.turn5.unwrap_or(0) as u8,
            turn6: self.defaults.turn6.unwrap_or(0) as u8,
        };

        let motion = FrcLinearMotion::new(
            instruction.line_number as u32,
            configuration,
            position,
            SpeedType::MMSec,
            speed,
            term_type,
            0, // term_value
        );

        SendPacket::Instruction(Instruction::FrcLinearMotion(motion))
    }
}

