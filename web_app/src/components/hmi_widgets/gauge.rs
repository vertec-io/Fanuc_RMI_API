//! Gauge Widget
//!
//! A radial gauge for displaying analog I/O values (AIN/AOUT).
//! Shows value with optional warning/alarm thresholds.

use leptos::prelude::*;
use super::alarm_state_color;
use crate::websocket::AlarmState;

/// Gauge component for analog value display.
#[component]
pub fn Gauge(
    /// Display name shown below the gauge
    #[prop(into)] name: String,
    /// Current value
    #[prop(into)] value: Signal<f64>,
    /// Minimum value
    #[prop(default = 0.0)] min: f64,
    /// Maximum value
    #[prop(default = 100.0)] max: f64,
    /// Unit label (e.g., "PSI", "°C", "%")
    #[prop(default = "")] unit: &'static str,
    /// Current alarm state
    #[prop(default = AlarmState::Normal)] alarm_state: AlarmState,
    /// Size variant
    #[prop(default = "md")] size: &'static str,
) -> impl IntoView {
    let alarm_state_clone = alarm_state.clone();

    // Determine gauge size based on variant
    let (gauge_size, value_size, label_size) = match size {
        "sm" => ("w-16 h-16", "text-[10px]", "text-[7px]"),
        "lg" => ("w-28 h-28", "text-[16px]", "text-[10px]"),
        _ => ("w-20 h-20", "text-[12px]", "text-[8px]"), // md default
    };

    // Calculate percentage for arc
    let percentage = move || {
        let v = value.get();
        let range = max - min;
        if range <= 0.0 { return 0.0; }
        ((v - min) / range * 100.0).clamp(0.0, 100.0)
    };

    // SVG arc calculation (270 degree sweep)
    let arc_path = move || {
        let pct = percentage();
        let angle = (pct / 100.0) * 270.0 - 135.0; // Start at -135°, sweep 270°
        let rad = angle.to_radians();
        let cx = 50.0;
        let cy = 50.0;
        let r = 40.0;
        let x = cx + r * rad.cos();
        let y = cy + r * rad.sin();
        let large_arc = if pct > 50.0 { 1 } else { 0 };
        format!(
            "M {} {} A {} {} 0 {} 1 {} {}",
            cx - r * (-135.0_f64).to_radians().cos(),
            cy - r * (-135.0_f64).to_radians().sin(),
            r, r, large_arc, x, y
        )
    };

    let color = alarm_state_color(&alarm_state_clone);

    view! {
        <div class="flex flex-col items-center gap-1">
            // Gauge SVG
            <div class=format!("{} relative", gauge_size)>
                <svg viewBox="0 0 100 100" class="w-full h-full">
                    // Background arc
                    <path
                        d="M 10.3 78.3 A 40 40 0 1 1 89.7 78.3"
                        fill="none"
                        stroke="#333333"
                        stroke-width="8"
                        stroke-linecap="round"
                    />
                    // Value arc
                    <path
                        d=arc_path
                        fill="none"
                        stroke=color
                        stroke-width="8"
                        stroke-linecap="round"
                        style=format!("filter: drop-shadow(0 0 4px {}80);", color)
                    />
                </svg>
                // Value display in center
                <div class="absolute inset-0 flex flex-col items-center justify-center">
                    <span class=format!("{} font-mono font-bold", value_size) style=format!("color: {};", color)>
                        {move || format!("{:.1}", value.get())}
                    </span>
                    {(!unit.is_empty()).then(|| view! {
                        <span class=format!("{} text-[#666666]", label_size)>{unit}</span>
                    })}
                </div>
            </div>
            // Label
            <span class=format!("{} text-[#888888] text-center", label_size)>
                {name}
            </span>
        </div>
    }
}

