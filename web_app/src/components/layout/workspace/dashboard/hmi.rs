//! HMI Panel View - Displays configured HMI panels with live I/O widgets.
//!
//! This view renders HMI panels as configured in Settings, with widgets
//! connected to real-time I/O values from the robot.

use leptos::prelude::*;
use crate::websocket::WebSocketManager;
use crate::components::hmi_widgets::{
    LedIndicator, IoButton, Gauge, Slider, Bar, Numeric, MultiState, StateOption,
};
use web_common::{AlarmState, IoPortConfig, HmiPanelWithPorts, IoType};

/// HMI Panel View tab content.
#[component]
pub fn HmiTab() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let hmi_panels = ws.hmi_panels;
    let current_panel = ws.current_hmi_panel;
    let robot_connected = ws.robot_connected;
    let active_connection_id = ws.active_connection_id;

    // Selected panel ID
    let (selected_panel_id, set_selected_panel_id) = signal::<Option<i64>>(None);

    // Load HMI panels when robot connection changes
    Effect::new(move |_| {
        if let Some(conn_id) = active_connection_id.get() {
            ws.get_hmi_panels(conn_id);
        }
    });

    // Load panel with ports when selection changes
    Effect::new(move |_| {
        if let Some(panel_id) = selected_panel_id.get() {
            ws.get_hmi_panel_with_ports(panel_id);
        }
    });

    // Auto-select first panel if none selected
    Effect::new(move |_| {
        let panels = hmi_panels.get();
        if selected_panel_id.get().is_none() && !panels.is_empty() {
            set_selected_panel_id.set(Some(panels[0].id));
        }
    });

    view! {
        <div class="h-full flex flex-col gap-2">
            // Panel selector header
            <div class="flex items-center gap-4 p-2 bg-[#0d0d0d] rounded border border-[#ffffff08]">
                <span class="text-[10px] text-[#666666] uppercase tracking-wider">"HMI Panel"</span>
                <select
                    class="flex-1 bg-[#111111] border border-[#ffffff10] rounded px-2 py-1 text-[11px] text-[#cccccc] focus:outline-none focus:border-[#00d9ff40]"
                    on:change=move |ev| {
                        let value = event_target_value(&ev);
                        if let Ok(id) = value.parse::<i64>() {
                            set_selected_panel_id.set(Some(id));
                        }
                    }
                >
                    <For
                        each=move || hmi_panels.get()
                        key=|panel| panel.id
                        children=move |panel| {
                            let panel_id = panel.id;
                            let is_selected = move || selected_panel_id.get() == Some(panel_id);
                            view! {
                                <option value=panel_id.to_string() selected=is_selected>
                                    {panel.name.clone()}
                                </option>
                            }
                        }
                    />
                </select>

                // Pop-out button
                <button
                    class="p-1.5 rounded bg-[#111111] border border-[#ffffff10] text-[#666666] hover:text-[#00d9ff] hover:border-[#00d9ff40] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    title="Pop out panel"
                    disabled=move || selected_panel_id.get().is_none()
                    on:click=move |_| {
                        if let Some(panel_id) = selected_panel_id.get() {
                            crate::hmi_broadcast::open_hmi_popup(panel_id);
                        }
                    }
                >
                    <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/>
                    </svg>
                </button>
            </div>

            // Panel content
            <div class="flex-1 overflow-auto">
                <Show
                    when=move || current_panel.get().is_some()
                    fallback=move || view! {
                        <div class="h-full flex items-center justify-center">
                            <div class="text-center">
                                <div class="text-[#333333] text-4xl mb-2">"📊"</div>
                                <p class="text-[#666666] text-[11px]">
                                    {move || if hmi_panels.get().is_empty() {
                                        "No HMI panels configured. Create one in Settings → HMI Panels."
                                    } else {
                                        "Select a panel to view"
                                    }}
                                </p>
                            </div>
                        </div>
                    }
                >
                    {move || {
                        current_panel.get().map(|panel_data| {
                            view! { <HmiPanelGrid panel_data=panel_data /> }
                        })
                    }}
                </Show>
            </div>
        </div>
    }
}

