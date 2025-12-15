//! Bar Widget
//!
//! A horizontal or vertical bar for displaying analog values (AIN/AOUT).
//! Shows value with optional warning/alarm threshold markers.

use leptos::prelude::*;
use super::alarm_state_color;
use crate::websocket::AlarmState;

/// Bar component for analog value display.
#[component]
pub fn Bar(
    /// Display name shown below the bar
    #[prop(into)] name: String,
    /// Current value
    #[prop(into)] value: Signal<f64>,
    /// Minimum value
    #[prop(default = 0.0)] min: f64,
    /// Maximum value
    #[prop(default = 100.0)] max: f64,
    /// Unit label (e.g., "PSI", "°C", "%")
    #[prop(default = "")] unit: &'static str,
    /// Warning threshold (optional)
    #[prop(optional)] warning_threshold: Option<f64>,
    /// Alarm threshold (optional)
    #[prop(optional)] alarm_threshold: Option<f64>,
    /// Current alarm state
    #[prop(default = AlarmState::Normal)] alarm_state: AlarmState,
    /// Orientation (horizontal or vertical)
    #[prop(default = "horizontal")] orientation: &'static str,
    /// Size variant
    #[prop(default = "md")] size: &'static str,
) -> impl IntoView {
    let color = alarm_state_color(&alarm_state);

    // Determine bar size based on variant and orientation
    let (bar_class, value_size, label_size) = match (orientation, size) {
        ("vertical", "sm") => ("w-3 h-16", "text-[8px]", "text-[7px]"),
        ("vertical", "lg") => ("w-5 h-28", "text-[12px]", "text-[10px]"),
        ("vertical", _) => ("w-4 h-20", "text-[10px]", "text-[8px]"),
        (_, "sm") => ("w-16 h-3", "text-[8px]", "text-[7px]"),
        (_, "lg") => ("w-28 h-5", "text-[12px]", "text-[10px]"),
        _ => ("w-20 h-4", "text-[10px]", "text-[8px]"),
    };

    let container_class = if orientation == "vertical" {
        "flex flex-col items-center gap-1"
    } else {
        "flex flex-col items-center gap-1"
    };

    // Calculate percentage
    let percentage = move || {
        let v = value.get();
        let range = max - min;
        if range <= 0.0 { return 0.0; }
        ((v - min) / range * 100.0).clamp(0.0, 100.0)
    };

    view! {
        <div class=container_class>
            // Value display
            <div class="flex items-baseline gap-1">
                <span class=format!("{} font-mono font-bold", value_size) style=format!("color: {};", color)>
                    {move || format!("{:.1}", value.get())}
                </span>
                {(!unit.is_empty()).then(|| view! {
                    <span class=format!("{} text-[#666666]", label_size)>{unit}</span>
                })}
            </div>

            // Bar container
            <div class=format!("{} relative bg-[#222222] rounded overflow-hidden", bar_class)>
                // Fill
                {if orientation == "vertical" {
                    view! {
                        <div
                            class="absolute bottom-0 left-0 w-full rounded transition-all duration-200"
                            style=move || {
                                let pct = percentage();
                                format!(
                                    "height: {}%; background-color: {}; box-shadow: 0 0 6px {}60;",
                                    pct, color, color
                                )
                            }
                        />
                    }.into_any()
                } else {
                    view! {
                        <div
                            class="absolute left-0 top-0 h-full rounded transition-all duration-200"
                            style=move || {
                                let pct = percentage();
                                format!(
                                    "width: {}%; background-color: {}; box-shadow: 0 0 6px {}60;",
                                    pct, color, color
                                )
                            }
                        />
                    }.into_any()
                }}

                // Warning threshold marker
                {warning_threshold.map(|threshold| {
                    let pct = ((threshold - min) / (max - min) * 100.0).clamp(0.0, 100.0);
                    let style = if orientation == "vertical" {
                        format!("bottom: {}%; left: 0; right: 0; height: 2px;", pct)
                    } else {
                        format!("left: {}%; top: 0; bottom: 0; width: 2px;", pct)
                    };
                    view! {
                        <div class="absolute bg-[#fbbf24]" style=style />
                    }
                })}

                // Alarm threshold marker
                {alarm_threshold.map(|threshold| {
                    let pct = ((threshold - min) / (max - min) * 100.0).clamp(0.0, 100.0);
                    let style = if orientation == "vertical" {
                        format!("bottom: {}%; left: 0; right: 0; height: 2px;", pct)
                    } else {
                        format!("left: {}%; top: 0; bottom: 0; width: 2px;", pct)
                    };
                    view! {
                        <div class="absolute bg-[#ff4444]" style=style />
                    }
                })}
            </div>

            // Label
            <span class=format!("{} text-[#888888] text-center", label_size)>
                {name}
            </span>
        </div>
    }
}

