//! Left navigation bar component.

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_location;

/// Left vertical navbar with navigation links.
#[component]
pub fn LeftNavbar() -> impl IntoView {
    view! {
        <nav class="w-12 bg-[#111111] border-r border-[#ffffff10] flex flex-col items-center py-2 shrink-0">
            <NavLink
                icon="📊"
                label="DASH"
                href="/dashboard"
                match_prefix="/dashboard"
                is_root=true
            />
            <NavLink
                icon="📁"
                label="PROG"
                href="/programs"
                match_prefix="/programs"
                is_root=false
            />
            <NavLink
                icon="⚙️"
                label="SET"
                href="/settings"
                match_prefix="/settings"
                is_root=false
            />
        </nav>
    }
}

/// Individual navigation link using leptos_router's A component.
#[component]
fn NavLink(
    icon: &'static str,
    label: &'static str,
    href: &'static str,
    match_prefix: &'static str,
    is_root: bool,
) -> impl IntoView {
    let location = use_location();
    let is_active = move || {
        let path = location.pathname.get();
        if is_root {
            path == "/" || path.starts_with(match_prefix)
        } else {
            path.starts_with(match_prefix)
        }
    };

    view! {
        <A
            href=href
            attr:class=move || if is_active() {
                "w-10 h-10 rounded bg-[#00d9ff15] border border-[#00d9ff40] flex flex-col items-center justify-center mb-1 transition-all no-underline"
            } else {
                "w-10 h-10 rounded hover:bg-[#ffffff08] border border-transparent flex flex-col items-center justify-center mb-1 transition-all no-underline"
            }
        >
            <span class="text-sm leading-none">{icon}</span>
            <span class=move || if is_active() {
                "text-[8px] text-[#00d9ff] mt-0.5 font-medium leading-none"
            } else {
                "text-[8px] text-[#666666] mt-0.5 leading-none"
            }>{label}</span>
        </A>
    }
}

