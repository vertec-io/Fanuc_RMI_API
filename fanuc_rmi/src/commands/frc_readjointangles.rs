use serde::{Deserialize, Serialize};
use crate::JointAngles;

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadJointAngles{
    #[serde(rename = "Group")]
    group: u8,
}

impl FrcReadJointAngles{
    #[allow(unused)]
    pub fn new(group: Option<u8>) -> Self {
        Self {
            group: match group {
                Some(gm) => gm,
                None => 1
            },
        }
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadJointAnglesResponse {    
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "TimeTag")]
    pub time_tag: i16,
    #[serde(rename = "JointAngles")]
    pub joint_angles: JointAngles,
    #[serde(rename = "Group")]
    pub group: u8,
}