#![cfg(feature = "DTO")]

use fanuc_rmi::{dto, protocol};

#[test]
fn roundtrip_simple_structs() {
    let pos = dto::Position { x: 1.0, y: 2.0, z: 3.0, w: 4.0, p: 5.0, r: 6.0, ext1: 0.1, ext2: 0.2, ext3: 0.3 };
    let conf = dto::Configuration { u_tool_number: 1, u_frame_number: 1, front: 1, up: 1, left: 1, flip: 0, turn4: 0, turn5: 0, turn6: 0 };

    // Serialize with bincode and deserialize
    let enc = bincode::serialize(&pos).unwrap();
    let dec: dto::Position = bincode::deserialize(&enc).unwrap();
    assert_eq!(pos, dec);

    let enc = bincode::serialize(&conf).unwrap();
    let dec: dto::Configuration = bincode::deserialize(&enc).unwrap();
    assert_eq!(conf, dec);
}

#[test]
fn roundtrip_read_cartesian_response() {
    let pos = dto::Position { x: 1.0, y: 2.0, z: 3.0, w: 4.0, p: 5.0, r: 6.0, ext1: 0.0, ext2: 0.0, ext3: 0.0 };
    let conf = dto::Configuration { u_tool_number: 1, u_frame_number: 1, front: 1, up: 1, left: 1, flip: 0, turn4: 0, turn5: 0, turn6: 0 };
    let resp = dto::FrcReadCartesianPositionResponse { error_id: 0, time_tag: 123, config: conf, pos, group: 1 };

    let enc = bincode::serialize(&resp).unwrap();
    let dec: dto::FrcReadCartesianPositionResponse = bincode::deserialize(&enc).unwrap();
    assert_eq!(resp, dec);
}

#[test]
fn roundtrip_read_cartesian_request() {
    let req = dto::FrcReadCartesianPosition { group: 1 };
    let enc = bincode::serialize(&req).unwrap();
    let dec: dto::FrcReadCartesianPosition = bincode::deserialize(&enc).unwrap();
    assert_eq!(req, dec);
}

#[test]
fn convert_protocol_to_dto_and_back() {
    // Build a protocol type, convert to dto, roundtrip, then back to protocol
    let p_pos = protocol::Position { x: 7.0, y:8.0, z:9.0, w:1.0, p:2.0, r:3.0, ext1:0.0, ext2:0.0, ext3:0.0 };
    let p_conf = protocol::Configuration::default();
    let p_resp = fanuc_rmi::commands::FrcReadCartesianPositionResponse { error_id: 0, time_tag: 42, config: p_conf, pos: p_pos, group: 1 };

    let dto_resp: dto::FrcReadCartesianPositionResponse = p_resp.into();
    let enc = bincode::serialize(&dto_resp).unwrap();
    let dec: dto::FrcReadCartesianPositionResponse = bincode::deserialize(&enc).unwrap();

    let p_roundtrip: protocol::commands::FrcReadCartesianPositionResponse = dec.into();
    
    // Field-by-field equality; derives PartialEq on protocol types so we can compare
    assert_eq!(p_roundtrip.error_id, 0);
    assert_eq!(p_roundtrip.time_tag, 42);
    assert_eq!(p_roundtrip.group, 1);
}

