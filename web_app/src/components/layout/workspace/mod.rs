//! Workspace module - Main workspace with routed content.
//!
//! This module contains:
//! - Main workspace component with routing
//! - Dashboard view (control and info tabs)
//! - Programs view (program management)
//! - Settings view (configuration)
//! - Shared context types

pub mod context;
pub mod dashboard;
pub mod programs;
pub mod settings;

pub use context::*;
pub use dashboard::DashboardView;
pub use dashboard::control::ControlTab;
pub use dashboard::info::InfoTab;
pub use programs::ProgramsView;
pub use settings::SettingsView;

use leptos::prelude::*;
use leptos_router::components::{ParentRoute, Redirect, Route, Routes};
use leptos_router::path;

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

