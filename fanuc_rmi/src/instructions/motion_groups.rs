//! Multi-group motion payloads for RMI motion instructions.
//!
//! Per the RMI Operators Manual (B-84184EN/03) §1.4.1 and §2.4.7.1, a single
//! motion instruction packet can drive **more than one motion group** (e.g. a
//! robot arm as Group 1 and a positioner as Group 2). The wire encoding of the
//! position payload differs by group count for the *same* `Instruction` name:
//!
//! - **Single group (flat):** the group's data sits at the top level of the
//!   packet — `"Configuration":{…},"Position":{…}` (Cartesian) or
//!   `"JointAngle":{…}` (joint representation).
//! - **Multiple groups (wrapped):** each group's data is nested under a
//!   `"G<n>"` key whose number matches the bit set in the session `GroupMask`
//!   (§2.4.7.1 NOTE: `GroupMask=5` ⇒ `G1` and `G3`). The shared
//!   `SpeedType`/`Speed`/`TermType`/`TermValue` and options (`COORD`, …) stay
//!   at the top level.
//!
//! Spec rules encoded here:
//! - The **first** group's representation is fixed by the instruction (a
//!   Cartesian instruction's G1 is Cartesian; a JRep instruction's G1 is joint).
//!   The **second** group is free — *"RMI does not check the second position's
//!   representation type"* (§2.4.7.1) — so a Cartesian instruction may carry
//!   `G2:{JointAngle:{J1,J2}}`, which is exactly the positioner case.
//! - A short joint block is valid: `{JointAngle:{J1,J2}}` for a 2-axis
//!   positioner; missing axes default to 0 (§2.4.7.1 NOTE, [`JointAngles`]).
//!
//! Naming policy (lenient-in / correct-out): serialization emits the key the
//! real controller expects (`"JointAngle"`, singular — matching both the manual
//! and real robots); deserialization also accepts the legacy/simulator alias
//! `"JointAngles"` (plural). See [`crate::commands`] for the analogous
//! read-side aliases (e.g. `FrameNumber` vs `UFrameNumber`).

