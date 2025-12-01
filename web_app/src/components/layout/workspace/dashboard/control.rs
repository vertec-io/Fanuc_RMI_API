//! Dashboard Control tab - Robot control and program execution.
//!
//! Contains components for quick commands, command composition,
//! console logging, and program execution visualization.

use leptos::prelude::*;
use leptos::either::Either;
use crate::components::layout::workspace::context::{WorkspaceContext, CommandLogEntry, CommandStatus, RecentCommand, ProgramLine};
use crate::websocket::WebSocketManager;
use fanuc_rmi::dto::{SendPacket, Instruction, Command, FrcLinearRelative, FrcLinearMotion, FrcJointMotion, FrcInitialize, Configuration, Position};
use fanuc_rmi::{SpeedType, TermType};

/// Control tab content (command composer).
#[component]
pub fn ControlTab() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let _ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let show_composer = ctx.show_composer;

    view! {
        <div class="h-full flex flex-col gap-2">
            // Quick Commands section
            <QuickCommandsPanel/>

            // Command input section
            <CommandInputSection/>

            // Two-column layout for Command Log and Program Display
            <div class="flex-1 grid grid-cols-2 gap-2 min-h-0">
                <CommandLogPanel/>
                <ProgramVisualDisplay/>
            </div>

            // Command Composer Modal
            <Show when=move || show_composer.get()>
                <CommandComposerModal/>
            </Show>
        </div>
    }
}

/// Quick Commands panel for robot control (Initialize, Reset, Abort, Continue).
#[component]
fn QuickCommandsPanel() -> impl IntoView {
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

/// Helper function to create a motion packet from a RecentCommand
fn create_motion_packet(cmd: &RecentCommand) -> SendPacket {
    let config = Configuration {
        u_tool_number: cmd.utool,
        u_frame_number: cmd.uframe,
        front: 1,
        up: 1,
        left: 0,
        flip: 0,
        turn4: 0,
        turn5: 0,
        turn6: 0,
    };
    let position = Position {
        x: cmd.x,
        y: cmd.y,
        z: cmd.z,
        w: cmd.w,
        p: cmd.p,
        r: cmd.r,
        ext1: 0.0,
        ext2: 0.0,
        ext3: 0.0,
    };
    let speed_type = SpeedType::MMSec;
    let term_type = if cmd.term_type == "FINE" { TermType::FINE } else { TermType::CNT };
    let term_value = if cmd.term_type == "FINE" { 0 } else { 100 };

    match cmd.command_type.as_str() {
        "linear_rel" => SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative {
            sequence_id: 0,
            configuration: config,
            position,
            speed_type,
            speed: cmd.speed,
            term_type,
            term_value,
        })),
        "linear_abs" => SendPacket::Instruction(Instruction::FrcLinearMotion(FrcLinearMotion {
            sequence_id: 0,
            configuration: config,
            position,
            speed_type,
            speed: cmd.speed,
            term_type,
            term_value,
        })),
        "joint" => SendPacket::Instruction(Instruction::FrcJointMotion(FrcJointMotion {
            sequence_id: 0,
            configuration: config,
            position,
            speed_type,
            speed: cmd.speed,
            term_type,
            term_value,
        })),
        _ => SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative {
            sequence_id: 0,
            configuration: config,
            position,
            speed_type,
            speed: cmd.speed,
            term_type,
            term_value,
        })),
    }
}

