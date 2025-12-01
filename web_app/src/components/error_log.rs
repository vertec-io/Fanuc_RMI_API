use leptos::prelude::*;
use leptos::either::Either;
use crate::websocket::WebSocketManager;

#[component]
pub fn ErrorLog() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let error_log = ws.error_log;
    let (collapsed, set_collapsed) = signal(false);

    let error_count = move || error_log.get().len();

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08]">
            <button
                class="w-full flex items-center justify-between p-2 hover:bg-[#ffffff05] transition-colors"
                on:click=move |_| set_collapsed.update(|v| *v = !*v)
            >
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
                <svg
                    class={move || format!("w-3 h-3 text-[#666666] transition-transform {}", if collapsed.get() { "-rotate-90" } else { "" })}
                    fill="none" stroke="currentColor" viewBox="0 0 24 24"
                >
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/>
                </svg>
            </button>
            <Show when=move || !collapsed.get()>
                <div class="px-2 pb-2 space-y-0.5 max-h-24 overflow-y-auto">
                    {move || {
                        let errors = error_log.get();
                        if errors.is_empty() {
                            Either::Left(view! {
                                <div class="text-center text-[#555555] py-1 text-[9px]">
                                    "No errors"
                                </div>
                            })
                        } else {
                            Either::Right(errors.into_iter().rev().take(10).map(|error| {
                                view! {
                                    <div class="bg-[#111111] border-l-2 border-[#ff4444] rounded-r px-1.5 py-0.5 text-[#ff6666] text-[10px] font-mono truncate">
                                        {error}
                                    </div>
                                }
                            }).collect_view())
                        }
                    }}
                </div>
            </Show>
        </div>
    }
}