use serde::de::{Error as DeError, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

use crate::{Configuration, JointAngles, Position};

/// One motion group's target within a multi-group packet.
///
/// The first group of a given instruction is constrained to that instruction's
/// native representation, but any second group is free to be Cartesian or joint
/// (§2.4.7.1). Construct with [`GroupBlock::cartesian`] / [`GroupBlock::joint`].
#[derive(Debug, Clone, PartialEq)]
pub enum GroupBlock {
    /// Cartesian target: `{ "Configuration": {…}, "Position": {…} }`.
    Cartesian {
        configuration: Configuration,
        position: Position,
    },
    /// Joint target: `{ "JointAngle": {…} }` (the positioner form).
    Joint { joint_angle: JointAngles },
}

impl GroupBlock {
    /// A Cartesian group target.
    pub fn cartesian(configuration: Configuration, position: Position) -> Self {
        GroupBlock::Cartesian { configuration, position }
    }
    /// A joint group target (e.g. a positioner). A short [`JointAngles`] with
    /// only `J1`/`J2` populated is valid for a 2-axis positioner.
    pub fn joint(joint_angle: JointAngles) -> Self {
        GroupBlock::Joint { joint_angle }
    }
}

impl Serialize for GroupBlock {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        match self {
            GroupBlock::Cartesian { configuration, position } => {
                m.serialize_entry("Configuration", configuration)?;
                m.serialize_entry("Position", position)?;
            }
            GroupBlock::Joint { joint_angle } => {
                // correct-out: singular "JointAngle"
                m.serialize_entry("JointAngle", joint_angle)?;
            }
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for GroupBlock {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct BlockVisitor;
        impl<'de> Visitor<'de> for BlockVisitor {
            type Value = GroupBlock;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a Cartesian ({Configuration,Position}) or joint ({JointAngle}) group block")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<GroupBlock, M::Error> {
                let mut configuration: Option<Configuration> = None;
                let mut position: Option<Position> = None;
                let mut joint_angle: Option<JointAngles> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "Configuration" => configuration = Some(map.next_value()?),
                        "Position" => position = Some(map.next_value()?),
                        // lenient-in: accept singular (real robot) and plural (sim)
                        "JointAngle" | "JointAngles" => joint_angle = Some(map.next_value()?),
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                if let Some(joint_angle) = joint_angle {
                    Ok(GroupBlock::Joint { joint_angle })
                } else {
                    let configuration = configuration
                        .ok_or_else(|| DeError::missing_field("Configuration"))?;
                    let position =
                        position.ok_or_else(|| DeError::missing_field("Position"))?;
                    Ok(GroupBlock::Cartesian { configuration, position })
                }
            }
        }
        d.deserialize_map(BlockVisitor)
    }
}

/// Serialize the wrapped multi-group form: `"G1":{…},"G2":{…}` keyed by the
/// group number. Shared by every group-payload enum's `Multi` arm.
fn serialize_multi<S, B>(blocks: &[(u8, B)], map: &mut S) -> Result<(), S::Error>
where
    S: SerializeMap,
    B: Serialize,
{
    for (n, block) in blocks {
        map.serialize_entry(&format!("G{n}"), block)?;
    }
    Ok(())
}

/// The Cartesian motion payload: flat single-group, or wrapped multi-group.
///
/// Used by the Cartesian-native instructions (linear, joint-by-Cartesian,
/// their relatives, spline). `Single` emits the flat top-level form; `Multi`
/// emits `G<n>` blocks (the first of which must be Cartesian per spec).
#[derive(Debug, Clone, PartialEq)]
pub enum CartesianGroups {
    Single {
        configuration: Configuration,
        position: Position,
    },
    Multi(Vec<(u8, GroupBlock)>),
}

impl CartesianGroups {
    /// Single-group (Group 1) flat form.
    pub fn single(configuration: Configuration, position: Position) -> Self {
        CartesianGroups::Single { configuration, position }
    }
    /// Multi-group form, group numbers explicit (must match the session
    /// `GroupMask`; the first block must be Cartesian).
    pub fn multi(blocks: Vec<(u8, GroupBlock)>) -> Self {
        CartesianGroups::Multi(blocks)
    }
    /// Convenience for the common arm(G1)+positioner(G2) coordinated case.
    pub fn arm_and_group2(
        configuration: Configuration,
        position: Position,
        group2: GroupBlock,
    ) -> Self {
        CartesianGroups::Multi(vec![
            (1, GroupBlock::Cartesian { configuration, position }),
            (2, group2),
        ])
    }
}

impl Serialize for CartesianGroups {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        match self {
            CartesianGroups::Single { configuration, position } => {
                m.serialize_entry("Configuration", configuration)?;
                m.serialize_entry("Position", position)?;
            }
            CartesianGroups::Multi(blocks) => serialize_multi(blocks, &mut m)?,
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for CartesianGroups {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_map(CartesianGroupsVisitor)
    }
}

struct CartesianGroupsVisitor;
impl<'de> Visitor<'de> for CartesianGroupsVisitor {
    type Value = CartesianGroups;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("flat Configuration/Position or G<n> group blocks")
    }
    fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<CartesianGroups, M::Error> {
        let mut configuration: Option<Configuration> = None;
        let mut position: Option<Position> = None;
        let mut multi: Vec<(u8, GroupBlock)> = Vec::new();
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "Configuration" => configuration = Some(map.next_value()?),
                "Position" => position = Some(map.next_value()?),
                k if is_group_key(k) => {
                    let n = parse_group_key::<M>(k)?;
                    multi.push((n, map.next_value()?));
                }
                _ => {
                    let _: serde::de::IgnoredAny = map.next_value()?;
                }
            }
        }
        if !multi.is_empty() {
            multi.sort_by_key(|(n, _)| *n);
            Ok(CartesianGroups::Multi(multi))
        } else {
            let configuration =
                configuration.ok_or_else(|| DeError::missing_field("Configuration"))?;
            let position = position.ok_or_else(|| DeError::missing_field("Position"))?;
            Ok(CartesianGroups::Single { configuration, position })
        }
    }
}

