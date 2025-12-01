//! Main workspace component - the central content area.

use leptos::prelude::*;
use leptos::either::Either;
use leptos_router::components::{Routes, Route, ParentRoute, Outlet, Redirect, A};
use leptos_router::hooks::{use_location, use_navigate};
use leptos_router::path;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, FileReader};
use super::LayoutContext;
use crate::websocket::WebSocketManager;
use fanuc_rmi::dto::{SendPacket, Instruction, Command, FrcLinearRelative, FrcLinearMotion, FrcJointMotion, FrcInitialize, Configuration, Position};
use fanuc_rmi::{SpeedType, TermType};

/// Shared context for frame/tool data and program state
#[derive(Clone, Copy)]
pub struct WorkspaceContext {
    /// Active UFrame number
    pub active_frame: RwSignal<usize>,
    /// Active UTool number
    pub active_tool: RwSignal<usize>,
    /// Expanded frame in accordion (-1 = none)
    pub expanded_frame: RwSignal<i32>,
    /// Expanded tool in accordion (-1 = none)
    pub expanded_tool: RwSignal<i32>,
    /// Command log entries
    pub command_log: RwSignal<Vec<CommandLogEntry>>,
    /// Recent commands that can be re-run
    pub recent_commands: RwSignal<Vec<RecentCommand>>,
    /// Currently selected command ID in the dropdown (None = no selection)
    pub selected_command_id: RwSignal<Option<usize>>,
    /// Current program lines
    pub program_lines: RwSignal<Vec<ProgramLine>>,
    /// Currently executing line (-1 = none)
    pub executing_line: RwSignal<i32>,
    /// Show command composer modal
    pub show_composer: RwSignal<bool>,
    /// Loaded program name (for display in Dashboard)
    pub loaded_program_name: RwSignal<Option<String>>,
    /// Loaded program ID (for execution)
    pub loaded_program_id: RwSignal<Option<i64>>,
    /// Program is currently running
    pub program_running: RwSignal<bool>,
    /// Program is paused
    pub program_paused: RwSignal<bool>,
}

#[derive(Clone, Debug)]
pub struct CommandLogEntry {
    pub timestamp: String,
    pub command: String,
    pub status: CommandStatus,
}

