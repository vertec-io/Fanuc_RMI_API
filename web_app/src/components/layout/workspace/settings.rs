//! Settings module - Robot and connection configuration.
//!
//! This module contains components for:
//! - Connection settings (IP addresses, ports)
//! - Saved robot connections management
//! - Motion defaults (speed, termination, frame/tool)
//! - Default rotation values
//! - Display preferences

use leptos::prelude::*;
use crate::websocket::WebSocketManager;

/// Settings view.
#[component]
pub fn SettingsView() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    let (settings_changed, set_settings_changed) = signal(false);
    let (save_status, set_save_status) = signal::<Option<String>>(None);

    // Connection settings (local only, not persisted to database)
    let (robot_ip, set_robot_ip) = signal("127.0.0.1".to_string());
    let (web_server_ip, set_web_server_ip) = signal("127.0.0.1".to_string());
    let (ws_port, set_ws_port) = signal("9000".to_string());
    let (rmi_port, set_rmi_port) = signal("16001".to_string());

    // Robot defaults (synced with database)
    let (default_speed, set_default_speed) = signal(50.0f64);
    let (default_term, set_default_term) = signal("CNT".to_string());
    let (default_uframe, set_default_uframe) = signal(0i32);
    let (default_utool, set_default_utool) = signal(0i32);

    // Default rotation (W, P, R)
    let (default_w, set_default_w) = signal(0.0f64);
    let (default_p, set_default_p) = signal(0.0f64);
    let (default_r, set_default_r) = signal(0.0f64);

    // Display preferences (local only)
    let (show_mm, set_show_mm) = signal(true);
    let (show_degrees, set_show_degrees) = signal(true);
    let (compact_mode, set_compact_mode) = signal(false);

    // Saved robot connections management
    let (show_add_connection, set_show_add_connection) = signal(false);
    let (editing_connection_id, set_editing_connection_id) = signal::<Option<i64>>(None);
    let (new_conn_name, set_new_conn_name) = signal(String::new());
    let (new_conn_desc, set_new_conn_desc) = signal(String::new());
    let (new_conn_ip, set_new_conn_ip) = signal("127.0.0.1".to_string());
    let (new_conn_port, set_new_conn_port) = signal("16001".to_string());

    let saved_connections = ws.robot_connections;

    // Load settings on mount
    Effect::new(move |_| {
        ws.get_settings();
        ws.list_robot_connections();
    });

    // Update local state when settings are received from server
    Effect::new(move |_| {
        if let Some(settings) = ws.settings.get() {
            set_default_speed.set(settings.default_speed);
            set_default_term.set(settings.default_term_type.clone());
            set_default_uframe.set(settings.default_uframe);
            set_default_utool.set(settings.default_utool);
            set_default_w.set(settings.default_w);
            set_default_p.set(settings.default_p);
            set_default_r.set(settings.default_r);
            set_settings_changed.set(false);
        }
    });

    view! {
        <div class="h-full p-2 flex flex-col gap-2">
            // Header with save button
            <SettingsHeader
                settings_changed=settings_changed
                set_settings_changed=set_settings_changed
                save_status=save_status
                set_save_status=set_save_status
                default_w=default_w
                default_p=default_p
                default_r=default_r
                default_speed=default_speed
                default_term=default_term
                default_uframe=default_uframe
                default_utool=default_utool
                set_robot_ip=set_robot_ip
                set_web_server_ip=set_web_server_ip
                set_ws_port=set_ws_port
                set_rmi_port=set_rmi_port
                set_default_speed=set_default_speed
                set_default_term=set_default_term
                set_default_uframe=set_default_uframe
                set_default_utool=set_default_utool
                set_default_w=set_default_w
                set_default_p=set_default_p
                set_default_r=set_default_r
            />

            // Settings grid
            <div class="flex-1 overflow-auto">
                <div class="grid grid-cols-2 gap-2">
                    // Connection settings
                    <ConnectionSettingsPanel
                        robot_ip=robot_ip
                        set_robot_ip=set_robot_ip
                        web_server_ip=web_server_ip
                        set_web_server_ip=set_web_server_ip
                        ws_port=ws_port
                        set_ws_port=set_ws_port
                        rmi_port=rmi_port
                        set_rmi_port=set_rmi_port
                        set_settings_changed=set_settings_changed
                    />

                    // Saved Robot Connections
                    <SavedConnectionsPanel
                        saved_connections=saved_connections
                        show_add_connection=show_add_connection
                        set_show_add_connection=set_show_add_connection
                        editing_connection_id=editing_connection_id
                        set_editing_connection_id=set_editing_connection_id
                        new_conn_name=new_conn_name
                        set_new_conn_name=set_new_conn_name
                        new_conn_desc=new_conn_desc
                        set_new_conn_desc=set_new_conn_desc
                        new_conn_ip=new_conn_ip
                        set_new_conn_ip=set_new_conn_ip
                        new_conn_port=new_conn_port
                        set_new_conn_port=set_new_conn_port
                    />

                    // Robot defaults
                    <MotionDefaultsPanel
                        default_speed=default_speed
                        set_default_speed=set_default_speed
                        default_term=default_term
                        set_default_term=set_default_term
                        default_uframe=default_uframe
                        set_default_uframe=set_default_uframe
                        default_utool=default_utool
                        set_default_utool=set_default_utool
                        set_settings_changed=set_settings_changed
                    />

                    // Default rotation
                    <DefaultRotationPanel
                        default_w=default_w
                        set_default_w=set_default_w
                        default_p=default_p
                        set_default_p=set_default_p
                        default_r=default_r
                        set_default_r=set_default_r
                        set_settings_changed=set_settings_changed
                    />

                    // Display preferences
                    <DisplayPreferencesPanel
                        show_mm=show_mm
                        set_show_mm=set_show_mm
                        show_degrees=show_degrees
                        set_show_degrees=set_show_degrees
                        compact_mode=compact_mode
                        set_compact_mode=set_compact_mode
                        set_settings_changed=set_settings_changed
                    />

                    // About / Info panel
                    <AboutPanel />

                    // Danger Zone panel
                    <DangerZonePanel />
                </div>
            </div>
        </div>
    }
}

