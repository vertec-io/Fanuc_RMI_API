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

/// Buffer size for instruction streaming (reserved for future buffered execution).
#[allow(dead_code)]
const BUFFER_SIZE: usize = 5;

/// Program execution status (reserved for future execution state machine).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ExecutionStatus {
    Idle,
    Running { current_line: usize, total_lines: usize },
    Paused { current_line: usize, total_lines: usize },
    Completed,
    Error(String),
}

/// Program executor manages program loading and execution.
pub struct ProgramExecutor {
    /// Currently loaded program.
    loaded_program: Option<Program>,
    /// Instructions for the loaded program.
    instructions: Vec<ProgramInstruction>,
    /// Current execution status.
    status: ExecutionStatus,
    /// Program defaults (from program or robot settings).
    defaults: ProgramDefaults,
    /// Current instruction index.
    current_index: usize,
    /// Number of instructions sent but not yet completed.
    pending_count: usize,
}

impl ProgramExecutor {
    /// Create a new program executor.
    pub fn new() -> Self {
        Self {
            loaded_program: None,
            instructions: Vec::new(),
            status: ExecutionStatus::Idle,
            defaults: ProgramDefaults::default(),
            current_index: 0,
            pending_count: 0,
        }
    }

    /// Load a program from the database.
    pub fn load_program(&mut self, db: &Database, program_id: i64) -> Result<(), String> {
        let program = db.get_program(program_id)
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Program {} not found", program_id))?;

        let instructions = db.get_instructions(program_id)
            .map_err(|e| format!("Failed to load instructions: {}", e))?;

        if instructions.is_empty() {
            return Err("Program has no instructions".to_string());
        }

        // Set defaults from program
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
        };

        self.loaded_program = Some(program);
        self.instructions = instructions;
        self.status = ExecutionStatus::Idle;
        self.current_index = 0;
        self.pending_count = 0;

        Ok(())
    }

    /// Unload the current program (reserved for future program management).
    #[allow(dead_code)]
    pub fn unload_program(&mut self) {
        self.loaded_program = None;
        self.instructions.clear();
        self.status = ExecutionStatus::Idle;
        self.current_index = 0;
        self.pending_count = 0;
    }

    /// Get the current execution status (reserved for future status display).
    #[allow(dead_code)]
    pub fn status(&self) -> &ExecutionStatus {
        &self.status
    }

    /// Get the loaded program (reserved for future program info display).
    #[allow(dead_code)]
    pub fn loaded_program(&self) -> Option<&Program> {
        self.loaded_program.as_ref()
    }

    /// Get the total number of instructions (reserved for future progress display).
    #[allow(dead_code)]
    pub fn total_instructions(&self) -> usize {
        self.instructions.len()
    }

    /// Get all motion packets for the loaded program.
    pub fn get_all_packets(&self) -> Vec<SendPacket> {
        let total = self.instructions.len();
        self.instructions.iter().enumerate().map(|(i, instr)| {
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

        // Build configuration with uframe/utool
        let uframe = instruction.uframe.unwrap_or(self.defaults.uframe.unwrap_or(0)) as u8;
        let utool = instruction.utool.unwrap_or(self.defaults.utool.unwrap_or(0)) as u8;
        let configuration = Configuration {
            u_tool_number: utool,
            u_frame_number: uframe,
            front: 0,
            up: 0,
            left: 0,
            flip: 0,
            turn4: 0,
            turn5: 0,
            turn6: 0,
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

