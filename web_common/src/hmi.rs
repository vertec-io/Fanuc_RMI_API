//! HMI Panel System types.
//!
//! These types define I/O port configurations and HMI panel layouts.
//! Following the "one type per concept" philosophy - no separate DTO types.

use serde::{Deserialize, Serialize};

/// I/O type enumeration for all digital, analog, and group I/O.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IoType {
    DIN,
    DOUT,
    AIN,
    AOUT,
    GIN,
    GOUT,
}

impl IoType {
    /// Convert to string representation for database storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            IoType::DIN => "DIN",
            IoType::DOUT => "DOUT",
            IoType::AIN => "AIN",
            IoType::AOUT => "AOUT",
            IoType::GIN => "GIN",
            IoType::GOUT => "GOUT",
        }
    }

    /// Parse from string (case-insensitive).
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "DIN" => Some(IoType::DIN),
            "DOUT" => Some(IoType::DOUT),
            "AIN" => Some(IoType::AIN),
            "AOUT" => Some(IoType::AOUT),
            "GIN" => Some(IoType::GIN),
            "GOUT" => Some(IoType::GOUT),
            _ => None,
        }
    }

    /// Returns true if this is an output type (can be written to).
    pub fn is_output(&self) -> bool {
        matches!(self, IoType::DOUT | IoType::AOUT | IoType::GOUT)
    }

    /// Returns true if this is an analog type.
    pub fn is_analog(&self) -> bool {
        matches!(self, IoType::AIN | IoType::AOUT)
    }

    /// Returns true if this is a group type.
    pub fn is_group(&self) -> bool {
        matches!(self, IoType::GIN | IoType::GOUT)
    }
}

impl std::fmt::Display for IoType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Widget type for I/O display in HMI panels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum WidgetType {
    /// System chooses based on I/O type.
    #[default]
    Auto,
    /// Momentary push button (for DOUT).
    Button,
    /// Toggle switch (for DOUT).
    Toggle,
    /// LED indicator (for DIN).
    Led,
    /// Circular gauge (for AIN/AOUT).
    Gauge,
    /// Linear slider (for AOUT).
    Slider,
    /// Horizontal/vertical bar graph (for AIN/AOUT).
    Bar,
    /// Numeric display with unit.
    Numeric,
    /// Multi-state indicator (for GIN/GOUT).
    MultiState,
}

impl WidgetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WidgetType::Auto => "auto",
            WidgetType::Button => "button",
            WidgetType::Toggle => "toggle",
            WidgetType::Led => "led",
            WidgetType::Gauge => "gauge",
            WidgetType::Slider => "slider",
            WidgetType::Bar => "bar",
            WidgetType::Numeric => "numeric",
            WidgetType::MultiState => "multi_state",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "auto" => Some(WidgetType::Auto),
            "button" => Some(WidgetType::Button),
            "toggle" => Some(WidgetType::Toggle),
            "led" => Some(WidgetType::Led),
            "gauge" => Some(WidgetType::Gauge),
            "slider" => Some(WidgetType::Slider),
            "bar" => Some(WidgetType::Bar),
            "numeric" => Some(WidgetType::Numeric),
            "multi_state" | "multistate" => Some(WidgetType::MultiState),
            _ => None,
        }
    }
}

impl std::fmt::Display for WidgetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Alarm state for threshold checking.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum AlarmState {
    #[default]
    Normal,
    Warning,
    Alarm,
}

