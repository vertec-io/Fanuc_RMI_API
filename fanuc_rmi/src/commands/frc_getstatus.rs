use serde::{Deserialize, Serialize};

/// State of the RMI_MOVE TP program, per B-84184EN/03 §2.3.7.
///
/// The raw wire value is easy to misread — `0` is *Running*, not idle — so
/// prefer [`FrcGetStatusResponse::program_state`] over comparing the integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramState {
    Running,
    Paused,
    Aborted,
    /// A value the manual does not define.
    Unknown(i8),
}

impl ProgramState {
    pub fn from_raw(raw: i8) -> Self {
        match raw {
            0 => Self::Running,
            1 => Self::Paused,
            2 => Self::Aborted,
            other => Self::Unknown(other),
        }
    }
}

impl std::fmt::Display for ProgramState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => f.write_str("running"),
            Self::Paused => f.write_str("paused"),
            Self::Aborted => f.write_str("aborted"),
            Self::Unknown(v) => write!(f, "unknown({v})"),
        }
    }
}

/// Whether the controller is in a state where `FRC_Initialize` can succeed, and
/// if not, why — so callers surface a cause instead of a bare error id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RmiReadiness {
    /// Every precondition in B-84184EN/03 §2.3.1 is satisfied.
    Ready,
    /// The RMI interface is already up; initializing again is not needed.
    AlreadyRunning,
    /// The teach pendant is enabled. RMI only runs while it is disabled.
    TeachPendantEnabled,
    /// Servo errors present. `FRC_Reset` may clear them.
    NotReady,
    /// The RMI interface is down but an RMI_MOVE program still holds program
    /// control. `FRC_Initialize` will return `7015 Program already exists`, and
    /// **no RMI command can clear this** — not `FRC_Abort` (it fails with
    /// `RMIT-014` because this session has no running RMI to abort), not
    /// `FRC_Reset`, and not a pendant ABORT (§2.3.1 says so explicitly).
    ///
    /// Caused by an earlier session that initialized successfully and then went
    /// away without sending `FRC_Abort` or `FRC_Disconnect`.
    OrphanedProgram { state: ProgramState },
}

impl RmiReadiness {
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Operator-facing explanation, including the remedy where one exists.
    pub fn explain(&self) -> &'static str {
        match self {
            Self::Ready => "Controller is ready to initialize RMI.",
            Self::AlreadyRunning => "The RMI interface is already running.",
            Self::TeachPendantEnabled => {
                "The teach pendant is enabled. RMI only runs while the pendant is disabled \
                 — set the pendant switch to OFF."
            }
            Self::NotReady => {
                "The controller is not ready for motion (servo or other errors). Clear the \
                 fault on the pendant, or send a reset, then retry."
            }
            Self::OrphanedProgram { .. } => {
                "An earlier RMI session ended without aborting, so the RMI_MOVE program still \
                 holds program control on the controller. Initialize will be rejected with \
                 'Program already exists' (7015). Aborting from the teach pendant does not \
                 release it; the controller needs its program control cleared."
            }
        }
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcGetStatusResponse {
    // #[serde(rename = "Command", default)]
    // pub command: Command,
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    /// `1` = the controller is ready for motion. Anything else is an error
    /// condition; `FRC_Reset` may clear it.
    ///
    /// Every field below is only meaningful when `error_id == 0`. On a non-zero
    /// `ErrorID` the manual states all other values in the packet are invalid —
    /// a wedged controller really does return values like `-4` and `-53` here.
    #[serde(rename = "ServoReady", default)]
    pub servo_ready: i8,
    /// Teach-pendant enable state: **0 = the pendant is disabled, 1 = enabled**.
    /// RMI only works while the pendant is DISABLED, so `0` is what a healthy
    /// remote session wants. (An earlier annotation had this backwards.)
    #[serde(rename = "TPMode", default)]
    pub tp_mode: i8,
    /// State of the Remote Motion **interface**: `1` = running, `0` = not
    /// running and `FRC_Initialize` may be sent to (re)start it.
    ///
    /// This is NOT the same as [`Self::program_status`], and the two can
    /// disagree: `rmi_motion_status == 0` with `program_status == 0` means the
    /// interface is down while an orphaned RMI_MOVE program is still running.
    /// That combination is what produces `7015 MEMO-015 Program already exists`
    /// on the next `FRC_Initialize`, and no RMI command can clear it.
    #[serde(rename = "RMIMotionStatus", default)]
    pub rmi_motion_status: i8,
    /// The RMI_MOVE TP program's state: **0 = Running, 1 = Paused,
    /// 2 = Aborted**.
    ///
    /// Note the polarity: `0` does NOT mean idle. An earlier annotation here
    /// read "1 = aborted", which inverted the meaning of the healthy value and
    /// hid a live 7015 diagnosis — the controller was reporting RMI_MOVE as
    /// *running* the whole time.
    #[serde(rename = "ProgramStatus", default)]
    pub program_status: i8,
    #[serde(rename = "SingleStepMode", default)]
    pub single_step_mode: i8,
    /// **Count** of user tools available on the controller — not the active
    /// UTool number.
    #[serde(rename = "NumberUTool", default)]
    pub number_utool: i8,
    /// **Count** of user frames available on the controller — not the active
    /// UFrame number.
    #[serde(rename = "NumberUFrame", default)]
    pub number_uframe: i8,
    #[serde(rename = "NextSequenceID", default)]
    pub next_sequence_id: u32,
    // Not in B-84184EN_02 docs, but Robot CRX-30iA returns it.
    #[serde(rename = "Override", default)]
    pub override_value: u32,
}

impl FrcGetStatusResponse {
    /// The RMI_MOVE program's state as a named value rather than a raw int.
    pub fn program_state(&self) -> ProgramState {
        ProgramState::from_raw(self.program_status)
    }

