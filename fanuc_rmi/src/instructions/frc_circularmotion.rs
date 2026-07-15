use serde::{Deserialize, Serialize};

use crate::instructions::{CircGroupBlock, CircularGroups};
use crate::packets::OnOff;
use crate::{Configuration, Position, SpeedType, TermType};

/// `FRC_CircularMotion` — add a circular motion instruction (Operators Manual
/// §2.4.11 single-group, §2.4.11.1 two-group).
///
/// A circular move needs three points (the implicit start, the via, and the
/// destination), so the position payload is carried by [`CircularGroups`]: each
/// group has a destination (`Configuration`/`Position`) plus a via
/// (`ViaConfiguration`/`ViaPosition`). `CircularGroups` serializes to the flat
/// single-group form or the wrapped multi-group form (`G1`/`G2` …). For a
/// coordinated arm + positioner move, build the groups with
/// [`CircularGroups::arm_and_group2`] and set [`coord`] to [`OnOff::ON`].
///
/// [`coord`]: FrcCircularMotion::coord
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcCircularMotion {
    #[serde(rename = "SequenceID")]
    pub sequence_id: u32,

    /// The motion target(s): single group (flat) or multiple groups (`G<n>`).
    #[serde(flatten)]
    pub groups: CircularGroups,

    #[serde(rename = "SpeedType")]
    pub speed_type: SpeedType,
    #[serde(rename = "Speed")]
    pub speed: f64,
    #[serde(rename = "TermType")]
    pub term_type: TermType,
    #[serde(rename = "TermValue")]
    pub term_value: u8,

    // ---- Optional keys (§2.4.11.1). All omitted from the wire when `None`. ----
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

impl FrcCircularMotion {
    /// Single-group (Group 1) circular motion. Signature-compatible with the
    /// pre-0.6 constructor: builds the flat single-group form, all options unset.
    pub fn new(
        sequence_id: u32,
        configuration: Configuration,
        position: Position,
        via_configuration: Configuration,
        via_position: Position,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        Self::single(
            sequence_id,
            configuration,
            position,
            via_configuration,
            via_position,
            speed_type,
            speed,
            term_type,
            term_value,
        )
    }

    /// Single-group (Group 1) circular motion (explicit name for [`new`]).
    ///
    /// [`new`]: FrcCircularMotion::new
    pub fn single(
        sequence_id: u32,
        configuration: Configuration,
        position: Position,
        via_configuration: Configuration,
        via_position: Position,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        Self {
            sequence_id,
            groups: CircularGroups::single(configuration, position, via_configuration, via_position),
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

    /// Multi-group circular motion with an explicit [`CircularGroups`] payload.
    pub fn with_groups(
        sequence_id: u32,
        groups: CircularGroups,
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

    /// Coordinated arm (Group 1) + Group 2 (e.g. a positioner) circular motion.
    /// Sets `COORD=ON`.
    pub fn coordinated(
        sequence_id: u32,
        g1: CircGroupBlock,
        group2: CircGroupBlock,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        let mut me = Self::with_groups(
            sequence_id,
            CircularGroups::arm_and_group2(g1, group2),
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
pub struct FrcCircularMotionResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID", default)]
    pub sequence_id: u32,
}
