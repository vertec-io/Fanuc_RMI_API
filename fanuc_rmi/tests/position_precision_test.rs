/// Test to demonstrate precision loss with f32 vs f64 for position data
use fanuc_rmi::Position;
use serde_json;

#[test]
fn test_f32_precision_loss() {
    // Typical FANUC position values (in millimeters)
    let test_values = vec![
        1234.567,
        -987.654,
        0.001,
        1000.0001,
        12345.6789,
    ];
    
    for value in test_values {
        // Simulate what happens with f32
        let as_f32 = value as f32;
        let back_to_f64 = as_f32 as f64;
        
        let diff = (value - back_to_f64).abs();
        
        println!("Original: {:.10}", value);
        println!("As f32:   {:.10}", as_f32);
        println!("Back:     {:.10}", back_to_f64);
        println!("Diff:     {:.10}", diff);
        println!();
        
        // f32 can lose precision beyond ~7 significant digits
        if diff > 0.0001 {
            println!("⚠️  Significant precision loss detected!");
        }
    }
}

#[test]
fn test_json_roundtrip_precision() {
    // Create a position with precise values
    let original = Position {
        x: 1234.5678,
        y: -987.6543,
        z: 456.7891,
        w: 12.3456,
        p: -45.6789,
        r: 90.1234,
        ext1: 0.0,
        ext2: 0.0,
        ext3: 0.0,
    };
    
    // Serialize to JSON
    let json = serde_json::to_string(&original).unwrap();
    println!("JSON: {}", json);
    
    // Deserialize back
    let deserialized: Position = serde_json::from_str(&json).unwrap();
    
    // Check precision loss
    println!("\nOriginal vs Deserialized:");
    println!("X: {:.10} vs {:.10} (diff: {:.10})", original.x, deserialized.x, (original.x - deserialized.x).abs());
    println!("Y: {:.10} vs {:.10} (diff: {:.10})", original.y, deserialized.y, (original.y - deserialized.y).abs());
    println!("Z: {:.10} vs {:.10} (diff: {:.10})", original.z, deserialized.z, (original.z - deserialized.z).abs());
    println!("W: {:.10} vs {:.10} (diff: {:.10})", original.w, deserialized.w, (original.w - deserialized.w).abs());
    println!("P: {:.10} vs {:.10} (diff: {:.10})", original.p, deserialized.p, (original.p - deserialized.p).abs());
    println!("R: {:.10} vs {:.10} (diff: {:.10})", original.r, deserialized.r, (original.r - deserialized.r).abs());
    
    // The values should be exactly equal after roundtrip
    assert_eq!(original, deserialized);
}

#[test]
fn test_fanuc_response_precision() {
    // Simulate a FANUC response with high-precision values
    let json_response = r#"{
        "X": 1234.567890,
        "Y": -987.654321,
        "Z": 456.789012,
        "W": 12.345678,
        "P": -45.678901,
        "R": 90.123456
    }"#;
    
    let position: Position = serde_json::from_str(json_response).unwrap();
    
    println!("Parsed position:");
    println!("X: {:.10}", position.x);
    println!("Y: {:.10}", position.y);
    println!("Z: {:.10}", position.z);
    println!("W: {:.10}", position.w);
    println!("P: {:.10}", position.p);
    println!("R: {:.10}", position.r);
    
    // Serialize back to JSON
    let json_out = serde_json::to_string_pretty(&position).unwrap();
    println!("\nSerialized back to JSON:\n{}", json_out);
    
    // Check if we lost precision
    // f32 has ~7 decimal digits of precision, so values beyond that will be truncated
    // For example, 1234.567890 might become 1234.5679 (rounded)
}

