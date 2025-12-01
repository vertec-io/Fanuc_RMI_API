use leptos::prelude::*;
use leptos::either::Either;
use crate::websocket::WebSocketManager;
use crate::robot_models::RobotModel;

#[component]
pub fn Settings() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");

    let (show_settings, set_show_settings) = signal(false);
    let (ws_url, set_ws_url) = signal("ws://127.0.0.1:9000".to_string());
    let (robot_model, set_robot_model) = signal(RobotModel::CRX10iA);
    let (robot_ip, set_robot_ip) = signal("127.0.0.1".to_string());
    let (robot_port, set_robot_port) = signal("16001".to_string());
    let (status_message, set_status_message) = signal(String::new());

    view! {
        <div class="relative">
            // Settings button
            <button
                on:click=move |_| {
                    set_show_settings.update(|v| *v = !*v);
                    set_status_message.set(String::new());
                }
                class="bg-[#1a1a1a] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-medium py-2 px-4 rounded transition-colors flex items-center space-x-2"
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                </svg>
                <span class="text-xs">"Settings"</span>
            </button>

            // Settings modal
            {move || if show_settings.get() {
                Either::Left(view! {
                    <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
                        <div class="bg-[#111111] border border-[#ffffff10] rounded-lg p-6 max-w-md w-full mx-4">
                            <div class="flex items-center justify-between mb-4">
                                <h2 class="text-lg font-semibold text-white">"Connection Settings"</h2>
                                <button
                                    on:click=move |_| {
                                        set_show_settings.update(|v| *v = !*v);
                                    }
                                    class="text-[#888888] hover:text-white transition-colors"
                                >
                                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                    </svg>
                                </button>
                            </div>

                            <div class="space-y-4">
                                // Robot Model Selection
                                <div>
                                    <label class="block text-[#888888] text-xs mb-1.5">"Robot Model"</label>
                                    <select
                                        class="w-full bg-[#1a1a1a] border border-[#ffffff08] rounded px-3 py-2 text-white text-sm focus:border-[#00d9ff] focus:outline-none"
                                        on:change=move |ev| {
                                            let value = event_target_value(&ev);
                                            if let Ok(model) = value.parse::<RobotModel>() {
                                                set_robot_model.set(model);
                                            }
                                        }
                                    >
                                        {RobotModel::all().into_iter().map(|model| {
                                            view! {
                                                <option
                                                    value=model.value()
                                                    selected=move || robot_model.get() == model
                                                >
                                                    {model.display_name()}
                                                </option>
                                            }
                                        }).collect_view()}
                                    </select>
                                    <p class="text-[#666666] text-xs mt-1">"Select your FANUC CRX robot model"</p>
                                </div>

                                // WebSocket URL
                                <div>
                                    <label class="block text-[#888888] text-xs mb-1.5">"WebSocket Server URL"</label>
                                    <input
                                        type="text"
                                        class="w-full bg-[#1a1a1a] border border-[#ffffff08] rounded px-3 py-2 text-white text-sm focus:border-[#00d9ff] focus:outline-none"
                                        prop:value=move || ws_url.get()
                                        on:input=move |ev| set_ws_url.set(event_target_value(&ev))
                                        placeholder="ws://127.0.0.1:9000"
                                    />
                                    <p class="text-[#666666] text-xs mt-1">"WebSocket server address"</p>
                                </div>

                                // Robot IP
                                <div>
                                    <label class="block text-[#888888] text-xs mb-1.5">"Robot IP Address"</label>
                                    <input
                                        type="text"
                                        class="w-full bg-[#1a1a1a] border border-[#ffffff08] rounded px-3 py-2 text-white text-sm focus:border-[#00d9ff] focus:outline-none"
                                        prop:value=move || robot_ip.get()
                                        on:input=move |ev| set_robot_ip.set(event_target_value(&ev))
                                        placeholder="127.0.0.1"
                                    />
                                    <p class="text-[#666666] text-xs mt-1">"Robot controller IP (requires server restart)"</p>
                                </div>

                                // Robot Port
                                <div>
                                    <label class="block text-[#888888] text-xs mb-1.5">"Robot Port"</label>
                                    <input
                                        type="number"
                                        class="w-full bg-[#1a1a1a] border border-[#ffffff08] rounded px-3 py-2 text-white text-sm focus:border-[#00d9ff] focus:outline-none"
                                        prop:value=move || robot_port.get()
                                        on:input=move |ev| set_robot_port.set(event_target_value(&ev))
                                        placeholder="16001"
                                    />
                                    <p class="text-[#666666] text-xs mt-1">"RMI port (default: 16001)"</p>
                                </div>

                                // Status message
                                {move || {
                                    let msg = status_message.get();
                                    if !msg.is_empty() {
                                        Either::Left(view! {
                                            <div class="text-xs text-[#00d9ff] bg-[#00d9ff10] border border-[#00d9ff20] rounded px-3 py-2">
                                                {msg}
                                            </div>
                                        })
                                    } else {
                                        Either::Right(())
                                    }
                                }}

                                // Buttons
                                <div class="flex space-x-2 pt-2">
                                    <button
                                        on:click={
                                            let ws = ws.clone();
                                            move |_| {
                                                let new_ws_url = ws_url.get();

                                                // Validate WebSocket URL
                                                if !new_ws_url.starts_with("ws://") && !new_ws_url.starts_with("wss://") {
                                                    set_status_message.set("❌ WebSocket URL must start with ws:// or wss://".to_string());
                                                    return;
                                                }

                                                // Validate robot port
                                                if robot_port.get().parse::<u16>().is_err() {
                                                    set_status_message.set("❌ Invalid port number".to_string());
                                                    return;
                                                }

                                                // Reconnect with new settings
                                                ws.reconnect(&new_ws_url);
                                                set_status_message.set("✓ Reconnecting with new settings...".to_string());
                                            }
                                        }
                                        class="flex-1 bg-[#00d9ff] hover:bg-[#00b8dd] text-black font-semibold py-2 px-4 rounded transition-colors text-sm"
                                    >
                                        "Apply"
                                    </button>
                                    <button
                                        on:click=move |_| {
                                            set_robot_model.set(RobotModel::CRX10iA);
                                            set_ws_url.set("ws://127.0.0.1:9000".to_string());
                                            set_robot_ip.set("127.0.0.1".to_string());
                                            set_robot_port.set("16001".to_string());
                                            set_status_message.set("✓ Reset to defaults".to_string());
                                        }
                                        class="flex-1 bg-[#1a1a1a] hover:bg-[#2a2a2a] border border-[#ffffff08] text-white font-medium py-2 px-4 rounded transition-colors text-sm"
                                    >
                                        "Reset"
                                    </button>
                                </div>
                            </div>
                        </div>
                    </div>
                })
            } else {
                Either::Right(())
            }}
        </div>
    }
}

