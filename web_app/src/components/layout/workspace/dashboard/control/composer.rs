//! Command Composer Modal - single-page composer for creating motion commands.
//!
//! Redesigned from wizard to single-page layout per specification.
//! Supports LinearAbsolute, LinearRelative, JointAbsolute, JointRelative instruction types.

use leptos::prelude::*;
use crate::components::layout::workspace::context::{WorkspaceContext, RecentCommand};
use crate::websocket::WebSocketManager;

/// Instruction types available in the composer
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum InstructionType {
    LinearAbsolute,
    LinearRelative,
    JointAbsolute,
    JointRelative,
}

impl InstructionType {
    fn label(&self) -> &'static str {
        match self {
            Self::LinearAbsolute => "Linear Absolute",
            Self::LinearRelative => "Linear Relative",
            Self::JointAbsolute => "Joint Absolute",
            Self::JointRelative => "Joint Relative",
        }
    }

    fn is_cartesian(&self) -> bool {
        matches!(self, Self::LinearAbsolute | Self::LinearRelative)
    }

    fn is_absolute(&self) -> bool {
        matches!(self, Self::LinearAbsolute | Self::JointAbsolute)
    }
}

/// Command Composer Modal - single-page layout for creating motion commands
#[component]
pub fn CommandComposerModal() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");

    // Instruction type selection
    let (instr_type, set_instr_type) = signal(InstructionType::LinearRelative);

    // Get current robot position/angles for defaults
    let current_pos = ws.position;
    let current_orient = ws.orientation;
    let current_joints = ws.joint_angles;
    let active_config = ws.active_configuration;

    // Position inputs (Cartesian: X,Y,Z,W,P,R or Joint: J1-J6)
    let (x, set_x) = signal(0.0f64);
    let (y, set_y) = signal(0.0f64);
    let (z, set_z) = signal(0.0f64);
    let (w, set_w) = signal(0.0f64);
    let (p, set_p) = signal(0.0f64);
    let (r, set_r) = signal(0.0f64);

    // Joint angles (for joint moves)
    let (j1, set_j1) = signal(0.0f64);
    let (j2, set_j2) = signal(0.0f64);
    let (j3, set_j3) = signal(0.0f64);
    let (j4, set_j4) = signal(0.0f64);
    let (j5, set_j5) = signal(0.0f64);
    let (j6, set_j6) = signal(0.0f64);

    // Motion parameters - get defaults from robot connection
    // Default to robot's cartesian jog speed, or 100 if no robot connected
    let default_cartesian_speed = ws.get_active_connection()
        .map(|c| c.default_cartesian_jog_speed)
        .unwrap_or(100.0);
    let (speed, set_speed) = signal(default_cartesian_speed);
    let (term_type, set_term_type) = signal("FINE".to_string());

    // Update position and speed defaults when instruction type changes
    Effect::new(move || {
        let itype = instr_type.get();

        // Update speed default based on instruction type
        // Cartesian moves use cartesian jog speed, joint moves use joint jog speed
        if let Some(conn) = ws.get_active_connection() {
            if itype.is_cartesian() {
                set_speed.set(conn.default_cartesian_jog_speed);
            } else {
                set_speed.set(conn.default_joint_jog_speed);
            }
        }

        if itype.is_absolute() {
            // Absolute moves: default to current position
            if itype.is_cartesian() {
                if let Some((px, py, pz)) = current_pos.get() {
                    set_x.set(px);
                    set_y.set(py);
                    set_z.set(pz);
                }
                if let Some((pw, pp, pr)) = current_orient.get() {
                    set_w.set(pw);
                    set_p.set(pp);
                    set_r.set(pr);
                }
            } else {
                // Joint absolute
                if let Some(angles) = current_joints.get() {
                    set_j1.set(angles[0] as f64);
                    set_j2.set(angles[1] as f64);
                    set_j3.set(angles[2] as f64);
                    set_j4.set(angles[3] as f64);
                    set_j5.set(angles[4] as f64);
                    set_j6.set(angles[5] as f64);
                }
            }
        } else {
            // Relative moves: default to zeros
            set_x.set(0.0);
            set_y.set(0.0);
            set_z.set(0.0);
            set_w.set(0.0);
            set_p.set(0.0);
            set_r.set(0.0);
            set_j1.set(0.0);
            set_j2.set(0.0);
            set_j3.set(0.0);
            set_j4.set(0.0);
            set_j5.set(0.0);
            set_j6.set(0.0);
        }
    });

    // Apply command - adds to recent commands and selects it (doesn't send to robot)
    let apply_command = move || {
        // Add to recent commands
        let new_id = js_sys::Date::now() as usize;
        let cmd = RecentCommand {
            id: new_id,
            name: format!("{} ({:.1}, {:.1}, {:.1})",
                instr_type.get_untracked().label(),
                if instr_type.get_untracked().is_cartesian() { x.get_untracked() } else { j1.get_untracked() },
                if instr_type.get_untracked().is_cartesian() { y.get_untracked() } else { j2.get_untracked() },
                if instr_type.get_untracked().is_cartesian() { z.get_untracked() } else { j3.get_untracked() }
            ),
            command_type: instr_type.get_untracked().label().to_string(),
            description: format!("{} {}", speed.get_untracked(), term_type.get_untracked()),
            x: x.get_untracked(),
            y: y.get_untracked(),
            z: z.get_untracked(),
            w: w.get_untracked(),
            p: p.get_untracked(),
            r: r.get_untracked(),
            speed: speed.get_untracked(),
            term_type: term_type.get_untracked(),
            uframe: active_config.get_untracked().map(|c| c.u_frame_number as u8).unwrap_or(0),
            utool: active_config.get_untracked().map(|c| c.u_tool_number as u8).unwrap_or(1),
        };
        ctx.recent_commands.update(|cmds| {
            cmds.insert(0, cmd);
            while cmds.len() > 15 {
                cmds.pop();
            }
        });
        ctx.selected_command_id.set(Some(new_id));
        ctx.show_composer.set(false);
    };
    let apply_command = StoredValue::new(apply_command);

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[480px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"/>
                        </svg>
                        "Compose Instruction"
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

                // Content
                <div class="p-3 space-y-3">
                    // Instruction Type dropdown
                    <div>
                        <label class="block text-[10px] text-[#888888] mb-1">"Instruction Type"</label>
                        <select
                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[11px] text-white focus:border-[#00d9ff] focus:outline-none"
                            on:change=move |ev| {
                                let val = event_target_value(&ev);
                                set_instr_type.set(match val.as_str() {
                                    "LinearAbsolute" => InstructionType::LinearAbsolute,
                                    "LinearRelative" => InstructionType::LinearRelative,
                                    "JointAbsolute" => InstructionType::JointAbsolute,
                                    "JointRelative" => InstructionType::JointRelative,
                                    _ => InstructionType::LinearRelative,
                                });
                            }
                        >
                            <option value="LinearRelative" selected=move || instr_type.get() == InstructionType::LinearRelative>"Linear Relative"</option>
                            <option value="LinearAbsolute" selected=move || instr_type.get() == InstructionType::LinearAbsolute>"Linear Absolute"</option>
                            <option value="JointAbsolute" selected=move || instr_type.get() == InstructionType::JointAbsolute>"Joint Absolute"</option>
                            <option value="JointRelative" selected=move || instr_type.get() == InstructionType::JointRelative>"Joint Relative"</option>
                        </select>
                    </div>

                    // Position section
                    <div class="bg-[#0a0a0a] border border-[#ffffff08] rounded p-2">
                        <label class="block text-[10px] text-[#888888] mb-1.5">"Position"</label>
                        {move || if instr_type.get().is_cartesian() {
                            // Cartesian position (X,Y,Z,W,P,R)
                            view! {
                                <div class="grid grid-cols-6 gap-1">
                                    <NumberInput label="X" value=x set_value=set_x step=0.1 unit="mm"/>
                                    <NumberInput label="Y" value=y set_value=set_y step=0.1 unit="mm"/>
                                    <NumberInput label="Z" value=z set_value=set_z step=0.1 unit="mm"/>
                                    <NumberInput label="W" value=w set_value=set_w step=0.1 unit="°"/>
                                    <NumberInput label="P" value=p set_value=set_p step=0.1 unit="°"/>
                                    <NumberInput label="R" value=r set_value=set_r step=0.1 unit="°"/>
                                </div>
                            }.into_any()
                        } else {
                            // Joint angles (J1-J6)
                            view! {
                                <div class="grid grid-cols-6 gap-1">
                                    <NumberInput label="J1" value=j1 set_value=set_j1 step=0.1 unit="°"/>
                                    <NumberInput label="J2" value=j2 set_value=set_j2 step=0.1 unit="°"/>
                                    <NumberInput label="J3" value=j3 set_value=set_j3 step=0.1 unit="°"/>
                                    <NumberInput label="J4" value=j4 set_value=set_j4 step=0.1 unit="°"/>
                                    <NumberInput label="J5" value=j5 set_value=set_j5 step=0.1 unit="°"/>
                                    <NumberInput label="J6" value=j6 set_value=set_j6 step=0.1 unit="°"/>
                                </div>
                            }.into_any()
                        }}
                    </div>

                    // Motion Parameters
                    <div class="bg-[#0a0a0a] border border-[#ffffff08] rounded p-2">
                        <label class="block text-[10px] text-[#888888] mb-1.5">"Motion Parameters"</label>
                        <div class="grid grid-cols-2 gap-2">
                            <div>
                                <label class="block text-[8px] text-[#555555] mb-0.5">"Speed"</label>
                                <div class="flex items-center gap-1">
                                    <SpeedInput
                                        value=speed
                                        set_value=set_speed
                                        is_cartesian=move || instr_type.get().is_cartesian()
                                    />
                                </div>
                            </div>
                            <div>
                                <label class="block text-[8px] text-[#555555] mb-0.5">"Termination"</label>
                                <select
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                    on:change=move |ev| set_term_type.set(event_target_value(&ev))
                                >
                                    <option value="FINE" selected=move || term_type.get() == "FINE">"FINE"</option>
                                    <option value="CNT" selected=move || term_type.get() == "CNT">"CNT"</option>
                                </select>
                            </div>
                        </div>
                    </div>

                    // Configuration (read-only, only for Cartesian moves)
                    <Show when=move || instr_type.get().is_cartesian()>
                        <div class="bg-[#0a0a0a] border border-[#ffffff08] rounded p-2">
                            <label class="block text-[10px] text-[#888888] mb-1.5">"Configuration (from active)"</label>
                            {move || {
                                let config = active_config.get();
                                view! {
                                    <div class="flex items-center gap-3 text-[10px]">
                                        <span class="text-[#666666]">"UFrame:"</span>
                                        <span class="text-white">{config.as_ref().map(|c| c.u_frame_number).unwrap_or(0)}</span>
                                        <span class="text-[#666666]">"UTool:"</span>
                                        <span class="text-white">{config.as_ref().map(|c| c.u_tool_number).unwrap_or(1)}</span>
                                        <span class="text-[#333333]">"|"</span>
                                        <span class="text-[#888888]">
                                            {config.as_ref().map(|c| {
                                                format!("{} {} {} {}",
                                                    if c.front != 0 { "Front" } else { "Back" },
                                                    if c.up != 0 { "Up" } else { "Down" },
                                                    if c.left != 0 { "Left" } else { "Right" },
                                                    if c.flip != 0 { "Flip" } else { "NoFlip" }
                                                )
                                            }).unwrap_or_else(|| "---".to_string())}
                                        </span>
                                    </div>
                                }
                            }}
                        </div>
                    </Show>
                </div>

                // Footer
                <div class="flex items-center justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
                        on:click=move |_| ctx.show_composer.set(false)
                    >
                        "Cancel"
                    </button>
                    <button
                        class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[10px] px-3 py-1.5 rounded hover:bg-[#00d9ff30]"
                        on:click=move |_| apply_command.with_value(|f| f())
                    >
                        "Apply"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Speed input component with validation and dynamic unit display
#[component]
fn SpeedInput(
    value: ReadSignal<f64>,
    set_value: WriteSignal<f64>,
    is_cartesian: impl Fn() -> bool + Send + Sync + 'static,
) -> impl IntoView {
    let (text_value, set_text_value) = signal(format!("{:.0}", value.get_untracked()));
    let (is_valid, set_is_valid) = signal(true);

    // Sync text when value changes externally
    Effect::new(move |_| {
        let v = value.get();
        set_text_value.set(format!("{:.0}", v));
    });

    view! {
        <input
            type="text"
            class=move || format!(
                "flex-1 bg-[#111111] rounded px-2 py-1 text-[10px] text-white focus:outline-none {}",
                if is_valid.get() {
                    "border border-[#ffffff08] focus:border-[#00d9ff]"
                } else {
                    "border-2 border-[#ff4444] focus:border-[#ff4444]"
                }
            )
            prop:value=move || text_value.get()
            on:input=move |ev| {
                let val = event_target_value(&ev);
                set_text_value.set(val.clone());
                match val.parse::<f64>() {
                    Ok(v) => {
                        set_is_valid.set(true);
                        set_value.set(v);
                    }
                    Err(_) => {
                        set_is_valid.set(false);
                    }
                }
            }
        />
        <span class="text-[8px] text-[#555555]">
            {move || if is_cartesian() { "mm/s" } else { "%" }}
        </span>
    }
}

/// Number input helper component with unit display and validation
#[component]
fn NumberInput(
    label: &'static str,
    value: ReadSignal<f64>,
    set_value: WriteSignal<f64>,
    step: f64,
    #[prop(default = "")] unit: &'static str,
) -> impl IntoView {
    let (text_value, set_text_value) = signal(format!("{:.1}", value.get_untracked()));
    let (is_valid, set_is_valid) = signal(true);

    // Sync text when value changes externally
    Effect::new(move |_| {
        let v = value.get();
        set_text_value.set(format!("{:.1}", v));
    });

    view! {
        <div>
            <label class="block text-[8px] text-[#555555] mb-0.5">{label}</label>
            <input
                type="text"
                class=move || format!(
                    "w-full bg-[#111111] rounded px-1.5 py-1 text-[10px] text-white focus:outline-none text-center {}",
                    if is_valid.get() {
                        "border border-[#ffffff08] focus:border-[#00d9ff]"
                    } else {
                        "border-2 border-[#ff4444] focus:border-[#ff4444]"
                    }
                )
                prop:value=move || text_value.get()
                on:input=move |ev| {
                    let val = event_target_value(&ev);
                    set_text_value.set(val.clone());
                    match val.parse::<f64>() {
                        Ok(v) => {
                            set_is_valid.set(true);
                            set_value.set(v);
                        }
                        Err(_) => {
                            set_is_valid.set(false);
                        }
                    }
                }
            />
            {(!unit.is_empty()).then(|| view! {
                <div class="text-[7px] text-[#444444] text-center mt-0.5">{unit}</div>
            })}
        </div>
    }
}