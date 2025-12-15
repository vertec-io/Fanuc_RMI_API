//! Numeric Widget
//!
//! A digital readout display for numeric values.
//! Shows value with optional units and alarm state visualization.

use leptos::prelude::*;
use super::alarm_state_color;
use crate::websocket::AlarmState;

/// Numeric component for digital value display.
#[component]
pub fn Numeric(
    /// Display name shown below the value
    #[prop(into)] name: String,
    /// Current value
    #[prop(into)] value: Signal<f64>,
    /// Unit label (e.g., "PSI", "°C", "%")
    #[prop(default = "")] unit: &'static str,
    /// Number of decimal places
    #[prop(default = 1)] decimals: usize,
    /// Current alarm state
    #[prop(default = AlarmState::Normal)] alarm_state: AlarmState,
    /// Size variant
    #[prop(default = "md")] size: &'static str,
) -> impl IntoView {
    let color = alarm_state_color(&alarm_state);

    // Determine sizes based on variant
    let (value_size, unit_size, label_size, container_width) = match size {
        "sm" => ("text-[14px]", "text-[8px]", "text-[7px]", "min-w-[50px]"),
        "lg" => ("text-[24px]", "text-[12px]", "text-[10px]", "min-w-[100px]"),
        _ => ("text-[18px]", "text-[10px]", "text-[8px]", "min-w-[70px]"), // md default
    };

    view! {
        <div class=format!("flex flex-col items-center gap-0.5 {}", container_width)>
            // Value display with unit
            <div class="flex items-baseline gap-1 bg-[#111111] border border-[#ffffff08] rounded px-2 py-1">
                <span
                    class=format!("{} font-mono font-bold tabular-nums", value_size)
                    style=format!("color: {}; text-shadow: 0 0 8px {}60;", color, color)
                >
                    {move || format!("{:.prec$}", value.get(), prec = decimals)}
                </span>
                {(!unit.is_empty()).then(|| view! {
                    <span class=format!("{} text-[#666666]", unit_size)>{unit}</span>
                })}
            </div>

            // Label
            <span class=format!("{} text-[#888888] text-center", label_size)>
                {name}
            </span>
        </div>
    }
}

