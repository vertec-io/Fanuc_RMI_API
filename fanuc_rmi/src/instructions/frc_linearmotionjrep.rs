use serde::{Deserialize, Serialize};
use crate::{JointAngles, SpeedType, TermType};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcLinearMotionJRep {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
    #[serde(rename = "JointAngles")]
    pub joint_angles: JointAngles,

    //may need to remove speedtype, it is not included in documentation but seems neccesary may be a typo may not
    #[serde(rename = "SpeedType")]
    pub speed_type: SpeedType,
    #[serde(rename = "Speed")]
    pub speed: f64,
    #[serde(rename = "TermType")]
    pub term_type: TermType,
    #[serde(rename = "TermValue")]
    pub term_value: u8,
}


impl FrcLinearMotionJRep{
    pub fn new(
        sequence_id: u32,
        joint_angles: JointAngles,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,

    ) -> Self {
        Self {
            sequence_id,
            joint_angles,
            speed_type,
            speed,
            term_type,
            term_value,
        }

    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcLinearMotionJRepResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
}