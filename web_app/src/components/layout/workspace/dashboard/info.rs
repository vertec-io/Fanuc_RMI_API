//! Dashboard Configuration tab - Frame and tool management display.
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

            // Request all frame data (1-9)
            // Note: Frame 0 (world frame) cannot be read - causes timeout
            // Valid frames are 1-9 only
            for i in 1..=9u8 {
                ws.read_frame_data(i);
            }

            // Request all tool data (1-10)
            // Note: Tool 0 does not exist - returns error
            // Valid tools are 1-10 only
            for i in 1..=10u8 {
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

    let robot_connected = ws.robot_connected;

    view! {
        <div class="h-full flex flex-col gap-2 overflow-y-auto">
            // Show "No Robot Connected" message when not connected
            <Show when=move || !robot_connected.get() fallback=move || {
                view! {
                    // Active Configuration Panel (full width at top)
                    <ActiveConfigurationPanel/>

                    // Jog Defaults Panel (full width)
                    <JogDefaultsPanel/>

                    // Two-column layout for frames and tools
                    <div class="grid grid-cols-2 gap-2">
                        // Left column - Frames
                        <div class="flex flex-col gap-2">
                            <FrameManagementPanel/>
                            <MultiFrameDisplay/>
                        </div>

                        // Right column - Tools
                        <div class="flex flex-col gap-2">
                            <ToolManagementPanel/>
                            <MultiToolDisplay/>
                        </div>
                    </div>
                }
            }>
                <div class="h-full flex items-center justify-center">
                    <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-8 max-w-md text-center">
                        <svg class="w-16 h-16 mx-auto mb-4 text-[#666666]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                        </svg>
                        <h2 class="text-lg font-semibold text-white mb-2">"No Robot Connected"</h2>
                        <p class="text-sm text-[#888888] mb-4">
                            "Connect to a robot to view and configure frame/tool settings, jog defaults, and arm configuration."
                        </p>
                        <p class="text-xs text-[#666666]">
                            "Use the Settings panel to create a robot connection, then connect from the Dashboard."
                        </p>
                    </div>
                </div>
            </Show>
        </div>
    }
}