/// The joint-representation motion payload (JRep instructions): flat single
/// `JointAngle`, or wrapped multi-group.
#[derive(Debug, Clone, PartialEq)]
pub enum JointGroups {
    Single { joint_angle: JointAngles },
    Multi(Vec<(u8, GroupBlock)>),
}

impl JointGroups {
    pub fn single(joint_angle: JointAngles) -> Self {
        JointGroups::Single { joint_angle }
    }
    pub fn multi(blocks: Vec<(u8, GroupBlock)>) -> Self {
        JointGroups::Multi(blocks)
    }
    /// Arm(G1, joint) + positioner(G2) coordinated case.
    pub fn arm_and_group2(g1_joint: JointAngles, group2: GroupBlock) -> Self {
        JointGroups::Multi(vec![(1, GroupBlock::Joint { joint_angle: g1_joint }), (2, group2)])
    }
}

impl Serialize for JointGroups {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        match self {
            JointGroups::Single { joint_angle } => {
                m.serialize_entry("JointAngle", joint_angle)?;
            }
            JointGroups::Multi(blocks) => serialize_multi(blocks, &mut m)?,
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for JointGroups {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct JointGroupsVisitor;
        impl<'de> Visitor<'de> for JointGroupsVisitor {
            type Value = JointGroups;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("flat JointAngle or G<n> group blocks")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<JointGroups, M::Error> {
                let mut joint_angle: Option<JointAngles> = None;
                let mut multi: Vec<(u8, GroupBlock)> = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "JointAngle" | "JointAngles" => joint_angle = Some(map.next_value()?),
                        k if is_group_key(k) => {
                            let n = parse_group_key::<M>(k)?;
                            multi.push((n, map.next_value()?));
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                if !multi.is_empty() {
                    multi.sort_by_key(|(n, _)| *n);
                    Ok(JointGroups::Multi(multi))
                } else {
                    let joint_angle =
                        joint_angle.ok_or_else(|| DeError::missing_field("JointAngle"))?;
                    Ok(JointGroups::Single { joint_angle })
                }
            }
        }
        d.deserialize_map(JointGroupsVisitor)
    }
}

/// One circular motion group's target: Cartesian destination + via, each with
/// its own [`Configuration`]. Circular moves need three points (the implicit
/// start, the via, and the destination), so a group carries both a destination
/// (`Configuration`/`Position`) and a via (`ViaConfiguration`/`ViaPosition`).
///
/// The manual documents circular two-group blocks as Cartesian only (§2.4.11.1).
/// The general free-second-group rule (§2.4.7.1) technically permits a joint
/// second group, but the manual specifies no joint-with-via wire shape for
/// circular, so this encodes the documented Cartesian form.
#[derive(Debug, Clone, PartialEq)]
pub struct CircGroupBlock {
    pub configuration: Configuration,
    pub position: Position,
    pub via_configuration: Configuration,
    pub via_position: Position,
}

impl CircGroupBlock {
    pub fn new(
        configuration: Configuration,
        position: Position,
        via_configuration: Configuration,
        via_position: Position,
    ) -> Self {
        Self { configuration, position, via_configuration, via_position }
    }
}

impl Serialize for CircGroupBlock {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(Some(4))?;
        m.serialize_entry("Configuration", &self.configuration)?;
        m.serialize_entry("Position", &self.position)?;
        m.serialize_entry("ViaConfiguration", &self.via_configuration)?;
        m.serialize_entry("ViaPosition", &self.via_position)?;
        m.end()
    }
}

impl<'de> Deserialize<'de> for CircGroupBlock {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = CircGroupBlock;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a circular group block ({Configuration,Position,ViaConfiguration,ViaPosition})")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<CircGroupBlock, M::Error> {
                let mut configuration: Option<Configuration> = None;
                let mut position: Option<Position> = None;
                let mut via_configuration: Option<Configuration> = None;
                let mut via_position: Option<Position> = None;
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "Configuration" => configuration = Some(map.next_value()?),
                        "Position" => position = Some(map.next_value()?),
                        "ViaConfiguration" => via_configuration = Some(map.next_value()?),
                        "ViaPosition" => via_position = Some(map.next_value()?),
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                Ok(CircGroupBlock {
                    configuration: configuration.ok_or_else(|| DeError::missing_field("Configuration"))?,
                    position: position.ok_or_else(|| DeError::missing_field("Position"))?,
                    via_configuration: via_configuration
                        .ok_or_else(|| DeError::missing_field("ViaConfiguration"))?,
                    via_position: via_position.ok_or_else(|| DeError::missing_field("ViaPosition"))?,
                })
            }
        }
        d.deserialize_map(V)
    }
}

