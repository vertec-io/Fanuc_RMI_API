use fanuc_rmi::commands::FrcReadUToolData;
use fanuc_rmi::packets::{Command, SendPacket, CommandResponse, ResponsePacket};

#[test]
fn test_read_utool_data_json_format() {
    // Create the command
    let cmd = FrcReadUToolData::new(None, 4);

    // Wrap in Command enum
    let command = Command::FrcReadUToolData(cmd);

    // Serialize to JSON
    let json = serde_json::to_string(&command).unwrap();

    println!("Generated JSON: {}", json);

    // Verify it contains "ToolNumber" not "FrameNumber"
    assert!(json.contains("\"ToolNumber\""), "JSON should contain 'ToolNumber' field");
    assert!(!json.contains("\"FrameNumber\""), "JSON should NOT contain 'FrameNumber' field");

    // Verify the command name
    assert!(json.contains("\"FRC_ReadUToolData\""), "JSON should contain command name");

    // Verify the tool number value
    assert!(json.contains("\"ToolNumber\":4") || json.contains("\"ToolNumber\" : 4"),
            "JSON should contain ToolNumber with value 4");
}

#[test]
fn test_read_utool_data_matches_manual_format() {
    // According to B-84184EN_02.pdf section 2.3.10, the format should be:
    // {"Command": "FRC_ReadUToolData", "ToolNumber": byteValue, "Group": byteValue2}

    let cmd = FrcReadUToolData::new(Some(1), 4);
    let command = Command::FrcReadUToolData(cmd);
    let json = serde_json::to_string(&command).unwrap();

    // Parse it back to verify structure
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["Command"], "FRC_ReadUToolData");
    assert_eq!(parsed["ToolNumber"], 4);
    assert_eq!(parsed["Group"], 1);
}

#[test]
fn test_send_packet_format() {
    // Test the full SendPacket format
    let cmd = FrcReadUToolData::new(None, 4);
    let packet = SendPacket::Command(Command::FrcReadUToolData(cmd));

    let json = serde_json::to_string(&packet).unwrap();

    println!("SendPacket JSON: {}", json);

    // Verify it contains the correct field name
    assert!(json.contains("\"ToolNumber\""), "SendPacket JSON should contain 'ToolNumber' field");
    assert!(!json.contains("\"FrameNumber\""), "SendPacket JSON should NOT contain 'FrameNumber' field");
}

#[test]
fn test_read_uframe_data_response_from_real_robot() {
    // This is the actual JSON from the real robot (from the error log)
    let robot_json = r#"{"Command" : "FRC_ReadUFrameData", "ErrorID" : 0, "FrameNumber" : 1, "Group" : 1, "Frame" : { "X" : 615.596, "Y" : -135.698, "Z" : 1019.187, "W" : 168.771, "P" : 3.647, "R" : -100.887}}"#;

    // First try to deserialize as CommandResponse
    let cmd_result: Result<CommandResponse, _> = serde_json::from_str(robot_json);
    match &cmd_result {
        Ok(resp) => println!("Successfully parsed as CommandResponse: {:?}", resp),
        Err(e) => println!("Failed to parse as CommandResponse: {}", e),
    }

    // Try to deserialize as ResponsePacket
    let result: Result<ResponsePacket, _> = serde_json::from_str(robot_json);

    match &result {
        Ok(packet) => println!("Successfully parsed as ResponsePacket: {:?}", packet),
        Err(e) => println!("Failed to parse as ResponsePacket: {}", e),
    }

    assert!(result.is_ok(), "Should successfully parse real robot response");

    // Verify it's the correct variant
    if let Ok(ResponsePacket::CommandResponse(CommandResponse::FrcReadUFrameData(resp))) = result {
        assert_eq!(resp.error_id, 0);
        assert_eq!(resp.frame_number, 1);
        assert_eq!(resp.group, 1);
        assert_eq!(resp.frame.x, 615.596);
        assert_eq!(resp.frame.y, -135.698);
        assert_eq!(resp.frame.z, 1019.187);
        assert_eq!(resp.frame.w, 168.771);
        assert_eq!(resp.frame.p, 3.647);
        assert_eq!(resp.frame.r, -100.887);
    } else {
        panic!("Wrong response variant");
    }
}

#[test]
fn test_read_utool_data_response_from_real_robot() {
    // This is the actual JSON from the real robot (from the error log)
    let robot_json = r#"{"Command" : "FRC_ReadUToolData", "ErrorID" : 0, "ToolNumber" : 2, "Group" : 1, "Frame" : { "X" : 0.000, "Y" : 0.000, "Z" : 0.000, "W" : 0.000, "P" : 0.000, "R" : 0.000}}"#;

    // First try to deserialize as CommandResponse
    let cmd_result: Result<CommandResponse, _> = serde_json::from_str(robot_json);
    match &cmd_result {
        Ok(resp) => println!("Successfully parsed as CommandResponse: {:?}", resp),
        Err(e) => println!("Failed to parse as CommandResponse: {}", e),
    }

    // Try to deserialize as ResponsePacket
    let result: Result<ResponsePacket, _> = serde_json::from_str(robot_json);

    match &result {
        Ok(packet) => println!("Successfully parsed as ResponsePacket: {:?}", packet),
        Err(e) => println!("Failed to parse as ResponsePacket: {}", e),
    }

    assert!(result.is_ok(), "Should successfully parse real robot response");

    // Verify it's the correct variant
    if let Ok(ResponsePacket::CommandResponse(CommandResponse::FrcReadUToolData(resp))) = result {
        assert_eq!(resp.error_id, 0);
        assert_eq!(resp.tool_number, 2);
        assert_eq!(resp.group, 1);
        assert_eq!(resp.frame.x, 0.0);
        assert_eq!(resp.frame.y, 0.0);
        assert_eq!(resp.frame.z, 0.0);
        assert_eq!(resp.frame.w, 0.0);
        assert_eq!(resp.frame.p, 0.0);
        assert_eq!(resp.frame.r, 0.0);
    } else {
        panic!("Wrong response variant");
    }
}

