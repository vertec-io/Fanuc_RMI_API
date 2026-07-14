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
fn motion_group_field_is_omitted_by_default() {
    let m = FrcLinearMotion::new(
        1,
        Configuration::default(),
        Position::default(),
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        1,
    );
    // No group by default -> byte-identical to the documented single-group packet.
    assert_eq!(m.group, None);
    let s = serde_json::to_string(&m).unwrap();
    assert!(!s.contains("GroupMask"), "expected no group field, got: {s}");

    // Opt-in via builder makes the wire format express the (UNTESTED) selector.
    let m = m.with_group(GroupMask::GROUP_2);
    assert_eq!(m.group, Some(GroupMask::GROUP_2));
    let s = serde_json::to_string(&m).unwrap();
    assert!(s.contains("\"GroupMask\":2"), "got: {s}");
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
