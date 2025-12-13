use leptos::prelude::*;
use crate::websocket::WebSocketManager;

/// Simple error indicator - shows error count, errors are displayed in console
#[component]
pub fn ErrorLog() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let error_log = ws.error_log;

    let error_count = move || error_log.get().len();

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#ff4444] flex items-center uppercase tracking-wide">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                "Errors"
                {move || {
                    let count = error_count();
                    if count > 0 {
                        Some(view! {
                            <span class="ml-1.5 bg-[#ff4444] text-black text-[9px] px-1 py-0.5 rounded-full font-bold">
                                {count}
                            </span>
                        })
                    } else {
                        None
                    }
                }}
            </h2>
            <div class="text-[8px] text-[#666666] mt-0.5">
                "View errors in Console"
            </div>
        </div>
    }
}

