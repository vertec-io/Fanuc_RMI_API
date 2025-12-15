//! LED Indicator Widget
//!
//! A status indicator for digital I/O (DIN/DOUT).
//! Shows ON/OFF state with configurable colors and alarm visualization.

use leptos::prelude::*;
#[allow(unused_imports)]
use super::{alarm_state_color, alarm_state_glow};
use crate::websocket::AlarmState;

/// LED Indicator component for digital I/O status display.
#[component]
pub fn LedIndicator(
    /// Display name shown below the LED
    #[prop(into)] name: String,
    /// Current value (true = ON, false = OFF)
    #[prop(into)] value: Signal<bool>,
    /// Color when ON
    #[prop(default = "#00ff88".to_string())] color_on: String,
    /// Color when OFF
    #[prop(default = "#333333".to_string())] color_off: String,
    /// Current alarm state
    #[prop(default = AlarmState::Normal)] alarm_state: AlarmState,
    /// Size variant
    #[prop(default = "md")] size: &'static str,
) -> impl IntoView {
    let color_on_clone = color_on.clone();
    let color_off_clone = color_off.clone();
    let alarm_state_clone = alarm_state.clone();

    // Determine LED size based on variant
    let (led_size, text_size, container_size) = match size {
        "sm" => ("w-4 h-4", "text-[7px]", "w-12"),
        "lg" => ("w-8 h-8", "text-[10px]", "w-20"),
        _ => ("w-6 h-6", "text-[8px]", "w-16"), // md default
    };

    view! {
        <div class=format!("flex flex-col items-center gap-1 {}", container_size)>
            // LED element
            <div
                class=format!(
                    "{} rounded-full transition-all duration-200 border-2",
                    led_size
                )
                style=move || {
                    let is_on = value.get();
                    let bg_color = if is_on { &color_on_clone } else { &color_off_clone };
                    let border_color = if is_on { &color_on_clone } else { "#555555" };
                    let glow = if is_on {
                        match alarm_state_clone {
                            AlarmState::Normal => format!("0 0 12px {}", color_on_clone),
                            AlarmState::Warning => alarm_state_glow(&AlarmState::Warning).to_string(),
                            AlarmState::Alarm => alarm_state_glow(&AlarmState::Alarm).to_string(),
                        }
                    } else {
                        "none".to_string()
                    };
                    format!(
                        "background-color: {}; border-color: {}; box-shadow: {};",
                        bg_color, border_color, glow
                    )
                }
            />
            // Label
            <span class=format!("{} text-[#888888] text-center truncate w-full", text_size)>
                {name}
            </span>
        </div>
    }
}

