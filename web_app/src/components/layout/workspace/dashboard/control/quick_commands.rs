//! Quick Commands panel for robot control (Initialize, Reset, Abort, Continue).

use leptos::prelude::*;
use crate::websocket::WebSocketManager;
use fanuc_rmi::dto::{SendPacket, Command, FrcSetOverRide};

/// Quick Commands panel for robot control (Initialize, Reset, Abort, Continue).
///
/// NOTE: This panel does NOT directly modify execution state (program_running, program_paused, etc).
/// All state updates come from server broadcasts (ExecutionStateChanged) to ensure UI
/// always reflects actual server state, not optimistic assumptions.
#[component]
pub fn QuickCommandsPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    // Local signal for slider value (synced with robot status when available)
    let (speed_override, set_speed_override) = signal(100u32);

    // Track when user is actively changing the value (to prevent polling from overwriting)
    let (user_editing, set_user_editing) = signal(false);
    // Timestamp of last user edit (to debounce sync with robot status)
    let (last_edit_time, set_last_edit_time) = signal(0.0f64);

    // Control lock state from WebSocketManager
    let has_control = ws.has_control;

    // Sync with robot status when it changes, but only if user isn't actively editing
    // and enough time has passed since last edit (1000ms debounce)
    let status = ws.status;
    Effect::new(move |_| {
        if let Some(s) = status.get() {
            // Don't overwrite if user is dragging the slider
            if user_editing.get() {
                return;
            }
            // Don't overwrite if user recently changed the value (give robot time to confirm)
            let now = js_sys::Date::now();
            if now - last_edit_time.get() < 1000.0 {
                return;
            }
            set_speed_override.set(s.speed_override);
        }
    });

    // Send override command when slider changes
    let send_override = move |value: u32| {
        let clamped = value.min(100) as u8;
        set_last_edit_time.set(js_sys::Date::now());
        ws.send_command(SendPacket::Command(Command::FrcSetOverRide(FrcSetOverRide { value: clamped })));
    };

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2 shrink-0">
            <div class="flex items-center justify-between mb-2">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    "Quick Commands"
                </h3>
                // Control lock indicator and button
                <div class="flex items-center gap-1">
                    {move || if has_control.get() {
                        view! {
                            <button
                                class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[8px] px-2 py-0.5 rounded hover:bg-[#22c55e30] flex items-center gap-1"
                                on:click=move |_| ws.release_control()
                                title="You have control. Click to release."
                            >
                                <svg class="w-2.5 h-2.5" fill="currentColor" viewBox="0 0 24 24">
                                    <path d="M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4zm-2 16l-4-4 1.41-1.41L10 14.17l6.59-6.59L18 9l-8 8z"/>
                                </svg>
                                "IN CONTROL"
                            </button>
                        }.into_any()
                    } else {
                        view! {
                            <button
                                class="bg-[#f59e0b20] border border-[#f59e0b40] text-[#f59e0b] text-[8px] px-2 py-0.5 rounded hover:bg-[#f59e0b30] flex items-center gap-1"
                                on:click=move |_| ws.request_control()
                                title="Request control of the robot"
                            >
                                <svg class="w-2.5 h-2.5" fill="currentColor" viewBox="0 0 24 24">
                                    <path d="M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z"/>
                                </svg>
                                "REQUEST CONTROL"
                            </button>
                        }.into_any()
                    }}
                </div>
            </div>
            <div class="flex gap-2 flex-wrap items-center">
                // Initialize button - uses API endpoint for proper error handling
                <button
                    class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[9px] px-3 py-1.5 rounded hover:bg-[#22c55e30] flex items-center gap-1"
                    on:click=move |_| {
                        ws.robot_initialize(Some(1));
                    }
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z"/>
                    </svg>
                    "Initialize"
                </button>
                // Reset button - uses API endpoint for proper error handling
                <button
                    class="bg-[#f59e0b20] border border-[#f59e0b40] text-[#f59e0b] text-[9px] px-3 py-1.5 rounded hover:bg-[#f59e0b30] flex items-center gap-1"
                    on:click=move |_| {
                        ws.robot_reset();
                    }
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                    </svg>
                    "Reset"
                </button>
                // Abort button - uses API endpoint for proper error handling and state sync
                // NOTE: Do NOT set local state here - wait for server ExecutionStateChanged broadcast
                // to ensure UI always reflects actual server state
                <button
                    class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[9px] px-3 py-1.5 rounded hover:bg-[#ff444430] flex items-center gap-1"
                    on:click=move |_| {
                        ws.robot_abort();
                        // Server will broadcast ExecutionStateChanged which will update:
                        // - program_running
                        // - program_paused
                        // - executing_line
                        // This ensures UI reflects actual server state, not optimistic assumptions
                    }
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                    </svg>
                    "Abort"
                </button>

                // Speed Override Slider
                <div class="flex items-center gap-2 ml-auto bg-[#1a1a1a] rounded px-2 py-1 border border-[#ffffff10]">
                    <svg class="w-3 h-3 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    <span class="text-[9px] text-gray-400 whitespace-nowrap">"Speed:"</span>
                    <input
                        type="range"
                        min="0"
                        max="100"
                        step="5"
                        class="w-20 h-1 bg-[#333] rounded-lg appearance-none cursor-pointer accent-[#00d9ff]"
                        prop:value=move || speed_override.get()
                        on:mousedown=move |_| set_user_editing.set(true)
                        on:touchstart=move |_| set_user_editing.set(true)
                        on:input=move |ev| {
                            // Mark that user is interacting - prevents status polling from overwriting
                            set_last_edit_time.set(js_sys::Date::now());
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                set_speed_override.set(val);
                            }
                        }
                        on:change=move |ev| {
                            // User finished interacting - send the command
                            set_user_editing.set(false);
                            if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                send_override(val);
                            }
                        }
                    />
                    <span class="text-[10px] text-[#00d9ff] font-mono w-8 text-right">
                        {move || format!("{}%", speed_override.get())}
                    </span>
                </div>
            </div>
        </div>
    }
}

