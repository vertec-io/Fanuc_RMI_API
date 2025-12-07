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
use crate::websocket::WebSocketManager;
use crate::components::layout::LayoutContext;

/// Main workspace with routed content.
#[component]
pub fn MainWorkspace() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");

    // Create and provide workspace context
    let workspace_ctx = WorkspaceContext::new();
    provide_context(workspace_ctx);

    // Clear recent commands and load jog defaults when switching robots
    // Track the previous connection ID to detect changes
    let prev_connection_id = StoredValue::new(ws.active_connection_id.get_untracked());
    Effect::new(move |_| {
        let current_id = ws.active_connection_id.get();
        let prev_id = prev_connection_id.get_value();

        // Only clear if the connection ID actually changed (not on initial load)
        if current_id != prev_id {
            log::info!("Robot connection changed from {:?} to {:?}, clearing recent commands", prev_id, current_id);
            workspace_ctx.recent_commands.set(Vec::new());
            workspace_ctx.selected_command_id.set(None);
            workspace_ctx.command_log.set(Vec::new());
            prev_connection_id.set_value(current_id);

            // Load robot-specific jog defaults when a robot connects
            if let Some(conn) = ws.get_active_connection() {
                log::info!("Loading jog defaults for robot: cart_speed={}, cart_step={}, joint_speed={}, joint_step={}",
                    conn.default_cartesian_jog_speed, conn.default_cartesian_jog_step,
                    conn.default_joint_jog_speed, conn.default_joint_jog_step);
                layout_ctx.jog_speed.set(conn.default_cartesian_jog_speed);
                layout_ctx.jog_step.set(conn.default_cartesian_jog_step);
                layout_ctx.joint_jog_speed.set(conn.default_joint_jog_speed);
                layout_ctx.joint_jog_step.set(conn.default_joint_jog_step);
            }
        }
    });

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

