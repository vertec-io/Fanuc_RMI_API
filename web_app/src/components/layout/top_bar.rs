//! Top bar / header component.

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use crate::websocket::WebSocketManager;
use super::LayoutContext;

/// Top bar with connection status, robot info, and settings.
#[component]
pub fn TopBar() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let _layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");
    let ws_connected = ws.connected;
    let robot_connected = ws.robot_connected;
    let robot_addr = ws.robot_addr;

    // State for connection dropdown
    let (show_connection_menu, set_show_connection_menu) = signal(false);

    // Fetch connection status on mount
    Effect::new(move |_| {
        if ws_connected.get() {
            ws.get_connection_status();
        }
    });

    view! {
        <header class="h-9 bg-[#111111] border-b border-[#ffffff10] flex items-center px-3 shrink-0">
            // Logo and title
            <div class="flex items-center space-x-2">
                <div class="w-6 h-6 bg-[#00d9ff] rounded flex items-center justify-center">
                    <svg class="w-3.5 h-3.5 text-black" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                    </svg>
                </div>
                <h1 class="text-xs font-semibold text-white">"FANUC RMI"</h1>
                <span class="text-[#555555] text-[10px]">"v2.0"</span>
            </div>

            // Spacer
            <div class="flex-1"></div>

            // Connection status indicators
            <div class="flex items-center space-x-4">
                // WebSocket status with reconnect
                <div class="relative">
                    <button
                        class="flex items-center space-x-1.5 px-2 py-1 rounded hover:bg-[#ffffff08] transition-colors"
                        on:click=move |_| set_show_connection_menu.update(|v| *v = !*v)
                    >
                        <div class={move || if ws_connected.get() {
                            "w-1.5 h-1.5 bg-[#00d9ff] rounded-full animate-pulse"
                        } else {
                            "w-1.5 h-1.5 bg-[#ff4444] rounded-full"
                        }}></div>
                        <span class="text-[10px] text-[#888888]">"WS"</span>
                        <svg class="w-2.5 h-2.5 text-[#666666]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                        </svg>
                    </button>

                    // Connection dropdown menu
                    {move || if show_connection_menu.get() {
                        view! {
                            <div class="absolute right-0 top-full mt-1 w-64 bg-[#1a1a1a] border border-[#ffffff15] rounded shadow-lg z-50">
                                <div class="p-2 border-b border-[#ffffff10]">
                                    <div class="text-[9px] text-[#666666] uppercase tracking-wide mb-1">"WebSocket"</div>
                                    <div class="flex items-center justify-between">
                                        <span class="text-[10px] text-[#aaaaaa]">
                                            {move || if ws_connected.get() { "Connected" } else { "Disconnected" }}
                                        </span>
                                        <button
                                            class="text-[9px] px-2 py-0.5 bg-[#00d9ff20] text-[#00d9ff] rounded hover:bg-[#00d9ff30]"
                                            on:click=move |_| {
                                                ws.reconnect("ws://127.0.0.1:9000");
                                                set_show_connection_menu.set(false);
                                            }
                                        >
                                            "Reconnect"
                                        </button>
                                    </div>
                                </div>
                                <div class="p-2">
                                    <div class="text-[9px] text-[#666666] uppercase tracking-wide mb-1">"Robot"</div>
                                    <div class="flex items-center justify-between mb-2">
                                        <span class="text-[10px] text-[#aaaaaa]">
                                            {move || robot_addr.get()}
                                        </span>
                                        <span class={move || if robot_connected.get() {
                                            "text-[9px] text-[#22c55e]"
                                        } else {
                                            "text-[9px] text-[#ff4444]"
                                        }}>
                                            {move || if robot_connected.get() { "Connected" } else { "Disconnected" }}
                                        </span>
                                    </div>
                                    <div class="flex gap-1">
                                        <button
                                            class="flex-1 text-[9px] px-2 py-1 bg-[#22c55e20] text-[#22c55e] rounded hover:bg-[#22c55e30]"
                                            on:click=move |_| {
                                                let addr = robot_addr.get();
                                                let parts: Vec<&str> = addr.split(':').collect();
                                                if parts.len() == 2 {
                                                    if let Ok(port) = parts[1].parse::<u32>() {
                                                        ws.connect_robot(parts[0], port);
                                                    }
                                                }
                                                set_show_connection_menu.set(false);
                                            }
                                        >
                                            "Connect"
                                        </button>
                                        <button
                                            class="flex-1 text-[9px] px-2 py-1 bg-[#ff444420] text-[#ff4444] rounded hover:bg-[#ff444430]"
                                            on:click=move |_| {
                                                ws.disconnect_robot();
                                                set_show_connection_menu.set(false);
                                            }
                                        >
                                            "Disconnect"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}
                </div>

                // Robot status indicator
                <div class="flex items-center space-x-1.5">
                    <div class={move || if robot_connected.get() {
                        "w-1.5 h-1.5 bg-[#22c55e] rounded-full animate-pulse"
                    } else {
                        "w-1.5 h-1.5 bg-[#444444] rounded-full"
                    }}></div>
                    <span class="text-[10px] text-[#888888]">"Robot"</span>
                </div>

                // Settings button - shows quick settings popup
                <QuickSettingsButton/>
            </div>
        </header>
    }
}

/// Quick Settings button with popup
#[component]
fn QuickSettingsButton() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let navigate = use_navigate();

    let (show_popup, set_show_popup) = signal(false);
    let (robot_ip, set_robot_ip) = signal("127.0.0.1".to_string());
    let (robot_port, set_robot_port) = signal("16001".to_string());
    let (ws_url, set_ws_url) = signal("ws://127.0.0.1:9000".to_string());

    // Load saved connections when popup opens
    Effect::new(move |_| {
        if show_popup.get() {
            ws.list_robot_connections();
        }
    });

    let saved_connections = ws.robot_connections;
    let active_connection_id = ws.active_connection_id;

    // Derive active connection name for display
    let active_connection_name = move || {
        if let Some(id) = active_connection_id.get() {
            saved_connections.get()
                .iter()
                .find(|c| c.id == id)
                .map(|c| c.name.clone())
        } else {
            None
        }
    };

    // Clone navigate for use in closure
    let nav_to_settings = navigate.clone();

    view! {
        <div class="relative">
            <button
                class="p-1 hover:bg-[#ffffff08] rounded transition-colors"
                on:click=move |_| set_show_popup.update(|v| *v = !*v)
                title="Quick Settings"
            >
                <svg class="w-3.5 h-3.5 text-[#888888] hover:text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                </svg>
            </button>

            // Quick Settings popup
            {
                let nav = nav_to_settings.clone();
                move || if show_popup.get() {
                view! {
                    <div class="absolute right-0 top-full mt-1 w-80 bg-[#1a1a1a] border border-[#ffffff15] rounded-lg shadow-lg z-50">
                        // Header
                        <div class="flex items-center justify-between p-2 border-b border-[#ffffff10]">
                            <span class="text-[10px] font-semibold text-[#00d9ff]">"Quick Settings"</span>
                            <button
                                class="text-[#666666] hover:text-white"
                                on:click=move |_| set_show_popup.set(false)
                            >
                                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                </svg>
                            </button>
                        </div>

                        // Current Connection indicator
                        {move || {
                            if let Some(name) = active_connection_name() {
                                view! {
                                    <div class="p-2 border-b border-[#ffffff10] bg-[#00d9ff10]">
                                        <div class="flex items-center gap-2">
                                            <svg class="w-3 h-3 text-[#22c55e]" fill="currentColor" viewBox="0 0 24 24">
                                                <circle cx="12" cy="12" r="6"/>
                                            </svg>
                                            <span class="text-[10px] text-[#22c55e] font-medium">"Connected: "{name}</span>
                                        </div>
                                    </div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }
                        }}

                        // Saved Connections dropdown
                        <div class="p-2 border-b border-[#ffffff10]">
                            <label class="text-[9px] text-[#666666] uppercase tracking-wide">"Saved Connections"</label>
                            <select
                                class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none mt-0.5"
                                prop:value=move || active_connection_id.get().map(|id| id.to_string()).unwrap_or_default()
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    if !value.is_empty() {
                                        if let Ok(id) = value.parse::<i64>() {
                                            // Find the connection and populate fields
                                            let connections = saved_connections.get();
                                            if let Some(conn) = connections.iter().find(|c| c.id == id) {
                                                set_robot_ip.set(conn.ip_address.clone());
                                                set_robot_port.set(conn.port.to_string());
                                                // Track the active connection
                                                ws.set_active_connection(Some(id));
                                            }
                                        }
                                    } else {
                                        // Clear active connection when deselected
                                        ws.set_active_connection(None);
                                    }
                                }
                            >
                                <option value="">"-- Select a saved connection --"</option>
                                <For
                                    each=move || saved_connections.get()
                                    key=|conn| conn.id
                                    children=move |conn| {
                                        let conn_id = conn.id;
                                        let display = format!("{} ({}:{})", conn.name, conn.ip_address, conn.port);
                                        let is_selected = move || active_connection_id.get() == Some(conn_id);
                                        view! {
                                            <option
                                                value={conn_id.to_string()}
                                                selected=is_selected
                                            >
                                                {display}
                                            </option>
                                        }
                                    }
                                />
                            </select>
                        </div>

                        // Connection settings
                        <div class="p-2 space-y-2">
                            // Robot IP
                            <div>
                                <label class="text-[9px] text-[#666666] uppercase tracking-wide">"Robot IP"</label>
                                <input
                                    type="text"
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none mt-0.5"
                                    prop:value=move || robot_ip.get()
                                    on:input=move |ev| set_robot_ip.set(event_target_value(&ev))
                                />
                            </div>

                            // Robot Port
                            <div>
                                <label class="text-[9px] text-[#666666] uppercase tracking-wide">"Robot Port"</label>
                                <input
                                    type="text"
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none mt-0.5"
                                    prop:value=move || robot_port.get()
                                    on:input=move |ev| set_robot_port.set(event_target_value(&ev))
                                />
                            </div>

                            // WebSocket URL
                            <div>
                                <label class="text-[9px] text-[#666666] uppercase tracking-wide">"WebSocket URL"</label>
                                <input
                                    type="text"
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none mt-0.5"
                                    prop:value=move || ws_url.get()
                                    on:input=move |ev| set_ws_url.set(event_target_value(&ev))
                                />
                            </div>

                            // Apply button
                            <button
                                class="w-full bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[9px] px-3 py-1.5 rounded hover:bg-[#00d9ff30]"
                                on:click=move |_| {
                                    // Reconnect WebSocket
                                    ws.reconnect(&ws_url.get());
                                    // Connect to robot
                                    if let Ok(port) = robot_port.get().parse::<u32>() {
                                        ws.connect_robot(&robot_ip.get(), port);
                                    }
                                    set_show_popup.set(false);
                                }
                            >
                                "Apply & Connect"
                            </button>
                        </div>

                        // Link to full settings
                        <div class="p-2 border-t border-[#ffffff10]">
                            <button
                                class="w-full text-[9px] text-[#888888] hover:text-[#00d9ff] flex items-center justify-center gap-1"
                                on:click={
                                    let nav = nav.clone();
                                    move |_| {
                                        nav("/settings", Default::default());
                                        set_show_popup.set(false);
                                    }
                                }
                            >
                                "Manage connections in "
                                <span class="underline">"Settings"</span>
                                <svg class="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7"/>
                                </svg>
                            </button>
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}
        </div>
    }
}

