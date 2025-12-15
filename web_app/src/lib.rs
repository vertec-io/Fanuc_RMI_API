use leptos::prelude::*;
use leptos::mount::mount_to_body;
use leptos_router::components::{Router, Routes, Route};
use leptos_router::path;
use wasm_bindgen::prelude::*;

mod components;
mod hmi_broadcast;
mod websocket;

use components::{DesktopLayout, FloatingJogControls, FloatingIOStatus, ToastContainer, HmiPopup};
use hmi_broadcast::HmiBroadcastHandler;
use websocket::WebSocketManager;
pub use web_common::RobotModel;

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
            <Routes fallback=|| view! { <DesktopLayout/> }>
                // HMI popup route - minimal chrome for pop-out windows
                <Route path=path!("/hmi-popup") view=HmiPopup />

                // All other routes go through DesktopLayout
                <Route path=path!("/*any") view=|| view! {
                    <DesktopLayout/>
                    <FloatingJogControls/>
                    <FloatingIOStatus/>
                    <ToastContainer/>
                    <HmiBroadcastHandler/>
                } />
            </Routes>
        </Router>
    }
}

