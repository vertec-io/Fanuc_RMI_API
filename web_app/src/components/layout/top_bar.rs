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
    let ws_connecting = ws.ws_connecting;
    let robot_connected = ws.robot_connected;
    let robot_connecting = ws.robot_connecting;
    let robot_addr = ws.robot_addr;
    let connected_robot_name = ws.connected_robot_name;

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
                            <div class="absolute right-0 top-full mt-1 w-56 bg-[#1a1a1a] border border-[#ffffff15] rounded shadow-lg z-50">
                                <div class="p-2">
                                    <div class="text-[9px] text-[#666666] uppercase tracking-wide mb-1">"WebSocket Server"</div>
                                    <div class="flex items-center justify-between mb-2">
                                        <span class="text-[10px] text-[#aaaaaa]">
                                            {move || {
                                                if ws_connecting.get() {
                                                    "Connecting..."
                                                } else if ws_connected.get() {
                                                    "Connected"
                                                } else {
                                                    "Disconnected"
                                                }
                                            }}
                                        </span>
                                        <div class={move || {
                                            if ws_connecting.get() {
                                                "w-1.5 h-1.5 bg-[#ffaa00] rounded-full animate-pulse"
                                            } else if ws_connected.get() {
                                                "w-1.5 h-1.5 bg-[#00d9ff] rounded-full animate-pulse"
                                            } else {
                                                "w-1.5 h-1.5 bg-[#ff4444] rounded-full"
                                            }
                                        }}></div>
                                    </div>
                                    <div class="text-[9px] text-[#555555] mb-2">"ws://127.0.0.1:9000"</div>
                                    <button
                                        class="w-full text-[9px] px-2 py-1 bg-[#00d9ff20] text-[#00d9ff] rounded hover:bg-[#00d9ff30] disabled:opacity-50 disabled:cursor-not-allowed"
                                        disabled=move || ws_connected.get() || ws_connecting.get()
                                        on:click=move |_| {
                                            ws.reconnect("ws://127.0.0.1:9000");
                                            set_show_connection_menu.set(false);
                                        }
                                    >
                                        {move || {
                                            if ws_connecting.get() {
                                                "Connecting..."
                                            } else if ws_connected.get() {
                                                "Connected"
                                            } else {
                                                "Reconnect"
                                            }
                                        }}
                                    </button>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}
                </div>

                // Robot status indicator with name (only show when WebSocket connected)
                <Show when=move || ws_connected.get()>
                    <div class="flex items-center space-x-1.5">
                        <div class={move || {
                            if robot_connecting.get() {
                                "w-1.5 h-1.5 bg-[#ffaa00] rounded-full animate-pulse"
                            } else if robot_connected.get() {
                                "w-1.5 h-1.5 bg-[#22c55e] rounded-full animate-pulse"
                            } else {
                                "w-1.5 h-1.5 bg-[#444444] rounded-full"
                            }
                        }}></div>
                        <span class="text-[10px] text-[#888888]">
                            {move || {
                                if robot_connecting.get() {
                                    "Connecting...".to_string()
                                } else if let Some(name) = connected_robot_name.get() {
                                    name
                                } else if robot_connected.get() {
                                    "Robot".to_string()
                                } else {
                                    "No Robot".to_string()
                                }
                            }}
                        </span>
                    </div>
                </Show>

                // Control status button (only show when WebSocket connected)
                <Show when=move || ws_connected.get()>
                    <ControlButton/>
                </Show>

                // Settings button - shows quick settings popup (only show when WebSocket connected)
                <Show when=move || ws_connected.get()>
                    <QuickSettingsButton/>
                </Show>
            </div>
        </header>
    }
}

