//! Dashboard Info tab - Frame and tool management display.
//!
//! Contains components for viewing user frames and user tools.
//! Uses live data from the robot via the Frame/Tool RMI API.
//! Joint angles are displayed in the Control tab's JointJogPanel.

use leptos::prelude::*;
use crate::components::layout::LayoutContext;
use crate::components::layout::workspace::context::WorkspaceContext;
use crate::websocket::WebSocketManager;

/// Info tab showing frame, tool, and joint data.
/// Loads frame/tool data from the robot on mount and syncs active frame/tool with context.
#[component]
pub fn InfoTab() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");

    // Load all frame and tool data on mount
    Effect::new(move || {
        // Only load if WebSocket is connected
        if ws.connected.get() {
            // Request active frame/tool
            ws.get_active_frame_tool();

            // Request all frame data (0-9)
            for i in 0..10u8 {
                ws.read_frame_data(i);
            }

            // Request all tool data (0-9)
            for i in 0..10u8 {
                ws.read_tool_data(i);
            }

            // Request robot configurations if connected to a robot
            if let Some(conn_id) = ws.active_connection_id.get() {
                ws.list_robot_configurations(conn_id);
            }
        }
    });

    // Sync active frame/tool from robot response to context
    Effect::new(move || {
        if let Some((uframe, utool)) = ws.active_frame_tool.get() {
            ctx.active_frame.set(uframe as usize);
            ctx.active_tool.set(utool as usize);
        }
    });

    view! {
        <div class="h-full flex flex-col gap-2 overflow-hidden">
            // Active Configuration Panel (full width at top)
            <ActiveConfigurationPanel/>

            // Jog Defaults Panel (full width)
            <JogDefaultsPanel/>

            // Two-column layout for frames and tools
            <div class="flex-1 grid grid-cols-2 gap-2 min-h-0 overflow-hidden">
                // Left column - Frames
                <div class="flex flex-col gap-2 min-h-0 overflow-hidden">
                    <FrameManagementPanel/>
                    <MultiFrameDisplay/>
                </div>

                // Right column - Tools
                <div class="flex flex-col gap-2 min-h-0 overflow-hidden">
                    <ToolManagementPanel/>
                    <MultiToolDisplay/>
                </div>
            </div>
        </div>
    }
}

