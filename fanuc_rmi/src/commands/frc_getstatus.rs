use serde::{Deserialize, Serialize};

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