/// Settings header with save/reset buttons
#[component]
fn SettingsHeader(
    settings_changed: ReadSignal<bool>,
    set_settings_changed: WriteSignal<bool>,
    save_status: ReadSignal<Option<String>>,
    set_save_status: WriteSignal<Option<String>>,
    default_w: ReadSignal<f64>,
    default_p: ReadSignal<f64>,
    default_r: ReadSignal<f64>,
    default_speed: ReadSignal<f64>,
    default_term: ReadSignal<String>,
    default_uframe: ReadSignal<i32>,
    default_utool: ReadSignal<i32>,
    set_robot_ip: WriteSignal<String>,
    set_web_server_ip: WriteSignal<String>,
    set_ws_port: WriteSignal<String>,
    set_rmi_port: WriteSignal<String>,
    set_default_speed: WriteSignal<f64>,
    set_default_term: WriteSignal<String>,
    set_default_uframe: WriteSignal<i32>,
    set_default_utool: WriteSignal<i32>,
    set_default_w: WriteSignal<f64>,
    set_default_p: WriteSignal<f64>,
    set_default_r: WriteSignal<f64>,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    view! {
        <div class="flex items-center justify-between bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-xs font-semibold text-white flex items-center">
                <svg class="w-3.5 h-3.5 mr-1.5 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                </svg>
                "Settings"
            </h2>
            <div class="flex items-center gap-2">
                {move || save_status.get().map(|s| view! {
                    <span class="text-[9px] text-[#22c55e]">{s}</span>
                })}
                <button
                    class={move || format!(
                        "text-[9px] px-3 py-1 rounded transition-colors {}",
                        if settings_changed.get() {
                            "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                        } else {
                            "bg-[#111111] border border-[#ffffff08] text-[#555555]"
                        }
                    )}
                    disabled=move || !settings_changed.get()
                    on:click=move |_| {
                        ws.update_settings(
                            default_w.get(),
                            default_p.get(),
                            default_r.get(),
                            default_speed.get(),
                            default_term.get(),
                            default_uframe.get(),
                            default_utool.get(),
                        );
                        set_save_status.set(Some("✓ Settings saved".to_string()));
                        set_settings_changed.set(false);
                    }
                >
                    "Save Settings"
                </button>
                <button
                    class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[9px] px-3 py-1 rounded"
                    on:click=move |_| {
                        set_robot_ip.set("127.0.0.1".to_string());
                        set_web_server_ip.set("127.0.0.1".to_string());
                        set_ws_port.set("9000".to_string());
                        set_rmi_port.set("16001".to_string());
                        set_default_speed.set(50.0);
                        set_default_term.set("CNT".to_string());
                        set_default_uframe.set(0);
                        set_default_utool.set(0);
                        set_default_w.set(0.0);
                        set_default_p.set(0.0);
                        set_default_r.set(0.0);
                        ws.update_settings(0.0, 0.0, 0.0, 50.0, "CNT".to_string(), 0, 0);
                        set_settings_changed.set(false);
                        set_save_status.set(Some("✓ Reset to defaults".to_string()));
                    }
                >
                    "Reset to Defaults"
                </button>
            </div>
        </div>
    }
}

