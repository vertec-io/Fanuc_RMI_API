use leptos::*;
use crate::websocket::WebSocketManager;

#[component]
pub fn ErrorLog() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let error_log = ws.error_log;

    view! {
        <div class="bg-[#111111] rounded border border-[#ffffff10] p-4">
            <h2 class="text-sm font-semibold text-[#ff4444] mb-3 flex items-center uppercase tracking-wide">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                "Errors"
            </h2>
            <div class="space-y-1.5 max-h-64 overflow-y-auto">
                {move || {
                    let errors = error_log.get();
                    if errors.is_empty() {
                        view! {
                            <div class="text-center text-[#666666] py-4 text-sm">
                                "No errors"
                            </div>
                        }.into_view()
                    } else {
                        errors.into_iter().rev().map(|error| {
                            view! {
                                <div class="bg-[#1a1a1a] border border-[#ff444420] rounded p-2 text-[#ff4444] text-xs font-mono">
                                    {error}
                                </div>
                            }
                        }).collect_view()
                    }
                }}
            </div>
        </div>
    }
}

