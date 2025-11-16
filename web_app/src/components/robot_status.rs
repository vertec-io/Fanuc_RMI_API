use leptos::*;
use crate::websocket::WebSocketManager;

#[component]
pub fn RobotStatus() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let status = ws.status;

    view! {
        <div class="bg-[#111111] rounded border border-[#ffffff10] p-4">
            <h2 class="text-sm font-semibold text-[#00d9ff] mb-3 flex items-center uppercase tracking-wide">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                "Robot Status"
            </h2>
            <div class="grid grid-cols-3 gap-3">
                {move || {
                    if let Some(s) = status.get() {
                        view! {
                            <div class="bg-[#1a1a1a] rounded border border-[#ffffff08] p-3">
                                <div class="text-[#888888] text-xs mb-1.5">"Servo Ready"</div>
                                <div class={move || if s.servo_ready == 1 {
                                    "text-lg font-semibold text-[#00d9ff]"
                                } else {
                                    "text-lg font-semibold text-[#666666]"
                                }}>
                                    {move || if s.servo_ready == 1 { "ON" } else { "OFF" }}
                                </div>
                            </div>
                            <div class="bg-[#1a1a1a] rounded border border-[#ffffff08] p-3">
                                <div class="text-[#888888] text-xs mb-1.5">"TP Mode"</div>
                                <div class="text-lg font-semibold text-white">
                                    {move || s.tp_mode}
                                </div>
                            </div>
                            <div class="bg-[#1a1a1a] rounded border border-[#ffffff08] p-3">
                                <div class="text-[#888888] text-xs mb-1.5">"Motion Status"</div>
                                <div class="text-lg font-semibold text-white">
                                    {move || s.motion_status}
                                </div>
                            </div>
                        }.into_view()
                    } else {
                        view! {
                            <div class="col-span-3 text-center text-[#666666] py-6 text-sm">
                                "Waiting for status data..."
                            </div>
                        }.into_view()
                    }
                }}
            </div>
        </div>
    }
}