/// Connection settings panel
#[component]
fn ConnectionSettingsPanel(
    robot_ip: ReadSignal<String>,
    set_robot_ip: WriteSignal<String>,
    web_server_ip: ReadSignal<String>,
    set_web_server_ip: WriteSignal<String>,
    ws_port: ReadSignal<String>,
    set_ws_port: WriteSignal<String>,
    rmi_port: ReadSignal<String>,
    set_rmi_port: WriteSignal<String>,
    set_settings_changed: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3">
            <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0"/>
                </svg>
                "Connection"
            </h3>
            <div class="space-y-2">
                <div class="grid grid-cols-2 gap-2">
                    <div>
                        <label class="block text-[#666666] text-[9px] mb-0.5">"Robot IP Address"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                            prop:value=move || robot_ip.get()
                            on:input=move |ev| {
                                set_robot_ip.set(event_target_value(&ev));
                                set_settings_changed.set(true);
                            }
                        />
                        <p class="text-[8px] text-[#555555] mt-0.5">"Robot controller IP"</p>
                    </div>
                    <div>
                        <label class="block text-[#666666] text-[9px] mb-0.5">"Web Server IP"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                            prop:value=move || web_server_ip.get()
                            on:input=move |ev| {
                                set_web_server_ip.set(event_target_value(&ev));
                                set_settings_changed.set(true);
                            }
                        />
                        <p class="text-[8px] text-[#555555] mt-0.5">"WebSocket server IP"</p>
                    </div>
                </div>
                <div class="grid grid-cols-2 gap-2">
                    <div>
                        <label class="block text-[#666666] text-[9px] mb-0.5">"WebSocket Port"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                            prop:value=move || ws_port.get()
                            on:input=move |ev| {
                                set_ws_port.set(event_target_value(&ev));
                                set_settings_changed.set(true);
                            }
                        />
                    </div>
                    <div>
                        <label class="block text-[#666666] text-[9px] mb-0.5">"RMI Port"</label>
                        <input
                            type="text"
                            class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                            prop:value=move || rmi_port.get()
                            on:input=move |ev| {
                                set_rmi_port.set(event_target_value(&ev));
                                set_settings_changed.set(true);
                            }
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}