/// The circular motion payload: flat single-group, or wrapped multi-group.
///
/// Used by the circular-native instructions. `Single` emits the flat top-level
/// form (`Configuration`/`Position`/`ViaConfiguration`/`ViaPosition`); `Multi`
/// emits `G<n>` blocks, each a [`CircGroupBlock`].
#[derive(Debug, Clone, PartialEq)]
pub enum CircularGroups {
    Single {
        configuration: Configuration,
        position: Position,
        via_configuration: Configuration,
        via_position: Position,
    },
    Multi(Vec<(u8, CircGroupBlock)>),
}

impl CircularGroups {
    /// Single-group (Group 1) flat form.
    pub fn single(
        configuration: Configuration,
        position: Position,
        via_configuration: Configuration,
        via_position: Position,
    ) -> Self {
        CircularGroups::Single { configuration, position, via_configuration, via_position }
    }
    /// Multi-group form, group numbers explicit (must match the session
    /// `GroupMask`; the first block is the instruction's own group).
    pub fn multi(blocks: Vec<(u8, CircGroupBlock)>) -> Self {
        CircularGroups::Multi(blocks)
    }
    /// Arm(G1) + Group 2 coordinated circular case.
    pub fn arm_and_group2(g1: CircGroupBlock, group2: CircGroupBlock) -> Self {
        CircularGroups::Multi(vec![(1, g1), (2, group2)])
    }
}

impl Serialize for CircularGroups {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut m = s.serialize_map(None)?;
        match self {
            CircularGroups::Single { configuration, position, via_configuration, via_position } => {
                m.serialize_entry("Configuration", configuration)?;
                m.serialize_entry("Position", position)?;
                m.serialize_entry("ViaConfiguration", via_configuration)?;
                m.serialize_entry("ViaPosition", via_position)?;
            }
            CircularGroups::Multi(blocks) => serialize_multi(blocks, &mut m)?,
        }
        m.end()
    }
}

impl<'de> Deserialize<'de> for CircularGroups {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = CircularGroups;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("flat circular Cartesian fields or G<n> circular blocks")
            }
            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<CircularGroups, M::Error> {
                let mut configuration: Option<Configuration> = None;
                let mut position: Option<Position> = None;
                let mut via_configuration: Option<Configuration> = None;
                let mut via_position: Option<Position> = None;
                let mut multi: Vec<(u8, CircGroupBlock)> = Vec::new();
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "Configuration" => configuration = Some(map.next_value()?),
                        "Position" => position = Some(map.next_value()?),
                        "ViaConfiguration" => via_configuration = Some(map.next_value()?),
                        "ViaPosition" => via_position = Some(map.next_value()?),
                        k if is_group_key(k) => {
                            let n = parse_group_key::<M>(k)?;
                            multi.push((n, map.next_value()?));
                        }
                        _ => {
                            let _: serde::de::IgnoredAny = map.next_value()?;
                        }
                    }
                }
                if !multi.is_empty() {
                    multi.sort_by_key(|(n, _)| *n);
                    Ok(CircularGroups::Multi(multi))
                } else {
                    Ok(CircularGroups::Single {
                        configuration: configuration
                            .ok_or_else(|| DeError::missing_field("Configuration"))?,
                        position: position.ok_or_else(|| DeError::missing_field("Position"))?,
                        via_configuration: via_configuration
                            .ok_or_else(|| DeError::missing_field("ViaConfiguration"))?,
                        via_position: via_position
                            .ok_or_else(|| DeError::missing_field("ViaPosition"))?,
                    })
                }
            }
        }
        d.deserialize_map(V)
    }
}