/// Active Configuration Panel - Shows loaded config, UFrame/UTool selectors, and arm config
#[component]
fn ActiveConfigurationPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let active_config = ws.active_configuration;
    let robot_configs = ws.robot_configurations;
    let robot_connected = ws.robot_connected;

    // Local state for pending UFrame/UTool changes (before Apply)
    let (pending_uframe, set_pending_uframe) = signal::<Option<i32>>(None);
    let (pending_utool, set_pending_utool) = signal::<Option<i32>>(None);

    // Show confirmation modal
    let (show_confirm_modal, set_show_confirm_modal) = signal(false);

    // Check if there are pending changes
    let has_pending_changes = move || {
        pending_uframe.get().is_some() || pending_utool.get().is_some()
    };

    // Get current UFrame/UTool from active config
    let current_uframe = move || {
        active_config.get().map(|c| c.u_frame_number).unwrap_or(0)
    };
    let current_utool = move || {
        active_config.get().map(|c| c.u_tool_number).unwrap_or(0)
    };

    // Get effective UFrame/UTool (pending or current)
    let effective_uframe = move || {
        pending_uframe.get().unwrap_or_else(|| current_uframe())
    };
    let effective_utool = move || {
        pending_utool.get().unwrap_or_else(|| current_utool())
    };

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3 shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                    </svg>
                    "Active Configuration"
                </h3>

                <div class="grid grid-cols-2 gap-4">
                    // Left side - Configuration selector and UFrame/UTool
                    <div class="space-y-2">
                        // Configuration dropdown with Revert button
                        <div class="flex items-center gap-2">
                            <label class="text-[9px] text-[#666666] w-16">"Loaded From:"</label>
                            <select
                                class="flex-1 bg-[#111111] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-white"
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    if let Ok(id) = value.parse::<i64>() {
                                        ws.load_configuration(id);
                                        // Clear pending changes when loading new config
                                        set_pending_uframe.set(None);
                                        set_pending_utool.set(None);
                                    }
                                }
                            >
                                <option value="" selected=move || active_config.get().and_then(|c| c.loaded_from_id).is_none()>
                                    "Custom"
                                </option>
                                {move || robot_configs.get().into_iter().map(|config| {
                                    let id = config.id;
                                    let name = config.name.clone();
                                    let is_selected = move || active_config.get().and_then(|c| c.loaded_from_id) == Some(id);
                                    view! {
                                        <option value={id.to_string()} selected=is_selected>
                                            {name}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                            // Revert button (only show if modified)
                            <Show when=move || active_config.get().map(|c| c.modified).unwrap_or(false)>
                                <button
                                    class="px-2 py-1 text-[9px] bg-[#ffaa0020] text-[#ffaa00] border border-[#ffaa00] rounded hover:bg-[#ffaa0030]"
                                    on:click=move |_| {
                                        if let Some(config) = active_config.get() {
                                            if let Some(id) = config.loaded_from_id {
                                                ws.load_configuration(id);
                                                set_pending_uframe.set(None);
                                                set_pending_utool.set(None);
                                            }
                                        }
                                    }
                                    title="Revert to saved configuration"
                                >
                                    "Revert"
                                </button>
                            </Show>
                        </div>

                        // UFrame/UTool selectors with Apply button
                        <div class="flex items-center gap-2 bg-[#111111] rounded p-2 border border-[#ffffff08]">
                            <div class="flex items-center gap-1">
                                <label class="text-[9px] text-[#666666]">"UFrame:"</label>
                                <select
                                    class="bg-[#0a0a0a] border border-[#ffffff15] rounded px-1.5 py-0.5 text-[10px] text-white w-12"
                                    on:change=move |ev| {
                                        let value = event_target_value(&ev);
                                        if let Ok(v) = value.parse::<i32>() {
                                            set_pending_uframe.set(Some(v));
                                        }
                                    }
                                >
                                    {(0..10).map(|i| {
                                        let is_selected = move || effective_uframe() == i;
                                        view! {
                                            <option value={i.to_string()} selected=is_selected>
                                                {i}
                                            </option>
                                        }
                                    }).collect_view()}
                                </select>
                            </div>
                            <div class="flex items-center gap-1">
                                <label class="text-[9px] text-[#666666]">"UTool:"</label>
                                <select
                                    class="bg-[#0a0a0a] border border-[#ffffff15] rounded px-1.5 py-0.5 text-[10px] text-white w-12"
                                    on:change=move |ev| {
                                        let value = event_target_value(&ev);
                                        if let Ok(v) = value.parse::<i32>() {
                                            set_pending_utool.set(Some(v));
                                        }
                                    }
                                >
                                    {(0..10).map(|i| {
                                        let is_selected = move || effective_utool() == i;
                                        view! {
                                            <option value={i.to_string()} selected=is_selected>
                                                {i}
                                            </option>
                                        }
                                    }).collect_view()}
                                </select>
                            </div>
                            // Apply button (only show if pending changes)
                            <Show when=has_pending_changes>
                                <button
                                    class="ml-auto px-2 py-1 text-[9px] bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded hover:bg-[#00d9ff30]"
                                    on:click=move |_| {
                                        set_show_confirm_modal.set(true);
                                    }
                                    title="Apply frame/tool changes to robot"
                                >
                                    "Apply"
                                </button>
                            </Show>
                        </div>
                    </div>

                    // Right side - Arm Configuration (read-only)
                    <div class="bg-[#111111] rounded p-2 border border-[#ffffff08]">
                        <div class="text-[9px] text-[#666666] mb-1">"Arm Configuration"</div>
                        {move || {
                            let config = active_config.get().unwrap_or_default();
                            view! {
                                <div class="grid grid-cols-2 gap-x-3 gap-y-0.5 text-[9px]">
                                    <div class="flex justify-between">
                                        <span class="text-[#555555]">"Front/Back:"</span>
                                        <span class="text-white">{if config.front == 1 { "Front" } else { "Back" }}</span>
                                    </div>
                                    <div class="flex justify-between">
                                        <span class="text-[#555555]">"Up/Down:"</span>
                                        <span class="text-white">{if config.up == 1 { "Up" } else { "Down" }}</span>
                                    </div>
                                    <div class="flex justify-between">
                                        <span class="text-[#555555]">"Left/Right:"</span>
                                        <span class="text-white">{if config.left == 1 { "Left" } else { "Right" }}</span>
                                    </div>
                                    <div class="flex justify-between">
                                        <span class="text-[#555555]">"Flip:"</span>
                                        <span class="text-white">{if config.flip == 1 { "Flip" } else { "NoFlip" }}</span>
                                    </div>
                                    <div class="flex justify-between col-span-2">
                                        <span class="text-[#555555]">"Turn (J4/J5/J6):"</span>
                                        <span class="text-white font-mono">{format!("{}/{}/{}", config.turn4, config.turn5, config.turn6)}</span>
                                    </div>
                                </div>
                            }
                        }}
                    </div>
                </div>
            </div>

            // Confirmation Modal
            <Show when=move || show_confirm_modal.get()>
                <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
                    <div class="bg-[#111111] border border-[#ffaa0040] rounded-lg w-[350px] shadow-xl">
                        // Header
                        <div class="flex items-center p-3 border-b border-[#ffffff08]">
                            <svg class="w-5 h-5 mr-2 text-[#ffaa00]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"/>
                            </svg>
                            <h2 class="text-sm font-semibold text-white">"Change Active Frame/Tool?"</h2>
                        </div>

                        // Content
                        <div class="p-4 space-y-3">
                            // Show changes
                            <div class="bg-[#0a0a0a] rounded p-3 space-y-2">
                                {move || {
                                    let old_uframe = current_uframe();
                                    let new_uframe = effective_uframe();
                                    let old_utool = current_utool();
                                    let new_utool = effective_utool();
                                    view! {
                                        <Show when=move || pending_uframe.get().is_some()>
                                            <div class="flex items-center text-[11px]">
                                                <span class="text-[#888888] w-16">"UFrame:"</span>
                                                <span class="text-[#ff6666] font-mono">{old_uframe}</span>
                                                <span class="text-[#666666] mx-2">"→"</span>
                                                <span class="text-[#66ff66] font-mono">{new_uframe}</span>
                                            </div>
                                        </Show>
                                        <Show when=move || pending_utool.get().is_some()>
                                            <div class="flex items-center text-[11px]">
                                                <span class="text-[#888888] w-16">"UTool:"</span>
                                                <span class="text-[#ff6666] font-mono">{old_utool}</span>
                                                <span class="text-[#666666] mx-2">"→"</span>
                                                <span class="text-[#66ff66] font-mono">{new_utool}</span>
                                            </div>
                                        </Show>
                                    }
                                }}
                            </div>

                            <p class="text-[10px] text-[#888888]">
                                "This will affect all subsequent motion commands."
                            </p>
                        </div>

                        // Footer
                        <div class="flex gap-2 p-3 border-t border-[#ffffff08]">
                            <button
                                class="flex-1 text-[10px] px-4 py-2 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                                on:click=move |_| {
                                    set_show_confirm_modal.set(false);
                                }
                            >
                                "Cancel"
                            </button>
                            <button
                                class="flex-1 text-[10px] px-4 py-2 bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded hover:bg-[#00d9ff30] font-medium"
                                on:click=move |_| {
                                    let uframe = effective_uframe() as u8;
                                    let utool = effective_utool() as u8;
                                    ws.set_active_frame_tool(uframe, utool);
                                    ctx.active_frame.set(uframe as usize);
                                    ctx.active_tool.set(utool as usize);
                                    set_pending_uframe.set(None);
                                    set_pending_utool.set(None);
                                    set_show_confirm_modal.set(false);
                                }
                            >
                                "Apply Changes"
                            </button>
                        </div>
                    </div>
                </div>
            </Show>
        </Show>
    }
}

/// Jog Defaults Panel - Configure per-robot jog speed and step defaults
#[component]
fn JogDefaultsPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");
    let robot_connected = ws.robot_connected;

    // Local state for editing
    let (cart_speed, set_cart_speed) = signal(String::new());
    let (cart_step, set_cart_step) = signal(String::new());
    let (joint_speed, set_joint_speed) = signal(String::new());
    let (joint_step, set_joint_step) = signal(String::new());
    let (has_changes, set_has_changes) = signal(false);

    // Initialize from current layout context values
    Effect::new(move || {
        set_cart_speed.set(format!("{:.1}", layout_ctx.jog_speed.get()));
        set_cart_step.set(format!("{:.1}", layout_ctx.jog_step.get()));
        set_joint_speed.set(format!("{:.1}", layout_ctx.joint_jog_speed.get()));
        set_joint_step.set(format!("{:.1}", layout_ctx.joint_jog_step.get()));
        set_has_changes.set(false);
    });

    // Check if values differ from current
    let check_changes = move || {
        let cs = cart_speed.get().parse::<f64>().unwrap_or(0.0);
        let cst = cart_step.get().parse::<f64>().unwrap_or(0.0);
        let js = joint_speed.get().parse::<f64>().unwrap_or(0.0);
        let jst = joint_step.get().parse::<f64>().unwrap_or(0.0);

        let changed = (cs - layout_ctx.jog_speed.get_untracked()).abs() > 0.01
            || (cst - layout_ctx.jog_step.get_untracked()).abs() > 0.01
            || (js - layout_ctx.joint_jog_speed.get_untracked()).abs() > 0.01
            || (jst - layout_ctx.joint_jog_step.get_untracked()).abs() > 0.01;
        set_has_changes.set(changed);
    };

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3 shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    "Jog Defaults"
                </h3>

                <div class="grid grid-cols-2 gap-4">
                    // Cartesian Jog Defaults
                    <div class="bg-[#111111] rounded p-2 border border-[#ffffff08]">
                        <div class="text-[9px] text-[#666666] mb-1.5">"Cartesian Jog"</div>
                        <div class="grid grid-cols-2 gap-2">
                            <div>
                                <label class="text-[8px] text-[#555555] block mb-0.5">"Speed (mm/s)"</label>
                                <input
                                    type="number"
                                    step="0.1"
                                    min="0.1"
                                    max="1000"
                                    class="w-full bg-[#0a0a0a] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-white"
                                    prop:value=cart_speed
                                    on:input=move |ev| {
                                        set_cart_speed.set(event_target_value(&ev));
                                        check_changes();
                                    }
                                />
                            </div>
                            <div>
                                <label class="text-[8px] text-[#555555] block mb-0.5">"Step (mm)"</label>
                                <input
                                    type="number"
                                    step="0.1"
                                    min="0.1"
                                    max="100"
                                    class="w-full bg-[#0a0a0a] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-white"
                                    prop:value=cart_step
                                    on:input=move |ev| {
                                        set_cart_step.set(event_target_value(&ev));
                                        check_changes();
                                    }
                                />
                            </div>
                        </div>
                    </div>

                    // Joint Jog Defaults
                    <div class="bg-[#111111] rounded p-2 border border-[#ffffff08]">
                        <div class="text-[9px] text-[#666666] mb-1.5">"Joint Jog"</div>
                        <div class="grid grid-cols-2 gap-2">
                            <div>
                                <label class="text-[8px] text-[#555555] block mb-0.5">"Speed (°/s)"</label>
                                <input
                                    type="number"
                                    step="0.1"
                                    min="0.1"
                                    max="100"
                                    class="w-full bg-[#0a0a0a] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-white"
                                    prop:value=joint_speed
                                    on:input=move |ev| {
                                        set_joint_speed.set(event_target_value(&ev));
                                        check_changes();
                                    }
                                />
                            </div>
                            <div>
                                <label class="text-[8px] text-[#555555] block mb-0.5">"Step (°)"</label>
                                <input
                                    type="number"
                                    step="0.1"
                                    min="0.1"
                                    max="90"
                                    class="w-full bg-[#0a0a0a] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-white"
                                    prop:value=joint_step
                                    on:input=move |ev| {
                                        set_joint_step.set(event_target_value(&ev));
                                        check_changes();
                                    }
                                />
                            </div>
                        </div>
                    </div>
                </div>

                // Save button row
                <Show when=move || has_changes.get()>
                    <div class="flex justify-end mt-2 gap-2">
                        <button
                            class="px-3 py-1 text-[9px] bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                            on:click=move |_| {
                                // Reset to current values
                                set_cart_speed.set(format!("{:.1}", layout_ctx.jog_speed.get()));
                                set_cart_step.set(format!("{:.1}", layout_ctx.jog_step.get()));
                                set_joint_speed.set(format!("{:.1}", layout_ctx.joint_jog_speed.get()));
                                set_joint_step.set(format!("{:.1}", layout_ctx.joint_jog_step.get()));
                                set_has_changes.set(false);
                            }
                        >
                            "Cancel"
                        </button>
                        <button
                            class="px-3 py-1 text-[9px] bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded hover:bg-[#00d9ff30]"
                            on:click=move |_| {
                                let cs = cart_speed.get().parse::<f64>().unwrap_or(10.0);
                                let cst = cart_step.get().parse::<f64>().unwrap_or(1.0);
                                let js = joint_speed.get().parse::<f64>().unwrap_or(10.0);
                                let jst = joint_step.get().parse::<f64>().unwrap_or(1.0);

                                // Update layout context (immediate effect)
                                layout_ctx.jog_speed.set(cs);
                                layout_ctx.jog_step.set(cst);
                                layout_ctx.joint_jog_speed.set(js);
                                layout_ctx.joint_jog_step.set(jst);

                                // Save to database
                                if let Some(conn_id) = ws.active_connection_id.get() {
                                    ws.update_robot_jog_defaults(conn_id, cs, cst, js, jst);
                                }

                                set_has_changes.set(false);
                            }
                        >
                            "Save Defaults"
                        </button>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