/// Saved robot connections panel
#[component]
fn SavedConnectionsPanel(
    saved_connections: ReadSignal<Vec<crate::websocket::RobotConnectionDto>>,
    show_add_connection: ReadSignal<bool>,
    set_show_add_connection: WriteSignal<bool>,
    editing_connection_id: ReadSignal<Option<i64>>,
    set_editing_connection_id: WriteSignal<Option<i64>>,
    new_conn_name: ReadSignal<String>,
    set_new_conn_name: WriteSignal<String>,
    new_conn_desc: ReadSignal<String>,
    set_new_conn_desc: WriteSignal<String>,
    new_conn_ip: ReadSignal<String>,
    set_new_conn_ip: WriteSignal<String>,
    new_conn_port: ReadSignal<String>,
    set_new_conn_port: WriteSignal<String>,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3">
            <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center justify-between">
                <span class="flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01"/>
                    </svg>
                    "Saved Connections"
                </span>
                <button
                    class="text-[8px] px-2 py-0.5 bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] rounded hover:bg-[#22c55e30]"
                    on:click=move |_| {
                        set_show_add_connection.set(true);
                        set_editing_connection_id.set(None);
                        set_new_conn_name.set(String::new());
                        set_new_conn_desc.set(String::new());
                        set_new_conn_ip.set("127.0.0.1".to_string());
                        set_new_conn_port.set("16001".to_string());
                    }
                >
                    "+ Add"
                </button>
            </h3>

            // Add/Edit form
            <Show when=move || show_add_connection.get()>
                <div class="mb-2 p-2 bg-[#111111] rounded border border-[#ffffff10]">
                    <div class="grid grid-cols-2 gap-2 mb-2">
                        <div>
                            <label class="block text-[#666666] text-[8px] mb-0.5">"Name"</label>
                            <input
                                type="text"
                                class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none"
                                placeholder="My Robot"
                                prop:value=move || new_conn_name.get()
                                on:input=move |ev| set_new_conn_name.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class="block text-[#666666] text-[8px] mb-0.5">"Description"</label>
                            <input
                                type="text"
                                class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none"
                                placeholder="Optional"
                                prop:value=move || new_conn_desc.get()
                                on:input=move |ev| set_new_conn_desc.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class="block text-[#666666] text-[8px] mb-0.5">"IP Address"</label>
                            <input
                                type="text"
                                class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                prop:value=move || new_conn_ip.get()
                                on:input=move |ev| set_new_conn_ip.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class="block text-[#666666] text-[8px] mb-0.5">"Port"</label>
                            <input
                                type="text"
                                class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                prop:value=move || new_conn_port.get()
                                on:input=move |ev| set_new_conn_port.set(event_target_value(&ev))
                            />
                        </div>
                    </div>
                    <div class="flex gap-1">
                        <button
                            class="flex-1 text-[8px] px-2 py-1 bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] rounded hover:bg-[#22c55e30]"
                            on:click=move |_| {
                                let name = new_conn_name.get();
                                let desc = new_conn_desc.get();
                                let ip = new_conn_ip.get();
                                let port: u32 = new_conn_port.get().parse().unwrap_or(16001);
                                let description = if desc.is_empty() { None } else { Some(desc) };

                                if let Some(id) = editing_connection_id.get() {
                                    ws.update_robot_connection(id, name, description, ip, port);
                                } else {
                                    ws.create_robot_connection(name, description, ip, port);
                                }
                                set_show_add_connection.set(false);
                                ws.list_robot_connections();
                            }
                        >
                            {move || if editing_connection_id.get().is_some() { "Update" } else { "Save" }}
                        </button>
                        <button
                            class="text-[8px] px-2 py-1 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                            on:click=move |_| set_show_add_connection.set(false)
                        >
                            "Cancel"
                        </button>
                    </div>
                </div>
            </Show>

            // Connections list
            <div class="space-y-1 max-h-32 overflow-y-auto">
                <For
                    each=move || saved_connections.get()
                    key=|conn| conn.id
                    children=move |conn| {
                        let conn_id = conn.id;
                        let conn_name = conn.name.clone();
                        let conn_desc = conn.description.clone();
                        let conn_ip = conn.ip_address.clone();
                        let conn_port = conn.port;
                        view! {
                            <div class="flex items-center justify-between p-1.5 bg-[#111111] rounded border border-[#ffffff08] hover:border-[#ffffff15]">
                                <div class="flex-1 min-w-0">
                                    <div class="text-[9px] text-white font-medium truncate">{conn_name.clone()}</div>
                                    <div class="text-[8px] text-[#666666] font-mono">{format!("{}:{}", conn_ip.clone(), conn_port)}</div>
                                </div>
                                <div class="flex gap-1 ml-2">
                                    <button
                                        class="text-[8px] px-1.5 py-0.5 text-[#00d9ff] hover:bg-[#00d9ff10] rounded"
                                        title="Edit"
                                        on:click=move |_| {
                                            set_editing_connection_id.set(Some(conn_id));
                                            set_new_conn_name.set(conn_name.clone());
                                            set_new_conn_desc.set(conn_desc.clone().unwrap_or_default());
                                            set_new_conn_ip.set(conn_ip.clone());
                                            set_new_conn_port.set(conn_port.to_string());
                                            set_show_add_connection.set(true);
                                        }
                                    >
                                        "Edit"
                                    </button>
                                    <button
                                        class="text-[8px] px-1.5 py-0.5 text-[#ff4444] hover:bg-[#ff444410] rounded"
                                        title="Delete"
                                        on:click=move |_| {
                                            ws.delete_robot_connection(conn_id);
                                            ws.list_robot_connections();
                                        }
                                    >
                                        "×"
                                    </button>
                                </div>
                            </div>
                        }
                    }
                />
                {move || if saved_connections.get().is_empty() {
                    view! {
                        <div class="text-[8px] text-[#555555] text-center py-2">"No saved connections"</div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }}
            </div>
        </div>
    }
}

