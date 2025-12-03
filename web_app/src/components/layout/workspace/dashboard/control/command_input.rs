//! Command input section with recent commands and composer button.

use leptos::prelude::*;
use crate::components::layout::workspace::context::{WorkspaceContext, CommandLogEntry, CommandStatus, RecentCommand};
use crate::websocket::WebSocketManager;
use fanuc_rmi::dto::{SendPacket, Instruction, FrcLinearRelative, FrcLinearMotion, FrcJointMotion, Configuration, Position};
use fanuc_rmi::{SpeedType, TermType};

/// Helper function to create a motion packet from a RecentCommand
/// Uses the WebSocketManager to get arm configuration from robot connection defaults
/// Returns None if no robot is connected (can't create valid packet without connection config)
pub fn create_motion_packet(cmd: &RecentCommand, ws: &WebSocketManager) -> Option<SendPacket> {
    // Get arm configuration from robot connection defaults
    // If no robot is connected, we can't create a valid motion packet
    let active_conn = ws.get_active_connection()?;

    let front = active_conn.default_front.unwrap_or(1) as i8;
    let up = active_conn.default_up.unwrap_or(1) as i8;
    let left = active_conn.default_left.unwrap_or(0) as i8;
    let flip = active_conn.default_flip.unwrap_or(0) as i8;
    let turn4 = active_conn.default_turn4.unwrap_or(0) as i8;
    let turn5 = active_conn.default_turn5.unwrap_or(0) as i8;
    let turn6 = active_conn.default_turn6.unwrap_or(0) as i8;

    let config = Configuration {
        u_tool_number: cmd.utool as i8,
        u_frame_number: cmd.uframe as i8,
        front,
        up,
        left,
        flip,
        turn4,
        turn5,
        turn6,
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

    Some(match cmd.command_type.as_str() {
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
    })
}

/// Command input section with recent commands and composer button
#[component]
pub fn CommandInputSection() -> impl IntoView {
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
                            let cmds = recent_commands.get();
                            if let Some(cmd) = cmds.iter().find(|c| c.id == idx) {
                                // Try to create motion packet - requires robot connection
                                if let Some(packet) = create_motion_packet(cmd, &ws) {
                                    ws.send_command(packet);
                                    ctx.command_log.update(|log| {
                                        log.push(CommandLogEntry {
                                            timestamp: js_sys::Date::new_0().to_locale_time_string("en-US").as_string().unwrap_or_else(|| "??:??:??".to_string()),
                                            command: cmd.name.clone(),
                                            status: CommandStatus::Pending,
                                        });
                                    });
                                } else {
                                    // No robot connected - show error
                                    ws.set_message("Cannot run command: No robot connected".to_string());
                                }
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

