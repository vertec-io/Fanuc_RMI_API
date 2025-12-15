//! I/O Button Widget
//!
//! A button control for digital outputs (DOUT).
//! Supports momentary (press-and-hold) and toggle modes.

use leptos::prelude::*;
use super::alarm_state_glow;
use crate::websocket::AlarmState;

/// Button mode for I/O control
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ButtonMode {
    /// Momentary: ON while pressed, OFF when released
    #[default]
    Momentary,
    /// Toggle: Click to toggle between ON and OFF
    Toggle,
}

/// I/O Button component for digital output control.
#[component]
pub fn IoButton(
    /// Display name shown on the button
    #[prop(into)] name: String,
    /// Current value (true = ON, false = OFF)
    #[prop(into)] value: Signal<bool>,
    /// Callback when button is pressed (momentary) or toggled
    #[prop(into)] on_change: Callback<bool>,
    /// Button mode (Momentary or Toggle)
    #[prop(default = ButtonMode::Momentary)] mode: ButtonMode,
    /// Color when ON
    #[prop(default = "#00ff88".to_string())] color_on: String,
    /// Color when OFF
    #[prop(default = "#333333".to_string())] color_off: String,
    /// Current alarm state
    #[prop(default = AlarmState::Normal)] alarm_state: AlarmState,
    /// Whether the user has control authority
    #[prop(default = true)] has_control: bool,
    /// Size variant
    #[prop(default = "md")] size: &'static str,
) -> impl IntoView {
    let color_on_clone = color_on.clone();
    let color_off_clone = color_off.clone();
    let alarm_state_clone = alarm_state.clone();

    // Determine button size based on variant
    let (btn_size, text_size) = match size {
        "sm" => ("min-w-[40px] h-8 px-2", "text-[8px]"),
        "lg" => ("min-w-[80px] h-14 px-4", "text-[12px]"),
        _ => ("min-w-[60px] h-10 px-3", "text-[10px]"), // md default
    };

    let disabled_class = if !has_control {
        "opacity-50 cursor-not-allowed"
    } else {
        "cursor-pointer"
    };

    view! {
        <button
            class=format!(
                "{} {} {} rounded font-medium transition-all duration-150 border-2 select-none",
                btn_size, text_size, disabled_class
            )
            style=move || {
                let is_on = value.get();
                let bg_color = if is_on {
                    format!("{}20", color_on_clone) // 20% opacity background
                } else {
                    format!("{}10", color_off_clone)
                };
                let border_color = if is_on { &color_on_clone } else { "#555555" };
                let text_color = if is_on { &color_on_clone } else { "#888888" };
                let glow = if is_on {
                    match alarm_state_clone {
                        AlarmState::Normal => format!("0 0 8px {}40", color_on_clone),
                        AlarmState::Warning => alarm_state_glow(&AlarmState::Warning).to_string(),
                        AlarmState::Alarm => alarm_state_glow(&AlarmState::Alarm).to_string(),
                    }
                } else {
                    "none".to_string()
                };
                format!(
                    "background-color: {}; border-color: {}; color: {}; box-shadow: {};",
                    bg_color, border_color, text_color, glow
                )
            }
            disabled=!has_control
            on:mousedown=move |_| {
                if has_control && mode == ButtonMode::Momentary {
                    on_change.run(true);
                }
            }
            on:mouseup=move |_| {
                if has_control && mode == ButtonMode::Momentary {
                    on_change.run(false);
                }
            }
            on:mouseleave=move |_| {
                if has_control && mode == ButtonMode::Momentary && value.get() {
                    on_change.run(false);
                }
            }
            on:click=move |_| {
                if has_control && mode == ButtonMode::Toggle {
                    on_change.run(!value.get());
                }
            }
        >
            {name}
        </button>
    }
}

