//! Command log panel showing console output.
//!
//! Displays all console messages in chronological order with sent/received indicators.

use leptos::prelude::*;
use leptos::either::Either;
use crate::websocket::{WebSocketManager, MessageDirection, MessageType};

/// Command Log panel - console-style output with chronological ordering
#[component]
pub fn CommandLogPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let console_messages = ws.console_messages;

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden">
            <div class="flex items-center justify-between p-2 border-b border-[#ffffff08] shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                    </svg>
                    "Console"
                </h3>
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
                {move || {
                    let messages = console_messages.get();

                    if messages.is_empty() {
                        Either::Left(view! {
                            <div class="text-[#555555] text-center py-4">
                                "Console output will appear here..."
                            </div>
                        })
                    } else {
                        // Messages are already sorted by timestamp_ms
                        let msg_views = messages.into_iter().map(|msg| {
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
                        }).collect_view();

                        Either::Right(msg_views)
                    }
                }}
            </div>
        </div>
    }
}

