use serde::{Deserialize, Deserializer, Serialize};
use crate::JointAngles;

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcReadJointAngles{
    #[serde(rename = "Group")]
    pub group: u8,
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

/// Helper to deserialize JointAngles from either "JointAngle" or "JointAngles" field
/// Real FANUC robots return "JointAngle" (singular) but our code expected "JointAngles" (plural)
fn deserialize_joint_angles<'de, D>(deserializer: D) -> Result<JointAngles, D::Error>
where
    D: Deserializer<'de>,
{
    JointAngles::deserialize(deserializer)
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadJointAnglesResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "TimeTag")]
    pub time_tag: u32,
    /// Joint angles - accepts both "JointAngle" (real robot) and "JointAngles" (simulator)
    #[serde(alias = "JointAngle", rename = "JointAngles", deserialize_with = "deserialize_joint_angles")]
    pub joint_angles: JointAngles,
    #[serde(rename = "Group")]
    pub group: u8,
}