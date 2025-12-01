//! Quick Commands panel for robot control (Initialize, Reset, Abort, Continue).

use leptos::prelude::*;
use crate::components::layout::workspace::context::WorkspaceContext;
use crate::websocket::WebSocketManager;
use fanuc_rmi::dto::{SendPacket, Command, FrcInitialize};

/// Quick Commands panel for robot control (Initialize, Reset, Abort, Continue).
#[component]
pub fn QuickCommandsPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2 shrink-0">
            <div class="flex items-center justify-between mb-2">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    "Quick Commands"
                </h3>
            </div>
            <div class="flex gap-2 flex-wrap">
                // Initialize button
                <button
                    class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[9px] px-3 py-1.5 rounded hover:bg-[#22c55e30] flex items-center gap-1"
                    on:click=move |_| {
                        ws.send_command(SendPacket::Command(Command::FrcInitialize(FrcInitialize { group_mask: 1 })));
                    }
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 3v4M3 5h4M6 17v4m-2-2h4m5-16l2.286 6.857L21 12l-5.714 2.143L13 21l-2.286-6.857L5 12l5.714-2.143L13 3z"/>
                    </svg>
                    "Initialize"
                </button>
                // Reset button
                <button
                    class="bg-[#f59e0b20] border border-[#f59e0b40] text-[#f59e0b] text-[9px] px-3 py-1.5 rounded hover:bg-[#f59e0b30] flex items-center gap-1"
                    on:click=move |_| {
                        ws.send_command(SendPacket::Command(Command::FrcReset));
                    }
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                    </svg>
                    "Reset"
                </button>
                // Abort button
                <button
                    class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[9px] px-3 py-1.5 rounded hover:bg-[#ff444430] flex items-center gap-1"
                    on:click=move |_| {
                        ws.send_command(SendPacket::Command(Command::FrcAbort));
                        // Also stop any running program
                        ctx.program_running.set(false);
                        ctx.program_paused.set(false);
                        ctx.executing_line.set(-1);
                    }
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                    </svg>
                    "Abort"
                </button>
            </div>
        </div>
    }
}