/// Quick Settings button with popup - focused on robot connection switching
#[component]
fn QuickSettingsButton() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let navigate = use_navigate();

    let (show_popup, set_show_popup) = signal(false);

    // Load saved connections when popup opens
    Effect::new(move |_| {
        if show_popup.get() {
            ws.list_robot_connections();
        }
    });

    let saved_connections = ws.robot_connections;
    let active_connection_id = ws.active_connection_id;
    let robot_connected = ws.robot_connected;
    let robot_connecting = ws.robot_connecting;
    let has_control = ws.has_control;
    let connected_robot_name = ws.connected_robot_name;

    let nav_to_settings = navigate.clone();

    view! {
        <div class="relative">
            <button
                class="p-1 hover:bg-[#ffffff08] rounded transition-colors"
                on:click=move |_| set_show_popup.update(|v| *v = !*v)
                title="Quick Connect"
            >
                <svg class="w-3.5 h-3.5 text-[#888888] hover:text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                </svg>
            </button>

            // Quick Connect popup
            {
                let nav = nav_to_settings.clone();
                move || if show_popup.get() {
                view! {
                    <div class="absolute right-0 top-full mt-1 w-72 bg-[#1a1a1a] border border-[#ffffff15] rounded-lg shadow-lg z-50">
                        // Header
                        <div class="flex items-center justify-between p-2 border-b border-[#ffffff10]">
                            <span class="text-[10px] font-semibold text-[#00d9ff]">"Quick Connect"</span>
                            <button
                                class="text-[#666666] hover:text-white"
                                on:click=move |_| set_show_popup.set(false)
                            >
                                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                </svg>
                            </button>
                        </div>

                        // Current Status Section
                        <div class="p-2 border-b border-[#ffffff10] space-y-1.5">
                            // Connection status
                            <div class="flex items-center justify-between">
                                <span class="text-[9px] text-[#666666] uppercase">"Robot"</span>
                                {move || {
                                    if robot_connecting.get() {
                                        view! {
                                            <div class="flex items-center gap-1.5">
                                                <div class="w-1.5 h-1.5 bg-[#ffaa00] rounded-full animate-pulse"></div>
                                                <span class="text-[10px] text-[#ffaa00] font-medium">"Connecting..."</span>
                                            </div>
                                        }.into_any()
                                    } else if robot_connected.get() {
                                        let name = connected_robot_name.get().unwrap_or_else(|| "Connected".to_string());
                                        view! {
                                            <div class="flex items-center gap-1.5">
                                                <div class="w-1.5 h-1.5 bg-[#22c55e] rounded-full animate-pulse"></div>
                                                <span class="text-[10px] text-[#22c55e] font-medium">{name}</span>
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div class="flex items-center gap-1.5">
                                                <div class="w-1.5 h-1.5 bg-[#666666] rounded-full"></div>
                                                <span class="text-[10px] text-[#888888]">"Not Connected"</span>
                                            </div>
                                        }.into_any()
                                    }
                                }}
                            </div>
                            // Control status
                            <div class="flex items-center justify-between">
                                <span class="text-[9px] text-[#666666] uppercase">"Control"</span>
                                {move || {
                                    if has_control.get() {
                                        view! {
                                            <span class="text-[10px] text-[#00d9ff] font-medium">"You have control"</span>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <span class="text-[10px] text-[#888888]">"No control"</span>
                                        }.into_any()
                                    }
                                }}
                            </div>
                        </div>

                        // Saved Connections List
                        <div class="p-2 border-b border-[#ffffff10]">
                            <div class="text-[9px] text-[#666666] uppercase tracking-wide mb-1.5">"Saved Robots"</div>
                            <div class="space-y-1 max-h-48 overflow-y-auto">
                                {move || {
                                    let connections = saved_connections.get();
                                    if connections.is_empty() {
                                        view! {
                                            <div class="text-[10px] text-[#555555] italic py-2 text-center">
                                                "No saved connections"
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <For
                                                each=move || saved_connections.get()
                                                key=|conn| conn.id
                                                children=move |conn| {
                                                    let conn_id = conn.id;
                                                    let conn_name = conn.name.clone();
                                                    let conn_addr = format!("{}:{}", conn.ip_address, conn.port);
                                                    let is_active = move || active_connection_id.get() == Some(conn_id);
                                                    let can_connect = move || has_control.get() && !is_active();

                                                    view! {
                                                        <div class={move || {
                                                            let base = "flex items-center justify-between p-1.5 rounded";
                                                            if is_active() {
                                                                format!("{} bg-[#22c55e15] border border-[#22c55e40]", base)
                                                            } else {
                                                                format!("{} bg-[#ffffff05] hover:bg-[#ffffff08]", base)
                                                            }
                                                        }}>
                                                            <div class="flex-1 min-w-0">
                                                                <div class="text-[10px] text-white font-medium truncate">{conn_name}</div>
                                                                <div class="text-[9px] text-[#666666]">{conn_addr}</div>
                                                            </div>
                                                            {move || {
                                                                if is_active() {
                                                                    // Show disconnect button for active connection
                                                                    view! {
                                                                        <button
                                                                            class="text-[8px] px-2 py-0.5 bg-[#ff444420] text-[#ff4444] rounded hover:bg-[#ff444430] disabled:opacity-50 disabled:cursor-not-allowed"
                                                                            disabled=move || !has_control.get() || robot_connecting.get()
                                                                            title=move || if has_control.get() { "Disconnect" } else { "Need control to disconnect" }
                                                                            on:click=move |_| {
                                                                                ws.disconnect_robot();
                                                                            }
                                                                        >
                                                                            "Disconnect"
                                                                        </button>
                                                                    }.into_any()
                                                                } else {
                                                                    // Show connect button for other connections
                                                                    view! {
                                                                        <button
                                                                            class="text-[8px] px-2 py-0.5 bg-[#00d9ff20] text-[#00d9ff] rounded hover:bg-[#00d9ff30] disabled:opacity-50 disabled:cursor-not-allowed"
                                                                            disabled=move || !can_connect() || robot_connecting.get()
                                                                            title=move || {
                                                                                if robot_connecting.get() {
                                                                                    "Connecting..."
                                                                                } else if !has_control.get() {
                                                                                    "Need control to connect"
                                                                                } else {
                                                                                    "Connect to this robot"
                                                                                }
                                                                            }
                                                                            on:click=move |_| {
                                                                                ws.connect_to_saved_robot(conn_id);
                                                                                set_show_popup.set(false);
                                                                            }
                                                                        >
                                                                            {move || if robot_connecting.get() { "Connecting..." } else { "Connect" }}
                                                                        </button>
                                                                    }.into_any()
                                                                }
                                                            }}
                                                        </div>
                                                    }
                                                }
                                            />
                                        }.into_any()
                                    }
                                }}
                            </div>
                        </div>

                        // Control actions
                        {move || {
                            if !has_control.get() {
                                view! {
                                    <div class="p-2 border-b border-[#ffffff10]">
                                        <button
                                            class="w-full text-[9px] px-3 py-1.5 bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] rounded hover:bg-[#00d9ff30]"
                                            on:click=move |_| {
                                                ws.request_control();
                                            }
                                        >
                                            "Request Control"
                                        </button>
                                    </div>
                                }.into_any()
                            } else {
                                view! {
                                    <div class="p-2 border-b border-[#ffffff10]">
                                        <button
                                            class="w-full text-[9px] px-3 py-1.5 bg-[#ff444420] border border-[#ff444440] text-[#ff4444] rounded hover:bg-[#ff444430]"
                                            on:click=move |_| {
                                                ws.release_control();
                                            }
                                        >
                                            "Release Control"
                                        </button>
                                    </div>
                                }.into_any()
                            }
                        }}

                        // Link to full settings
                        <div class="p-2">
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

/// Control button - Request/Release control (styled like QuickCommandsPanel)
#[component]
fn ControlButton() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let has_control = ws.has_control;

    view! {
        <button
            class=move || if has_control.get() {
                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[8px] px-2 py-0.5 rounded hover:bg-[#22c55e30] flex items-center gap-1"
            } else {
                "bg-[#f59e0b20] border border-[#f59e0b40] text-[#f59e0b] text-[8px] px-2 py-0.5 rounded hover:bg-[#f59e0b30] flex items-center gap-1"
            }
            on:click=move |_| {
                if has_control.get() {
                    ws.release_control();
                } else {
                    ws.request_control();
                }
            }
            title=move || if has_control.get() {
                "You have control. Click to release."
            } else {
                "Request control of the robot"
            }
        >
            {move || if has_control.get() {
                view! {
                    <svg class="w-2.5 h-2.5" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4zm-2 16l-4-4 1.41-1.41L10 14.17l6.59-6.59L18 9l-8 8z"/>
                    </svg>
                    "IN CONTROL"
                }.into_any()
            } else {
                view! {
                    <svg class="w-2.5 h-2.5" fill="currentColor" viewBox="0 0 24 24">
                        <path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4z"/>
                    </svg>
                    "REQUEST CONTROL"
                }.into_any()
            }}
        </button>
    }
}
