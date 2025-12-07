//! Dashboard Control tab - Robot control and program execution.
//!
//! Contains components for quick commands, command composition,
//! console logging, program execution visualization, and joint jogging.

mod quick_commands;
mod command_input;
mod command_log;
mod program_display;
mod load_modal;
mod composer;
mod joint_jog;

pub use quick_commands::QuickCommandsPanel;
pub use command_input::CommandInputSection;
pub use command_log::CommandLogPanel;
pub use program_display::ProgramVisualDisplay;
pub use load_modal::LoadProgramModal;
pub use composer::CommandComposerModal;
pub use joint_jog::JointJogPanel;

use leptos::prelude::*;
use crate::components::layout::workspace::context::WorkspaceContext;
use crate::websocket::WebSocketManager;

/// Control tab content (command composer).
#[component]
pub fn ControlTab() -> impl IntoView {
    let ctx = use_context::<WorkspaceContext>().expect("WorkspaceContext not found");
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager context");
    let show_composer = ctx.show_composer;
    let robot_connected = ws.robot_connected;

    view! {
        <div class="h-full flex flex-col gap-2">
            // Quick Commands section
            <QuickCommandsPanel/>

            // Command input section
            <CommandInputSection/>

            // Joint Jog Panel (only show when connected to robot)
            <Show when=move || robot_connected.get()>
                <JointJogPanel/>
            </Show>

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

