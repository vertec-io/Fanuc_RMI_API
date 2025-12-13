//! Right panel component - always visible in Dashboard route.

use leptos::prelude::*;
use leptos::html::Div;
use leptos_use::{use_draggable_with_options, UseDraggableOptions, UseDraggableReturn, core::Position};
use leptos_router::hooks::use_location;
use super::LayoutContext;
use crate::components::{PositionDisplay, ErrorLog, JogControls, RobotStatus};
use crate::websocket::WebSocketManager;

/// Right sidebar panel with position, errors, I/O, and jog controls.
#[component]
pub fn RightPanel() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let jog_popped = layout_ctx.jog_popped;
    let robot_connected = ws.robot_connected;

    view! {
        <aside class="w-56 bg-[#0d0d0d] border-l border-[#ffffff08] flex flex-col overflow-hidden shrink-0">
            <div class="flex-1 overflow-y-auto p-1.5 space-y-1.5">
                // Robot status (compact)
                <RobotStatus/>

                // Position display
                <PositionDisplay/>

                // Errors panel
                <ErrorLog/>

                // I/O Status (only show when robot connected)
                <Show when=move || robot_connected.get()>
                    <IOStatusPanel/>
                </Show>

                // Jog controls (only show when robot connected and not popped)
                <Show when=move || robot_connected.get() && !jog_popped.get()>
                    <JogControlsPanel/>
                </Show>
            </div>
        </aside>
    }
}

