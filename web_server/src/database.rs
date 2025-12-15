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

/// I/O display configuration for a robot (legacy - use IoPortConfig instead).
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

/// Extended I/O port configuration for HMI panels.
#[derive(Debug, Clone)]
pub struct DbIoPortConfig {
    pub id: i64,
    pub robot_connection_id: i64,
    pub io_type: String,
    pub io_index: i32,
    pub display_name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub widget_type: String,
    pub color_on: String,
    pub color_off: String,
    pub icon: Option<String>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub unit: Option<String>,
    pub decimal_places: i32,
    pub warning_low: Option<f64>,
    pub warning_high: Option<f64>,
    pub alarm_low: Option<f64>,
    pub alarm_high: Option<f64>,
    pub warning_enabled: bool,
    pub alarm_enabled: bool,
    pub hmi_enabled: bool,
    pub hmi_x: Option<i32>,
    pub hmi_y: Option<i32>,
    pub hmi_width: i32,
    pub hmi_height: i32,
    pub hmi_panel_id: Option<i64>,
    pub is_visible: bool,
    pub display_order: Option<i32>,
}

/// HMI Panel configuration stored in database.
#[derive(Debug, Clone)]
pub struct DbHmiPanel {
    pub id: i64,
    pub robot_connection_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub grid_columns: i32,
    pub grid_rows: i32,
    pub background_color: String,
    pub is_default: bool,
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
                u_frame_number INTEGER NOT NULL DEFAULT 0,
                u_tool_number INTEGER NOT NULL DEFAULT 0,
                front INTEGER NOT NULL DEFAULT 0,
                up INTEGER NOT NULL DEFAULT 0,
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

