//! Programs module - Program management and editing.
//!
//! This module contains components for:
//! - Program browser and selection
//! - Program creation, editing, and deletion
//! - CSV upload for program data
//! - Program preview and details

mod modals;

pub use modals::*;

use leptos::prelude::*;
use leptos::either::Either;
use leptos_router::hooks::use_navigate;
use crate::components::layout::workspace::context::{WorkspaceContext, ProgramLine};
use crate::components::layout::LayoutContext;
use crate::websocket::WebSocketManager;

/// Programs view (toolpath creation and editing).
#[component]
pub fn ProgramsView() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");

    let (show_new_program, set_show_new_program) = signal(false);
    let (show_csv_upload, set_show_csv_upload) = signal(false);
    let (show_open_modal, set_show_open_modal) = signal(false);
    let (show_save_as_modal, set_show_save_as_modal) = signal(false);
    let (selected_program_id, set_selected_program_id) = signal::<Option<i64>>(None);

    // Menu dropdown states
    let (show_file_menu, set_show_file_menu) = signal(false);
    let (show_view_menu, set_show_view_menu) = signal(false);

    // Load programs on mount (wait for WebSocket connection)
    Effect::new(move |_| {
        if ws.connected.get() {
            ws.list_programs();
        }
    });

    // Derive programs from WebSocket manager
    let programs = ws.programs;
    let current_program = ws.current_program;

    // When a program is selected, fetch its details
    Effect::new(move |_| {
        if let Some(id) = selected_program_id.get() {
            ws.get_program(id);
        }
    });

    view! {
        <div class="h-full flex flex-col">
            // Menu bar
            <div class="h-7 border-b border-[#ffffff08] flex items-center px-2 shrink-0 bg-[#0d0d0d]">
                // File menu
                <FileMenu
                    show_file_menu=show_file_menu
                    set_show_file_menu=set_show_file_menu
                    set_show_view_menu=set_show_view_menu
                    set_show_new_program=set_show_new_program
                    set_show_open_modal=set_show_open_modal
                    set_show_save_as_modal=set_show_save_as_modal
                    set_show_csv_upload=set_show_csv_upload
                    selected_program_id=selected_program_id
                    set_selected_program_id=set_selected_program_id
                    current_program=current_program
                />

                // View menu
                <ViewMenu
                    show_view_menu=show_view_menu
                    set_show_view_menu=set_show_view_menu
                    set_show_file_menu=set_show_file_menu
                />

                // Spacer
                <div class="flex-1"></div>

                // Current program indicator
                {move || current_program.get().map(|prog| view! {
                    <span class="text-[9px] text-[#666666]">
                        "Current: "
                        <span class="text-[#00d9ff]">{prog.name}</span>
                    </span>
                })}
            </div>

            // Main content area
            <div class="flex-1 p-2 flex gap-2 min-h-0">
                // Left: Program browser (conditionally shown)
                <Show when=move || layout_ctx.show_program_browser.get()>
                    <ProgramBrowser
                        programs=programs
                        selected_program_id=selected_program_id
                        set_selected_program_id=set_selected_program_id
                    />
                </Show>

                // Right: Program details
                <ProgramDetails
                    current_program=current_program
                    selected_program_id=selected_program_id
                    set_selected_program_id=set_selected_program_id
                    set_show_csv_upload=set_show_csv_upload
                    set_show_open_modal=set_show_open_modal
                    set_show_new_program=set_show_new_program
                    ctx=ctx
                />
            </div>

            // Modals
            <Show when=move || show_new_program.get()>
                <NewProgramModal
                    on_close=move || set_show_new_program.set(false)
                    on_created=move |id| {
                        set_show_new_program.set(false);
                        set_selected_program_id.set(Some(id));
                        ws.list_programs();
                        ws.get_program(id);
                    }
                />
            </Show>

            <Show when=move || show_open_modal.get()>
                <OpenProgramModal
                    on_close=move || set_show_open_modal.set(false)
                    on_selected=move |id| {
                        set_show_open_modal.set(false);
                        set_selected_program_id.set(Some(id));
                        ws.get_program(id);
                    }
                />
            </Show>

            <Show when=move || show_save_as_modal.get()>
                <SaveAsProgramModal
                    on_close=move || set_show_save_as_modal.set(false)
                    on_saved=move |id| {
                        set_show_save_as_modal.set(false);
                        set_selected_program_id.set(Some(id));
                        ws.list_programs();
                        ws.get_program(id);
                    }
                />
            </Show>

            <Show when=move || show_csv_upload.get() && selected_program_id.get().is_some()>
                {move || selected_program_id.get().map(|prog_id| view! {
                    <CSVUploadModal
                        program_id=prog_id
                        on_close=move || set_show_csv_upload.set(false)
                        on_uploaded=move || {
                            set_show_csv_upload.set(false);
                            ws.get_program(prog_id);
                            ws.list_programs();
                        }
                    />
                })}
            </Show>
        </div>
    }
}

