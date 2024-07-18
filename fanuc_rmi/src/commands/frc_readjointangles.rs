use serde::{Deserialize, Serialize};
use crate::JointAngles;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadJointAngles{
    #[serde(rename = "Group")]
    group: u8,
}

impl FrcReadJointAngles{
    #[allow(unused)]
    fn new(groupentered: Option<u8>) -> Self {
        Self {
            group: match groupentered {
                Some(gm) => gm,
                None => 1
            },
        }
    }
}

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