        // Migration: Create HMI panels table
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS hmi_panels (
                id INTEGER PRIMARY KEY,
                robot_connection_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                grid_columns INTEGER NOT NULL DEFAULT 8,
                grid_rows INTEGER NOT NULL DEFAULT 6,
                background_color TEXT NOT NULL DEFAULT '#1a1a1a',
                is_default INTEGER NOT NULL DEFAULT 0,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (robot_connection_id) REFERENCES robot_connections(id) ON DELETE CASCADE
            );"
        )?;

        // Migration: Create io_port_config table (extended I/O configuration)
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS io_port_config (
                id INTEGER PRIMARY KEY,
                robot_connection_id INTEGER NOT NULL,
                io_type TEXT NOT NULL,      -- 'DIN', 'DOUT', 'AIN', 'AOUT', 'GIN', 'GOUT'
                io_index INTEGER NOT NULL,

                -- Identity & Description
                display_name TEXT NOT NULL DEFAULT '',
                description TEXT,
                category TEXT,

                -- Display Configuration
                widget_type TEXT NOT NULL DEFAULT 'auto',
                color_on TEXT NOT NULL DEFAULT '#00ff88',
                color_off TEXT NOT NULL DEFAULT '#333333',
                icon TEXT,

                -- Value Constraints (for analog/group)
                min_value REAL,
                max_value REAL,
                unit TEXT,
                decimal_places INTEGER NOT NULL DEFAULT 2,

                -- Warning/Alarm Thresholds
                warning_low REAL,
                warning_high REAL,
                alarm_low REAL,
                alarm_high REAL,
                warning_enabled INTEGER NOT NULL DEFAULT 0,
                alarm_enabled INTEGER NOT NULL DEFAULT 0,

                -- HMI Panel Layout
                hmi_enabled INTEGER NOT NULL DEFAULT 0,
                hmi_x INTEGER,
                hmi_y INTEGER,
                hmi_width INTEGER NOT NULL DEFAULT 1,
                hmi_height INTEGER NOT NULL DEFAULT 1,
                hmi_panel_id INTEGER,

                -- Standard I/O View
                is_visible INTEGER NOT NULL DEFAULT 1,
                display_order INTEGER,

                FOREIGN KEY (robot_connection_id) REFERENCES robot_connections(id) ON DELETE CASCADE,
                FOREIGN KEY (hmi_panel_id) REFERENCES hmi_panels(id) ON DELETE SET NULL,
                UNIQUE(robot_connection_id, io_type, io_index)
            );"
        )?;

        // Migration: Copy existing io_display_config data to io_port_config if table has data
        let has_old_data: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM io_display_config LIMIT 1)",
            [],
            |row| row.get(0)
        ).unwrap_or(false);

        if has_old_data {
            let new_table_empty: bool = self.conn.query_row(
                "SELECT NOT EXISTS(SELECT 1 FROM io_port_config LIMIT 1)",
                [],
                |row| row.get(0)
            ).unwrap_or(true);

            if new_table_empty {
                self.conn.execute_batch(
                    "INSERT INTO io_port_config (
                        robot_connection_id, io_type, io_index, display_name, is_visible, display_order
                    )
                    SELECT
                        robot_connection_id, io_type, io_index,
                        COALESCE(display_name, ''), is_visible, display_order
                    FROM io_display_config;"
                )?;
                tracing::info!("Migration: Copied io_display_config data to io_port_config");
            }
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
                default_uframe INTEGER DEFAULT 0,
                default_utool INTEGER DEFAULT 0
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
                    default_speed, default_term_type, default_uframe, default_utool,
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
                default_uframe: row.get(8)?,
                default_utool: row.get(9)?,
                start_x: row.get(10)?,
                start_y: row.get(11)?,
                start_z: row.get(12)?,
                end_x: row.get(13)?,
                end_y: row.get(14)?,
                end_z: row.get(15)?,
                move_speed: row.get(16)?,
                created_at: row.get(17)?,
                updated_at: row.get(18)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all programs.
    pub fn list_programs(&self) -> Result<Vec<Program>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, default_w, default_p, default_r,
                    default_speed, default_term_type, default_uframe, default_utool,
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
                default_uframe: row.get(8)?,
                default_utool: row.get(9)?,
                start_x: row.get(10)?,
                start_y: row.get(11)?,
                start_z: row.get(12)?,
                end_x: row.get(13)?,
                end_y: row.get(14)?,
                end_z: row.get(15)?,
                move_speed: row.get(16)?,
                created_at: row.get(17)?,
                updated_at: row.get(18)?,
            })
        })?;

        rows.collect()
    }

    /// Update program metadata.
    #[allow(clippy::too_many_arguments)]
    pub fn update_program(&self, id: i64, name: &str, description: Option<&str>,
                          default_w: f64, default_p: f64, default_r: f64,
                          default_speed: Option<f64>, default_term_type: &str,
                          default_uframe: Option<i32>, default_utool: Option<i32>,
                          start_x: Option<f64>, start_y: Option<f64>, start_z: Option<f64>,
                          end_x: Option<f64>, end_y: Option<f64>, end_z: Option<f64>,
                          move_speed: Option<f64>) -> Result<()> {
        self.conn.execute(
            "UPDATE programs SET
                name = ?1, description = ?2, default_w = ?3, default_p = ?4, default_r = ?5,
                default_speed = ?6, default_term_type = ?7, default_uframe = ?8, default_utool = ?9,
                start_x = ?10, start_y = ?11, start_z = ?12, end_x = ?13, end_y = ?14, end_z = ?15,
                move_speed = ?16, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?17",
            params![name, description, default_w, default_p, default_r,
                    default_speed, default_term_type, default_uframe, default_utool,
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
                (program_id, line_number, x, y, z, w, p, r, ext1, ext2, ext3, speed, speed_type, term_type, uframe, utool)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                program_id, instruction.line_number,
                instruction.x, instruction.y, instruction.z,
                instruction.w, instruction.p, instruction.r,
                instruction.ext1, instruction.ext2, instruction.ext3,
                instruction.speed, instruction.speed_type, instruction.term_type,
                instruction.uframe, instruction.utool
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get all instructions for a program, ordered by line number.
    pub fn get_instructions(&self, program_id: i64) -> Result<Vec<ProgramInstruction>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, program_id, line_number, x, y, z, w, p, r, ext1, ext2, ext3, speed, speed_type, term_type, uframe, utool
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
                uframe: row.get(15)?,
                utool: row.get(16)?,
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

    // ========== Extended I/O Port Config Operations ==========

    /// Get all I/O port configs for a robot.
    pub fn get_io_port_configs(&self, robot_connection_id: i64) -> Result<Vec<DbIoPortConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, robot_connection_id, io_type, io_index, display_name, description, category,
                    widget_type, color_on, color_off, icon, min_value, max_value, unit, decimal_places,
                    warning_low, warning_high, alarm_low, alarm_high, warning_enabled, alarm_enabled,
                    hmi_enabled, hmi_x, hmi_y, hmi_width, hmi_height, hmi_panel_id, is_visible, display_order
             FROM io_port_config WHERE robot_connection_id = ?1 ORDER BY io_type, display_order, io_index"
        )?;

        let rows = stmt.query_map(params![robot_connection_id], |row| {
            Ok(DbIoPortConfig {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                io_type: row.get(2)?,
                io_index: row.get(3)?,
                display_name: row.get(4)?,
                description: row.get(5)?,
                category: row.get(6)?,
                widget_type: row.get(7)?,
                color_on: row.get(8)?,
                color_off: row.get(9)?,
                icon: row.get(10)?,
                min_value: row.get(11)?,
                max_value: row.get(12)?,
                unit: row.get(13)?,
                decimal_places: row.get(14)?,
                warning_low: row.get(15)?,
                warning_high: row.get(16)?,
                alarm_low: row.get(17)?,
                alarm_high: row.get(18)?,
                warning_enabled: row.get::<_, i64>(19)? != 0,
                alarm_enabled: row.get::<_, i64>(20)? != 0,
                hmi_enabled: row.get::<_, i64>(21)? != 0,
                hmi_x: row.get(22)?,
                hmi_y: row.get(23)?,
                hmi_width: row.get(24)?,
                hmi_height: row.get(25)?,
                hmi_panel_id: row.get(26)?,
                is_visible: row.get::<_, i64>(27)? != 0,
                display_order: row.get(28)?,
            })
        })?;

        rows.collect()
    }

    /// Upsert an I/O port config.
    pub fn upsert_io_port_config(&self, robot_connection_id: i64, config: &DbIoPortConfig) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO io_port_config (
                robot_connection_id, io_type, io_index, display_name, description, category,
                widget_type, color_on, color_off, icon, min_value, max_value, unit, decimal_places,
                warning_low, warning_high, alarm_low, alarm_high, warning_enabled, alarm_enabled,
                hmi_enabled, hmi_x, hmi_y, hmi_width, hmi_height, hmi_panel_id, is_visible, display_order
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28)
            ON CONFLICT(robot_connection_id, io_type, io_index) DO UPDATE SET
                display_name = excluded.display_name, description = excluded.description, category = excluded.category,
                widget_type = excluded.widget_type, color_on = excluded.color_on, color_off = excluded.color_off,
                icon = excluded.icon, min_value = excluded.min_value, max_value = excluded.max_value,
                unit = excluded.unit, decimal_places = excluded.decimal_places,
                warning_low = excluded.warning_low, warning_high = excluded.warning_high,
                alarm_low = excluded.alarm_low, alarm_high = excluded.alarm_high,
                warning_enabled = excluded.warning_enabled, alarm_enabled = excluded.alarm_enabled,
                hmi_enabled = excluded.hmi_enabled, hmi_x = excluded.hmi_x, hmi_y = excluded.hmi_y,
                hmi_width = excluded.hmi_width, hmi_height = excluded.hmi_height, hmi_panel_id = excluded.hmi_panel_id,
                is_visible = excluded.is_visible, display_order = excluded.display_order",
            params![
                robot_connection_id, config.io_type, config.io_index, config.display_name,
                config.description, config.category, config.widget_type, config.color_on, config.color_off,
                config.icon, config.min_value, config.max_value, config.unit, config.decimal_places,
                config.warning_low, config.warning_high, config.alarm_low, config.alarm_high,
                config.warning_enabled as i64, config.alarm_enabled as i64,
                config.hmi_enabled as i64, config.hmi_x, config.hmi_y, config.hmi_width, config.hmi_height,
                config.hmi_panel_id, config.is_visible as i64, config.display_order
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Delete an I/O port config.
    pub fn delete_io_port_config(&self, robot_connection_id: i64, io_type: &str, io_index: i32) -> Result<bool> {
        let count = self.conn.execute(
            "DELETE FROM io_port_config WHERE robot_connection_id = ?1 AND io_type = ?2 AND io_index = ?3",
            params![robot_connection_id, io_type, io_index],
        )?;
        Ok(count > 0)
    }

    // ========== HMI Panel Operations ==========

    /// Get all HMI panels for a robot.
    pub fn get_hmi_panels(&self, robot_connection_id: i64) -> Result<Vec<DbHmiPanel>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, robot_connection_id, name, description, grid_columns, grid_rows, background_color, is_default
             FROM hmi_panels WHERE robot_connection_id = ?1 ORDER BY name"
        )?;

        let rows = stmt.query_map(params![robot_connection_id], |row| {
            Ok(DbHmiPanel {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                grid_columns: row.get(4)?,
                grid_rows: row.get(5)?,
                background_color: row.get(6)?,
                is_default: row.get::<_, i64>(7)? != 0,
            })
        })?;

        rows.collect()
    }

    /// Get a single HMI panel by ID.
    pub fn get_hmi_panel_by_id(&self, panel_id: i64) -> Result<Option<DbHmiPanel>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, robot_connection_id, name, description, grid_columns, grid_rows, background_color, is_default
             FROM hmi_panels WHERE id = ?1"
        )?;

        let mut rows = stmt.query_map(params![panel_id], |row| {
            Ok(DbHmiPanel {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                grid_columns: row.get(4)?,
                grid_rows: row.get(5)?,
                background_color: row.get(6)?,
                is_default: row.get::<_, i64>(7)? != 0,
            })
        })?;

        match rows.next() {
            Some(Ok(panel)) => Ok(Some(panel)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /// Create or update an HMI panel.
    pub fn upsert_hmi_panel(&self, panel: &DbHmiPanel) -> Result<i64> {
        if panel.id > 0 {
            // Update existing
            self.conn.execute(
                "UPDATE hmi_panels SET name = ?1, description = ?2, grid_columns = ?3, grid_rows = ?4,
                 background_color = ?5, is_default = ?6 WHERE id = ?7",
                params![panel.name, panel.description, panel.grid_columns, panel.grid_rows,
                        panel.background_color, panel.is_default as i64, panel.id],
            )?;
            Ok(panel.id)
        } else {
            // Insert new
            self.conn.execute(
                "INSERT INTO hmi_panels (robot_connection_id, name, description, grid_columns, grid_rows, background_color, is_default)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![panel.robot_connection_id, panel.name, panel.description, panel.grid_columns,
                        panel.grid_rows, panel.background_color, panel.is_default as i64],
            )?;
            Ok(self.conn.last_insert_rowid())
        }
    }

    /// Delete an HMI panel.
    pub fn delete_hmi_panel(&self, panel_id: i64) -> Result<bool> {
        let count = self.conn.execute("DELETE FROM hmi_panels WHERE id = ?1", params![panel_id])?;
        Ok(count > 0)
    }

    /// Get I/O port configs for a specific HMI panel.
    pub fn get_io_port_configs_for_panel(&self, hmi_panel_id: i64) -> Result<Vec<DbIoPortConfig>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, robot_connection_id, io_type, io_index, display_name, description, category,
                    widget_type, color_on, color_off, icon, min_value, max_value, unit, decimal_places,
                    warning_low, warning_high, alarm_low, alarm_high, warning_enabled, alarm_enabled,
                    hmi_enabled, hmi_x, hmi_y, hmi_width, hmi_height, hmi_panel_id, is_visible, display_order
             FROM io_port_config WHERE hmi_panel_id = ?1 AND hmi_enabled = 1 ORDER BY hmi_y, hmi_x"
        )?;

        let rows = stmt.query_map(params![hmi_panel_id], |row| {
            Ok(DbIoPortConfig {
                id: row.get(0)?,
                robot_connection_id: row.get(1)?,
                io_type: row.get(2)?,
                io_index: row.get(3)?,
                display_name: row.get(4)?,
                description: row.get(5)?,
                category: row.get(6)?,
                widget_type: row.get(7)?,
                color_on: row.get(8)?,
                color_off: row.get(9)?,
                icon: row.get(10)?,
                min_value: row.get(11)?,
                max_value: row.get(12)?,
                unit: row.get(13)?,
                decimal_places: row.get(14)?,
                warning_low: row.get(15)?,
                warning_high: row.get(16)?,
                alarm_low: row.get(17)?,
                alarm_high: row.get(18)?,
                warning_enabled: row.get::<_, i64>(19)? != 0,
                alarm_enabled: row.get::<_, i64>(20)? != 0,
                hmi_enabled: row.get::<_, i64>(21)? != 0,
                hmi_x: row.get(22)?,
                hmi_y: row.get(23)?,
                hmi_width: row.get(24)?,
                hmi_height: row.get(25)?,
                hmi_panel_id: row.get(26)?,
                is_visible: row.get::<_, i64>(27)? != 0,
                display_order: row.get(28)?,
            })
        })?;

        rows.collect()
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

