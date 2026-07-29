//! The capability sweep itself.
//!
//! Phase order is deliberate:
//!   1. `inventory`   — pure reads. Nothing here changes controller state.
//!   2. `initialize`  — the group-mask discovery step (0b01 / 0b11 / 0b10).
//!   3. `lifecycle`   — abort/reset/pause/continue/program-pause (opt-in).
//!   4. `registers`   — numeric register write-and-restore (opt-in).
//!   5. `motion`      — 1 mm linear on Group 1, ±1° on Group 2 (opt-in, gated).
//!   6. `teardown`    — leave the controller as we found it, then disconnect.

use std::time::Duration;

use fanuc_rmi::commands::*;
use fanuc_rmi::instructions::{
    CartesianGroups, FrcJointRelativeJRep, FrcLinearRelative, GroupBlock, JointGroups,
};
use fanuc_rmi::packets::{Command, Communication, Instruction, SendPacket};
use fanuc_rmi::{Configuration, JointAngles, Position, SpeedType, TermType};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::capture::StepRecord;
use crate::session::{Expect, Session};
use crate::Cli;

/// Founder-set safety envelope. These are ceilings, not suggestions: the CLI
/// refuses to run with anything faster.
pub const MAX_LINEAR_SPEED_MM_S: f64 = 1.0;
pub const MAX_ROTARY_SPEED_DEG_S: f64 = 0.5;
/// Motion amplitudes are fixed, not configurable.
pub const LINEAR_STEP_MM: f64 = 1.0;
pub const ROTARY_STEP_DEG: f32 = 1.0;
/// How far a completed move may sit from its commanded target before we call it
/// a failure and stop.
const LINEAR_TOLERANCE_MM: f64 = 0.35;
const ROTARY_TOLERANCE_DEG: f32 = 0.25;
/// How far any Group-1 joint may drift while a Group-2-only packet is executing
/// before we call it arm motion and stop. A zero G1 block means "hold", so this
/// should be exactly zero on a compliant controller.
const ARM_HOLD_TOLERANCE_DEG: f32 = 0.05;