/// Active Configuration Panel - Shows loaded config and arm config (read-only display)
/// UFrame/UTool selection is done in the Frame/Tool Management Panels below
#[component]
fn ActiveConfigurationPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let active_config = ws.active_configuration;
    let robot_configs = ws.robot_configurations;
    let robot_connected = ws.robot_connected;
    let program_running = ws.program_running;

    // Modal state for save confirmation
    let (show_save_modal, set_show_save_modal) = signal(false);

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3 shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center group">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                    </svg>
                    "Active Configuration"
                    <svg
                        class="w-2.5 h-2.5 ml-1 text-[#666666] group-hover:text-[#00d9ff]"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                        title="Displays the currently loaded robot configuration. Select a different saved configuration to load its UFrame, UTool, and arm configuration settings to the robot."
                    >
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                </h3>

                <div class="grid grid-cols-2 gap-4">
                    // Left side - Configuration selector
                    <div class="space-y-2">
                        // Configuration dropdown with Revert button
                        <div class="flex items-center gap-2">
                            <label class="text-[9px] text-[#666666] w-16">"Loaded From:"</label>
                            <select
                                class=move || format!(
                                    "flex-1 bg-[#111111] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-white {}",
                                    if program_running.get() { "opacity-50 cursor-not-allowed" } else { "" }
                                )
                                disabled=move || program_running.get()
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    if let Ok(id) = value.parse::<i64>() {
                                        ws.load_configuration(id);
                                    }
                                }
                            >
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
                            // Revert button (only show if changes have been made)
                            <Show when=move || active_config.get().map(|c| c.changes_count > 0).unwrap_or(false)>
                                <button
                                    class="px-2 py-1 text-[9px] bg-[#ffaa0020] text-[#ffaa00] border border-[#ffaa00] rounded hover:bg-[#ffaa0030]"
                                    on:click=move |_| {
                                        if let Some(config) = active_config.get() {
                                            if let Some(id) = config.loaded_from_id {
                                                ws.load_configuration(id);
                                            }
                                        }
                                    }
                                    title="Revert to saved configuration"
                                    disabled=move || program_running.get()
                                >
                                    "Revert"
                                </button>
                            </Show>
                            // Save button (only show if changes have been made)
                            <Show when=move || active_config.get().map(|c| c.changes_count > 0).unwrap_or(false)>
                                <button
                                    class="px-2 py-1 text-[9px] bg-[#22c55e20] text-[#22c55e] border border-[#22c55e] rounded hover:bg-[#22c55e30] disabled:opacity-50 disabled:cursor-not-allowed"
                                    on:click=move |_| {
                                        set_show_save_modal.set(true);
                                    }
                                    title="Save current configuration to database"
                                    disabled=move || program_running.get()
                                >
                                    "Save"
                                </button>
                            </Show>
                        </div>

                        // Current UFrame/UTool display (read-only)
                        <div class="bg-[#111111] rounded p-2 border border-[#ffffff08]">
                            <div class="text-[9px] text-[#666666] mb-1">"Active Frame/Tool"</div>
                            {move || {
                                let config = active_config.get().unwrap_or_default();
                                view! {
                                    <div class="flex gap-4 text-[10px]">
                                        <div class="flex items-center gap-1">
                                            <span class="text-[#555555]">"UFrame:"</span>
                                            <span class="text-[#00d9ff] font-medium">{config.u_frame_number}</span>
                                        </div>
                                        <div class="flex items-center gap-1">
                                            <span class="text-[#555555]">"UTool:"</span>
                                            <span class="text-[#00d9ff] font-medium">{config.u_tool_number}</span>
                                        </div>
                                    </div>
                                    <div class="text-[8px] text-[#555555] mt-1">
                                        "Use panels below to change"
                                    </div>
                                }
                            }}
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

                // Save Configuration Modal
                <Show when=move || show_save_modal.get()>
                    <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
                        on:click=move |_| set_show_save_modal.set(false)
                    >
                        <div class="bg-[#0a0a0a] border border-[#ffffff15] rounded-lg p-6 max-w-md"
                            on:click=move |e| e.stop_propagation()
                        >
                            <h3 class="text-sm font-semibold text-white mb-3 flex items-center gap-2">
                                <svg class="w-4 h-4 text-[#22c55e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"/>
                                </svg>
                                "Save Configuration"
                            </h3>
                            <p class="text-xs text-[#aaaaaa] mb-4">
                                "This will save the current active configuration (UFrame, UTool, arm config) and jog settings to the database."
                            </p>
                            {move || {
                                let config = active_config.get().unwrap_or_default();
                                view! {
                                    <div class="bg-[#111111] rounded p-3 mb-4 border border-[#ffffff08]">
                                        <div class="text-[9px] text-[#666666] mb-2">"Configuration to Save:"</div>
                                        <div class="text-[10px] text-white font-medium mb-1">
                                            {config.loaded_from_name.clone().unwrap_or_else(|| "Unnamed Configuration".to_string())}
                                        </div>
                                        <div class="text-[9px] text-[#888888]">
                                            {format!("UFrame: {}, UTool: {}", config.u_frame_number, config.u_tool_number)}
                                        </div>
                                        <div class="text-[9px] text-[#ffaa00] mt-2">
                                            {format!("{} unsaved change{}", config.changes_count, if config.changes_count == 1 { "" } else { "s" })}
                                        </div>
                                    </div>

                                    // Changelog display
                                    {
                                        let change_log = config.change_log.clone();
                                        let has_changes = !change_log.is_empty();
                                        if has_changes {
                                            view! {
                                                <div class="bg-[#111111] rounded p-3 mb-4 border border-[#ffffff08] max-h-64 overflow-y-auto">
                                                    <div class="text-[9px] text-[#666666] mb-2">"Changes to be saved:"</div>
                                                    <div class="space-y-1">
                                                        {change_log.iter().enumerate().map(|(i, entry)| {
                                                            let field_name = entry.field_name.clone();
                                                            let old_value = entry.old_value.clone();
                                                            let new_value = entry.new_value.clone();
                                                            view! {
                                                                <div class="text-[9px] text-[#aaaaaa]">
                                                                    <span class="text-[#666666]">{format!("{}. ", i + 1)}</span>
                                                                    "Change "
                                                                    <span class="text-[#22c55e]">{field_name}</span>
                                                                    " from "
                                                                    <span class="text-[#ff6b6b]">{old_value}</span>
                                                                    " to "
                                                                    <span class="text-[#22c55e]">{new_value}</span>
                                                                </div>
                                                            }
                                                        }).collect_view()}
                                                    </div>
                                                </div>
                                            }.into_any()
                                        } else {
                                            view! {}.into_any()
                                        }
                                    }
                                }
                            }}
                            <div class="flex justify-end gap-2">
                                <button
                                    class="px-3 py-1.5 text-[10px] bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                                    on:click=move |_| set_show_save_modal.set(false)
                                >
                                    "Cancel"
                                </button>
                                <button
                                    class="px-3 py-1.5 text-[10px] bg-[#22c55e20] text-[#22c55e] border border-[#22c55e] rounded hover:bg-[#22c55e30]"
                                    on:click=move |_| {
                                        ws.save_current_configuration(None);
                                        set_show_save_modal.set(false);
                                    }
                                >
                                    "Save Configuration"
                                </button>
                            </div>
                        </div>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