/// `true` if `key` is a `G<n>` group key with a valid group number (1..=8).
fn is_group_key(key: &str) -> bool {
    key.len() >= 2
        && key.as_bytes()[0] == b'G'
        && key[1..].parse::<u8>().map(|n| (1..=8).contains(&n)).unwrap_or(false)
}

fn parse_group_key<'de, M: MapAccess<'de>>(key: &str) -> Result<u8, M::Error> {
    key[1..]
        .parse::<u8>()
        .map_err(|_| DeError::custom(format!("invalid group key {key:?}")))
}

// ---------------------------------------------------------------------------
// DTO mirrors (feature = "DTO"): bincode-safe, plain derived serde.
//
// The `mirror_dto` macro cannot rewrite the `Vec<(u8, GroupBlock)>` element
// type inside these enums, so it would embed the JSON-only protocol block in
// the DTO and break bincode. These mirrors are therefore hand-written with the
// already-mirrored `…Dto` element types, and carry the derived (self-describing
// and bincode-safe) serde impls. The flat/wrapped JSON logic lives ONLY on the
// protocol types above; the DTO is an internal transport shape.
// ---------------------------------------------------------------------------

#[cfg(feature = "DTO")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum GroupBlockDto {
    Cartesian {
        configuration: crate::ConfigurationDto,
        position: crate::PositionDto,
    },
    Joint {
        joint_angle: crate::JointAnglesDto,
    },
}

#[cfg(feature = "DTO")]
impl From<GroupBlock> for GroupBlockDto {
    fn from(src: GroupBlock) -> Self {
        match src {
            GroupBlock::Cartesian { configuration, position } => GroupBlockDto::Cartesian {
                configuration: configuration.into(),
                position: position.into(),
            },
            GroupBlock::Joint { joint_angle } => {
                GroupBlockDto::Joint { joint_angle: joint_angle.into() }
            }
        }
    }
}

#[cfg(feature = "DTO")]
impl From<GroupBlockDto> for GroupBlock {
    fn from(src: GroupBlockDto) -> Self {
        match src {
            GroupBlockDto::Cartesian { configuration, position } => GroupBlock::Cartesian {
                configuration: configuration.into(),
                position: position.into(),
            },
            GroupBlockDto::Joint { joint_angle } => {
                GroupBlock::Joint { joint_angle: joint_angle.into() }
            }
        }
    }
}

#[cfg(feature = "DTO")]
fn blocks_to_dto(v: Vec<(u8, GroupBlock)>) -> Vec<(u8, GroupBlockDto)> {
    v.into_iter().map(|(n, b)| (n, b.into())).collect()
}

#[cfg(feature = "DTO")]
fn blocks_from_dto(v: Vec<(u8, GroupBlockDto)>) -> Vec<(u8, GroupBlock)> {
    v.into_iter().map(|(n, b)| (n, b.into())).collect()
}

#[cfg(feature = "DTO")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CartesianGroupsDto {
    Single {
        configuration: crate::ConfigurationDto,
        position: crate::PositionDto,
    },
    Multi(Vec<(u8, GroupBlockDto)>),
}

#[cfg(feature = "DTO")]
impl From<CartesianGroups> for CartesianGroupsDto {
    fn from(src: CartesianGroups) -> Self {
        match src {
            CartesianGroups::Single { configuration, position } => CartesianGroupsDto::Single {
                configuration: configuration.into(),
                position: position.into(),
            },
            CartesianGroups::Multi(v) => CartesianGroupsDto::Multi(blocks_to_dto(v)),
        }
    }
}

#[cfg(feature = "DTO")]
impl From<CartesianGroupsDto> for CartesianGroups {
    fn from(src: CartesianGroupsDto) -> Self {
        match src {
            CartesianGroupsDto::Single { configuration, position } => CartesianGroups::Single {
                configuration: configuration.into(),
                position: position.into(),
            },
            CartesianGroupsDto::Multi(v) => CartesianGroups::Multi(blocks_from_dto(v)),
        }
    }
}

