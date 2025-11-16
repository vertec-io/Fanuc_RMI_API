use leptos::*;
use crate::websocket::WebSocketManager;

#[component]
pub fn MotionLog() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let motion_log = ws.motion_log;

    view! {
        <div class="bg-[#111111] rounded border border-[#ffffff10] p-4">
            <h2 class="text-sm font-semibold text-[#00d9ff] mb-3 flex items-center uppercase tracking-wide">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
                </svg>
                "Motion Log"
            </h2>
            <div class="space-y-1 max-h-96 overflow-y-auto font-mono text-xs">
                {move || {
                    let log = motion_log.get();
                    if log.is_empty() {
                        view! {
                            <div class="text-center text-[#666666] py-4 text-sm">
                                "No motion events yet"
                            </div>
                        }.into_view()
                    } else {
                        log.into_iter().rev().take(20).map(|entry| {
                            view! {
                                <div class="bg-[#1a1a1a] border border-[#ffffff08] rounded px-2.5 py-1.5 text-[#00d9ff]">
                                    {entry}
                                </div>
                            }
                        }).collect_view()
                    }
                }}
            </div>
        </div>
    }
}

