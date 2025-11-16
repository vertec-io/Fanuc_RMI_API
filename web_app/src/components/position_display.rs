use leptos::*;
use crate::websocket::WebSocketManager;

#[component]
pub fn PositionDisplay() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let position = ws.position;

    view! {
        <div class="bg-[#111111] rounded border border-[#ffffff10] p-4">
            <h2 class="text-sm font-semibold text-[#00d9ff] mb-3 flex items-center uppercase tracking-wide">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"/>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"/>
                </svg>
                "Position"
            </h2>
            {move || {
                if let Some((x, y, z)) = position.get() {
                    view! {
                        <div class="space-y-2">
                            <div class="flex justify-between items-center bg-[#1a1a1a] rounded border border-[#ffffff08] p-2.5">
                                <span class="text-[#888888] text-sm font-medium">"X:"</span>
                                <span class="text-base font-mono text-white">{format!("{:.2}", x)} " mm"</span>
                            </div>
                            <div class="flex justify-between items-center bg-[#1a1a1a] rounded border border-[#ffffff08] p-2.5">
                                <span class="text-[#888888] text-sm font-medium">"Y:"</span>
                                <span class="text-base font-mono text-white">{format!("{:.2}", y)} " mm"</span>
                            </div>
                            <div class="flex justify-between items-center bg-[#1a1a1a] rounded border border-[#ffffff08] p-2.5">
                                <span class="text-[#888888] text-sm font-medium">"Z:"</span>
                                <span class="text-base font-mono text-white">{format!("{:.2}", z)} " mm"</span>
                            </div>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="text-center text-[#666666] py-6 text-sm">
                            "Waiting for position data..."
                        </div>
                    }.into_view()
                }
            }}
        </div>
    }
}

