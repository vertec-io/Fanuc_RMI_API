//! SQLite database for storing programs and settings.
//!
//! Database location: `./data/fanuc_rmi.db` (relative to executable)
//! The directory is created automatically if it doesn't exist.

use rusqlite::{Connection, Result, params};
use std::path::Path;
use std::fs;

/// Database wrapper for program and settings storage.
pub struct Database {
    conn: Connection,
}

/// A stored program with metadata and default values.
#[derive(Debug, Clone)]
pub struct Program {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    pub default_speed: Option<f64>,
    pub default_term_type: String,
    /// Default term_value for CNT moves (0-100). 100 = maximum smoothness.
    /// If not specified, defaults to 100 for CNT, 0 for FINE.
    pub default_term_value: Option<u8>,
    pub default_uframe: Option<i32>,
    pub default_utool: Option<i32>,
    // Start position (where robot moves before toolpath)
    pub start_x: Option<f64>,
    pub start_y: Option<f64>,
    pub start_z: Option<f64>,
    // End position (where robot moves after toolpath)
    pub end_x: Option<f64>,
    pub end_y: Option<f64>,
    pub end_z: Option<f64>,
    // Speed for moving to start/end positions
    pub move_speed: Option<f64>,
    pub created_at: String,
    pub updated_at: String,
}

/// A single instruction in a program.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProgramInstruction {
    pub id: i64,
    pub program_id: i64,
    pub line_number: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: Option<f64>,
    pub p: Option<f64>,
    pub r: Option<f64>,
    pub ext1: Option<f64>,
    pub ext2: Option<f64>,
    pub ext3: Option<f64>,
    pub speed: Option<f64>,
    pub speed_type: Option<String>,  // mmSec, InchMin, Time, mSec
    pub term_type: Option<String>,
    /// Term value for CNT blending (0-100). 100 = maximum smoothness.
    /// If None, uses program default_term_value.
    pub term_value: Option<u8>,
    pub uframe: Option<i32>,
    pub utool: Option<i32>,
}

/// Robot default settings (per-robot configuration).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RobotSettings {
    pub id: i64,
    pub name: String,
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    pub default_speed: f64,
    pub default_term_type: String,
    pub default_uframe: i32,
    pub default_utool: i32,
}

/// A saved robot connection configuration.
/// Motion defaults (speed, term_type, w/p/r) and jog defaults are stored here.
/// Frame/tool/arm configuration is stored in robot_configurations table.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RobotConnection {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub ip_address: String,
    pub port: u32,
    // Motion defaults (required - no global fallback)
    pub default_speed: f64,
    pub default_speed_type: String,  // mmSec, InchMin, Time, mSec
    pub default_term_type: String,
    pub default_w: f64,
    pub default_p: f64,
    pub default_r: f64,
    // Jog defaults
    pub default_cartesian_jog_speed: f64,
    pub default_cartesian_jog_step: f64,
    pub default_joint_jog_speed: f64,
    pub default_joint_jog_step: f64,
    pub created_at: String,
    pub updated_at: String,
}

/// A named robot configuration (UFrame, UTool, arm posture).
/// Multiple configurations can be saved per robot, with one marked as default.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RobotConfiguration {
    pub id: i64,
    pub robot_connection_id: i64,
    pub name: String,
    pub is_default: bool,
    // Frame and tool
    pub u_frame_number: i32,
    pub u_tool_number: i32,
    // Arm configuration
    pub front: i32,
    pub up: i32,
    pub left: i32,
    pub flip: i32,
    pub turn4: i32,
    pub turn5: i32,
    pub turn6: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// I/O display configuration for a robot.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct IoDisplayConfig {
    pub id: i64,
    pub robot_connection_id: i64,
    pub io_type: String,  // 'DIN', 'DOUT', 'AIN', 'AOUT', 'GIN', 'GOUT'
    pub io_index: i32,
    pub display_name: Option<String>,
    pub is_visible: bool,
    pub display_order: Option<i32>,
}

/// Server setting key-value pair.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ServerSetting {
    pub id: i64,
    pub key: String,
    pub value: Option<String>,
    pub description: Option<String>,
}

impl Database {
    /// Default database path.
    pub const DEFAULT_PATH: &'static str = "./data/fanuc_rmi.db";

