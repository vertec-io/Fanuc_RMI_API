//! Exchange capture: every raw JSON frame that crosses the socket is written to
//! disk, in order, alongside the typed parse result and (critically) the serde
//! error whenever a frame fails to deserialize.
//!
//! Layout of an output run directory:
//!
//! ```text
//! probe-output/<UTC timestamp>/
//!   frames/0001-sent-003-FRC_GetStatus.json     one file per raw frame
//!   transcript.ndjson                           every frame, appended, one JSON object per line
//!   steps.json                                  one record per logical exchange
//!   summary.json                                machine-readable findings
//!   capability-report.md                        the human-readable report
//! ```

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use fanuc_rmi::drivers::{RawDirection, RawFrame, RawFrameHook};
use fanuc_rmi::packets::{ResponsePacket, SendPacket};
use serde::Serialize;
use serde_json::Value;

/// One raw frame exactly as it crossed the wire, plus every parse we could
/// attempt on it.
#[derive(Serialize, Clone, Debug)]
pub struct FrameRecord {
    /// Monotonic frame counter across the whole run (1-based).
    pub index: u64,
    /// Wall-clock capture time (UTC, second resolution) and ms since run start.
    pub timestamp_utc: String,
    pub elapsed_ms: u128,
    /// The exchange this frame belongs to.
    pub step: String,
    /// `sent` or `received`.
    pub direction: String,
    /// The exact bytes, minus the `\r\n` frame terminator.
    pub raw: String,
    /// `raw` parsed as generic JSON (None when it is not even valid JSON).
    pub json: Option<Value>,
    /// Set when `raw` is not valid JSON at all.
    pub json_error: Option<String>,
    /// Debug of the strongly-typed `fanuc_rmi` value, when it deserializes.
    pub typed: Option<String>,
    /// The serde error when the typed deserialization FAILED. Never swallowed:
    /// this is the whole point of the capture.
    pub typed_error: Option<String>,
    /// The driver's own note (its serde error for received frames).
    pub driver_note: Option<String>,
}

impl FrameRecord {
    pub fn is_received(&self) -> bool {
        self.direction == "received"
    }
    pub fn failed_to_deserialize(&self) -> bool {
        self.typed_error.is_some()
    }
}

/// One logical request/response exchange.
#[derive(Serialize, Clone, Debug)]
pub struct StepRecord {
    pub index: usize,
    /// Unique per run, e.g. `007-FRC_ReadCartesianPosition-g2`.
    pub label: String,
    /// The RMI packet name, e.g. `FRC_GetStatus`.
    pub command: String,
    pub phase: String,
    pub description: String,
    /// `ok` | `controller_error` | `unknown_command` | `deserialize_failed`
    /// | `timeout` | `send_failed` | `skipped`
    pub status: String,
    pub error_id: Option<u32>,
    pub error_text: Option<String>,
    pub sent_typed: Option<String>,
    pub sent_raw: Option<String>,
    pub received_raw: Option<String>,
    pub received_typed: Option<String>,
    pub detail: Option<String>,
    pub duration_ms: u128,
    pub frame_indices: Vec<u64>,
}

impl StepRecord {
    pub fn succeeded(&self) -> bool {
        self.status == "ok"
    }
    /// The response JSON, when we got one and it was valid JSON.
    pub fn response_json(&self) -> Option<Value> {
        serde_json::from_str(self.received_raw.as_deref()?).ok()
    }
}

pub struct Recorder {
    dir: PathBuf,
    frames_dir: PathBuf,
    started: Instant,
    frame_seq: AtomicU64,
    step_seq: AtomicU64,
    current_step: Mutex<String>,
    transcript: Mutex<File>,
    frames_by_step: Mutex<HashMap<String, Vec<FrameRecord>>>,
    all_frames: Mutex<Vec<FrameRecord>>,
    steps: Mutex<Vec<StepRecord>>,
}

impl Recorder {
    /// Create `<out_root>/<UTC timestamp>/` and its `frames/` subdirectory.
    pub fn new(out_root: &Path) -> std::io::Result<Arc<Self>> {
        let stamp = utc_compact(now_unix_ms());
        let dir = out_root.join(&stamp);
        let frames_dir = dir.join("frames");
        fs::create_dir_all(&frames_dir)?;
        let transcript = OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("transcript.ndjson"))?;
        Ok(Arc::new(Self {
            dir,
            frames_dir,
            started: Instant::now(),
            frame_seq: AtomicU64::new(0),
            step_seq: AtomicU64::new(0),
            current_step: Mutex::new("startup".to_string()),
            transcript: Mutex::new(transcript),
            frames_by_step: Mutex::new(HashMap::new()),
            all_frames: Mutex::new(Vec::new()),
            steps: Mutex::new(Vec::new()),
        }))
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Build the hook handed to `FanucDriver::connect_with_raw_hook`.
    pub fn hook(self: &Arc<Self>) -> RawFrameHook {
        let me = Arc::clone(self);
        RawFrameHook::new(move |frame: RawFrame| me.record(frame))
    }