/// Renders the HMI panel grid with widgets.
#[component]
pub fn HmiPanelGrid(panel_data: HmiPanelWithPorts) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let has_control = ws.has_control;
    let din_values = ws.din_values;
    let dout_values = ws.dout_values;
    let ain_values = ws.ain_values;
    let aout_values = ws.aout_values;
    let gin_values = ws.gin_values;
    let gout_values = ws.gout_values;

    let panel = panel_data.panel;
    let ports = panel_data.ports;

    // Grid style based on panel configuration
    let grid_style = format!(
        "display: grid; grid-template-columns: repeat({}, 1fr); grid-template-rows: repeat({}, 1fr); gap: 8px; padding: 8px;",
        panel.grid_columns,
        panel.grid_rows
    );

    view! {
        <div class="h-full bg-[#0a0a0a] rounded border border-[#ffffff08]" style=grid_style>
            <For
                each=move || ports.clone()
                key=|port| (port.io_type.clone(), port.io_index)
                children=move |port| {
                    view! { <HmiWidget port=port.clone() has_control=has_control /> }
                }
            />
        </div>
    }
}

/// Renders a single HMI widget based on port configuration.
#[component]
fn HmiWidget(
    port: IoPortConfig,
    has_control: ReadSignal<bool>,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let din_values = ws.din_values;
    let dout_values = ws.dout_values;
    let ain_values = ws.ain_values;
    let aout_values = ws.aout_values;
    let gin_values = ws.gin_values;
    let gout_values = ws.gout_values;

    let io_type = port.io_type;
    let io_index = port.io_index;
    let display_name = if port.display_name.is_empty() {
        format!("{}[{}]", port.io_type, port.io_index)
    } else {
        port.display_name.clone()
    };
    let widget_type = port.widget_type;
    let color_on = port.color_on.clone();
    let color_off = port.color_off.clone();
    let min_value = port.min_value.unwrap_or(0.0);
    let max_value = port.max_value.unwrap_or(100.0);
    let unit = port.unit.clone().unwrap_or_default();
    let warning_high = port.warning_high;
    let alarm_high = port.alarm_high;

    // Determine alarm state based on thresholds
    let alarm_state = move || {
        // For analog values, check thresholds
        match io_type {
            IoType::AIN => {
                let value = ain_values.get().get(&io_index).copied().unwrap_or(0.0);
                if let Some(alarm) = alarm_high {
                    if value >= alarm { return AlarmState::Alarm; }
                }
                if let Some(warning) = warning_high {
                    if value >= warning { return AlarmState::Warning; }
                }
                AlarmState::Normal
            }
            IoType::AOUT => {
                let value = aout_values.get().get(&io_index).copied().unwrap_or(0.0);
                if let Some(alarm) = alarm_high {
                    if value >= alarm { return AlarmState::Alarm; }
                }
                if let Some(warning) = warning_high {
                    if value >= warning { return AlarmState::Warning; }
                }
                AlarmState::Normal
            }
            _ => AlarmState::Normal,
        }
    };

    // Widget container with grid positioning (CSS grid is 1-based, our storage is 0-based)
    let grid_style = format!(
        "grid-column: {} / span {}; grid-row: {} / span {};",
        port.hmi_x.map(|x| x + 1).unwrap_or(1),
        port.hmi_width,
        port.hmi_y.map(|y| y + 1).unwrap_or(1),
        port.hmi_height
    );

    // Render appropriate widget based on type and I/O type
    let widget_view = move || {
        use web_common::{IoType, WidgetType};

        match (io_type, widget_type) {
            // Digital Inputs - always LED (read-only)
            (IoType::DIN, _) => {
                let value = Signal::derive(move || din_values.get().get(&io_index).copied().unwrap_or(false));
                view! {
                    <LedIndicator
                        name=display_name.clone()
                        value=value
                        color_on=color_on.clone()
                        color_off=color_off.clone()
                        alarm_state=alarm_state()
                    />
                }.into_any()
            }
            // Digital Outputs - LED display only
            (IoType::DOUT, WidgetType::Led) => {
                let value = Signal::derive(move || dout_values.get().get(&io_index).copied().unwrap_or(false));
                view! {
                    <LedIndicator
                        name=display_name.clone()
                        value=value
                        color_on=color_on.clone()
                        color_off=color_off.clone()
                        alarm_state=alarm_state()
                    />
                }.into_any()
            }
            // Digital Outputs - Button (default)
            (IoType::DOUT, _) => {
                let value = Signal::derive(move || dout_values.get().get(&io_index).copied().unwrap_or(false));
                let on_change = Callback::new(move |new_value: bool| {
                    ws.write_dout(io_index, new_value);
                });
                view! {
                    <IoButton
                        name=display_name.clone()
                        value=value
                        on_change=on_change
                        color_on=color_on.clone()
                        color_off=color_off.clone()
                        alarm_state=alarm_state()
                        has_control=has_control.get()
                    />
                }.into_any()
            }
            // Analog Inputs - Bar
            (IoType::AIN, WidgetType::Bar) => {
                let value = Signal::derive(move || ain_values.get().get(&io_index).copied().unwrap_or(0.0));
                view! {
                    <Bar
                        name=display_name.clone()
                        value=value
                        min=min_value
                        max=max_value
                        unit=unit.clone().leak()
                        alarm_state=alarm_state()
                    />
                }.into_any()
            }
            // Analog Inputs - Numeric
            (IoType::AIN, WidgetType::Numeric) => {
                let value = Signal::derive(move || ain_values.get().get(&io_index).copied().unwrap_or(0.0));
                view! {
                    <Numeric
                        name=display_name.clone()
                        value=value
                        unit=unit.clone().leak()
                        alarm_state=alarm_state()
                    />
                }.into_any()
            }
            // Analog Inputs - Gauge (default)
            (IoType::AIN, _) => {
                let value = Signal::derive(move || ain_values.get().get(&io_index).copied().unwrap_or(0.0));
                view! {
                    <Gauge
                        name=display_name.clone()
                        value=value
                        min=min_value
                        max=max_value
                        unit=unit.clone().leak()
                        alarm_state=alarm_state()
                    />
                }.into_any()
            }
            // Analog Outputs - Slider (default)
            (IoType::AOUT, WidgetType::Slider) | (IoType::AOUT, WidgetType::Auto) => {
                let value = Signal::derive(move || aout_values.get().get(&io_index).copied().unwrap_or(0.0));
                let on_change = Callback::new(move |new_value: f64| {
                    ws.write_aout(io_index, new_value);
                });
                view! {
                    <Slider
                        name=display_name.clone()
                        value=value
                        on_change=on_change
                        min=min_value
                        max=max_value
                        unit=unit.clone().leak()
                        alarm_state=alarm_state()
                        has_control=has_control.get()
                    />
                }.into_any()
            }
            // Analog Outputs - Gauge display
            (IoType::AOUT, _) => {
                let value = Signal::derive(move || aout_values.get().get(&io_index).copied().unwrap_or(0.0));
                view! {
                    <Gauge
                        name=display_name.clone()
                        value=value
                        min=min_value
                        max=max_value
                        unit=unit.clone().leak()
                        alarm_state=alarm_state()
                    />
                }.into_any()
            }
            // Group Input - Numeric display
            (IoType::GIN, _) => {
                let value = Signal::derive(move || gin_values.get().get(&io_index).copied().unwrap_or(0) as f64);
                view! {
                    <Numeric
                        name=display_name.clone()
                        value=value
                        decimals=0
                    />
                }.into_any()
            }
            // Group Output - MultiState or Numeric
            (IoType::GOUT, _) => {
                if has_control.get() {
                    let value = Signal::derive(move || gout_values.get().get(&io_index).copied().unwrap_or(0));
                    let on_change = Callback::new(move |new_value: u32| {
                        ws.write_gout(io_index, new_value);
                    });
                    // Simple multi-state with 4 options (0-3)
                    let options = vec![
                        StateOption { value: 0, label: "0".to_string(), color: None },
                        StateOption { value: 1, label: "1".to_string(), color: None },
                        StateOption { value: 2, label: "2".to_string(), color: None },
                        StateOption { value: 3, label: "3".to_string(), color: None },
                    ];
                    view! {
                        <MultiState
                            name=display_name.clone()
                            value=value
                            on_change=on_change
                            options=options
                            has_control=has_control.get()
                        />
                    }.into_any()
                } else {
                    let value = Signal::derive(move || gout_values.get().get(&io_index).copied().unwrap_or(0) as f64);
                    view! {
                        <Numeric
                            name=display_name.clone()
                            value=value
                            decimals=0
                        />
                    }.into_any()
                }
            }
        }
    };

    view! {
        <div class="hmi-widget" style=grid_style>
            {widget_view}
        </div>
    }
}

