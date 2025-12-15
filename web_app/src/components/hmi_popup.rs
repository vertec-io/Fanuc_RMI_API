//! HMI Pop-out Window Component.
//!
//! This component renders a minimal HMI panel view for pop-out windows.
//! It uses BroadcastChannel to communicate with the parent window and
//! inherits control authority from the parent session.

use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use crate::hmi_broadcast::{use_hmi_broadcast, HmiBroadcastMessage, generate_tab_id};
use crate::websocket::WebSocketManager;
use crate::components::layout::workspace::dashboard::hmi::HmiPanelGrid;

/// Minimal HMI popup window component.
/// 
/// This is rendered at /hmi-popup?panel={panel_id} and shows only the HMI panel
/// without the full application chrome.
#[component]
pub fn HmiPopup() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let query = use_query_map();
    
    // Get panel ID from query params
    let panel_id = Memo::new(move |_| {
        query.get().get("panel")
            .and_then(|s| s.parse::<i64>().ok())
    });
    
    // Generate unique tab ID for this popup
    let tab_id = generate_tab_id();
    
    // Set up broadcast channel for cross-tab communication
    let broadcast = use_hmi_broadcast();
    let (parent_connected, set_parent_connected) = signal(false);
    let (has_control, set_has_control) = signal(false);

    // Clone post function for use in multiple closures
    let post_for_ready = broadcast.post.clone();
    let post_for_cleanup = broadcast.post.clone();
    let is_supported = broadcast.is_supported;
    let message = broadcast.message;

    // Send ChildReady message when component mounts
    let tab_id_clone = tab_id.clone();
    Effect::new(move |_| {
        if is_supported.get() {
            (post_for_ready.clone())(&HmiBroadcastMessage::ChildReady {
                child_id: tab_id_clone.clone()
            });
            log::info!("HMI popup sent ChildReady message");
        }
    });

    // Listen for messages from parent
    Effect::new(move |_| {
        if let Some(msg) = message.get() {
            match msg {
                HmiBroadcastMessage::SessionInfo { has_control: ctrl, .. } => {
                    set_parent_connected.set(true);
                    set_has_control.set(ctrl);
                    log::info!("HMI popup received session info, has_control={}", ctrl);
                }
                HmiBroadcastMessage::ControlGranted => {
                    set_has_control.set(true);
                    log::info!("HMI popup: control granted");
                }
                HmiBroadcastMessage::ControlRevoked => {
                    set_has_control.set(false);
                    log::info!("HMI popup: control revoked");
                }
                HmiBroadcastMessage::ParentClosing => {
                    log::info!("HMI popup: parent closing, closing popup");
                    // Close this window when parent closes
                    if let Some(win) = web_sys::window() {
                        let _ = win.close();
                    }
                }
                _ => {}
            }
        }
    });

    // Send ChildClosing message when window closes
    let tab_id_for_cleanup = tab_id.clone();
    on_cleanup(move || {
        (post_for_cleanup.clone())(&HmiBroadcastMessage::ChildClosing {
            child_id: tab_id_for_cleanup.clone()
        });
    });
    
    // Load the panel when we have a panel ID AND we're connected
    let ws_connected = ws.connected;
    Effect::new(move |_| {
        if ws_connected.get() {
            if let Some(id) = panel_id.get() {
                ws.get_hmi_panel_with_ports(id);
            }
        }
    });
    
    view! {
        <div class="min-h-screen bg-[#0a0a0a] text-white p-4">
            // Header with status
            <div class="flex items-center justify-between mb-4 pb-2 border-b border-[#1a1a1a]">
                <h1 class="text-lg font-semibold text-[#00d9ff]">
                    "HMI Panel"
                </h1>
                <div class="flex items-center gap-4 text-sm">
                    // Connection status
                    <div class="flex items-center gap-2">
                        <div class={move || format!(
                            "w-2 h-2 rounded-full {}",
                            if parent_connected.get() { "bg-[#00ff88]" } else { "bg-[#ff4444]" }
                        )}></div>
                        <span class="text-gray-400">
                            {move || if parent_connected.get() { "Connected" } else { "Connecting..." }}
                        </span>
                    </div>
                    // Control status
                    <div class="flex items-center gap-2">
                        <span class={move || format!(
                            "px-2 py-0.5 rounded text-xs {}",
                            if has_control.get() { 
                                "bg-[#00ff88]/20 text-[#00ff88]" 
                            } else { 
                                "bg-gray-700 text-gray-400" 
                            }
                        )}>
                            {move || if has_control.get() { "CONTROL" } else { "VIEW ONLY" }}
                        </span>
                    </div>
                </div>
            </div>
            
            // HMI Panel content
            {move || {
                match (panel_id.get(), ws.current_hmi_panel.get()) {
                    (Some(_id), Some(panel_data)) => {
                        view! {
                            <HmiPanelGrid panel_data=panel_data />
                        }.into_any()
                    }
                    (Some(_id), None) => {
                        view! {
                            <div class="flex items-center justify-center h-64 text-gray-500">
                                <div class="flex items-center gap-2">
                                    <div class="w-4 h-4 border-2 border-[#00d9ff] border-t-transparent rounded-full animate-spin"></div>
                                    "Loading panel..."
                                </div>
                            </div>
                        }.into_any()
                    }
                    (None, _) => {
                        view! {
                            <div class="flex items-center justify-center h-64 text-gray-500">
                                "No panel specified. Use ?panel=ID in URL."
                            </div>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}