    fn record(&self, frame: RawFrame) {
        let index = self.frame_seq.fetch_add(1, Ordering::SeqCst) + 1;
        let step = self
            .current_step
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| "unknown".into());

        let (json, json_error) = match serde_json::from_str::<Value>(&frame.payload) {
            Ok(v) => (Some(v), None),
            Err(e) => (None, Some(e.to_string())),
        };

        // Round-trip the frame through the strongly-typed model. For received
        // frames this is the compatibility signal we care about; for sent frames
        // it proves our own serializer produces something we can read back.
        let (typed, typed_error) = match frame.direction {
            RawDirection::Received => match serde_json::from_str::<ResponsePacket>(&frame.payload) {
                Ok(v) => (Some(format!("{v:?}")), None),
                Err(e) => (None, Some(e.to_string())),
            },
            RawDirection::Sent => match serde_json::from_str::<SendPacket>(&frame.payload) {
                Ok(v) => (Some(format!("{v:?}")), None),
                Err(e) => (None, Some(e.to_string())),
            },
        };

        let record = FrameRecord {
            index,
            timestamp_utc: utc_rfc3339(now_unix_ms()),
            elapsed_ms: self.started.elapsed().as_millis(),
            step: step.clone(),
            direction: frame.direction.as_str().to_string(),
            raw: frame.payload.clone(),
            json,
            json_error,
            typed,
            typed_error,
            driver_note: frame.note.clone(),
        };

        // One file per frame, numbered so the exchange order is obvious.
        let file = self.frames_dir.join(format!(
            "{index:04}-{}-{}.json",
            record.direction,
            sanitize(&step)
        ));
        if let Ok(text) = serde_json::to_string_pretty(&record) {
            let _ = fs::write(file, text);
        }
        if let (Ok(mut t), Ok(line)) = (self.transcript.lock(), serde_json::to_string(&record)) {
            let _ = writeln!(t, "{line}");
            let _ = t.flush();
        }
        if let Ok(mut by_step) = self.frames_by_step.lock() {
            by_step.entry(step).or_default().push(record.clone());
        }
        if let Ok(mut all) = self.all_frames.lock() {
            all.push(record);
        }
    }

    pub fn next_step_index(&self) -> usize {
        (self.step_seq.fetch_add(1, Ordering::SeqCst) + 1) as usize
    }

    pub fn begin_step(&self, label: &str) {
        if let Ok(mut cur) = self.current_step.lock() {
            *cur = label.to_string();
        }
    }

    /// Frames captured while `label` was the active step.
    pub fn frames_for(&self, label: &str) -> Vec<FrameRecord> {
        self.frames_by_step
            .lock()
            .ok()
            .and_then(|m| m.get(label).cloned())
            .unwrap_or_default()
    }

    pub fn push_step(&self, step: StepRecord) {
        if let Ok(mut steps) = self.steps.lock() {
            steps.push(step);
        }
    }

    pub fn steps(&self) -> Vec<StepRecord> {
        self.steps.lock().map(|s| s.clone()).unwrap_or_default()
    }

    pub fn frames(&self) -> Vec<FrameRecord> {
        self.all_frames.lock().map(|f| f.clone()).unwrap_or_default()
    }

    pub fn write_file(&self, name: &str, contents: &str) -> std::io::Result<PathBuf> {
        let path = self.dir.join(name);
        fs::write(&path, contents)?;
        Ok(path)
    }

    /// Flush the per-step index. Called once at the end of the run.
    pub fn write_steps(&self) -> std::io::Result<()> {
        let steps = self.steps();
        let text = serde_json::to_string_pretty(&steps).unwrap_or_else(|_| "[]".into());
        self.write_file("steps.json", &text)?;
        Ok(())
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

pub fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// `20260728-193012Z` — used for the run directory name.
pub fn utc_compact(unix_ms: u128) -> String {
    let (y, mo, d, h, mi, s) = utc_parts(unix_ms);
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}Z")
}

/// `2026-07-28T19:30:12Z`
pub fn utc_rfc3339(unix_ms: u128) -> String {
    let (y, mo, d, h, mi, s) = utc_parts(unix_ms);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

/// Civil-from-days (Howard Hinnant's algorithm) so the tool needs no date crate.
fn utc_parts(unix_ms: u128) -> (i64, u32, u32, u32, u32, u32) {
    let secs = (unix_ms / 1000) as i64;
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d, (tod / 3600) as u32, ((tod % 3600) / 60) as u32, (tod % 60) as u32)
}
