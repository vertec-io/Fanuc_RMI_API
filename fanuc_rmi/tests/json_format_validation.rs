/// Test to validate JSON serialization matches FANUC RMI specification
/// Reference: FANUC B-84184EN_02 specification
use fanuc_rmi::{Configuration, Position};
use serde_json;

#[test]
fn test_configuration_json_format() {
    // Create a Configuration with known values
    let config = Configuration {
        u_tool_number: 1,
        u_frame_number: 2,
        front: 1,
        up: 1,
        left: 0,
        flip: 0,
        turn4: 0,
        turn5: 0,
        turn6: 0,
    };

    // Serialize to JSON
    let json = serde_json::to_string(&config).unwrap();

    // Print for verification (visible with --nocapture)
    println!("\n=== Configuration JSON Output ===");
    println!("{}", serde_json::to_string_pretty(&config).unwrap());
    println!("=================================\n");
    
    // Parse back to verify field names
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    
    // Verify all required fields are present with correct PascalCase names
    // as specified in FANUC B-84184EN_02
    assert!(value.get("UToolNumber").is_some(), "Missing UToolNumber field");
    assert!(value.get("UFrameNumber").is_some(), "Missing UFrameNumber field");
    assert!(value.get("Front").is_some(), "Missing Front field");
    assert!(value.get("Up").is_some(), "Missing Up field");
    assert!(value.get("Left").is_some(), "Missing Left field");
    assert!(value.get("Flip").is_some(), "Missing Flip field");
    assert!(value.get("Turn4").is_some(), "Missing Turn4 field");
    assert!(value.get("Turn5").is_some(), "Missing Turn5 field");
    assert!(value.get("Turn6").is_some(), "Missing Turn6 field");
    
    // Verify values
    assert_eq!(value["UToolNumber"], 1);
    assert_eq!(value["UFrameNumber"], 2);
    assert_eq!(value["Front"], 1);
    assert_eq!(value["Up"], 1);
    assert_eq!(value["Left"], 0);
    assert_eq!(value["Flip"], 0);
    assert_eq!(value["Turn4"], 0);
    assert_eq!(value["Turn5"], 0);
    assert_eq!(value["Turn6"], 0);
    
    // Verify no incorrect field names exist
    assert!(value.get("F").is_none(), "Incorrect field name 'F' found");
    assert!(value.get("U").is_none(), "Incorrect field name 'U' found");
    assert!(value.get("T").is_none(), "Incorrect field name 'T' found");
    assert!(value.get("B1").is_none(), "Incorrect field name 'B1' found");
    assert!(value.get("B2").is_none(), "Incorrect field name 'B2' found");
    assert!(value.get("B3").is_none(), "Incorrect field name 'B3' found");
}

#[test]
fn test_configuration_default_json_format() {
    // Test that default Configuration serializes correctly
    let config = Configuration::default();
    let json = serde_json::to_string(&config).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    
    // Verify default values match FANUC specification
    assert_eq!(value["UToolNumber"], 1);
    assert_eq!(value["UFrameNumber"], 1);
    assert_eq!(value["Front"], 1);
    assert_eq!(value["Up"], 1);
    assert_eq!(value["Left"], 1);
    assert_eq!(value["Flip"], 0);
    assert_eq!(value["Turn4"], 0);
    assert_eq!(value["Turn5"], 0);
    assert_eq!(value["Turn6"], 0);
}

#[test]
fn test_position_json_format() {
    // Verify Position struct also uses correct PascalCase
    let pos = Position {
        x: 100.0,
        y: 200.0,
        z: 300.0,
        w: 0.0,
        p: 90.0,
        r: 0.0,
        ext1: 0.0,
        ext2: 0.0,
        ext3: 0.0,
    };
    
    let json = serde_json::to_string(&pos).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    
    // Verify PascalCase field names as per FANUC spec
    assert!(value.get("X").is_some(), "Missing X field");
    assert!(value.get("Y").is_some(), "Missing Y field");
    assert!(value.get("Z").is_some(), "Missing Z field");
    assert!(value.get("W").is_some(), "Missing W field");
    assert!(value.get("P").is_some(), "Missing P field");
    assert!(value.get("R").is_some(), "Missing R field");
    assert!(value.get("Ext1").is_some(), "Missing Ext1 field");
    assert!(value.get("Ext2").is_some(), "Missing Ext2 field");
    assert!(value.get("Ext3").is_some(), "Missing Ext3 field");
}

#[test]
fn test_configuration_deserialization_from_fanuc_json() {
    // Test that we can deserialize JSON in FANUC format
    let fanuc_json = r#"{
        "UToolNumber": 1,
        "UFrameNumber": 1,
        "Front": 1,
        "Up": 1,
        "Left": 1,
        "Flip": 0,
        "Turn4": 0,
        "Turn5": 0,
        "Turn6": 0
    }"#;
    
    let config: Configuration = serde_json::from_str(fanuc_json).unwrap();
    
    assert_eq!(config.u_tool_number, 1);
    assert_eq!(config.u_frame_number, 1);
    assert_eq!(config.front, 1);
    assert_eq!(config.up, 1);
    assert_eq!(config.left, 1);
    assert_eq!(config.flip, 0);
    assert_eq!(config.turn4, 0);
    assert_eq!(config.turn5, 0);
    assert_eq!(config.turn6, 0);
}

#[test]
fn test_configuration_roundtrip_json() {
    // Test that serialization and deserialization are symmetric
    let original = Configuration {
        u_tool_number: 3,
        u_frame_number: 5,
        front: 1,
        up: 0,
        left: 1,
        flip: 1,
        turn4: 1,
        turn5: 0,
        turn6: 1,
    };
    
    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Configuration = serde_json::from_str(&json).unwrap();
    
    assert_eq!(original, deserialized);
}