/// A recently executed command that can be re-run
#[derive(Clone, Debug)]
pub struct RecentCommand {
    pub id: usize,
    pub name: String,
    pub command_type: String,
    pub description: String,
    // Motion parameters
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub term_type: String,
    pub uframe: u8,
    pub utool: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CommandStatus {
    Pending,
    Success,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct ProgramLine {
    pub line_number: usize,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub speed: f64,
    pub term_type: String,
}

impl WorkspaceContext {
    pub fn new() -> Self {
        Self {
            active_frame: RwSignal::new(0),
            active_tool: RwSignal::new(0),
            expanded_frame: RwSignal::new(0), // Default expand frame 0
            expanded_tool: RwSignal::new(0),  // Default expand tool 0
            command_log: RwSignal::new(Vec::new()),
            recent_commands: RwSignal::new(Vec::new()),
            selected_command_id: RwSignal::new(None),
            program_lines: RwSignal::new(Vec::new()),
            executing_line: RwSignal::new(-1),
            show_composer: RwSignal::new(false),
            loaded_program_name: RwSignal::new(None),
            loaded_program_id: RwSignal::new(None),
            program_running: RwSignal::new(false),
            program_paused: RwSignal::new(false),
        }
    }
}

/// Main workspace with routed content.
#[component]
pub fn MainWorkspace() -> impl IntoView {
    // Create and provide workspace context
    let workspace_ctx = WorkspaceContext::new();
    provide_context(workspace_ctx);

    view! {
        <main class="flex-1 flex flex-col overflow-hidden bg-[#080808]">
            <Routes fallback=|| view! { <DashboardView/> }>
                // Root redirects to dashboard
                <Route path=path!("/") view=|| view! { <Redirect path="/dashboard/control" /> } />

                // Dashboard with nested tabs
                <ParentRoute path=path!("/dashboard") view=DashboardView>
                    <Route path=path!("/") view=|| view! { <Redirect path="/dashboard/control" /> } />
                    <Route path=path!("/control") view=ControlTab />
                    <Route path=path!("/info") view=InfoTab />
                </ParentRoute>

                // Programs view
                <Route path=path!("/programs") view=ProgramsView />

                // Settings view
                <Route path=path!("/settings") view=SettingsView />
            </Routes>
        </main>
    }
}

/// Dashboard view with tabs - uses Outlet for nested routes.
#[component]
pub fn DashboardView() -> impl IntoView {
    let location = use_location();
    let is_control_active = move || {
        let path = location.pathname.get();
        path == "/dashboard/control" || path == "/dashboard" || path == "/"
    };
    let is_info_active = move || {
        location.pathname.get() == "/dashboard/info"
    };

    view! {
        <div class="h-full flex flex-col">
            // Tab bar - Control first, Info second (using router links)
            <div class="h-7 border-b border-[#ffffff08] flex items-center px-2 shrink-0 bg-[#0d0d0d]">
                <A
                    href="/dashboard/control"
                    attr:class=move || if is_control_active() {
                        "px-3 py-1 text-[10px] text-[#00d9ff] border-b border-[#00d9ff] font-medium -mb-px no-underline"
                    } else {
                        "px-3 py-1 text-[10px] text-[#555555] hover:text-[#888888] transition-colors no-underline"
                    }
                >
                    "Control"
                </A>
                <A
                    href="/dashboard/info"
                    attr:class=move || if is_info_active() {
                        "px-3 py-1 text-[10px] text-[#00d9ff] border-b border-[#00d9ff] font-medium -mb-px no-underline"
                    } else {
                        "px-3 py-1 text-[10px] text-[#555555] hover:text-[#888888] transition-colors no-underline"
                    }
                >
                    "Info"
                </A>
            </div>

            // Tab content via Outlet
            <div class="flex-1 overflow-auto p-2">
                <Outlet />
            </div>
        </div>
    }
}

/// Info tab content - Frame/Tool management
#[component]
pub fn InfoTab() -> impl IntoView {
    view! {
        <div class="h-full grid grid-cols-2 gap-2">
            // Left column - Frames
            <div class="flex flex-col gap-2 min-h-0 overflow-hidden">
                <FrameManagementPanel/>
                <MultiFrameDisplay/>
                <JointAnglesPanel/>
            </div>

            // Right column - Tools
            <div class="flex flex-col gap-2 min-h-0 overflow-hidden">
                <ToolManagementPanel/>
                <MultiToolDisplay/>
            </div>
        </div>
    }
}

/// Frame Management Panel
#[component]
fn FrameManagementPanel() -> impl IntoView {
    // For now, default to frame 0. Will be updated when we add FrcGetUFrameUTool support
    let (active_frame, _set_active_frame) = signal(0usize);

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

/// Tool Management Panel
#[component]
fn ToolManagementPanel() -> impl IntoView {
    // For now, default to tool 0. Will be updated when we add FrcGetUFrameUTool support
    let (active_tool, _set_active_tool) = signal(0usize);

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

/// Multi-Frame Display - Accordion showing detailed frame data
#[component]
fn MultiFrameDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let expanded = ctx.expanded_frame;
    let (expand_all, set_expand_all) = signal(false);

    // Mock frame data - will be populated via RMI commands
    let frame_data: Vec<(usize, f64, f64, f64, f64, f64, f64)> = (0..10)
        .map(|i| (i, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0))
        .collect();

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
                            // -2 indicates "show all"
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
                {frame_data.into_iter().map(|(i, x, y, z, w, p, r)| {
                    // -2 means expand all, otherwise check if this frame is expanded
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
                                    <div class="grid grid-cols-3 gap-x-2 gap-y-0.5 text-[9px]">
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"X"</span>
                                            <span class="text-white font-mono tabular-nums">{format!("{:.3}", x)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"Y"</span>
                                            <span class="text-white font-mono tabular-nums">{format!("{:.3}", y)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"Z"</span>
                                            <span class="text-white font-mono tabular-nums">{format!("{:.3}", z)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"W"</span>
                                            <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", w)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"P"</span>
                                            <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", p)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"R"</span>
                                            <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", r)}</span>
                                        </div>
                                    </div>
                                </div>
                            </Show>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Multi-Tool Display - Accordion showing detailed tool geometry
#[component]
fn MultiToolDisplay() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let expanded = ctx.expanded_tool;
    let (expand_all, set_expand_all) = signal(false);

    // Mock tool data - will be populated via RMI commands
    let tool_data: Vec<(usize, f64, f64, f64, f64, f64, f64)> = (0..10)
        .map(|i| (i, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0))
        .collect();

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
                {tool_data.into_iter().map(|(i, x, y, z, w, p, r)| {
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
                                    <div class="grid grid-cols-3 gap-x-2 gap-y-0.5 text-[9px]">
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"X"</span>
                                            <span class="text-white font-mono tabular-nums">{format!("{:.3}", x)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"Y"</span>
                                            <span class="text-white font-mono tabular-nums">{format!("{:.3}", y)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"Z"</span>
                                            <span class="text-white font-mono tabular-nums">{format!("{:.3}", z)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"W"</span>
                                            <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", w)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"P"</span>
                                            <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", p)}</span>
                                        </div>
                                        <div class="flex justify-between">
                                            <span class="text-[#666666]">"R"</span>
                                            <span class="text-[#cccccc] font-mono tabular-nums">{format!("{:.2}°", r)}</span>
                                        </div>
                                    </div>
                                </div>
                            </Show>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Joint Angles Panel - Shows J1-J6 angles (live from WebSocket)
#[component]
fn JointAnglesPanel() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let joint_angles = ws.joint_angles;

    let joint_names = ["J1", "J2", "J3", "J4", "J5", "J6"];

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2 shrink-0">
            <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 uppercase tracking-wide flex items-center">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                </svg>
                "Joint Angles"
            </h3>
            <div class="grid grid-cols-6 gap-1">
                {joint_names.into_iter().enumerate().map(|(i, name)| {
                    view! {
                        <div class="bg-[#111111] rounded px-1.5 py-1 text-center">
                            <div class="text-[8px] text-[#666666]">{name}</div>
                            <div class="text-[10px] text-white font-mono tabular-nums">
                                {move || {
                                    joint_angles.get()
                                        .map(|angles| format!("{:.1}°", angles[i]))
                                        .unwrap_or_else(|| "---".to_string())
                                }}
                            </div>
                        </div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Control tab content (command composer).
#[component]
pub fn ControlTab() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
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
    // This highlights the line currently being executed (sent to robot)
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
                                // Stop any running program first
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
        // Only request programs when WebSocket is connected
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
                                // Fetch the program details
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

/// Command Composer Modal - step-by-step command builder
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

                // Content
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
                            <div class="space-y-3">
                                // Position inputs
                                <div>
                                    <label class="block text-[10px] text-[#888888] mb-1">"Position (mm / deg)"</label>
                                    <div class="grid grid-cols-3 gap-1">
                                        {[("X", x, set_x), ("Y", y, set_y), ("Z", z, set_z)].into_iter().map(|(lbl, val, set_val)| {
                                            view! {
                                                <div>
                                                    <label class="block text-[8px] text-[#555555] mb-0.5">{lbl}</label>
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
                                        {[("W", w, set_w), ("P", p, set_p), ("R", r, set_r)].into_iter().map(|(lbl, val, set_val)| {
                                            view! {
                                                <div>
                                                    <label class="block text-[8px] text-[#555555] mb-0.5">{lbl}</label>
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

/// Program data structure
#[derive(Clone, Debug)]
pub struct SavedProgram {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub line_count: usize,
    pub default_speed: f64,
    pub default_uframe: u8,
    pub default_utool: u8,
    pub created_at: String,
}

/// Programs view (toolpath creation and editing).
#[component]
pub fn ProgramsView() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");
    let navigate = use_navigate();

    let (show_new_program, set_show_new_program) = signal(false);
    let (show_csv_upload, set_show_csv_upload) = signal(false);
    let (show_open_modal, set_show_open_modal) = signal(false);
    let (show_save_as_modal, set_show_save_as_modal) = signal(false);
    let (selected_program_id, set_selected_program_id) = signal::<Option<i64>>(None);

    // Menu dropdown states
    let (show_file_menu, set_show_file_menu) = signal(false);
    let (show_view_menu, set_show_view_menu) = signal(false);

    // Load programs on mount (wait for WebSocket connection)
    Effect::new(move |_| {
        // Only request programs when WebSocket is connected
        if ws.connected.get() {
            ws.list_programs();
        }
    });

    // Derive programs from WebSocket manager
    let programs = ws.programs;
    let current_program = ws.current_program;

    // When a program is selected, fetch its details
    Effect::new(move |_| {
        if let Some(id) = selected_program_id.get() {
            ws.get_program(id);
        }
    });

    view! {
        <div class="h-full flex flex-col">
            // Menu bar
            <div class="h-7 border-b border-[#ffffff08] flex items-center px-2 shrink-0 bg-[#0d0d0d]">
                // File menu
                <div class="relative">
                    <button
                        class={move || format!(
                            "px-2 py-1 text-[10px] rounded transition-colors {}",
                            if show_file_menu.get() { "bg-[#ffffff10] text-white" } else { "text-[#888888] hover:text-white hover:bg-[#ffffff08]" }
                        )}
                        on:click=move |_| {
                            set_show_file_menu.update(|v| *v = !*v);
                            set_show_view_menu.set(false);
                        }
                    >
                        "File"
                    </button>
                    {move || if show_file_menu.get() {
                        view! {
                            <div class="absolute left-0 top-full mt-0.5 w-40 bg-[#1a1a1a] border border-[#ffffff15] rounded shadow-lg z-50">
                                <button
                                    class="w-full text-left px-3 py-1.5 text-[10px] text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white flex items-center gap-2"
                                    on:click=move |_| {
                                        set_show_new_program.set(true);
                                        set_show_file_menu.set(false);
                                    }
                                >
                                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"/>
                                    </svg>
                                    "New Program"
                                </button>
                                <button
                                    class="w-full text-left px-3 py-1.5 text-[10px] text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white flex items-center gap-2"
                                    on:click=move |_| {
                                        set_show_open_modal.set(true);
                                        set_show_file_menu.set(false);
                                    }
                                >
                                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                                    </svg>
                                    "Open..."
                                </button>
                                <div class="border-t border-[#ffffff10] my-1"></div>
                                <button
                                    class={move || format!(
                                        "w-full text-left px-3 py-1.5 text-[10px] flex items-center gap-2 {}",
                                        if current_program.get().is_some() {
                                            "text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white"
                                        } else {
                                            "text-[#444444] cursor-not-allowed"
                                        }
                                    )}
                                    on:click=move |_| {
                                        if current_program.get().is_some() {
                                            set_show_save_as_modal.set(true);
                                            set_show_file_menu.set(false);
                                        }
                                    }
                                >
                                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"/>
                                    </svg>
                                    "Save As..."
                                </button>
                                <div class="border-t border-[#ffffff10] my-1"></div>
                                <button
                                    class={move || format!(
                                        "w-full text-left px-3 py-1.5 text-[10px] flex items-center gap-2 {}",
                                        if selected_program_id.get().is_some() {
                                            "text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white"
                                        } else {
                                            "text-[#444444] cursor-not-allowed"
                                        }
                                    )}
                                    on:click=move |_| {
                                        if selected_program_id.get().is_some() {
                                            set_show_csv_upload.set(true);
                                            set_show_file_menu.set(false);
                                        }
                                    }
                                >
                                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"/>
                                    </svg>
                                    "Upload CSV..."
                                </button>
                                <div class="border-t border-[#ffffff10] my-1"></div>
                                <button
                                    class={move || format!(
                                        "w-full text-left px-3 py-1.5 text-[10px] flex items-center gap-2 {}",
                                        if selected_program_id.get().is_some() || current_program.get().is_some() {
                                            "text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white"
                                        } else {
                                            "text-[#444444] cursor-not-allowed"
                                        }
                                    )}
                                    disabled=move || selected_program_id.get().is_none() && current_program.get().is_none()
                                    on:click=move |_| {
                                        // Close the current program (deselect it and clear from WebSocket)
                                        set_selected_program_id.set(None);
                                        ws.clear_current_program();
                                        set_show_file_menu.set(false);
                                    }
                                >
                                    <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                    </svg>
                                    "Close"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}
                </div>

                // View menu
                <div class="relative">
                    <button
                        class={move || format!(
                            "px-2 py-1 text-[10px] rounded transition-colors {}",
                            if show_view_menu.get() { "bg-[#ffffff10] text-white" } else { "text-[#888888] hover:text-white hover:bg-[#ffffff08]" }
                        )}
                        on:click=move |_| {
                            set_show_view_menu.update(|v| *v = !*v);
                            set_show_file_menu.set(false);
                        }
                    >
                        "View"
                    </button>
                    {move || if show_view_menu.get() {
                        view! {
                            <div class="absolute left-0 top-full mt-0.5 w-48 bg-[#1a1a1a] border border-[#ffffff15] rounded shadow-lg z-50">
                                <button
                                    class="w-full text-left px-3 py-1.5 text-[10px] text-[#aaaaaa] hover:bg-[#ffffff10] hover:text-white flex items-center justify-between"
                                    on:click=move |_| {
                                        layout_ctx.show_program_browser.update(|v| *v = !*v);
                                        set_show_view_menu.set(false);
                                    }
                                >
                                    <span class="flex items-center gap-2">
                                        <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h7"/>
                                        </svg>
                                        "Program Browser"
                                    </span>
                                    {move || if layout_ctx.show_program_browser.get() {
                                        view! { <span class="text-[#00d9ff]">"✓"</span> }.into_any()
                                    } else {
                                        view! { <span></span> }.into_any()
                                    }}
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }}
                </div>

                // Spacer
                <div class="flex-1"></div>

                // Current program indicator
                {move || current_program.get().map(|prog| view! {
                    <span class="text-[9px] text-[#666666]">
                        "Current: "
                        <span class="text-[#00d9ff]">{prog.name}</span>
                    </span>
                })}
            </div>

            // Main content area
            <div class="flex-1 p-2 flex gap-2 min-h-0">
                // Left: Program browser (conditionally shown)
                <Show when=move || layout_ctx.show_program_browser.get()>
                    <div class="w-64 bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden shrink-0">
                        <div class="flex items-center justify-between p-2 border-b border-[#ffffff08]">
                            <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                                </svg>
                                "Programs"
                            </h3>
                            <button
                                class="text-[#666666] hover:text-white"
                                on:click=move |_| layout_ctx.show_program_browser.set(false)
                                title="Close browser"
                            >
                                <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                </svg>
                            </button>
                        </div>
                        <div class="flex-1 overflow-y-auto p-1.5 space-y-1">
                            {move || {
                                let progs = programs.get();
                                if progs.is_empty() {
                                    Either::Left(view! {
                                        <div class="text-[#555555] text-[9px] text-center py-4">
                                            "No programs saved"
                                        </div>
                                    })
                                } else {
                                    Either::Right(progs.into_iter().map(|prog| {
                                        let is_selected = move || selected_program_id.get() == Some(prog.id);
                                        let prog_id = prog.id;
                                        let prog_name = prog.name.clone();
                                        let lines_str = format!("{} lines", prog.instruction_count);
                                        view! {
                                            <button
                                                class={move || format!(
                                                    "w-full text-left p-2 rounded border text-[9px] transition-colors {}",
                                                    if is_selected() {
                                                        "bg-[#00d9ff10] border-[#00d9ff40] text-white"
                                                    } else {
                                                        "bg-[#111111] border-[#ffffff08] text-[#888888] hover:border-[#ffffff20]"
                                                    }
                                                )}
                                                on:click=move |_| set_selected_program_id.set(Some(prog_id))
                                            >
                                                <div class="font-medium text-[10px] mb-0.5">{prog_name}</div>
                                                <div class="text-[#555555]">{lines_str}</div>
                                            </button>
                                        }
                                    }).collect_view())
                                }
                            }}
                        </div>
                    </div>
                </Show>

            // Right: Program details
            <div class="flex-1 bg-[#0a0a0a] rounded border border-[#ffffff08] flex flex-col overflow-hidden">
                {move || {
                    // Use current_program from WebSocket (fetched when selected)
                    if let Some(prog) = current_program.get() {
                        let prog_id = prog.id;
                        let prog_name = prog.name.clone();
                        let prog_name_for_load = prog.name.clone();
                        let prog_desc = prog.description.clone().unwrap_or_default();
                        let prog_created = "N/A".to_string(); // Not in ProgramDetail
                        let line_count = prog.instructions.len();
                        let speed_str = prog.instructions.first()
                            .and_then(|i| i.speed)
                            .map(|s| format!("{} mm/s", s))
                            .unwrap_or_else(|| "N/A".to_string());
                        // Get frame/tool from first instruction if available
                        let frame_tool_str = "N/A".to_string();

                        // Generate preview lines from actual instructions
                        let preview_lines: Vec<String> = prog.instructions.iter()
                            .take(5)
                            .map(|i| {
                                let term = i.term_type.clone().unwrap_or_else(|| "CNT100".to_string());
                                let spd = i.speed.unwrap_or(100.0);
                                format!("{:03}: L P[{}] {:.0}mm/sec {}", i.line_number, i.line_number, spd, term)
                            })
                            .collect();

                        // Convert instructions to ProgramLines for loading
                        let program_lines_for_load: Vec<ProgramLine> = prog.instructions.iter()
                            .map(|i| ProgramLine {
                                line_number: i.line_number as usize,
                                x: i.x,
                                y: i.y,
                                z: i.z,
                                w: i.w.unwrap_or(0.0),
                                p: i.p.unwrap_or(0.0),
                                r: i.r.unwrap_or(0.0),
                                speed: i.speed.unwrap_or(100.0),
                                term_type: i.term_type.clone().unwrap_or_else(|| "CNT100".to_string()),
                            })
                            .collect();

                        Either::Left(view! {
                            <div class="h-full flex flex-col">
                                // Header
                                <div class="p-3 border-b border-[#ffffff08]">
                                    <div class="flex items-start justify-between">
                                        <div>
                                            <h2 class="text-sm font-semibold text-white">{prog_name}</h2>
                                            <p class="text-[#666666] text-[9px] mt-0.5">{prog_desc}</p>
                                        </div>
                                        <div class="flex gap-1">
                                            <button
                                                class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[9px] px-2 py-1 rounded hover:bg-[#22c55e30]"
                                                on:click={
                                                    let lines = program_lines_for_load.clone();
                                                    let name = prog_name_for_load.clone();
                                                    let nav = navigate.clone();
                                                    move |_| {
                                                        // Load program into Dashboard
                                                        ctx.program_lines.set(lines.clone());
                                                        ctx.loaded_program_name.set(Some(name.clone()));
                                                        ctx.loaded_program_id.set(Some(prog_id));
                                                        ctx.executing_line.set(-1);
                                                        // Navigate to Dashboard
                                                        nav("/dashboard/control", Default::default());
                                                    }
                                                }
                                            >
                                                "▶ Load to Dashboard"
                                            </button>
                                            <button
                                                class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[9px] px-2 py-1 rounded hover:bg-[#00d9ff30]"
                                                on:click=move |_| set_show_csv_upload.set(true)
                                            >
                                                "⬆ Upload CSV"
                                            </button>
                                            <button
                                                class="bg-[#ff444420] border border-[#ff444440] text-[#ff4444] text-[9px] px-2 py-1 rounded hover:bg-[#ff444430]"
                                                on:click=move |_| {
                                                    ws.delete_program(prog_id);
                                                    set_selected_program_id.set(None);
                                                    // Refresh program list after delete
                                                    ws.list_programs();
                                                }
                                            >
                                                "Delete"
                                            </button>
                                        </div>
                                    </div>
                                </div>

                                // Metadata
                                <div class="p-3 border-b border-[#ffffff08] grid grid-cols-4 gap-3">
                                    <div>
                                        <div class="text-[8px] text-[#555555] uppercase">"Lines"</div>
                                        <div class="text-[11px] text-white font-mono">{line_count}</div>
                                    </div>
                                    <div>
                                        <div class="text-[8px] text-[#555555] uppercase">"Speed"</div>
                                        <div class="text-[11px] text-white font-mono">{speed_str}</div>
                                    </div>
                                    <div>
                                        <div class="text-[8px] text-[#555555] uppercase">"Frame/Tool"</div>
                                        <div class="text-[11px] text-white font-mono">{frame_tool_str}</div>
                                    </div>
                                    <div>
                                        <div class="text-[8px] text-[#555555] uppercase">"Created"</div>
                                        <div class="text-[11px] text-white font-mono">{prog_created}</div>
                                    </div>
                                </div>

                                // Preview area
                                <div class="flex-1 p-3 overflow-auto">
                                    <h4 class="text-[9px] text-[#666666] uppercase mb-2">"Program Preview"</h4>
                                    <div class="bg-[#111111] rounded border border-[#ffffff08] p-2 font-mono text-[9px] text-[#888888]">
                                        {if preview_lines.is_empty() {
                                            Either::Left(view! {
                                                <div class="text-[#555555]">"No instructions"</div>
                                            })
                                        } else {
                                            Either::Right(view! {
                                                <div class="text-[#555555]">"// First 5 lines preview"</div>
                                                {preview_lines.into_iter().map(|line| view! {
                                                    <div>{line}</div>
                                                }).collect_view()}
                                                {if line_count > 5 {
                                                    Either::Left(view! { <div class="text-[#555555]">"..."</div> })
                                                } else {
                                                    Either::Right(())
                                                }}
                                            })
                                        }}
                                    </div>
                                </div>
                            </div>
                        })
                    } else {
                        Either::Right(view! {
                            <div class="h-full flex items-center justify-center">
                                <div class="text-center">
                                    <svg class="w-12 h-12 mx-auto mb-2 text-[#333333]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
                                    </svg>
                                    <p class="text-[#555555] text-[10px] mb-3">"No program open"</p>
                                    <div class="flex gap-2 justify-center">
                                        <button
                                            class="bg-[#00d9ff20] border border-[#00d9ff40] text-[#00d9ff] text-[9px] px-3 py-1.5 rounded hover:bg-[#00d9ff30]"
                                            on:click=move |_| set_show_open_modal.set(true)
                                        >
                                            "Open Program"
                                        </button>
                                        <button
                                            class="bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] text-[9px] px-3 py-1.5 rounded hover:bg-[#22c55e30]"
                                            on:click=move |_| set_show_new_program.set(true)
                                        >
                                            "New Program"
                                        </button>
                                    </div>
                                </div>
                            </div>
                        })
                    }
                }}
            </div>
            </div> // Close flex-1 flex gap-2 wrapper

            // New Program Modal
            <Show when=move || show_new_program.get()>
                <NewProgramModal
                    on_close=move || set_show_new_program.set(false)
                    on_created=move |id| {
                        set_show_new_program.set(false);
                        set_selected_program_id.set(Some(id));
                        ws.list_programs();
                        ws.get_program(id);
                    }
                />
            </Show>

            // Open Program Modal
            <Show when=move || show_open_modal.get()>
                <OpenProgramModal
                    on_close=move || set_show_open_modal.set(false)
                    on_selected=move |id| {
                        set_show_open_modal.set(false);
                        set_selected_program_id.set(Some(id));
                        ws.get_program(id);
                    }
                />
            </Show>

            // Save As Modal
            <Show when=move || show_save_as_modal.get()>
                <SaveAsProgramModal
                    on_close=move || set_show_save_as_modal.set(false)
                    on_saved=move |id| {
                        set_show_save_as_modal.set(false);
                        set_selected_program_id.set(Some(id));
                        ws.list_programs();
                        ws.get_program(id);
                    }
                />
            </Show>

            // CSV Upload Modal (for selected program)
            <Show when=move || show_csv_upload.get() && selected_program_id.get().is_some()>
                {move || selected_program_id.get().map(|prog_id| view! {
                    <CSVUploadModal
                        program_id=prog_id
                        on_close=move || set_show_csv_upload.set(false)
                        on_uploaded=move || {
                            set_show_csv_upload.set(false);
                            ws.get_program(prog_id);
                            ws.list_programs();
                        }
                    />
                })}
            </Show>
        </div>
    }
}

/// New Program Modal - Simple modal to create a program with name and description
#[component]
fn NewProgramModal(
    on_close: impl Fn() + 'static + Clone + Send,
    on_created: impl Fn(i64) + 'static + Clone + Send,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    let (program_name, set_program_name) = signal("".to_string());
    let (description, set_description) = signal("".to_string());
    let (is_creating, set_is_creating) = signal(false);
    let (error_message, set_error_message) = signal::<Option<String>>(None);

    let on_close_clone = on_close.clone();
    let on_created_clone = on_created.clone();

    // Watch for new program in the programs list
    let programs = ws.programs;
    let api_error = ws.api_error;
    // Use get_untracked to avoid reactive context warning
    let initial_count = programs.get_untracked().len();

    // Watch for API errors during creation
    Effect::new(move |_| {
        if is_creating.get() {
            // Check for errors
            if let Some(err) = api_error.get() {
                set_is_creating.set(false);
                // Parse error message for user-friendly display
                let user_msg = if err.contains("UNIQUE constraint failed") {
                    "A program with this name already exists. Please choose a different name.".to_string()
                } else {
                    err
                };
                set_error_message.set(Some(user_msg));
                ws.clear_api_error();
                return;
            }

            // Check for successful creation
            let progs = programs.get();
            if progs.len() > initial_count {
                // Find the newest program (highest ID)
                if let Some(newest) = progs.iter().max_by_key(|p| p.id) {
                    set_is_creating.set(false);
                    on_created_clone(newest.id);
                }
            }
        }
    });

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[400px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6"/>
                        </svg>
                        "New Program"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content
                <div class="p-3 space-y-3">
                    // Error message display
                    <Show when=move || error_message.get().is_some()>
                        <div class="bg-[#ff444420] border border-[#ff444440] rounded p-2 flex items-start gap-2">
                            <svg class="w-4 h-4 text-[#ff4444] flex-shrink-0 mt-0.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            <span class="text-[10px] text-[#ff4444]">{move || error_message.get().unwrap_or_default()}</span>
                        </div>
                    </Show>
                    <div>
                        <label class="block text-[9px] text-[#888888] mb-1">"Program Name *"</label>
                        <input
                            type="text"
                            placeholder="e.g., Spiral Cylinder"
                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                            prop:value=move || program_name.get()
                            on:input=move |ev| {
                                set_program_name.set(event_target_value(&ev));
                                set_error_message.set(None); // Clear error when user types
                            }
                        />
                    </div>
                    <div>
                        <label class="block text-[9px] text-[#888888] mb-1">"Description"</label>
                        <textarea
                            placeholder="Optional description..."
                            rows="2"
                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none resize-none"
                            prop:value=move || description.get()
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                        ></textarea>
                    </div>
                    <p class="text-[8px] text-[#555555]">
                        "After creating the program, you can upload a CSV file with motion data."
                    </p>
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
                        on:click={
                            let on_close = on_close_clone.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class={move || format!(
                            "text-[10px] px-3 py-1.5 rounded {}",
                            if !program_name.get().is_empty() && !is_creating.get() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
                            }
                        )}
                        disabled=move || program_name.get().is_empty() || is_creating.get()
                        on:click=move |_| {
                            let name = program_name.get();
                            let desc = description.get();
                            let desc_opt = if desc.is_empty() { None } else { Some(desc) };

                            set_is_creating.set(true);
                            ws.create_program(name, desc_opt);
                            ws.list_programs();
                        }
                    >
                        {move || if is_creating.get() { "Creating..." } else { "Create Program" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Open Program Modal - Select a program to open
#[component]
fn OpenProgramModal(
    on_close: impl Fn() + 'static + Clone + Send,
    on_selected: impl Fn(i64) + 'static + Clone + Send,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let programs = ws.programs;

    // Refresh programs list when modal opens (wait for WebSocket connection)
    Effect::new(move |_| {
        // Only request programs when WebSocket is connected
        if ws.connected.get() {
            ws.list_programs();
        }
    });

    let on_close_clone = on_close.clone();

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[400px] max-h-[500px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"/>
                        </svg>
                        "Open Program"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Program list
                <div class="flex-1 overflow-y-auto p-2">
                    {move || {
                        let progs = programs.get();
                        if progs.is_empty() {
                            Either::Left(view! {
                                <div class="text-[#555555] text-[10px] text-center py-8">
                                    "No programs available. Create one first."
                                </div>
                            })
                        } else {
                            Either::Right(progs.into_iter().map(|prog| {
                                let prog_id = prog.id;
                                let prog_name = prog.name.clone();
                                let lines_str = format!("{} lines", prog.instruction_count);
                                let on_selected = on_selected.clone();
                                view! {
                                    <button
                                        class="w-full text-left p-3 rounded border border-[#ffffff08] bg-[#0a0a0a] hover:border-[#00d9ff40] hover:bg-[#00d9ff10] transition-colors mb-1"
                                        on:click=move |_| on_selected(prog_id)
                                    >
                                        <div class="font-medium text-[11px] text-white mb-0.5">{prog_name}</div>
                                        <div class="text-[9px] text-[#555555]">{lines_str}</div>
                                    </button>
                                }
                            }).collect_view())
                        }
                    }}
                </div>

                // Footer
                <div class="flex justify-end p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
                        on:click={
                            move |_| on_close_clone()
                        }
                    >
                        "Cancel"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Save As Modal - Save current program with a new name
#[component]
fn SaveAsProgramModal(
    on_close: impl Fn() + 'static + Clone + Send,
    on_saved: impl Fn(i64) + 'static + Clone + Send,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let current_program = ws.current_program;

    let (new_name, set_new_name) = signal("".to_string());
    let (new_description, set_new_description) = signal("".to_string());
    let (is_saving, set_is_saving) = signal(false);

    let on_close_clone = on_close.clone();
    let on_saved_clone = on_saved.clone();

    // Initialize with current program name + " (copy)"
    Effect::new(move |_| {
        if let Some(prog) = current_program.get() {
            set_new_name.set(format!("{} (copy)", prog.name));
            set_new_description.set(prog.description.unwrap_or_default());
        }
    });

    // Watch for new program creation
    let programs = ws.programs;
    let initial_count = programs.get().len();

    Effect::new(move |_| {
        if is_saving.get() {
            let progs = programs.get();
            if progs.len() > initial_count {
                if let Some(newest) = progs.iter().max_by_key(|p| p.id) {
                    set_is_saving.set(false);
                    on_saved_clone(newest.id);
                }
            }
        }
    });

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[400px] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"/>
                        </svg>
                        "Save As"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content
                <div class="p-3 space-y-3">
                    <div>
                        <label class="block text-[9px] text-[#888888] mb-1">"New Program Name *"</label>
                        <input
                            type="text"
                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                            prop:value=move || new_name.get()
                            on:input=move |ev| set_new_name.set(event_target_value(&ev))
                        />
                    </div>
                    <div>
                        <label class="block text-[9px] text-[#888888] mb-1">"Description"</label>
                        <textarea
                            rows="2"
                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none resize-none"
                            prop:value=move || new_description.get()
                            on:input=move |ev| set_new_description.set(event_target_value(&ev))
                        ></textarea>
                    </div>
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
                        on:click={
                            let on_close = on_close_clone.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class={move || format!(
                            "text-[10px] px-3 py-1.5 rounded {}",
                            if !new_name.get().is_empty() && !is_saving.get() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
                            }
                        )}
                        disabled=move || new_name.get().is_empty() || is_saving.get()
                        on:click=move |_| {
                            let name = new_name.get();
                            let desc = new_description.get();
                            let desc_opt = if desc.is_empty() { None } else { Some(desc) };

                            // Create new program with the new name
                            set_is_saving.set(true);
                            ws.create_program(name, desc_opt);
                            ws.list_programs();
                            // Note: We'd need to copy instructions too, but for now just create empty
                        }
                    >
                        {move || if is_saving.get() { "Saving..." } else { "Save As" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// CSV Upload Modal - Upload CSV to an existing program
#[component]
fn CSVUploadModal(
    program_id: i64,
    on_close: impl Fn() + 'static + Clone + Send,
    on_uploaded: impl Fn() + 'static + Clone + Send,
) -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    let (file_name, set_file_name) = signal::<Option<String>>(None);
    let (csv_content, set_csv_content) = signal::<Option<String>>(None);
    let (preview_lines, set_preview_lines) = signal::<Vec<String>>(vec![]);
    let (parse_error, set_parse_error) = signal::<Option<String>>(None);
    let (is_uploading, set_is_uploading) = signal(false);
    let (line_count, set_line_count) = signal(0usize);

    let on_close_clone = on_close.clone();

    view! {
        <div class="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div class="bg-[#111111] border border-[#ffffff10] rounded-lg w-[500px] max-h-[80vh] flex flex-col">
                // Header
                <div class="flex items-center justify-between p-3 border-b border-[#ffffff08]">
                    <h2 class="text-sm font-semibold text-white flex items-center">
                        <svg class="w-4 h-4 mr-2 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
                        </svg>
                        "Upload CSV Data"
                    </h2>
                    <button
                        class="text-[#666666] hover:text-white"
                        on:click={
                            let on_close = on_close.clone();
                            move |_| on_close()
                        }
                    >
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content
                <div class="flex-1 overflow-y-auto p-3 space-y-3">
                    // File upload area
                    <label class="block border-2 border-dashed border-[#ffffff20] rounded-lg p-4 text-center hover:border-[#00d9ff40] transition-colors cursor-pointer">
                        <svg class="w-6 h-6 mx-auto mb-2 text-[#555555]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
                        </svg>
                        <p class="text-[10px] text-[#888888] mb-1">"Drop CSV file here or click to browse"</p>
                        <p class="text-[8px] text-[#555555]">"Required: x, y, z | Optional: w, p, r, speed, term_type"</p>
                        <input
                            type="file"
                            accept=".csv"
                            class="hidden"
                            on:change=move |ev| {
                                let target = ev.target().unwrap();
                                let input: HtmlInputElement = target.dyn_into().unwrap();
                                if let Some(files) = input.files() {
                                    if let Some(file) = files.get(0) {
                                        let name = file.name();
                                        set_file_name.set(Some(name.clone()));

                                        let reader = FileReader::new().unwrap();
                                        let reader_clone = reader.clone();
                                        let onload = Closure::wrap(Box::new(move || {
                                            if let Ok(result) = reader_clone.result() {
                                                if let Some(text) = result.as_string() {
                                                    let all_lines: Vec<&str> = text.lines().collect();
                                                    let data_lines = all_lines.len().saturating_sub(1);
                                                    set_line_count.set(data_lines);

                                                    let preview: Vec<String> = text.lines()
                                                        .skip(1)
                                                        .take(8)
                                                        .map(|s| s.to_string())
                                                        .collect();
                                                    set_preview_lines.set(preview);
                                                    set_csv_content.set(Some(text));
                                                    set_parse_error.set(None);
                                                }
                                            }
                                        }) as Box<dyn Fn()>);
                                        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                                        onload.forget();
                                        let _ = reader.read_as_text(&file);
                                    }
                                }
                            }
                        />
                    </label>

                    // File selected indicator
                    {move || file_name.get().map(|name| view! {
                        <div class="bg-[#0a0a0a] rounded p-2 flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <svg class="w-4 h-4 text-[#22c55e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                                </svg>
                                <span class="text-[10px] text-white">{name}</span>
                            </div>
                            <span class="text-[9px] text-[#666666]">{move || format!("{} lines", line_count.get())}</span>
                        </div>
                    })}

                    // Preview table
                    <Show when=move || !preview_lines.get().is_empty()>
                        <div>
                            <label class="block text-[9px] text-[#888888] mb-1">"Preview (first 8 lines)"</label>
                            <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] overflow-hidden max-h-40 overflow-y-auto">
                                <table class="w-full text-[9px]">
                                    <thead class="sticky top-0 bg-[#0d0d0d]">
                                        <tr class="text-[#666666] border-b border-[#ffffff08]">
                                            <th class="text-left px-2 py-1">"#"</th>
                                            <th class="text-right px-2 py-1">"X"</th>
                                            <th class="text-right px-2 py-1">"Y"</th>
                                            <th class="text-right px-2 py-1">"Z"</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        {move || preview_lines.get().into_iter().enumerate().map(|(i, line)| {
                                            let parts: Vec<String> = line.split(',').map(|s| s.trim().to_string()).collect();
                                            let x = parts.first().cloned().unwrap_or_else(|| "-".to_string());
                                            let y = parts.get(1).cloned().unwrap_or_else(|| "-".to_string());
                                            let z = parts.get(2).cloned().unwrap_or_else(|| "-".to_string());
                                            view! {
                                                <tr class="border-b border-[#ffffff05] text-[#cccccc]">
                                                    <td class="px-2 py-0.5 text-[#555555]">{i + 1}</td>
                                                    <td class="px-2 py-0.5 text-right font-mono">{x}</td>
                                                    <td class="px-2 py-0.5 text-right font-mono">{y}</td>
                                                    <td class="px-2 py-0.5 text-right font-mono">{z}</td>
                                                </tr>
                                            }
                                        }).collect_view()}
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </Show>

                    // Error display
                    {move || parse_error.get().map(|err| view! {
                        <div class="bg-[#ff444410] border border-[#ff444420] rounded p-2 text-[9px] text-[#ff4444]">
                            {err}
                        </div>
                    })}
                </div>

                // Footer
                <div class="flex justify-end gap-2 p-3 border-t border-[#ffffff08]">
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[10px] px-3 py-1.5 rounded"
                        on:click={
                            let on_close = on_close_clone.clone();
                            move |_| on_close()
                        }
                    >
                        "Cancel"
                    </button>
                    <button
                        class={move || format!(
                            "text-[10px] px-3 py-1.5 rounded {}",
                            if csv_content.get().is_some() && !is_uploading.get() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555] cursor-not-allowed"
                            }
                        )}
                        disabled=move || csv_content.get().is_none() || is_uploading.get()
                        on:click={
                            let on_uploaded = on_uploaded.clone();
                            move |_| {
                                if let Some(content) = csv_content.get() {
                                    set_is_uploading.set(true);
                                    ws.upload_csv(program_id, content, None);
                                    // Give the upload time to process, then close
                                    // In a real app, we'd wait for a response
                                    on_uploaded();
                                }
                            }
                        }
                    >
                        {move || if is_uploading.get() { "Uploading..." } else { "Upload CSV" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Settings view.
#[component]
pub fn SettingsView() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");

    let (settings_changed, set_settings_changed) = signal(false);
    let (save_status, set_save_status) = signal::<Option<String>>(None);

    // Connection settings (local only, not persisted to database)
    let (robot_ip, set_robot_ip) = signal("127.0.0.1".to_string());
    let (web_server_ip, set_web_server_ip) = signal("127.0.0.1".to_string());
    let (ws_port, set_ws_port) = signal("9000".to_string());
    let (rmi_port, set_rmi_port) = signal("16001".to_string());

    // Robot defaults (synced with database)
    let (default_speed, set_default_speed) = signal(50.0f64);
    let (default_term, set_default_term) = signal("CNT".to_string());
    let (default_uframe, set_default_uframe) = signal(0i32);
    let (default_utool, set_default_utool) = signal(0i32);

    // Default rotation (W, P, R)
    let (default_w, set_default_w) = signal(0.0f64);
    let (default_p, set_default_p) = signal(0.0f64);
    let (default_r, set_default_r) = signal(0.0f64);

    // Display preferences (local only)
    let (show_mm, set_show_mm) = signal(true);
    let (show_degrees, set_show_degrees) = signal(true);
    let (compact_mode, set_compact_mode) = signal(false);

    // Saved robot connections management
    let (show_add_connection, set_show_add_connection) = signal(false);
    let (editing_connection_id, set_editing_connection_id) = signal::<Option<i64>>(None);
    let (new_conn_name, set_new_conn_name) = signal(String::new());
    let (new_conn_desc, set_new_conn_desc) = signal(String::new());
    let (new_conn_ip, set_new_conn_ip) = signal("127.0.0.1".to_string());
    let (new_conn_port, set_new_conn_port) = signal("16001".to_string());

    let saved_connections = ws.robot_connections;

    // Load settings on mount
    Effect::new(move |_| {
        ws.get_settings();
        ws.list_robot_connections();
    });

    // Update local state when settings are received from server
    Effect::new(move |_| {
        if let Some(settings) = ws.settings.get() {
            set_default_speed.set(settings.default_speed);
            set_default_term.set(settings.default_term_type.clone());
            set_default_uframe.set(settings.default_uframe);
            set_default_utool.set(settings.default_utool);
            set_default_w.set(settings.default_w);
            set_default_p.set(settings.default_p);
            set_default_r.set(settings.default_r);
            set_settings_changed.set(false);
        }
    });

    view! {
        <div class="h-full p-2 flex flex-col gap-2">
            // Header with save button
            <div class="flex items-center justify-between bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
                <h2 class="text-xs font-semibold text-white flex items-center">
                    <svg class="w-3.5 h-3.5 mr-1.5 text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                    </svg>
                    "Settings"
                </h2>
                <div class="flex items-center gap-2">
                    {move || save_status.get().map(|s| view! {
                        <span class="text-[9px] text-[#22c55e]">{s}</span>
                    })}
                    <button
                        class={move || format!(
                            "text-[9px] px-3 py-1 rounded transition-colors {}",
                            if settings_changed.get() {
                                "bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] hover:bg-[#22c55e30]"
                            } else {
                                "bg-[#111111] border border-[#ffffff08] text-[#555555]"
                            }
                        )}
                        disabled=move || !settings_changed.get()
                        on:click=move |_| {
                            // Save settings via WebSocket
                            // Signature: (default_w, default_p, default_r, default_speed, default_term_type, default_uframe, default_utool)
                            ws.update_settings(
                                default_w.get(),
                                default_p.get(),
                                default_r.get(),
                                default_speed.get(),
                                default_term.get(),
                                default_uframe.get(),
                                default_utool.get(),
                            );
                            set_save_status.set(Some("✓ Settings saved".to_string()));
                            set_settings_changed.set(false);
                        }
                    >
                        "Save Settings"
                    </button>
                    <button
                        class="bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] hover:text-white text-[9px] px-3 py-1 rounded"
                        on:click=move |_| {
                            // Reset to defaults and save
                            set_robot_ip.set("127.0.0.1".to_string());
                            set_web_server_ip.set("127.0.0.1".to_string());
                            set_ws_port.set("9000".to_string());
                            set_rmi_port.set("16001".to_string());
                            set_default_speed.set(50.0);
                            set_default_term.set("CNT".to_string());
                            set_default_uframe.set(0);
                            set_default_utool.set(0);
                            set_default_w.set(0.0);
                            set_default_p.set(0.0);
                            set_default_r.set(0.0);
                            // Save defaults to database (w, p, r, speed, term, uframe, utool)
                            ws.update_settings(0.0, 0.0, 0.0, 50.0, "CNT".to_string(), 0, 0);
                            set_settings_changed.set(false);
                            set_save_status.set(Some("✓ Reset to defaults".to_string()));
                        }
                    >
                        "Reset to Defaults"
                    </button>
                </div>
            </div>

            // Settings grid
            <div class="flex-1 overflow-auto">
                <div class="grid grid-cols-2 gap-2">
                    // Connection settings
                    <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3">
                        <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                            <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0"/>
                            </svg>
                            "Connection"
                        </h3>
                        <div class="space-y-2">
                            <div class="grid grid-cols-2 gap-2">
                                <div>
                                    <label class="block text-[#666666] text-[9px] mb-0.5">"Robot IP Address"</label>
                                    <input
                                        type="text"
                                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                        prop:value=move || robot_ip.get()
                                        on:input=move |ev| {
                                            set_robot_ip.set(event_target_value(&ev));
                                            set_settings_changed.set(true);
                                        }
                                    />
                                    <p class="text-[8px] text-[#555555] mt-0.5">"Robot controller IP"</p>
                                </div>
                                <div>
                                    <label class="block text-[#666666] text-[9px] mb-0.5">"Web Server IP"</label>
                                    <input
                                        type="text"
                                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                        prop:value=move || web_server_ip.get()
                                        on:input=move |ev| {
                                            set_web_server_ip.set(event_target_value(&ev));
                                            set_settings_changed.set(true);
                                        }
                                    />
                                    <p class="text-[8px] text-[#555555] mt-0.5">"WebSocket server IP"</p>
                                </div>
                            </div>
                            <div class="grid grid-cols-2 gap-2">
                                <div>
                                    <label class="block text-[#666666] text-[9px] mb-0.5">"WebSocket Port"</label>
                                    <input
                                        type="text"
                                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                        prop:value=move || ws_port.get()
                                        on:input=move |ev| {
                                            set_ws_port.set(event_target_value(&ev));
                                            set_settings_changed.set(true);
                                        }
                                    />
                                </div>
                                <div>
                                    <label class="block text-[#666666] text-[9px] mb-0.5">"RMI Port"</label>
                                    <input
                                        type="text"
                                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                        prop:value=move || rmi_port.get()
                                        on:input=move |ev| {
                                            set_rmi_port.set(event_target_value(&ev));
                                            set_settings_changed.set(true);
                                        }
                                    />
                                </div>
                            </div>
                        </div>
                    </div>

                    // Saved Robot Connections
                    <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3">
                        <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center justify-between">
                            <span class="flex items-center">
                                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01"/>
                                </svg>
                                "Saved Connections"
                            </span>
                            <button
                                class="text-[8px] px-2 py-0.5 bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] rounded hover:bg-[#22c55e30]"
                                on:click=move |_| {
                                    set_show_add_connection.set(true);
                                    set_editing_connection_id.set(None);
                                    set_new_conn_name.set(String::new());
                                    set_new_conn_desc.set(String::new());
                                    set_new_conn_ip.set("127.0.0.1".to_string());
                                    set_new_conn_port.set("16001".to_string());
                                }
                            >
                                "+ Add"
                            </button>
                        </h3>

                        // Add/Edit form
                        <Show when=move || show_add_connection.get()>
                            <div class="mb-2 p-2 bg-[#111111] rounded border border-[#ffffff10]">
                                <div class="grid grid-cols-2 gap-2 mb-2">
                                    <div>
                                        <label class="block text-[#666666] text-[8px] mb-0.5">"Name"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            placeholder="My Robot"
                                            prop:value=move || new_conn_name.get()
                                            on:input=move |ev| set_new_conn_name.set(event_target_value(&ev))
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[8px] mb-0.5">"Description"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none"
                                            placeholder="Optional"
                                            prop:value=move || new_conn_desc.get()
                                            on:input=move |ev| set_new_conn_desc.set(event_target_value(&ev))
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[8px] mb-0.5">"IP Address"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || new_conn_ip.get()
                                            on:input=move |ev| set_new_conn_ip.set(event_target_value(&ev))
                                        />
                                    </div>
                                    <div>
                                        <label class="block text-[#666666] text-[8px] mb-0.5">"Port"</label>
                                        <input
                                            type="text"
                                            class="w-full bg-[#0a0a0a] border border-[#ffffff08] rounded px-2 py-1 text-[9px] text-white focus:border-[#00d9ff] focus:outline-none font-mono"
                                            prop:value=move || new_conn_port.get()
                                            on:input=move |ev| set_new_conn_port.set(event_target_value(&ev))
                                        />
                                    </div>
                                </div>
                                <div class="flex gap-1">
                                    <button
                                        class="flex-1 text-[8px] px-2 py-1 bg-[#22c55e20] border border-[#22c55e40] text-[#22c55e] rounded hover:bg-[#22c55e30]"
                                        on:click=move |_| {
                                            let name = new_conn_name.get();
                                            let desc = new_conn_desc.get();
                                            let ip = new_conn_ip.get();
                                            let port: u32 = new_conn_port.get().parse().unwrap_or(16001);
                                            let description = if desc.is_empty() { None } else { Some(desc) };

                                            if let Some(id) = editing_connection_id.get() {
                                                ws.update_robot_connection(id, name, description, ip, port);
                                            } else {
                                                ws.create_robot_connection(name, description, ip, port);
                                            }
                                            set_show_add_connection.set(false);
                                            // Refresh list after a short delay
                                            ws.list_robot_connections();
                                        }
                                    >
                                        {move || if editing_connection_id.get().is_some() { "Update" } else { "Save" }}
                                    </button>
                                    <button
                                        class="text-[8px] px-2 py-1 bg-[#1a1a1a] border border-[#ffffff08] text-[#888888] rounded hover:text-white"
                                        on:click=move |_| set_show_add_connection.set(false)
                                    >
                                        "Cancel"
                                    </button>
                                </div>
                            </div>
                        </Show>

                        // Connections list
                        <div class="space-y-1 max-h-32 overflow-y-auto">
                            <For
                                each=move || saved_connections.get()
                                key=|conn| conn.id
                                children=move |conn| {
                                    let conn_id = conn.id;
                                    let conn_name = conn.name.clone();
                                    let conn_desc = conn.description.clone();
                                    let conn_ip = conn.ip_address.clone();
                                    let conn_port = conn.port;
                                    view! {
                                        <div class="flex items-center justify-between p-1.5 bg-[#111111] rounded border border-[#ffffff08] hover:border-[#ffffff15]">
                                            <div class="flex-1 min-w-0">
                                                <div class="text-[9px] text-white font-medium truncate">{conn_name.clone()}</div>
                                                <div class="text-[8px] text-[#666666] font-mono">{format!("{}:{}", conn_ip.clone(), conn_port)}</div>
                                            </div>
                                            <div class="flex gap-1 ml-2">
                                                <button
                                                    class="text-[8px] px-1.5 py-0.5 text-[#00d9ff] hover:bg-[#00d9ff10] rounded"
                                                    title="Edit"
                                                    on:click=move |_| {
                                                        set_editing_connection_id.set(Some(conn_id));
                                                        set_new_conn_name.set(conn_name.clone());
                                                        set_new_conn_desc.set(conn_desc.clone().unwrap_or_default());
                                                        set_new_conn_ip.set(conn_ip.clone());
                                                        set_new_conn_port.set(conn_port.to_string());
                                                        set_show_add_connection.set(true);
                                                    }
                                                >
                                                    "Edit"
                                                </button>
                                                <button
                                                    class="text-[8px] px-1.5 py-0.5 text-[#ff4444] hover:bg-[#ff444410] rounded"
                                                    title="Delete"
                                                    on:click=move |_| {
                                                        ws.delete_robot_connection(conn_id);
                                                        ws.list_robot_connections();
                                                    }
                                                >
                                                    "×"
                                                </button>
                                            </div>
                                        </div>
                                    }
                                }
                            />
                            {move || if saved_connections.get().is_empty() {
                                view! {
                                    <div class="text-[8px] text-[#555555] text-center py-2">"No saved connections"</div>
                                }.into_any()
                            } else {
                                view! { <div></div> }.into_any()
                            }}
                        </div>
                    </div>

                    // Robot defaults
                    <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3">
                        <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                            <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                            </svg>
                            "Motion Defaults"
                        </h3>
                        <div class="grid grid-cols-2 gap-2">
                            <div>
                                <label class="block text-[#666666] text-[9px] mb-0.5">"Speed (mm/s)"</label>
                                <input
                                    type="number"
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                    prop:value=move || default_speed.get()
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse() {
                                            set_default_speed.set(v);
                                            set_settings_changed.set(true);
                                        }
                                    }
                                />
                            </div>
                            <div>
                                <label class="block text-[#666666] text-[9px] mb-0.5">"Termination"</label>
                                <select
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                    on:change=move |ev| {
                                        set_default_term.set(event_target_value(&ev));
                                        set_settings_changed.set(true);
                                    }
                                >
                                    <option value="CNT" selected=move || default_term.get() == "CNT">"CNT (Continuous)"</option>
                                    <option value="FINE" selected=move || default_term.get() == "FINE">"FINE (Stop)"</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-[#666666] text-[9px] mb-0.5">"UFrame"</label>
                                <input
                                    type="number"
                                    min="0" max="9"
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                    prop:value=move || default_uframe.get()
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse() {
                                            set_default_uframe.set(v);
                                            set_settings_changed.set(true);
                                        }
                                    }
                                />
                            </div>
                            <div>
                                <label class="block text-[#666666] text-[9px] mb-0.5">"UTool"</label>
                                <input
                                    type="number"
                                    min="0" max="9"
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                    prop:value=move || default_utool.get()
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse() {
                                            set_default_utool.set(v);
                                            set_settings_changed.set(true);
                                        }
                                    }
                                />
                            </div>
                        </div>
                    </div>

                    // Default rotation
                    <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3">
                        <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                            <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
                            </svg>
                            "Default Rotation"
                        </h3>
                        <p class="text-[8px] text-[#555555] mb-2">"Used when rotation not specified in CSV"</p>
                        <div class="grid grid-cols-3 gap-2">
                            <div>
                                <label class="block text-[#666666] text-[9px] mb-0.5">"W (deg)"</label>
                                <input
                                    type="number"
                                    step="0.1"
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                    prop:value=move || default_w.get()
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse() {
                                            set_default_w.set(v);
                                            set_settings_changed.set(true);
                                        }
                                    }
                                />
                            </div>
                            <div>
                                <label class="block text-[#666666] text-[9px] mb-0.5">"P (deg)"</label>
                                <input
                                    type="number"
                                    step="0.1"
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                    prop:value=move || default_p.get()
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse() {
                                            set_default_p.set(v);
                                            set_settings_changed.set(true);
                                        }
                                    }
                                />
                            </div>
                            <div>
                                <label class="block text-[#666666] text-[9px] mb-0.5">"R (deg)"</label>
                                <input
                                    type="number"
                                    step="0.1"
                                    class="w-full bg-[#111111] border border-[#ffffff08] rounded px-2 py-1.5 text-[10px] text-white focus:border-[#00d9ff] focus:outline-none"
                                    prop:value=move || default_r.get()
                                    on:input=move |ev| {
                                        if let Ok(v) = event_target_value(&ev).parse() {
                                            set_default_r.set(v);
                                            set_settings_changed.set(true);
                                        }
                                    }
                                />
                            </div>
                        </div>
                    </div>

                    // Display preferences
                    <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3">
                        <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                            <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"/>
                            </svg>
                            "Display Preferences"
                        </h3>
                        <div class="space-y-2">
                            <label class="flex items-center gap-2 text-[9px] text-[#888888] cursor-pointer">
                                <input
                                    type="checkbox"
                                    class="accent-[#00d9ff]"
                                    prop:checked=move || show_mm.get()
                                    on:change=move |ev| {
                                        set_show_mm.set(event_target_checked(&ev));
                                        set_settings_changed.set(true);
                                    }
                                />
                                "Show position in millimeters"
                            </label>
                            <label class="flex items-center gap-2 text-[9px] text-[#888888] cursor-pointer">
                                <input
                                    type="checkbox"
                                    class="accent-[#00d9ff]"
                                    prop:checked=move || show_degrees.get()
                                    on:change=move |ev| {
                                        set_show_degrees.set(event_target_checked(&ev));
                                        set_settings_changed.set(true);
                                    }
                                />
                                "Show angles in degrees"
                            </label>
                            <label class="flex items-center gap-2 text-[9px] text-[#888888] cursor-pointer">
                                <input
                                    type="checkbox"
                                    class="accent-[#00d9ff]"
                                    prop:checked=move || compact_mode.get()
                                    on:change=move |ev| {
                                        set_compact_mode.set(event_target_checked(&ev));
                                        set_settings_changed.set(true);
                                    }
                                />
                                "Compact mode (smaller text and spacing)"
                            </label>
                        </div>
                    </div>

                    // About / Info panel
                    <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-3 col-span-2">
                        <h3 class="text-[10px] font-semibold text-[#00d9ff] mb-2 uppercase tracking-wide flex items-center">
                            <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                            </svg>
                            "About"
                        </h3>
                        <div class="grid grid-cols-3 gap-4 text-[9px]">
                            <div>
                                <span class="text-[#555555]">"Version:"</span>
                                <span class="text-white ml-1">"0.1.0"</span>
                            </div>
                            <div>
                                <span class="text-[#555555]">"RMI Protocol:"</span>
                                <span class="text-white ml-1">"v5+"</span>
                            </div>
                            <div>
                                <span class="text-[#555555]">"Database:"</span>
                                <span class="text-white ml-1">"SQLite"</span>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

fn event_target_checked(ev: &leptos::ev::Event) -> bool {
    use wasm_bindgen::JsCast;
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|e| e.checked())
        .unwrap_or(false)
}