/// File menu dropdown
#[component]
fn FileMenu(
    show_file_menu: ReadSignal<bool>,
    set_show_file_menu: WriteSignal<bool>,
    set_show_view_menu: WriteSignal<bool>,
    set_show_new_program: WriteSignal<bool>,
    set_show_open_modal: WriteSignal<bool>,
    set_show_save_as_modal: WriteSignal<bool>,
    set_show_csv_upload: WriteSignal<bool>,
    selected_program_id: ReadSignal<Option<i64>>,
    set_selected_program_id: WriteSignal<Option<i64>>,
    current_program: ReadSignal<Option<crate::websocket::ProgramDetail>>,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    view! {
        <div class="relative">
            <button
                class={move || format!(
                    "px-2 py-1 text-[10px] rounded transition-colors {}",
                    if show_file_menu.get() { "bg-[#ffffff10] text-white" } else { "text-[#888888] hover:text-white hover:bg-[#ffffff08]" }
                )}
                on:click=move |_| {
                    set_show_file_menu.update(|v| *v = !*v);
                    set_show_view_menu.set(false);
                }
            >
                "File"
            </button>
            {move || if show_file_menu.get() {
                view! {
                    <div class="absolute left-0 top-full mt-0.5 w-40 bg-[#1a1a1a] border border-[#ffffff15] rounded shadow-lg z-50">
                        <button
                            class="w-full text-left px-3 py-1.5 text-[10px] text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white flex items-center gap-2"
                            on:click=move |_| {
                                set_show_new_program.set(true);
                                set_show_file_menu.set(false);
                            }
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"/>
                            </svg>
                            "New Program"
                        </button>
                        <button
                            class="w-full text-left px-3 py-1.5 text-[10px] text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white flex items-center gap-2"
                            on:click=move |_| {
                                set_show_open_modal.set(true);
                                set_show_file_menu.set(false);
                            }
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                            </svg>
                            "Open..."
                        </button>
                        <div class="border-t border-[#ffffff10] my-1"></div>
                        <button
                            class={move || format!(
                                "w-full text-left px-3 py-1.5 text-[10px] flex items-center gap-2 {}",
                                if current_program.get().is_some() {
                                    "text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white"
                                } else {
                                    "text-[#444444] cursor-not-allowed"
                                }
                            )}
                            on:click=move |_| {
                                if current_program.get().is_some() {
                                    set_show_save_as_modal.set(true);
                                    set_show_file_menu.set(false);
                                }
                            }
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"/>
                            </svg>
                            "Save As..."
                        </button>
                        <div class="border-t border-[#ffffff10] my-1"></div>
                        <button
                            class={move || format!(
                                "w-full text-left px-3 py-1.5 text-[10px] flex items-center gap-2 {}",
                                if selected_program_id.get().is_some() {
                                    "text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white"
                                } else {
                                    "text-[#444444] cursor-not-allowed"
                                }
                            )}
                            on:click=move |_| {
                                if selected_program_id.get().is_some() {
                                    set_show_csv_upload.set(true);
                                    set_show_file_menu.set(false);
                                }
                            }
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
                            </svg>
                            "Upload CSV..."
                        </button>
                        <div class="border-t border-[#ffffff10] my-1"></div>
                        <button
                            class={move || format!(
                                "w-full text-left px-3 py-1.5 text-[10px] flex items-center gap-2 {}",
                                if selected_program_id.get().is_some() || current_program.get().is_some() {
                                    "text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white"
                                } else {
                                    "text-[#444444] cursor-not-allowed"
                                }
                            )}
                            disabled=move || selected_program_id.get().is_none() && current_program.get().is_none()
                            on:click=move |_| {
                                set_selected_program_id.set(None);
                                ws.clear_current_program();
                                set_show_file_menu.set(false);
                            }
                        >
                            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                            </svg>
                            "Close"
                        </button>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}
        </div>
    }
}

/// View menu dropdown
#[component]
fn ViewMenu(
    show_view_menu: ReadSignal<bool>,
    set_show_view_menu: WriteSignal<bool>,
    set_show_file_menu: WriteSignal<bool>,
) -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");

    view! {
        <div class="relative">
            <button
                class={move || format!(
                    "px-2 py-1 text-[10px] rounded transition-colors {}",
                    if show_view_menu.get() { "bg-[#ffffff10] text-white" } else { "text-[#888888] hover:text-white hover:bg-[#ffffff08]" }
                )}
                on:click=move |_| {
                    set_show_view_menu.update(|v| *v = !*v);
                    set_show_file_menu.set(false);
                }
            >
                "View"
            </button>
            {move || if show_view_menu.get() {
                view! {
                    <div class="absolute left-0 top-full mt-0.5 w-48 bg-[#1a1a1a] border border-[#ffffff15] rounded shadow-lg z-50">
                        <button
                            class="w-full text-left px-3 py-1.5 text-[10px] text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white flex items-center justify-between"
                            on:click=move |_| {
                                layout_ctx.show_program_browser.update(|v| *v = !*v);
                                set_show_view_menu.set(false);
                            }
                        >
                            <span class="flex items-center gap-2">
                                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h7"/>
                                </svg>
                                "Program Browser"
                            </span>
                            {move || if layout_ctx.show_program_browser.get() {
                                view! { <span class="text-[#00d9ff]">"✓"</span> }.into_any()
                            } else {
                                view! { <span></span> }.into_any()
                            }}
                        </button>
                    </div>
                }.into_any()
            } else {
                view! { <div></div> }.into_any()
            }}
        </div>
    }
}

/// Program browser sidebar
#[component]
fn ProgramBrowser(
    programs: ReadSignal<Vec<crate::websocket::ProgramInfo>>,
    selected_program_id: ReadSignal<Option<i64>>,
    set_selected_program_id: WriteSignal<Option<i64>>,
) -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");

    view! {
        <div class="w-64 bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden shrink-0">
            <div class="flex items-center justify-between p-2 border-b border-[#ffffff08]">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                    </svg>
                    "Programs"
                </h3>
                <button
                    class="text-[#666666] hover:text-white"
                    on:click=move |_| layout_ctx.show_program_browser.set(false)
                    title="Close browser"
                >
                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                    </svg>
                </button>
            </div>
            <div class="flex-1 overflow-y-auto p-1.5 space-y-1">
                {move || {
                    let progs = programs.get();
                    if progs.is_empty() {
                        Either::Left(view! {
                            <div class="text-[#555555] text-[9px] text-center py-4">
                                "No programs saved"
                            </div>
                        })
                    } else {
                        Either::Right(progs.into_iter().map(|prog| {
                            let is_selected = move || selected_program_id.get() == Some(prog.id);
                            let prog_id = prog.id;
                            let prog_name = prog.name.clone();
                            let lines_str = format!("{} lines", prog.instruction_count);
                            view! {
                                <button
                                    class={move || format!(
                                        "w-full text-left p-2 rounded border text-[9px] transition-colors {}",
                                        if is_selected() {
                                            "bg-[#00d9ff10] border-[#00d9ff40] text-white"
                                        } else {
                                            "bg-[#111111] border-[#ffffff08] text-[#888888] hover:border-[#ffffff20]"
                                        }
                                    )}
                                    on:click=move |_| set_selected_program_id.set(Some(prog_id))
                                >
                                    <div class="font-medium text-[10px] mb-0.5">{prog_name}</div>
                                    <div class="text-[#555555]">{lines_str}</div>
                                </button>
                            }
                        }).collect_view())
                    }
                }}
            </div>
        </div>
    }
}