/// Motion defaults panel
#[component]
fn MotionDefaultsPanel(
    default_speed: ReadSignal<f64>,
    set_default_speed: WriteSignal<f64>,
    default_term: ReadSignal<String>,
    set_default_term: WriteSignal<String>,
    default_uframe: ReadSignal<i32>,
    set_default_uframe: WriteSignal<i32>,
    default_utool: ReadSignal<i32>,
    set_default_utool: WriteSignal<i32>,
    set_settings_changed: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3">
            <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                </svg>
                "Motion Defaults"
            </h3>
            <div class="grid grid-cols-2 gap-2">
                <div>
                    <label class="block text-[#666666] text-[9px] mb-0.5">"Speed (mm/s)"</label>
                    <input
                        type="number"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                        prop:value=move || default_speed.get()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse() {
                                set_default_speed.set(v);
                                set_settings_changed.set(true);
                            }
                        }
                    />
                </div>
                <div>
                    <label class="block text-[#666666] text-[9px] mb-0.5">"Termination"</label>
                    <select
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                        on:change=move |ev| {
                            set_default_term.set(event_target_value(&ev));
                            set_settings_changed.set(true);
                        }
                    >
                        <option value="CNT" selected=move || default_term.get() == "CNT">"CNT (Continuous)"</option>
                        <option value="FINE" selected=move || default_term.get() == "FINE">"FINE (Stop)"</option>
                    </select>
                </div>
                <div>
                    <label class="block text-[#666666] text-[9px] mb-0.5">"UFrame"</label>
                    <input
                        type="number"
                        min="0" max="9"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                        prop:value=move || default_uframe.get()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse() {
                                set_default_uframe.set(v);
                                set_settings_changed.set(true);
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
                        prop:value=move || default_utool.get()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse() {
                                set_default_utool.set(v);
                                set_settings_changed.set(true);
                            }
                        }
                    />
                </div>
            </div>
        </div>
    }
}

/// Default rotation panel
#[component]
fn DefaultRotationPanel(
    default_w: ReadSignal<f64>,
    set_default_w: WriteSignal<f64>,
    default_p: ReadSignal<f64>,
    set_default_p: WriteSignal<f64>,
    default_r: ReadSignal<f64>,
    set_default_r: WriteSignal<f64>,
    set_settings_changed: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3">
            <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                </svg>
                "Default Rotation"
            </h3>
            <p class="text-[8px] text-[#555555] mb-2">"Used when rotation not specified in CSV"</p>
            <div class="grid grid-cols-3 gap-2">
                <div>
                    <label class="block text-[#666666] text-[9px] mb-0.5">"W (deg)"</label>
                    <input
                        type="number"
                        step="0.1"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                        prop:value=move || default_w.get()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse() {
                                set_default_w.set(v);
                                set_settings_changed.set(true);
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
                        prop:value=move || default_p.get()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse() {
                                set_default_p.set(v);
                                set_settings_changed.set(true);
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
                        prop:value=move || default_r.get()
                        on:input=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse() {
                                set_default_r.set(v);
                                set_settings_changed.set(true);
                            }
                        }
                    />
                </div>
            </div>
        </div>
    }
}

