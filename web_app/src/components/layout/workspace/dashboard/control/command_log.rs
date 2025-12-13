//! Command log panel showing console output.
//!
//! Displays all console messages in chronological order with sent/received indicators.
//! Has tabs for "Messages" (all messages) and "Errors" (error messages only).

use leptos::prelude::*;
use crate::websocket::{WebSocketManager, MessageDirection, MessageType};

/// Command Log panel - console-style output with chronological ordering and tabs
#[component]
pub fn CommandLogPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let console_messages = ws.console_messages;
    let error_log = ws.error_log;

    // Tab state: "messages" or "errors"
    let (active_tab, set_active_tab) = signal("messages");

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden">
            <div class="flex items-center justify-between p-2 border-b border-[#ffffff08] shrink-0">
                <div class="flex items-center gap-2">
                    <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                        </svg>
                        "Console"
                    </h3>
                    // Tabs
                    <div class="flex gap-1">
                        <button
                            class=move || if active_tab.get() == "messages" {
                                "text-[9px] px-2 py-0.5 bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded"
                            } else {
                                "text-[9px] px-2 py-0.5 bg-[#ffffff05] text-[#666666] border border-[#ffffff15] rounded hover:bg-[#ffffff10]"
                            }
                            on:click=move |_| set_active_tab.set("messages")
                        >
                            "Messages"
                        </button>
                        <button
                            class=move || {
                                let error_count = error_log.get().len();
                                let base_class = if active_tab.get() == "errors" {
                                    "text-[9px] px-2 py-0.5 bg-[#ff444420] text-[#ff4444] border border-[#ff4444] rounded"
                                } else {
                                    "text-[9px] px-2 py-0.5 bg-[#ffffff05] text-[#666666] border border-[#ffffff15] rounded hover:bg-[#ffffff10]"
                                };
                                if error_count > 0 {
                                    format!("{} relative", base_class)
                                } else {
                                    base_class.to_string()
                                }
                            }
                            on:click=move |_| set_active_tab.set("errors")
                        >
                            "Errors"
                            {move || {
                                let count = error_log.get().len();
                                if count > 0 {
                                    Some(view! {
                                        <span class="ml-1 bg-[#ff4444] text-black text-[8px] px-1 rounded-full font-bold">
                                            {count}
                                        </span>
                                    })
                                } else {
                                    None
                                }
                            }}
                        </button>
                    </div>
                </div>
                <button
                    class="text-[8px] text-[#666666] hover:text-[#ff4444]"
                    on:click=move |_| {
                        ws.clear_console_messages();
                        ws.clear_motion_log();
                        ws.clear_error_log();
                    }
                    title="Clear console"
                >
                    "Clear"
                </button>
            </div>
            <div class="flex-1 overflow-y-auto p-2 font-mono text-[9px]">
                <Show when=move || active_tab.get() == "messages" fallback=move || {
                    // Show errors only
                    let errors = error_log.get();

                    errors.into_iter().rev().map(|error| {
                        view! {
                            <div class="py-0.5 border-b border-[#ffffff05] flex items-start border-l-2 border-l-[#ff4444] pl-2">
                                <span class="text-[#ff4444]">{error}</span>
                            </div>
                        }
                    }).collect_view()
                }>
                    {move || {
                        // Show all console messages
                        let messages = console_messages.get();

                        // Messages are already sorted by timestamp_ms
                        messages.into_iter().map(|msg| {
                            // Direction indicator and color
                            let (dir_icon, dir_class) = match msg.direction {
                                MessageDirection::Sent => ("→", "text-[#00d9ff]"),
                                MessageDirection::Received => ("←", "text-[#22c55e]"),
                                MessageDirection::System => ("•", "text-[#f59e0b]"),
                            };

                            // Message type color
                            let content_class = match msg.msg_type {
                                MessageType::Command => "text-[#00d9ff]",
                                MessageType::Response => "text-[#22c55e]",
                                MessageType::Error => "text-[#ff4444]",
                                MessageType::Status => "text-[#888888]",
                                MessageType::Config => "text-[#f59e0b]",
                            };

                            // Sequence ID display
                            let seq_display = msg.sequence_id.map(|id| format!(" seq={}", id)).unwrap_or_default();

                            view! {
                                <div class="py-0.5 border-b border-[#ffffff05] flex items-start">
                                    <span class="text-[#555555] mr-1 shrink-0">{format!("[{}]", msg.timestamp)}</span>
                                    <span class={format!("{} mr-1 shrink-0", dir_class)}>{dir_icon}</span>
                                    <span class={content_class}>
                                        {msg.content}
                                        <span class="text-[#666666]">{seq_display}</span>
                                    </span>
                                </div>
                            }
                        }).collect_view()
                    }}
                </Show>
            </div>
        </div>
    }
}