/// I/O Status panel with DIN/DOUT/AIN/AOUT/GIN/GOUT support.
#[component]
fn IOStatusPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let (collapsed, set_collapsed) = signal(true);
    let (selected_tab, set_selected_tab) = signal::<&'static str>("din");

    // Default ports to display (1-8 for compact view)
    const DEFAULT_PORTS: [u16; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    // Get I/O config for display names
    let io_config = ws.io_config;

    // Helper to get display name for an I/O port
    let get_display_name = move |io_type: &str, port: u16| -> String {
        let config = io_config.get();
        if let Some(cfg) = config.get(&(io_type.to_string(), port as i32)) {
            if let Some(ref name) = cfg.display_name {
                return name.clone();
            }
        }
        port.to_string()
    };

    // Helper to check if a port is visible
    let is_port_visible = move |io_type: &str, port: u16| -> bool {
        let config = io_config.get();
        if let Some(cfg) = config.get(&(io_type.to_string(), port as i32)) {
            return cfg.is_visible;
        }
        true // Default to visible if no config
    };

    // Refresh I/O values when panel is opened
    let refresh_io = move || {
        // Clear cache first to ensure fresh values
        ws.clear_io_cache();
        // Read all DIN ports
        let ports: Vec<u16> = DEFAULT_PORTS.to_vec();
        ws.read_din_batch(ports);
        // Also read individual ports for DOUT (they may have different values)
        for &port in &DEFAULT_PORTS {
            ws.read_din(port);
        }
        // Read analog and group I/O
        for &port in &DEFAULT_PORTS {
            ws.read_ain(port);
            ws.read_gin(port);
        }
    };

    // Toggle collapse and refresh on open
    let toggle_collapse = move |_| {
        let was_collapsed = collapsed.get();
        set_collapsed.update(|v| *v = !*v);
        if was_collapsed {
            refresh_io();
        }
    };

    let din_values = ws.din_values;
    let dout_values = ws.dout_values;
    let ain_values = ws.ain_values;
    let aout_values = ws.aout_values;
    let gin_values = ws.gin_values;
    let gout_values = ws.gout_values;

    // Tab button helper
    let tab_class = move |tab: &'static str| {
        format!(
            "flex-1 text-[8px] py-1 rounded transition-colors {}",
            if selected_tab.get() == tab { "bg-[#00d9ff20] text-[#00d9ff]" } else { "bg-[#ffffff05] text-[#666666] hover:text-[#888888]" }
        )
    };

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08]">
            <button
                class="w-full flex items-center justify-between p-2 hover:bg-[#ffffff05] transition-colors"
                on:click=toggle_collapse
            >
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    "I/O"
                </h3>
                <svg
                    class={move || format!("w-3 h-3 text-[#666666] transition-transform {}", if collapsed.get() { "-rotate-90" } else { "" })}
                    fill="none" stroke="currentColor" viewBox="0 0 24 24"
                >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                </svg>
            </button>
            <Show when=move || !collapsed.get()>
                <div class="px-2 pb-2 space-y-2">
                    // Tab buttons - row 1: Digital
                    <div class="flex gap-1">
                        <button class={move || tab_class("din")} on:click=move |_| set_selected_tab.set("din")>"DIN"</button>
                        <button class={move || tab_class("dout")} on:click=move |_| set_selected_tab.set("dout")>"DOUT"</button>
                        <button class={move || tab_class("ain")} on:click=move |_| set_selected_tab.set("ain")>"AIN"</button>
                        <button class={move || tab_class("aout")} on:click=move |_| set_selected_tab.set("aout")>"AOUT"</button>
                        <button class={move || tab_class("gin")} on:click=move |_| set_selected_tab.set("gin")>"GIN"</button>
                        <button class={move || tab_class("gout")} on:click=move |_| set_selected_tab.set("gout")>"GOUT"</button>
                        <button
                            class="p-1 text-[#555555] hover:text-[#00d9ff] transition-colors"
                            title="Refresh I/O"
                            on:click=move |_| refresh_io()
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                            </svg>
                        </button>
                    </div>

                    // I/O grid - DIN
                    <Show when=move || selected_tab.get() == "din">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("DIN", port)).map(|&port| {
                                let name = get_display_name("DIN", port);
                                view! {
                                    <IOIndicator
                                        port=port
                                        name=name
                                        value=Signal::derive(move || din_values.get().get(&port).copied().unwrap_or(false))
                                    />
                                }
                            }).collect_view()}
                        </div>
                    </Show>

                    // I/O grid - DOUT
                    <Show when=move || selected_tab.get() == "dout">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("DOUT", port)).map(|&port| {
                                let name = get_display_name("DOUT", port);
                                view! {
                                    <IOButton
                                        port=port
                                        name=name
                                        value=Signal::derive(move || dout_values.get().get(&port).copied().unwrap_or(false))
                                    />
                                }
                            }).collect_view()}
                        </div>
                    </Show>

                    // I/O grid - AIN (analog input - read only)
                    <Show when=move || selected_tab.get() == "ain">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("AIN", port)).map(|&port| {
                                let name = get_display_name("AIN", port);
                                view! {
                                    <AnalogIndicator
                                        port=port
                                        name=name
                                        value=Signal::derive(move || ain_values.get().get(&port).copied().unwrap_or(0.0))
                                    />
                                }
                            }).collect_view()}
                        </div>
                    </Show>

                    // I/O grid - AOUT (analog output - writable)
                    <Show when=move || selected_tab.get() == "aout">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("AOUT", port)).map(|&port| {
                                let name = get_display_name("AOUT", port);
                                view! {
                                    <AnalogOutput
                                        port=port
                                        name=name
                                        value=Signal::derive(move || aout_values.get().get(&port).copied().unwrap_or(0.0))
                                    />
                                }
                            }).collect_view()}
                        </div>
                    </Show>

                    // I/O grid - GIN (group input - read only)
                    <Show when=move || selected_tab.get() == "gin">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("GIN", port)).map(|&port| {
                                let name = get_display_name("GIN", port);
                                view! {
                                    <GroupIndicator
                                        port=port
                                        name=name
                                        value=Signal::derive(move || gin_values.get().get(&port).copied().unwrap_or(0))
                                    />
                                }
                            }).collect_view()}
                        </div>
                    </Show>

                    // I/O grid - GOUT (group output - writable)
                    <Show when=move || selected_tab.get() == "gout">
                        <div class="grid grid-cols-4 gap-1">
                            {DEFAULT_PORTS.iter().filter(|&&port| is_port_visible("GOUT", port)).map(|&port| {
                                let name = get_display_name("GOUT", port);
                                view! {
                                    <GroupOutput
                                        port=port
                                        name=name
                                        value=Signal::derive(move || gout_values.get().get(&port).copied().unwrap_or(0))
                                    />
                                }
                            }).collect_view()}
                        </div>
                    </Show>
                </div>
            </Show>
        </div>
    }
}

/// Read-only I/O indicator (for DIN).
#[component]
fn IOIndicator(
    port: u16,
    name: String,
    value: Signal<bool>,
) -> impl IntoView {
    let display_name = name.clone();
    let title_name = name;
    view! {
        <div
            class={move || format!(
                "flex flex-col items-center justify-center p-1 rounded text-[8px] {}",
                if value.get() { "bg-[#00ff0020] text-[#00ff00]" } else { "bg-[#ffffff05] text-[#555555]" }
            )}
            title={format!("DIN[{}]", port)}
        >
            <span class="font-mono truncate max-w-full" title={title_name}>{display_name}</span>
            <div class={move || format!(
                "w-2 h-2 rounded-full mt-0.5 {}",
                if value.get() { "bg-[#00ff00]" } else { "bg-[#333333]" }
            )}/>
        </div>
    }
}

