use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcGetStatusResponse {
    // #[serde(rename = "Command")]
    // pub command: Command,
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "ServoReady")]
    pub servo_ready: i8,
    #[serde(rename = "TPMode")]
    pub tp_mode: i8,
    #[serde(rename = "RMIMotionStatus")]
    pub rmi_motion_status: i8,
    #[serde(rename = "ProgramStatus")]
    pub program_status: i8,
    #[serde(rename = "SingleStepMode")]
    pub single_step_mode: i8,
    #[serde(rename = "NumberUTool")]
    pub number_utool: i8,
    #[serde(rename = "NumberUFrame")]
    pub number_uframe: i8,
    #[serde(rename = "NextSequenceID")]
    pub next_sequence_id: u32,
    // Not in B-84184EN_02 docs, but Robot CRX-30iA returns it. 
    #[serde(rename = "Override")]
    pub override_value: u32,
}