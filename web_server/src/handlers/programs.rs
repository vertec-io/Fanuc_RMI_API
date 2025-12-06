//! Program management handlers.
//!
//! Handles CRUD operations for programs and CSV upload.

use crate::api_types::*;
use crate::database::{Database, ProgramInstruction};
use crate::program_parser::{parse_csv_string, ProgramDefaults};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// List all programs.
pub async fn list_programs(db: Arc<Mutex<Database>>) -> ServerResponse {
    let db = db.lock().await;
    match db.list_programs() {
        Ok(programs) => {
            let program_infos: Vec<ProgramInfo> = programs.iter().map(|p| {
                let count = db.instruction_count(p.id).unwrap_or(0);
                ProgramInfo {
                    id: p.id,
                    name: p.name.clone(),
                    description: p.description.clone(),
                    instruction_count: count,
                    created_at: p.created_at.clone(),
                    updated_at: p.updated_at.clone(),
                }
            }).collect();
            ServerResponse::Programs { programs: program_infos }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to list programs: {}", e) }
    }
}

/// Get a program by ID.
pub async fn get_program(db: Arc<Mutex<Database>>, id: i64) -> ServerResponse {
    let db = db.lock().await;
    match db.get_program(id) {
        Ok(Some(program)) => {
            let instructions = db.get_instructions(id).unwrap_or_default();
            let instruction_dtos: Vec<InstructionDto> = instructions.iter().map(|i| {
                InstructionDto {
                    line_number: i.line_number,
                    x: i.x,
                    y: i.y,
                    z: i.z,
                    w: i.w,
                    p: i.p,
                    r: i.r,
                    speed: i.speed,
                    term_type: i.term_type.clone(),
                    uframe: i.uframe,
                    utool: i.utool,
                }
            }).collect();
            ServerResponse::Program {
                program: ProgramDetail {
                    id: program.id,
                    name: program.name,
                    description: program.description,
                    instructions: instruction_dtos,
                    start_x: program.start_x,
                    start_y: program.start_y,
                    start_z: program.start_z,
                }
            }
        }
        Ok(None) => ServerResponse::Error { message: "Program not found".to_string() },
        Err(e) => ServerResponse::Error { message: format!("Failed to get program: {}", e) }
    }
}

/// Create a new program.
pub async fn create_program(db: Arc<Mutex<Database>>, name: &str, description: Option<&str>) -> ServerResponse {
    let db = db.lock().await;
    match db.create_program(name, description) {
        Ok(id) => {
            info!("Created program '{}' with id {}", name, id);
            ServerResponse::Success { message: format!("Created program with id {}", id) }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to create program: {}", e) }
    }
}

/// Delete a program.
pub async fn delete_program(db: Arc<Mutex<Database>>, id: i64) -> ServerResponse {
    let db = db.lock().await;
    match db.delete_program(id) {
        Ok(_) => {
            info!("Deleted program {}", id);
            ServerResponse::Success { message: "Program deleted".to_string() }
        }
        Err(e) => ServerResponse::Error { message: format!("Failed to delete program: {}", e) }
    }
}

/// Upload CSV content to a program.
pub async fn upload_csv(
    db: Arc<Mutex<Database>>,
    program_id: i64,
    csv_content: &str,
    start_position: Option<StartPosition>,
) -> ServerResponse {
    let db = db.lock().await;
    
    // Get robot settings for defaults
    let settings = match db.get_robot_settings() {
        Ok(s) => s,
        Err(e) => return ServerResponse::Error { 
            message: format!("Failed to get robot settings: {}", e) 
        }
    };
    
    let defaults = ProgramDefaults {
        w: settings.default_w,
        p: settings.default_p,
        r: settings.default_r,
        ext1: 0.0,
        ext2: 0.0,
        ext3: 0.0,
        speed: settings.default_speed,
        term_type: settings.default_term_type.clone(),
        uframe: Some(settings.default_uframe),
        utool: Some(settings.default_utool),
        // Configuration defaults are not used during CSV parsing, only during execution
        front: None,
        up: None,
        left: None,
        flip: None,
        turn4: None,
        turn5: None,
        turn6: None,
    };

    // Parse CSV
    let instructions = match parse_csv_string(csv_content, &defaults) {
        Ok(instrs) => instrs,
        Err(e) => return ServerResponse::Error {
            message: format!("Failed to parse CSV: {:?}", e)
        }
    };

    // Clear existing instructions
    if let Err(e) = db.clear_instructions(program_id) {
        return ServerResponse::Error {
            message: format!("Failed to clear existing instructions: {}", e)
        };
    }

    // Add new instructions
    for instr in &instructions {
        let db_instr = ProgramInstruction {
            id: 0,
            program_id,
            line_number: instr.line_number,
            x: instr.x,
            y: instr.y,
            z: instr.z,
            w: instr.w,
            p: instr.p,
            r: instr.r,
            ext1: instr.ext1,
            ext2: instr.ext2,
            ext3: instr.ext3,
            speed: instr.speed,
            term_type: instr.term_type.clone(),
            uframe: instr.uframe,
            utool: instr.utool,
        };
        if let Err(e) = db.add_instruction(program_id, &db_instr) {
            return ServerResponse::Error {
                message: format!("Failed to add instruction: {}", e)
            };
        }
    }

    // Update start position if provided (use first instruction as default if not)
    let (start_x, start_y, start_z) = if let Some(start) = start_position {
        (Some(start.x), Some(start.y), Some(start.z))
    } else if let Some(first) = instructions.first() {
        (Some(first.x), Some(first.y), Some(first.z))
    } else {
        (Some(0.0), Some(0.0), Some(0.0))
    };

    // Update program with start position and defaults from robot settings
    if let Ok(Some(prog)) = db.get_program(program_id) {
        let _ = db.update_program(
            program_id,
            &prog.name,
            prog.description.as_deref(),
            settings.default_w,
            settings.default_p,
            settings.default_r,
            Some(settings.default_speed),
            &settings.default_term_type,
            Some(settings.default_uframe),
            Some(settings.default_utool),
            start_x,
            start_y,
            start_z,
        );
    }

    info!("Uploaded {} instructions to program {}", instructions.len(), program_id);
    ServerResponse::Success {
        message: format!("Uploaded {} instructions", instructions.len())
    }
}