/// Clickable I/O button (for DOUT).
#[component]
fn IOButton(
    port: u16,
    name: String,
    value: Signal<bool>,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let display_name = name.clone();
    let title_name = name;

    let toggle = move |_| {
        let current = value.get();
        ws.write_dout(port, !current);
        // Optimistically update local state
        ws.update_dout_cache(port, !current);
    };

    view! {
        <button
            class={move || format!(
                "flex flex-col items-center justify-center p-1 rounded text-[8px] cursor-pointer transition-colors {}",
                if value.get() { "bg-[#ff880020] text-[#ff8800] hover:bg-[#ff880030]" } else { "bg-[#ffffff05] text-[#555555] hover:bg-[#ffffff10]" }
            )}
            title={format!("DOUT[{}] - Click to toggle", port)}
            on:click=toggle
        >
            <span class="font-mono truncate max-w-full" title={title_name}>{display_name}</span>
            <div class={move || format!(
                "w-2 h-2 rounded-full mt-0.5 {}",
                if value.get() { "bg-[#ff8800]" } else { "bg-[#333333]" }
            )}/>
        </button>
    }
}

/// Read-only analog input indicator (for AIN).
#[component]
fn AnalogIndicator(
    port: u16,
    name: String,
    value: Signal<f64>,
) -> impl IntoView {
    let display_name = name.clone();
    let title_name = name;
    view! {
        <div
            class="flex flex-col items-center justify-center p-1 rounded text-[8px] bg-[#ffffff05]"
            title={format!("AIN[{}]", port)}
        >
            <span class="font-mono text-[#00d9ff] truncate max-w-full" title={title_name}>{display_name}</span>
            <span class="font-mono text-[#888888] text-[7px]">
                {move || format!("{:.1}", value.get())}
            </span>
        </div>
    }
}

/// Analog output with input field (for AOUT).
#[component]
fn AnalogOutput(
    port: u16,
    name: String,
    value: Signal<f64>,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let display_name = name.clone();
    let title_name = name;
    let (editing, set_editing) = signal(false);
    let (input_value, set_input_value) = signal(String::new());

    let start_edit = move |_| {
        set_input_value.set(format!("{:.2}", value.get()));
        set_editing.set(true);
    };

    let do_submit = move || {
        if let Ok(new_val) = input_value.get().parse::<f64>() {
            ws.write_aout(port, new_val);
            ws.update_aout_cache(port, new_val);
        }
        set_editing.set(false);
    };

    view! {
        <div
            class="flex flex-col items-center justify-center p-1 rounded text-[8px] bg-[#ff880010] cursor-pointer hover:bg-[#ff880020]"
            title={format!("AOUT[{}] - Click to edit", port)}
        >
            <span class="font-mono text-[#ff8800] truncate max-w-full" title={title_name}>{display_name}</span>
            <Show when=move || !editing.get() fallback=move || view! {
                <input
                    type="text"
                    class="w-10 text-[7px] bg-[#1a1a1a] border border-[#ff8800] rounded px-0.5 text-center text-white"
                    prop:value=input_value
                    on:input=move |ev| set_input_value.set(event_target_value(&ev))
                    on:blur=move |_| do_submit()
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" { do_submit(); }
                        if ev.key() == "Escape" { set_editing.set(false); }
                    }
                />
            }>
                <span class="font-mono text-[#888888] text-[7px]" on:click=start_edit>
                    {move || format!("{:.1}", value.get())}
                </span>
            </Show>
        </div>
    }
}

/// Read-only group input indicator (for GIN).
#[component]
fn GroupIndicator(
    port: u16,
    name: String,
    value: Signal<u32>,
) -> impl IntoView {
    let display_name = name.clone();
    let title_name = name;
    view! {
        <div
            class="flex flex-col items-center justify-center p-1 rounded text-[8px] bg-[#ffffff05]"
            title={format!("GIN[{}]", port)}
        >
            <span class="font-mono text-[#00ff88] truncate max-w-full" title={title_name}>{display_name}</span>
            <span class="font-mono text-[#888888] text-[7px]">
                {move || format!("{}", value.get())}
            </span>
        </div>
    }
}

