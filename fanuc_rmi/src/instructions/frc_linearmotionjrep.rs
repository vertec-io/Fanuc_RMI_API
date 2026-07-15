use serde::{Deserialize, Serialize};

use crate::instructions::{GroupBlock, JointGroups};
use crate::packets::OnOff;
use crate::JointAngles;
use crate::{SpeedType, TermType};

/// `FRC_LinearMotionJRep` — add a linear motion instruction in joint
/// representation (Operators Manual §2.4.15 single-group, §2.4.15.1 two-group).
///
/// The target payload is carried by [`JointGroups`], which serializes to the
/// flat single-group form (`JointAngle` at top level) or the wrapped multi-group
/// form (`G1`/`G2` …). For an arm + positioner, build the groups with
/// [`JointGroups::arm_and_group2`] and set [`coord`] to [`OnOff::ON`] for
/// coordinated motion.
///
/// [`coord`]: FrcLinearMotionJRep::coord
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcLinearMotionJRep {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,

    /// The motion target(s): single group (flat `JointAngle`) or multiple
    /// groups (`G<n>`).
    #[serde(flatten)]
    pub groups: JointGroups,

    #[serde(rename = "SpeedType")]
    pub speed_type: SpeedType,
    #[serde(rename = "Speed")]
    pub speed: f64,
    #[serde(rename = "TermType")]
    pub term_type: TermType,
    #[serde(rename = "TermValue")]
    pub term_value: u8,

    // ---- Optional keys (§2.4.15.1). All omitted from the wire when `None`. ----
    /// Coordinated motion between groups. Two-group only; requires the
    /// controller's `RMI_MOVE` TP program to be configured for 2 groups.
    #[serde(rename = "COORD", default, skip_serializing_if = "Option::is_none")]
    pub coord: Option<OnOff>,
    #[serde(rename = "ACC", default, skip_serializing_if = "Option::is_none")]
    pub acc: Option<u8>,
    #[serde(rename = "OffsetPRNumber", default, skip_serializing_if = "Option::is_none")]
    pub offset_pr_number: Option<i16>,
    #[serde(rename = "VisionPRNumber", default, skip_serializing_if = "Option::is_none")]
    pub vision_pr_number: Option<i16>,
    #[serde(rename = "WristJoint", default, skip_serializing_if = "Option::is_none")]
    pub wrist_joint: Option<OnOff>,
    #[serde(rename = "MROT", default, skip_serializing_if = "Option::is_none")]
    pub mrot: Option<OnOff>,
    /// Linear ALIM (single-group per manual §2.4.15).
    #[serde(rename = "ALIM", default, skip_serializing_if = "Option::is_none")]
    pub alim: Option<u8>,
    #[serde(rename = "ALIMREG", default, skip_serializing_if = "Option::is_none")]
    pub alim_reg: Option<i16>,
    #[serde(rename = "LCBType", default, skip_serializing_if = "Option::is_none")]
    pub lcb_type: Option<String>,
    #[serde(rename = "LCBValue", default, skip_serializing_if = "Option::is_none")]
    pub lcb_value: Option<i16>,
    #[serde(rename = "PortType", default, skip_serializing_if = "Option::is_none")]
    pub port_type: Option<u8>,
    #[serde(rename = "PortNumber", default, skip_serializing_if = "Option::is_none")]
    pub port_number: Option<i16>,
    #[serde(rename = "PortValue", default, skip_serializing_if = "Option::is_none")]
    pub port_value: Option<String>,
    #[serde(rename = "ToolOffsetPRNumber", default, skip_serializing_if = "Option::is_none")]
    pub tool_offset_pr_number: Option<i16>,
    #[serde(rename = "NoBlend", default, skip_serializing_if = "Option::is_none")]
    pub no_blend: Option<OnOff>,
}

impl FrcLinearMotionJRep {
    /// Single-group (Group 1) joint-representation linear motion.
    /// Signature-compatible with the pre-0.6 constructor: builds the flat
    /// single-group form, all options unset.
    pub fn new(
        sequence_id: u32,
        joint_angles: JointAngles,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        Self::single(sequence_id, joint_angles, speed_type, speed, term_type, term_value)
    }

    /// Single-group (Group 1) joint-representation linear motion (explicit name
    /// for [`new`]).
    ///
    /// [`new`]: FrcLinearMotionJRep::new
    pub fn single(
        sequence_id: u32,
        joint_angles: JointAngles,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        Self {
            sequence_id,
            groups: JointGroups::single(joint_angles),
            speed_type,
            speed,
            term_type,
            term_value,
            coord: None,
            acc: None,
            offset_pr_number: None,
            vision_pr_number: None,
            wrist_joint: None,
            mrot: None,
            alim: None,
            alim_reg: None,
            lcb_type: None,
            lcb_value: None,
            port_type: None,
            port_number: None,
            port_value: None,
            tool_offset_pr_number: None,
            no_blend: None,
        }
    }

    /// Multi-group joint-representation linear motion with an explicit
    /// [`JointGroups`] payload.
    pub fn with_groups(
        sequence_id: u32,
        groups: JointGroups,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        Self {
            sequence_id,
            groups,
            speed_type,
            speed,
            term_type,
            term_value,
            coord: None,
            acc: None,
            offset_pr_number: None,
            vision_pr_number: None,
            wrist_joint: None,
            mrot: None,
            alim: None,
            alim_reg: None,
            lcb_type: None,
            lcb_value: None,
            port_type: None,
            port_number: None,
            port_value: None,
            tool_offset_pr_number: None,
            no_blend: None,
        }
    }

    /// Coordinated arm (Group 1, joint) + Group 2 (e.g. a positioner) motion.
    /// Sets `COORD=ON`.
    pub fn coordinated(
        sequence_id: u32,
        g1_joint: JointAngles,
        group2: GroupBlock,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        let mut me = Self::with_groups(
            sequence_id,
            JointGroups::arm_and_group2(g1_joint, group2),
            speed_type,
            speed,
            term_type,
            term_value,
        );
        me.coord = Some(OnOff::ON);
        me
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcLinearMotionJRepResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID", default)]
    pub sequence_id: u32,
}
