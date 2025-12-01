//! Dashboard module - Robot status, control, and monitoring.
//!
//! This module contains the Dashboard view components organized into:
//! - `info` - Information tab with position display, frame/tool management
//! - `control` - Control tab with command composer, console, program execution

pub mod info;
pub mod control;

pub use info::*;
pub use control::*;

use leptos::prelude::*;
use crate::components::layout::workspace::context::WorkspaceContext;

/// Dashboard view with Info and Control tabs.
#[component]
pub fn DashboardView() -> impl IntoView {
    let _ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let (active_tab, set_active_tab) = signal("info".to_string());

    view! {
        <div class="h-full flex flex-col">
            // Tab bar
            <div class="flex border-b border-[#ffffff08] shrink-0">
                {["info", "control"].into_iter().map(|tab| {
                    let label = match tab {
                        "info" => "Info",
                        _ => "Control",
                    };
                    let is_active = move || active_tab.get() == tab;
                    view! {
                        <button
                            class={move || format!(
                                "px-4 py-2 text-[10px] font-medium transition-colors {}",
                                if is_active() {
                                    "text-[#00d9ff] border-b-2 border-[#00d9ff]"
                                } else {
                                    "text-[#666666] hover:text-[#888888]"
                                }
                            )}
                            on:click=move |_| set_active_tab.set(tab.to_string())
                        >
                            {label}
                        </button>
                    }
                }).collect_view()}
            </div>

            // Tab content
            <div class="flex-1 p-2 overflow-hidden">
                {move || match active_tab.get().as_str() {
                    "info" => leptos::either::Either::Left(view! { <InfoTab/> }),
                    _ => leptos::either::Either::Right(view! { <ControlTab/> }),
                }}
            </div>
        </div>
    }
}

