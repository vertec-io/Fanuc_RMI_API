use serde::{Deserialize, Serialize};

use crate::instructions::{CartesianGroups, GroupBlock};
use crate::packets::OnOff;
use crate::{Configuration, Position, SpeedType, TermType};

/// `FRC_JointRelative` — add an incremental (relative) joint motion instruction
/// (Operators Manual §2.4.10 single-group, §2.4.10.1 two-group).
///
/// Like [`FrcJointMotion`] but with incremental Cartesian targets: the position
/// payload is carried by [`CartesianGroups`], which serializes to the flat
/// single-group form (`Configuration`/`Position` at top level) or the wrapped
/// multi-group form (`G1`/`G2` …). For an arm + positioner, build the groups
/// with [`CartesianGroups::arm_and_group2`].
///
/// [`FrcJointMotion`]: crate::instructions::FrcJointMotion
/// [`coord`]: FrcJointRelative::coord
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcJointRelative {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,

    /// The motion target(s): single group (flat) or multiple groups (`G<n>`).
    #[serde(flatten)]
    pub groups: CartesianGroups,

    #[serde(rename = "SpeedType")]
    pub speed_type: SpeedType,
    #[serde(rename = "Speed")]
    pub speed: f64,
    #[serde(rename = "TermType")]
    pub term_type: TermType,
    #[serde(rename = "TermValue")]
    pub term_value: u8,

    // ---- Optional keys (§2.4.10.1). All omitted from the wire when `None`. ----
    /// Coordinated motion between groups. Two-group only; requires the
    /// controller's `RMI_MOVE` TP program to be configured for 2 groups.
    ///
    /// NOTE: the controller rejects coordinated joint motion with error
    /// `RMIT-041` ("Joint motion with COORD"). Prefer a Cartesian instruction
    /// (e.g. [`FrcLinearMotion`]) for coordinated arm + positioner moves.
    ///
    /// [`FrcLinearMotion`]: crate::instructions::FrcLinearMotion
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

impl FrcJointRelative {
    /// Single-group (Group 1) relative joint motion. Signature-compatible with
    /// the pre-0.6 constructor: builds the flat single-group form, all options unset.
    pub fn new(
        sequence_id: u32,
        configuration: Configuration,
        position: Position,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        Self::single(sequence_id, configuration, position, speed_type, speed, term_type, term_value)
    }

    /// Single-group (Group 1) relative joint motion (explicit name for [`new`]).
    ///
    /// [`new`]: FrcJointRelative::new
    pub fn single(
        sequence_id: u32,
        configuration: Configuration,
        position: Position,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        Self {
            sequence_id,
            groups: CartesianGroups::single(configuration, position),
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
            lcb_type: None,
            lcb_value: None,
            port_type: None,
            port_number: None,
            port_value: None,
            tool_offset_pr_number: None,
            no_blend: None,
        }
    }

    /// Multi-group relative joint motion with an explicit [`CartesianGroups`] payload.
    pub fn with_groups(
        sequence_id: u32,
        groups: CartesianGroups,
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
            lcb_type: None,
            lcb_value: None,
            port_type: None,
            port_number: None,
            port_value: None,
            tool_offset_pr_number: None,
            no_blend: None,
        }
    }

    /// Coordinated arm (Group 1, Cartesian) + Group 2 (e.g. a positioner)
    /// relative motion. Sets `COORD=ON`.
    ///
    /// NOTE: the controller rejects coordinated joint motion with `RMIT-041`
    /// ("Joint motion with COORD"); see [`coord`]. Prefer a Cartesian
    /// instruction for coordinated arm + positioner moves.
    ///
    /// [`coord`]: FrcJointRelative::coord
    pub fn coordinated(
        sequence_id: u32,
        configuration: Configuration,
        position: Position,
        group2: GroupBlock,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        let mut me = Self::with_groups(
            sequence_id,
            CartesianGroups::arm_and_group2(configuration, position, group2),
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
pub struct FrcJointRelativeResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID", default)]
    pub sequence_id: u32,
}
