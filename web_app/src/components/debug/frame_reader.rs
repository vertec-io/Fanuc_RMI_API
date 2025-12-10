use leptos::prelude::*;
use crate::websocket::WebSocketManager;

/// Temporary debugging component to test reading arbitrary frame numbers
#[component]
pub fn FrameReaderDebug() -> impl IntoView {
    let ws = expect_context::<WebSocketManager>();
    
    let (frame_number, set_frame_number) = signal(0u8);
    let (last_request, set_last_request) = signal::<Option<u8>>(None);
    let (last_set_request, set_last_set_request) = signal::<Option<u8>>(None);
    let (response_data, set_response_data) = signal::<Option<String>>(None);
    let (error_message, set_error_message) = signal::<Option<String>>(None);
    
    // Listen for frame data responses (from read command)
    Effect::new(move || {
        if let Some(requested_frame) = last_request.get() {
            // Check if we got a response for this frame
            if let Some(frame_data) = ws.frame_data.get().get(&requested_frame) {
                set_response_data.set(Some(format!(
                    "✅ Read Frame {}: X={:.3}, Y={:.3}, Z={:.3}, W={:.3}, P={:.3}, R={:.3}",
                    requested_frame,
                    frame_data.x, frame_data.y, frame_data.z,
                    frame_data.w, frame_data.p, frame_data.r
                )));
                set_error_message.set(None);
            }
        }
    });

    // Listen for active frame/tool changes (from set command)
    Effect::new(move || {
        if let Some(set_frame) = last_set_request.get() {
            // Check if the active frame matches what we requested
            if let Some((active_frame, active_tool)) = ws.active_frame_tool.get() {
                if active_frame == set_frame {
                    set_response_data.set(Some(format!(
                        "✅ Set Active Frame to {} (UTool: {})",
                        active_frame, active_tool
                    )));
                    set_error_message.set(None);
                    set_last_set_request.set(None); // Clear so we don't keep showing this
                }
            }
        }
    });
    
    let read_frame = move |_| {
        let frame = frame_number.get();
        set_last_request.set(Some(frame));
        set_response_data.set(None);
        set_error_message.set(Some(format!("Requesting frame {}... (waiting for response)", frame)));

        ws.read_frame_data(frame);

        // Set a timeout to detect if no response comes back
        set_timeout(
            move || {
                if response_data.get().is_none() {
                    set_error_message.set(Some(format!("⚠️ No response received for frame {} (timeout after 6 seconds)", frame)));
                }
            },
            std::time::Duration::from_secs(6),
        );
    };

    let set_frame = move |_| {
        let frame = frame_number.get();
        // Get current active tool (or default to 1)
        let current_tool = ws.active_frame_tool.get()
            .map(|(_, tool)| tool)
            .unwrap_or(1);

        set_last_set_request.set(Some(frame));
        set_response_data.set(None);
        set_error_message.set(Some(format!("Setting active frame to {} (keeping UTool {})...", frame, current_tool)));

        ws.set_active_frame_tool(frame, current_tool);

        // Set a timeout to detect if no response comes back
        set_timeout(
            move || {
                if response_data.get().is_none() {
                    set_error_message.set(Some(format!("⚠️ No confirmation received for set frame {} (timeout after 6 seconds)", frame)));
                }
            },
            std::time::Duration::from_secs(6),
        );
    };
    
    view! {
        <div class="bg-[#0a0a0a] border border-[#00d9ff]/20 rounded-lg p-4">
            <div class="flex items-center gap-2 mb-3">
                <span class="text-[#00d9ff] text-sm font-medium">"[DEBUG] Frame Reader"</span>
            </div>
            
            <div class="space-y-3">
                <div class="flex items-center gap-2">
                    <label class="text-[#888888] text-xs">Frame Number:</label>
                    <input
                        type="number"
                        min="0"
                        max="255"
                        class="bg-[#111111] border border-[#333333] rounded px-2 py-1 text-white text-sm w-20 focus:outline-none focus:border-[#00d9ff]"
                        prop:value=move || frame_number.get()
                        on:input=move |ev| {
                            if let Ok(val) = event_target_value(&ev).parse::<u8>() {
                                set_frame_number.set(val);
                            }
                        }
                    />
                    <button
                        class="bg-[#00d9ff] hover:bg-[#00b8d4] text-black px-3 py-1 rounded text-sm font-medium transition-colors"
                        on:click=read_frame
                    >
                        "Read Frame"
                    </button>
                    <button
                        class="bg-[#ff9500] hover:bg-[#ff8000] text-black px-3 py-1 rounded text-sm font-medium transition-colors"
                        on:click=set_frame
                    >
                        "Set Frame"
                    </button>
                </div>
                
                {move || {
                    if let Some(err) = error_message.get() {
                        view! {
                            <div class="bg-[#111111] border border-[#ff6b6b]/30 rounded p-2">
                                <p class="text-[#ff6b6b] text-xs font-mono">{err}</p>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
                
                {move || {
                    if let Some(data) = response_data.get() {
                        view! {
                            <div class="bg-[#111111] border border-[#00d9ff]/30 rounded p-2">
                                <p class="text-[#00d9ff] text-xs font-mono">{data}</p>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div></div> }.into_any()
                    }
                }}
                
                <div class="text-[#666666] text-xs">
                    <p>"• Enter a frame number (0-255) and click 'Read Frame'"</p>
                    <p>"• Watch for response or timeout"</p>
                    <p>"• Pendant shows: UFrames 0-9, UTools 1-10"</p>
                    <p>"• FRC_GetStatus reports: NumberUFrame=9, NumberUTool=10"</p>
                </div>
            </div>
        </div>
    }
}

