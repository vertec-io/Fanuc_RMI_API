//! HMI Broadcast Channel - Cross-tab synchronization for HMI panels.
//!
//! Uses the BroadcastChannel API to synchronize HMI state between
//! the main application tab and popped-out HMI windows.

use leptos::prelude::*;
use leptos_use::{use_broadcast_channel, UseBroadcastChannelReturn};
use codee::string::JsonSerdeCodec;
use serde::{Deserialize, Serialize};
use web_common::ClientRequest;

/// Channel name for HMI synchronization
const HMI_CHANNEL_NAME: &str = "fanuc-hmi-sync";

/// Messages sent between parent and child tabs via BroadcastChannel.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum HmiBroadcastMessage {
    // === Parent → Child ===
    /// Parent sends session info when child connects
    SessionInfo {
        session_id: String,
        robot_connection_id: i64,
        has_control: bool,
    },
    /// Control was granted to the session
    ControlGranted,
    /// Control was revoked from the session
    ControlRevoked,
    /// Parent is closing, children should close too
    ParentClosing,

    // === Child → Parent ===
    /// Child tab is ready and requesting session info
    ChildReady { child_id: String },
    /// Child tab is closing
    ChildClosing { child_id: String },
    /// Child wants to send a command through parent's WebSocket
    CommandRequest { child_id: String, command: ClientRequest },
}

/// Hook to use the HMI broadcast channel.
///
/// This provides cross-tab communication for HMI synchronization.
/// Returns the broadcast channel with post/close functions and message signal.
pub fn use_hmi_broadcast() -> UseBroadcastChannelReturn<
    HmiBroadcastMessage,
    impl Fn(&HmiBroadcastMessage) + Clone + Send + Sync,
    impl Fn() + Clone + Send + Sync,
    JsonSerdeCodec,
> {
    use_broadcast_channel::<HmiBroadcastMessage, JsonSerdeCodec>(HMI_CHANNEL_NAME)
}

/// Generate a unique ID for this tab/window.
pub fn generate_tab_id() -> String {
    use web_sys::window;
    
    // Try to get from session storage first (persists across refreshes)
    if let Some(win) = window() {
        if let Ok(Some(storage)) = win.session_storage() {
            if let Ok(Some(id)) = storage.get_item("hmi_tab_id") {
                return id;
            }
            // Generate new ID
            let id = format!("tab_{}", js_sys::Date::now() as u64);
            let _ = storage.set_item("hmi_tab_id", &id);
            return id;
        }
    }
    
    // Fallback
    format!("tab_{}", js_sys::Date::now() as u64)
}

/// Check if this is a popped-out HMI window (opened by window.open).
pub fn is_popup_window() -> bool {
    use web_sys::window;
    
    if let Some(win) = window() {
        // Check if window.opener exists (indicates this was opened by another window)
        win.opener().map(|o| !o.is_null() && !o.is_undefined()).unwrap_or(false)
    } else {
        false
    }
}

/// Open a new HMI pop-out window.
pub fn open_hmi_popup(panel_id: i64) -> Option<web_sys::Window> {
    use web_sys::window;

    if let Some(win) = window() {
        let url = format!("/hmi-popup?panel={}", panel_id);
        let features = "width=800,height=600,menubar=no,toolbar=no,location=no,status=no,resizable=yes";

        win.open_with_url_and_target_and_features(&url, "_blank", features)
            .ok()
            .flatten()
    } else {
        None
    }
}

/// Parent-side broadcast handler component.
///
/// This component should be included in the main app layout (not in popups).
/// It listens for child messages and broadcasts control status changes.
#[component]
pub fn HmiBroadcastHandler() -> impl IntoView {
    use crate::websocket::WebSocketManager;

    // Only run in parent windows (not popups)
    if is_popup_window() {
        return view! {}.into_any();
    }

    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let has_control = ws.has_control;
    let active_connection_id = ws.active_connection_id;

    let broadcast = use_hmi_broadcast();
    let post = broadcast.post.clone();
    let post_for_control = broadcast.post.clone();
    let post_for_cleanup = broadcast.post.clone();
    let message = broadcast.message;

    // Track connected children
    let (child_ids, set_child_ids) = signal::<Vec<String>>(Vec::new());

    // Listen for child messages
    Effect::new(move |_| {
        if let Some(msg) = message.get() {
            match msg {
                HmiBroadcastMessage::ChildReady { child_id } => {
                    log::info!("Parent: child {} ready, sending session info", child_id);

                    // Add to tracked children
                    set_child_ids.update(|ids| {
                        if !ids.contains(&child_id) {
                            ids.push(child_id.clone());
                        }
                    });

                    // Send session info to child
                    let session_id = generate_tab_id();
                    let robot_connection_id = active_connection_id.get().unwrap_or(-1);
                    let ctrl = has_control.get();

                    (post.clone())(&HmiBroadcastMessage::SessionInfo {
                        session_id,
                        robot_connection_id,
                        has_control: ctrl,
                    });
                }
                HmiBroadcastMessage::ChildClosing { child_id } => {
                    log::info!("Parent: child {} closing", child_id);
                    set_child_ids.update(|ids| {
                        ids.retain(|id| id != &child_id);
                    });
                }
                HmiBroadcastMessage::CommandRequest { child_id, command } => {
                    log::info!("Parent: received command from child {}: {:?}", child_id, command);
                    // Forward command through parent's WebSocket
                    ws.send_api_request(command);
                }
                _ => {}
            }
        }
    });

    // Broadcast control status changes
    let prev_control = StoredValue::new(has_control.get_untracked());
    Effect::new(move |_| {
        let current = has_control.get();
        let prev = prev_control.get_value();

        if current != prev {
            prev_control.set_value(current);

            if current {
                log::info!("Parent: broadcasting ControlGranted");
                (post_for_control.clone())(&HmiBroadcastMessage::ControlGranted);
            } else {
                log::info!("Parent: broadcasting ControlRevoked");
                (post_for_control.clone())(&HmiBroadcastMessage::ControlRevoked);
            }
        }
    });

    // Broadcast ParentClosing when window closes
    on_cleanup(move || {
        log::info!("Parent: broadcasting ParentClosing");
        (post_for_cleanup.clone())(&HmiBroadcastMessage::ParentClosing);
    });

    // This component renders nothing
    view! {}.into_any()
}
