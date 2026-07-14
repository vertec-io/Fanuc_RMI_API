//! Tests for FANUC Group-2 (coordinated positioner) support primitives:
//! the `GroupMask` bitmask, `FRC_Initialize` group selection, the optional
//! (UNTESTED) motion-instruction group field, and numeric-register commands.

use fanuc_rmi::commands::{FrcInitialize, FrcReadRegister, FrcWriteRegister};
use fanuc_rmi::instructions::FrcLinearMotion;
use fanuc_rmi::{Configuration, GroupMask, Position, SpeedType, TermType};

#[test]
fn group_mask_helpers() {
    assert_eq!(GroupMask::default(), GroupMask::GROUP_1);
    assert_eq!(GroupMask::GROUP_1.bits(), 0b0000_0001);
    assert_eq!(GroupMask::GROUP_2.bits(), 0b0000_0010);

    // 1-based group-number construction
    assert_eq!(GroupMask::from_group(1), GroupMask::GROUP_1);
    assert_eq!(GroupMask::from_group(2), GroupMask::GROUP_2);
    assert_eq!(GroupMask::from_group(0), GroupMask::empty());
    assert_eq!(GroupMask::from_group(9), GroupMask::empty());

    let both = GroupMask::GROUP_1 | GroupMask::GROUP_2;
    assert_eq!(both.bits(), 0b0000_0011);
    assert!(both.is_multi_group());
    assert!(!GroupMask::GROUP_1.is_multi_group());
    assert!(both.contains(GroupMask::GROUP_2));
    assert!(both.contains_group(2));
    assert!(!GroupMask::GROUP_1.contains_group(2));
    assert_eq!(both.count(), 2);
    assert_eq!(both.groups().collect::<Vec<_>>(), vec![1, 2]);
}

#[test]
fn group_mask_is_serde_transparent() {
    // Serializes as a bare integer (wire-compatible with the raw GroupMask byte).
    let v = serde_json::to_value(GroupMask::from_bits(3)).unwrap();
    assert_eq!(v, serde_json::json!(3));

    let back: GroupMask = serde_json::from_value(serde_json::json!(2)).unwrap();
    assert_eq!(back, GroupMask::GROUP_2);
}

#[test]
fn initialize_serializes_group_mask_as_integer() {
    let init = FrcInitialize::new(Some(GroupMask::GROUP_1 | GroupMask::GROUP_2));
    let s = serde_json::to_string(&init).unwrap();
    assert!(s.contains("\"GroupMask\":3"), "got: {s}");

    // Default is Group 1.
    let s = serde_json::to_string(&FrcInitialize::default()).unwrap();
    assert!(s.contains("\"GroupMask\":1"), "got: {s}");

    // Raw-bits constructor.
    let s = serde_json::to_string(&FrcInitialize::from_bits(0x03)).unwrap();
    assert!(s.contains("\"GroupMask\":3"), "got: {s}");
}

#[test]
fn single_group_motion_is_flat_with_no_groupmask_key() {
    // Single-group motion serializes FLAT (§2.4.7). `GroupMask` belongs on
    // FRC_Initialize (§2.3.1), NOT on motion packets.
    let m = FrcLinearMotion::new(
        1,
        Configuration::default(),
        Position::default(),
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        1,
    );
    let s = serde_json::to_string(&m).unwrap();
    assert!(!s.contains("GroupMask"), "motion packets carry no GroupMask key: {s}");
    assert!(s.contains("\"Configuration\""), "single-group is flat: {s}");
    assert!(!s.contains("\"G1\""), "single-group is not wrapped: {s}");
}

#[test]
fn two_group_motion_wraps_g1_g2_with_coord() {
    use fanuc_rmi::instructions::GroupBlock;
    use fanuc_rmi::JointAngles;
    // Arm (G1, Cartesian) + positioner (G2, joint) coordinated motion (§2.4.7.1).
    let m = FrcLinearMotion::coordinated(
        1,
        Configuration::default(),
        Position::default(),
        GroupBlock::joint(JointAngles { j1: 30.0, ..Default::default() }),
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        1,
    );
    let s = serde_json::to_string(&m).unwrap();
    assert!(s.contains("\"G1\""), "two-group wraps in G1: {s}");
    assert!(s.contains("\"G2\""), "two-group wraps in G2: {s}");
    assert!(s.contains("\"COORD\":\"ON\""), "coordinated sets COORD: {s}");
    assert!(!s.contains("GroupMask"), "still no GroupMask on the motion packet: {s}");
}

#[test]
fn numeric_register_commands_roundtrip() {
    let w = FrcWriteRegister::new(5, 12.5);
    let s = serde_json::to_string(&w).unwrap();
    assert!(s.contains("\"RegisterNumber\":5"));
    assert!(s.contains("\"RegisterValue\":12.5"));

    let r = FrcReadRegister::new(7);
    let s = serde_json::to_string(&r).unwrap();
    assert!(s.contains("\"RegisterNumber\":7"));
}