/// Jog Defaults Panel - Configure per-robot jog speed and step defaults (saved to database)
#[component]
fn JogDefaultsPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let robot_connected = ws.robot_connected;
    let active_configuration = ws.active_configuration;
    let active_jog_settings = ws.active_jog_settings;
    let program_running = ws.program_running;

    // Derive active connection from robot_connections and active_connection_id
    let active_connection = Memo::new(move |_| ws.get_active_connection());

    // Local state for editing
    let (cart_speed, set_cart_speed) = signal(String::new());
    let (cart_step, set_cart_step) = signal(String::new());
    let (joint_speed, set_joint_speed) = signal(String::new());
    let (joint_step, set_joint_step) = signal(String::new());
    let (has_changes, set_has_changes) = signal(false);

    // Initialize from active default jog settings (from active_configuration)
    Effect::new(move || {
        if let Some(config) = active_configuration.get() {
            set_cart_speed.set(format!("{:.1}", config.default_cartesian_jog_speed));
            set_cart_step.set(format!("{:.1}", config.default_cartesian_jog_step));
            set_joint_speed.set(format!("{:.1}", config.default_joint_jog_speed));
            set_joint_step.set(format!("{:.1}", config.default_joint_jog_step));
            set_has_changes.set(false);
        }
    });

    // Check if edited values differ from active default jog settings
    let check_changes = move || {
        if let Some(config) = active_configuration.get_untracked() {
            let cs = cart_speed.get().parse::<f64>().unwrap_or(0.0);
            let cst = cart_step.get().parse::<f64>().unwrap_or(0.0);
            let js = joint_speed.get().parse::<f64>().unwrap_or(0.0);
            let jst = joint_step.get().parse::<f64>().unwrap_or(0.0);

            let changed = (cs - config.default_cartesian_jog_speed).abs() > 0.01
                || (cst - config.default_cartesian_jog_step).abs() > 0.01
                || (js - config.default_joint_jog_speed).abs() > 0.01
                || (jst - config.default_joint_jog_step).abs() > 0.01;
            set_has_changes.set(changed);
        }
    };

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3 shrink-0">
                <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center group">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    "Jog Defaults"
                    <svg
                        class="w-2.5 h-2.5 ml-1 text-[#666666] group-hover:text-[#00d9ff]"
                        fill="none"
                        stroke="currentColor"
                        viewBox="0 0 24 24"
                        title="Default jog speed and step values loaded when connecting to the robot. These values are used as initial settings for the Jog and Joint Jog panels. Click 'Apply' to update the active jog settings, or 'Save' to update the robot's default configuration."
                    >
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                    </svg>
                </h3>

                <div class="grid grid-cols-2 gap-4">
                    // Cartesian Jog Defaults
                    <div class="bg-[#111111] rounded p-2 border border-[#ffffff08]">
                        <div class="text-[9px] text-[#666666] mb-1.5">"Cartesian Jog"</div>
                        <div class="grid grid-cols-2 gap-2">
                            <div>
                                <label class="text-[8px] text-[#555555] block mb-0.5">"Speed (mm/s)"</label>
                                <NumberInput
                                    value=cart_speed
                                    on_input=move |val: String| {
                                        set_cart_speed.set(val);
                                        check_changes();
                                    }
                                    min=0.1
                                    max=1000.0
                                    disabled=program_running.get()
                                />
                            </div>
                            <div>
                                <label class="text-[8px] text-[#555555] block mb-0.5">"Step (mm)"</label>
                                <NumberInput
                                    value=cart_step
                                    on_input=move |val: String| {
                                        set_cart_step.set(val);
                                        check_changes();
                                    }
                                    min=0.1
                                    max=100.0
                                    disabled=program_running.get()
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
                                <NumberInput
                                    value=joint_speed
                                    on_input=move |val: String| {
                                        set_joint_speed.set(val);
                                        check_changes();
                                    }
                                    min=0.1
                                    max=100.0
                                    disabled=program_running.get()
                                />
                            </div>
                            <div>
                                <label class="text-[8px] text-[#555555] block mb-0.5">"Step (°)"</label>
                                <NumberInput
                                    value=joint_step
                                    on_input=move |val: String| {
                                        set_joint_step.set(val);
                                        check_changes();
                                    }
                                    min=0.1
                                    max=90.0
                                    disabled=program_running.get()
                                />
                            </div>
                        </div>
                    </div>
                </div>

                // Apply button row
                <Show when=move || has_changes.get()>
                    <div class="flex justify-end mt-2 gap-2">
                        <button
                            class="px-3 py-1 text-[9px] bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white disabled:opacity-50 disabled:cursor-not-allowed"
                            on:click=move |_| {
                                // Reset to active default jog settings
                                if let Some(config) = active_configuration.get() {
                                    set_cart_speed.set(format!("{:.1}", config.default_cartesian_jog_speed));
                                    set_cart_step.set(format!("{:.1}", config.default_cartesian_jog_step));
                                    set_joint_speed.set(format!("{:.1}", config.default_joint_jog_speed));
                                    set_joint_step.set(format!("{:.1}", config.default_joint_jog_step));
                                    set_has_changes.set(false);
                                }
                            }
                            disabled=move || program_running.get()
                        >
                            "Cancel"
                        </button>
                        <button
                            class="px-3 py-1 text-[9px] bg-[#ffaa0020] text-[#ffaa00] border border-[#ffaa00] rounded hover:bg-[#ffaa0030] disabled:opacity-50 disabled:cursor-not-allowed"
                            on:click=move |_| {
                                let cs = cart_speed.get().parse::<f64>().unwrap_or(10.0);
                                let cst = cart_step.get().parse::<f64>().unwrap_or(1.0);
                                let js = joint_speed.get().parse::<f64>().unwrap_or(10.0);
                                let jst = joint_step.get().parse::<f64>().unwrap_or(1.0);
                                // Use current rotation settings (this panel doesn't edit rotation)
                                let (rs, rst) = active_jog_settings.get_untracked()
                                    .map(|s| (s.rotation_jog_speed, s.rotation_jog_step))
                                    .unwrap_or((5.0, 1.0));

                                // Apply these values to active jog settings (increments changes_count)
                                ws.apply_jog_settings(cs, cst, js, jst, rs, rst);

                                set_has_changes.set(false);
                            }
                            title="Apply these values to the active jog settings (increments changes counter, does not save to database)"
                            disabled=move || program_running.get()
                        >
                            "Apply"
                        </button>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

