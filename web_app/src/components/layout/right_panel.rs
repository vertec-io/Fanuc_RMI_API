//! Right panel component - always visible in Dashboard route.

use leptos::prelude::*;
use leptos::html::Div;
use leptos_use::{use_draggable_with_options, UseDraggableOptions, UseDraggableReturn, core::Position};
use leptos_router::hooks::use_location;
use super::LayoutContext;
use crate::components::{PositionDisplay, ErrorLog, JogControls, RobotStatus};

/// Right sidebar panel with position, errors, I/O, and jog controls.
#[component]
pub fn RightPanel() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");
    let jog_popped = layout_ctx.jog_popped;

    view! {
        <aside class="w-56 bg-[#0d0d0d] border-l border-[#ffffff08] flex flex-col overflow-hidden shrink-0">
            <div class="flex-1 overflow-y-auto p-1.5 space-y-1.5">
                // Robot status (compact)
                <RobotStatus/>

                // Position display
                <PositionDisplay/>

                // Errors panel
                <ErrorLog/>

                // I/O Status (placeholder)
                <IOStatusPanel/>

                // Jog controls (when not popped)
                <Show when=move || !jog_popped.get()>
                    <JogControlsPanel/>
                </Show>
            </div>
        </aside>
    }
}

/// I/O Status placeholder panel.
#[component]
fn IOStatusPanel() -> impl IntoView {
    let (collapsed, set_collapsed) = signal(true);

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08]">
            <button
                class="w-full flex items-center justify-between p-2 hover:bg-[#ffffff05] transition-colors"
                on:click=move |_| set_collapsed.update(|v| *v = !*v)
            >
                <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
                    </svg>
                    "I/O"
                </h3>
                <svg
                    class={move || format!("w-3 h-3 text-[#666666] transition-transform {}", if collapsed.get() { "-rotate-90" } else { "" })}
                    fill="none" stroke="currentColor" viewBox="0 0 24 24"
                >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                </svg>
            </button>
            <Show when=move || !collapsed.get()>
                <div class="px-2 pb-2">
                    <p class="text-[#555555] text-[9px]">
                        "Coming soon"
                    </p>
                </div>
            </Show>
        </div>
    }
}

/// Jog controls panel with pop-out button.
#[component]
fn JogControlsPanel() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");
    let jog_popped = layout_ctx.jog_popped;

    view! {
        <div class="relative">
            // Pop-out button
            <button
                class="absolute top-1.5 right-1.5 p-0.5 hover:bg-[#ffffff10] rounded z-10"
                title="Pop out jog controls"
                on:click=move |_| jog_popped.set(true)
            >
                <svg class="w-3 h-3 text-[#555555] hover:text-[#00d9ff]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/>
                </svg>
            </button>

            <JogControls/>
        </div>
    }
}

/// Floating jog controls (when popped out).
#[component]
pub fn FloatingJogControls() -> impl IntoView {
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");
    let jog_popped = layout_ctx.jog_popped;
    let location = use_location();

    // Only show if popped AND in Dashboard route
    let is_dashboard = move || {
        let path = location.pathname.get();
        path == "/" || path.starts_with("/dashboard")
    };
    let should_show = move || jog_popped.get() && is_dashboard();

    // Create node ref for the HEADER (drag handle only)
    let header_el = NodeRef::<Div>::new();

    // Use leptos-use draggable hook on the header only
    let UseDraggableReturn {
        style,
        ..
    } = use_draggable_with_options(
        header_el,
        UseDraggableOptions::default()
            .initial_value(Position { x: 100.0, y: 100.0 })
            .prevent_default(true),
    );

    view! {
        <Show when=should_show>
            <div
                class="fixed bg-[#111111] rounded border border-[#00d9ff40] shadow-2xl z-50 select-none"
                style=move || format!("touch-action: none; min-width: 200px; {}", style.get())
            >
                // Header with close button - THIS is the drag handle
                <div
                    node_ref=header_el
                    class="flex items-center justify-between p-2 cursor-move border-b border-[#ffffff10]"
                >
                    <h3 class="text-[10px] font-semibold text-[#00d9ff] uppercase tracking-wide flex items-center gap-1">
                        <svg class="w-3 h-3 text-[#555555]" fill="currentColor" viewBox="0 0 24 24">
                            <path d="M8 6h2v2H8V6zm6 0h2v2h-2V6zM8 11h2v2H8v-2zm6 0h2v2h-2v-2zm-6 5h2v2H8v-2zm6 0h2v2h-2v-2z"/>
                        </svg>
                        "Jog Controls"
                    </h3>
                    <button
                        class="p-0.5 hover:bg-[#ffffff10] rounded cursor-pointer"
                        title="Dock jog controls"
                        on:click=move |_| jog_popped.set(false)
                    >
                        <svg class="w-3 h-3 text-[#666666] hover:text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                        </svg>
                    </button>
                </div>

                // Content area - NOT draggable, so inputs work normally
                <div class="p-2">
                    <JogControls/>
                </div>
            </div>
        </Show>
    }
}

