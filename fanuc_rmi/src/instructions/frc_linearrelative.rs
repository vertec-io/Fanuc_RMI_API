use serde::{Deserialize, Serialize};

use crate::instructions::{CartesianGroups, GroupBlock};
use crate::packets::OnOff;
use crate::{Configuration, Position, SpeedType, TermType};

/// `FRC_LinearRelative` — add an incremental (relative) linear motion
/// instruction (Operators Manual §2.4.8 single-group, §2.4.8.1 two-group).
///
/// Identical wire shape to [`FrcLinearMotion`] but with incremental Cartesian
/// positions. The position payload is carried by [`CartesianGroups`]: flat
/// single-group, or wrapped `G1`/`G2` multi-group.
///
/// [`FrcLinearMotion`]: crate::instructions::FrcLinearMotion
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcLinearRelative {
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

    // ---- Optional keys (§2.4.8). All omitted from the wire when `None`. ----
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

impl FrcLinearRelative {
    /// Single-group (Group 1) relative linear motion. Signature-compatible with
    /// the pre-0.6 constructor: builds the flat single-group form.
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

    /// Single-group (Group 1) relative linear motion (explicit name for [`new`]).
    ///
    /// [`new`]: FrcLinearRelative::new
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

    /// Multi-group relative linear motion with an explicit [`CartesianGroups`].
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

    /// Coordinated arm (Group 1, Cartesian) + Group 2 relative motion.
    /// Sets `COORD=ON`.
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
pub struct FrcLinearRelativeResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID", default)]
    pub sequence_id: u32,
}
