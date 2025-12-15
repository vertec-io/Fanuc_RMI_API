//! Slider Widget
//!
//! A linear slider control for analog outputs (AOUT).
//! Supports horizontal and vertical orientations.

use leptos::prelude::*;
use super::alarm_state_color;
use crate::websocket::AlarmState;

/// Slider component for analog output control.
#[component]
pub fn Slider(
    /// Display name shown below the slider
    #[prop(into)] name: String,
    /// Current value
    #[prop(into)] value: Signal<f64>,
    /// Callback when value changes
    #[prop(into)] on_change: Callback<f64>,
    /// Minimum value
    #[prop(default = 0.0)] min: f64,
    /// Maximum value
    #[prop(default = 100.0)] max: f64,
    /// Step increment
    #[prop(default = 1.0)] step: f64,
    /// Unit label (e.g., "PSI", "°C", "%")
    #[prop(default = "")] unit: &'static str,
    /// Current alarm state
    #[prop(default = AlarmState::Normal)] alarm_state: AlarmState,
    /// Whether the user has control authority
    #[prop(default = true)] has_control: bool,
    /// Orientation (horizontal or vertical)
    #[prop(default = "horizontal")] orientation: &'static str,
) -> impl IntoView {
    let color = alarm_state_color(&alarm_state);

    let disabled_class = if !has_control {
        "opacity-50 cursor-not-allowed"
    } else {
        ""
    };

    let (container_class, slider_class) = if orientation == "vertical" {
        ("flex flex-col items-center gap-2 h-32", "h-24 w-2 -rotate-90")
    } else {
        ("flex flex-col items-center gap-1 w-32", "w-full h-2")
    };

    view! {
        <div class=format!("{} {}", container_class, disabled_class)>
            // Value display
            <div class="flex items-baseline gap-1">
                <span class="text-[11px] font-mono font-bold" style=format!("color: {};", color)>
                    {move || format!("{:.1}", value.get())}
                </span>
                {(!unit.is_empty()).then(|| view! {
                    <span class="text-[8px] text-[#666666]">{unit}</span>
                })}
            </div>

            // Slider track
            <div class=format!("{} relative bg-[#222222] rounded-full", slider_class)>
                // Fill
                <div
                    class="absolute left-0 top-0 h-full rounded-full transition-all duration-100"
                    style=move || {
                        let pct = ((value.get() - min) / (max - min) * 100.0).clamp(0.0, 100.0);
                        format!(
                            "width: {}%; background-color: {}; box-shadow: 0 0 6px {}60;",
                            pct, color, color
                        )
                    }
                />
                // Input range (invisible, for interaction)
                <input
                    type="range"
                    class="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
                    min=min
                    max=max
                    step=step
                    prop:value=move || value.get()
                    disabled=!has_control
                    on:input=move |ev| {
                        if has_control {
                            if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                on_change.run(v);
                            }
                        }
                    }
                />
                // Thumb indicator
                <div
                    class="absolute top-1/2 -translate-y-1/2 w-4 h-4 rounded-full border-2 bg-[#0a0a0a] transition-all duration-100"
                    style=move || {
                        let pct = ((value.get() - min) / (max - min) * 100.0).clamp(0.0, 100.0);
                        format!(
                            "left: calc({}% - 8px); border-color: {}; box-shadow: 0 0 8px {}80;",
                            pct, color, color
                        )
                    }
                />
            </div>

            // Label
            <span class="text-[8px] text-[#888888] text-center">
                {name}
            </span>
        </div>
    }
}

