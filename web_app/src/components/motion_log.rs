use leptos::prelude::*;
use leptos::either::Either;
use crate::websocket::WebSocketManager;

#[component]
pub fn MotionLog() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let motion_log = ws.motion_log;
    let (collapsed, set_collapsed) = signal(false);

    let log_count = move || motion_log.get().len();

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08]">
            <button
                class="w-full flex items-center justify-between p-2 hover:bg-[#ffffff05] transition-colors"
                on:click=move |_| set_collapsed.update(|v| *v = !*v)
            >
                <h2 class="text-[10px] font-semibold text-[#00d9ff] flex items-center uppercase tracking-wide">
                    <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"/>
                    </svg>
                    "Motion Log"
                    {move || {
                        let count = log_count();
                        if count > 0 {
                            Some(view! {
                                <span class="ml-1.5 bg-[#00d9ff20] text-[#00d9ff] text-[9px] px-1 py-0.5 rounded font-medium">
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
                <div class="px-2 pb-2 space-y-0.5 max-h-32 overflow-y-auto font-mono">
                    {move || {
                        let log = motion_log.get();
                        if log.is_empty() {
                            Either::Left(view! {
                                <div class="text-center text-[#555555] py-1 text-[9px]">
                                    "No motion events"
                                </div>
                            })
                        } else {
                            Either::Right(log.into_iter().rev().take(15).map(|entry| {
                                view! {
                                    <div class="bg-[#111111] rounded px-1.5 py-0.5 text-[#00d9ff] text-[10px] truncate">
                                        {entry}
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

