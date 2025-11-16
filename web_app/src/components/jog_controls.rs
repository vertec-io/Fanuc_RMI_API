use leptos::*;
use fanuc_rmi::dto::*;
use crate::websocket::WebSocketManager;

#[component]
pub fn JogControls() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let (jog_speed, set_jog_speed) = create_signal(10.0);
    let (step_distance, set_step_distance) = create_signal(1.0);

    let send_jog = move |dx: f32, dy: f32, dz: f32| {
        let packet = SendPacket::Instruction(Instruction::FrcLinearRelative(
            FrcLinearRelative {
                sequence_id: (js_sys::Date::now() as u32) % 1000000,
                configuration: Configuration {
                    front: 1,
                    up: 1,
                    left: 0,
                    turn4: 0,
                    turn5: 0,
                    turn6: 0,
                },
                position: Position {
                    x: dx,
                    y: dy,
                    z: dz,
                    w: 0.0,
                    p: 0.0,
                    r: 0.0,
                    ext1: 0.0,
                    ext2: 0.0,
                    ext3: 0.0,
                },
                speed_type: fanuc_rmi::SpeedType::MMSec,
                speed: jog_speed.get_untracked() as f64,
                term_type: fanuc_rmi::TermType::FINE,
                term_value: 1,
            },
        ));
        ws.send_command(packet);
    };
    let send_jog = store_value(send_jog);

    view! {
        <div class="bg-[#111111] rounded border border-[#ffffff10] p-4">
            <h2 class="text-sm font-semibold text-[#00d9ff] mb-3 flex items-center uppercase tracking-wide">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/>
                </svg>
                "Jog Controls"
            </h2>

            <div class="grid grid-cols-2 gap-3 mb-4">
                <div>
                    <label class="block text-[#888888] text-xs mb-1.5">"Speed (mm/s)"</label>
                    <input
                        type="number"
                        class="w-full bg-[#1a1a1a] border border-[#ffffff08] rounded px-3 py-2 text-white text-sm focus:border-[#00d9ff] focus:outline-none"
                        prop:value=move || jog_speed.get()
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            if let Ok(v) = val.parse::<f64>() {
                                set_jog_speed.set(v);
                            }
                        }
                    />
                </div>
                <div>
                    <label class="block text-[#888888] text-xs mb-1.5">"Step (mm)"</label>
                    <input
                        type="number"
                        class="w-full bg-[#1a1a1a] border border-[#ffffff08] rounded px-3 py-2 text-white text-sm focus:border-[#00d9ff] focus:outline-none"
                        prop:value=move || step_distance.get()
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            if let Ok(v) = val.parse::<f64>() {
                                set_step_distance.set(v);
                            }
                        }
                    />
                </div>
            </div>

            <div class="grid grid-cols-3 gap-3">
                <div class="col-span-3 grid grid-cols-3 gap-2">
                    <div></div>
                    <button
                        class="bg-[#1a1a1a] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-3 px-4 rounded transition-colors"
                        on:click=move |_| send_jog.with_value(|f| f(0.0, step_distance.get_untracked() as f32, 0.0))
                    >
                        <div class="text-xl">"↑"</div>
                        <div class="text-[10px] mt-0.5">"Y+"</div>
                    </button>
                    <div></div>
                </div>

                <div class="col-span-3 grid grid-cols-3 gap-2">
                    <button
                        class="bg-[#1a1a1a] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-3 px-4 rounded transition-colors"
                        on:click=move |_| send_jog.with_value(|f| f(-step_distance.get_untracked() as f32, 0.0, 0.0))
                    >
                        <div class="text-xl">"←"</div>
                        <div class="text-[10px] mt-0.5">"X-"</div>
                    </button>
                    <button
                        class="bg-[#1a1a1a] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-3 px-4 rounded transition-colors"
                        on:click=move |_| send_jog.with_value(|f| f(0.0, 0.0, step_distance.get_untracked() as f32))
                    >
                        <div class="text-xl">"▲"</div>
                        <div class="text-[10px] mt-0.5">"Z+"</div>
                    </button>
                    <button
                        class="bg-[#1a1a1a] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-3 px-4 rounded transition-colors"
                        on:click=move |_| send_jog.with_value(|f| f(step_distance.get_untracked() as f32, 0.0, 0.0))
                    >
                        <div class="text-xl">"→"</div>
                        <div class="text-[10px] mt-0.5">"X+"</div>
                    </button>
                </div>

                <div class="col-span-3 grid grid-cols-3 gap-2">
                    <div></div>
                    <button
                        class="bg-[#1a1a1a] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-3 px-4 rounded transition-colors"
                        on:click=move |_| send_jog.with_value(|f| f(0.0, -step_distance.get_untracked() as f32, 0.0))
                    >
                        <div class="text-xl">"↓"</div>
                        <div class="text-[10px] mt-0.5">"Y-"</div>
                    </button>
                    <button
                        class="bg-[#1a1a1a] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-3 px-4 rounded transition-colors"
                        on:click=move |_| send_jog.with_value(|f| f(0.0, 0.0, -step_distance.get_untracked() as f32))
                    >
                        <div class="text-xl">"▼"</div>
                        <div class="text-[10px] mt-0.5">"Z-"</div>
                    </button>
                </div>
            </div>
        </div>
    }
}

