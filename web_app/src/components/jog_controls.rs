use leptos::prelude::*;
use fanuc_rmi::dto::*;
use crate::websocket::WebSocketManager;
use crate::components::layout::LayoutContext;

#[component]
pub fn JogControls() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let layout_ctx = use_context::<LayoutContext>().expect("LayoutContext not found");

    // Use shared signals from LayoutContext so settings persist between docked/floating
    let jog_speed = layout_ctx.jog_speed;
    let step_distance = layout_ctx.jog_step;

    let send_jog = move |dx: f64, dy: f64, dz: f64| {
        let packet = SendPacket::Instruction(Instruction::FrcLinearRelative(
            FrcLinearRelative {
                sequence_id: 0, // Will be assigned by driver
                configuration: Configuration {
                    u_tool_number: 4, // Update to match your physical robot
                    u_frame_number: 3, // Update to match your physical robot
                    front: 1,
                    up: 1,
                    left: 0,
                    flip: 0,
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
    let send_jog = StoredValue::new(send_jog);

    view! {
        <div class="bg-[#0a0a0a] rounded border border-[#ffffff08] p-2">
            <h2 class="text-[10px] font-semibold text-[#00d9ff] mb-1.5 flex items-center uppercase tracking-wide">
                <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3"/>
                </svg>
                "Jog"
            </h2>

            <div class="grid grid-cols-2 gap-1 mb-2">
                <div>
                    <label class="block text-[#666666] text-[9px] mb-0.5">"Speed mm/s"</label>
                    <input
                        type="number"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-1.5 py-1 text-white text-[11px] focus:border-[#00d9ff] focus:outline-none"
                        prop:value=move || jog_speed.get()
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            if let Ok(v) = val.parse::<f64>() {
                                jog_speed.set(v);
                            }
                        }
                    />
                </div>
                <div>
                    <label class="block text-[#666666] text-[9px] mb-0.5">"Step mm"</label>
                    <input
                        type="number"
                        class="w-full bg-[#111111] border border-[#ffffff08] rounded px-1.5 py-1 text-white text-[11px] focus:border-[#00d9ff] focus:outline-none"
                        prop:value=move || step_distance.get()
                        on:input=move |ev| {
                            let val = event_target_value(&ev);
                            if let Ok(v) = val.parse::<f64>() {
                                step_distance.set(v);
                            }
                        }
                    />
                </div>
            </div>

            <div class="grid grid-cols-3 gap-1">
                <div class="col-span-3 grid grid-cols-3 gap-1">
                    <div></div>
                    <button
                        class="bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        on:click=move |_| send_jog.with_value(|f| f(0.0, step_distance.get_untracked(), 0.0))
                    >
                        <div class="text-sm leading-none">"↑"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"Y+"</div>
                    </button>
                    <div></div>
                </div>

                <div class="col-span-3 grid grid-cols-3 gap-1">
                    <button
                        class="bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        on:click=move |_| send_jog.with_value(|f| f(-step_distance.get_untracked(), 0.0, 0.0))
                    >
                        <div class="text-sm leading-none">"←"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"X-"</div>
                    </button>
                    <button
                        class="bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        on:click=move |_| send_jog.with_value(|f| f(0.0, 0.0, step_distance.get_untracked()))
                    >
                        <div class="text-sm leading-none">"▲"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"Z+"</div>
                    </button>
                    <button
                        class="bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        on:click=move |_| send_jog.with_value(|f| f(step_distance.get_untracked(), 0.0, 0.0))
                    >
                        <div class="text-sm leading-none">"→"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"X+"</div>
                    </button>
                </div>

                <div class="col-span-3 grid grid-cols-3 gap-1">
                    <div></div>
                    <button
                        class="bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        on:click=move |_| send_jog.with_value(|f| f(0.0, -step_distance.get_untracked(), 0.0))
                    >
                        <div class="text-sm leading-none">"↓"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"Y-"</div>
                    </button>
                    <button
                        class="bg-[#111111] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-1.5 rounded transition-colors text-center"
                        on:click=move |_| send_jog.with_value(|f| f(0.0, 0.0, -step_distance.get_untracked()))
                    >
                        <div class="text-sm leading-none">"▼"</div>
                        <div class="text-[8px] text-[#666666] mt-0.5">"Z-"</div>
                    </button>
                </div>
            </div>
        </div>
    }
}

