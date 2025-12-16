//! Joint Jog Panel - Individual joint jogging controls.
//!
//! Provides up/down buttons for each joint (J1-J6) with configurable step and speed.
//! Uses FrcJointRelativeJRep instruction for relative joint motion.

use leptos::prelude::*;
use fanuc_rmi::dto::*;
use crate::websocket::WebSocketManager;

/// Joint Jog Panel - Jog individual joints with up/down buttons.
#[component]
pub fn JointJogPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let active_jog_settings = ws.active_jog_settings;

    // Local string state for inputs (initialized from server state)
    let (speed_str, set_speed_str) = signal(String::new());
    let (step_str, set_step_str) = signal(String::new());

    // Initialize from server state
    Effect::new(move || {
        if let Some(settings) = active_jog_settings.get() {
            set_speed_str.set(format!("{:.1}", settings.joint_jog_speed));
            set_step_str.set(format!("{:.1}", settings.joint_jog_step));
        }
    });

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

        // Get jog settings from server state
        let (step, speed) = if let Some(settings) = active_jog_settings.get_untracked() {
            (settings.joint_jog_step as f32 * direction, settings.joint_jog_speed)
        } else {
            (1.0 * direction, 10.0)
        };

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
                            prop:value=move || step_str.get()
                            on:input=move |ev| {
                                let val = event_target_value(&ev);
                                set_step_str.set(val.clone());
                            }
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                if let Ok(new_step) = val.parse::<f64>() {
                                    // Get current settings from server
                                    if let Some(settings) = active_jog_settings.get_untracked() {
                                        // Update jog controls only (does NOT update defaults or increment changes_count)
                                        ws.update_jog_controls(
                                            settings.cartesian_jog_speed,
                                            settings.cartesian_jog_step,
                                            settings.joint_jog_speed,
                                            new_step,
                                            settings.rotation_jog_speed,
                                            settings.rotation_jog_step,
                                        );
                                    }
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
                            prop:value=move || speed_str.get()
                            on:input=move |ev| {
                                let val = event_target_value(&ev);
                                set_speed_str.set(val.clone());
                            }
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                if let Ok(new_speed) = val.parse::<f64>() {
                                    // Get current settings from server
                                    if let Some(settings) = active_jog_settings.get_untracked() {
                                        // Update jog controls only (does NOT update defaults or increment changes_count)
                                        ws.update_jog_controls(
                                            settings.cartesian_jog_speed,
                                            settings.cartesian_jog_step,
                                            new_speed,
                                            settings.joint_jog_step,
                                            settings.rotation_jog_speed,
                                            settings.rotation_jog_step,
                                        );
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