#[cfg(feature = "DTO")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum JointGroupsDto {
    Single { joint_angle: crate::JointAnglesDto },
    Multi(Vec<(u8, GroupBlockDto)>),
}

#[cfg(feature = "DTO")]
impl From<JointGroups> for JointGroupsDto {
    fn from(src: JointGroups) -> Self {
        match src {
            JointGroups::Single { joint_angle } => {
                JointGroupsDto::Single { joint_angle: joint_angle.into() }
            }
            JointGroups::Multi(v) => JointGroupsDto::Multi(blocks_to_dto(v)),
        }
    }
}

#[cfg(feature = "DTO")]
impl From<JointGroupsDto> for JointGroups {
    fn from(src: JointGroupsDto) -> Self {
        match src {
            JointGroupsDto::Single { joint_angle } => {
                JointGroups::Single { joint_angle: joint_angle.into() }
            }
            JointGroupsDto::Multi(v) => JointGroups::Multi(blocks_from_dto(v)),
        }
    }
}

#[cfg(feature = "DTO")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CircGroupBlockDto {
    pub configuration: crate::ConfigurationDto,
    pub position: crate::PositionDto,
    pub via_configuration: crate::ConfigurationDto,
    pub via_position: crate::PositionDto,
}

#[cfg(feature = "DTO")]
impl From<CircGroupBlock> for CircGroupBlockDto {
    fn from(src: CircGroupBlock) -> Self {
        CircGroupBlockDto {
            configuration: src.configuration.into(),
            position: src.position.into(),
            via_configuration: src.via_configuration.into(),
            via_position: src.via_position.into(),
        }
    }
}

#[cfg(feature = "DTO")]
impl From<CircGroupBlockDto> for CircGroupBlock {
    fn from(src: CircGroupBlockDto) -> Self {
        CircGroupBlock {
            configuration: src.configuration.into(),
            position: src.position.into(),
            via_configuration: src.via_configuration.into(),
            via_position: src.via_position.into(),
        }
    }
}

#[cfg(feature = "DTO")]
fn circ_blocks_to_dto(v: Vec<(u8, CircGroupBlock)>) -> Vec<(u8, CircGroupBlockDto)> {
    v.into_iter().map(|(n, b)| (n, b.into())).collect()
}

#[cfg(feature = "DTO")]
fn circ_blocks_from_dto(v: Vec<(u8, CircGroupBlockDto)>) -> Vec<(u8, CircGroupBlock)> {
    v.into_iter().map(|(n, b)| (n, b.into())).collect()
}

#[cfg(feature = "DTO")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CircularGroupsDto {
    Single {
        configuration: crate::ConfigurationDto,
        position: crate::PositionDto,
        via_configuration: crate::ConfigurationDto,
        via_position: crate::PositionDto,
    },
    Multi(Vec<(u8, CircGroupBlockDto)>),
}

#[cfg(feature = "DTO")]
impl From<CircularGroups> for CircularGroupsDto {
    fn from(src: CircularGroups) -> Self {
        match src {
            CircularGroups::Single { configuration, position, via_configuration, via_position } => {
                CircularGroupsDto::Single {
                    configuration: configuration.into(),
                    position: position.into(),
                    via_configuration: via_configuration.into(),
                    via_position: via_position.into(),
                }
            }
            CircularGroups::Multi(v) => CircularGroupsDto::Multi(circ_blocks_to_dto(v)),
        }
    }
}

#[cfg(feature = "DTO")]
impl From<CircularGroupsDto> for CircularGroups {
    fn from(src: CircularGroupsDto) -> Self {
        match src {
            CircularGroupsDto::Single { configuration, position, via_configuration, via_position } => {
                CircularGroups::Single {
                    configuration: configuration.into(),
                    position: position.into(),
                    via_configuration: via_configuration.into(),
                    via_position: via_position.into(),
                }
            }
            CircularGroupsDto::Multi(v) => CircularGroups::Multi(circ_blocks_from_dto(v)),
        }
    }
}