/// Frame Management Panel - Quick frame selector
/// Clicking a frame button selects it and sends SetActiveFrameTool to the robot.
///
/// NOTE: Uses optimistic update for immediate UI feedback. Server broadcasts
/// ActiveFrameTool on success which will confirm (or correct) the local state.
/// This is acceptable because frame selection should feel instant to the user.
#[component]
fn FrameManagementPanel() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let active_frame = ctx.active_frame;
    let active_tool = ctx.active_tool;

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 uppercase tracking-wide flex items-center">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z"/>
                </svg>
                "User Frames"
            </h3>
            <div class="grid grid-cols-5 gap-0.5">
                {(0..10).map(|i| {
                    let is_active = move || active_frame.get() == i;
                    view! {
                        <button
                            class={move || if is_active() {
                                "bg-[#00d9ff20] border border-[#00d9ff] text-[#00d9ff] text-[9px] py-1 rounded font-medium"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] text-[9px] py-1 rounded hover:border-[#ffffff20] hover:text-[#888888]"
                            }}
                            on:click=move |_| {
                                // Optimistic update for immediate UI feedback
                                // Server broadcasts ActiveFrameTool on success to confirm
                                active_frame.set(i);
                                ws.set_active_frame_tool(i as u8, active_tool.get() as u8);
                            }
                            title={format!("UFrame {}", i)}
                        >
                            {i}
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Tool Management Panel - Quick tool selector
/// Clicking a tool button selects it and sends SetActiveFrameTool to the robot.
///
/// NOTE: Uses optimistic update for immediate UI feedback. Server broadcasts
/// ActiveFrameTool on success which will confirm (or correct) the local state.
#[component]
fn ToolManagementPanel() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let active_frame = ctx.active_frame;
    let active_tool = ctx.active_tool;

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 uppercase tracking-wide flex items-center">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                </svg>
                "User Tools"
            </h3>
            <div class="grid grid-cols-5 gap-0.5">
                {(0..10).map(|i| {
                    let is_active = move || active_tool.get() == i;
                    view! {
                        <button
                            class={move || if is_active() {
                                "bg-[#00d9ff20] border border-[#00d9ff] text-[#00d9ff] text-[9px] py-1 rounded font-medium"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] text-[9px] py-1 rounded hover:border-[#ffffff20] hover:text-[#888888]"
                            }}
                            on:click=move |_| {
                                // Optimistic update for immediate UI feedback
                                // Server broadcasts ActiveFrameTool on success to confirm
                                active_tool.set(i);
                                ws.set_active_frame_tool(active_frame.get() as u8, i as u8);
                            }
                            title={format!("UTool {}", i)}
                        >
                            {i}
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Multi-Frame Display - Accordion showing detailed frame data from robot
#[component]
fn MultiFrameDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let expanded = ctx.expanded_frame;
    let (expand_all, set_expand_all) = signal(false);
    let frame_data = ws.frame_data;

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] flex-1 min-h-0 overflow-hidden flex flex-col">
            <div class="flex items-center justify-between p-2 shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 17V7m0 10a2 2 0 01-2 2H5a2 2 0 01-2-2V7a2 2 0 012-2h2a2 2 0 012 2m0 10a2 2 0 002 2h2a2 2 0 002-2M9 7a2 2 0 012-2h2a2 2 0 012 2m0 10V7m0 10a2 2 0 002 2h2a2 2 0 002-2V7a2 2 0 00-2-2h-2a2 2 0 00-2 2"/>
                    </svg>
                    "Frame Data"
                </h3>
                <div class="flex gap-1">
                    <button
                        class="text-[8px] text-[#666666] hover:text-[#00d9ff] px-1"
                        on:click=move |_| {
                            set_expand_all.set(true);
                            expanded.set(-2);
                        }
                        title="Expand All"
                    >
                        "▼ All"
                    </button>
                    <button
                        class="text-[8px] text-[#666666] hover:text-[#00d9ff] px-1"
                        on:click=move |_| {
                            set_expand_all.set(false);
                            expanded.set(-1);
                        }
                        title="Collapse All"
                    >
                        "▲ All"
                    </button>
                </div>
            </div>
            <div class="flex-1 overflow-y-auto px-2 pb-2 space-y-0.5">
                {(0u8..10).map(|i| {
                    let is_expanded = move || expand_all.get() || expanded.get() == i as i32;
                    view! {
                        <div class="border border-[#ffffff08] rounded overflow-hidden">
                            <button
                                class={move || format!(
                                    "w-full flex items-center justify-between px-2 py-1 text-[9px] transition-colors {}",
                                    if is_expanded() { "bg-[#00d9ff10] text-[#00d9ff]" } else { "bg-[#111111] text-[#888888] hover:bg-[#151515]" }
                                )}
                                on:click=move |_| {
                                    if expand_all.get() {
                                        set_expand_all.set(false);
                                        expanded.set(i as i32);
                                    } else {
                                        expanded.set(if expanded.get() == i as i32 { -1 } else { i as i32 });
                                    }
                                }
                            >
                                <span class="font-medium">{format!("UFrame {}", i)}</span>
                                <svg
                                    class={move || format!("w-2.5 h-2.5 transition-transform {}", if is_expanded() { "" } else { "-rotate-90" })}
                                    fill="none" stroke="currentColor" viewBox="0 0 24 24"
                                >
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                                </svg>
                            </button>
                            <Show when=is_expanded>
                                <div class="px-2 py-1.5 bg-[#0d0d0d] border-t border-[#ffffff08]">
                                    {move || {
                                        let data = frame_data.get();
                                        let fd = data.get(&i).cloned().unwrap_or_default();
                                        view! {
                                            <div class="grid grid-cols-3 gap-x-2 gap-y-0.5 text-[9px]">
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"X"</span>
                                                    <span class="text-white font-mono tabular-nums">{format!("{:.3}", fd.x)}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"Y"</span>
                                                    <span class="text-white font-mono tabular-nums">{format!("{:.3}", fd.y)}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"Z"</span>
                                                    <span class="text-white font-mono tabular-nums">{format!("{:.3}", fd.z)}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"W"</span>
                                                    <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", fd.w)}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"P"</span>
                                                    <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", fd.p)}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"R"</span>
                                                    <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", fd.r)}</span>
                                                </div>
                                            </div>
                                        }
                                    }}
                                </div>
                            </Show>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Multi-Tool Display - Accordion showing detailed tool geometry from robot
#[component]
fn MultiToolDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let expanded = ctx.expanded_tool;
    let (expand_all, set_expand_all) = signal(false);
    let tool_data = ws.tool_data;

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] flex-1 min-h-0 overflow-hidden flex flex-col">
            <div class="flex items-center justify-between p-2 shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                    </svg>
                    "Tool Geometry"
                </h3>
                <div class="flex gap-1">
                    <button
                        class="text-[8px] text-[#666666] hover:text-[#00d9ff] px-1"
                        on:click=move |_| {
                            set_expand_all.set(true);
                            expanded.set(-2);
                        }
                        title="Expand All"
                    >
                        "▼ All"
                    </button>
                    <button
                        class="text-[8px] text-[#666666] hover:text-[#00d9ff] px-1"
                        on:click=move |_| {
                            set_expand_all.set(false);
                            expanded.set(-1);
                        }
                        title="Collapse All"
                    >
                        "▲ All"
                    </button>
                </div>
            </div>
            <div class="flex-1 overflow-y-auto px-2 pb-2 space-y-0.5">
                {(0u8..10).map(|i| {
                    let is_expanded = move || expand_all.get() || expanded.get() == i as i32;
                    view! {
                        <div class="border border-[#ffffff08] rounded overflow-hidden">
                            <button
                                class={move || format!(
                                    "w-full flex items-center justify-between px-2 py-1 text-[9px] transition-colors {}",
                                    if is_expanded() { "bg-[#00d9ff10] text-[#00d9ff]" } else { "bg-[#111111] text-[#888888] hover:bg-[#151515]" }
                                )}
                                on:click=move |_| {
                                    if expand_all.get() {
                                        set_expand_all.set(false);
                                        expanded.set(i as i32);
                                    } else {
                                        expanded.set(if expanded.get() == i as i32 { -1 } else { i as i32 });
                                    }
                                }
                            >
                                <span class="font-medium">{format!("UTool {}", i)}</span>
                                <svg
                                    class={move || format!("w-2.5 h-2.5 transition-transform {}", if is_expanded() { "" } else { "-rotate-90" })}
                                    fill="none" stroke="currentColor" viewBox="0 0 24 24"
                                >
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                                </svg>
                            </button>
                            <Show when=is_expanded>
                                <div class="px-2 py-1.5 bg-[#0d0d0d] border-t border-[#ffffff08]">
                                    {move || {
                                        let data = tool_data.get();
                                        let td = data.get(&i).cloned().unwrap_or_default();
                                        view! {
                                            <div class="grid grid-cols-3 gap-x-2 gap-y-0.5 text-[9px]">
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"X"</span>
                                                    <span class="text-white font-mono tabular-nums">{format!("{:.3}", td.x)}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"Y"</span>
                                                    <span class="text-white font-mono tabular-nums">{format!("{:.3}", td.y)}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"Z"</span>
                                                    <span class="text-white font-mono tabular-nums">{format!("{:.3}", td.z)}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"W"</span>
                                                    <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", td.w)}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"P"</span>
                                                    <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", td.p)}</span>
                                                </div>
                                                <div class="flex justify-between">
                                                    <span class="text-[#666666]">"R"</span>
                                                    <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", td.r)}</span>
                                                </div>
                                            </div>
                                        }
                                    }}
                                </div>
                            </Show>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}