/// Command input section with recent commands and composer button
#[component]
fn CommandInputSection() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let recent_commands = ctx.recent_commands;
    let selected_cmd_id = ctx.selected_command_id;

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2 shrink-0">
            <div class="flex items-center justify-between mb-1.5">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                    "Recent Commands"
                </h3>
                <button
                    class="text-[8px] text-[#666666] hover:text-[#ff4444]"
                    on:click=move |_| {
                        recent_commands.set(Vec::new());
                        selected_cmd_id.set(None);
                    }
                    title="Clear all recent commands"
                >
                    "Clear"
                </button>
            </div>
            <div class="flex gap-1">
                <select
                    class="flex-1 bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                    prop:value=move || selected_cmd_id.get().map(|id| id.to_string()).unwrap_or_default()
                    on:change=move |ev| {
                        let val = event_target_value(&ev);
                        selected_cmd_id.set(val.parse().ok());
                    }
                >
                    <option value="">{move || {
                        let count = recent_commands.get().len();
                        if count == 0 {
                            "No recent commands - use Composer to create".to_string()
                        } else {
                            format!("Select from {} recent commands...", count)
                        }
                    }}</option>
                    {move || recent_commands.get().into_iter().map(|cmd| {
                        view! {
                            <option value={cmd.id.to_string()}>
                                {format!("{} - {}", cmd.name, cmd.description)}
                            </option>
                        }
                    }).collect_view()}
                </select>
                <button
                    class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[9px] px-3 py-1 rounded hover:bg-[#00d9ff30]"
                    on:click=move |_| ctx.show_composer.set(true)
                >
                    "+ Compose"
                </button>
                <button
                    class={move || format!(
                        "text-[9px] px-3 py-1 rounded transition-colors {}",
                        if selected_cmd_id.get().is_none() {
                            "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
                        } else {
                            "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                        }
                    )}
                    disabled=move || selected_cmd_id.get().is_none()
                    on:click=move |_| {
                        if let Some(idx) = selected_cmd_id.get() {
                            // Find the command by ID
                            let cmds = recent_commands.get();
                            if let Some(cmd) = cmds.iter().find(|c| c.id == idx) {
                                // Create and send the motion packet
                                let packet = create_motion_packet(cmd);
                                ws.send_command(packet);

                                // Add to command log
                                ctx.command_log.update(|log| {
                                    log.push(CommandLogEntry {
                                        timestamp: js_sys::Date::new_0().to_locale_time_string("en-US").as_string().unwrap_or_else(|| "??:??:??".to_string()),
                                        command: cmd.name.clone(),
                                        status: CommandStatus::Pending,
                                    });
                                });
                            }
                        }
                    }
                >
                    "▶ Run"
                </button>
            </div>
        </div>
    }
}

