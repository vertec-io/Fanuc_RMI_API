use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcGetStatusResponse {
    // #[serde(rename = "Command", default)]
    // pub command: Command,
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "ServoReady", default)]
    pub servo_ready: i8,
    /// Teach-pendant enable state, per FANUC B-84184EN/02: **0 = the teach
    /// pendant is disabled, 1 = it is enabled**. RMI only works while the
    /// pendant is DISABLED, so `0` is the value a healthy remote session wants.
    ///
    /// (An earlier annotation here read "1 = AUTO mode, 0 = manual", which is
    /// backwards and cost real debugging time on an R-30iB. The authority is
    /// the manual and [`FanucDriver::startup_sequence`], which gates on
    /// `tp_mode != 0`.)
    #[serde(rename = "TPMode", default)]
    pub tp_mode: i8,
    #[serde(rename = "RMIMotionStatus", default)]
    pub rmi_motion_status: i8,
    #[serde(rename = "ProgramStatus", default)]
    pub program_status: i8,
    #[serde(rename = "SingleStepMode", default)]
    pub single_step_mode: i8,
    #[serde(rename = "NumberUTool", default)]
    pub number_utool: i8,
    #[serde(rename = "NumberUFrame", default)]
    pub number_uframe: i8,
    #[serde(rename = "NextSequenceID", default)]
    pub next_sequence_id: u32,
    // Not in B-84184EN_02 docs, but Robot CRX-30iA returns it. 
    #[serde(rename = "Override", default)]
    pub override_value: u32,
}