    /// True when the Remote Motion **interface** is running (`RMIMotionStatus == 1`).
    pub fn rmi_interface_running(&self) -> bool {
        self.rmi_motion_status == 1
    }

    /// True when the teach pendant is disabled, which is what RMI requires.
    pub fn teach_pendant_disabled(&self) -> bool {
        self.tp_mode == 0
    }

    /// Classify the controller against the §2.3.1 preconditions.
    ///
    /// Order matters: an unusable field combination is reported before the
    /// merely-not-ready ones, so callers surface the most specific cause.
    pub fn readiness(&self) -> RmiReadiness {
        if self.error_id != 0 {
            // Every other field is documented invalid in this case.
            return RmiReadiness::NotReady;
        }
        if !self.teach_pendant_disabled() {
            return RmiReadiness::TeachPendantEnabled;
        }
        if self.servo_ready != 1 {
            return RmiReadiness::NotReady;
        }
        if self.rmi_interface_running() {
            return RmiReadiness::AlreadyRunning;
        }
        // Interface down, but the program is not aborted: an earlier session
        // left RMI_MOVE holding program control. This is the 7015 trap.
        match self.program_state() {
            ProgramState::Aborted => RmiReadiness::Ready,
            state => RmiReadiness::OrphanedProgram { state },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(tp: i8, servo: i8, rmi: i8, prog: i8) -> FrcGetStatusResponse {
        FrcGetStatusResponse {
            error_id: 0,
            servo_ready: servo,
            tp_mode: tp,
            rmi_motion_status: rmi,
            program_status: prog,
            single_step_mode: 0,
            number_utool: 10,
            number_uframe: 9,
            next_sequence_id: 1,
            override_value: 10,
        }
    }

    /// Polarity guard. `0` is Running, NOT idle — an inverted annotation here
    /// hid a live 7015 diagnosis on an R-30iB.
    #[test]
    fn program_state_polarity_matches_the_manual() {
        assert_eq!(ProgramState::from_raw(0), ProgramState::Running);
        assert_eq!(ProgramState::from_raw(1), ProgramState::Paused);
        assert_eq!(ProgramState::from_raw(2), ProgramState::Aborted);
    }

    /// The exact frame COMET1 returned while rejecting every FRC_Initialize.
    #[test]
    fn orphaned_program_is_detected_from_the_real_comet1_frame() {
        let s = status(0, 1, 0, 0);
        assert_eq!(
            s.readiness(),
            RmiReadiness::OrphanedProgram { state: ProgramState::Running }
        );
        assert!(!s.readiness().is_ready());
    }

    #[test]
    fn aborted_program_with_interface_down_is_ready() {
        assert_eq!(status(0, 1, 0, 2).readiness(), RmiReadiness::Ready);
    }

    #[test]
    fn enabled_pendant_is_reported_before_anything_else() {
        assert_eq!(status(1, 1, 0, 2).readiness(), RmiReadiness::TeachPendantEnabled);
    }

    #[test]
    fn running_interface_is_not_an_error() {
        assert_eq!(status(0, 1, 1, 0).readiness(), RmiReadiness::AlreadyRunning);
    }
}