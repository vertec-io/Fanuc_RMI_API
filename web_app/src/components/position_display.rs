use leptos::prelude::*;
use crate::websocket::WebSocketManager;

#[component]
pub fn PositionDisplay() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let position = ws.position;

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 flex items-center uppercase tracking-wide">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"/>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 11a3 3 0 11-6 0 3 3 0 016 0z"/>
                </svg>
                "Position"
            </h2>
            <Show
                when=move || position.get().is_some()
                fallback=|| view! {
                    <div class="text-center text-[#555555] py-2 text-[10px]">
                        "Waiting..."
                    </div>
                }
            >
                {move || {
                    let (x, y, z) = position.get().unwrap();
                    view! {
                        <div class="space-y-0.5">
                            <div class="flex justify-between items-center bg-[#111111] rounded px-1.5 py-1">
                                <span class="text-[#666666] text-[10px] font-medium">"X"</span>
                                <span class="text-[11px] font-mono text-white tabular-nums">{format!("{:.2}", x)}<span class="text-[#555555] ml-0.5">"mm"</span></span>
                            </div>
                            <div class="flex justify-between items-center bg-[#111111] rounded px-1.5 py-1">
                                <span class="text-[#666666] text-[10px] font-medium">"Y"</span>
                                <span class="text-[11px] font-mono text-white tabular-nums">{format!("{:.2}", y)}<span class="text-[#555555] ml-0.5">"mm"</span></span>
                            </div>
                            <div class="flex justify-between items-center bg-[#111111] rounded px-1.5 py-1">
                                <span class="text-[#666666] text-[10px] font-medium">"Z"</span>
                                <span class="text-[11px] font-mono text-white tabular-nums">{format!("{:.2}", z)}<span class="text-[#555555] ml-0.5">"mm"</span></span>
                            </div>
                        </div>
                    }
                }}
            </Show>
        </div>
    }
}