/// Console Log Panel - unified console showing commands, motion events, and errors
#[component]
fn CommandLogPanel() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let command_log = ctx.command_log;
    let motion_log = ws.motion_log;
    let error_log = ws.error_log;

    // Merge all logs into a unified view
    let clear_all = move |_| {
        command_log.set(Vec::new());
        ws.clear_motion_log();
        ws.clear_error_log();
    };

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden">
            <div class="flex items-center justify-between p-2 border-b border-[#ffffff08] shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z"/>
                    </svg>
                    "Console"
                    {move || {
                        let total = command_log.get().len() + motion_log.get().len() + error_log.get().len();
                        if total > 0 {
                            Some(view! {
                                <span class="ml-1.5 bg-[#00d9ff20] text-[#00d9ff] text-[8px] px-1 py-0.5 rounded font-medium">
                                    {total}
                                </span>
                            })
                        } else {
                            None
                        }
                    }}
                </h3>
                <button
                    class="text-[8px] text-[#666666] hover:text-white"
                    on:click=clear_all
                >
                    "Clear"
                </button>
            </div>
            <div class="flex-1 overflow-y-auto p-2 space-y-0.5 font-mono">
                {move || {
                    let commands = command_log.get();
                    let motions = motion_log.get();
                    let errors = error_log.get();

                    if commands.is_empty() && motions.is_empty() && errors.is_empty() {
                        Either::Left(view! {
                            <div class="text-[#555555] text-[9px] text-center py-4">
                                "Console ready"
                            </div>
                        })
                    } else {
                        // Show command entries
                        let cmd_views = commands.into_iter().map(|entry| {
                            let status_class = match &entry.status {
                                CommandStatus::Pending => "text-[#ffaa00]",
                                CommandStatus::Success => "text-[#22c55e]",
                                CommandStatus::Error(_) => "text-[#ff4444]",
                            };
                            let status_icon = match &entry.status {
                                CommandStatus::Pending => "⏳",
                                CommandStatus::Success => "✓",
                                CommandStatus::Error(_) => "✗",
                            };
                            let timestamp = entry.timestamp.clone();
                            let command = entry.command.clone();
                            view! {
                                <div class="flex items-start gap-1.5 text-[9px] py-0.5 border-b border-[#ffffff05]">
                                    <span class="text-[#555555] shrink-0">{timestamp}</span>
                                    <span class={status_class}>{status_icon}</span>
                                    <span class="text-[#cccccc] break-all">{command}</span>
                                </div>
                            }
                        }).collect_view();

                        // Show motion log entries (most recent last)
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

/// Program Visual Display - G-code style line-by-line view
#[component]
fn ProgramVisualDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let lines = ctx.program_lines;
    let executing = ctx.executing_line;
    let loaded_name = ctx.loaded_program_name;
    let loaded_id = ctx.loaded_program_id;
    let is_running = ctx.program_running;
    let is_paused = ctx.program_paused;
    let (show_load_modal, set_show_load_modal) = signal(false);

    // Sync WebSocket program_running state with context
    Effect::new(move |_| {
        let ws_running = ws.program_running.get();
        if is_running.get() != ws_running {
            ctx.program_running.set(ws_running);
            if !ws_running {
                // Program completed - reset paused state and executing line
                ctx.program_paused.set(false);
                ctx.executing_line.set(-1);
            }
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
                                        ws.start_program(id);
                                        ctx.program_running.set(true);
                                        ctx.program_paused.set(false);
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
                                                ws.resume_program();
                                                ctx.program_paused.set(false);
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
                                                ws.pause_program();
                                                ctx.program_paused.set(true);
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
                                    ws.stop_program();
                                    ctx.program_running.set(false);
                                    ctx.program_paused.set(false);
                                    ctx.executing_line.set(-1);
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
                                ws.stop_program();
                                ctx.program_lines.set(Vec::new());
                                ctx.loaded_program_name.set(None);
                                ctx.loaded_program_id.set(None);
                                ctx.executing_line.set(-1);
                                ctx.program_running.set(false);
                                ctx.program_paused.set(false);
                            }
                            title="Unload program from dashboard"
                        >
                            <svg class="w-2.5 h-2.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"/>
                            </svg>
                            "Unload"
                        </button>
                    </Show>
                </div>
            </div>
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
        </div>

        // Load Program Modal
        <Show when=move || show_load_modal.get()>
            <LoadProgramModal on_close=move || set_show_load_modal.set(false)/>
        </Show>
    }
}

/// Load Program Modal - select a program to load
#[component]
fn LoadProgramModal(on_close: impl Fn() + 'static + Clone) -> impl IntoView {
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
                                set_loading.set(true);
                                ws.get_program(id);
                            }
                        }
                    >
                        {move || if loading.get() { "Loading..." } else { "Load Program" }}
                    </button>
                </div>
            </div>
        </div>

        // Effect to handle program loaded
        {
            let on_close = on_close_clone.clone();
            Effect::new(move |_| {
                if loading.get() {
                    if let Some(detail) = ws.current_program.get() {
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

/// Command Composer Modal - wizard for creating motion commands
#[component]
fn CommandComposerModal() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");

    // Check if we're editing an existing command
    let editing_cmd = ctx.selected_command_id.get().and_then(|id| {
        ctx.recent_commands.get().into_iter().find(|c| c.id == id)
    });

    // Initialize values from selected command or use defaults
    let (step, set_step) = signal(1usize);
    let (cmd_type, set_cmd_type) = signal(
        editing_cmd.as_ref().map(|c| c.command_type.clone()).unwrap_or_else(|| "linear_rel".to_string())
    );

    // Position inputs - initialize from selected command if editing
    let (x, set_x) = signal(editing_cmd.as_ref().map(|c| c.x).unwrap_or(0.0));
    let (y, set_y) = signal(editing_cmd.as_ref().map(|c| c.y).unwrap_or(0.0));
    let (z, set_z) = signal(editing_cmd.as_ref().map(|c| c.z).unwrap_or(0.0));
    let (w, set_w) = signal(editing_cmd.as_ref().map(|c| c.w).unwrap_or(0.0));
    let (p, set_p) = signal(editing_cmd.as_ref().map(|c| c.p).unwrap_or(0.0));
    let (r, set_r) = signal(editing_cmd.as_ref().map(|c| c.r).unwrap_or(0.0));

    // Config inputs - initialize from selected command if editing
    let (speed, set_speed) = signal(editing_cmd.as_ref().map(|c| c.speed).unwrap_or(50.0));
    let (term_type, set_term_type) = signal(
        editing_cmd.as_ref().map(|c| c.term_type.clone()).unwrap_or_else(|| "CNT".to_string())
    );
    let (uframe, set_uframe) = signal(editing_cmd.as_ref().map(|c| c.uframe).unwrap_or(0));
    let (utool, set_utool) = signal(editing_cmd.as_ref().map(|c| c.utool).unwrap_or(0));

    // Track if we're editing (for display purposes)
    let is_editing = editing_cmd.is_some();

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[500px] max-h-[80vh] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/>
                        </svg>
                        {if is_editing { "Edit Command (creates copy)" } else { "Command Composer" }}
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
                        on:click=move |_| ctx.show_composer.set(false)
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Step indicator
                <div class="flex items-center gap-2 px-3 py-2 border-b border-[#ffffff08]">
                    {[1, 2, 3].into_iter().map(|s| {
                        let is_active = move || step.get() == s;
                        let is_complete = move || step.get() > s;
                        let label = match s {
                            1 => "Type",
                            2 => "Parameters",
                            _ => "Preview",
                        };
                        view! {
                            <div class="flex items-center gap-1.5">
                                <div class={move || format!(
                                    "w-5 h-5 rounded-full flex items-center justify-center text-[9px] font-medium {}",
                                    if is_active() { "bg-[#00d9ff] text-black" }
                                    else if is_complete() { "bg-[#22c55e] text-black" }
                                    else { "bg-[#333333] text-[#666666]" }
                                )}>
                                    {s}
                                </div>
                                <span class={move || format!(
                                    "text-[10px] {}",
                                    if is_active() { "text-[#00d9ff]" } else { "text-[#666666]" }
                                )}>
                                    {label}
                                </span>
                                {(s < 3).then(|| view! {
                                    <div class="w-8 h-px bg-[#333333]"></div>
                                })}
                            </div>
                        }
                    }).collect_view()}
                </div>

                // Content - Step 1: Command Type
                <div class="flex-1 overflow-y-auto p-3">
                    {move || match step.get() {
                        1 => Either::Left(Either::Left(view! {
                            <div class="space-y-2">
                                <label class="block text-[10px] text-[#888888] mb-1">"Command Type"</label>
                                <div class="grid grid-cols-2 gap-1">
                                    {["linear_rel", "linear_abs", "joint", "wait"].into_iter().map(|t| {
                                        let label = match t {
                                            "linear_rel" => "Linear (Relative)",
                                            "linear_abs" => "Linear (Absolute)",
                                            "joint" => "Joint Motion",
                                            _ => "Wait Time",
                                        };
                                        let is_selected = move || cmd_type.get() == t;
                                        view! {
                                            <button
                                                class={move || format!(
                                                    "p-2 rounded border text-[10px] text-left {}",
                                                    if is_selected() {
                                                        "bg-[#00d9ff20] border-[#00d9ff] text-[#00d9ff]"
                                                    } else {
                                                        "bg-[#0a0a0a] border-[#ffffff08] text-[#888888] hover:border-[#ffffff20]"
                                                    }
                                                )}
                                                on:click=move |_| set_cmd_type.set(t.to_string())
                                            >
                                                {label}
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        })),
                        2 => Either::Left(Either::Right(view! {
                            // Step 2: Parameters
                            <div class="space-y-3">
                                // Position inputs
                                <div>
                                    <label class="block text-[10px] text-[#888888] mb-1">"Position (mm / degrees)"</label>
                                    <div class="grid grid-cols-3 gap-1">
                                        {[("X", x, set_x), ("Y", y, set_y), ("Z", z, set_z)].into_iter().map(|(label, val, set_val)| {
                                            view! {
                                                <div>
                                                    <label class="block text-[8px] text-[#555555] mb-0.5">{label}</label>
                                                    <input
                                                        type="number"
                                                        step="0.1"
                                                        class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                                        prop:value=move || val.get()
                                                        on:input=move |ev| {
                                                            if let Ok(v) = event_target_value(&ev).parse() {
                                                                set_val.set(v);
                                                            }
                                                        }
                                                    />
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                    <div class="grid grid-cols-3 gap-1 mt-1">
                                        {[("W", w, set_w), ("P", p, set_p), ("R", r, set_r)].into_iter().map(|(label, val, set_val)| {
                                            view! {
                                                <div>
                                                    <label class="block text-[8px] text-[#555555] mb-0.5">{label}</label>
                                                    <input
                                                        type="number"
                                                        step="0.1"
                                                        class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                                        prop:value=move || val.get()
                                                        on:input=move |ev| {
                                                            if let Ok(v) = event_target_value(&ev).parse() {
                                                                set_val.set(v);
                                                            }
                                                        }
                                                    />
                                                </div>
                                            }
                                        }).collect_view()}
                                    </div>
                                </div>

                                // Motion config
                                <div>
                                    <label class="block text-[10px] text-[#888888] mb-1">"Motion Config"</label>
                                    <div class="grid grid-cols-4 gap-1">
                                        <div>
                                            <label class="block text-[8px] text-[#555555] mb-0.5">"Speed"</label>
                                            <input
                                                type="number"
                                                class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                                prop:value=move || speed.get()
                                                on:input=move |ev| {
                                                    if let Ok(v) = event_target_value(&ev).parse() {
                                                        set_speed.set(v);
                                                    }
                                                }
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-[8px] text-[#555555] mb-0.5">"Term"</label>
                                            <select
                                                class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                                on:change=move |ev| set_term_type.set(event_target_value(&ev))
                                            >
                                                <option value="CNT" selected=move || term_type.get() == "CNT">"CNT"</option>
                                                <option value="FINE" selected=move || term_type.get() == "FINE">"FINE"</option>
                                            </select>
                                        </div>
                                        <div>
                                            <label class="block text-[8px] text-[#555555] mb-0.5">"UFrame"</label>
                                            <input
                                                type="number"
                                                min="0" max="9"
                                                class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                                prop:value=move || uframe.get()
                                                on:input=move |ev| {
                                                    if let Ok(v) = event_target_value(&ev).parse() {
                                                        set_uframe.set(v);
                                                    }
                                                }
                                            />
                                        </div>
                                        <div>
                                            <label class="block text-[8px] text-[#555555] mb-0.5">"UTool"</label>
                                            <input
                                                type="number"
                                                min="0" max="9"
                                                class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                                prop:value=move || utool.get()
                                                on:input=move |ev| {
                                                    if let Ok(v) = event_target_value(&ev).parse() {
                                                        set_utool.set(v);
                                                    }
                                                }
                                            />
                                        </div>
                                    </div>
                                </div>
                            </div>
                        })),
                        _ => Either::Right(view! {
                            // Step 3: Preview
                            <div class="space-y-2">
                                <label class="block text-[10px] text-[#888888] mb-1">"Command Preview"</label>
                                <div class="bg-[#0a0a0a] border border-[#ffffff08] rounded p-2 font-mono text-[10px] text-[#00d9ff]">
                                    {move || format!(
                                        "{}(X:{:.2}, Y:{:.2}, Z:{:.2}, W:{:.1}, P:{:.1}, R:{:.1}) @ {}mm/s {}",
                                        if cmd_type.get() == "linear_rel" { "L_REL" } else if cmd_type.get() == "linear_abs" { "L_ABS" } else { "JOINT" },
                                        x.get(), y.get(), z.get(), w.get(), p.get(), r.get(),
                                        speed.get(), term_type.get()
                                    )}
                                </div>
                                <p class="text-[9px] text-[#555555]">
                                    {move || format!("Using UFrame {} and UTool {}", uframe.get(), utool.get())}
                                </p>
                            </div>
                        }),
                    }}
                </div>

                // Footer
                <div class="flex items-center justify-between p-3 border-t border-[#ffffff08]">
                    <button
                        class={move || format!(
                            "text-[10px] px-3 py-1.5 rounded {}",
                            if step.get() > 1 {
                                "bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white"
                            } else {
                                "invisible"
                            }
                        )}
                        on:click=move |_| set_step.update(|s| *s = (*s).saturating_sub(1).max(1))
                    >
                        "← Back"
                    </button>
                    <div class="flex gap-1">
                        <button
                            class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
                            on:click=move |_| ctx.show_composer.set(false)
                        >
                            "Cancel"
                        </button>
                        {move || if step.get() < 3 {
                            Either::Left(view! {
                                <button
                                    class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[10px] px-3 py-1.5 rounded hover:bg-[#00d9ff30]"
                                    on:click=move |_| set_step.update(|s| *s += 1)
                                >
                                    "Next →"
                                </button>
                            })
                        } else {
                            Either::Right(view! {
                                <button
                                    class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[10px] px-3 py-1.5 rounded hover:bg-[#22c55e30]"
                                    on:click=move |_| {
                                        // Generate a unique ID based on timestamp
                                        let new_id = js_sys::Date::now() as usize;

                                        // Create a RecentCommand and add to recent commands list
                                        let cmd = RecentCommand {
                                            id: new_id,
                                            name: format!("{} ({:.1}, {:.1}, {:.1})",
                                                if cmd_type.get() == "linear_rel" { "L_REL" }
                                                else if cmd_type.get() == "linear_abs" { "L_ABS" }
                                                else { "JOINT" },
                                                x.get(), y.get(), z.get()
                                            ),
                                            command_type: cmd_type.get(),
                                            description: format!("{}mm/s {}", speed.get(), term_type.get()),
                                            x: x.get(),
                                            y: y.get(),
                                            z: z.get(),
                                            w: w.get(),
                                            p: p.get(),
                                            r: r.get(),
                                            speed: speed.get(),
                                            term_type: term_type.get(),
                                            uframe: uframe.get(),
                                            utool: utool.get(),
                                        };
                                        ctx.recent_commands.update(|cmds| {
                                            // Insert at the beginning (most recent first)
                                            cmds.insert(0, cmd);
                                            // Keep only last 15 commands
                                            while cmds.len() > 15 {
                                                cmds.pop();
                                            }
                                        });
                                        // Auto-select the newly created command
                                        ctx.selected_command_id.set(Some(new_id));
                                        ctx.show_composer.set(false);
                                    }
                                >
                                    "✓ Apply"
                                </button>
                            })
                        }}
                    </div>
                </div>
            </div>
        </div>
    }
}

