//! One-request/one-response exchange driver.
//!
//! Every probe step goes through [`Session::exchange`], which subscribes to the
//! driver's response channel *and* its protocol-error channel before sending, so
//! a response that fails to deserialize is reported as such rather than showing
//! up as a mystery timeout.

use std::sync::Arc;
use std::time::{Duration, Instant};

use fanuc_rmi::drivers::FanucDriver;
use fanuc_rmi::packets::{PacketPriority, ResponsePacket, SendPacket};
use serde_json::Value;

use crate::capture::{Recorder, StepRecord};

/// Which response frame closes this exchange.
#[derive(Clone, Copy, Debug)]
pub enum Expect {
    /// Match `{"Communication": "<name>"}`.
    Communication(&'static str),
    /// Match `{"Command": "<name>"}`.
    Command(&'static str),
    /// Match `{"Instruction": "<name>"}`.
    Instruction(&'static str),
}

impl Expect {
    fn key(&self) -> &'static str {
        match self {
            Expect::Communication(_) => "Communication",
            Expect::Command(_) => "Command",
            Expect::Instruction(_) => "Instruction",
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            Expect::Communication(n) | Expect::Command(n) | Expect::Instruction(n) => n,
        }
    }
}

pub struct Session {
    pub driver: FanucDriver,
    pub rec: Arc<Recorder>,
    pub timeout: Duration,
}

impl Session {
    /// Send `packet` and wait for the matching response, recording the raw JSON
    /// in both directions plus the typed parse.
    pub async fn exchange(
        &self,
        phase: &str,
        description: &str,
        packet: SendPacket,
        expect: Expect,
        timeout: Duration,
    ) -> StepRecord {
        self.exchange_labeled(phase, description, packet, expect, timeout, None).await
    }

    /// As [`Self::exchange`], with a suffix that disambiguates repeats of the
    /// same command (e.g. `g2`, `mask-0b11`, `R7`).
    pub async fn exchange_labeled(
        &self,
        phase: &str,
        description: &str,
        packet: SendPacket,
        expect: Expect,
        timeout: Duration,
        suffix: Option<&str>,
    ) -> StepRecord {
        let index = self.rec.next_step_index();
        let label = match suffix {
            Some(s) => format!("{index:03}-{}-{s}", expect.name()),
            None => format!("{index:03}-{}", expect.name()),
        };
        self.rec.begin_step(&label);

        let sent_typed = format!("{packet:?}");
        let mut resp_rx = self.driver.response_tx.subscribe();
        let mut err_rx = self.driver.error_tx.subscribe();
        let started = Instant::now();

        let mut status = "ok".to_string();
        let mut detail: Option<String> = None;
        let mut received_typed: Option<String> = None;

        if let Err(e) = self.driver.send_packet(packet, PacketPriority::Standard) {
            status = "send_failed".into();
            detail = Some(e);
        } else {
            let wanted = expect.name();
            let key = expect.key();
            let outcome = tokio::time::timeout(timeout, async {
                loop {
                    tokio::select! {
                        got = resp_rx.recv() => match got {
                            Ok(resp) => {
                                if let Some(v) = tag_of(&resp, key) {
                                    // `Unknown` is how a controller reports a
                                    // command it does not implement.
                                    if v == wanted || v == "Unknown" {
                                        return Ok((v, format!("{resp:?}")));
                                    }
                                }
                            }
                            Err(e) => return Err(format!("response channel: {e}")),
                        },
                        got = err_rx.recv() => match got {
                            Ok(pe) => {
                                return Err(format!(
                                    "deserialize_failed::{}::{}",
                                    pe.message,
                                    pe.raw_data.unwrap_or_default()
                                ));
                            }
                            Err(e) => return Err(format!("error channel: {e}")),
                        },
                    }
                }
            })
            .await;

            match outcome {
                Ok(Ok((tag, typed))) => {
                    received_typed = Some(typed);
                    if tag == "Unknown" && tag != wanted {
                        status = "unknown_command".into();
                        detail = Some(format!(
                            "controller answered with an Unknown response instead of {wanted}"
                        ));
                    }
                }
                Ok(Err(e)) if e.starts_with("deserialize_failed::") => {
                    status = "deserialize_failed".into();
                    detail = Some(e.trim_start_matches("deserialize_failed::").to_string());
                }
                Ok(Err(e)) => {
                    status = "send_failed".into();
                    detail = Some(e);
                }
                Err(_) => {
                    status = "timeout".into();
                    detail = Some(format!("no {wanted} response within {:?}", timeout));
                }
            }
        }

        let duration_ms = started.elapsed().as_millis();
        let frames = self.rec.frames_for(&label);
        let sent_raw = frames.iter().find(|f| !f.is_received()).map(|f| f.raw.clone());
        let received = frames.iter().rev().find(|f| f.is_received());
        let received_raw = received.map(|f| f.raw.clone());

        // A frame that reached us but would not deserialize is the single most
        // important thing to surface; make sure it wins over a bare timeout.
        if let Some(f) = frames.iter().find(|f| f.is_received() && f.failed_to_deserialize()) {
            if status == "timeout" {
                status = "deserialize_failed".into();
            }
            detail = Some(match detail.take() {
                Some(d) => format!("{d} | serde: {}", f.typed_error.clone().unwrap_or_default()),
                None => format!("serde: {}", f.typed_error.clone().unwrap_or_default()),
            });
        }

        let error_id = received_raw
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .and_then(|v| v.get("ErrorID").and_then(|e| e.as_u64()))
            .map(|e| e as u32);

        if status == "ok" {
            if let Some(id) = error_id {
                if id != 0 {
                    status = "controller_error".into();
                }
            }
        }

        let error_text = error_id
            .filter(|id| *id != 0)
            .map(fanuc_rmi::format_error_id);

        let step = StepRecord {
            index,
            label: label.clone(),
            command: expect.name().to_string(),
            phase: phase.to_string(),
            description: description.to_string(),
            status,
            error_id,
            error_text,
            sent_typed: Some(sent_typed),
            sent_raw,
            received_raw,
            received_typed,
            detail,
            duration_ms,
            frame_indices: frames.iter().map(|f| f.index).collect(),
        };

        println!("{}", format_step_line(&step));
        self.rec.push_step(step.clone());
        self.rec.begin_step(&format!("after-{label}"));
        step
    }

    /// Record a step that was deliberately not attempted (safety gate, missing
    /// precondition, …) so the report can say *why* rather than staying silent.
    pub fn skip(&self, phase: &str, command: &str, description: &str, reason: &str) -> StepRecord {
        let index = self.rec.next_step_index();
        let step = StepRecord {
            index,
            label: format!("{index:03}-{command}-skipped"),
            command: command.to_string(),
            phase: phase.to_string(),
            description: description.to_string(),
            status: "skipped".into(),
            error_id: None,
            error_text: None,
            sent_typed: None,
            sent_raw: None,
            received_raw: None,
            received_typed: None,
            detail: Some(reason.to_string()),
            duration_ms: 0,
            frame_indices: vec![],
        };
        println!("{}", format_step_line(&step));
        self.rec.push_step(step.clone());
        step
    }
}

fn format_step_line(step: &StepRecord) -> String {
    let mark = match step.status.as_str() {
        "ok" => "  ok  ",
        "controller_error" => " FAIL ",
        "deserialize_failed" => " DESER",
        "timeout" => " TMOUT",
        "unknown_command" => " UNSUP",
        "skipped" => " skip ",
        _ => " ERR  ",
    };
    let extra = match (&step.error_text, &step.detail) {
        (Some(e), _) => format!("  <- {e}"),
        (None, Some(d)) => format!("  <- {d}"),
        _ => String::new(),
    };
    format!(
        "[{mark}] {:>3}. {:<28} {}{}",
        step.index, step.command, step.description, extra
    )
}

/// The serde tag of a response packet (`Command` / `Communication` /
/// `Instruction`), obtained by re-serializing the typed value.
fn tag_of(resp: &ResponsePacket, key: &str) -> Option<String> {
    serde_json::to_value(resp)
        .ok()?
        .get(key)?
        .as_str()
        .map(|s| s.to_string())
}
