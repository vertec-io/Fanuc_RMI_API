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
    pub start_x: Option<f64>,
    pub start_y: Option<f64>,
    pub start_z: Option<f64>,
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
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RobotConnection {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub ip_address: String,
    pub port: u32,
    // Per-robot defaults
    pub default_speed: Option<f64>,
    pub default_term_type: Option<String>,
    pub default_uframe: Option<i32>,
    pub default_utool: Option<i32>,
    pub default_w: Option<f64>,
    pub default_p: Option<f64>,
    pub default_r: Option<f64>,
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
        Ok(db)
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
             DROP TABLE IF EXISTS robot_connections;"
        )?;
        self.initialize_schema()
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
                    start_x, start_y, start_z, created_at, updated_at
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
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
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
                    start_x, start_y, start_z, created_at, updated_at
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
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?;

        rows.collect()
    }

    /// Update program metadata.
    pub fn update_program(&self, id: i64, name: &str, description: Option<&str>,
                          default_w: f64, default_p: f64, default_r: f64,
                          default_speed: Option<f64>, default_term_type: &str,
                          default_uframe: Option<i32>, default_utool: Option<i32>,
                          start_x: Option<f64>, start_y: Option<f64>, start_z: Option<f64>) -> Result<()> {
        self.conn.execute(
            "UPDATE programs SET
                name = ?1, description = ?2, default_w = ?3, default_p = ?4, default_r = ?5,
                default_speed = ?6, default_term_type = ?7, default_uframe = ?8, default_utool = ?9,
                start_x = ?10, start_y = ?11, start_z = ?12, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?13",
            params![name, description, default_w, default_p, default_r,
                    default_speed, default_term_type, default_uframe, default_utool,
                    start_x, start_y, start_z, id],
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
                (program_id, line_number, x, y, z, w, p, r, ext1, ext2, ext3, speed, term_type, uframe, utool)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                program_id, instruction.line_number,
                instruction.x, instruction.y, instruction.z,
                instruction.w, instruction.p, instruction.r,
                instruction.ext1, instruction.ext2, instruction.ext3,
                instruction.speed, instruction.term_type,
                instruction.uframe, instruction.utool
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get all instructions for a program, ordered by line number.
    pub fn get_instructions(&self, program_id: i64) -> Result<Vec<ProgramInstruction>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, program_id, line_number, x, y, z, w, p, r, ext1, ext2, ext3, speed, term_type, uframe, utool
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
                term_type: row.get(13)?,
                uframe: row.get(14)?,
                utool: row.get(15)?,
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

    /// Create a new robot connection.
    pub fn create_robot_connection(&self, name: &str, description: Option<&str>, ip_address: &str, port: u32) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO robot_connections (name, description, ip_address, port) VALUES (?1, ?2, ?3, ?4)",
            params![name, description, ip_address, port],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Get a robot connection by ID.
    pub fn get_robot_connection(&self, id: i64) -> Result<Option<RobotConnection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, ip_address, port,
                    default_speed, default_term_type, default_uframe, default_utool,
                    default_w, default_p, default_r, created_at, updated_at
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
                default_term_type: row.get(6)?,
                default_uframe: row.get(7)?,
                default_utool: row.get(8)?,
                default_w: row.get(9)?,
                default_p: row.get(10)?,
                default_r: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            }))
        } else {
            Ok(None)
        }
    }

    /// List all robot connections.
    pub fn list_robot_connections(&self) -> Result<Vec<RobotConnection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, description, ip_address, port,
                    default_speed, default_term_type, default_uframe, default_utool,
                    default_w, default_p, default_r, created_at, updated_at
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
                default_term_type: row.get(6)?,
                default_uframe: row.get(7)?,
                default_utool: row.get(8)?,
                default_w: row.get(9)?,
                default_p: row.get(10)?,
                default_r: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
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

    /// Update robot connection defaults.
    #[allow(clippy::too_many_arguments)]
    pub fn update_robot_connection_defaults(
        &self,
        id: i64,
        default_speed: Option<f64>,
        default_term_type: Option<&str>,
        default_uframe: Option<i32>,
        default_utool: Option<i32>,
        default_w: Option<f64>,
        default_p: Option<f64>,
        default_r: Option<f64>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE robot_connections SET
                default_speed = ?1, default_term_type = ?2, default_uframe = ?3, default_utool = ?4,
                default_w = ?5, default_p = ?6, default_r = ?7, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?8",
            params![default_speed, default_term_type, default_uframe, default_utool, default_w, default_p, default_r, id],
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
}

