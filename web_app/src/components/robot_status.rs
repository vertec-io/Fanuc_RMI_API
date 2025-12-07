use leptos::prelude::*;
use crate::websocket::WebSocketManager;

#[component]
pub fn RobotStatus() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let status = ws.status;
    let tp_initialized = ws.tp_program_initialized;
    let robot_connected = ws.robot_connected;
    let active_configuration = ws.active_configuration;

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 flex items-center uppercase tracking-wide">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>
                </svg>
                "Status"
            </h2>
            <div class="grid grid-cols-4 gap-1">
                <Show
                    when=move || status.get().is_some()
                    fallback=move || view! {
                        <div class="col-span-4 text-center text-[#555555] py-2 text-[10px]">
                            "Waiting..."
                        </div>
                    }
                >
                    {move || {
                        let s = status.get().unwrap();
                        view! {
                            <div class="bg-[#111111] rounded px-1 py-1 text-center">
                                <div class="text-[#666666] text-[8px] mb-0.5">"Servo"</div>
                                <div class={move || if s.servo_ready == 1 {
                                    "text-[10px] font-semibold text-[#00d9ff]"
                                } else {
                                    "text-[10px] font-semibold text-[#555555]"
                                }}>
                                    {move || if s.servo_ready == 1 { "ON" } else { "OFF" }}
                                </div>
                            </div>
                            <div class="bg-[#111111] rounded px-1 py-1 text-center">
                                <div class="text-[#666666] text-[8px] mb-0.5">"TP"</div>
                                <div class="text-[10px] font-semibold text-white">
                                    {move || s.tp_mode}
                                </div>
                            </div>
                            <div class="bg-[#111111] rounded px-1 py-1 text-center">
                                <div class="text-[#666666] text-[8px] mb-0.5">"Motion"</div>
                                <div class="text-[10px] font-semibold text-white">
                                    {move || s.motion_status}
                                </div>
                            </div>
                            // TP Program Initialized indicator
                            <div class="bg-[#111111] rounded px-1 py-1 text-center" title="TP Program Initialized - Required for motion commands">
                                <div class="text-[#666666] text-[8px] mb-0.5">"TP Init"</div>
                                <div class={move || {
                                    if !robot_connected.get() {
                                        "text-[10px] font-semibold text-[#555555]"
                                    } else if tp_initialized.get() {
                                        "text-[10px] font-semibold text-[#22c55e]"
                                    } else {
                                        "text-[10px] font-semibold text-[#ff4444]"
                                    }
                                }}>
                                    {move || {
                                        if !robot_connected.get() {
                                            "—"
                                        } else if tp_initialized.get() {
                                            "✓"
                                        } else {
                                            "✗"
                                        }
                                    }}
                                </div>
                            </div>
                        }
                    }}
                </Show>
            </div>

            // Active Configuration Display
            <Show when=move || robot_connected.get() && active_configuration.get().is_some()>
                {move || {
                    let config = active_configuration.get().unwrap();
                    let config_name = config.loaded_from_name.clone().unwrap_or_else(|| "Custom".to_string());
                    let modified = config.modified;
                    view! {
                        <div class="mt-2 pt-2 border-t border-[#ffffff08]">
                            // Configuration name with modified indicator
                            <div class="flex items-center justify-between mb-1">
                                <span class="text-[#666666] text-[8px]">"Config:"</span>
                                <span class="text-[10px] font-medium text-white flex items-center">
                                    {config_name}
                                    <Show when=move || modified>
                                        <span class="ml-1 text-[#ffaa00]" title="Configuration has been modified">"⚠️"</span>
                                    </Show>
                                </span>
                            </div>
                            // UFrame and UTool
                            <div class="flex items-center justify-between">
                                <span class="text-[#666666] text-[8px]">"UFrame:"</span>
                                <span class="text-[10px] font-medium text-[#00d9ff]">{config.u_frame_number}</span>
                                <span class="text-[#666666] text-[8px] ml-2">"UTool:"</span>
                                <span class="text-[10px] font-medium text-[#00d9ff]">{config.u_tool_number}</span>
                            </div>
                        </div>
                    }
                }}
            </Show>
        </div>
    }
}

