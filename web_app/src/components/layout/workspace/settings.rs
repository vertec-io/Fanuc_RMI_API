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

            // Robot Creation Wizard
            <Show when=move || show_add_robot.get()>
                <crate::components::RobotCreationWizard
                    on_close=move |_| set_show_add_robot.set(false)
                    on_created=move |id| {
                        set_show_add_robot.set(false);
                        set_selected_robot_id.set(Some(id));
                        ws.list_robot_connections();
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
                        "+ Create Robot"
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
    // Motion defaults (required - no global fallback)
    let (edit_speed, set_edit_speed) = signal::<String>("100.0".to_string());
    let (edit_speed_type, set_edit_speed_type) = signal::<String>("mmSec".to_string());
    let (edit_term, set_edit_term) = signal::<String>("CNT".to_string());
    let (edit_w, set_edit_w) = signal::<String>("0.0".to_string());
    let (edit_p, set_edit_p) = signal::<String>("0.0".to_string());
    let (edit_r, set_edit_r) = signal::<String>("0.0".to_string());
    // Jog defaults
    let (edit_cart_jog_speed, set_edit_cart_jog_speed) = signal::<String>("10.0".to_string());
    let (edit_cart_jog_step, set_edit_cart_jog_step) = signal::<String>("1.0".to_string());
    let (edit_joint_jog_speed, set_edit_joint_jog_speed) = signal::<String>("0.1".to_string());
    let (edit_joint_jog_step, set_edit_joint_jog_step) = signal::<String>("0.25".to_string());
    let (has_changes, set_has_changes) = signal(false);
    let (save_status, set_save_status) = signal::<Option<String>>(None);

    // Configuration management
    let (configurations, set_configurations) = signal::<Vec<crate::websocket::RobotConfigurationDto>>(Vec::new());
    let (selected_config_id, set_selected_config_id) = signal::<Option<i64>>(None);
    let (show_config_form, set_show_config_form) = signal(false);
    let (show_delete_config_confirm, set_show_delete_config_confirm) = signal(false);
    let (config_to_delete, set_config_to_delete) = signal::<Option<(i64, String)>>(None);
    let (editing_config_id, set_editing_config_id) = signal::<Option<i64>>(None);
    let (is_saving_config, set_is_saving_config) = signal(false);
    let (is_deleting_config, set_is_deleting_config) = signal(false);
    let (config_error_message, set_config_error_message) = signal::<Option<String>>(None);

    // Configuration form fields
    let (config_name, set_config_name) = signal::<String>(String::new());
    let (config_uframe, set_config_uframe) = signal::<String>("0".to_string());
    let (config_utool, set_config_utool) = signal::<String>("0".to_string());
    let (config_front, set_config_front) = signal::<String>("0".to_string());
    let (config_up, set_config_up) = signal::<String>("0".to_string());
    let (config_left, set_config_left) = signal::<String>("0".to_string());
    let (config_flip, set_config_flip) = signal::<String>("0".to_string());
    let (config_turn4, set_config_turn4) = signal::<String>("0".to_string());
    let (config_turn5, set_config_turn5) = signal::<String>("0".to_string());
    let (config_turn6, set_config_turn6) = signal::<String>("0".to_string());
    let (config_is_default, set_config_is_default) = signal(false);

    // Load robot data when selection changes
    Effect::new(move |_| {
        if let Some(robot) = selected_robot() {
            set_edit_name.set(robot.name.clone());
            set_edit_desc.set(robot.description.clone().unwrap_or_default());
            set_edit_ip.set(robot.ip_address.clone());
            set_edit_port.set(robot.port.to_string());
            // Motion defaults
            set_edit_speed.set(robot.default_speed.to_string());
            set_edit_speed_type.set(robot.default_speed_type.clone());
            set_edit_term.set(robot.default_term_type.clone());
            set_edit_w.set(robot.default_w.to_string());
            set_edit_p.set(robot.default_p.to_string());
            set_edit_r.set(robot.default_r.to_string());
            // Jog defaults
            set_edit_cart_jog_speed.set(robot.default_cartesian_jog_speed.to_string());
            set_edit_cart_jog_step.set(robot.default_cartesian_jog_step.to_string());
            set_edit_joint_jog_speed.set(robot.default_joint_jog_speed.to_string());
            set_edit_joint_jog_step.set(robot.default_joint_jog_step.to_string());
            set_has_changes.set(false);
            set_save_status.set(None);

            // Load configurations for this robot
            ws.list_robot_configurations(robot.id);
        }
    });

    // Subscribe to robot_configurations signal from WebSocket manager
    Effect::new(move |_| {
        let configs = ws.robot_configurations.get();
        set_configurations.set(configs);
    });

    // Handle configuration save success/error
    let (config_list_version, set_config_list_version) = signal(0u32);
    Effect::new(move |_| {
        // Increment version whenever the list changes
        let _ = configurations.get();
        set_config_list_version.update(|v| *v = v.wrapping_add(1));
    });

    Effect::new(move |prev_version: Option<u32>| {
        if is_saving_config.get() {
            // Check for errors
            if let Some(err) = ws.api_error.get() {
                set_is_saving_config.set(false);
                set_config_error_message.set(Some(err));
                ws.clear_api_error();
                return config_list_version.get();
            }

            // Check if the configuration list was updated (version changed)
            let current_version = config_list_version.get();
            if let Some(prev) = prev_version {
                if current_version != prev {
                    // List was updated, close the modal
                    set_is_saving_config.set(false);
                    set_show_config_form.set(false);
                    set_editing_config_id.set(None);
                    set_config_name.set(String::new());
                    set_config_uframe.set("0".to_string());
                    set_config_utool.set("0".to_string());
                    set_config_front.set("0".to_string());
                    set_config_up.set("0".to_string());
                    set_config_left.set("0".to_string());
                    set_config_flip.set("0".to_string());
                    set_config_turn4.set("0".to_string());
                    set_config_turn5.set("0".to_string());
                    set_config_turn6.set("0".to_string());
                    set_config_is_default.set(false);
                    set_config_error_message.set(None);
                }
            }
        }
        config_list_version.get()
    });

    // Handle configuration delete success/error
    Effect::new(move |_| {
        if is_deleting_config.get() {
            // Check for errors
            if let Some(err) = ws.api_error.get() {
                set_is_deleting_config.set(false);
                set_config_error_message.set(Some(err));
                ws.clear_api_error();
                return;
            }

            // Check if the configuration was removed from the list
            if let Some((id, _)) = config_to_delete.get() {
                let configs = configurations.get();
                if !configs.iter().any(|c| c.id == id) {
                    set_is_deleting_config.set(false);
                    set_show_delete_config_confirm.set(false);
                    set_config_to_delete.set(None);
                    set_config_error_message.set(None);
                }
            }
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

                                            // Parse numeric values from strings
                                            let speed: f64 = edit_speed.get().parse().unwrap_or(100.0);
                                            let w: f64 = edit_w.get().parse().unwrap_or(0.0);
                                            let p: f64 = edit_p.get().parse().unwrap_or(0.0);
                                            let r: f64 = edit_r.get().parse().unwrap_or(0.0);

                                            ws.update_robot_connection(id, name, description, ip, port);
                                            ws.update_robot_connection_defaults(
                                                id,
                                                speed,
                                                edit_speed_type.get(),
                                                edit_term.get(),
                                                w,
                                                p,
                                                r,
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
                                <div class="grid grid-cols-3 gap-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Default Speed"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_speed.get()
                                            on:input=move |ev| {
                                                set_edit_speed.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="100.0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5 flex items-center gap-1">
                                            <span>"Speed Type"</span>
                                            <span
                                                class="inline-flex items-center justify-center w-3 h-3 rounded-full bg-[#00d9ff] text-[#0a0a0a] text-[8px] font-bold"
                                                title="Speed Type determines how motion speed is interpreted:\n\n• mm/sec: Linear speed in millimeters per second (most common)\n• 0.1 inch/min: Linear speed in 0.1 inch per minute increments\n• 0.1 seconds: Time-based - motion completes in specified time (0.1 sec units)\n• milliseconds: Time-based - motion completes in specified milliseconds\n\nThis setting affects:\n✓ All motion commands (MOVE, MOVEJ, MOVEC)\n✓ Program execution\n✓ Quick commands from Dashboard\n\nNOTE: Individual commands can override this with their own speed/speedType parameters."
                                            >
                                                "?"
                                            </span>
                                        </label>
                                        <select
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            on:change=move |ev| {
                                                set_edit_speed_type.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                        >
                                            <option value="mmSec" selected=move || edit_speed_type.get() == "mmSec">"mm/sec (Linear)"</option>
                                            <option value="InchMin" selected=move || edit_speed_type.get() == "InchMin">"0.1 inch/min"</option>
                                            <option value="Time" selected=move || edit_speed_type.get() == "Time">"0.1 seconds (Time-based)"</option>
                                            <option value="mSec" selected=move || edit_speed_type.get() == "mSec">"milliseconds"</option>
                                        </select>
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
                                </div>
                            </div>

                            // Orientation Defaults
                            <div>
                                <h4 class="text-[10px] font-semibold text-[#888888] mb-2 uppercase tracking-wide">"Orientation Defaults (W, P, R)"</h4>
                                <div class="grid grid-cols-3 gap-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"W (deg)"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_w.get()
                                            on:input=move |ev| {
                                                set_edit_w.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="0.0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"P (deg)"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_p.get()
                                            on:input=move |ev| {
                                                set_edit_p.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="0.0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"R (deg)"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_r.get()
                                            on:input=move |ev| {
                                                set_edit_r.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="0.0"
                                        />
                                    </div>
                                </div>
                            </div>

                            // Jog Defaults
                            <div>
                                <h4 class="text-[10px] font-semibold text-[#888888] mb-2 uppercase tracking-wide">"Jog Defaults"</h4>
                                <div class="grid grid-cols-2 gap-3">
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Cartesian Jog Speed"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_cart_jog_speed.get()
                                            on:input=move |ev| {
                                                set_edit_cart_jog_speed.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="10.0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Cartesian Jog Step"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_cart_jog_step.get()
                                            on:input=move |ev| {
                                                set_edit_cart_jog_step.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="1.0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Joint Jog Speed"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_joint_jog_speed.get()
                                            on:input=move |ev| {
                                                set_edit_joint_jog_speed.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="0.1"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[9px] mb-0.5">"Joint Jog Step"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || edit_joint_jog_step.get()
                                            on:input=move |ev| {
                                                set_edit_joint_jog_step.set(event_target_value(&ev));
                                                set_has_changes.set(true);
                                            }
                                            placeholder="0.25"
                                        />
                                    </div>
                                </div>
                            </div>

                            // Configurations Section
                            <div>
                                <div class="flex items-center justify-between mb-2">
                                    <h4 class="text-[10px] font-semibold text-[#888888] uppercase tracking-wide">"Robot Configurations"</h4>
                                    <button
                                        class="text-[8px] px-2 py-0.5 bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] rounded hover:bg-[#22c55e30]"
                                        on:click=move |_| set_show_config_form.set(true)
                                    >
                                        "+ Add Configuration"
                                    </button>
                                </div>

                                // Configuration list
                                <div class="space-y-1.5">
                                    <For
                                        each=move || configurations.get()
                                        key=|config| config.id
                                        children=move |config| {
                                            let config_id = config.id;
                                            let is_selected = move || selected_config_id.get() == Some(config_id);

                                            // Helper to get current config data from signal
                                            let get_config = move || {
                                                configurations.get()
                                                    .into_iter()
                                                    .find(|c| c.id == config_id)
                                            };

                                            view! {
                                                <div
                                                    class=move || {
                                                        if is_selected() {
                                                            "bg-[#00d9ff10] border border-[#00d9ff40] rounded p-2 cursor-pointer hover:bg-[#00d9ff15]"
                                                        } else {
                                                            "bg-[#111111] border border-[#ffffff08] rounded p-2 cursor-pointer hover:bg-[#ffffff05]"
                                                        }
                                                    }
                                                    on:click=move |_| {
                                                        if is_selected() {
                                                            set_selected_config_id.set(None);
                                                        } else {
                                                            set_selected_config_id.set(Some(config_id));
                                                        }
                                                    }
                                                >
                                                    <div class="flex items-center justify-between">
                                                        <div class="flex-1 min-w-0">
                                                            <div class="flex items-center gap-1.5">
                                                                <span class="text-[9px] text-white font-medium">
                                                                    {move || get_config().map(|c| c.name.clone()).unwrap_or_default()}
                                                                </span>
                                                                {move || {
                                                                    if let Some(cfg) = get_config() {
                                                                        if cfg.is_default {
                                                                            view! {
                                                                                <span class="text-[8px] px-1.5 py-0.5 bg-[#fbbf2420] border border-[#fbbf2440] text-[#fbbf24] rounded">"DEFAULT"</span>
                                                                            }.into_any()
                                                                        } else {
                                                                            view! { <span></span> }.into_any()
                                                                        }
                                                                    } else {
                                                                        view! { <span></span> }.into_any()
                                                                    }
                                                                }}
                                                            </div>
                                                            <div class="text-[8px] text-[#666666] mt-0.5 font-mono">
                                                                {move || {
                                                                    get_config()
                                                                        .map(|c| format!("UFrame: {} | UTool: {}", c.u_frame_number, c.u_tool_number))
                                                                        .unwrap_or_default()
                                                                }}
                                                            </div>
                                                        </div>
                                                        <div class="flex gap-1 ml-2">
                                                            <button
                                                                class="text-[8px] px-1.5 py-0.5 text-[#00d9ff] hover:bg-[#00d9ff10] rounded"
                                                                title="Edit"
                                                                on:click=move |ev| {
                                                                    ev.stop_propagation();
                                                                    // Load configuration data into form
                                                                    if let Some(cfg) = get_config() {
                                                                        set_editing_config_id.set(Some(config_id));
                                                                        set_config_name.set(cfg.name.clone());
                                                                        set_config_uframe.set(cfg.u_frame_number.to_string());
                                                                        set_config_utool.set(cfg.u_tool_number.to_string());
                                                                        set_config_front.set(cfg.front.to_string());
                                                                        set_config_up.set(cfg.up.to_string());
                                                                        set_config_left.set(cfg.left.to_string());
                                                                        set_config_flip.set(cfg.flip.to_string());
                                                                        set_config_turn4.set(cfg.turn4.to_string());
                                                                        set_config_turn5.set(cfg.turn5.to_string());
                                                                        set_config_turn6.set(cfg.turn6.to_string());
                                                                        set_config_is_default.set(cfg.is_default);
                                                                        set_show_config_form.set(true);
                                                                    }
                                                                }
                                                            >
                                                                "✎"
                                                            </button>
                                                            {move || {
                                                                if let Some(cfg) = get_config() {
                                                                    if !cfg.is_default {
                                                                        view! {
                                                                            <button
                                                                                class="text-[8px] px-1.5 py-0.5 text-[#fbbf24] hover:bg-[#fbbf2410] rounded"
                                                                                title="Set as Default"
                                                                                on:click=move |ev| {
                                                                                    ev.stop_propagation();
                                                                                    ws.set_default_robot_configuration(config_id);
                                                                                    // Reload configurations
                                                                                    if let Some(robot) = selected_robot() {
                                                                                        ws.list_robot_configurations(robot.id);
                                                                                    }
                                                                                }
                                                                            >
                                                                                "⭐"
                                                                            </button>
                                                                        }.into_any()
                                                                    } else {
                                                                        view! { <span></span> }.into_any()
                                                                    }
                                                                } else {
                                                                    view! { <span></span> }.into_any()
                                                                }
                                                            }}
                                                            <button
                                                                class="text-[8px] px-1.5 py-0.5 text-[#ff4444] hover:bg-[#ff444410] rounded"
                                                                title="Delete"
                                                                on:click=move |ev| {
                                                                    ev.stop_propagation();
                                                                    if let Some(cfg) = get_config() {
                                                                        set_config_to_delete.set(Some((config_id, cfg.name.clone())));
                                                                        set_show_delete_config_confirm.set(true);
                                                                    }
                                                                }
                                                            >
                                                                "×"
                                                            </button>
                                                        </div>
                                                    </div>

                                                    // Configuration details (shown when selected)
                                                    <Show when=is_selected>
                                                        <div class="mt-2 pt-2 border-t border-[#ffffff08] space-y-1.5">
                                                            {move || {
                                                                if let Some(cfg) = get_config() {
                                                                    view! {
                                                                        <div class="grid grid-cols-3 gap-2 text-[8px]">
                                                                            <div>
                                                                                <span class="text-[#666666]">"Front: "</span>
                                                                                <span class="text-white font-mono">{cfg.front}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Up: "</span>
                                                                                <span class="text-white font-mono">{cfg.up}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Left: "</span>
                                                                                <span class="text-white font-mono">{cfg.left}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Flip: "</span>
                                                                                <span class="text-white font-mono">{cfg.flip}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Turn4: "</span>
                                                                                <span class="text-white font-mono">{cfg.turn4}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Turn5: "</span>
                                                                                <span class="text-white font-mono">{cfg.turn5}</span>
                                                                            </div>
                                                                            <div>
                                                                                <span class="text-[#666666]">"Turn6: "</span>
                                                                                <span class="text-white font-mono">{cfg.turn6}</span>
                                                                            </div>
                                                                        </div>
                                                                    }.into_any()
                                                                } else {
                                                                    view! { <div></div> }.into_any()
                                                                }
                                                            }}
                                                        </div>
                                                    </Show>
                                                </div>
                                            }
                                        }
                                    />
                                    {move || if configurations.get().is_empty() {
                                        view! {
                                            <div class="text-[8px] text-[#555555] text-center py-4 bg-[#111111] border border-[#ffffff08] rounded">
                                                "No configurations"<br/>
                                                "Click + Add to create one"
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! { <div></div> }.into_any()
                                    }}
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

            // Configuration Form Modal
            <Show when=move || show_config_form.get()>
                <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
                    <div class="bg-[#0a0a0a] border border-[#ffffff20] rounded-lg p-6 w-[500px] max-h-[80vh] overflow-y-auto">
                        <h3 class="text-[12px] font-semibold text-white mb-4">
                            {move || if editing_config_id.get().is_some() {
                                "Edit Configuration"
                            } else {
                                "New Configuration"
                            }}
                        </h3>

                        // Error message display
                        <Show when=move || config_error_message.get().is_some()>
                            <div class="bg-[#ff444420] border border-[#ff444440] rounded p-2 flex items-start gap-2 mb-3">
                                <svg class="w-4 h-4 text-[#ff4444] flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                </svg>
                                <span class="text-[10px] text-[#ff4444]">{move || config_error_message.get().unwrap_or_default()}</span>
                            </div>
                        </Show>

                        <div class="space-y-3">
                            // Configuration Name
                            <div>
                                <label class="block text-[#666666] text-[9px] mb-1">"Configuration Name"</label>
                                <input
                                    type="text"
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                    prop:value=move || config_name.get()
                                    on:input=move |ev| set_config_name.set(event_target_value(&ev))
                                    placeholder="e.g., Default Config, Welding Setup"
                                />
                            </div>

                            // UFrame and UTool
                            <div class="grid grid-cols-2 gap-3">
                                <div>
                                    <label class="block text-[#666666] text-[9px] mb-1">"User Frame (UFrame)"</label>
                                    <input
                                        type="text"
                                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                        prop:value=move || config_uframe.get()
                                        on:input=move |ev| set_config_uframe.set(event_target_value(&ev))
                                        placeholder="0"
                                    />
                                </div>
                                <div>
                                    <label class="block text-[#666666] text-[9px] mb-1">"User Tool (UTool)"</label>
                                    <input
                                        type="text"
                                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                        prop:value=move || config_utool.get()
                                        on:input=move |ev| set_config_utool.set(event_target_value(&ev))
                                        placeholder="0"
                                    />
                                </div>
                            </div>

                            // Arm Configuration
                            <div>
                                <label class="block text-[#666666] text-[9px] mb-1">"Arm Configuration"</label>
                                <div class="grid grid-cols-3 gap-2">
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Front"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_front.get()
                                            on:input=move |ev| set_config_front.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Up"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_up.get()
                                            on:input=move |ev| set_config_up.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Left"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_left.get()
                                            on:input=move |ev| set_config_left.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Flip"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_flip.get()
                                            on:input=move |ev| set_config_flip.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Turn4"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_turn4.get()
                                            on:input=move |ev| set_config_turn4.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Turn5"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_turn5.get()
                                            on:input=move |ev| set_config_turn5.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#555555] text-[8px] mb-0.5">"Turn6"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || config_turn6.get()
                                            on:input=move |ev| set_config_turn6.set(event_target_value(&ev))
                                            placeholder="0"
                                        />
                                    </div>
                                </div>
                            </div>

                            // Set as Default checkbox
                            <div class="flex items-center gap-2">
                                <input
                                    type="checkbox"
                                    id="config-is-default"
                                    class="w-3.5 h-3.5 bg-[#111111] border border-[#ffffff08] rounded"
                                    prop:checked=move || config_is_default.get()
                                    on:change=move |ev| set_config_is_default.set(event_target_checked(&ev))
                                />
                                <label for="config-is-default" class="text-[9px] text-[#888888]">"Set as default configuration"</label>
                            </div>
                        </div>

                        // Action buttons
                        <div class="flex gap-2 mt-4">
                            <button
                                class="flex-1 text-[10px] px-4 py-2 bg-[#00d9ff] text-[#0a0a0a] rounded hover:bg-[#00e5ff] font-medium disabled:opacity-50 disabled:cursor-not-allowed"
                                disabled=move || is_saving_config.get()
                                on:click=move |_| {
                                    if is_saving_config.get() {
                                        return;
                                    }

                                    let robot_id = match selected_robot_id.get() {
                                        Some(id) => id,
                                        None => {
                                            set_config_error_message.set(Some("No robot selected".to_string()));
                                            return;
                                        }
                                    };

                                    let name = config_name.get();
                                    if name.is_empty() {
                                        set_config_error_message.set(Some("Configuration name is required".to_string()));
                                        return;
                                    }

                                    let uframe: i32 = config_uframe.get().parse().unwrap_or(0);
                                    let utool: i32 = config_utool.get().parse().unwrap_or(0);
                                    let front: i32 = config_front.get().parse().unwrap_or(0);
                                    let up: i32 = config_up.get().parse().unwrap_or(0);
                                    let left: i32 = config_left.get().parse().unwrap_or(0);
                                    let flip: i32 = config_flip.get().parse().unwrap_or(0);
                                    let turn4: i32 = config_turn4.get().parse().unwrap_or(0);
                                    let turn5: i32 = config_turn5.get().parse().unwrap_or(0);
                                    let turn6: i32 = config_turn6.get().parse().unwrap_or(0);
                                    let is_default = config_is_default.get();

                                    // Clear any previous errors
                                    set_config_error_message.set(None);
                                    set_is_saving_config.set(true);

                                    if let Some(edit_id) = editing_config_id.get() {
                                        // Update existing configuration
                                        ws.update_robot_configuration(
                                            edit_id,
                                            name,
                                            is_default,
                                            uframe,
                                            utool,
                                            front,
                                            up,
                                            left,
                                            flip,
                                            turn4,
                                            turn5,
                                            turn6,
                                        );
                                    } else {
                                        // Create new configuration
                                        ws.create_robot_configuration(
                                            robot_id,
                                            name,
                                            is_default,
                                            uframe,
                                            utool,
                                            front,
                                            up,
                                            left,
                                            flip,
                                            turn4,
                                            turn5,
                                            turn6,
                                        );
                                    }

                                    // Reload configurations
                                    ws.list_robot_configurations(robot_id);

                                    // The Effect will handle closing the modal on success or showing error
                                }
                            >
                                {move || {
                                    if is_saving_config.get() {
                                        "Saving..."
                                    } else if editing_config_id.get().is_some() {
                                        "Update"
                                    } else {
                                        "Create"
                                    }
                                }}
                            </button>
                            <button
                                class="flex-1 text-[10px] px-4 py-2 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                                on:click=move |_| {
                                    set_show_config_form.set(false);
                                    set_editing_config_id.set(None);
                                    set_config_name.set(String::new());
                                    set_config_uframe.set("0".to_string());
                                    set_config_utool.set("0".to_string());
                                    set_config_front.set("0".to_string());
                                    set_config_up.set("0".to_string());
                                    set_config_left.set("0".to_string());
                                    set_config_flip.set("0".to_string());
                                    set_config_turn4.set("0".to_string());
                                    set_config_turn5.set("0".to_string());
                                    set_config_turn6.set("0".to_string());
                                    set_config_is_default.set(false);
                                }
                            >
                                "Cancel"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>

            // Delete Configuration Confirmation Modal
            <Show when=move || show_delete_config_confirm.get()>
                <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
                    <div class="bg-[#0a0a0a] border border-[#ff444440] rounded-lg p-6 w-[400px]">
                        <h3 class="text-[12px] font-semibold text-white mb-2">"Delete Configuration"</h3>
                        <p class="text-[10px] text-[#888888] mb-4">
                            "Are you sure you want to delete the configuration "
                            <span class="text-white font-medium">
                                {move || config_to_delete.get().map(|(_, name)| name).unwrap_or_default()}
                            </span>
                            "? This action cannot be undone."
                        </p>

                        <div class="flex gap-2">
                            <button
                                class="flex-1 text-[10px] px-4 py-2 bg-[#ff4444] text-white rounded hover:bg-[#ff5555] font-medium disabled:opacity-50 disabled:cursor-not-allowed"
                                disabled=move || is_deleting_config.get()
                                on:click=move |_| {
                                    if is_deleting_config.get() {
                                        return;
                                    }

                                    if let Some((id, _)) = config_to_delete.get() {
                                        set_is_deleting_config.set(true);
                                        ws.delete_robot_configuration(id);

                                        // Reload configurations
                                        if let Some(robot_id) = selected_robot_id.get() {
                                            ws.list_robot_configurations(robot_id);
                                        }

                                        // Clear selection if we deleted the selected config
                                        if selected_config_id.get() == Some(id) {
                                            set_selected_config_id.set(None);
                                        }
                                    }

                                    // The Effect will handle closing the modal on success
                                }
                            >
                                {move || if is_deleting_config.get() { "Deleting..." } else { "Delete" }}
                            </button>
                            <button
                                class="flex-1 text-[10px] px-4 py-2 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                                on:click=move |_| {
                                    set_show_delete_config_confirm.set(false);
                                    set_config_to_delete.set(None);
                                }
                            >
                                "Cancel"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
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