use leptos::prelude::*;
use fanuc_rmi::dto::*;
use crate::websocket::WebSocketManager;

#[component]
pub fn JogControls() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let active_jog_settings = ws.active_jog_settings;

    // Local string state for inputs - start empty, will be populated from server
    let (speed_str, set_speed_str) = signal(String::new());
    let (step_str, set_step_str) = signal(String::new());
    let (rot_speed_str, set_rot_speed_str) = signal(String::new());
    let (rot_step_str, set_rot_step_str) = signal(String::new());

    // Always sync from server state - these should reflect server truth
    Effect::new(move || {
        if let Some(settings) = active_jog_settings.get() {
            set_speed_str.set(format!("{:.1}", settings.cartesian_jog_speed));
            set_step_str.set(format!("{:.1}", settings.cartesian_jog_step));
            set_rot_speed_str.set(format!("{:.1}", settings.rotation_jog_speed));
            set_rot_step_str.set(format!("{:.1}", settings.rotation_jog_step));
        }
    });



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
        let Some(_active_conn) = ws.get_active_connection() else {
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

        // Get jog speed from server state
        let jog_speed = active_jog_settings.get_untracked()
            .map(|s| s.cartesian_jog_speed)
            .unwrap_or(10.0);

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
                speed: jog_speed as f64,
                term_type: fanuc_rmi::TermType::FINE,
                term_value: 1,
            },
        ));
        ws.send_command(packet);
    };
    let send_jog = StoredValue::new(send_jog);

    // Rotation jog function - rotates about W/P/R axes
    let send_rotation_jog = move |dw: f64, dp: f64, dr: f64| {
        if controls_disabled() {
            ws.set_message("Cannot jog: Program is running".to_string());
            return;
        }
        let Some(_active_conn) = ws.get_active_connection() else {
            ws.set_message("Cannot jog: No robot connected".to_string());
            return;
        };

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
            log::warn!("No active configuration found for rotation jog - using fallback defaults");
            (0, 1, 1, 1, 0, 0, 0, 0, 0)
        };

        // Use rotation jog speed (degrees/sec)
        let jog_speed = active_jog_settings.get_untracked()
            .map(|s| s.rotation_jog_speed)
            .unwrap_or(5.0);

        let packet = SendPacket::Instruction(Instruction::FrcLinearRelative(
            FrcLinearRelative {
                sequence_id: 0,
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
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    w: dw,
                    p: dp,
                    r: dr,
                    ext1: 0.0,
                    ext2: 0.0,
                    ext3: 0.0,
                },
                speed_type: fanuc_rmi::SpeedType::MMSec,
                speed: jog_speed as f64,
                term_type: fanuc_rmi::TermType::FINE,
                term_value: 1,
            },
        ));
        ws.send_command(packet);
    };
    let send_rotation_jog = StoredValue::new(send_rotation_jog);

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
                    <NumberInput
                        value=speed_str
                        on_change=move |val: String| {
                            set_speed_str.set(val.clone());
                            if let Ok(new_speed) = val.parse::<f64>() {
                                // Get current settings from server
                                if let Some(settings) = active_jog_settings.get_untracked() {
                                    // Update jog controls only (does NOT update defaults or increment changes_count)
                                    ws.update_jog_controls(
                                        new_speed,
                                        settings.cartesian_jog_step,
                                        settings.joint_jog_speed,
                                        settings.joint_jog_step,
                                        settings.rotation_jog_speed,
                                        settings.rotation_jog_step,
                                    );
                                }
                            }
                        }
                        min=0.1
                        max=1000.0
                    />
                </div>
                <div>
                    <label class="block text-[#666666] text-[9px] mb-0.5">"Step mm"</label>
                    <NumberInput
                        value=step_str
                        on_change=move |val: String| {
                            set_step_str.set(val.clone());
                            if let Ok(new_step) = val.parse::<f64>() {
                                // Get current settings from server
                                if let Some(settings) = active_jog_settings.get_untracked() {
                                    // Update jog controls only (does NOT update defaults or increment changes_count)
                                    ws.update_jog_controls(
                                        settings.cartesian_jog_speed,
                                        new_step,
                                        settings.joint_jog_speed,
                                        settings.joint_jog_step,
                                        settings.rotation_jog_speed,
                                        settings.rotation_jog_step,
                                    );
                                }
                            }
                        }
                        min=0.1
                        max=100.0
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
                        on:click=move |_| {
                            let step = active_jog_settings.get_untracked().map(|s| s.cartesian_jog_step).unwrap_or(1.0);
                            send_jog.with_value(|f| f(0.0, step, 0.0));
                        }
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
                        on:click=move |_| {
                            let step = active_jog_settings.get_untracked().map(|s| s.cartesian_jog_step).unwrap_or(1.0);
                            send_jog.with_value(|f| f(-step, 0.0, 0.0));
                        }
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
                        on:click=move |_| {
                            let step = active_jog_settings.get_untracked().map(|s| s.cartesian_jog_step).unwrap_or(1.0);
                            send_jog.with_value(|f| f(0.0, 0.0, step));
                        }
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
                        on:click=move |_| {
                            let step = active_jog_settings.get_untracked().map(|s| s.cartesian_jog_step).unwrap_or(1.0);
                            send_jog.with_value(|f| f(step, 0.0, 0.0));
                        }
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
                        on:click=move |_| {
                            let step = active_jog_settings.get_untracked().map(|s| s.cartesian_jog_step).unwrap_or(1.0);
                            send_jog.with_value(|f| f(0.0, -step, 0.0));
                        }
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
                        on:click=move |_| {
                            let step = active_jog_settings.get_untracked().map(|s| s.cartesian_jog_step).unwrap_or(1.0);
                            send_jog.with_value(|f| f(0.0, 0.0, -step));
                        }
                    >
                        <div class="text-sm leading-none">"▼"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"Z-"</div>
                    </button>
                </div>
            </div>

            // W/P/R Rotation Controls
            <div class="mt-2 pt-2 border-t border-[#ffffff08]">
                <div class="text-[8px] text-[#666666] mb-1 text-center">"Rotation (W/P/R)"</div>

                // Rotation speed/step inputs
                <div class="grid grid-cols-2 gap-1 mb-2">
                    <div>
                        <label class="block text-[#666666] text-[9px] mb-0.5">"Speed °/s"</label>
                        <NumberInput
                            value=rot_speed_str
                            on_change=move |val: String| {
                                set_rot_speed_str.set(val.clone());
                                if let Ok(new_speed) = val.parse::<f64>() {
                                    if let Some(settings) = active_jog_settings.get_untracked() {
                                        ws.update_jog_controls(
                                            settings.cartesian_jog_speed,
                                            settings.cartesian_jog_step,
                                            settings.joint_jog_speed,
                                            settings.joint_jog_step,
                                            new_speed,
                                            settings.rotation_jog_step,
                                        );
                                    }
                                }
                            }
                            min=0.1
                            max=180.0
                        />
                    </div>
                    <div>
                        <label class="block text-[#666666] text-[9px] mb-0.5">"Step °"</label>
                        <NumberInput
                            value=rot_step_str
                            on_change=move |val: String| {
                                set_rot_step_str.set(val.clone());
                                if let Ok(new_step) = val.parse::<f64>() {
                                    if let Some(settings) = active_jog_settings.get_untracked() {
                                        ws.update_jog_controls(
                                            settings.cartesian_jog_speed,
                                            settings.cartesian_jog_step,
                                            settings.joint_jog_speed,
                                            settings.joint_jog_step,
                                            settings.rotation_jog_speed,
                                            new_step,
                                        );
                                    }
                                }
                            }
                            min=0.1
                            max=90.0
                        />
                    </div>
                </div>

                <div class="grid grid-cols-3 gap-1">
                    // W = Yaw (rotation about Z axis)
                    <RotationButton
                        label="W"
                        axis="Z"
                        disabled=controls_disabled
                        on_jog_up=move || {
                            let step = active_jog_settings.get_untracked().map(|s| s.rotation_jog_step).unwrap_or(1.0);
                            send_rotation_jog.with_value(|f| f(step, 0.0, 0.0));
                        }
                        on_jog_down=move || {
                            let step = active_jog_settings.get_untracked().map(|s| s.rotation_jog_step).unwrap_or(1.0);
                            send_rotation_jog.with_value(|f| f(-step, 0.0, 0.0));
                        }
                    />
                    // P = Pitch (rotation about Y axis)
                    <RotationButton
                        label="P"
                        axis="Y"
                        disabled=controls_disabled
                        on_jog_up=move || {
                            let step = active_jog_settings.get_untracked().map(|s| s.rotation_jog_step).unwrap_or(1.0);
                            send_rotation_jog.with_value(|f| f(0.0, step, 0.0));
                        }
                        on_jog_down=move || {
                            let step = active_jog_settings.get_untracked().map(|s| s.rotation_jog_step).unwrap_or(1.0);
                            send_rotation_jog.with_value(|f| f(0.0, -step, 0.0));
                        }
                    />
                    // R = Roll (rotation about X axis)
                    <RotationButton
                        label="R"
                        axis="X"
                        disabled=controls_disabled
                        on_jog_up=move || {
                            let step = active_jog_settings.get_untracked().map(|s| s.rotation_jog_step).unwrap_or(1.0);
                            send_rotation_jog.with_value(|f| f(0.0, 0.0, step));
                        }
                        on_jog_down=move || {
                            let step = active_jog_settings.get_untracked().map(|s| s.rotation_jog_step).unwrap_or(1.0);
                            send_rotation_jog.with_value(|f| f(0.0, 0.0, -step));
                        }
                    />
                </div>
            </div>
        </div>
    }
}

