use leptos::prelude::*;
use fanuc_rmi::dto::*;
use crate::websocket::WebSocketManager;
use crate::components::layout::LayoutContext;

#[component]
pub fn JogControls() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");

    // Use shared signals from LayoutContext so settings persist between docked/floating
    let jog_speed = layout_ctx.jog_speed;
    let step_distance = layout_ctx.jog_step;

    // Speed change confirmation modal state
    let (show_speed_confirm, set_show_speed_confirm) = signal(false);
    let (pending_speed, set_pending_speed) = signal::<Option<f64>>(None);
    let (previous_speed, set_previous_speed) = signal(jog_speed.get_untracked());

    // Disable jog controls when a program is actively running (not paused)
    let program_running = ws.program_running;
    let program_paused = ws.program_paused;
    let controls_disabled = move || program_running.get() && !program_paused.get();

    let send_jog = move |dx: f64, dy: f64, dz: f64| {
        // Don't allow jogging while program is running
        if controls_disabled() {
            ws.set_message("Cannot jog: Program is running".to_string());
            return;
        }
        // Get arm configuration from robot connection defaults
        // If no robot is connected, show error and don't send jog command
        let Some(active_conn) = ws.get_active_connection() else {
            ws.set_message("Cannot jog: No robot connected".to_string());
            return;
        };

        // Get configuration from active_configuration (loaded from robot_configurations table)
        // This is the authoritative value that reflects what the robot is actually using
        let active_config = ws.active_configuration.get_untracked();

        let (u_frame, u_tool, front, up, left, flip, turn4, turn5, turn6) = if let Some(config) = active_config {
            (
                config.u_frame_number as i8,
                config.u_tool_number as i8,
                config.front as i8,
                config.up as i8,
                config.left as i8,
                config.flip as i8,
                config.turn4 as i8,
                config.turn5 as i8,
                config.turn6 as i8,
            )
        } else {
            // Fallback if no active configuration (shouldn't happen if robot is connected)
            log::warn!("No active configuration found for jog - using fallback defaults");
            (0, 1, 1, 1, 0, 0, 0, 0, 0)
        };

        let packet = SendPacket::Instruction(Instruction::FrcLinearRelative(
            FrcLinearRelative {
                sequence_id: 0, // Will be assigned by driver
                configuration: Configuration {
                    u_tool_number: u_tool as i8,
                    u_frame_number: u_frame as i8,
                    front,
                    up,
                    left,
                    flip,
                    turn4,
                    turn5,
                    turn6,
                },
                position: Position {
                    x: dx,
                    y: dy,
                    z: dz,
                    w: 0.0,
                    p: 0.0,
                    r: 0.0,
                    ext1: 0.0,
                    ext2: 0.0,
                    ext3: 0.0,
                },
                speed_type: fanuc_rmi::SpeedType::MMSec,
                speed: jog_speed.get_untracked() as f64,
                term_type: fanuc_rmi::TermType::FINE,
                term_value: 1,
            },
        ));
        ws.send_command(packet);
    };
    let send_jog = StoredValue::new(send_jog);

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 flex items-center uppercase tracking-wide">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/>
                </svg>
                "Jog"
            </h2>

            <div class="grid grid-cols-2 gap-1 mb-2">
                <div>
                    <label class="block text-[#666666] text-[9px] mb-0.5">"Speed mm/s"</label>
                    <input
                        type="number"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-1.5 py-1 text-white text-[11px] focus:border-[#00d9ff] focus:outline-none"
                        prop:value=move || jog_speed.get()
                        on:change=move |ev| {
                            let val = event_target_value(&ev);
                            if let Ok(new_speed) = val.parse::<f64>() {
                                let old_speed = previous_speed.get();
                                // Check if change is >50%
                                let percent_change = if old_speed > 0.0 {
                                    ((new_speed - old_speed).abs() / old_speed) * 100.0
                                } else {
                                    0.0
                                };

                                if percent_change > 50.0 {
                                    // Show confirmation modal
                                    set_pending_speed.set(Some(new_speed));
                                    set_show_speed_confirm.set(true);
                                    // Reset input to old value until confirmed
                                    jog_speed.set(old_speed);
                                } else {
                                    // Apply immediately
                                    jog_speed.set(new_speed);
                                    set_previous_speed.set(new_speed);
                                }
                            }
                        }
                    />
                </div>
                <div>
                    <label class="block text-[#666666] text-[9px] mb-0.5">"Step mm"</label>
                    <input
                        type="number"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-1.5 py-1 text-white text-[11px] focus:border-[#00d9ff] focus:outline-none"
                        prop:value=move || step_distance.get()
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            if let Ok(v) = val.parse::<f64>() {
                                step_distance.set(v);
                            }
                        }
                    />
                </div>
            </div>

            // Show warning when controls are disabled
            <Show when=controls_disabled>
                <div class="text-[9px] text-[#ff8800] mb-1 text-center">
                    "⚠ Disabled: Program running"
                </div>
            </Show>

            <div class="grid grid-cols-3 gap-1">
                <div class="col-span-3 grid grid-cols-3 gap-1">
                    <div></div>
                    <button
                        class=move || if controls_disabled() {
                            "bg-[#0a0a0a] border border-[#ffffff08] text-[#444444] font-semibold py-1.5 rounded cursor-not-allowed text-center"
                        } else {
                            "bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        }
                        disabled=controls_disabled
                        on:click=move |_| send_jog.with_value(|f| f(0.0, step_distance.get_untracked(), 0.0))
                    >
                        <div class="text-sm leading-none">"↑"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"Y+"</div>
                    </button>
                    <div></div>
                </div>

                <div class="col-span-3 grid grid-cols-3 gap-1">
                    <button
                        class=move || if controls_disabled() {
                            "bg-[#0a0a0a] border border-[#ffffff08] text-[#444444] font-semibold py-1.5 rounded cursor-not-allowed text-center"
                        } else {
                            "bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        }
                        disabled=controls_disabled
                        on:click=move |_| send_jog.with_value(|f| f(-step_distance.get_untracked(), 0.0, 0.0))
                    >
                        <div class="text-sm leading-none">"←"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"X-"</div>
                    </button>
                    <button
                        class=move || if controls_disabled() {
                            "bg-[#0a0a0a] border border-[#ffffff08] text-[#444444] font-semibold py-1.5 rounded cursor-not-allowed text-center"
                        } else {
                            "bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        }
                        disabled=controls_disabled
                        on:click=move |_| send_jog.with_value(|f| f(0.0, 0.0, step_distance.get_untracked()))
                    >
                        <div class="text-sm leading-none">"▲"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"Z+"</div>
                    </button>
                    <button
                        class=move || if controls_disabled() {
                            "bg-[#0a0a0a] border border-[#ffffff08] text-[#444444] font-semibold py-1.5 rounded cursor-not-allowed text-center"
                        } else {
                            "bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        }
                        disabled=controls_disabled
                        on:click=move |_| send_jog.with_value(|f| f(step_distance.get_untracked(), 0.0, 0.0))
                    >
                        <div class="text-sm leading-none">"→"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"X+"</div>
                    </button>
                </div>

                <div class="col-span-3 grid grid-cols-3 gap-1">
                    <div></div>
                    <button
                        class=move || if controls_disabled() {
                            "bg-[#0a0a0a] border border-[#ffffff08] text-[#444444] font-semibold py-1.5 rounded cursor-not-allowed text-center"
                        } else {
                            "bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        }
                        disabled=controls_disabled
                        on:click=move |_| send_jog.with_value(|f| f(0.0, -step_distance.get_untracked(), 0.0))
                    >
                        <div class="text-sm leading-none">"↓"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"Y-"</div>
                    </button>
                    <button
                        class=move || if controls_disabled() {
                            "bg-[#0a0a0a] border border-[#ffffff08] text-[#444444] font-semibold py-1.5 rounded cursor-not-allowed text-center"
                        } else {
                            "bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        }
                        disabled=controls_disabled
                        on:click=move |_| send_jog.with_value(|f| f(0.0, 0.0, -step_distance.get_untracked()))
                    >
                        <div class="text-sm leading-none">"▼"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"Z-"</div>
                    </button>
                </div>
            </div>

            // Speed change confirmation modal
            <Show when=move || show_speed_confirm.get()>
                <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
                    <div class="bg-[#111111] border border-[#ffaa0040] rounded-lg w-[320px] shadow-xl">
                        // Header
                        <div class="flex items-center p-3 border-b border-[#ffffff08]">
                            <svg class="w-5 h-5 mr-2 text-[#ffaa00]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                            </svg>
                            <h2 class="text-sm font-semibold text-white">"Large Speed Change"</h2>
                        </div>

                        // Content
                        <div class="p-4 space-y-3">
                            {move || {
                                let old = previous_speed.get();
                                let new = pending_speed.get().unwrap_or(old);
                                let percent = if old > 0.0 { ((new - old).abs() / old) * 100.0 } else { 0.0 };
                                view! {
                                    <div class="bg-[#0a0a0a] rounded p-3 space-y-2">
                                        <div class="flex items-center text-[11px]">
                                            <span class="text-[#888888] w-20">"Speed:"</span>
                                            <span class="text-[#ff6666] font-mono">{format!("{:.1}", old)}</span>
                                            <span class="text-[#666666] mx-2">"→"</span>
                                            <span class="text-[#66ff66] font-mono">{format!("{:.1}", new)}</span>
                                            <span class="text-[#888888] ml-1">"mm/s"</span>
                                        </div>
                                        <div class="flex items-center text-[11px]">
                                            <span class="text-[#888888] w-20">"Change:"</span>
                                            <span class="text-[#ffaa00] font-mono">{format!("{:.0}%", percent)}</span>
                                        </div>
                                    </div>
                                    <p class="text-[10px] text-[#888888]">
                                        "This is a significant speed change. Please confirm."
                                    </p>
                                }
                            }}
                        </div>

                        // Footer
                        <div class="flex gap-2 p-3 border-t border-[#ffffff08]">
                            <button
                                class="flex-1 text-[10px] px-4 py-2 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                                on:click=move |_| {
                                    set_pending_speed.set(None);
                                    set_show_speed_confirm.set(false);
                                }
                            >
                                "Cancel"
                            </button>
                            <button
                                class="flex-1 text-[10px] px-4 py-2 bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded hover:bg-[#00d9ff30] font-medium"
                                on:click=move |_| {
                                    if let Some(new_speed) = pending_speed.get() {
                                        jog_speed.set(new_speed);
                                        set_previous_speed.set(new_speed);
                                    }
                                    set_pending_speed.set(None);
                                    set_show_speed_confirm.set(false);
                                }
                            >
                                "Confirm"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

