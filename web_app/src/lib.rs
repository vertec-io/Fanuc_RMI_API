use leptos::*;
use wasm_bindgen::prelude::*;

mod components;
mod websocket;
mod robot_models;

use components::*;
use websocket::WebSocketManager;
pub use robot_models::RobotModel;

#[wasm_bindgen(start)]
pub fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    mount_to_body(|| view! { <App/> })
}

#[component]
pub fn App() -> impl IntoView {
    let ws_manager = WebSocketManager::new();
    provide_context(ws_manager);

    view! {
        <div class="min-h-screen bg-[#0a0a0a]">
            <div class="container mx-auto px-6 py-6">
                <Header/>
                <div class="grid grid-cols-1 lg:grid-cols-3 gap-4 mt-6">
                    <div class="lg:col-span-2">
                        <RobotStatus/>
                        <div class="mt-4">
                            <JogControls/>
                        </div>
                    </div>
                    <div>
                        <PositionDisplay/>
                        <div class="mt-4">
                            <ErrorLog/>
                        </div>
                    </div>
                </div>
                <div class="mt-4">
                    <MotionLog/>
                </div>
            </div>
        </div>
    }
}

#[component]
fn Header() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let connected = ws.connected;

    view! {
        <header class="bg-[#111111] rounded border border-[#ffffff10] p-4">
            <div class="flex items-center justify-between">
                <div class="flex items-center space-x-3">
                    <div class="w-10 h-10 bg-[#00d9ff] rounded flex items-center justify-center">
                        <svg class="w-6 h-6 text-black" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"/>
                        </svg>
                    </div>
                    <div>
                        <h1 class="text-xl font-semibold text-white">
                            "FANUC RMI Control"
                        </h1>
                        <p class="text-[#888888] text-xs">"Real-time Robot Monitoring & Control"</p>
                    </div>
                </div>
                <div class="flex items-center space-x-4">
                    <Settings/>
                    <div class="flex items-center space-x-2">
                        <div class={move || if connected.get() {
                            "w-2 h-2 bg-[#00d9ff] rounded-full animate-pulse"
                        } else {
                            "w-2 h-2 bg-[#666666] rounded-full"
                        }}></div>
                        <span class="text-[#cccccc] text-xs font-medium">
                            {move || if connected.get() { "Connected" } else { "Disconnected" }}
                        </span>
                    </div>
                </div>
            </div>
        </header>
    }
}

