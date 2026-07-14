//! Spec-fidelity fixtures for multi-group `FRC_LinearMotion`
//! (RMI Operators Manual §2.4.7 single-group, §2.4.7.1 two-group).
//!
//! These assert the EXACT wire shape against the manual, not just round-trip
//! stability — this is the canonical proof that the reshape matches the spec.

use fanuc_rmi::instructions::{CartesianGroups, FrcLinearMotion, GroupBlock};
use fanuc_rmi::packets::{Instruction, OnOff};
use fanuc_rmi::{Configuration, JointAngles, Position, SpeedType, TermType};
use serde_json::Value;

fn arm_position() -> Position {
    Position { x: 100.0, y: 200.0, z: 300.0, ..Default::default() }
}

// ----- §2.4.7 single-group: FLAT top-level Configuration/Position -----

#[test]
fn single_group_serializes_flat() {
    let m = FrcLinearMotion::single(
        5,
        Configuration::default(),
        arm_position(),
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        0,
    );
    let v: Value = serde_json::to_value(&m).unwrap();

    // flat: Configuration/Position at top level, no G-wrapper, no COORD
    assert_eq!(v["SequenceID"], 5);
    assert_eq!(v["Configuration"]["UToolNumber"], 1);
    assert_eq!(v["Position"]["X"], 100.0);
    assert_eq!(v["Position"]["Z"], 300.0);
    assert!(v.get("G1").is_none(), "single-group must not wrap in G1");
    assert!(v.get("COORD").is_none(), "COORD omitted when unset");
    assert!(v.get("SpeedType").is_some());
    assert!(v.get("TermValue").is_some());
}

#[test]
fn single_group_round_trips() {
    let m = FrcLinearMotion::single(
        5,
        Configuration::default(),
        arm_position(),
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        0,
    );
    let j = serde_json::to_string(&m).unwrap();
    let back: FrcLinearMotion = serde_json::from_str(&j).unwrap();
    assert_eq!(m, back);
    assert!(matches!(back.groups, CartesianGroups::Single { .. }));
}

// ----- §2.4.7.1 two-group: WRAPPED G1 (arm) + G2 (positioner) + COORD -----

#[test]
fn coordinated_arm_plus_positioner_serializes_wrapped() {
    // 2-axis positioner as Group 2 joint target (the §2.4.7.1 NOTE example).
    let positioner = JointAngles { j1: 45.0, j2: 10.0, ..Default::default() };
    let m = FrcLinearMotion::coordinated(
        7,
        Configuration::default(),
        arm_position(),
        GroupBlock::joint(positioner),
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        0,
    );
    let v: Value = serde_json::to_value(&m).unwrap();

    // No flat position region; both groups wrapped.
    assert!(v.get("Configuration").is_none(), "wrapped form has no flat Configuration");
    assert!(v.get("Position").is_none());
    assert_eq!(v["G1"]["Configuration"]["UToolNumber"], 1);
    assert_eq!(v["G1"]["Position"]["X"], 100.0);
    // Positioner: singular "JointAngle" key (correct-out), J1/J2 set.
    assert_eq!(v["G2"]["JointAngle"]["J1"], 45.0);
    assert_eq!(v["G2"]["JointAngle"]["J2"], 10.0);
    assert!(v["G2"].get("JointAngles").is_none(), "must emit singular JointAngle");
    // Coordinated.
    assert_eq!(v["COORD"], "ON");
    // Shared motion params stay top-level.
    assert_eq!(v["SequenceID"], 7);
    assert!(v.get("SpeedType").is_some());
}

#[test]
fn coordinated_round_trips() {
    let positioner = JointAngles { j1: 45.0, j2: 10.0, ..Default::default() };
    let m = FrcLinearMotion::coordinated(
        7,
        Configuration::default(),
        arm_position(),
        GroupBlock::joint(positioner),
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        0,
    );
    let j = serde_json::to_string(&m).unwrap();
    let back: FrcLinearMotion = serde_json::from_str(&j).unwrap();
    assert_eq!(m, back);
    match &back.groups {
        CartesianGroups::Multi(blocks) => {
            assert_eq!(blocks.len(), 2);
            assert_eq!(blocks[0].0, 1);
            assert!(matches!(blocks[0].1, GroupBlock::Cartesian { .. }));
            assert_eq!(blocks[1].0, 2);
            assert!(matches!(blocks[1].1, GroupBlock::Joint { .. }));
        }
        _ => panic!("expected Multi"),
    }
    assert_eq!(back.coord, Some(OnOff::ON));
}

