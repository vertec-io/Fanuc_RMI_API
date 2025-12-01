//! Command log panel showing console output.

use leptos::prelude::*;
use leptos::either::Either;
use crate::components::layout::workspace::context::{WorkspaceContext, CommandStatus};
use crate::websocket::WebSocketManager;

/// Command Log panel - console-style output
#[component]
pub fn CommandLogPanel() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let log = ctx.command_log;

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
                        log.set(Vec::new());
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
                    let entries = log.get();
                    let motions = ws.motion_log.get();
                    let errors = ws.error_log.get();

                    if entries.is_empty() && motions.is_empty() && errors.is_empty() {
                        Either::Left(view! {
                            <div class="text-[#555555] text-center py-4">
                                "Console output will appear here..."
                            </div>
                        })
                    } else {
                        // Show command log entries
                        let cmd_views = entries.into_iter().map(|entry| {
                            let status_class = match &entry.status {
                                CommandStatus::Pending => "text-[#f59e0b]",
                                CommandStatus::Success => "text-[#22c55e]",
                                CommandStatus::Error(_) => "text-[#ff4444]",
                            };
                            let status_icon = match &entry.status {
                                CommandStatus::Pending => "⏳",
                                CommandStatus::Success => "✓",
                                CommandStatus::Error(_) => "✗",
                            };
                            view! {
                                <div class="py-0.5 border-b border-[#ffffff05]">
                                    <span class="text-[#555555] mr-1">{entry.timestamp}</span>
                                    <span class={status_class}>{status_icon}</span>
                                    <span class="text-[#cccccc] ml-1">{entry.command}</span>
                                </div>
                            }
                        }).collect_view();

                        // Show motion log entries (last 20)
                        let motion_views = motions.into_iter().rev().take(20).collect::<Vec<_>>().into_iter().rev().map(|msg| {
                            view! {
                                <div class="text-[9px] py-0.5 text-[#00d9ff] border-b border-[#ffffff05]">
                                    <span class="text-[#22c55e] mr-1">"✓"</span>
                                    {msg}
                                </div>
                            }
                        }).collect_view();

                        // Show error entries
                        let error_views = errors.into_iter().map(|msg| {
                            view! {
                                <div class="text-[9px] py-0.5 text-[#ff4444] border-b border-[#ffffff05]">
                                    <span class="mr-1">"✗"</span>
                                    {msg}
                                </div>
                            }
                        }).collect_view();

                        Either::Right(view! {
                            {cmd_views}
                            {motion_views}
                            {error_views}
                        })
                    }
                }}
            </div>
        </div>
    }
}

