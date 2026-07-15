//! Spec-fidelity fixtures for the reshaped motion instructions beyond
//! `FRC_LinearMotion` (covered in `multigroup_linear_test.rs`): circular,
//! spline, and the joint-representation (JRep) instructions.
//!
//! These assert the EXACT wire shape against the RMI Operators Manual
//! (B-84184EN/03) — the canonical proof that the reshape matches the spec.

use fanuc_rmi::instructions::{
    CircGroupBlock, CircularGroups, FrcCircularMotion, FrcJointMotionJRep, FrcSplineMotion,
    FrcSplineMotionJRep, GroupBlock, JointGroups,
};
use fanuc_rmi::packets::{Instruction, OnOff};
use fanuc_rmi::{Configuration, JointAngles, Position, SpeedType, TermType};
use serde_json::Value;

fn pos(x: f64, y: f64, z: f64) -> Position {
    Position { x, y, z, ..Default::default() }
}

// ---------- §2.4.11 circular: flat single-group with via ----------

#[test]
fn circular_single_group_serializes_flat_with_via() {
    let m = FrcCircularMotion::single(
        5,
        Configuration::default(),
        pos(100.0, 0.0, 0.0),
        Configuration::default(),
        pos(50.0, 50.0, 0.0),
        SpeedType::MMSec,
        50.0,
        TermType::CNT,
        100,
    );
    let v: Value = serde_json::to_value(&m).unwrap();
    // flat: destination + via at top level, no G-wrapper, no COORD
    assert_eq!(v["Position"]["X"], 100.0);
    assert_eq!(v["ViaPosition"]["X"], 50.0);
    assert_eq!(v["ViaPosition"]["Y"], 50.0);
    assert!(v.get("Configuration").is_some());
    assert!(v.get("ViaConfiguration").is_some());
    assert!(v.get("G1").is_none(), "single-group must not wrap");
    assert!(v.get("COORD").is_none());

    let back: FrcCircularMotion = serde_json::from_value(v).unwrap();
    assert_eq!(m, back);
    assert!(matches!(back.groups, CircularGroups::Single { .. }));
}

// ---------- §2.4.11.1 circular: wrapped two-group with via ----------

#[test]
fn circular_two_group_wraps_g1_g2_with_via_and_coord() {
    let g1 = CircGroupBlock::new(
        Configuration::default(),
        pos(100.0, 0.0, 0.0),
        Configuration::default(),
        pos(50.0, 50.0, 0.0),
    );
    let g2 = CircGroupBlock::new(
        Configuration::default(),
        pos(10.0, 0.0, 0.0),
        Configuration::default(),
        pos(5.0, 5.0, 0.0),
    );
    let m = FrcCircularMotion::coordinated(
        7,
        g1,
        g2,
        SpeedType::MMSec,
        50.0,
        TermType::CNT,
        100,
    );
    let v: Value = serde_json::to_value(&m).unwrap();
    assert!(v.get("Position").is_none(), "wrapped form has no flat Position");
    assert!(v.get("ViaPosition").is_none());
    assert_eq!(v["G1"]["Position"]["X"], 100.0);
    assert_eq!(v["G1"]["ViaPosition"]["Y"], 50.0);
    assert_eq!(v["G2"]["Position"]["X"], 10.0);
    assert_eq!(v["G2"]["ViaPosition"]["Y"], 5.0);
    assert_eq!(v["COORD"], "ON");

    let back: FrcCircularMotion = serde_json::from_value(v).unwrap();
    assert_eq!(m, back);
    assert_eq!(back.coord, Some(OnOff::ON));
    match &back.groups {
        CircularGroups::Multi(blocks) => {
            assert_eq!(blocks.len(), 2);
            assert_eq!(blocks[0].0, 1);
            assert_eq!(blocks[1].0, 2);
        }
        _ => panic!("expected Multi"),
    }
}

// ---------- §2.4.17 spline: flat, reduced option set, no COORD ----------

#[test]
fn spline_single_group_serializes_flat() {
    let m = FrcSplineMotion::single(
        19,
        Configuration::default(),
        pos(1.0, 2.0, 3.0),
        SpeedType::MMSec,
        200.0,
        TermType::CNT,
        100,
    );
    let v: Value = serde_json::to_value(&m).unwrap();
    assert_eq!(v["Position"]["X"], 1.0);
    assert!(v.get("G1").is_none());
    assert!(v.get("COORD").is_none(), "spline never carries COORD");

    let back: FrcSplineMotion = serde_json::from_value(v).unwrap();
    assert_eq!(m, back);
}

// ---------- §2.4.17.1 spline two-group: wrapped, NO COORD key ----------