/// Program details panel
#[component]
fn ProgramDetails(
    current_program: ReadSignal<Option<crate::websocket::ProgramDetail>>,
    #[allow(unused_variables)]
    selected_program_id: ReadSignal<Option<i64>>,
    set_selected_program_id: WriteSignal<Option<i64>>,
    set_show_csv_upload: WriteSignal<bool>,
    set_show_open_modal: WriteSignal<bool>,
    set_show_new_program: WriteSignal<bool>,
    ctx: WorkspaceContext,
) -> impl IntoView {
    let navigate = use_navigate();
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    view! {
        <div class="flex-1 bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden">
            {move || {
                if let Some(prog) = current_program.get() {
                    let prog_id = prog.id;
                    let prog_name = prog.name.clone();
                    let prog_name_for_load = prog.name.clone();
                    let prog_desc = prog.description.clone().unwrap_or_default();
                    let prog_created = "N/A".to_string();
                    let line_count = prog.instructions.len();
                    let speed_str = prog.instructions.first()
                        .and_then(|i| i.speed)
                        .map(|s| format!("{} mm/s", s))
                        .unwrap_or_else(|| "N/A".to_string());
                    let frame_tool_str = "N/A".to_string();

                    let preview_lines: Vec<String> = prog.instructions.iter()
                        .take(5)
                        .map(|i| {
                            let term = i.term_type.clone().unwrap_or_else(|| "CNT100".to_string());
                            let spd = i.speed.unwrap_or(100.0);
                            format!("{:03}: L P[{}] {:.0}mm/sec {}", i.line_number, i.line_number, spd, term)
                        })
                        .collect();

                    let program_lines_for_load: Vec<ProgramLine> = prog.instructions.iter()
                        .map(|i| ProgramLine {
                            line_number: i.line_number as usize,
                            x: i.x,
                            y: i.y,
                            z: i.z,
                            w: i.w.unwrap_or(0.0),
                            p: i.p.unwrap_or(0.0),
                            r: i.r.unwrap_or(0.0),
                            speed: i.speed.unwrap_or(100.0),
                            term_type: i.term_type.clone().unwrap_or_else(|| "CNT100".to_string()),
                        })
                        .collect();

                    Either::Left(view! {
                        <div class="h-full flex flex-col">
                            // Header
                            <div class="p-3 border-b border-[#ffffff08]">
                                <div class="flex items-start justify-between">
                                    <div>
                                        <h2 class="text-sm font-semibold text-white">{prog_name}</h2>
                                        <p class="text-[#666666] text-[9px] mt-0.5">{prog_desc}</p>
                                    </div>
                                    <div class="flex gap-1">
                                        <button
                                            class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[9px] px-2 py-1 rounded hover:bg-[#22c55e30]"
                                            on:click={
                                                let lines = program_lines_for_load.clone();
                                                let name = prog_name_for_load.clone();
                                                let nav = navigate.clone();
                                                move |_| {
                                                    ctx.program_lines.set(lines.clone());
                                                    ctx.loaded_program_name.set(Some(name.clone()));
                                                    ctx.loaded_program_id.set(Some(prog_id));
                                                    ctx.executing_line.set(-1);
                                                    nav("/dashboard/control", Default::default());
                                                }
                                            }
                                        >
                                            "▶ Load to Dashboard"
                                        </button>
                                        <button
                                            class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[9px] px-2 py-1 rounded hover:bg-[#00d9ff30]"
                                            on:click=move |_| set_show_csv_upload.set(true)
                                        >
                                            "⬆ Upload CSV"
                                        </button>
                                        <button
                                            class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[9px] px-2 py-1 rounded hover:bg-[#ff444430]"
                                            on:click=move |_| {
                                                ws.delete_program(prog_id);
                                                set_selected_program_id.set(None);
                                                ws.list_programs();
                                            }
                                        >
                                            "Delete"
                                        </button>
                                    </div>
                                </div>
                            </div>

                            // Metadata
                            <div class="p-3 border-b border-[#ffffff08] grid grid-cols-4 gap-3">
                                <div>
                                    <div class="text-[8px] text-[#555555] uppercase">"Lines"</div>
                                    <div class="text-[11px] text-white font-mono">{line_count}</div>
                                </div>
                                <div>
                                    <div class="text-[8px] text-[#555555] uppercase">"Speed"</div>
                                    <div class="text-[11px] text-white font-mono">{speed_str}</div>
                                </div>
                                <div>
                                    <div class="text-[8px] text-[#555555] uppercase">"Frame/Tool"</div>
                                    <div class="text-[11px] text-white font-mono">{frame_tool_str}</div>
                                </div>
                                <div>
                                    <div class="text-[8px] text-[#555555] uppercase">"Created"</div>
                                    <div class="text-[11px] text-white font-mono">{prog_created}</div>
                                </div>
                            </div>

                            // Preview area
                            <div class="flex-1 p-3 overflow-auto">
                                <h4 class="text-[9px] text-[#666666] uppercase mb-2">"Program Preview"</h4>
                                <div class="bg-[#111111] rounded border border-[#ffffff08] p-2 font-mono text-[9px] text-[#888888]">
                                    {if preview_lines.is_empty() {
                                        Either::Left(view! {
                                            <div class="text-[#555555]">"No instructions"</div>
                                        })
                                    } else {
                                        Either::Right(view! {
                                            <div class="text-[#555555]">"// First 5 lines preview"</div>
                                            {preview_lines.into_iter().map(|line| view! {
                                                <div>{line}</div>
                                            }).collect_view()}
                                            {if line_count > 5 {
                                                Either::Left(view! { <div class="text-[#555555]">"..."</div> })
                                            } else {
                                                Either::Right(())
                                            }}
                                        })
                                    }}
                                </div>
                            </div>
                        </div>
                    })
                } else {
                    Either::Right(view! {
                        <div class="h-full flex items-center justify-center">
                            <div class="text-center">
                                <svg class="w-12 h-12 mx-auto mb-2 text-[#333333]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
                                </svg>
                                <p class="text-[#555555] text-[10px] mb-3">"No program open"</p>
                                <div class="flex gap-2 justify-center">
                                    <button
                                        class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[9px] px-3 py-1.5 rounded hover:bg-[#00d9ff30]"
                                        on:click=move |_| set_show_open_modal.set(true)
                                    >
                                        "Open Program"
                                    </button>
                                    <button
                                        class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[9px] px-3 py-1.5 rounded hover:bg-[#22c55e30]"
                                        on:click=move |_| set_show_new_program.set(true)
                                    >
                                        "New Program"
                                    </button>
                                </div>
                            </div>
                        </div>
                    })
                }
            }}
        </div>
    }
}