#[derive(Serialize, Clone, Debug, Default)]
pub struct InitAttempt {
    pub mask: u8,
    pub mask_binary: String,
    pub meaning: String,
    pub status: String,
    pub error_id: Option<u32>,
    pub error_text: Option<String>,
    pub echoed_group_mask: Option<u16>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct RegisterReading {
    pub register: u16,
    pub status: String,
    pub value: Option<f32>,
    pub error_id: Option<u32>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct AxisMotionResult {
    pub axis: String,
    pub commanded_mm: f64,
    pub measured_mm: Option<f64>,
    pub residual_mm: Option<f64>,
    pub passed: bool,
    pub detail: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct MotionReport {
    pub attempted: bool,
    pub linear_speed_mm_s: f64,
    pub rotary_speed_deg_s: f64,
    pub start_position: Option<Value>,
    pub end_position: Option<Value>,
    pub return_residual_mm: Option<f64>,
    pub axes: Vec<AxisMotionResult>,
    pub group2: Vec<AxisMotionResult>,
    /// Set when the arm moved during a Group-2 packet whose G1 block was all
    /// zeros — i.e. the controller read the "hold" block as an absolute target.
    pub group2_arm_moved: Option<String>,
    pub aborted: bool,
    pub abort_reason: Option<String>,
}

#[derive(Serialize, Clone, Debug, Default)]
pub struct Findings {
    pub addr: String,
    pub started_utc: String,
    pub mode: String,
    pub connect: Option<Value>,
    pub status: Option<Value>,
    pub group1_cartesian: Option<Value>,
    pub group2_cartesian: Option<Value>,
    pub group1_joints: Option<Value>,
    pub group2_joints: Option<Value>,
    pub uframe_utool_g1: Option<Value>,
    pub uframe_utool_g2: Option<Value>,
    pub active_configuration: Option<Value>,
    pub init_attempts: Vec<InitAttempt>,
    pub session_mask: Option<u8>,
    pub registers: Vec<RegisterReading>,
    pub register_write: Option<Value>,
    pub motion: MotionReport,
    pub notes: Vec<String>,
}

pub struct Sweep<'a> {
    pub s: &'a Session,
    pub cli: &'a Cli,
    pub findings: Findings,
    /// True while an RMI session is initialized (so teardown knows to abort).
    initialized: bool,
}

impl<'a> Sweep<'a> {
    pub fn new(s: &'a Session, cli: &'a Cli, findings: Findings) -> Self {
        Self { s, cli, findings, initialized: false }
    }

    /// The default per-command response timeout (from the session).
    fn t(&self) -> Duration {
        self.s.timeout
    }
    fn motion_t(&self) -> Duration {
        Duration::from_secs(self.cli.motion_timeout_secs)
    }

    fn note(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        println!("        note: {msg}");
        self.findings.notes.push(msg);
    }

    // ---------------------------------------------------------------- phase 1

    pub async fn inventory(&mut self) {
        let cmd = |c: Command| SendPacket::Command(c);

        let status = self
            .s
            .exchange("inventory", "controller status / servo / TP mode", cmd(Command::FrcGetStatus), Expect::Command("FRC_GetStatus"), self.t())
            .await;
        self.findings.status = status.response_json();
        let status_typed: Option<FrcGetStatusResponse> = parse_ok(&status);

        self.s
            .exchange("inventory", "most recent controller error", cmd(Command::FrcReadError(FrcReadError::new(Some(1)))), Expect::Command("FRC_ReadError"), self.t())
            .await;

        self.s
            .exchange("inventory", "current TCP speed", cmd(Command::FrcReadTCPSpeed), Expect::Command("FRC_ReadTCPSpeed"), self.t())
            .await;

        for group in [1u8, 2u8] {
            let st = self
                .s
                .exchange_labeled("inventory", &format!("active UFrame/UTool, group {group}"), cmd(Command::FrcGetUFrameUTool(FrcGetUFrameUTool::new(Some(group)))), Expect::Command("FRC_GetUFrameUTool"), self.t(), Some(&format!("g{group}")))
                .await;
            match group {
                1 => self.findings.uframe_utool_g1 = st.response_json(),
                _ => self.findings.uframe_utool_g2 = st.response_json(),
            }
        }

        for group in [1u8, 2u8] {
            let st = self
                .s
                .exchange_labeled("inventory", &format!("Cartesian position + live Configuration, group {group}"), cmd(Command::FrcReadCartesianPosition(FrcReadCartesianPosition::new(Some(group)))), Expect::Command("FRC_ReadCartesianPosition"), self.t(), Some(&format!("g{group}")))
                .await;
            match group {
                1 => {
                    self.findings.group1_cartesian = st.response_json();
                    if let Some(r) = parse_ok::<FrcReadCartesianPositionResponse>(&st) {
                        self.findings.active_configuration = serde_json::to_value(&r.config).ok();
                    }
                }
                _ => self.findings.group2_cartesian = st.response_json(),
            }
        }

        for group in [1u8, 2u8] {
            let st = self
                .s
                .exchange_labeled("inventory", &format!("joint angles, group {group}"), cmd(Command::FrcReadJointAngles(FrcReadJointAngles::new(Some(group)))), Expect::Command("FRC_ReadJointAngles"), self.t(), Some(&format!("g{group}")))
                .await;
            match group {
                1 => self.findings.group1_joints = st.response_json(),
                _ => self.findings.group2_joints = st.response_json(),
            }
        }

        // Frame/tool payloads: the active ones (per FRC_GetStatus) plus #1 as a
        // control, so a controller that only populates some slots is obvious.
        let active_uframe = status_typed.as_ref().map(|s| s.number_uframe).unwrap_or(1);
        let active_utool = status_typed.as_ref().map(|s| s.number_utool).unwrap_or(1);
        for frame in dedup2(active_uframe, 1) {
            self.s
                .exchange_labeled("inventory", &format!("UFrame {frame} data"), cmd(Command::FrcReadUFrameData(FrcReadUFrameData::new(Some(1), frame))), Expect::Command("FRC_ReadUFrameData"), self.t(), Some(&format!("uf{frame}")))
                .await;
        }
        for tool in dedup2(active_utool, 1) {
            self.s
                .exchange_labeled("inventory", &format!("UTool {tool} data"), cmd(Command::FrcReadUToolData(FrcReadUToolData::new(Some(1), tool))), Expect::Command("FRC_ReadUToolData"), self.t(), Some(&format!("ut{tool}")))
                .await;
        }

        for reg in [1u16, 2u16] {
            self.s
                .exchange_labeled("inventory", &format!("position register PR[{reg}], group 1"), cmd(Command::FrcReadPositionRegister(FrcReadPositionRegister::new(Some(1), reg))), Expect::Command("FRC_ReadPositionRegister"), self.t(), Some(&format!("pr{reg}")))
                .await;
        }

        self.s
            .exchange("inventory", "digital input DI[1]", cmd(Command::FrcReadDIN(FrcReadDIN::new(1))), Expect::Command("FRC_ReadDIN"), self.t())
            .await;
        self.s
            .exchange("inventory", "analog input AI[1]", cmd(Command::FrcReadAIN(FrcReadAIN::new(1))), Expect::Command("FRC_ReadAIN"), self.t())
            .await;
        self.s
            .exchange("inventory", "group input GI[1]", cmd(Command::FrcReadGIN(FrcReadGIN::new(1))), Expect::Command("FRC_ReadGIN"), self.t())
            .await;

        // Numeric register sweep — R[n] is not part of the base RMI command set,
        // so this doubles as a portability probe.
        for reg in 1..=self.cli.register_sweep {
            let st = self
                .s
                .exchange_labeled("inventory", &format!("numeric register R[{reg}]"), cmd(Command::FrcReadRegister(FrcReadRegister::new(reg))), Expect::Command("FRC_ReadRegister"), self.t(), Some(&format!("R{reg}")))
                .await;
            let value = parse_ok::<FrcReadRegisterResponse>(&st).map(|r| r.register_value);
            self.findings.registers.push(RegisterReading {
                register: reg,
                status: st.status.clone(),
                value,
                error_id: st.error_id,
            });
            // A controller without R[n] support answers the first one with an
            // error or Unknown; don't hammer it 24 more times.
            if reg == 1 && !st.succeeded() {
                self.note(format!(
                    "FRC_ReadRegister R[1] returned `{}` — stopping the register sweep early (this controller likely lacks numeric-register RMI support)",
                    st.status
                ));
                break;
            }
        }
    }

    // ---------------------------------------------------------------- phase 2

    /// THE discovery step: which motion groups can this session actually
    /// reserve? meteorite hardcodes `0b11` whenever a positioner is bound, so
    /// the answer decides whether that hardcode is right for this controller.
    pub async fn initialize_discovery(&mut self) {
        if self.cli.skip_initialize {
            self.s.skip("initialize", "FRC_Initialize", "group-mask discovery", "--skip-initialize was passed");
            return;
        }

        let attempts: [(u8, &str); 3] = [
            (0b01, "Group 1 only (the arm) — the RMI default"),
            (0b11, "Groups 1+2 (arm + positioner) — what meteorite hardcodes when a positioner is bound"),
            (0b10, "Group 2 only (positioner) — proves whether the group exists independently of the arm"),
        ];

        for (mask, meaning) in attempts {
            if self.initialized {
                // A session must be torn down before it can be re-reserved.
                self.abort_step("initialize", "release the previous RMI session before re-initializing").await;
                self.initialized = false;
            }
            let st = self
                .s
                .exchange_labeled(
                    "initialize",
                    &format!("FRC_Initialize GroupMask={mask:#04b} — {meaning}"),
                    SendPacket::Command(Command::FrcInitialize(FrcInitialize::from_bits(mask))),
                    Expect::Command("FRC_Initialize"),
                    self.t(),
                    Some(&format!("mask{mask:02b}")),
                )
                .await;
            let echoed = parse_ok::<FrcInitializeResponse>(&st).map(|r| r.group_mask);
            if st.succeeded() {
                self.initialized = true;
            }
            self.findings.init_attempts.push(InitAttempt {
                mask,
                mask_binary: format!("{mask:#04b}"),
                meaning: meaning.to_string(),
                status: st.status.clone(),
                error_id: st.error_id,
                error_text: st.error_text.clone(),
                echoed_group_mask: echoed,
            });
        }

        // Settle on the richest mask that actually worked, and leave the session
        // initialized on it (motion needs it; teardown will abort).
        let ok = |m: u8, f: &Findings| {
            f.init_attempts.iter().any(|a| a.mask == m && a.status == "ok")
        };
        let chosen = if ok(0b11, &self.findings) {
            Some(0b11)
        } else if ok(0b01, &self.findings) {
            Some(0b01)
        } else {
            None
        };

        if self.initialized {
            self.abort_step("initialize", "release the probe session before selecting the final mask").await;
            self.initialized = false;
        }

        match chosen {
            Some(mask) => {
                // The discovery aborts above can leave the controller latched in
                // a HOLD/abort state that silently swallows later instructions
                // (observed on the simulator; FRC_Reset is the documented way to
                // clear it per B-84184EN/02 §2.4). Reset moves nothing.
                self.s
                    .exchange_labeled("initialize", "reset any HOLD/abort state left by the discovery aborts", SendPacket::Command(Command::FrcReset), Expect::Command("FRC_Reset"), self.t(), Some("pre-final"))
                    .await;
                let st = self
                    .s
                    .exchange_labeled(
                        "initialize",
                        &format!("re-initialize on the selected working mask {mask:#04b}"),
                        SendPacket::Command(Command::FrcInitialize(FrcInitialize::from_bits(mask))),
                        Expect::Command("FRC_Initialize"),
                        self.t(),
                        Some("final"),
                    )
                    .await;
                if st.succeeded() {
                    self.initialized = true;
                    self.findings.session_mask = Some(mask);
                } else {
                    self.note("re-initialize on the selected mask failed; motion will be skipped");
                }
            }
            None => self.note("no GroupMask initialized successfully — motion cannot be attempted"),
        }
    }

    // ---------------------------------------------------------------- phase 3

    /// True when the live RMI session reserved two motion groups.
    ///
    /// Under a two-group session every motion packet must carry two groups'
    /// worth of position data (B-84184EN/03 §2.3.1), so callers must build
    /// multi-group instructions or the controller answers RMIT-040.
    fn two_group_session(&self) -> bool {
        self.findings
            .session_mask
            .map(|m| m.count_ones() > 1)
            .unwrap_or(false)
    }

    pub async fn lifecycle(&mut self) {
        if !self.cli.lifecycle {
            self.s.skip("lifecycle", "FRC_Pause/FRC_Continue/FRC_Reset", "non-motion lifecycle probes", "not requested (pass --lifecycle)");
            return;
        }
        self.s
            .exchange("lifecycle", "pause the controller (no motion queued)", SendPacket::Command(Command::FrcPause), Expect::Command("FRC_Pause"), self.t())
            .await;
        self.s
            .exchange("lifecycle", "resume after pause", SendPacket::Command(Command::FrcContinue), Expect::Command("FRC_Continue"), self.t())
            .await;
        self.s
            .exchange("lifecycle", "reset (clears a HOLD state)", SendPacket::Command(Command::FrcReset), Expect::Command("FRC_Reset"), self.t())
            .await;
        // Re-read status: this is what tells us whether pause/continue/reset
        // left the controller usable.
        self.s
            .exchange_labeled("lifecycle", "status after the lifecycle probes", SendPacket::Command(Command::FrcGetStatus), Expect::Command("FRC_GetStatus"), self.t(), Some("post-lifecycle"))
            .await;
    }

    // ---------------------------------------------------------------- phase 4

    pub async fn register_write(&mut self) {
        let Some(reg) = self.cli.write_register else {
            self.s.skip("registers", "FRC_WriteRegister", "numeric register write + restore", "not requested (pass --write-register <N>)");
            return;
        };

        let before = self
            .s
            .exchange_labeled("registers", &format!("read R[{reg}] before writing"), SendPacket::Command(Command::FrcReadRegister(FrcReadRegister::new(reg))), Expect::Command("FRC_ReadRegister"), self.t(), Some(&format!("R{reg}-before")))
            .await;
        let Some(original) = parse_ok::<FrcReadRegisterResponse>(&before).map(|r| r.register_value) else {
            self.note(format!("R[{reg}] could not be read ({}); refusing to write a register whose original value is unknown", before.status));
            return;
        };

        let probe_value = original + 1.0;
        self.s
            .exchange_labeled("registers", &format!("write R[{reg}] = {probe_value}"), SendPacket::Command(Command::FrcWriteRegister(FrcWriteRegister::new(reg, probe_value))), Expect::Command("FRC_WriteRegister"), self.t(), Some(&format!("R{reg}-write")))
            .await;
        let readback = self
            .s
            .exchange_labeled("registers", &format!("read R[{reg}] back"), SendPacket::Command(Command::FrcReadRegister(FrcReadRegister::new(reg))), Expect::Command("FRC_ReadRegister"), self.t(), Some(&format!("R{reg}-readback")))
            .await;
        let observed = parse_ok::<FrcReadRegisterResponse>(&readback).map(|r| r.register_value);

        // Always put it back.
        self.s
            .exchange_labeled("registers", &format!("restore R[{reg}] = {original}"), SendPacket::Command(Command::FrcWriteRegister(FrcWriteRegister::new(reg, original))), Expect::Command("FRC_WriteRegister"), self.t(), Some(&format!("R{reg}-restore")))
            .await;

        self.findings.register_write = serde_json::to_value(serde_json::json!({
            "register": reg,
            "original": original,
            "written": probe_value,
            "read_back": observed,
            "verified": observed.map(|v| (v - probe_value).abs() < 0.001).unwrap_or(false),
            "restored": true,
        }))
        .ok();
    }

    // ---------------------------------------------------------------- phase 5

    pub async fn motion(&mut self) {
        self.findings.motion.linear_speed_mm_s = self.cli.linear_speed;
        self.findings.motion.rotary_speed_deg_s = self.cli.rotary_speed;

        if !self.cli.motion {
            self.s.skip("motion", "FRC_LinearRelative", "1 mm relative moves on X/Y/Z", "read-only run (pass --motion to enable)");
            self.s.skip("motion", "FRC_JointRelativeJRep", "±1° on the Group-2 positioner", "read-only run (pass --motion to enable)");
            return;
        }

        // ---- preconditions ------------------------------------------------
        let status = self
            .s
            .exchange_labeled("motion", "pre-motion status check", SendPacket::Command(Command::FrcGetStatus), Expect::Command("FRC_GetStatus"), self.t(), Some("pre-motion"))
            .await;
        let Some(st) = parse_ok::<FrcGetStatusResponse>(&status) else {
            self.deny_motion("FRC_GetStatus did not return a readable status; refusing to move");
            return;
        };
        if st.error_id != 0 {
            self.deny_motion(format!("FRC_GetStatus reported error {}", st.error_id));
            return;
        }
        if st.servo_ready != 1 {
            self.deny_motion(format!("servos are not ready (ServoReady={}); refusing to move", st.servo_ready));
            return;
        }
        if st.tp_mode != 0 {
            self.deny_motion("the teach pendant is enabled (TPMode=1); RMI motion requires it disabled");
            return;
        }
        if self.findings.session_mask.is_none() || !self.initialized {
            self.deny_motion("no RMI session is initialized; refusing to move");
            return;
        }

        let start = self
            .s
            .exchange_labeled("motion", "start pose (Group 1) — every move is relative to this", SendPacket::Command(Command::FrcReadCartesianPosition(FrcReadCartesianPosition::new(Some(1)))), Expect::Command("FRC_ReadCartesianPosition"), self.t(), Some("start"))
            .await;
        let Some(start_pose) = parse_ok::<FrcReadCartesianPositionResponse>(&start) else {
            self.deny_motion("could not read the Group-1 start pose; refusing to move");
            return;
        };
        self.findings.motion.attempted = true;
        self.findings.motion.start_position = serde_json::to_value(start_pose.pos).ok();
        // The live Configuration from the controller — UFrame/UTool/front/up/
        // left/flip/turn — is what we echo back on every motion packet, rather
        // than guessing a default.
        let config = start_pose.config.clone();

        // ---- Group 1: LINEAR XYZ ONLY -------------------------------------
        for axis in ["X", "Y", "Z"] {
            let pair_start = match self.read_g1_pose("pair-start").await {
                Some(p) => p,
                None => {
                    self.abort_motion("could not read the pose before the axis pair").await;
                    return;
                }
            };
            for sign in [1.0_f64, -1.0_f64] {
                let delta = LINEAR_STEP_MM * sign;
                let result = self.linear_relative_g1(axis, delta, &config).await;
                let ok = result.passed;
                self.findings.motion.axes.push(result);
                if !ok {
                    self.abort_motion(format!("{axis}{delta:+} mm did not complete as commanded")).await;
                    return;
                }
            }
            // After the pair we must be back where we started.
            if let Some(now) = self.read_g1_pose("pair-end").await {
                let residual = distance(&pair_start.pos, &now.pos);
                if residual > LINEAR_TOLERANCE_MM {
                    self.findings.motion.axes.push(AxisMotionResult {
                        axis: format!("{axis} return"),
                        commanded_mm: 0.0,
                        measured_mm: Some(residual),
                        residual_mm: Some(residual),
                        passed: false,
                        detail: Some("did not return to the pose captured before this axis pair".into()),
                    });
                    self.abort_motion(format!("{axis} pair did not return to its start pose ({residual:.3} mm off)")).await;
                    return;
                }
            }
        }

        // ---- Group 2: ROTARY ±1° ------------------------------------------
        if self.findings.session_mask == Some(0b11) {
            self.group2_rotary().await;
        } else {
            self.s.skip("motion", "FRC_JointRelativeJRep", "±1° on the Group-2 positioner", "the session could not reserve GroupMask 0b11, so Group 2 is not addressable");
        }

        // ---- final proof we are home ---------------------------------------
        if let Some(end) = self.read_g1_pose("end").await {
            let residual = distance(&start_pose.pos, &end.pos);
            self.findings.motion.end_position = serde_json::to_value(end.pos).ok();
            self.findings.motion.return_residual_mm = Some(residual);
            if residual > LINEAR_TOLERANCE_MM {
                self.note(format!("WARNING: final pose is {residual:.3} mm from the start pose"));
            } else {
                self.note(format!("returned to the start pose within {residual:.3} mm"));
            }
        }
    }

    /// FOUNDER DIRECTIVE (2026-07-28): the arm (Group 1) is **never** commanded
    /// in rotation by this tool. W/P/R stay at exactly zero on every Group-1
    /// motion packet, under every flag. This is an enforced refusal — not an
    /// unimplemented feature — so that no future edit can quietly add a wrist
    /// move to a "tiny safe motion" sweep. Rotary motion is permitted only on
    /// Group 2 (the positioner), ±1°.
    fn enforce_no_arm_rotation(delta: &Position) -> Result<(), String> {
        if delta.w != 0.0 || delta.p != 0.0 || delta.r != 0.0 {
            return Err(format!(
                "REFUSED: Group-1 rotary motion is forbidden by founder directive (W={}, P={}, R={} must all be 0)",
                delta.w, delta.p, delta.r
            ));
        }
        Ok(())
    }

    async fn linear_relative_g1(
        &mut self,
        axis: &str,
        delta_mm: f64,
        config: &Configuration,
    ) -> AxisMotionResult {
        let mut result = AxisMotionResult {
            axis: axis.to_string(),
            commanded_mm: delta_mm,
            ..Default::default()
        };

        let mut delta = Position::default();
        match axis {
            "X" => delta.x = delta_mm,
            "Y" => delta.y = delta_mm,
            "Z" => delta.z = delta_mm,
            other => {
                result.detail = Some(format!("REFUSED: unknown / non-linear axis {other}"));
                return result;
            }
        }
        if let Err(e) = Self::enforce_no_arm_rotation(&delta) {
            result.detail = Some(e);
            return result;
        }

        let before = match self.read_g1_pose(&format!("before-{axis}{delta_mm:+}")).await {
            Some(p) => p,
            None => {
                result.detail = Some("could not read the pose before the move".into());
                return result;
            }
        };

        // The session is reserved on the working mask (0b11 when a positioner
        // exists), and B-84184EN/03 §2.3.1 is explicit: with two non-zero
        // GroupMask bits, EVERY motion packet must carry two sets of position
        // data. A single-group FRC_LinearRelative under a two-group session is
        // rejected with RMIT-040 Invalid Group Mask — observed live on COMET1.
        //
        // So mirror what the positioner moves already do, inverted: command the
        // arm and hold Group 2 at zero delta.
        let instr = if self.two_group_session() {
            FrcLinearRelative::with_groups(
                0,
                CartesianGroups::arm_and_group2(
                    config.clone(),
                    delta,
                    GroupBlock::joint(JointAngles::default()),
                ),
                SpeedType::MMSec,
                self.cli.linear_speed,
                TermType::FINE,
                0,
            )
        } else {
            FrcLinearRelative::new(
                0, // the driver assigns the sequence id at dispatch
                config.clone(),
                delta,
                SpeedType::MMSec,
                self.cli.linear_speed,
                TermType::FINE,
                0,
            )
        };
        let st = self
            .s
            .exchange_labeled(
                "motion",
                &format!("Group 1 linear relative {axis} {delta_mm:+} mm @ {} mm/s", self.cli.linear_speed),
                SendPacket::Instruction(Instruction::FrcLinearRelative(instr)),
                Expect::Instruction("FRC_LinearRelative"),
                self.motion_t(),
                Some(&format!("{}{}", axis, if delta_mm > 0.0 { "plus" } else { "minus" })),
            )
            .await;
        if !st.succeeded() {
            result.detail = Some(format!(
                "instruction status `{}`{}",
                st.status,
                st.error_text.map(|e| format!(" ({e})")).unwrap_or_default()
            ));
            return result;
        }

        let after = match self.read_g1_pose(&format!("after-{axis}{delta_mm:+}")).await {
            Some(p) => p,
            None => {
                result.detail = Some("move reported complete but the pose could not be re-read".into());
                return result;
            }
        };
        let moved = match axis {
            "X" => after.pos.x - before.pos.x,
            "Y" => after.pos.y - before.pos.y,
            _ => after.pos.z - before.pos.z,
        };
        let residual = (moved - delta_mm).abs();
        result.measured_mm = Some(moved);
        result.residual_mm = Some(residual);
        result.passed = residual <= LINEAR_TOLERANCE_MM;
        if !result.passed {
            result.detail = Some(format!(
                "commanded {delta_mm:+.3} mm, measured {moved:+.3} mm (off by {residual:.3} mm)"
            ));
        }
        result
    }

    async fn group2_rotary(&mut self) {
        let before = self
            .s
            .exchange_labeled("motion", "Group-2 joint angles before rotating", SendPacket::Command(Command::FrcReadJointAngles(FrcReadJointAngles::new(Some(2)))), Expect::Command("FRC_ReadJointAngles"), self.t(), Some("g2-before"))
            .await;
        let Some(start) = parse_ok::<FrcReadJointAnglesResponse>(&before) else {
            self.s.skip("motion", "FRC_JointRelativeJRep", "±1° on the Group-2 positioner", "Group-2 joint angles are not readable");
            return;
        };

        // A Group-2 move is sent as a two-group packet whose G1 block is all
        // zeros ("hold the arm"). A controller that reads that block as an
        // ABSOLUTE joint target instead of a relative delta would drive the arm
        // to its zero pose. We cannot stop the first packet, but we can refuse to
        // send a second one — so snapshot the arm's joints and re-check after
        // every positioner move.
        let arm_before = self.read_g1_joints("g2-arm-before").await;
        if arm_before.is_none() {
            self.note("Group-1 joint angles are unreadable, so arm immobility cannot be verified during the positioner test — skipping the Group-2 rotary moves");
            self.s.skip("motion", "FRC_JointRelativeJRep", "±1° on the Group-2 positioner", "cannot verify the arm stays put without Group-1 joint reads");
            return;
        }

        for sign in [1.0_f32, -1.0_f32] {
            let delta = ROTARY_STEP_DEG * sign;
            let mut result = AxisMotionResult {
                axis: format!("G2 J1 {delta:+}°"),
                commanded_mm: delta as f64,
                ..Default::default()
            };

            // G1 block is all zeros: the arm must not move at all during the
            // positioner test. G2 carries the single-axis delta.
            let g1_hold = JointAngles::default();
            let g2_delta = JointAngles { j1: delta, ..Default::default() };
            let instr = FrcJointRelativeJRep::with_groups(
                0,
                JointGroups::arm_and_group2(g1_hold, GroupBlock::joint(g2_delta)),
                // Speed is deliberately 0.5: safe under every plausible unit
                // reading of SpeedType on a JRep packet (0.5 mm/s, 0.5 °/s, or
                // 0.5 % of max) — see the report's caveat.
                SpeedType::MMSec,
                self.cli.rotary_speed,
                TermType::FINE,
                0,
            );
            let st = self
                .s
                .exchange_labeled(
                    "motion",
                    &format!("Group 2 rotary J1 {delta:+}° @ {} deg/s (G1 block held at zero)", self.cli.rotary_speed),
                    SendPacket::Instruction(Instruction::FrcJointRelativeJRep(instr)),
                    Expect::Instruction("FRC_JointRelativeJRep"),
                    self.motion_t(),
                    Some(if sign > 0.0 { "g2-plus" } else { "g2-minus" }),
                )
                .await;

            if !st.succeeded() {
                result.detail = Some(format!(
                    "instruction status `{}`{}",
                    st.status,
                    st.error_text.clone().map(|e| format!(" ({e})")).unwrap_or_default()
                ));
                self.findings.motion.group2.push(result);
                self.abort_motion("the Group-2 rotary move did not complete").await;
                return;
            }

            // Did the arm hold? Check this BEFORE anything else, and stop dead if
            // it did not — no further motion packets of any kind.
            if let (Some(a0), Some(a1)) = (
                arm_before.as_ref(),
                self.read_g1_joints(if sign > 0.0 { "g2-arm-after-plus" } else { "g2-arm-after-minus" }).await,
            ) {
                if let Some(drift) = arm_drift(&a0.joint_angles, &a1.joint_angles) {
                    let msg = format!(
                        "THE ARM MOVED during a Group-2 packet whose G1 block was all zeros ({drift}). \
                         This controller reads the G1 hold block as an ABSOLUTE joint target, not a \
                         relative delta — coordinated two-group packets are NOT safe to send this way. \
                         Stopping immediately."
                    );
                    result.detail = Some(msg.clone());
                    self.findings.motion.group2_arm_moved = Some(msg.clone());
                    self.findings.motion.group2.push(result);
                    self.abort_motion(msg).await;
                    return;
                }
            }

            let after = self
                .s
                .exchange_labeled("motion", "Group-2 joint angles after rotating", SendPacket::Command(Command::FrcReadJointAngles(FrcReadJointAngles::new(Some(2)))), Expect::Command("FRC_ReadJointAngles"), self.t(), Some(if sign > 0.0 { "g2-after-plus" } else { "g2-after-minus" }))
                .await;
            if let Some(now) = parse_ok::<FrcReadJointAnglesResponse>(&after) {
                let moved = now.joint_angles.j1 - start.joint_angles.j1;
                let expected = if sign > 0.0 { delta } else { 0.0 };
                let residual = (moved - expected).abs();
                result.measured_mm = Some(moved as f64);
                result.residual_mm = Some(residual as f64);
                result.passed = residual <= ROTARY_TOLERANCE_DEG;
                if !result.passed {
                    result.detail = Some(format!(
                        "expected J1 to sit {expected:+.3}° from its start, measured {moved:+.3}°"
                    ));
                }
            } else {
                result.detail = Some("Group-2 joint angles could not be re-read after the move".into());
            }
            let ok = result.passed;
            self.findings.motion.group2.push(result);
            if !ok {
                self.abort_motion("the Group-2 rotary move did not land where commanded").await;
                return;
            }
        }
    }

    async fn read_g1_joints(&self, tag: &str) -> Option<FrcReadJointAnglesResponse> {
        let st = self
            .s
            .exchange_labeled("motion", "read Group-1 joint angles (arm-hold check)", SendPacket::Command(Command::FrcReadJointAngles(FrcReadJointAngles::new(Some(1)))), Expect::Command("FRC_ReadJointAngles"), self.t(), Some(tag))
            .await;
        parse_ok::<FrcReadJointAnglesResponse>(&st)
    }

    async fn read_g1_pose(&self, tag: &str) -> Option<FrcReadCartesianPositionResponse> {
        let st = self
            .s
            .exchange_labeled("motion", "read Group-1 Cartesian pose", SendPacket::Command(Command::FrcReadCartesianPosition(FrcReadCartesianPosition::new(Some(1)))), Expect::Command("FRC_ReadCartesianPosition"), self.t(), Some(tag))
            .await;
        parse_ok::<FrcReadCartesianPositionResponse>(&st)
    }

    fn deny_motion(&mut self, reason: impl Into<String>) {
        let reason = reason.into();
        self.s.skip("motion", "FRC_LinearRelative", "1 mm relative moves on X/Y/Z", &reason);
        self.findings.motion.aborted = true;
        self.findings.motion.abort_reason = Some(reason);
    }

    /// Stop cleanly the moment anything is off: abort the running RMI program,
    /// record why, and do not issue another motion packet.
    async fn abort_motion(&mut self, reason: impl Into<String>) {
        let reason = reason.into();
        println!("        !! aborting motion: {reason}");
        self.findings.motion.aborted = true;
        self.findings.motion.abort_reason = Some(reason);
        self.abort_step("motion", "emergency abort after a failed motion step").await;
        self.initialized = false;
    }

    async fn abort_step(&self, phase: &str, description: &str) -> StepRecord {
        self.s
            .exchange_labeled(phase, description, SendPacket::Command(Command::FrcAbort), Expect::Command("FRC_Abort"), self.t(), Some("abort"))
            .await
    }

    // ---------------------------------------------------------------- phase 6

    pub async fn tp_program(&mut self) {
        let Some(name) = self.cli.run_tp_program.clone() else {
            self.s.skip("tp-program", "FRC_Call", "run a controller-resident TP program", "not requested (pass --run-tp-program <NAME>)");
            return;
        };
        if !self.cli.motion {
            self.s.skip("tp-program", "FRC_Call", &format!("run TP program {name}"), "--run-tp-program requires --motion: a TP program can move the robot");
            return;
        }
        self.s
            .exchange("tp-program", &format!("run controller-resident TP program {name}"), SendPacket::Instruction(Instruction::FrcCall(fanuc_rmi::instructions::FrcCall::new(0, name))), Expect::Instruction("FRC_Call"), self.motion_t())
            .await;
    }

    pub async fn teardown(&mut self) {
        if self.initialized {
            self.abort_step("teardown", "release the RMI session we reserved").await;
            self.initialized = false;
        }
        self.s
            .exchange("teardown", "close the RMI connection", SendPacket::Communication(Communication::FrcDisconnect), Expect::Communication("FRC_Disconnect"), self.t())
            .await;
    }
}

/// Deserialize a step's raw response into a typed `fanuc_rmi` struct.
///
/// Several response structs carry `#[serde(default)]` fields, so a controller's
/// `{"Command":"Unknown","ErrorID":…}` reply happily parses into a struct full of
/// zeros. Anything that makes a DECISION from a response must therefore use
/// [`parse_ok`], which refuses unless the exchange actually succeeded.
fn parse<T: DeserializeOwned>(step: &StepRecord) -> Option<T> {
    serde_json::from_str::<T>(step.received_raw.as_deref()?).ok()
}

/// [`parse`], but only for an exchange that succeeded (`ErrorID` 0 and the
/// expected response type).
fn parse_ok<T: DeserializeOwned>(step: &StepRecord) -> Option<T> {
    if !step.succeeded() {
        return None;
    }
    parse(step)
}

fn dedup2(a: i8, b: i8) -> Vec<i8> {
    if a == b { vec![a] } else { vec![a, b] }
}

/// Describes how far the arm's joints moved, or `None` if it held within
/// [`ARM_HOLD_TOLERANCE_DEG`].
fn arm_drift(before: &JointAngles, after: &JointAngles) -> Option<String> {
    let pairs = [
        ("J1", before.j1, after.j1),
        ("J2", before.j2, after.j2),
        ("J3", before.j3, after.j3),
        ("J4", before.j4, after.j4),
        ("J5", before.j5, after.j5),
        ("J6", before.j6, after.j6),
    ];
    let moved: Vec<String> = pairs
        .iter()
        .filter(|(_, b, a)| (a - b).abs() > ARM_HOLD_TOLERANCE_DEG)
        .map(|(n, b, a)| format!("{n} {b:+.3}° -> {a:+.3}°"))
        .collect();
    if moved.is_empty() {
        None
    } else {
        Some(moved.join(", "))
    }
}

fn distance(a: &Position, b: &Position) -> f64 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}
