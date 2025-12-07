//! Settings module - Robot and connection configuration.
//!
//! This module uses a two-panel layout similar to the Program browser:
//! - Left sidebar: Robot browser (list of saved robots) + System Settings
//! - Right panel: Robot-specific settings for the selected robot
//!
//! Settings are organized into:
//! - System Settings: WebSocket server URL, display preferences
//! - Robot Settings: Per-robot motion defaults, orientation, I/O config

use leptos::prelude::*;
use crate::websocket::WebSocketManager;

/// Settings view with two-panel layout.
#[component]
pub fn SettingsView() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    // Selected robot for editing
    let (selected_robot_id, set_selected_robot_id) = signal::<Option<i64>>(None);

    // Modal states
    let (show_add_robot, set_show_add_robot) = signal(false);
    let (show_delete_confirm, set_show_delete_confirm) = signal(false);
    let (robot_to_delete, set_robot_to_delete) = signal::<Option<(i64, String)>>(None);

    // New robot form fields
    let (new_robot_name, set_new_robot_name) = signal(String::new());
    let (new_robot_desc, set_new_robot_desc) = signal(String::new());
    let (new_robot_ip, set_new_robot_ip) = signal("127.0.0.1".to_string());
    let (new_robot_port, set_new_robot_port) = signal("16001".to_string());

    let saved_connections = ws.robot_connections;

    // Load robot connections on mount
    Effect::new(move |_| {
        ws.list_robot_connections();
    });

    // Get selected robot details (using a derived signal instead of Memo since RobotConnectionDto doesn't impl PartialEq)
    let selected_robot = move || {
        let id = selected_robot_id.get()?;
        saved_connections.get().into_iter().find(|r| r.id == id)
    };

    view! {
        <div class="h-full flex flex-col">
            // Header
            <div class="h-8 border-b border-[#ffffff08] flex items-center px-3 shrink-0 bg-[#0d0d0d]">
                <h2 class="text-[11px] font-semibold text-white flex items-center">
                    <svg class="w-3.5 h-3.5 mr-1.5 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                    </svg>
                    "Settings"
                </h2>
            </div>

            // Main content - two panel layout
            <div class="flex-1 p-2 flex gap-2 min-h-0">
                // Left sidebar: Robot browser + System settings
                <RobotBrowser
                    saved_connections=saved_connections
                    selected_robot_id=selected_robot_id
                    set_selected_robot_id=set_selected_robot_id
                    set_show_add_robot=set_show_add_robot
                    set_show_delete_confirm=set_show_delete_confirm
                    set_robot_to_delete=set_robot_to_delete
                />

                // Right panel: Robot settings (or empty state)
                <RobotSettingsPanel
                    selected_robot=selected_robot
                    selected_robot_id=selected_robot_id
                />
            </div>

            // Add Robot Modal
            <Show when=move || show_add_robot.get()>
                <AddRobotModal
                    new_robot_name=new_robot_name
                    set_new_robot_name=set_new_robot_name
                    new_robot_desc=new_robot_desc
                    set_new_robot_desc=set_new_robot_desc
                    new_robot_ip=new_robot_ip
                    set_new_robot_ip=set_new_robot_ip
                    new_robot_port=new_robot_port
                    set_new_robot_port=set_new_robot_port
                    on_close=move || set_show_add_robot.set(false)
                    on_created=move |id| {
                        set_show_add_robot.set(false);
                        set_selected_robot_id.set(Some(id));
                    }
                />
            </Show>

            // Delete Confirmation Modal
            <Show when=move || show_delete_confirm.get()>
                <DeleteConfirmModal
                    robot_to_delete=robot_to_delete
                    set_show_delete_confirm=set_show_delete_confirm
                    set_robot_to_delete=set_robot_to_delete
                    set_selected_robot_id=set_selected_robot_id
                    selected_robot_id=selected_robot_id
                />
            </Show>
        </div>
    }
}