    /// Create or open the database at the given path.
    pub fn new(path: &str) -> Result<Self> {
        // Create data directory if it doesn't exist
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(|e| {
                rusqlite::Error::InvalidPath(format!("Failed to create directory: {}", e).into())
            })?;
        }

        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.initialize_schema()?;
        db.run_migrations()?;
        Ok(db)
    }

    /// Run database migrations to add columns that may be missing from older schemas.
    fn run_migrations(&self) -> Result<()> {
        // Migration: Add new columns to robot_connections if they don't exist
        // Note: Frame/tool/arm config moved to robot_configurations table
        let columns_to_add = [
            ("default_speed", "REAL"),
            ("default_speed_type", "TEXT"),  // mmSec, InchMin, Time, mSec
            ("default_term_type", "TEXT"),
            ("default_w", "REAL"),
            ("default_p", "REAL"),
            ("default_r", "REAL"),
            // Jog defaults
            ("default_cartesian_jog_speed", "REAL"),
            ("default_cartesian_jog_step", "REAL"),
            ("default_joint_jog_speed", "REAL"),
            ("default_joint_jog_step", "REAL"),
        ];

        for (column_name, column_type) in columns_to_add {
            // Check if column exists by trying to select it
            let column_exists = self
                .conn
                .prepare(&format!(
                    "SELECT {} FROM robot_connections LIMIT 1",
                    column_name
                ))
                .is_ok();

            if !column_exists {
                // Add the column
                self.conn.execute(
                    &format!(
                        "ALTER TABLE robot_connections ADD COLUMN {} {}",
                        column_name, column_type
                    ),
                    [],
                )?;
                tracing::info!("Migration: Added column {} to robot_connections", column_name);
            }
        }

        // Migration: Create robot_configurations table if it doesn't exist
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS robot_configurations (
                id INTEGER PRIMARY KEY,
                robot_connection_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0,
                -- FANUC uses 1-based indexing for frames and tools
                u_frame_number INTEGER NOT NULL DEFAULT 1,
                u_tool_number INTEGER NOT NULL DEFAULT 1,
                -- Arm configuration defaults
                front INTEGER NOT NULL DEFAULT 1,
                up INTEGER NOT NULL DEFAULT 1,
                left INTEGER NOT NULL DEFAULT 0,
                flip INTEGER NOT NULL DEFAULT 0,
                turn4 INTEGER NOT NULL DEFAULT 0,
                turn5 INTEGER NOT NULL DEFAULT 0,
                turn6 INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (robot_connection_id) REFERENCES robot_connections(id) ON DELETE CASCADE,
                UNIQUE(robot_connection_id, name)
            );"
        )?;

        // Migration: Add new columns to programs table if they don't exist
        let program_columns_to_add = [
            ("end_x", "REAL"),
            ("end_y", "REAL"),
            ("end_z", "REAL"),
            ("move_speed", "REAL DEFAULT 100.0"),
        ];

        for (column_name, column_type) in program_columns_to_add {
            let column_exists = self
                .conn
                .prepare(&format!(
                    "SELECT {} FROM programs LIMIT 1",
                    column_name
                ))
                .is_ok();

            if !column_exists {
                self.conn.execute(
                    &format!(
                        "ALTER TABLE programs ADD COLUMN {} {}",
                        column_name, column_type
                    ),
                    [],
                )?;
                tracing::info!("Migration: Added column {} to programs", column_name);
            }
        }

        // Migration: Add speed_type column to program_instructions table if it doesn't exist
        let column_exists = self
            .conn
            .prepare("SELECT speed_type FROM program_instructions LIMIT 1")
            .is_ok();

        if !column_exists {
            self.conn.execute(
                "ALTER TABLE program_instructions ADD COLUMN speed_type TEXT",
                [],
            )?;
            tracing::info!("Migration: Added column speed_type to program_instructions");
        }

        // Migration: Add term_value column to program_instructions if it doesn't exist
        let column_exists = self
            .conn
            .prepare("SELECT term_value FROM program_instructions LIMIT 1")
            .is_ok();

        if !column_exists {
            self.conn.execute(
                "ALTER TABLE program_instructions ADD COLUMN term_value INTEGER",
                [],
            )?;
            tracing::info!("Migration: Added column term_value to program_instructions");
        }

        // Migration: Add default_term_value column to programs if it doesn't exist
        let column_exists = self
            .conn
            .prepare("SELECT default_term_value FROM programs LIMIT 1")
            .is_ok();

        if !column_exists {
            self.conn.execute(
                "ALTER TABLE programs ADD COLUMN default_term_value INTEGER DEFAULT 100",
                [],
            )?;
            tracing::info!("Migration: Added column default_term_value to programs");
        }

        Ok(())
    }

    /// Initialize database schema.
    fn initialize_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS programs (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                default_w REAL DEFAULT 0.0,
                default_p REAL DEFAULT 0.0,
                default_r REAL DEFAULT 0.0,
                default_speed REAL,
                default_term_type TEXT DEFAULT 'CNT',
                default_term_value INTEGER DEFAULT 100,
                default_uframe INTEGER,
                default_utool INTEGER,
                start_x REAL,
                start_y REAL,
                start_z REAL,
                end_x REAL,
                end_y REAL,
                end_z REAL,
                move_speed REAL DEFAULT 100.0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS program_instructions (
                id INTEGER PRIMARY KEY,
                program_id INTEGER NOT NULL,
                line_number INTEGER NOT NULL,
                x REAL NOT NULL,
                y REAL NOT NULL,
                z REAL NOT NULL,
                w REAL,
                p REAL,
                r REAL,
                ext1 REAL,
                ext2 REAL,
                ext3 REAL,
                speed REAL,
                term_type TEXT,
                term_value INTEGER,
                uframe INTEGER,
                utool INTEGER,
                FOREIGN KEY (program_id) REFERENCES programs(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS robot_settings (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE DEFAULT 'default',
                default_w REAL DEFAULT 0.0,
                default_p REAL DEFAULT 0.0,
                default_r REAL DEFAULT 0.0,
                default_speed REAL DEFAULT 100.0,
                default_term_type TEXT DEFAULT 'CNT',
                -- FANUC uses 1-based indexing for frames and tools
                default_uframe INTEGER DEFAULT 1,
                default_utool INTEGER DEFAULT 1
            );

            CREATE TABLE IF NOT EXISTS robot_connections (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                ip_address TEXT NOT NULL DEFAULT '127.0.0.1',
                port INTEGER NOT NULL DEFAULT 16001,
                -- Per-robot defaults (override global robot_settings)
                default_speed REAL,
                default_term_type TEXT,
                default_uframe INTEGER,
                default_utool INTEGER,
                default_w REAL,
                default_p REAL,
                default_r REAL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            -- I/O display configuration per robot
            CREATE TABLE IF NOT EXISTS io_display_config (
                id INTEGER PRIMARY KEY,
                robot_connection_id INTEGER NOT NULL,
                io_type TEXT NOT NULL,  -- 'DIN', 'DOUT', 'AIN', 'AOUT', 'GIN', 'GOUT'
                io_index INTEGER NOT NULL,
                display_name TEXT,
                is_visible INTEGER DEFAULT 1,
                display_order INTEGER,
                FOREIGN KEY (robot_connection_id) REFERENCES robot_connections(id) ON DELETE CASCADE,
                UNIQUE(robot_connection_id, io_type, io_index)
            );

            -- Global server settings
            CREATE TABLE IF NOT EXISTS server_settings (
                id INTEGER PRIMARY KEY,
                key TEXT NOT NULL UNIQUE,
                value TEXT,
                description TEXT
            );

            -- Insert default robot settings if not exists
            INSERT OR IGNORE INTO robot_settings (name) VALUES ('default');

            -- Insert default server settings
            INSERT OR IGNORE INTO server_settings (key, value, description) VALUES
                ('theme', 'dark', 'UI theme: dark or light'),
                ('default_robot_id', NULL, 'Default robot connection to use on startup'),
                ('auto_connect', 'false', 'Automatically connect to default robot on startup');"
        )
    }

    /// Reset database - IRREVERSIBLE! Drops all tables and recreates them.
    pub fn reset(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "DROP TABLE IF EXISTS program_instructions;
             DROP TABLE IF EXISTS programs;
             DROP TABLE IF EXISTS robot_settings;
             DROP TABLE IF EXISTS io_display_config;
             DROP TABLE IF EXISTS server_settings;
             DROP TABLE IF EXISTS robot_configurations;
             DROP TABLE IF EXISTS robot_connections;"
        )?;
        self.initialize_schema()?;
        self.run_migrations()
    }

    // ========== Program CRUD Operations ==========

    /// Create a new program.
    pub fn create_program(&self, name: &str, description: Option<&str>) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO programs (name, description) VALUES (?1, ?2)",
            params![name, description],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get a program by ID.
    pub fn get_program(&self, id: i64) -> Result<Option<Program>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, default_w, default_p, default_r,
                    default_speed, default_term_type, default_term_value, default_uframe, default_utool,
                    start_x, start_y, start_z, end_x, end_y, end_z,
                    COALESCE(move_speed, 100.0), created_at, updated_at
             FROM programs WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Program {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                default_w: row.get(3)?,
                default_p: row.get(4)?,
                default_r: row.get(5)?,
                default_speed: row.get(6)?,
                default_term_type: row.get(7)?,
                default_term_value: row.get::<_, Option<i32>>(8)?.map(|v| v as u8),
                default_uframe: row.get(9)?,
                default_utool: row.get(10)?,
                start_x: row.get(11)?,
                start_y: row.get(12)?,
                start_z: row.get(13)?,
                end_x: row.get(14)?,
                end_y: row.get(15)?,
                end_z: row.get(16)?,
                move_speed: row.get(17)?,
                created_at: row.get(18)?,
                updated_at: row.get(19)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all programs.
    pub fn list_programs(&self) -> Result<Vec<Program>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, default_w, default_p, default_r,
                    default_speed, default_term_type, default_term_value, default_uframe, default_utool,
                    start_x, start_y, start_z, end_x, end_y, end_z,
                    COALESCE(move_speed, 100.0), created_at, updated_at
             FROM programs ORDER BY name"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(Program {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                default_w: row.get(3)?,
                default_p: row.get(4)?,
                default_r: row.get(5)?,
                default_speed: row.get(6)?,
                default_term_type: row.get(7)?,
                default_term_value: row.get::<_, Option<i32>>(8)?.map(|v| v as u8),
                default_uframe: row.get(9)?,
                default_utool: row.get(10)?,
                start_x: row.get(11)?,
                start_y: row.get(12)?,
                start_z: row.get(13)?,
                end_x: row.get(14)?,
                end_y: row.get(15)?,
                end_z: row.get(16)?,
                move_speed: row.get(17)?,
                created_at: row.get(18)?,
                updated_at: row.get(19)?,
            })
        })?;

        rows.collect()
    }

    /// Update program metadata.
    #[allow(clippy::too_many_arguments)]
    pub fn update_program(&self, id: i64, name: &str, description: Option<&str>,
                          default_w: f64, default_p: f64, default_r: f64,
                          default_speed: Option<f64>, default_term_type: &str,
                          default_term_value: Option<u8>,
                          default_uframe: Option<i32>, default_utool: Option<i32>,
                          start_x: Option<f64>, start_y: Option<f64>, start_z: Option<f64>,
                          end_x: Option<f64>, end_y: Option<f64>, end_z: Option<f64>,
                          move_speed: Option<f64>) -> Result<()> {
        self.conn.execute(
            "UPDATE programs SET
                name = ?1, description = ?2, default_w = ?3, default_p = ?4, default_r = ?5,
                default_speed = ?6, default_term_type = ?7, default_term_value = ?8,
                default_uframe = ?9, default_utool = ?10,
                start_x = ?11, start_y = ?12, start_z = ?13, end_x = ?14, end_y = ?15, end_z = ?16,
                move_speed = ?17, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?18",
            params![name, description, default_w, default_p, default_r,
                    default_speed, default_term_type, default_term_value.map(|v| v as i32),
                    default_uframe, default_utool,
                    start_x, start_y, start_z, end_x, end_y, end_z, move_speed, id],
        )?;
        Ok(())
    }

    /// Delete a program and all its instructions.
    pub fn delete_program(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM program_instructions WHERE program_id = ?1", params![id])?;
        self.conn.execute("DELETE FROM programs WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ========== Program Instructions ==========

    /// Add an instruction to a program.
    pub fn add_instruction(&self, program_id: i64, instruction: &ProgramInstruction) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO program_instructions
                (program_id, line_number, x, y, z, w, p, r, ext1, ext2, ext3, speed, speed_type, term_type, term_value, uframe, utool)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                program_id, instruction.line_number,
                instruction.x, instruction.y, instruction.z,
                instruction.w, instruction.p, instruction.r,
                instruction.ext1, instruction.ext2, instruction.ext3,
                instruction.speed, instruction.speed_type, instruction.term_type,
                instruction.term_value.map(|v| v as i32),
                instruction.uframe, instruction.utool
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get all instructions for a program, ordered by line number.
    pub fn get_instructions(&self, program_id: i64) -> Result<Vec<ProgramInstruction>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, program_id, line_number, x, y, z, w, p, r, ext1, ext2, ext3, speed, speed_type, term_type, term_value, uframe, utool
             FROM program_instructions WHERE program_id = ?1 ORDER BY line_number"
        )?;

        let rows = stmt.query_map(params![program_id], |row| {
            Ok(ProgramInstruction {
                id: row.get(0)?,
                program_id: row.get(1)?,
                line_number: row.get(2)?,
                x: row.get(3)?,
                y: row.get(4)?,
                z: row.get(5)?,
                w: row.get(6)?,
                p: row.get(7)?,
                r: row.get(8)?,
                ext1: row.get(9)?,
                ext2: row.get(10)?,
                ext3: row.get(11)?,
                speed: row.get(12)?,
                speed_type: row.get(13)?,
                term_type: row.get(14)?,
                term_value: row.get::<_, Option<i32>>(15)?.map(|v| v as u8),
                uframe: row.get(16)?,
                utool: row.get(17)?,
            })
        })?;

        rows.collect()
    }

    /// Clear all instructions for a program.
    pub fn clear_instructions(&self, program_id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM program_instructions WHERE program_id = ?1", params![program_id])?;
        Ok(())
    }

    /// Get instruction count for a program.
    pub fn instruction_count(&self, program_id: i64) -> Result<i64> {
        self.conn.query_row(
            "SELECT COUNT(*) FROM program_instructions WHERE program_id = ?1",
            params![program_id],
            |row| row.get(0)
        )
    }

    // ========== Robot Settings ==========

    /// Get robot settings (creates default if not exists).
    pub fn get_robot_settings(&self) -> Result<RobotSettings> {
        self.conn.query_row(
            "SELECT id, name, default_w, default_p, default_r, default_speed, default_term_type, default_uframe, default_utool
             FROM robot_settings WHERE name = 'default'",
            [],
            |row| Ok(RobotSettings {
                id: row.get(0)?,
                name: row.get(1)?,
                default_w: row.get(2)?,
                default_p: row.get(3)?,
                default_r: row.get(4)?,
                default_speed: row.get(5)?,
                default_term_type: row.get(6)?,
                default_uframe: row.get(7)?,
                default_utool: row.get(8)?,
            })
        )
    }

    /// Update robot settings.
    pub fn update_robot_settings(&self, default_w: f64, default_p: f64, default_r: f64,
                                  default_speed: f64, default_term_type: &str,
                                  default_uframe: i32, default_utool: i32) -> Result<()> {
        self.conn.execute(
            "UPDATE robot_settings SET
                default_w = ?1, default_p = ?2, default_r = ?3,
                default_speed = ?4, default_term_type = ?5, default_uframe = ?6, default_utool = ?7
             WHERE name = 'default'",
            params![default_w, default_p, default_r, default_speed, default_term_type, default_uframe, default_utool],
        )?;
        Ok(())
    }

    // ========== Robot Connections CRUD Operations ==========

    /// Create a new robot connection with all defaults.
    #[allow(clippy::too_many_arguments)]
    pub fn create_robot_connection(
        &self,
        name: &str,
        description: Option<&str>,
        ip_address: &str,
        port: u32,
        default_speed: f64,
        default_speed_type: &str,
        default_term_type: &str,
        default_w: f64,
        default_p: f64,
        default_r: f64,
        default_cartesian_jog_speed: f64,
        default_cartesian_jog_step: f64,
        default_joint_jog_speed: f64,
        default_joint_jog_step: f64,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO robot_connections (
                name, description, ip_address, port,
                default_speed, default_speed_type, default_term_type, default_w, default_p, default_r,
                default_cartesian_jog_speed, default_cartesian_jog_step,
                default_joint_jog_speed, default_joint_jog_step
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                name, description, ip_address, port,
                default_speed, default_speed_type, default_term_type, default_w, default_p, default_r,
                default_cartesian_jog_speed, default_cartesian_jog_step,
                default_joint_jog_speed, default_joint_jog_step
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get a robot connection by ID.
    /// Uses COALESCE to provide sensible defaults for NULL values in existing data.
    pub fn get_robot_connection(&self, id: i64) -> Result<Option<RobotConnection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, ip_address, port,
                    COALESCE(default_speed, 100.0),
                    COALESCE(default_speed_type, 'mmSec'),
                    COALESCE(default_term_type, 'CNT'),
                    COALESCE(default_w, 0.0),
                    COALESCE(default_p, 0.0),
                    COALESCE(default_r, 0.0),
                    COALESCE(default_cartesian_jog_speed, 10.0),
                    COALESCE(default_cartesian_jog_step, 1.0),
                    COALESCE(default_joint_jog_speed, 0.1),
                    COALESCE(default_joint_jog_step, 0.25),
                    created_at, updated_at
             FROM robot_connections WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(RobotConnection {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                ip_address: row.get(3)?,
                port: row.get::<_, i64>(4)? as u32,
                default_speed: row.get(5)?,
                default_speed_type: row.get(6)?,
                default_term_type: row.get(7)?,
                default_w: row.get(8)?,
                default_p: row.get(9)?,
                default_r: row.get(10)?,
                default_cartesian_jog_speed: row.get(11)?,
                default_cartesian_jog_step: row.get(12)?,
                default_joint_jog_speed: row.get(13)?,
                default_joint_jog_step: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all robot connections.
    /// Uses COALESCE to provide sensible defaults for NULL values in existing data.
    pub fn list_robot_connections(&self) -> Result<Vec<RobotConnection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, ip_address, port,
                    COALESCE(default_speed, 100.0),
                    COALESCE(default_speed_type, 'mmSec'),
                    COALESCE(default_term_type, 'CNT'),
                    COALESCE(default_w, 0.0),
                    COALESCE(default_p, 0.0),
                    COALESCE(default_r, 0.0),
                    COALESCE(default_cartesian_jog_speed, 10.0),
                    COALESCE(default_cartesian_jog_step, 1.0),
                    COALESCE(default_joint_jog_speed, 0.1),
                    COALESCE(default_joint_jog_step, 0.25),
                    created_at, updated_at
             FROM robot_connections ORDER BY name"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(RobotConnection {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                ip_address: row.get(3)?,
                port: row.get::<_, i64>(4)? as u32,
                default_speed: row.get(5)?,
                default_speed_type: row.get(6)?,
                default_term_type: row.get(7)?,
                default_w: row.get(8)?,
                default_p: row.get(9)?,
                default_r: row.get(10)?,
                default_cartesian_jog_speed: row.get(11)?,
                default_cartesian_jog_step: row.get(12)?,
                default_joint_jog_speed: row.get(13)?,
                default_joint_jog_step: row.get(14)?,
                created_at: row.get(15)?,
                updated_at: row.get(16)?,
            })
        })?;

        rows.collect()
    }

    /// Update a robot connection (basic fields only).
    pub fn update_robot_connection(&self, id: i64, name: &str, description: Option<&str>, ip_address: &str, port: u32) -> Result<()> {
        self.conn.execute(
            "UPDATE robot_connections SET
                name = ?1, description = ?2, ip_address = ?3, port = ?4, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?5",
            params![name, description, ip_address, port, id],
        )?;
        Ok(())
    }

    /// Update robot connection motion defaults.
    /// Motion parameters (speed, speed_type, term_type, w/p/r) only.
    /// Frame/tool/arm config is managed via robot_configurations table.
    pub fn update_robot_connection_defaults(
        &self,
        id: i64,
        default_speed: f64,
        default_speed_type: &str,
        default_term_type: &str,
        default_w: f64,
        default_p: f64,
        default_r: f64,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE robot_connections SET
                default_speed = ?1, default_speed_type = ?2, default_term_type = ?3,
                default_w = ?4, default_p = ?5, default_r = ?6,
                updated_at = CURRENT_TIMESTAMP
             WHERE id = ?7",
            params![default_speed, default_speed_type, default_term_type, default_w, default_p, default_r, id],
        )?;
        Ok(())
    }

    /// Update robot connection jog defaults.
    pub fn update_robot_connection_jog_defaults(
        &self,
        id: i64,
        cartesian_jog_speed: f64,
        cartesian_jog_step: f64,
        joint_jog_speed: f64,
        joint_jog_step: f64,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE robot_connections SET
                default_cartesian_jog_speed = ?1,
                default_cartesian_jog_step = ?2,
                default_joint_jog_speed = ?3,
                default_joint_jog_step = ?4,
                updated_at = CURRENT_TIMESTAMP
             WHERE id = ?5",
            params![cartesian_jog_speed, cartesian_jog_step, joint_jog_speed, joint_jog_step, id],
        )?;
        Ok(())
    }

    /// Delete a robot connection.
    pub fn delete_robot_connection(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM robot_connections WHERE id = ?1", params![id])?;
        Ok(())
    }

    // ========== I/O Display Config Operations ==========

    /// Get I/O display config for a robot.
    pub fn get_io_display_config(&self, robot_connection_id: i64) -> Result<Vec<IoDisplayConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, robot_connection_id, io_type, io_index, display_name, is_visible, display_order
             FROM io_display_config WHERE robot_connection_id = ?1 ORDER BY io_type, display_order, io_index"
        )?;

        let rows = stmt.query_map(params![robot_connection_id], |row| {
            Ok(IoDisplayConfig {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                io_type: row.get(2)?,
                io_index: row.get(3)?,
                display_name: row.get(4)?,
                is_visible: row.get::<_, i64>(5)? != 0,
                display_order: row.get(6)?,
            })
        })?;

        rows.collect()
    }

    /// Upsert I/O display config.
    pub fn upsert_io_display_config(
        &self,
        robot_connection_id: i64,
        io_type: &str,
        io_index: i32,
        display_name: Option<&str>,
        is_visible: bool,
        display_order: Option<i32>,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO io_display_config (robot_connection_id, io_type, io_index, display_name, is_visible, display_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(robot_connection_id, io_type, io_index) DO UPDATE SET
                display_name = excluded.display_name,
                is_visible = excluded.is_visible,
                display_order = excluded.display_order",
            params![robot_connection_id, io_type, io_index, display_name, is_visible as i64, display_order],
        )?;
        Ok(())
    }

    // ========== Server Settings Operations ==========

    /// Get a server setting by key.
    pub fn get_server_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT value FROM server_settings WHERE key = ?1"
        )?;

        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            Ok(None)
        }
    }

    /// Set a server setting.
    pub fn set_server_setting(&self, key: &str, value: Option<&str>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO server_settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get all server settings.
    pub fn get_all_server_settings(&self) -> Result<Vec<ServerSetting>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, key, value, description FROM server_settings ORDER BY key"
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(ServerSetting {
                id: row.get(0)?,
                key: row.get(1)?,
                value: row.get(2)?,
                description: row.get(3)?,
            })
        })?;

        rows.collect()
    }

    // ========== Robot Configuration Operations ==========

    /// Create a new robot configuration.
    /// If is_default is true, clears is_default on all other configs for this robot.
    #[allow(clippy::too_many_arguments)]
    pub fn create_robot_configuration(
        &self,
        robot_connection_id: i64,
        name: &str,
        is_default: bool,
        u_frame_number: i32,
        u_tool_number: i32,
        front: i32,
        up: i32,
        left: i32,
        flip: i32,
        turn4: i32,
        turn5: i32,
        turn6: i32,
    ) -> Result<i64> {
        // If this is the default, clear other defaults first
        if is_default {
            self.conn.execute(
                "UPDATE robot_configurations SET is_default = 0 WHERE robot_connection_id = ?1",
                params![robot_connection_id],
            )?;
        }

        self.conn.execute(
            "INSERT INTO robot_configurations (
                robot_connection_id, name, is_default, u_frame_number, u_tool_number,
                front, up, left, flip, turn4, turn5, turn6
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                robot_connection_id, name, is_default as i32,
                u_frame_number, u_tool_number,
                front, up, left, flip, turn4, turn5, turn6
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get a robot configuration by ID.
    pub fn get_robot_configuration(&self, id: i64) -> Result<Option<RobotConfiguration>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, robot_connection_id, name, is_default, u_frame_number, u_tool_number,
                    front, up, left, flip, turn4, turn5, turn6, created_at, updated_at
             FROM robot_configurations WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(RobotConfiguration {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                name: row.get(2)?,
                is_default: row.get::<_, i32>(3)? != 0,
                u_frame_number: row.get(4)?,
                u_tool_number: row.get(5)?,
                front: row.get(6)?,
                up: row.get(7)?,
                left: row.get(8)?,
                flip: row.get(9)?,
                turn4: row.get(10)?,
                turn5: row.get(11)?,
                turn6: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all configurations for a robot.
    pub fn list_robot_configurations(&self, robot_connection_id: i64) -> Result<Vec<RobotConfiguration>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, robot_connection_id, name, is_default, u_frame_number, u_tool_number,
                    front, up, left, flip, turn4, turn5, turn6, created_at, updated_at
             FROM robot_configurations WHERE robot_connection_id = ?1 ORDER BY is_default DESC, name"
        )?;

        let rows = stmt.query_map(params![robot_connection_id], |row| {
            Ok(RobotConfiguration {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                name: row.get(2)?,
                is_default: row.get::<_, i32>(3)? != 0,
                u_frame_number: row.get(4)?,
                u_tool_number: row.get(5)?,
                front: row.get(6)?,
                up: row.get(7)?,
                left: row.get(8)?,
                flip: row.get(9)?,
                turn4: row.get(10)?,
                turn5: row.get(11)?,
                turn6: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        rows.collect()
    }

    /// Get the default configuration for a robot.
    pub fn get_default_robot_configuration(&self, robot_connection_id: i64) -> Result<Option<RobotConfiguration>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, robot_connection_id, name, is_default, u_frame_number, u_tool_number,
                    front, up, left, flip, turn4, turn5, turn6, created_at, updated_at
             FROM robot_configurations WHERE robot_connection_id = ?1 AND is_default = 1"
        )?;

        let mut rows = stmt.query(params![robot_connection_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(RobotConfiguration {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                name: row.get(2)?,
                is_default: row.get::<_, i32>(3)? != 0,
                u_frame_number: row.get(4)?,
                u_tool_number: row.get(5)?,
                front: row.get(6)?,
                up: row.get(7)?,
                left: row.get(8)?,
                flip: row.get(9)?,
                turn4: row.get(10)?,
                turn5: row.get(11)?,
                turn6: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Update a robot configuration.
    /// If is_default is true, clears is_default on all other configs for this robot.
    #[allow(clippy::too_many_arguments)]
    pub fn update_robot_configuration(
        &self,
        id: i64,
        name: &str,
        is_default: bool,
        u_frame_number: i32,
        u_tool_number: i32,
        front: i32,
        up: i32,
        left: i32,
        flip: i32,
        turn4: i32,
        turn5: i32,
        turn6: i32,
    ) -> Result<()> {
        // If setting as default, clear other defaults first
        if is_default {
            // Get the robot_connection_id for this config
            let robot_connection_id: i64 = self.conn.query_row(
                "SELECT robot_connection_id FROM robot_configurations WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )?;
            self.conn.execute(
                "UPDATE robot_configurations SET is_default = 0 WHERE robot_connection_id = ?1",
                params![robot_connection_id],
            )?;
        }

        self.conn.execute(
            "UPDATE robot_configurations SET
                name = ?1, is_default = ?2, u_frame_number = ?3, u_tool_number = ?4,
                front = ?5, up = ?6, left = ?7, flip = ?8,
                turn4 = ?9, turn5 = ?10, turn6 = ?11, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?12",
            params![
                name, is_default as i32, u_frame_number, u_tool_number,
                front, up, left, flip, turn4, turn5, turn6, id
            ],
        )?;
        Ok(())
    }

    /// Delete a robot configuration.
    pub fn delete_robot_configuration(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM robot_configurations WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Set a configuration as the default for its robot.
    pub fn set_default_robot_configuration(&self, id: i64) -> Result<()> {
        // Get the robot_connection_id for this config
        let robot_connection_id: i64 = self.conn.query_row(
            "SELECT robot_connection_id FROM robot_configurations WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;

        // Clear all defaults for this robot
        self.conn.execute(
            "UPDATE robot_configurations SET is_default = 0 WHERE robot_connection_id = ?1",
            params![robot_connection_id],
        )?;

        // Set this config as default
        self.conn.execute(
            "UPDATE robot_configurations SET is_default = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
            params![id],
        )?;

        Ok(())
    }
}

