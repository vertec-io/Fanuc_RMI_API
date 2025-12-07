//! Workspace context and shared types.
//!
//! Contains the WorkspaceContext for sharing state between workspace views
//! and common data structures used throughout the workspace.

use leptos::prelude::*;

/// Shared context for frame/tool data and program state
#[derive(Clone, Copy)]
pub struct WorkspaceContext {
    /// Active UFrame number
    pub active_frame: RwSignal<usize>,
    /// Active UTool number
    pub active_tool: RwSignal<usize>,
    /// Expanded frame in accordion (-1 = none)
    pub expanded_frame: RwSignal<i32>,
    /// Expanded tool in accordion (-1 = none)
    pub expanded_tool: RwSignal<i32>,
    /// Command log entries (legacy - kept for compatibility)
    pub command_log: RwSignal<Vec<CommandLogEntry>>,
    /// Recent commands that can be re-run
    pub recent_commands: RwSignal<Vec<RecentCommand>>,
    /// Currently selected command ID in the dropdown (None = no selection)
    pub selected_command_id: RwSignal<Option<usize>>,
    /// Current program lines
    pub program_lines: RwSignal<Vec<ProgramLine>>,
    /// Currently executing line (-1 = none)
    pub executing_line: RwSignal<i32>,
    /// Show command composer modal
    pub show_composer: RwSignal<bool>,
    /// Loaded program name (for display in Dashboard)
    pub loaded_program_name: RwSignal<Option<String>>,
    /// Loaded program ID (for execution)
    pub loaded_program_id: RwSignal<Option<i64>>,
    /// Program is currently running
    pub program_running: RwSignal<bool>,
    /// Program is paused
    pub program_paused: RwSignal<bool>,
}

impl WorkspaceContext {
    pub fn new() -> Self {
        Self {
            active_frame: RwSignal::new(0),
            active_tool: RwSignal::new(0),
            expanded_frame: RwSignal::new(0), // Default expand frame 0
            expanded_tool: RwSignal::new(0),  // Default expand tool 0
            command_log: RwSignal::new(Vec::new()),
            recent_commands: RwSignal::new(Vec::new()),
            selected_command_id: RwSignal::new(None),
            program_lines: RwSignal::new(Vec::new()),
            executing_line: RwSignal::new(-1),
            show_composer: RwSignal::new(false),
            loaded_program_name: RwSignal::new(None),
            loaded_program_id: RwSignal::new(None),
            program_running: RwSignal::new(false),
            program_paused: RwSignal::new(false),
        }
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct CommandLogEntry {
    pub timestamp: String,
    pub command: String,
    pub status: CommandStatus,
}

/// A recently executed command that can be re-run
#[derive(Clone, Debug)]
pub struct RecentCommand {
    pub id: usize,
    pub name: String,
    pub command_type: String,
    pub description: String,
    // Motion parameters
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub term_type: String,
    pub uframe: u8,
    pub utool: u8,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum CommandStatus {
    Pending,
    Success,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct ProgramLine {
    pub line_number: usize,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub term_type: String,
    pub uframe: Option<i32>,
    pub utool: Option<i32>,
}

