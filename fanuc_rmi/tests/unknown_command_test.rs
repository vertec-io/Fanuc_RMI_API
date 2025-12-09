use fanuc_rmi::packets::{ResponsePacket, CommandResponse};
use fanuc_rmi::commands::FrcUnknownResponse;

#[test]
fn test_unknown_command_response_deserialization() {
    // This is the actual JSON that the robot sends when it doesn't recognize a command
    let json = r#"{"Command" : "Unknown", "ErrorID" : 2556950}"#;
    
    let result: Result<ResponsePacket, _> = serde_json::from_str(json);
    
    assert!(result.is_ok(), "Failed to deserialize unknown command response: {:?}", result.err());
    
    let packet = result.unwrap();
    
    match packet {
        ResponsePacket::CommandResponse(CommandResponse::Unknown(response)) => {
            assert_eq!(response.error_id, 2556950, "Error ID should be 2556950 (InvalidTextString)");
        }
        _ => panic!("Expected Unknown command response, got: {:?}", packet),
    }
}

#[test]
fn test_unknown_command_with_different_error_code() {
    // Test with InvalidRMICommand error code
    let json = r#"{"Command" : "Unknown", "ErrorID" : 2556941}"#;
    
    let result: Result<ResponsePacket, _> = serde_json::from_str(json);
    
    assert!(result.is_ok(), "Failed to deserialize unknown command response");
    
    let packet = result.unwrap();
    
    match packet {
        ResponsePacket::CommandResponse(CommandResponse::Unknown(response)) => {
            assert_eq!(response.error_id, 2556941, "Error ID should be 2556941 (InvalidRMICommand)");
        }
        _ => panic!("Expected Unknown command response, got: {:?}", packet),
    }
}

#[test]
fn test_unknown_command_serialization() {
    let response = FrcUnknownResponse {
        error_id: 2556950,
    };
    
    let command_response = CommandResponse::Unknown(response);
    let json = serde_json::to_string(&command_response).unwrap();
    
    // Verify it can be deserialized back
    let deserialized: CommandResponse = serde_json::from_str(&json).unwrap();
    
    match deserialized {
        CommandResponse::Unknown(resp) => {
            assert_eq!(resp.error_id, 2556950);
        }
        _ => panic!("Expected Unknown variant"),
    }
}