/// Left sidebar: Robot browser with list of saved robots + System Settings
#[component]
fn RobotBrowser(
    saved_connections: ReadSignal<Vec<crate::websocket::RobotConnectionDto>>,
    selected_robot_id: ReadSignal<Option<i64>>,
    set_selected_robot_id: WriteSignal<Option<i64>>,
    set_show_add_robot: WriteSignal<bool>,
    set_show_delete_confirm: WriteSignal<bool>,
    set_robot_to_delete: WriteSignal<Option<(i64, String)>>,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    view! {
        <div class="w-64 flex flex-col gap-2 shrink-0">
            // Robot Connections section
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col flex-1 min-h-0">
                <div class="flex items-center justify-between p-2 border-b border-[#ffffff08]">
                    <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01"/>
                        </svg>
                        "Robot Connections"
                    </h3>
                    <button
                        class="text-[8px] px-2 py-0.5 bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] rounded hover:bg-[#22c55e30]"
                        on:click=move |_| set_show_add_robot.set(true)
                    >
                        "+ Add"
                    </button>
                </div>

                // Robot list
                <div class="flex-1 overflow-y-auto p-1.5 space-y-1">
                    <For
                        each=move || saved_connections.get()
                        key=|conn| conn.id
                        children=move |conn| {
                            let conn_id = conn.id;
                            let conn_name = conn.name.clone();
                            let conn_name_for_delete = conn.name.clone();
                            let conn_ip = conn.ip_address.clone();
                            let conn_port = conn.port;

                            view! {
                                <div
                                    class=move || format!(
                                        "p-2 rounded border cursor-pointer transition-colors {}",
                                        if selected_robot_id.get() == Some(conn_id) {
                                            "bg-[#00d9ff10] border-[#00d9ff40]"
                                        } else {
                                            "bg-[#111111] border-[#ffffff08] hover:border-[#ffffff15]"
                                        }
                                    )
                                    on:click=move |_| set_selected_robot_id.set(Some(conn_id))
                                >
                                    <div class="flex items-center justify-between">
                                        <div class="flex-1 min-w-0">
                                            <div class="text-[9px] text-white font-medium truncate">
                                                {conn_name.clone()}
                                            </div>
                                            <div class="text-[8px] text-[#666666] font-mono">{format!("{}:{}", conn_ip, conn_port)}</div>
                                        </div>
                                        <div class="flex gap-1 ml-2">
                                            <button
                                                class="text-[8px] px-1.5 py-0.5 text-[#22c55e] hover:bg-[#22c55e10] rounded font-medium"
                                                title="Connect"
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    ws.connect_to_saved_robot(conn_id);
                                                }
                                            >
                                                "▶"
                                            </button>
                                            <button
                                                class="text-[8px] px-1.5 py-0.5 text-[#ff4444] hover:bg-[#ff444410] rounded"
                                                title="Delete"
                                                on:click=move |ev| {
                                                    ev.stop_propagation();
                                                    set_robot_to_delete.set(Some((conn_id, conn_name_for_delete.clone())));
                                                    set_show_delete_confirm.set(true);
                                                }
                                            >
                                                "×"
                                            </button>
                                        </div>
                                    </div>
                                </div>
                            }
                        }
                    />
                    {move || if saved_connections.get().is_empty() {
                        view! {
                            <div class="text-[8px] text-[#555555] text-center py-4">
                                "No saved robots"<br/>
                                "Click + Add to create one"
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}
                </div>
            </div>

            // System Settings section
            <SystemSettingsPanel />
        </div>
    }
}

/// System settings panel (global settings not tied to any robot)
#[component]
fn SystemSettingsPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let (confirm_reset, set_confirm_reset) = signal(false);

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h3 class="text-[10px] font-semibold text-[#888888] mb-2 uppercase tracking-wide flex items-center">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                </svg>
                "System"
            </h3>
            <div class="space-y-2 text-[9px]">
                <div class="flex items-center justify-between">
                    <span class="text-[#666666]">"Version"</span>
                    <span class="text-white font-mono">"0.8.0"</span>
                </div>
                <div class="flex items-center justify-between">
                    <span class="text-[#666666]">"RMI Protocol"</span>
                    <span class="text-white font-mono">"v5+"</span>
                </div>
                <div class="pt-2 border-t border-[#ffffff08]">
                    <Show
                        when=move || confirm_reset.get()
                        fallback=move || view! {
                            <button
                                class="w-full text-[8px] px-2 py-1 bg-[#ff444410] border border-[#ff444420] text-[#ff4444] rounded hover:bg-[#ff444420]"
                                on:click=move |_| set_confirm_reset.set(true)
                            >
                                "Reset Database"
                            </button>
                        }
                    >
                        <div class="space-y-1">
                            <p class="text-[8px] text-[#ff4444]">"Delete all data?"</p>
                            <div class="flex gap-1">
                                <button
                                    class="flex-1 text-[8px] px-2 py-1 bg-[#ff4444] text-white rounded hover:bg-[#ff5555]"
                                    on:click=move |_| {
                                        ws.reset_database();
                                        set_confirm_reset.set(false);
                                        ws.list_programs();
                                        ws.list_robot_connections();
                                    }
                                >
                                    "Yes"
                                </button>
                                <button
                                    class="flex-1 text-[8px] px-2 py-1 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                                    on:click=move |_| set_confirm_reset.set(false)
                                >
                                    "No"
                                </button>
                            </div>
                        </div>
                    </Show>
                </div>
            </div>
        </div>
    }
}

