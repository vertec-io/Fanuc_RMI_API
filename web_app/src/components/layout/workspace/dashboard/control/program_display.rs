//! Program visual display - G-code style line-by-line view.

use leptos::prelude::*;
use leptos::either::Either;
use crate::components::layout::workspace::context::{WorkspaceContext, ProgramLine};
use crate::websocket::WebSocketManager;
use super::LoadProgramModal;

/// Program Visual Display - G-code style line-by-line view
#[component]
pub fn ProgramVisualDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let lines = ctx.program_lines;
    let executing = ctx.executing_line;
    let loaded_name = ctx.loaded_program_name;
    let loaded_id = ctx.loaded_program_id;
    let is_running = ctx.program_running;
    let is_paused = ctx.program_paused;
    let (show_load_modal, set_show_load_modal) = signal(false);

    // Request execution state when WebSocket connects (for new clients or reconnection)
    Effect::new(move |_| {
        if ws.connected.get() {
            // Request current execution state from server
            ws.get_execution_state();
        }
    });

    // Sync loaded program ID from server - server is the source of truth
    // When server broadcasts a program is loaded/unloaded, update local context
    Effect::new(move |_| {
        let ws_program_id = ws.loaded_program_id.get();
        let local_program_id = ctx.loaded_program_id.get();

        if ws_program_id != local_program_id {
            match ws_program_id {
                Some(id) => {
                    // A program was loaded on the server - fetch its details
                    ctx.loaded_program_id.set(Some(id));
                    ws.get_program(id);
                }
                None => {
                    // Program was unloaded from server - clear local state
                    ctx.loaded_program_id.set(None);
                    ctx.loaded_program_name.set(None);
                    ctx.program_lines.set(Vec::new());
                    ctx.executing_line.set(-1);
                    ctx.program_running.set(false);
                    ctx.program_paused.set(false);
                }
            }
        }
    });

    // Sync current_program details to local state when fetched
    Effect::new(move |_| {
        if let Some(detail) = ws.current_program.get() {
            // Only update if this is the loaded program
            if ctx.loaded_program_id.get() == Some(detail.id) {
                // Convert instructions to ProgramLine format
                let lines: Vec<ProgramLine> = detail.instructions.iter().map(|i| {
                    ProgramLine {
                        line_number: i.line_number as usize,
                        x: i.x,
                        y: i.y,
                        z: i.z,
                        w: i.w.unwrap_or(0.0),
                        p: i.p.unwrap_or(0.0),
                        r: i.r.unwrap_or(0.0),
                        speed: i.speed.unwrap_or(100.0),
                        term_type: i.term_type.clone().unwrap_or_else(|| "CNT".to_string()),
                        uframe: i.uframe,
                        utool: i.utool,
                    }
                }).collect();

                ctx.program_lines.set(lines);
                ctx.loaded_program_name.set(Some(detail.name.clone()));
            }
        }
    });

    // Sync WebSocket program_running state with context
    // Server is the source of truth - only update context from server state
    Effect::new(move |_| {
        let ws_running = ws.program_running.get();
        if is_running.get() != ws_running {
            ctx.program_running.set(ws_running);
            if !ws_running {
                // Program completed/stopped - reset executing line
                ctx.executing_line.set(-1);
            }
        }
    });

    // Sync WebSocket program_paused state with context
    // Server is the source of truth - UI only changes when server confirms
    Effect::new(move |_| {
        let ws_paused = ws.program_paused.get();
        if is_paused.get() != ws_paused {
            ctx.program_paused.set(ws_paused);
        }
    });

    // Sync WebSocket executing_line to context executing_line
    Effect::new(move |_| {
        if let Some(line) = ws.executing_line.get() {
            ctx.executing_line.set(line as i32);
        }
    });

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden">
            <div class="flex items-center justify-between p-2 border-b border-[#ffffff08] shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                    </svg>
                    {move || loaded_name.get().unwrap_or_else(|| "Program".to_string())}
                </h3>
                <div class="flex items-center gap-1">
                    <span class="text-[8px] text-[#666666] mr-1">
                        {move || format!("{} lines", lines.get().len())}
                    </span>
                    // Load button - show when no program is loaded
                    <Show when=move || loaded_id.get().is_none()>
                        <button
                            class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[8px] px-2 py-0.5 rounded hover:bg-[#00d9ff30]"
                            on:click=move |_| {
                                ws.list_programs();
                                set_show_load_modal.set(true);
                            }
                        >
                            "📂 Load"
                        </button>
                    </Show>
                    // Control buttons - only show when program is loaded
                    <Show when=move || loaded_id.get().is_some()>
                        // Run button - show when not running
                        <Show when=move || !is_running.get()>
                            <button
                                class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[8px] px-2 py-0.5 rounded hover:bg-[#22c55e30]"
                                on:click=move |_| {
                                    if let Some(id) = loaded_id.get() {
                                        // Send request - server will broadcast state change
                                        ws.start_program(id);
                                    }
                                }
                            >
                                "▶ Run"
                            </button>
                        </Show>
                        // Pause/Resume button - show when running
                        <Show when=move || is_running.get()>
                            {move || {
                                if is_paused.get() {
                                    Either::Left(view! {
                                        <button
                                            class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[8px] px-2 py-0.5 rounded hover:bg-[#22c55e30]"
                                            on:click=move |_| {
                                                // Send request - server will broadcast state change if control check passes
                                                ws.resume_program();
                                            }
                                        >
                                            "▶ Resume"
                                        </button>
                                    })
                                } else {
                                    Either::Right(view! {
                                        <button
                                            class="bg-[#f59e0b20] border border-[#f59e0b40] text-[#f59e0b] text-[8px] px-2 py-0.5 rounded hover:bg-[#f59e0b30]"
                                            on:click=move |_| {
                                                // Send request - server will broadcast state change if control check passes
                                                ws.pause_program();
                                            }
                                        >
                                            "⏸ Pause"
                                        </button>
                                    })
                                }
                            }}
                        </Show>
                        // Stop button - show when running
                        <Show when=move || is_running.get()>
                            <button
                                class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[8px] px-2 py-0.5 rounded hover:bg-[#ff444430]"
                                on:click=move |_| {
                                    // Send request - server will broadcast state change if control check passes
                                    ws.stop_program();
                                }
                            >
                                "■ Stop"
                            </button>
                        </Show>
                    </Show>
                    // Unload button - only show when not running
                    <Show when=move || loaded_id.get().is_some() && !is_running.get()>
                        <button
                            class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[8px] px-2 py-0.5 rounded hover:bg-[#ff444430] flex items-center gap-1"
                            on:click=move |_| {
                                // Send UnloadProgram to server (requires control)
                                // Server will broadcast ExecutionStateChanged with idle state and program_id=None
                                ws.unload_program();
                            }
                            title="Unload program from robot"
                        >
                            <svg class="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"/>
                            </svg>
                            "Unload"
                        </button>
                    </Show>
                </div>
            </div>
            // Progress bar - show when program is running
            <Show when=move || is_running.get() && !lines.get().is_empty()>
                <ProgramProgressBar
                    current_line=Signal::derive(move || {
                        // Use program_progress from WebSocket if available, otherwise use executing line
                        ws.program_progress.get()
                            .map(|(current, _)| current)
                            .unwrap_or_else(|| executing.get().max(0) as usize)
                    })
                    total_lines=Signal::derive(move || {
                        // Use program_progress from WebSocket if available, otherwise use lines count
                        ws.program_progress.get()
                            .map(|(_, total)| total)
                            .unwrap_or_else(|| lines.get().len())
                    })
                    execution_status=ws.execution_status
                />
            </Show>
            <ProgramTable lines=lines executing=executing/>
        </div>

        // Load Program Modal
        <Show when=move || show_load_modal.get()>
            <LoadProgramModal on_close=move || set_show_load_modal.set(false)/>
        </Show>
    }
}

/// Program table component showing the program lines
#[component]
fn ProgramTable(
    lines: RwSignal<Vec<ProgramLine>>,
    executing: RwSignal<i32>,
) -> impl IntoView {
    view! {
        <div class="flex-1 overflow-y-auto">
            <Show
                when=move || !lines.get().is_empty()
                fallback=|| view! {
                    <div class="text-[#555555] text-[9px] text-center py-4 px-2">
                        "No program loaded. Click 'Load' to select a program."
                    </div>
                }
            >
                <table class="w-full text-[9px]">
                    <thead class="sticky top-0 bg-[#0d0d0d]">
                        <tr class="text-[#666666] border-b border-[#ffffff08]">
                            <th class="text-left px-1.5 py-1 w-8">"#"</th>
                            <th class="text-right px-1.5 py-1">"X"</th>
                            <th class="text-right px-1.5 py-1">"Y"</th>
                            <th class="text-right px-1.5 py-1">"Z"</th>
                            <th class="text-right px-1.5 py-1">"W"</th>
                            <th class="text-right px-1.5 py-1">"P"</th>
                            <th class="text-right px-1.5 py-1">"R"</th>
                            <th class="text-right px-1.5 py-1">"Spd"</th>
                            <th class="text-center px-1.5 py-1">"Term"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <For
                            each=move || lines.get()
                            key=|line| line.line_number
                            children=move |line| {
                                let line_num = line.line_number;
                                let term = line.term_type.clone();
                                view! {
                                    <tr class=move || format!(
                                        "border-b border-[#ffffff05] {}",
                                        if executing.get() == line_num as i32 { "bg-[#00d9ff20] text-[#00d9ff]" } else { "text-[#cccccc]" }
                                    )>
                                        <td class="px-1.5 py-0.5 text-[#555555] font-mono">{line_num}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums">{format!("{:.2}", line.x)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums">{format!("{:.2}", line.y)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums">{format!("{:.2}", line.z)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums text-[#888888]">{format!("{:.1}", line.w)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums text-[#888888]">{format!("{:.1}", line.p)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums text-[#888888]">{format!("{:.1}", line.r)}</td>
                                        <td class="px-1.5 py-0.5 text-right font-mono tabular-nums">{format!("{:.0}", line.speed)}</td>
                                        <td class="px-1.5 py-0.5 text-center">{term}</td>
                                    </tr>
                                }
                            }
                        />
                    </tbody>
                </table>
            </Show>
        </div>
    }
}

use crate::websocket::ExecutionStatusData;

/// Program progress bar component
#[component]
fn ProgramProgressBar(
    current_line: Signal<usize>,
    total_lines: Signal<usize>,
    execution_status: ReadSignal<Option<ExecutionStatusData>>,
) -> impl IntoView {
    let progress_percent = move || {
        let total = total_lines.get();
        if total == 0 {
            0.0
        } else {
            (current_line.get() as f64 / total as f64) * 100.0
        }
    };

    view! {
        <div class="px-2 py-1 border-b border-[#ffffff08] bg-[#0d0d0d]">
            <div class="flex items-center gap-2">
                // Status indicator
                {move || execution_status.get().map(|status| {
                    let status_class = match status.status.as_str() {
                        "running" => "text-[#22c55e]",
                        "paused" => "text-[#f59e0b]",
                        "error" => "text-[#ff4444]",
                        _ => "text-[#666666]",
                    };
                    view! {
                        <span class=format!("text-[8px] font-medium uppercase {}", status_class)>
                            {status.status}
                        </span>
                    }
                })}
                <div class="flex-1 h-1.5 bg-[#1a1a1a] rounded-full overflow-hidden">
                    <div
                        class="h-full bg-gradient-to-r from-[#00d9ff] to-[#22c55e] transition-all duration-300"
                        style=move || format!("width: {}%", progress_percent())
                    />
                </div>
                <span class="text-[8px] text-[#666666] font-mono tabular-nums min-w-[60px] text-right">
                    {move || format!("{} / {} ({:.0}%)", current_line.get(), total_lines.get(), progress_percent())}
                </span>
            </div>
            // Error message if present
            {move || execution_status.get().and_then(|s| s.error).map(|err| view! {
                <div class="text-[8px] text-[#ff4444] mt-1">{err}</div>
            })}
        </div>
    }
}
