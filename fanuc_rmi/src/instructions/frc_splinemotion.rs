use serde::{Deserialize, Serialize};

use crate::instructions::{CartesianGroups, GroupBlock};
use crate::{Configuration, Position, SpeedType, TermType};

/// `FRC_SplineMotion` — add a spline motion instruction (Operators Manual
/// §2.4.17 single-group, §2.4.17.1 two-group).
///
/// The position payload is carried by [`CartesianGroups`], which serializes to
/// the flat single-group form (`Configuration`/`Position` at top level) or the
/// wrapped multi-group form (`G1`/`G2` …). Spline motion supports a reduced
/// option set (no `COORD`, `WristJoint`, `MROT`, `ALIM`/`ALIMREG`, or `NoBlend`
/// per the manual); a two-group spline is built with
/// [`CartesianGroups::arm_and_group2`] and carries no `COORD` key.
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcSplineMotion {
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

    // ---- Optional keys (§2.4.17). All omitted from the wire when `None`. ----
    #[serde(rename = "ACC", default, skip_serializing_if = "Option::is_none")]
    pub acc: Option<u8>,
    #[serde(rename = "OffsetPRNumber", default, skip_serializing_if = "Option::is_none")]
    pub offset_pr_number: Option<i16>,
    #[serde(rename = "VisionPRNumber", default, skip_serializing_if = "Option::is_none")]
    pub vision_pr_number: Option<i16>,
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
}

impl FrcSplineMotion {
    /// Single-group (Group 1) spline motion. Signature-compatible with the
    /// pre-0.6 constructor: builds the flat single-group form, all options unset.
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

    /// Single-group (Group 1) spline motion (explicit name for [`new`]).
    ///
    /// [`new`]: FrcSplineMotion::new
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
            acc: None,
            offset_pr_number: None,
            vision_pr_number: None,
            lcb_type: None,
            lcb_value: None,
            port_type: None,
            port_number: None,
            port_value: None,
            tool_offset_pr_number: None,
        }
    }

    /// Multi-group spline motion with an explicit [`CartesianGroups`] payload.
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
            acc: None,
            offset_pr_number: None,
            vision_pr_number: None,
            lcb_type: None,
            lcb_value: None,
            port_type: None,
            port_number: None,
            port_value: None,
            tool_offset_pr_number: None,
        }
    }

    /// Arm (Group 1, Cartesian) + Group 2 (e.g. a positioner) spline motion.
    /// Spline has no `COORD` key, so this simply carries both groups.
    pub fn two_group(
        sequence_id: u32,
        configuration: Configuration,
        position: Position,
        group2: GroupBlock,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,
    ) -> Self {
        Self::with_groups(
            sequence_id,
            CartesianGroups::arm_and_group2(configuration, position, group2),
            speed_type,
            speed,
            term_type,
            term_value,
        )
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FrcSplineMotionResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "SequenceID", default)]
    pub sequence_id: u32,
}