/// Group output with input field (for GOUT).
#[component]
fn GroupOutput(
    port: u16,
    name: String,
    value: Signal<u32>,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let display_name = name.clone();
    let title_name = name;
    let (editing, set_editing) = signal(false);
    let (input_value, set_input_value) = signal(String::new());

    let start_edit = move |_| {
        set_input_value.set(format!("{}", value.get()));
        set_editing.set(true);
    };

    let do_submit = move || {
        if let Ok(new_val) = input_value.get().parse::<u32>() {
            ws.write_gout(port, new_val);
            ws.update_gout_cache(port, new_val);
        }
        set_editing.set(false);
    };

    view! {
        <div
            class="flex flex-col items-center justify-center p-1 rounded text-[8px] bg-[#ff00ff10] cursor-pointer hover:bg-[#ff00ff20]"
            title={format!("GOUT[{}] - Click to edit", port)}
        >
            <span class="font-mono text-[#ff00ff] truncate max-w-full" title={title_name}>{display_name}</span>
            <Show when=move || !editing.get() fallback=move || view! {
                <input
                    type="text"
                    class="w-10 text-[7px] bg-[#1a1a1a] border border-[#ff00ff] rounded px-0.5 text-center text-white"
                    prop:value=input_value
                    on:input=move |ev| set_input_value.set(event_target_value(&ev))
                    on:blur=move |_| do_submit()
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" { do_submit(); }
                        if ev.key() == "Escape" { set_editing.set(false); }
                    }
                />
            }>
                <span class="font-mono text-[#888888] text-[7px]" on:click=start_edit>
                    {move || format!("{}", value.get())}
                </span>
            </Show>
        </div>
    }
}

/// Jog controls panel with pop-out button.
#[component]
fn JogControlsPanel() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");
    let jog_popped = layout_ctx.jog_popped;

    view! {
        <div class="relative">
            // Pop-out button
            <button
                class="absolute top-1.5 right-1.5 p-0.5 hover:bg-[#ffffff10] rounded z-10"
                title="Pop out jog controls"
                on:click=move |_| jog_popped.set(true)
            >
                <svg class="w-3 h-3 text-[#555555] hover:text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/>
                </svg>
            </button>

            <JogControls/>
        </div>
    }
}

/// Floating jog controls (when popped out).
#[component]
pub fn FloatingJogControls() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");
    let jog_popped = layout_ctx.jog_popped;
    let location = use_location();

    // Only show if popped AND in Dashboard route
    let is_dashboard = move || {
        let path = location.pathname.get();
        path == "/" || path.starts_with("/dashboard")
    };
    let should_show = move || jog_popped.get() && is_dashboard();

    // Create node ref for the HEADER (drag handle only)
    let header_el = NodeRef::<Div>::new();

    // Use leptos-use draggable hook on the header only
    let UseDraggableReturn {
        style,
        ..
    } = use_draggable_with_options(
        header_el,
        UseDraggableOptions::default()
            .initial_value(Position { x: 100.0, y: 100.0 })
            .prevent_default(true),
    );

    view! {
        <Show when=should_show>
            <div
                class="fixed bg-[#111111] rounded border border-[#00d9ff40] shadow-2xl z-50 select-none"
                style=move || format!("touch-action: none; min-width: 200px; {}", style.get())
            >
                // Header with close button - THIS is the drag handle
                <div
                    node_ref=header_el
                    class="flex items-center justify-between p-2 cursor-move border-b border-[#ffffff10]"
                >
                    <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center gap-1">
                        <svg class="w-3 h-3 text-[#555555]" fill="currentColor" viewBox="0 0 24 24">
                            <path d="M8 6h2v2H8V6zm6 0h2v2h-2V6zM8 11h2v2H8v-2zm6 0h2v2h-2v-2zm-6 5h2v2H8v-2zm6 0h2v2h-2v-2z"/>
                        </svg>
                        "Jog Controls"
                    </h3>
                    <button
                        class="p-0.5 hover:bg-[#ffffff10] rounded cursor-pointer"
                        title="Dock jog controls"
                        on:click=move |_| jog_popped.set(false)
                    >
                        <svg class="w-3 h-3 text-[#666666] hover:text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content area - NOT draggable, so inputs work normally
                <div class="p-2">
                    <JogControls/>
                </div>
            </div>
        </Show>
    }
}

