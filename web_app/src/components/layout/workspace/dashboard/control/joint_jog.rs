//! Joint Jog Panel - Individual joint jogging controls.
//!
//! Provides up/down buttons for each joint (J1-J6) with configurable step and speed.
//! Uses FrcJointRelativeJRep instruction for relative joint motion.

use leptos::prelude::*;
use fanuc_rmi::dto::*;
use crate::websocket::WebSocketManager;
use crate::components::layout::LayoutContext;

/// Joint Jog Panel - Jog individual joints with up/down buttons.
#[component]
pub fn JointJogPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");

    // Use shared signals from LayoutContext
    let joint_speed = layout_ctx.joint_jog_speed;
    let joint_step = layout_ctx.joint_jog_step;

    // Speed change confirmation modal state
    let (show_speed_confirm, set_show_speed_confirm) = signal(false);
    let (pending_speed, set_pending_speed) = signal::<Option<f64>>(None);
    let (previous_speed, set_previous_speed) = signal(joint_speed.get_untracked());

    // Disable jog controls when a program is actively running (not paused)
    let program_running = ws.program_running;
    let program_paused = ws.program_paused;
    let controls_disabled = move || program_running.get() && !program_paused.get();

    // Get current joint angles from robot
    let joint_angles = ws.joint_angles;

    // Send joint jog command for a specific joint
    let send_joint_jog = move |joint_index: usize, direction: f32| {
        if controls_disabled() {
            ws.set_message("Cannot jog: Program is running".to_string());
            return;
        }
        if ws.get_active_connection().is_none() {
            ws.set_message("Cannot jog: No robot connected".to_string());
            return;
        }

        let step = joint_step.get_untracked() as f32 * direction;
        let speed = joint_speed.get_untracked();

        // Create joint angles with only the target joint's delta set
        let mut angles = JointAngles {
            j1: 0.0, j2: 0.0, j3: 0.0, j4: 0.0, j5: 0.0, j6: 0.0,
            j7: 0.0, j8: 0.0, j9: 0.0,
        };
        match joint_index {
            0 => angles.j1 = step,
            1 => angles.j2 = step,
            2 => angles.j3 = step,
            3 => angles.j4 = step,
            4 => angles.j5 = step,
            5 => angles.j6 = step,
            _ => return,
        }

        let packet = SendPacket::Instruction(Instruction::FrcJointRelativeJRep(
            FrcJointRelativeJRep {
                sequence_id: 0, // Will be assigned by driver
                joint_angles: angles,
                speed_type: fanuc_rmi::SpeedType::Time, // Use time-based for joint motion
                speed,
                term_type: fanuc_rmi::TermType::FINE,
                term_value: 1,
            },
        ));
        ws.send_command(packet);
    };
    let send_joint_jog = StoredValue::new(send_joint_jog);

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <div class="flex items-center justify-between mb-2">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                    </svg>
                    "Joint Jog"
                </h3>
                // Step and Speed inputs
                <div class="flex items-center gap-2">
                    <div class="flex items-center gap-1">
                        <label class="text-[8px] text-[#666666]">"Step:"</label>
                        <input
                            type="number"
                            step="0.1"
                            class="w-12 bg-[#111111] border border-[#ffffff08] rounded px-1 py-0.5 text-white text-[9px] focus:border-[#00d9ff] focus:outline-none text-center"
                            prop:value=move || format!("{:.1}", joint_step.get())
                            on:input=move |ev| {
                                if let Ok(v) = event_target_value(&ev).parse::<f64>() {
                                    joint_step.set(v);
                                }
                            }
                        />
                        <span class="text-[8px] text-[#666666]">"°"</span>
                    </div>
                    <div class="flex items-center gap-1">
                        <label class="text-[8px] text-[#666666]">"Speed:"</label>
                        <input
                            type="number"
                            step="1"
                            class="w-12 bg-[#111111] border border-[#ffffff08] rounded px-1 py-0.5 text-white text-[9px] focus:border-[#00d9ff] focus:outline-none text-center"
                            prop:value=move || format!("{:.0}", joint_speed.get())
                            on:change=move |ev| {
                                if let Ok(new_speed) = event_target_value(&ev).parse::<f64>() {
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
                                        joint_speed.set(old_speed);
                                    } else {
                                        // Apply immediately
                                        joint_speed.set(new_speed);
                                        set_previous_speed.set(new_speed);
                                    }
                                }
                            }
                        />
                        <span class="text-[8px] text-[#666666]">"°/s"</span>
                    </div>
                </div>
            </div>

            // Show warning when controls are disabled
            <Show when=controls_disabled>
                <div class="text-[9px] text-[#ff8800] mb-1 text-center">
                    "⚠ Disabled: Program running"
                </div>
            </Show>

            // Joint buttons grid - 6 columns for J1-J6
            <div class="grid grid-cols-6 gap-1">
                {(0..6).map(|i| {
                    let joint_name = format!("J{}", i + 1);
                    let get_angle = move || {
                        joint_angles.get().map(|a| a[i]).unwrap_or(0.0)
                    };
                    view! {
                        <JointButton
                            joint_index=i
                            joint_name=joint_name
                            angle=get_angle
                            disabled=controls_disabled
                            on_jog=move |dir| send_joint_jog.with_value(|f| f(i, dir))
                        />
                    }
                }).collect_view()}
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
                                            <span class="text-[#888888] ml-1">"°/s"</span>
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
                                        joint_speed.set(new_speed);
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

/// Individual joint button with up/down controls and angle display.
#[component]
fn JointButton(
    #[allow(unused)] joint_index: usize,
    joint_name: String,
    #[prop(into)] angle: Signal<f32>,
    #[prop(into)] disabled: Signal<bool>,
    on_jog: impl Fn(f32) + 'static + Clone,
) -> impl IntoView {
    let on_jog_up = on_jog.clone();
    let on_jog_down = on_jog;

    let button_class = move || {
        if disabled.get() {
            "w-full bg-[#0a0a0a] border border-[#ffffff08] text-[#444444] py-1 rounded cursor-not-allowed text-[10px]"
        } else {
            "w-full bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black py-1 rounded transition-colors text-[10px]"
        }
    };

    view! {
        <div class="flex flex-col items-center">
            // Up button
            <button
                class=button_class
                disabled=disabled
                on:click=move |_| on_jog_up(1.0)
                title=format!("{} +", joint_name)
            >
                "▲"
            </button>
            // Joint name and angle
            <div class="py-1 text-center w-full bg-[#111111] border-x border-[#ffffff08]">
                <div class="text-[9px] text-[#00d9ff] font-semibold">{joint_name.clone()}</div>
                <div class="text-[10px] text-white font-mono">
                    {move || format!("{:.2}°", angle.get())}
                </div>
            </div>
            // Down button
            <button
                class=button_class
                disabled=disabled
                on:click=move |_| on_jog_down(-1.0)
                title=format!("{} -", joint_name)
            >
                "▼"
            </button>
        </div>
    }
}