#[test]
fn two_cartesian_groups_serializes_wrapped() {
    // G2 as a second Cartesian group (arm + arm) — both Cartesian is legal.
    let g2 = GroupBlock::cartesian(
        Configuration::default(),
        Position { x: 1.0, y: 2.0, z: 3.0, ..Default::default() },
    );
    let m = FrcLinearMotion::with_groups(
        9,
        CartesianGroups::arm_and_group2(Configuration::default(), arm_position(), g2),
        SpeedType::MMSec,
        50.0,
        TermType::CNT,
        50,
    );
    let v: Value = serde_json::to_value(&m).unwrap();
    assert_eq!(v["G1"]["Position"]["X"], 100.0);
    assert_eq!(v["G2"]["Position"]["X"], 1.0);
    assert!(v.get("COORD").is_none(), "with_groups does not force COORD");
    let back: FrcLinearMotion = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
    assert_eq!(m, back);
}

// ----- lenient-in: accept the legacy/sim plural "JointAngles" on deserialize -----

#[test]
fn deserializes_plural_jointangles_alias() {
    // A packet using the (wrong) plural key, as our simulator emits today.
    let raw = r#"{
        "SequenceID": 3,
        "G1": { "Configuration": {"UToolNumber":1,"UFrameNumber":1,"Front":1,"Up":1,"Left":1,"Flip":0,"Turn4":0,"Turn5":0,"Turn6":0},
                "Position": {"X":1.0,"Y":2.0,"Z":3.0} },
        "G2": { "JointAngles": {"J1":12.0,"J2":34.0,"J3":0,"J4":0,"J5":0,"J6":0} },
        "SpeedType": "mmSec", "Speed": 50.0, "TermType": "FINE", "TermValue": 0
    }"#;
    let m: FrcLinearMotion = serde_json::from_str(raw).unwrap();
    match &m.groups {
        CartesianGroups::Multi(blocks) => match &blocks[1].1 {
            GroupBlock::Joint { joint_angle } => {
                assert_eq!(joint_angle.j1, 12.0);
                assert_eq!(joint_angle.j2, 34.0);
            }
            _ => panic!("G2 should be joint"),
        },
        _ => panic!("expected Multi"),
    }
    // correct-out: re-serialization emits singular.
    let v: Value = serde_json::to_value(&m).unwrap();
    assert_eq!(v["G2"]["JointAngle"]["J1"].as_f64(), Some(12.0));
    assert!(v["G2"].get("JointAngles").is_none());
}

// ----- Instruction enum: tag injection stays intact through flatten -----

#[test]
fn through_instruction_enum_tag_and_shape() {
    let m = FrcLinearMotion::single(
        1,
        Configuration::default(),
        arm_position(),
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        0,
    );
    let instr = Instruction::FrcLinearMotion(m.clone());
    let v: Value = serde_json::to_value(&instr).unwrap();
    assert_eq!(v["Instruction"], "FRC_LinearMotion");
    assert_eq!(v["Position"]["X"], 100.0); // flat, alongside the tag
    let back: Instruction = serde_json::from_value(v).unwrap();
    assert_eq!(instr, back);
}

// ----- DTO must be bincode-safe (non-self-describing) -----

#[cfg(feature = "DTO")]
#[test]
fn dto_bincode_round_trips_single_and_multi() {
    use fanuc_rmi::instructions::dto::FrcLinearMotion as FrcLinearMotionDto;

    // single
    let single = FrcLinearMotion::single(
        5,
        Configuration::default(),
        arm_position(),
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        0,
    );
    let dto: FrcLinearMotionDto = single.clone().into();
    let bytes = bincode::serialize(&dto).expect("DTO must serialize in bincode");
    let back_dto: FrcLinearMotionDto = bincode::deserialize(&bytes).expect("DTO must deserialize in bincode");
    let back: FrcLinearMotion = back_dto.into();
    assert_eq!(single, back);

    // multi (arm + positioner)
    let positioner = JointAngles { j1: 45.0, j2: 10.0, ..Default::default() };
    let multi = FrcLinearMotion::coordinated(
        7,
        Configuration::default(),
        arm_position(),
        GroupBlock::joint(positioner),
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        0,
    );
    let dto: FrcLinearMotionDto = multi.clone().into();
    let bytes = bincode::serialize(&dto).expect("multi DTO must serialize in bincode");
    let back_dto: FrcLinearMotionDto = bincode::deserialize(&bytes).unwrap();
    let back: FrcLinearMotion = back_dto.into();
    assert_eq!(multi, back);
}
