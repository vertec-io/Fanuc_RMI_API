use leptos::prelude::*;
use crate::websocket::WebSocketManager;

#[component]
pub fn RobotStatus() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let status = ws.status;

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 flex items-center uppercase tracking-wide">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                "Status"
            </h2>
            <div class="grid grid-cols-3 gap-1">
                <Show
                    when=move || status.get().is_some()
                    fallback=|| view! {
                        <div class="col-span-3 text-center text-[#555555] py-2 text-[10px]">
                            "Waiting..."
                        </div>
                    }
                >
                    {move || {
                        let s = status.get().unwrap();
                        view! {
                            <div class="bg-[#111111] rounded px-1.5 py-1 text-center">
                                <div class="text-[#666666] text-[9px] mb-0.5">"Servo"</div>
                                <div class={move || if s.servo_ready == 1 {
                                    "text-xs font-semibold text-[#00d9ff]"
                                } else {
                                    "text-xs font-semibold text-[#555555]"
                                }}>
                                    {move || if s.servo_ready == 1 { "ON" } else { "OFF" }}
                                </div>
                            </div>
                            <div class="bg-[#111111] rounded px-1.5 py-1 text-center">
                                <div class="text-[#666666] text-[9px] mb-0.5">"TP"</div>
                                <div class="text-xs font-semibold text-white">
                                    {move || s.tp_mode}
                                </div>
                            </div>
                            <div class="bg-[#111111] rounded px-1.5 py-1 text-center">
                                <div class="text-[#666666] text-[9px] mb-0.5">"Motion"</div>
                                <div class="text-xs font-semibold text-white">
                                    {move || s.motion_status}
                                </div>
                            </div>
                        }
                    }}
                </Show>
            </div>
        </div>
    }
}

