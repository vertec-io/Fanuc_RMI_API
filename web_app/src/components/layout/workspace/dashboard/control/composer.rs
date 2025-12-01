//! Command Composer Modal - wizard for creating motion commands.

use leptos::prelude::*;
use leptos::either::Either;
use crate::components::layout::workspace::context::{WorkspaceContext, RecentCommand};

/// Command Composer Modal - wizard for creating motion commands
#[component]
pub fn CommandComposerModal() -> impl IntoView {
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
                <ComposerHeader is_editing=is_editing ctx=ctx/>

                // Step indicator
                <StepIndicator step=step/>

                // Content - Step 1: Command Type
                <div class="flex-1 overflow-y-auto p-3">
                    {move || match step.get() {
                        1 => Either::Left(Either::Left(view! {
                            <Step1TypeSelection cmd_type=cmd_type set_cmd_type=set_cmd_type/>
                        })),
                        2 => Either::Left(Either::Right(view! {
                            <Step2Parameters
                                x=x set_x=set_x y=y set_y=set_y z=z set_z=set_z
                                w=w set_w=set_w p=p set_p=set_p r=r set_r=set_r
                                speed=speed set_speed=set_speed
                                term_type=term_type set_term_type=set_term_type
                                uframe=uframe set_uframe=set_uframe
                                utool=utool set_utool=set_utool
                            />
                        })),
                        _ => Either::Right(view! {
                            <Step3Preview
                                cmd_type=cmd_type x=x y=y z=z w=w p=p r=r
                                speed=speed term_type=term_type uframe=uframe utool=utool
                            />
                        }),
                    }}
                </div>

                // Footer
                <ComposerFooter
                    step=step set_step=set_step ctx=ctx
                    cmd_type=cmd_type x=x y=y z=z w=w p=p r=r
                    speed=speed term_type=term_type uframe=uframe utool=utool
                />
            </div>
        </div>
    }
}

/// Composer modal header
#[component]
fn ComposerHeader(is_editing: bool, ctx: WorkspaceContext) -> impl IntoView {
    view! {
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
    }
}

/// Step indicator component
#[component]
fn StepIndicator(step: ReadSignal<usize>) -> impl IntoView {
    view! {
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
    }
}

/// Step 1: Command type selection
#[component]
fn Step1TypeSelection(
    cmd_type: ReadSignal<String>,
    set_cmd_type: WriteSignal<String>,
) -> impl IntoView {
    view! {
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
    }
}

/// Step 2: Parameters input
#[component]
fn Step2Parameters(
    x: ReadSignal<f64>, set_x: WriteSignal<f64>,
    y: ReadSignal<f64>, set_y: WriteSignal<f64>,
    z: ReadSignal<f64>, set_z: WriteSignal<f64>,
    w: ReadSignal<f64>, set_w: WriteSignal<f64>,
    p: ReadSignal<f64>, set_p: WriteSignal<f64>,
    r: ReadSignal<f64>, set_r: WriteSignal<f64>,
    speed: ReadSignal<f64>, set_speed: WriteSignal<f64>,
    term_type: ReadSignal<String>, set_term_type: WriteSignal<String>,
    uframe: ReadSignal<u8>, set_uframe: WriteSignal<u8>,
    utool: ReadSignal<u8>, set_utool: WriteSignal<u8>,
) -> impl IntoView {
    view! {
        <div class="space-y-3">
            // Position inputs
            <div>
                <label class="block text-[10px] text-[#888888] mb-1">"Position (mm / degrees)"</label>
                <div class="grid grid-cols-3 gap-1">
                    <NumberInput label="X" value=x set_value=set_x step=0.1/>
                    <NumberInput label="Y" value=y set_value=set_y step=0.1/>
                    <NumberInput label="Z" value=z set_value=set_z step=0.1/>
                </div>
                <div class="grid grid-cols-3 gap-1 mt-1">
                    <NumberInput label="W" value=w set_value=set_w step=0.1/>
                    <NumberInput label="P" value=p set_value=set_p step=0.1/>
                    <NumberInput label="R" value=r set_value=set_r step=0.1/>
                </div>
            </div>

            // Motion config
            <div>
                <label class="block text-[10px] text-[#888888] mb-1">"Motion Config"</label>
                <div class="grid grid-cols-4 gap-1">
                    <NumberInput label="Speed" value=speed set_value=set_speed step=1.0/>
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
                    <U8Input label="UFrame" value=uframe set_value=set_uframe min=0 max=9/>
                    <U8Input label="UTool" value=utool set_value=set_utool min=0 max=9/>
                </div>
            </div>
        </div>
    }
}

/// Number input helper component
#[component]
fn NumberInput(
    label: &'static str,
    value: ReadSignal<f64>,
    set_value: WriteSignal<f64>,
    step: f64,
) -> impl IntoView {
    view! {
        <div>
            <label class="block text-[8px] text-[#555555] mb-0.5">{label}</label>
            <input
                type="number"
                step=step
                class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                prop:value=move || value.get()
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse() {
                        set_value.set(v);
                    }
                }
            />
        </div>
    }
}

/// U8 input helper component
#[component]
fn U8Input(
    label: &'static str,
    value: ReadSignal<u8>,
    set_value: WriteSignal<u8>,
    min: u8,
    max: u8,
) -> impl IntoView {
    view! {
        <div>
            <label class="block text-[8px] text-[#555555] mb-0.5">{label}</label>
            <input
                type="number"
                min=min
                max=max
                class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                prop:value=move || value.get()
                on:input=move |ev| {
                    if let Ok(v) = event_target_value(&ev).parse() {
                        set_value.set(v);
                    }
                }
            />
        </div>
    }
}

/// Step 3: Preview
#[component]
fn Step3Preview(
    cmd_type: ReadSignal<String>,
    x: ReadSignal<f64>,
    y: ReadSignal<f64>,
    z: ReadSignal<f64>,
    w: ReadSignal<f64>,
    p: ReadSignal<f64>,
    r: ReadSignal<f64>,
    speed: ReadSignal<f64>,
    term_type: ReadSignal<String>,
    uframe: ReadSignal<u8>,
    utool: ReadSignal<u8>,
) -> impl IntoView {
    view! {
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
    }
}

/// Composer footer with navigation and apply buttons
#[component]
fn ComposerFooter(
    step: ReadSignal<usize>,
    set_step: WriteSignal<usize>,
    ctx: WorkspaceContext,
    cmd_type: ReadSignal<String>,
    x: ReadSignal<f64>,
    y: ReadSignal<f64>,
    z: ReadSignal<f64>,
    w: ReadSignal<f64>,
    p: ReadSignal<f64>,
    r: ReadSignal<f64>,
    speed: ReadSignal<f64>,
    term_type: ReadSignal<String>,
    uframe: ReadSignal<u8>,
    utool: ReadSignal<u8>,
) -> impl IntoView {
    view! {
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
    }
}
