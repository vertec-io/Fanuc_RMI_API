use serde::{Deserialize, Serialize};
use crate::{JointAngles, SpeedType, TermType};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcJointMotionJRep {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,
    #[serde(rename = "JointAngles")]
    pub joint_angles: JointAngles,
    #[serde(rename = "SpeedType")]
    pub speed_type: SpeedType,
    #[serde(rename = "Speed")]
    pub speed: f64,
    #[serde(rename = "TermType")]
    pub term_type: TermType,
    #[serde(rename = "TermValue")]
    pub term_value: u8,
    /// Optional, **UNTESTED** motion-group selector. Omitted from the wire when
    /// `None` (the default) — byte-identical to the documented single-group packet.
    ///
    /// The RMI motion protocol is documented single-group (Group 1) and its motion
    /// packets carry no group field, so **setting this does NOT produce coordinated
    /// positioner (Group 2) motion** and may simply be ignored by the controller. It
    /// exists only so the wire format can express a group selector for experimentation.
    /// For real coordinated motion, drive a controller-resident TP/COORD program via
    /// [`crate::instructions::FrcCall`]. The `GroupMask` field name here is a best guess.
    #[serde(rename = "GroupMask", default, skip_serializing_if = "Option::is_none")]
    pub group: Option<crate::GroupMask>,
}


impl FrcJointMotionJRep{
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
            group: None,
        }

    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcJointMotionJRepResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID", default)]
    pub sequence_id: u32,
}


impl FrcJointMotionJRep {
    /// Attach an **UNTESTED** motion-group selector (see the `group` field).
    /// Does not by itself produce coordinated Group-2 motion; the RMI motion
    /// protocol is single-group. Provided so callers can express intent on the wire.
    pub fn with_group(mut self, group: crate::GroupMask) -> Self {
        self.group = Some(group);
        self
    }
}