/// Right panel: Robot settings for selected robot (or empty state)
#[component]
fn RobotSettingsPanel<F>(
    selected_robot: F,
    selected_robot_id: ReadSignal<Option<i64>>,
) -> impl IntoView
where
    F: Fn() -> Option<crate::websocket::RobotConnectionDto> + Copy + Send + Sync + 'static,
{
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    // Editable fields for the selected robot
    let (edit_name, set_edit_name) = signal(String::new());
    let (edit_desc, set_edit_desc) = signal(String::new());
    let (edit_ip, set_edit_ip) = signal(String::new());
    let (edit_port, set_edit_port) = signal(String::new());
    // Per-robot defaults (all required - no global fallback)
    let (edit_speed, set_edit_speed) = signal::<f64>(50.0);
    let (edit_term, set_edit_term) = signal::<String>("CNT".to_string());
    let (edit_uframe, set_edit_uframe) = signal::<i32>(0);
    let (edit_utool, set_edit_utool) = signal::<i32>(1);
    let (edit_w, set_edit_w) = signal::<f64>(0.0);
    let (edit_p, set_edit_p) = signal::<f64>(0.0);
    let (edit_r, set_edit_r) = signal::<f64>(0.0);
    // Robot arm configuration defaults (all required)
    let (edit_front, set_edit_front) = signal::<i32>(1);
    let (edit_up, set_edit_up) = signal::<i32>(1);
    let (edit_left, set_edit_left) = signal::<i32>(0);
    let (edit_flip, set_edit_flip) = signal::<i32>(0);
    let (edit_turn4, set_edit_turn4) = signal::<i32>(0);
    let (edit_turn5, set_edit_turn5) = signal::<i32>(0);
    let (edit_turn6, set_edit_turn6) = signal::<i32>(0);
    let (has_changes, set_has_changes) = signal(false);
    let (save_status, set_save_status) = signal::<Option<String>>(None);

    // Load robot data when selection changes
    Effect::new(move |_| {
        if let Some(robot) = selected_robot() {
            set_edit_name.set(robot.name.clone());
            set_edit_desc.set(robot.description.clone().unwrap_or_default());
            set_edit_ip.set(robot.ip_address.clone());
            set_edit_port.set(robot.port.to_string());
            // All defaults are now required (no Option)
            set_edit_speed.set(robot.default_speed);
            set_edit_term.set(robot.default_term_type.clone());
            set_edit_uframe.set(robot.default_uframe);
            set_edit_utool.set(robot.default_utool);
            set_edit_w.set(robot.default_w);
            set_edit_p.set(robot.default_p);
            set_edit_r.set(robot.default_r);
            set_edit_front.set(robot.default_front);
            set_edit_up.set(robot.default_up);
            set_edit_left.set(robot.default_left);
            set_edit_flip.set(robot.default_flip);
            set_edit_turn4.set(robot.default_turn4);
            set_edit_turn5.set(robot.default_turn5);
            set_edit_turn6.set(robot.default_turn6);
            set_has_changes.set(false);
            set_save_status.set(None);
        }
    });

    view! {
        <div class="flex-1 bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col min-h-0">
            {move || {
                if let Some(robot) = selected_robot() {
                    let robot_name = robot.name.clone();
                    view! {
                        // Header with robot name
                        <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                            <h3 class="text-[11px] font-semibold text-white flex items-center">
                                <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                                </svg>
                                "Robot Settings: "
                                <span class="text-[#00d9ff] ml-1">{robot_name}</span>
                            </h3>
                            <div class="flex items-center gap-2">
                                {move || save_status.get().map(|s| view! {
                                    <span class="text-[9px] text-[#22c55e]">{s}</span>
                                })}
                                <button
                                    class=move || format!(
                                        "text-[9px] px-3 py-1 rounded transition-colors {}",
                                        if has_changes.get() {
                                            "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                                        } else {
                                            "bg-[#111111] border border-[#ffffff08] text-[#555555]"
                                        }
                                    )
                                    disabled=move || !has_changes.get()
                                    on:click=move |_| {
                                        if let Some(id) = selected_robot_id.get() {
                                            let name = edit_name.get();
                                            let desc = edit_desc.get();
                                            let ip = edit_ip.get();
                                            let port: u32 = edit_port.get().parse().unwrap_or(16001);
                                            let description = if desc.is_empty() { None } else { Some(desc) };

                                            ws.update_robot_connection(id, name, description, ip, port);
                                            ws.update_robot_connection_defaults(
                                                id,
                                                edit_speed.get(),
                                                edit_term.get(),
                                                edit_uframe.get(),
                                                edit_utool.get(),
                                                edit_w.get(),
                                                edit_p.get(),
                                                edit_r.get(),
                                                edit_front.get(),
                                                edit_up.get(),
                                                edit_left.get(),
                                                edit_flip.get(),
                                                edit_turn4.get(),
                                                edit_turn5.get(),
                                                edit_turn6.get(),
                                            );
                                            set_has_changes.set(false);
                                            set_save_status.set(Some("✓ Saved".to_string()));
                                            ws.list_robot_connections();
                                        }
                                    }
                                >
                                    "Save Changes"
                                </button>
                            </div>
                        </div>

                        // Settings content
                        <div class="flex-1 overflow-y-auto p-3 space-y-4">
                            // Connection Details
                            <div>
                                <h4 class="text-[10px] font-semibold text-[#888888] mb-2 uppercase tracking-wide">"Connection Details"</h4>
                                <div class="grid grid-cols-2 gap-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Name"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            prop:value=move || edit_name.get()
                                            on:input=move |ev| {
                                                set_edit_name.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Description"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            placeholder="Optional"
                                            prop:value=move || edit_desc.get()
                                            on:input=move |ev| {
                                                set_edit_desc.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"IP Address"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_ip.get()
                                            on:input=move |ev| {
                                                set_edit_ip.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Port"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_port.get()
                                            on:input=move |ev| {
                                                set_edit_port.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        />
                                    </div>
                                </div>
                            </div>

                            // Motion Defaults
                            <div>
                                <h4 class="text-[10px] font-semibold text-[#888888] mb-2 uppercase tracking-wide">"Motion Defaults"</h4>
                                <div class="grid grid-cols-4 gap-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Speed (mm/s)"</label>
                                        <input
                                            type="number"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            prop:value=move || edit_speed.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                    set_edit_speed.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Termination"</label>
                                        <select
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            on:change=move |ev| {
                                                set_edit_term.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        >
                                            <option value="CNT" selected=move || edit_term.get() == "CNT">"CNT"</option>
                                            <option value="FINE" selected=move || edit_term.get() == "FINE">"FINE"</option>
                                        </select>
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"UFrame"</label>
                                        <input
                                            type="number"
                                            min="0" max="9"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            prop:value=move || edit_uframe.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                                    set_edit_uframe.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"UTool"</label>
                                        <input
                                            type="number"
                                            min="0" max="9"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            prop:value=move || edit_utool.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                                    set_edit_utool.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        />
                                    </div>
                                </div>
                            </div>

                            // Orientation Defaults
                            <div>
                                <h4 class="text-[10px] font-semibold text-[#888888] mb-2 uppercase tracking-wide">"Orientation Defaults (W, P, R)"</h4>
                                <div class="grid grid-cols-3 gap-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"W (deg)"</label>
                                        <input
                                            type="number"
                                            step="0.1"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            prop:value=move || edit_w.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                    set_edit_w.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"P (deg)"</label>
                                        <input
                                            type="number"
                                            step="0.1"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            prop:value=move || edit_p.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                    set_edit_p.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"R (deg)"</label>
                                        <input
                                            type="number"
                                            step="0.1"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            prop:value=move || edit_r.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                                    set_edit_r.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        />
                                    </div>
                                </div>
                            </div>

                            // Robot Arm Configuration
                            <div>
                                <h4 class="text-[10px] font-semibold text-[#888888] mb-2 uppercase tracking-wide">"Robot Arm Configuration"</h4>
                                <div class="grid grid-cols-4 gap-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Front/Back"</label>
                                        <select
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            on:change=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                                    set_edit_front.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        >
                                            <option value="1" selected=move || edit_front.get() == 1>"Front"</option>
                                            <option value="0" selected=move || edit_front.get() == 0>"Back"</option>
                                        </select>
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Up/Down"</label>
                                        <select
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            on:change=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                                    set_edit_up.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        >
                                            <option value="1" selected=move || edit_up.get() == 1>"Up"</option>
                                            <option value="0" selected=move || edit_up.get() == 0>"Down"</option>
                                        </select>
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Left/Right"</label>
                                        <select
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            on:change=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                                    set_edit_left.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        >
                                            <option value="1" selected=move || edit_left.get() == 1>"Left"</option>
                                            <option value="0" selected=move || edit_left.get() == 0>"Right"</option>
                                        </select>
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Flip/NoFlip"</label>
                                        <select
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            on:change=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                                    set_edit_flip.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        >
                                            <option value="1" selected=move || edit_flip.get() == 1>"Flip"</option>
                                            <option value="0" selected=move || edit_flip.get() == 0>"NoFlip"</option>
                                        </select>
                                    </div>
                                </div>
                                <div class="grid grid-cols-3 gap-3 mt-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Turn4"</label>
                                        <input
                                            type="number"
                                            min="0" max="1"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            prop:value=move || edit_turn4.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                                    set_edit_turn4.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Turn5"</label>
                                        <input
                                            type="number"
                                            min="0" max="1"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            prop:value=move || edit_turn5.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                                    set_edit_turn5.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Turn6"</label>
                                        <input
                                            type="number"
                                            min="0" max="1"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            prop:value=move || edit_turn6.get().to_string()
                                            on:input=move |ev| {
                                                if let Ok(v) = event_target_value(&ev).parse::<i32>() {
                                                    set_edit_turn6.set(v);
                                                    set_has_changes.set(true);
                                                }
                                            }
                                        />
                                    </div>
                                </div>
                            </div>

                            // Quick Connect button
                            <div class="pt-2 border-t border-[#ffffff08]">
                                <button
                                    class="w-full text-[10px] px-4 py-2 bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] rounded hover:bg-[#22c55e30] font-medium"
                                    on:click=move |_| {
                                        if let Some(id) = selected_robot_id.get() {
                                            ws.connect_to_saved_robot(id);
                                        }
                                    }
                                >
                                    "▶ Connect to this Robot"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    // Empty state
                    view! {
                        <div class="flex-1 flex items-center justify-center">
                            <div class="text-center">
                                <svg class="w-12 h-12 mx-auto mb-3 text-[#333333]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                                </svg>
                                <p class="text-[11px] text-[#555555]">"Select a robot connection"</p>
                                <p class="text-[9px] text-[#444444] mt-1">"to view and edit its settings"</p>
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Modal for adding a new robot connection
#[component]
fn AddRobotModal<F1, F2>(
    new_robot_name: ReadSignal<String>,
    set_new_robot_name: WriteSignal<String>,
    new_robot_desc: ReadSignal<String>,
    set_new_robot_desc: WriteSignal<String>,
    new_robot_ip: ReadSignal<String>,
    set_new_robot_ip: WriteSignal<String>,
    new_robot_port: ReadSignal<String>,
    set_new_robot_port: WriteSignal<String>,
    on_close: F1,
    on_created: F2,
) -> impl IntoView
where
    F1: Fn() + Clone + 'static,
    F2: Fn(i64) + Clone + 'static,
{
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let on_close_clone = on_close.clone();

    view! {
        <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div class="bg-[#0d0d0d] border border-[#ffffff15] rounded-lg p-4 w-96 shadow-xl">
                <h3 class="text-[12px] font-semibold text-white mb-4 flex items-center">
                    <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"/>
                    </svg>
                    "Add Robot Connection"
                </h3>

                <div class="space-y-3">
                    <div>
                        <label class="block text-[#666666] text-[9px] mb-0.5">"Name"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                            placeholder="My Robot"
                            prop:value=move || new_robot_name.get()
                            on:input=move |ev| set_new_robot_name.set(event_target_value(&ev))
                        />
                    </div>
                    <div>
                        <label class="block text-[#666666] text-[9px] mb-0.5">"Description (optional)"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                            placeholder="Production cell #1"
                            prop:value=move || new_robot_desc.get()
                            on:input=move |ev| set_new_robot_desc.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="grid grid-cols-2 gap-3">
                        <div>
                            <label class="block text-[#666666] text-[9px] mb-0.5">"IP Address"</label>
                            <input
                                type="text"
                                class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                prop:value=move || new_robot_ip.get()
                                on:input=move |ev| set_new_robot_ip.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class="block text-[#666666] text-[9px] mb-0.5">"Port"</label>
                            <input
                                type="text"
                                class="w-full bg-[#111111] border border-[#ffffff08] rounded px-3 py-2 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                prop:value=move || new_robot_port.get()
                                on:input=move |ev| set_new_robot_port.set(event_target_value(&ev))
                            />
                        </div>
                    </div>
                </div>

                <div class="flex gap-2 mt-4">
                    <button
                        class="flex-1 text-[10px] px-4 py-2 bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] rounded hover:bg-[#22c55e30] font-medium"
                        on:click=move |_| {
                            let name = new_robot_name.get();
                            if name.is_empty() {
                                return;
                            }
                            let desc = new_robot_desc.get();
                            let ip = new_robot_ip.get();
                            let port: u32 = new_robot_port.get().parse().unwrap_or(16001);
                            let description = if desc.is_empty() { None } else { Some(desc) };

                            ws.create_robot_connection(name, description, ip, port);
                            ws.list_robot_connections();
                            // Note: We don't have the ID yet, so we just close
                            // The parent will need to select the new robot from the list
                            on_created(0);
                        }
                    >
                        "Create Robot"
                    </button>
                    <button
                        class="text-[10px] px-4 py-2 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                        on:click=move |_| on_close_clone()
                    >
                        "Cancel"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Modal for confirming robot deletion
#[component]
fn DeleteConfirmModal(
    robot_to_delete: ReadSignal<Option<(i64, String)>>,
    set_show_delete_confirm: WriteSignal<bool>,
    set_robot_to_delete: WriteSignal<Option<(i64, String)>>,
    set_selected_robot_id: WriteSignal<Option<i64>>,
    selected_robot_id: ReadSignal<Option<i64>>,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    view! {
        <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div class="bg-[#0d0d0d] border border-[#ff444440] rounded-lg p-4 w-80 shadow-xl">
                <h3 class="text-[12px] font-semibold text-[#ff4444] mb-3 flex items-center">
                    <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                    </svg>
                    "Delete Robot Connection"
                </h3>

                <p class="text-[10px] text-[#888888] mb-4">
                    "Are you sure you want to delete "
                    <span class="text-white font-medium">
                        "\""
                        {move || robot_to_delete.get().map(|(_, name)| name).unwrap_or_default()}
                        "\""
                    </span>
                    "? This action cannot be undone."
                </p>

                <div class="flex gap-2">
                    <button
                        class="flex-1 text-[10px] px-4 py-2 bg-[#ff4444] text-white rounded hover:bg-[#ff5555] font-medium"
                        on:click=move |_| {
                            if let Some((id, _)) = robot_to_delete.get() {
                                ws.delete_robot_connection(id);
                                ws.list_robot_connections();
                                // Clear selection if we deleted the selected robot
                                if selected_robot_id.get() == Some(id) {
                                    set_selected_robot_id.set(None);
                                }
                            }
                            set_show_delete_confirm.set(false);
                            set_robot_to_delete.set(None);
                        }
                    >
                        "Delete"
                    </button>
                    <button
                        class="flex-1 text-[10px] px-4 py-2 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                        on:click=move |_| {
                            set_show_delete_confirm.set(false);
                            set_robot_to_delete.set(None);
                        }
                    >
                        "Cancel"
                    </button>
                </div>
            </div>
        </div>
    }
}
