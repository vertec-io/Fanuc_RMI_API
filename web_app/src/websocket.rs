use fanuc_rmi::dto::*;
use leptos::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{BinaryType, ErrorEvent, MessageEvent, WebSocket};

#[derive(Clone)]
pub struct WebSocketManager {
    pub connected: ReadSignal<bool>,
    set_connected: WriteSignal<bool>,
    pub position: ReadSignal<Option<(f64, f64, f64)>>,
    set_position: WriteSignal<Option<(f64, f64, f64)>>,
    pub status: ReadSignal<Option<RobotStatusData>>,
    set_status: WriteSignal<Option<RobotStatusData>>,
    pub motion_log: ReadSignal<Vec<String>>,
    set_motion_log: WriteSignal<Vec<String>>,
    pub error_log: ReadSignal<Vec<String>>,
    set_error_log: WriteSignal<Vec<String>>,
    ws: StoredValue<Option<WebSocket>>,
    ws_url: StoredValue<String>,
}

#[derive(Clone, Debug)]
pub struct RobotStatusData {
    pub servo_ready: i8,
    pub tp_mode: i8,
    pub motion_status: i8,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (connected, set_connected) = create_signal(false);
        let (position, set_position) = create_signal(None);
        let (status, set_status) = create_signal(None);
        let (motion_log, set_motion_log) = create_signal(Vec::new());
        let (error_log, set_error_log) = create_signal(Vec::new());
        let ws = store_value(None);
        let ws_url = store_value("ws://127.0.0.1:9000".to_string());

        let manager = Self {
            connected,
            set_connected,
            position,
            set_position,
            status,
            set_status,
            motion_log,
            set_motion_log,
            error_log,
            set_error_log,
            ws,
            ws_url,
        };

        manager.connect();
        manager
    }

    fn connect(&self) {
        let url = self.ws_url.get_value();
        let ws = match WebSocket::new(&url) {
            Ok(ws) => ws,
            Err(e) => {
                log::error!("Failed to create WebSocket: {:?}", e);
                return;
            }
        };
        ws.set_binary_type(BinaryType::Arraybuffer);

        let set_connected = self.set_connected;
        let set_position = self.set_position;
        let set_status = self.set_status;
        let set_motion_log = self.set_motion_log;
        let set_error_log = self.set_error_log;

        // On open
        let onopen_callback = Closure::wrap(Box::new(move |_| {
            set_connected.set(true);
            log::info!("WebSocket connected");
        }) as Box<dyn FnMut(JsValue)>);
        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        onopen_callback.forget();

        // On message
        let onmessage_callback = Closure::wrap(Box::new(move |e: MessageEvent| {
            if let Ok(array_buffer) = e.data().dyn_into::<js_sys::ArrayBuffer>() {
                let uint8_array = js_sys::Uint8Array::new(&array_buffer);
                let bytes = uint8_array.to_vec();

                if let Ok(response) = bincode::deserialize::<ResponsePacket>(&bytes) {
                    match response {
                        ResponsePacket::InstructionResponse(resp) => {
                            let (seq_id, error_id) = get_response_ids(&resp);

                            if error_id != 0 {
                                set_error_log.update(|log| {
                                    log.push(format!("Motion error: Seq:{} Err:{}", seq_id, error_id));
                                    if log.len() > 10 {
                                        log.remove(0);
                                    }
                                });
                            } else {
                                let msg = format_instruction_response(&resp, seq_id);
                                set_motion_log.update(|log| {
                                    log.push(msg);
                                    if log.len() > 50 {
                                        log.remove(0);
                                    }
                                });
                            }
                        }
                        ResponsePacket::CommandResponse(resp) => match resp {
                            CommandResponse::FrcReadCartesianPosition(r) => {
                                if r.error_id == 0 {
                                    set_position.set(Some((
                                        r.pos.x as f64,
                                        r.pos.y as f64,
                                        r.pos.z as f64,
                                    )));
                                }
                            }
                            CommandResponse::FrcGetStatus(s) => {
                                if s.error_id == 0 {
                                    set_status.set(Some(RobotStatusData {
                                        servo_ready: s.servo_ready,
                                        tp_mode: s.tp_mode,
                                        motion_status: s.rmi_motion_status,
                                    }));
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);
        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();

        // On error
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            log::error!("WebSocket error: {:?}", e);
        }) as Box<dyn FnMut(ErrorEvent)>);
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        onerror_callback.forget();

        self.ws.set_value(Some(ws));
    }

    pub fn send_command(&self, packet: SendPacket) {
        if let Some(ws) = self.ws.get_value() {
            if let Ok(binary) = bincode::serialize(&packet) {
                let _ = ws.send_with_u8_array(&binary);
            }
        }
    }

    pub fn reconnect(&self, new_url: &str) {
        // Close existing connection
        if let Some(ws) = self.ws.get_value() {
            let _ = ws.close();
        }

        // Update URL
        self.ws_url.set_value(new_url.to_string());

        // Set disconnected
        self.set_connected.set(false);

        // Clear data
        self.set_position.set(None);
        self.set_status.set(None);

        // Reconnect
        self.connect();
    }
}

fn get_response_ids(resp: &InstructionResponse) -> (u32, u32) {
    match resp {
        InstructionResponse::FrcLinearRelative(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcJointMotion(r) => (r.sequence_id, r.error_id),
        InstructionResponse::FrcWaitTime(r) => (r.sequence_id, r.error_id),
        _ => (0, 0),
    }
}

fn format_instruction_response(resp: &InstructionResponse, seq_id: u32) -> String {
    match resp {
        InstructionResponse::FrcLinearRelative(_) => {
            format!("✓ Linear move completed (Seq:{})", seq_id)
        }
        InstructionResponse::FrcJointMotion(_) => {
            format!("✓ Joint move completed (Seq:{})", seq_id)
        }
        InstructionResponse::FrcWaitTime(_) => {
            format!("✓ Wait completed (Seq:{})", seq_id)
        }
        _ => format!("✓ Motion completed (Seq:{})", seq_id),
    }
}
