use leptos::prelude::*;
use leptos::mount::mount_to_body;
use leptos_router::components::Router;
use wasm_bindgen::prelude::*;

mod components;
mod websocket;
mod robot_models;

use components::{DesktopLayout, FloatingJogControls};
use websocket::WebSocketManager;
pub use robot_models::RobotModel;

#[wasm_bindgen(start)]
pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    mount_to_body(|| view! { <App/> });
}

#[component]
pub fn App() -> impl IntoView {
    let ws_manager = WebSocketManager::new();
    provide_context(ws_manager);

    view! {
        <Router>
            <DesktopLayout/>
            <FloatingJogControls/>
        </Router>
    }
}

