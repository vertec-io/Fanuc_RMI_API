//! Load Program Modal - select a program to load.

use leptos::prelude::*;
use leptos::either::Either;
use crate::components::layout::workspace::context::{WorkspaceContext, ProgramLine};
use crate::websocket::WebSocketManager;

/// Load Program Modal - select a program to load
#[component]
pub fn LoadProgramModal(on_close: impl Fn() + 'static + Clone) -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let programs = ws.programs;
    let (selected_id, set_selected_id) = signal::<Option<i64>>(None);
    let (loading, set_loading) = signal(false);

    // Refresh programs list when modal opens (wait for WebSocket connection)
    Effect::new(move |_| {
        if ws.connected.get() {
            ws.list_programs();
        }
    });

    let on_close_clone = on_close.clone();

    view! {
        <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50">
            <div class="bg-[#0d0d0d] border border-[#ffffff15] rounded-lg w-[400px] max-h-[500px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff10]">
                    <h2 class="text-[11px] font-semibold text-[#00d9ff]">"Load Program"</h2>
                    <button
                        class="text-[#666666] hover:text-white text-[14px]"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        "✕"
                    </button>
                </div>

                // Program list
                <div class="flex-1 overflow-y-auto p-2">
                    {move || {
                        let progs = programs.get();
                        if progs.is_empty() {
                            Either::Left(view! {
                                <div class="text-[#555555] text-[9px] text-center py-4">
                                    "No programs available. Create one in the Control tab."
                                </div>
                            })
                        } else {
                            Either::Right(view! {
                                <div class="space-y-1">
                                    {progs.into_iter().map(|prog| {
                                        let prog_id = prog.id;
                                        let prog_name = prog.name.clone();
                                        let prog_desc = prog.description.clone();
                                        let line_count = prog.instruction_count;
                                        view! {
                                            <button
                                                class={move || format!(
                                                    "w-full text-left p-2 rounded border {} transition-colors",
                                                    if selected_id.get() == Some(prog_id) {
                                                        "bg-[#00d9ff20] border-[#00d9ff40] text-[#00d9ff]"
                                                    } else {
                                                        "bg-[#0a0a0a] border-[#ffffff08] text-[#cccccc] hover:bg-[#ffffff08]"
                                                    }
                                                )}
                                                on:click=move |_| set_selected_id.set(Some(prog_id))
                                            >
                                                <div class="text-[10px] font-medium">{prog_name}</div>
                                                <div class="text-[8px] text-[#666666] mt-0.5">
                                                    {prog_desc.unwrap_or_else(|| "No description".to_string())}
                                                </div>
                                                <div class="text-[8px] text-[#555555] mt-0.5">
                                                    {format!("{} instructions", line_count)}
                                                </div>
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                            })
                        }
                    }}
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff10]">
                    <button
                        class="text-[9px] px-3 py-1.5 rounded bg-[#1a1a1a] text-[#888888] hover:bg-[#222222]"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class="text-[9px] px-3 py-1.5 rounded bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] hover:bg-[#00d9ff30] disabled:opacity-50 disabled:cursor-not-allowed"
                        disabled=move || selected_id.get().is_none() || loading.get()
                        on:click=move |_| {
                            if let Some(id) = selected_id.get() {
                                // Send LoadProgram to server (requires control)
                                // Server will broadcast ExecutionStateChanged with program_id
                                ws.load_program(id);
                                // Also fetch program details for local display
                                ws.get_program(id);
                                set_loading.set(true);
                            }
                        }
                    >
                        {move || if loading.get() { "Loading..." } else { "Load Program" }}
                    </button>
                </div>
            </div>
        </div>

        // Effect to handle program loaded - wait for program details to arrive
        {
            let on_close = on_close_clone.clone();
            Effect::new(move |_| {
                if loading.get() {
                    if let Some(detail) = ws.current_program.get() {
                        // Convert instructions to ProgramLine format for local display
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
                        ctx.loaded_program_id.set(Some(detail.id));
                        ctx.executing_line.set(-1);
                        set_loading.set(false);
                        on_close();
                    }
                }
            });
        }
    }
}

