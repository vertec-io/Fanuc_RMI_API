//! Dashboard Info tab - Frame and tool management display.
//!
//! Contains components for viewing user frames, user tools, and joint angles.

use leptos::prelude::*;
use crate::components::layout::workspace::context::WorkspaceContext;
use crate::websocket::WebSocketManager;

/// Info tab showing frame, tool, and joint data.
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

/// Frame Management Panel - Quick frame selector
#[component]
fn FrameManagementPanel() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let active_frame = ctx.active_frame;

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
                            on:click=move |_| active_frame.set(i)
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
#[component]
fn ToolManagementPanel() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
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
                            on:click=move |_| active_tool.set(i)
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