#[test]
fn spline_two_group_wraps_without_coord() {
    // Arm (G1 Cartesian) + positioner (G2 joint) — the free-second-group rule.
    let m = FrcSplineMotion::two_group(
        20,
        Configuration::default(),
        pos(1.0, 2.0, 3.0),
        GroupBlock::joint(JointAngles { j1: 30.0, ..Default::default() }),
        SpeedType::MMSec,
        200.0,
        TermType::CNT,
        100,
    );
    let v: Value = serde_json::to_value(&m).unwrap();
    assert_eq!(v["G1"]["Position"]["X"], 1.0);
    assert_eq!(v["G2"]["JointAngle"]["J1"], 30.0);
    assert!(v.get("COORD").is_none(), "spline two-group carries NO COORD key");

    let back: FrcSplineMotion = serde_json::from_value(v).unwrap();
    assert_eq!(m, back);
}

// ---------- §2.4.18 spline JRep: singular JointAngle ----------

#[test]
fn spline_jrep_emits_singular_jointangle() {
    let m = FrcSplineMotionJRep::single(
        20,
        JointAngles { j1: 10.0, j2: 20.0, ..Default::default() },
        SpeedType::MMSec,
        200.0,
        TermType::CNT,
        100,
    );
    let v: Value = serde_json::to_value(&m).unwrap();
    // correct-out: singular "JointAngle", flat (single group)
    assert_eq!(v["JointAngle"]["J1"], 10.0);
    assert!(v.get("JointAngles").is_none(), "must emit singular JointAngle");
    assert!(v.get("G1").is_none());

    let back: FrcSplineMotionJRep = serde_json::from_value(v).unwrap();
    assert_eq!(m, back);
    assert!(matches!(back.groups, JointGroups::Single { .. }));
}

// ---------- JRep naming policy: singular out, plural accepted in ----------

#[test]
fn joint_jrep_emits_singular_and_accepts_plural_alias() {
    let m = FrcJointMotionJRep::single(
        3,
        JointAngles { j1: 5.0, ..Default::default() },
        SpeedType::MMSec,
        10.0,
        TermType::FINE,
        0,
    );
    let v: Value = serde_json::to_value(&m).unwrap();
    assert_eq!(v["JointAngle"]["J1"], 5.0);
    assert!(v.get("JointAngles").is_none(), "reshape fixes the plural bug");

    // lenient-in: a legacy/sim packet using the plural key still deserializes.
    let raw = r#"{
        "SequenceID": 3,
        "JointAngles": {"J1":5.0,"J2":0,"J3":0,"J4":0,"J5":0,"J6":0},
        "SpeedType": "mmSec", "Speed": 10.0, "TermType": "FINE", "TermValue": 0
    }"#;
    let back: FrcJointMotionJRep = serde_json::from_str(raw).unwrap();
    assert_eq!(m, back);
}

// ---------- Instruction enum: spline tag + flatten coexist ----------

#[test]
fn spline_through_instruction_enum_tag_and_shape() {
    let m = FrcSplineMotion::single(
        1,
        Configuration::default(),
        pos(1.0, 2.0, 3.0),
        SpeedType::MMSec,
        200.0,
        TermType::CNT,
        100,
    );
    let instr = Instruction::FrcSplineMotion(m.clone());
    let v: Value = serde_json::to_value(&instr).unwrap();
    assert_eq!(v["Instruction"], "FRC_SplineMotion");
    assert_eq!(v["Position"]["X"], 1.0); // flat, alongside the tag
    let back: Instruction = serde_json::from_value(v).unwrap();
    assert_eq!(instr, back);
}

// ---------- DTO bincode round-trip for circular (via-carrying) ----------

#[cfg(feature = "DTO")]
#[test]
fn circular_dto_bincode_round_trips() {
    use fanuc_rmi::instructions::dto::FrcCircularMotion as FrcCircularMotionDto;

    let g1 = CircGroupBlock::new(
        Configuration::default(),
        pos(100.0, 0.0, 0.0),
        Configuration::default(),
        pos(50.0, 50.0, 0.0),
    );
    let g2 = CircGroupBlock::new(
        Configuration::default(),
        pos(10.0, 0.0, 0.0),
        Configuration::default(),
        pos(5.0, 5.0, 0.0),
    );
    let m = FrcCircularMotion::coordinated(
        7, g1, g2, SpeedType::MMSec, 50.0, TermType::CNT, 100,
    );
    let dto: FrcCircularMotionDto = m.clone().into();
    let bytes = bincode::serialize(&dto).expect("circular DTO must serialize in bincode");
    let back_dto: FrcCircularMotionDto = bincode::deserialize(&bytes).unwrap();
    let back: FrcCircularMotion = back_dto.into();
    assert_eq!(m, back);
}
