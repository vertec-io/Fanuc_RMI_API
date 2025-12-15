//! HMI Widget Component Library
//!
//! This module provides industrial-grade HMI widgets for factory floor operators.
//! All widgets follow the design system established in the Fanuc RMI web application
//! with glassmorphism effects, neon accents, and proper alarm state visualization.
//!
//! ## Widget Types
//! - **LED Indicator**: Status display for digital I/O (DIN/DOUT)
//! - **Button**: Momentary or toggle control for digital outputs
//! - **Gauge**: Radial gauge for analog values (AIN/AOUT)
//! - **Slider**: Linear control for analog outputs
//! - **Bar**: Horizontal/vertical bar for analog display
//! - **Numeric**: Digital readout with optional units
//! - **MultiState**: Multi-position selector for group I/O
//!
//! ## Design Principles
//! - Server is the single source of truth - widgets react to server broadcasts
//! - All widgets support alarm states (Normal, Warning, Alarm)
//! - Touch-friendly sizing for industrial touchscreens
//! - High contrast for visibility in factory environments

mod led_indicator;
mod io_button;
mod gauge;
mod slider;
mod bar;
mod numeric;
mod multi_state;

pub use led_indicator::LedIndicator;
pub use io_button::IoButton;
pub use gauge::Gauge;
pub use slider::Slider;
pub use bar::Bar;
pub use numeric::Numeric;
pub use multi_state::{MultiState, StateOption};

#[allow(unused_imports)]
use crate::websocket::{IoType, WidgetType, AlarmState};

/// Common props for all HMI widgets
#[derive(Clone, Debug)]
pub struct WidgetProps {
    /// Display name shown on the widget
    pub display_name: String,
    /// I/O type (DIN, DOUT, AIN, AOUT, GIN, GOUT)
    pub io_type: IoType,
    /// I/O port index
    pub io_index: u16,
    /// Current alarm state
    pub alarm_state: AlarmState,
    /// Color when ON/active
    pub color_on: String,
    /// Color when OFF/inactive
    pub color_off: String,
    /// Whether the user has control authority
    pub has_control: bool,
}

impl Default for WidgetProps {
    fn default() -> Self {
        Self {
            display_name: "I/O".to_string(),
            io_type: IoType::DOUT,
            io_index: 1,
            alarm_state: AlarmState::Normal,
            color_on: "#00ff88".to_string(),
            color_off: "#333333".to_string(),
            has_control: false,
        }
    }
}

/// Get CSS color for alarm state
pub fn alarm_state_color(state: &AlarmState) -> &'static str {
    match state {
        AlarmState::Normal => "#00ff88",  // Green
        AlarmState::Warning => "#fbbf24", // Amber
        AlarmState::Alarm => "#ff4444",   // Red
    }
}

/// Get CSS glow effect for alarm state
pub fn alarm_state_glow(state: &AlarmState) -> &'static str {
    match state {
        AlarmState::Normal => "0 0 10px rgba(0, 255, 136, 0.3)",
        AlarmState::Warning => "0 0 10px rgba(251, 191, 36, 0.5)",
        AlarmState::Alarm => "0 0 15px rgba(255, 68, 68, 0.6)",
    }
}