/// Rotation button component with up/down controls for W/P/R axes
#[component]
fn RotationButton(
    /// Rotation label (W, P, or R)
    label: &'static str,
    /// Axis this rotation is about (Z, Y, or X)
    axis: &'static str,
    #[prop(into)] disabled: Signal<bool>,
    on_jog_up: impl Fn() + 'static + Clone,
    on_jog_down: impl Fn() + 'static + Clone,
) -> impl IntoView {
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
                on:click=move |_| on_jog_up()
                title=format!("{} + (rotate about {})", label, axis)
            >
                "▲"
            </button>
            // Axis label with rotation axis info
            <div class="py-0.5 text-center w-full bg-[#111111] border-x border-[#ffffff08]">
                <div class="text-[9px] text-[#00d9ff] font-semibold">{label}</div>
                <div class="text-[7px] text-[#666666]">{format!("⟳{}", axis)}</div>
            </div>
            // Down button
            <button
                class=button_class
                disabled=disabled
                on:click=move |_| on_jog_down()
                title=format!("{} - (rotate about {})", label, axis)
            >
                "▼"
            </button>
        </div>
    }
}

/// Number input component with validation
#[component]
fn NumberInput(
    #[prop(into)] value: Signal<String>,
    on_change: impl Fn(String) + 'static,
    #[prop(optional)] placeholder: &'static str,
    #[prop(default = 0.0)] min: f64,
    #[prop(default = f64::MAX)] max: f64,
) -> impl IntoView {
    let is_valid = move || {
        if let Ok(v) = value.get().parse::<f64>() {
            if v < min || v > max {
                return false;
            }
            true
        } else {
            value.get().is_empty()
        }
    };

    view! {
        <input
            type="text"
            class=move || format!(
                "w-full bg-[#111111] rounded px-1.5 py-1 text-white text-[11px] focus:outline-none {}",
                if is_valid() {
                    "border border-[#ffffff08] focus:border-[#00d9ff]"
                } else {
                    "border-2 border-[#ff4444]"
                }
            )
            placeholder=placeholder
            prop:value=value
            on:change=move |ev| {
                on_change(event_target_value(&ev));
            }
        />
    }
}
