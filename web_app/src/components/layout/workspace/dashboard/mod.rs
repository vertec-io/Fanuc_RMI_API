//! Dashboard module - Robot status, control, and monitoring.
//!
//! This module contains the Dashboard view components organized into:
//! - `info` - Configuration tab with frame/tool management and robot configuration
//! - `control` - Control tab with command composer, console, program execution
//! - `hmi` - HMI Panel View with live I/O widgets

pub mod info;
pub mod control;
pub mod hmi;

use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::hooks::use_location;

/// Dashboard view with Configuration and Control tabs using nested routing.
#[component]
pub fn DashboardView() -> impl IntoView {
    let location = use_location();

    // Determine active tab from current path
    let is_active_tab = move |tab: &str| {
        let path = location.pathname.get();
        path.ends_with(tab)
    };

    view! {
        <div class="h-full flex flex-col">
            // Tab bar with navigation links
            <div class="flex border-b border-[#ffffff08] shrink-0">
                <A
                    href="/dashboard/control"
                    attr:class=move || format!(
                        "px-4 py-2 text-[10px] font-medium transition-colors {}",
                        if is_active_tab("control") {
                            "text-[#00d9ff] border-b-2 border-[#00d9ff]"
                        } else {
                            "text-[#666666] hover:text-[#888888]"
                        }
                    )
                >
                    "Control"
                </A>
                <A
                    href="/dashboard/hmi"
                    attr:class=move || format!(
                        "px-4 py-2 text-[10px] font-medium transition-colors {}",
                        if is_active_tab("hmi") {
                            "text-[#00d9ff] border-b-2 border-[#00d9ff]"
                        } else {
                            "text-[#666666] hover:text-[#888888]"
                        }
                    )
                >
                    "HMI"
                </A>
                <A
                    href="/dashboard/info"
                    attr:class=move || format!(
                        "px-4 py-2 text-[10px] font-medium transition-colors {}",
                        if is_active_tab("info") {
                            "text-[#00d9ff] border-b-2 border-[#00d9ff]"
                        } else {
                            "text-[#666666] hover:text-[#888888]"
                        }
                    )
                >
                    "Configuration"
                </A>
            </div>

            // Tab content - renders nested route
            <div class="flex-1 p-2 overflow-hidden">
                <Outlet/>
            </div>
        </div>
    }
}

