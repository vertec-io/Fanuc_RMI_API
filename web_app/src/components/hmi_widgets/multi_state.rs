//! MultiState Widget
//!
//! A multi-position selector for group I/O (GIN/GOUT).
//! Displays current state and allows selection of predefined states.

use leptos::prelude::*;
use super::alarm_state_color;
use crate::websocket::AlarmState;

/// A state option for the MultiState widget
#[derive(Clone, Debug)]
pub struct StateOption {
    /// The numeric value for this state
    pub value: u32,
    /// Display label for this state
    pub label: String,
    /// Optional color override for this state
    pub color: Option<String>,
}

/// MultiState component for group I/O control.
#[component]
pub fn MultiState(
    /// Display name shown below the selector
    #[prop(into)] name: String,
    /// Current value
    #[prop(into)] value: Signal<u32>,
    /// Callback when value changes
    #[prop(into)] on_change: Callback<u32>,
    /// Available state options
    #[prop(into)] options: Vec<StateOption>,
    /// Current alarm state
    #[prop(default = AlarmState::Normal)] alarm_state: AlarmState,
    /// Whether the user has control authority
    #[prop(default = true)] has_control: bool,
    /// Size variant
    #[prop(default = "md")] size: &'static str,
) -> impl IntoView {
    let color = alarm_state_color(&alarm_state);

    // Determine sizes based on variant
    let (btn_size, text_size, label_size) = match size {
        "sm" => ("px-2 py-1", "text-[7px]", "text-[7px]"),
        "lg" => ("px-4 py-2", "text-[11px]", "text-[10px]"),
        _ => ("px-3 py-1.5", "text-[9px]", "text-[8px]"), // md default
    };

    let disabled_class = if !has_control {
        "opacity-50"
    } else {
        ""
    };

    view! {
        <div class=format!("flex flex-col items-center gap-1 {}", disabled_class)>
            // State buttons
            <div class="flex gap-1 bg-[#111111] border border-[#ffffff08] rounded p-1">
                <For
                    each=move || options.clone()
                    key=|opt| opt.value
                    children=move |opt| {
                        let opt_value = opt.value;
                        let opt_label = opt.label.clone();
                        let opt_color = opt.color.clone();
                        let is_selected = move || value.get() == opt_value;

                        view! {
                            <button
                                class=format!(
                                    "{} {} rounded font-medium transition-all duration-150 border",
                                    btn_size, text_size
                                )
                                style=move || {
                                    let selected = is_selected();
                                    let btn_color = opt_color.as_deref().unwrap_or(color);
                                    if selected {
                                        format!(
                                            "background-color: {}20; border-color: {}; color: {}; box-shadow: 0 0 6px {}40;",
                                            btn_color, btn_color, btn_color, btn_color
                                        )
                                    } else {
                                        "background-color: transparent; border-color: #333333; color: #666666;".to_string()
                                    }
                                }
                                disabled=!has_control
                                on:click=move |_| {
                                    if has_control {
                                        on_change.run(opt_value);
                                    }
                                }
                            >
                                {opt_label}
                            </button>
                        }
                    }
                />
            </div>

            // Label
            <span class=format!("{} text-[#888888] text-center", label_size)>
                {name}
            </span>
        </div>
    }
}