/// Frame Management Panel - Frame selector with Apply button
/// User selects a frame, then clicks Apply to send to robot.
/// Has toggle to switch between button grid view and dropdown view.
#[component]
fn FrameManagementPanel() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let active_frame = ctx.active_frame;
    let active_tool = ctx.active_tool;
    let robot_connected = ws.robot_connected;
    let has_control = ws.has_control;
    let program_running = ws.program_running;

    // Local state for pending frame selection
    let (pending_frame, set_pending_frame) = signal::<Option<usize>>(None);

    // View mode: "buttons" or "dropdown"
    let (view_mode, set_view_mode) = signal("buttons");

    // Get effective frame (pending or current)
    let effective_frame = move || {
        pending_frame.get().unwrap_or_else(|| active_frame.get())
    };

    // Check if there are pending changes
    let has_pending = move || {
        pending_frame.get().is_some()
    };

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
                <div class="flex items-center justify-between mb-1.5">
                    <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center group">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z"/>
                        </svg>
                        "User Frames"
                        <svg
                            class="w-2.5 h-2.5 ml-1 text-[#666666] group-hover:text-[#00d9ff]"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                            title="Select the active User Frame (coordinate system) for robot motion commands. Changes require clicking 'Apply' to send to the robot."
                        >
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                        </svg>
                    </h3>
                    // View toggle button
                    <button
                        class="text-[8px] text-[#666666] hover:text-[#00d9ff] px-1.5 py-0.5 border border-[#ffffff08] rounded"
                        on:click=move |_| {
                            if view_mode.get() == "buttons" {
                                set_view_mode.set("dropdown");
                            } else {
                                set_view_mode.set("buttons");
                            }
                        }
                        title="Toggle view mode"
                    >
                        {move || if view_mode.get() == "buttons" { "▼" } else { "▦" }}
                    </button>
                </div>

                // Button grid view
                <Show when=move || view_mode.get() == "buttons" fallback=move || {
                    // Dropdown view
                    view! {
                        <div class="flex items-center gap-2">
                            <select
                                class=move || format!(
                                    "flex-1 bg-[#111111] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-white {}",
                                    if !has_control.get() || program_running.get() { "opacity-50 cursor-not-allowed" } else { "" }
                                )
                                disabled=move || !has_control.get() || program_running.get()
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    if let Ok(v) = value.parse::<usize>() {
                                        set_pending_frame.set(Some(v));
                                    }
                                }
                            >
                                {(0..10).map(|i| {
                                    let is_selected = move || effective_frame() == i;
                                    view! {
                                        <option value={i.to_string()} selected=is_selected>
                                            {format!("UFrame {}", i)}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                            // Apply button
                            <Show when=has_pending>
                                <button
                                    class="px-2 py-1 text-[9px] bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded hover:bg-[#00d9ff30] disabled:opacity-50 disabled:cursor-not-allowed"
                                    on:click=move |_| {
                                        if let Some(frame) = pending_frame.get() {
                                            ws.set_active_frame_tool(frame as u8, active_tool.get() as u8);
                                            set_pending_frame.set(None);
                                        }
                                    }
                                    title="Apply frame change to robot"
                                    disabled=move || program_running.get()
                                >
                                    "Apply"
                                </button>
                            </Show>
                        </div>
                    }
                }>
                    <div class="space-y-1">
                        <div class="grid grid-cols-5 gap-0.5">
                            {(0..10).map(|i| {
                                let is_selected = move || effective_frame() == i;
                                let is_active = move || active_frame.get() == i;
                                let has_ctrl = has_control;
                                let prog_running = program_running;
                                view! {
                                    <button
                                        class={move || {
                                            let disabled = !has_ctrl.get() || prog_running.get();
                                            let selected = is_selected();
                                            let active = is_active();

                                            if selected && active {
                                                // Current active frame
                                                if disabled {
                                                    "bg-[#00d9ff10] border border-[#00d9ff40] text-[#00d9ff60] text-[9px] py-1 rounded font-medium cursor-not-allowed"
                                                } else {
                                                    "bg-[#00d9ff20] border border-[#00d9ff] text-[#00d9ff] text-[9px] py-1 rounded font-medium"
                                                }
                                            } else if selected {
                                                // Pending selection (not yet applied)
                                                "bg-[#ffaa0020] border border-[#ffaa00] text-[#ffaa00] text-[9px] py-1 rounded font-medium"
                                            } else {
                                                // Not selected
                                                if disabled {
                                                    "bg-[#111111] border border-[#ffffff08] text-[#333333] text-[9px] py-1 rounded cursor-not-allowed"
                                                } else {
                                                    "bg-[#111111] border border-[#ffffff08] text-[#555555] text-[9px] py-1 rounded hover:border-[#ffffff20] hover:text-[#888888]"
                                                }
                                            }
                                        }}
                                        disabled=move || !has_ctrl.get() || prog_running.get()
                                        on:click=move |_| {
                                            set_pending_frame.set(Some(i));
                                        }
                                        title=move || if !has_ctrl.get() {
                                            "Request control to change UFrame".to_string()
                                        } else if prog_running.get() {
                                            "Cannot change UFrame while program is running".to_string()
                                        } else {
                                            format!("UFrame {}", i)
                                        }
                                    >
                                        {i}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                        // Apply button (only show if pending changes)
                        <Show when=has_pending>
                            <button
                                class="w-full px-2 py-1 text-[9px] bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded hover:bg-[#00d9ff30] disabled:opacity-50 disabled:cursor-not-allowed"
                                on:click=move |_| {
                                    if let Some(frame) = pending_frame.get() {
                                        ws.set_active_frame_tool(frame as u8, active_tool.get() as u8);
                                        set_pending_frame.set(None);
                                    }
                                }
                                title="Apply frame change to robot"
                                disabled=move || program_running.get()
                            >
                                "Apply"
                            </button>
                        </Show>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

/// Tool Management Panel - Tool selector with Apply button
/// User selects a tool, then clicks Apply to send to robot.
/// Has toggle to switch between button grid view and dropdown view.
#[component]
fn ToolManagementPanel() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let active_frame = ctx.active_frame;
    let active_tool = ctx.active_tool;
    let robot_connected = ws.robot_connected;
    let has_control = ws.has_control;
    let program_running = ws.program_running;

    // Local state for pending tool selection
    let (pending_tool, set_pending_tool) = signal::<Option<usize>>(None);

    // View mode: "buttons" or "dropdown"
    let (view_mode, set_view_mode) = signal("buttons");

    // Get effective tool (pending or current)
    let effective_tool = move || {
        pending_tool.get().unwrap_or_else(|| active_tool.get())
    };

    // Check if there are pending changes
    let has_pending = move || {
        pending_tool.get().is_some()
    };

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
                <div class="flex items-center justify-between mb-1.5">
                    <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center group">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                        </svg>
                        "User Tools"
                        <svg
                            class="w-2.5 h-2.5 ml-1 text-[#666666] group-hover:text-[#00d9ff]"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                            title="Select the active User Tool (TCP offset) for robot motion commands. Changes require clicking 'Apply' to send to the robot."
                        >
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                        </svg>
                    </h3>
                    // View toggle button
                    <button
                        class="text-[8px] text-[#666666] hover:text-[#00d9ff] px-1.5 py-0.5 border border-[#ffffff08] rounded"
                        on:click=move |_| {
                            if view_mode.get() == "buttons" {
                                set_view_mode.set("dropdown");
                            } else {
                                set_view_mode.set("buttons");
                            }
                        }
                        title="Toggle view mode"
                    >
                        {move || if view_mode.get() == "buttons" { "▼" } else { "▦" }}
                    </button>
                </div>

                // Button grid view
                <Show when=move || view_mode.get() == "buttons" fallback=move || {
                    // Dropdown view
                    view! {
                        <div class="flex items-center gap-2">
                            <select
                                class=move || format!(
                                    "flex-1 bg-[#111111] border border-[#ffffff15] rounded px-2 py-1 text-[10px] text-white {}",
                                    if !has_control.get() || program_running.get() { "opacity-50 cursor-not-allowed" } else { "" }
                                )
                                disabled=move || !has_control.get() || program_running.get()
                                on:change=move |ev| {
                                    let value = event_target_value(&ev);
                                    if let Ok(v) = value.parse::<usize>() {
                                        set_pending_tool.set(Some(v));
                                    }
                                }
                            >
                                {(1..=10).map(|i| {
                                    let is_selected = move || effective_tool() == i;
                                    view! {
                                        <option value={i.to_string()} selected=is_selected>
                                            {format!("UTool {}", i)}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                            // Apply button
                            <Show when=has_pending>
                                <button
                                    class="px-2 py-1 text-[9px] bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded hover:bg-[#00d9ff30] disabled:opacity-50 disabled:cursor-not-allowed"
                                    on:click=move |_| {
                                        if let Some(tool) = pending_tool.get() {
                                            ws.set_active_frame_tool(active_frame.get() as u8, tool as u8);
                                            set_pending_tool.set(None);
                                        }
                                    }
                                    title="Apply tool change to robot"
                                    disabled=move || program_running.get()
                                >
                                    "Apply"
                                </button>
                            </Show>
                        </div>
                    }
                }>
                    <div class="space-y-1">
                        <div class="grid grid-cols-5 gap-0.5">
                            {(1..=10).map(|i| {
                                let is_selected = move || effective_tool() == i;
                                let is_active = move || active_tool.get() == i;
                                let has_ctrl = has_control;
                                let prog_running = program_running;
                                view! {
                                    <button
                                        class={move || {
                                            let disabled = !has_ctrl.get() || prog_running.get();
                                            let selected = is_selected();
                                            let active = is_active();

                                            if selected && active {
                                                // Current active tool
                                                if disabled {
                                                    "bg-[#00d9ff10] border border-[#00d9ff40] text-[#00d9ff60] text-[9px] py-1 rounded font-medium cursor-not-allowed"
                                                } else {
                                                    "bg-[#00d9ff20] border border-[#00d9ff] text-[#00d9ff] text-[9px] py-1 rounded font-medium"
                                                }
                                            } else if selected {
                                                // Pending selection (not yet applied)
                                                "bg-[#ffaa0020] border border-[#ffaa00] text-[#ffaa00] text-[9px] py-1 rounded font-medium"
                                            } else {
                                                // Not selected
                                                if disabled {
                                                    "bg-[#111111] border border-[#ffffff08] text-[#333333] text-[9px] py-1 rounded cursor-not-allowed"
                                                } else {
                                                    "bg-[#111111] border border-[#ffffff08] text-[#555555] text-[9px] py-1 rounded hover:border-[#ffffff20] hover:text-[#888888]"
                                                }
                                            }
                                        }}
                                        disabled=move || !has_ctrl.get() || prog_running.get()
                                        on:click=move |_| {
                                            set_pending_tool.set(Some(i));
                                        }
                                        title=move || if !has_ctrl.get() {
                                            "Request control to change UTool".to_string()
                                        } else if prog_running.get() {
                                            "Cannot change UTool while program is running".to_string()
                                        } else {
                                            format!("UTool {}", i)
                                        }
                                    >
                                        {i}
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                        // Apply button (only show if pending changes)
                        <Show when=has_pending>
                            <button
                                class="w-full px-2 py-1 text-[9px] bg-[#00d9ff20] text-[#00d9ff] border border-[#00d9ff] rounded hover:bg-[#00d9ff30] disabled:opacity-50 disabled:cursor-not-allowed"
                                on:click=move |_| {
                                    if let Some(tool) = pending_tool.get() {
                                        ws.set_active_frame_tool(active_frame.get() as u8, tool as u8);
                                        set_pending_tool.set(None);
                                    }
                                }
                                title="Apply tool change to robot"
                                disabled=move || program_running.get()
                            >
                                "Apply"
                            </button>
                        </Show>
                    </div>
                </Show>
            </div>
        </Show>
    }
}

/// Multi-Frame Display - Accordion showing detailed frame data from robot
/// Active frame accordion is automatically expanded when active_frame changes.
#[component]
fn MultiFrameDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let expanded_frames = ctx.expanded_frames;
    let active_frame = ctx.active_frame;
    let (expand_all, set_expand_all) = signal(false);
    let (user_interacted, set_user_interacted) = signal(false); // Track if user manually interacted
    let frame_data = ws.frame_data;
    let robot_connected = ws.robot_connected;

    // Reset user_interacted flag when robot connection changes
    Effect::new(move || {
        let _connected = robot_connected.get();
        // Reset the flag so auto-expand works on new connections
        set_user_interacted.set(false);
        set_expand_all.set(false);
    });

    // Auto-expand active frame when it changes (unless user has manually interacted or expand_all is active)
    Effect::new(move || {
        let active = active_frame.get() as i32;
        // Only auto-expand if:
        // 1. Not in "expand all" mode (expand_all == false)
        // 2. User hasn't manually interacted (user_interacted == false)
        // 3. Active frame is valid (1-9)
        if !expand_all.get() && !user_interacted.get() && active >= 1 && active <= 9 {
            expanded_frames.update(|set| {
                set.clear();
                set.insert(active);
            });
        }
    });

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08]">
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
                            set_user_interacted.set(false);
                            // Expand all frames (1-9)
                            expanded_frames.update(|set| {
                                set.clear();
                                for i in 1..=9 {
                                    set.insert(i);
                                }
                            });
                        }
                        title="Expand All"
                    >
                        "▼ All"
                    </button>
                    <button
                        class="text-[8px] text-[#666666] hover:text-[#00d9ff] px-1"
                        on:click=move |_| {
                            set_expand_all.set(false);
                            set_user_interacted.set(true);
                            // Collapse all frames
                            expanded_frames.update(|set| {
                                set.clear();
                            });
                        }
                        title="Collapse All"
                    >
                        "▲ All"
                    </button>
                </div>
            </div>
            <div class="px-2 pb-2 space-y-0.5">
                {(1u8..=9).map(|i| {
                    let is_expanded = move || {
                        expand_all.get() || expanded_frames.with(|set| set.contains(&(i as i32)))
                    };
                    view! {
                        <div class="border border-[#ffffff08] rounded overflow-hidden">
                            <button
                                class={move || format!(
                                    "w-full flex items-center justify-between px-2 py-1 text-[9px] transition-colors {}",
                                    if is_expanded() { "bg-[#00d9ff10] text-[#00d9ff]" } else { "bg-[#111111] text-[#888888] hover:bg-[#151515]" }
                                )}
                                on:click=move |_| {
                                    // Always exit expand_all mode when clicking individual accordion
                                    set_expand_all.set(false);
                                    // Mark that user has manually interacted
                                    set_user_interacted.set(true);

                                    // Toggle this frame in the set
                                    let frame_num = i as i32;
                                    expanded_frames.update(|set| {
                                        if set.contains(&frame_num) {
                                            set.remove(&frame_num);
                                        } else {
                                            set.insert(frame_num);
                                        }
                                    });
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
        </Show>
    }
}

/// Multi-Tool Display - Accordion showing detailed tool geometry from robot
/// Active tool accordion is automatically expanded when active_tool changes.
#[component]
fn MultiToolDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let expanded_tools = ctx.expanded_tools;
    let active_tool = ctx.active_tool;
    let (expand_all, set_expand_all) = signal(false);
    let (user_interacted, set_user_interacted) = signal(false); // Track if user manually interacted
    let tool_data = ws.tool_data;
    let robot_connected = ws.robot_connected;

    // Reset user_interacted flag when robot connection changes
    Effect::new(move || {
        let _connected = robot_connected.get();
        // Reset the flag so auto-expand works on new connections
        set_user_interacted.set(false);
        set_expand_all.set(false);
    });

    // Auto-expand active tool when it changes (unless user has manually interacted or expand_all is active)
    Effect::new(move || {
        let active = active_tool.get() as i32;
        // Only auto-expand if:
        // 1. Not in "expand all" mode (expand_all == false)
        // 2. User hasn't manually interacted (user_interacted == false)
        // 3. Active tool is valid (1-10)
        if !expand_all.get() && !user_interacted.get() && active >= 1 && active <= 10 {
            expanded_tools.update(|set| {
                set.clear();
                set.insert(active);
            });
        }
    });

    view! {
        <Show when=move || robot_connected.get()>
            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08]">
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
                            set_user_interacted.set(false);
                            // Expand all tools (1-10)
                            expanded_tools.update(|set| {
                                set.clear();
                                for i in 1..=10 {
                                    set.insert(i);
                                }
                            });
                        }
                        title="Expand All"
                    >
                        "▼ All"
                    </button>
                    <button
                        class="text-[8px] text-[#666666] hover:text-[#00d9ff] px-1"
                        on:click=move |_| {
                            set_expand_all.set(false);
                            set_user_interacted.set(true);
                            // Collapse all tools
                            expanded_tools.update(|set| {
                                set.clear();
                            });
                        }
                        title="Collapse All"
                    >
                        "▲ All"
                    </button>
                </div>
            </div>
            <div class="px-2 pb-2 space-y-0.5">
                {(1u8..=10).map(|i| {
                    let is_expanded = move || {
                        expand_all.get() || expanded_tools.with(|set| set.contains(&(i as i32)))
                    };
                    view! {
                        <div class="border border-[#ffffff08] rounded overflow-hidden">
                            <button
                                class={move || format!(
                                    "w-full flex items-center justify-between px-2 py-1 text-[9px] transition-colors {}",
                                    if is_expanded() { "bg-[#00d9ff10] text-[#00d9ff]" } else { "bg-[#111111] text-[#888888] hover:bg-[#151515]" }
                                )}
                                on:click=move |_| {
                                    // Always exit expand_all mode when clicking individual accordion
                                    set_expand_all.set(false);
                                    // Mark that user has manually interacted
                                    set_user_interacted.set(true);

                                    // Toggle this tool in the set
                                    let tool_num = i as i32;
                                    expanded_tools.update(|set| {
                                        if set.contains(&tool_num) {
                                            set.remove(&tool_num);
                                        } else {
                                            set.insert(tool_num);
                                        }
                                    });
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
        </Show>
    }
}

/// Number input component with validation
#[component]
fn NumberInput(
    #[prop(into)] value: Signal<String>,
    on_input: impl Fn(String) + 'static,
    #[prop(optional)] placeholder: &'static str,
    #[prop(default = 0.0)] min: f64,
    #[prop(default = f64::MAX)] max: f64,
    #[prop(default = false)] disabled: bool,
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
                "w-full bg-[#0a0a0a] rounded px-2 py-1 text-[10px] text-white {} {}",
                if is_valid() {
                    "border border-[#ffffff15]"
                } else {
                    "border-2 border-[#ff4444]"
                },
                if disabled { "opacity-50 cursor-not-allowed" } else { "" }
            )
            placeholder=placeholder
            prop:value=value
            disabled=disabled
            on:input=move |ev| {
                on_input(event_target_value(&ev));
            }
        />
    }
}