/// Configuration for a single I/O port on a robot connection.
/// Defines display properties, value constraints, thresholds, and HMI layout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IoPortConfig {
    /// I/O type (DIN, DOUT, AIN, AOUT, GIN, GOUT).
    pub io_type: IoType,
    /// Port index (1-based, matching robot controller).
    pub io_index: u16,

    // === Identity & Description ===
    /// Display name shown in UI (e.g., "Gripper Open" instead of "DOUT1").
    pub display_name: String,
    /// Detailed description of what this port does.
    pub description: Option<String>,
    /// Category for grouping (e.g., "Gripper", "Conveyor", "Safety").
    pub category: Option<String>,

    // === Display Configuration ===
    /// Widget type to use for this port.
    pub widget_type: WidgetType,
    /// Color when port is on/active (CSS color string).
    pub color_on: String,
    /// Color when port is off/inactive (CSS color string).
    pub color_off: String,
    /// Icon identifier (optional).
    pub icon: Option<String>,

    // === Value Constraints (for analog/group I/O) ===
    /// Minimum value (for analog I/O).
    pub min_value: Option<f64>,
    /// Maximum value (for analog I/O).
    pub max_value: Option<f64>,
    /// Engineering unit (e.g., "PSI", "°C", "%").
    pub unit: Option<String>,
    /// Number of decimal places to display.
    pub decimal_places: u8,

    // === Warning/Alarm Thresholds ===
    /// Low warning threshold.
    pub warning_low: Option<f64>,
    /// High warning threshold.
    pub warning_high: Option<f64>,
    /// Low alarm threshold.
    pub alarm_low: Option<f64>,
    /// High alarm threshold.
    pub alarm_high: Option<f64>,
    /// Whether warning thresholds are enabled.
    pub warning_enabled: bool,
    /// Whether alarm thresholds are enabled.
    pub alarm_enabled: bool,

    // === HMI Panel Layout ===
    /// Whether this port is shown on an HMI panel.
    pub hmi_enabled: bool,
    /// X position in HMI grid.
    pub hmi_x: Option<u16>,
    /// Y position in HMI grid.
    pub hmi_y: Option<u16>,
    /// Width in grid units.
    pub hmi_width: u16,
    /// Height in grid units.
    pub hmi_height: u16,
    /// HMI panel this port belongs to.
    pub hmi_panel_id: Option<i64>,

    // === Standard I/O View ===
    /// Whether visible in standard I/O view.
    pub is_visible: bool,
    /// Display order in standard view.
    pub display_order: Option<u16>,
}

impl Default for IoPortConfig {
    fn default() -> Self {
        Self {
            io_type: IoType::DIN,
            io_index: 1,
            display_name: String::new(),
            description: None,
            category: None,
            widget_type: WidgetType::Auto,
            color_on: "#00ff88".to_string(),
            color_off: "#333333".to_string(),
            icon: None,
            min_value: None,
            max_value: None,
            unit: None,
            decimal_places: 2,
            warning_low: None,
            warning_high: None,
            alarm_low: None,
            alarm_high: None,
            warning_enabled: false,
            alarm_enabled: false,
            hmi_enabled: false,
            hmi_x: None,
            hmi_y: None,
            hmi_width: 1,
            hmi_height: 1,
            hmi_panel_id: None,
            is_visible: true,
            display_order: None,
        }
    }
}

/// HMI Panel configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HmiPanel {
    /// Panel ID.
    pub id: i64,
    /// Robot connection this panel belongs to.
    pub robot_connection_id: i64,
    /// Panel name (e.g., "Gripper Control", "Conveyor Status").
    pub name: String,
    /// Panel description.
    pub description: Option<String>,
    /// Number of columns in the grid.
    pub grid_columns: u16,
    /// Number of rows in the grid.
    pub grid_rows: u16,
    /// Background color (CSS color string).
    pub background_color: String,
    /// Whether this is the default panel for the robot.
    pub is_default: bool,
}

impl Default for HmiPanel {
    fn default() -> Self {
        Self {
            id: 0,
            robot_connection_id: 0,
            name: "New Panel".to_string(),
            description: None,
            grid_columns: 8,
            grid_rows: 6,
            background_color: "#1a1a1a".to_string(),
            is_default: false,
        }
    }
}

/// Full HMI panel with its configured ports.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HmiPanelWithPorts {
    /// The panel configuration.
    #[serde(flatten)]
    pub panel: HmiPanel,
    /// Ports assigned to this panel.
    pub ports: Vec<IoPortConfig>,
}
