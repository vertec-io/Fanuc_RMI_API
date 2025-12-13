//! Program management handlers.
//!
//! Handles CRUD operations for programs and CSV upload.

use crate::api_types::*;
use crate::database::{Database, ProgramInstruction};
use crate::program_parser::{parse_csv_string, ProgramDefaults};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

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
                    end_x: program.end_x,
                    end_y: program.end_y,
                    end_z: program.end_z,
                    move_speed: program.move_speed,
                    created_at: program.created_at,
                    updated_at: program.updated_at,
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
///
/// CSV contains generic waypoints (X, Y, Z, optional W, P, R, speed, term_type).
/// Robot-specific configuration (UFrame, UTool, arm config) is NOT stored in the program -
/// it is applied at execution time from the active robot configuration.
pub async fn upload_csv(
    db: Arc<Mutex<Database>>,
    program_id: i64,
    csv_content: &str,
    start_position: Option<StartPosition>,
) -> ServerResponse {
    let db = db.lock().await;

    // Use sensible defaults for CSV parsing
    // Robot-specific config (uframe, utool, arm config) is NULL in stored instructions
    // and will be applied from active configuration at execution time
    let defaults = ProgramDefaults {
        w: 0.0,           // Default rotation if not specified in CSV
        p: 0.0,
        r: 0.0,
        ext1: 0.0,
        ext2: 0.0,
        ext3: 0.0,
        speed: 100.0,     // Default speed if not specified in CSV
        speed_type: "mmSec".to_string(),  // Default speed type
        term_type: "FINE".to_string(),  // Safe default
        uframe: None,     // NULL - use active configuration at execution time
        utool: None,      // NULL - use active configuration at execution time
        // Arm configuration is NEVER stored, always from active config at execution time
        front: None,
        up: None,
        left: None,
        flip: None,
        turn4: None,
        turn5: None,
        turn6: None,
    };

    // Parse CSV with full validation
    let parse_result = match parse_csv_string(csv_content, &defaults) {
        Ok(result) => result,
        Err(e) => return ServerResponse::Error {
            message: format!("Failed to parse CSV: {}", e)
        }
    };

    let instructions = parse_result.instructions;

    // Log any warnings
    for warning in &parse_result.warnings {
        warn!("CSV parse warning: {}", warning);
    }

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
            speed_type: instr.speed_type.clone(),
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

    // Auto-populate start position from first instruction if not provided
    let (start_x, start_y, start_z) = if let Some(start) = start_position {
        (Some(start.x), Some(start.y), Some(start.z))
    } else if let Some(first) = instructions.first() {
        (Some(first.x), Some(first.y), Some(first.z))
    } else {
        (None, None, None)
    };

    // Auto-populate end position from last instruction
    let (end_x, end_y, end_z) = if let Some(last) = instructions.last() {
        (Some(last.x), Some(last.y), Some(last.z))
    } else {
        (None, None, None)
    };

    // Get the current program to preserve move_speed if already set
    let current_move_speed = db.get_program(program_id)
        .ok()
        .flatten()
        .and_then(|p| p.move_speed)
        .or(Some(100.0));

    // Update program with positions
    // Robot-specific config (uframe, utool) is NULL - applied at execution time
    if let Ok(Some(prog)) = db.get_program(program_id) {
        let _ = db.update_program(
            program_id,
            &prog.name,
            prog.description.as_deref(),
            defaults.w,
            defaults.p,
            defaults.r,
            Some(defaults.speed),
            &defaults.term_type,
            None,  // uframe - NULL, applied at execution time
            None,  // utool - NULL, applied at execution time
            start_x,
            start_y,
            start_z,
            end_x,
            end_y,
            end_z,
            current_move_speed,
        );
    }

    info!("Uploaded {} instructions to program {}", instructions.len(), program_id);
    ServerResponse::Success {
        message: format!("Uploaded {} instructions", instructions.len())
    }
}

/// Update program settings (start/end positions, move speed).
#[allow(clippy::too_many_arguments)]
pub async fn update_program_settings(
    db: Arc<Mutex<Database>>,
    program_id: i64,
    start_x: Option<f64>,
    start_y: Option<f64>,
    start_z: Option<f64>,
    end_x: Option<f64>,
    end_y: Option<f64>,
    end_z: Option<f64>,
    move_speed: Option<f64>,
) -> ServerResponse {
    let db = db.lock().await;

    // Get the current program to preserve other fields
    let prog = match db.get_program(program_id) {
        Ok(Some(p)) => p,
        Ok(None) => return ServerResponse::Error { message: "Program not found".to_string() },
        Err(e) => return ServerResponse::Error { message: format!("Failed to get program: {}", e) },
    };

    // Update program with new position settings
    if let Err(e) = db.update_program(
        program_id,
        &prog.name,
        prog.description.as_deref(),
        prog.default_w,
        prog.default_p,
        prog.default_r,
        prog.default_speed,
        &prog.default_term_type,
        prog.default_uframe,
        prog.default_utool,
        start_x,
        start_y,
        start_z,
        end_x,
        end_y,
        end_z,
        move_speed,
    ) {
        return ServerResponse::Error { message: format!("Failed to update program: {}", e) };
    }

    info!("Updated program {} settings: start=({:?},{:?},{:?}), end=({:?},{:?},{:?}), speed={:?}",
          program_id, start_x, start_y, start_z, end_x, end_y, end_z, move_speed);

    ServerResponse::Success {
        message: "Program settings updated".to_string()
    }
}
