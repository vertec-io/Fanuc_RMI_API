//! Desktop layout components.

mod top_bar;
mod left_navbar;
mod right_panel;
mod main_workspace;

pub use top_bar::TopBar;
pub use left_navbar::LeftNavbar;
pub use right_panel::{RightPanel, FloatingJogControls};
pub use main_workspace::MainWorkspace;

use leptos::prelude::*;
use leptos_router::hooks::use_location;

/// Desktop layout context - provides shared state across layout components.
#[derive(Clone, Copy)]
pub struct LayoutContext {
    /// Current dashboard tab (0 = Control, 1 = Info).
    pub dashboard_tab: RwSignal<usize>,
    /// Whether jog controls are popped out (floating).
    pub jog_popped: RwSignal<bool>,
    /// Jog controls floating position (x, y).
    pub jog_position: RwSignal<(i32, i32)>,
    /// Whether the program browser sidebar is visible in Programs view.
    pub show_program_browser: RwSignal<bool>,
    /// Jog speed in mm/s (shared between docked and floating jog controls).
    pub jog_speed: RwSignal<f64>,
    /// Jog step distance in mm (shared between docked and floating jog controls).
    pub jog_step: RwSignal<f64>,
}

impl LayoutContext {
    pub fn new() -> Self {
        Self {
            dashboard_tab: RwSignal::new(0),
            jog_popped: RwSignal::new(false),
            jog_position: RwSignal::new((100, 100)),
            show_program_browser: RwSignal::new(false), // Hidden by default
            jog_speed: RwSignal::new(10.0),
            jog_step: RwSignal::new(1.0),
        }
    }
}

/// Root desktop layout component.
#[component]
pub fn DesktopLayout() -> impl IntoView {
    // Create and provide layout context
    let layout_ctx = LayoutContext::new();
    provide_context(layout_ctx);

    // Get current location to determine if we're on dashboard
    let location = use_location();
    let is_dashboard = move || {
        let path = location.pathname.get();
        path == "/" || path.starts_with("/dashboard")
    };

    view! {
        <div class="h-screen w-screen flex flex-col bg-[#0a0a0a] overflow-hidden">
            // Header
            <TopBar/>

            // Main content area (navbar + workspace + right panel)
            <div class="flex-1 flex overflow-hidden">
                // Left navbar
                <LeftNavbar/>

                // Main workspace with routes
                <MainWorkspace/>

                // Right panel (only visible in Dashboard routes)
                <Show when=is_dashboard>
                    <RightPanel/>
                </Show>
            </div>
        </div>
    }
}