/// Display preferences panel
#[component]
fn DisplayPreferencesPanel(
    show_mm: ReadSignal<bool>,
    set_show_mm: WriteSignal<bool>,
    show_degrees: ReadSignal<bool>,
    set_show_degrees: WriteSignal<bool>,
    compact_mode: ReadSignal<bool>,
    set_compact_mode: WriteSignal<bool>,
    set_settings_changed: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3">
            <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
                </svg>
                "Display Preferences"
            </h3>
            <div class="space-y-2">
                <label class="flex items-center gap-2 text-[9px] text-[#888888] cursor-pointer">
                    <input
                        type="checkbox"
                        class="accent-[#00d9ff]"
                        prop:checked=move || show_mm.get()
                        on:change=move |ev| {
                            set_show_mm.set(event_target_checked(&ev));
                            set_settings_changed.set(true);
                        }
                    />
                    "Show position in millimeters"
                </label>
                <label class="flex items-center gap-2 text-[9px] text-[#888888] cursor-pointer">
                    <input
                        type="checkbox"
                        class="accent-[#00d9ff]"
                        prop:checked=move || show_degrees.get()
                        on:change=move |ev| {
                            set_show_degrees.set(event_target_checked(&ev));
                            set_settings_changed.set(true);
                        }
                    />
                    "Show angles in degrees"
                </label>
                <label class="flex items-center gap-2 text-[9px] text-[#888888] cursor-pointer">
                    <input
                        type="checkbox"
                        class="accent-[#00d9ff]"
                        prop:checked=move || compact_mode.get()
                        on:change=move |ev| {
                            set_compact_mode.set(event_target_checked(&ev));
                            set_settings_changed.set(true);
                        }
                    />
                    "Compact mode (smaller text and spacing)"
                </label>
            </div>
        </div>
    }
}

/// About panel
#[component]
fn AboutPanel() -> impl IntoView {
    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3 col-span-2">
            <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                "About"
            </h3>
            <div class="grid grid-cols-3 gap-4 text-[9px]">
                <div>
                    <span class="text-[#555555]">"Version:"</span>
                    <span class="text-white ml-1">"0.1.0"</span>
                </div>
                <div>
                    <span class="text-[#555555]">"RMI Protocol:"</span>
                    <span class="text-white ml-1">"v5+"</span>
                </div>
                <div>
                    <span class="text-[#555555]">"Database:"</span>
                    <span class="text-white ml-1">"SQLite"</span>
                </div>
            </div>
        </div>
    }
}

/// Danger Zone panel - destructive operations
#[component]
fn DangerZonePanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let (confirm_reset, set_confirm_reset) = signal(false);
    let (reset_status, set_reset_status) = signal::<Option<String>>(None);

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ff444440] p-3 col-span-2">
            <h3 class="text-[10px] font-semibold text-[#ff4444] mb-2 uppercase tracking-wide flex items-center">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                </svg>
                "Danger Zone"
            </h3>
            <p class="text-[8px] text-[#888888] mb-3">"These actions are destructive and cannot be undone."</p>

            <div class="flex items-center justify-between p-2 bg-[#111111] rounded border border-[#ff444420]">
                <div>
                    <div class="text-[9px] text-white font-medium">"Reset Database"</div>
                    <div class="text-[8px] text-[#666666]">"Delete all programs, settings, and saved connections"</div>
                </div>
                <div class="flex items-center gap-2">
                    {move || reset_status.get().map(|s| view! {
                        <span class="text-[8px] text-[#22c55e]">{s}</span>
                    })}
                    <Show
                        when=move || confirm_reset.get()
                        fallback=move || view! {
                            <button
                                class="text-[8px] px-3 py-1 bg-[#ff444420] border border-[#ff444440] text-[#ff4444] rounded hover:bg-[#ff444430]"
                                on:click=move |_| set_confirm_reset.set(true)
                            >
                                "Reset Database"
                            </button>
                        }
                    >
                        <div class="flex items-center gap-1">
                            <span class="text-[8px] text-[#ff4444]">"Are you sure?"</span>
                            <button
                                class="text-[8px] px-2 py-1 bg-[#ff4444] text-white rounded hover:bg-[#ff5555]"
                                on:click=move |_| {
                                    ws.reset_database();
                                    set_confirm_reset.set(false);
                                    set_reset_status.set(Some("✓ Database reset".to_string()));
                                    // Refresh data
                                    ws.list_programs();
                                    ws.get_settings();
                                    ws.list_robot_connections();
                                }
                            >
                                "Yes, Reset"
                            </button>
                            <button
                                class="text-[8px] px-2 py-1 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                                on:click=move |_| set_confirm_reset.set(false)
                            >
                                "Cancel"
                            </button>
                        </div>
                    </Show>
                </div>
            </div>
        </div>
    }
}

fn event_target_checked(ev: &leptos::ev::Event) -> bool {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|e| e.checked())
        .unwrap_or(false)
}
