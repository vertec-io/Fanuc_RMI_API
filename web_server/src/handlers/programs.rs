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
                    term_value: i.term_value,
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
                    default_term_type: program.default_term_type,
                    default_term_value: program.default_term_value,
                    start_x: program.start_x,
                    start_y: program.start_y,
                    start_z: program.start_z,
                    start_w: program.start_w,
                    start_p: program.start_p,
                    start_r: program.start_r,
                    end_x: program.end_x,
                    end_y: program.end_y,
                    end_z: program.end_z,
                    end_w: program.end_w,
                    end_p: program.end_p,
                    end_r: program.end_r,
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
        term_type: "CNT".to_string(),  // Default to CNT for smooth motion
        term_value: Some(100),  // Default to 100 for maximum smoothness
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
            term_value: instr.term_value,
            uframe: instr.uframe,
            utool: instr.utool,
        };
        if let Err(e) = db.add_instruction(program_id, &db_instr) {
            return ServerResponse::Error {
                message: format!("Failed to add instruction: {}", e)
            };
        }
    }

    // Auto-populate start position (X, Y, Z, W, P, R) from first instruction if not provided
    let (start_x, start_y, start_z, start_w, start_p, start_r) = if let Some(start) = start_position {
        (Some(start.x), Some(start.y), Some(start.z), None, None, None)
    } else if let Some(first) = instructions.first() {
        (Some(first.x), Some(first.y), Some(first.z), first.w, first.p, first.r)
    } else {
        (None, None, None, None, None, None)
    };

    // Auto-populate end position (X, Y, Z, W, P, R) from last instruction
    let (end_x, end_y, end_z, end_w, end_p, end_r) = if let Some(last) = instructions.last() {
        (Some(last.x), Some(last.y), Some(last.z), last.w, last.p, last.r)
    } else {
        (None, None, None, None, None, None)
    };

    // Extract default orientation (W, P, R) from first instruction if available
    // This is used as program defaults and for any instruction missing orientation
    let (default_w, default_p, default_r) = if let Some(first) = instructions.first() {
        (
            first.w.unwrap_or(defaults.w),
            first.p.unwrap_or(defaults.p),
            first.r.unwrap_or(defaults.r),
        )
    } else {
        (defaults.w, defaults.p, defaults.r)
    };

    // Get the current program to preserve move_speed if already set
    let current_move_speed = db.get_program(program_id)
        .ok()
        .flatten()
        .and_then(|p| p.move_speed)
        .or(Some(100.0));

    // Update program with positions and orientation from first/last instructions
    // Robot-specific config (uframe, utool) is NULL - applied at execution time
    if let Ok(Some(prog)) = db.get_program(program_id) {
        let _ = db.update_program(
            program_id,
            &prog.name,
            prog.description.as_deref(),
            default_w,
            default_p,
            default_r,
            Some(defaults.speed),
            &defaults.term_type,
            defaults.term_value,  // Default term_value for CNT blending
            None,  // uframe - NULL, applied at execution time
            None,  // utool - NULL, applied at execution time
            start_x,
            start_y,
            start_z,
            start_w,
            start_p,
            start_r,
            end_x,
            end_y,
            end_z,
            end_w,
            end_p,
            end_r,
            current_move_speed,
        );
    }

    info!("Uploaded {} instructions to program {}", instructions.len(), program_id);
    ServerResponse::Success {
        message: format!("Uploaded {} instructions", instructions.len())
    }
}

/// Update program settings (start/end positions with orientation, move speed, termination defaults).
#[allow(clippy::too_many_arguments)]
pub async fn update_program_settings(
    db: Arc<Mutex<Database>>,
    program_id: i64,
    start_x: Option<f64>,
    start_y: Option<f64>,
    start_z: Option<f64>,
    start_w: Option<f64>,
    start_p: Option<f64>,
    start_r: Option<f64>,
    end_x: Option<f64>,
    end_y: Option<f64>,
    end_z: Option<f64>,
    end_w: Option<f64>,
    end_p: Option<f64>,
    end_r: Option<f64>,
    move_speed: Option<f64>,
    default_term_type: Option<String>,
    default_term_value: Option<u8>,
) -> ServerResponse {
    let db = db.lock().await;

    // Get the current program to preserve other fields
    let prog = match db.get_program(program_id) {
        Ok(Some(p)) => p,
        Ok(None) => return ServerResponse::Error { message: "Program not found".to_string() },
        Err(e) => return ServerResponse::Error { message: format!("Failed to get program: {}", e) },
    };

    // Use new values if provided, otherwise preserve existing
    let term_type = default_term_type.as_deref().unwrap_or(&prog.default_term_type);
    let term_value = default_term_value.or(prog.default_term_value);

    // Update program with new settings
    if let Err(e) = db.update_program(
        program_id,
        &prog.name,
        prog.description.as_deref(),
        prog.default_w,
        prog.default_p,
        prog.default_r,
        prog.default_speed,
        term_type,
        term_value,
        prog.default_uframe,
        prog.default_utool,
        start_x,
        start_y,
        start_z,
        start_w,
        start_p,
        start_r,
        end_x,
        end_y,
        end_z,
        end_w,
        end_p,
        end_r,
        move_speed,
    ) {
        return ServerResponse::Error { message: format!("Failed to update program: {}", e) };
    }

    info!("Updated program {} settings: start=({:?},{:?},{:?},{:?},{:?},{:?}), end=({:?},{:?},{:?},{:?},{:?},{:?}), speed={:?}, term_type={}, term_value={:?}",
          program_id, start_x, start_y, start_z, start_w, start_p, start_r, end_x, end_y, end_z, end_w, end_p, end_r, move_speed, term_type, term_value);

    ServerResponse::Success {
        message: "Program settings updated".to_string()
    }
}
