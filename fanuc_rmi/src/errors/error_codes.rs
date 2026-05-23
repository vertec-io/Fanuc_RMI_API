//! FANUC RMI ErrorID lookup table.
//!
//! Source: B-84184EN/03 Appendix A.1 — RMI ErrorID Reference Table.
//! RMIT codes encode as `2556928 + N` for N = 1..=57. MEMO codes are bare ids.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RmiErrorInfo {
    pub code: &'static str,
    pub message: &'static str,
}

const TABLE: &[(u32, RmiErrorInfo)] = &[
    (2556929, RmiErrorInfo { code: "RMIT-001", message: "Internal System Error" }),
    (2556930, RmiErrorInfo { code: "RMIT-002", message: "Invalid UTool Number" }),
    (2556931, RmiErrorInfo { code: "RMIT-003", message: "Invalid UFrame Number" }),
    (2556932, RmiErrorInfo { code: "RMIT-004", message: "Invalid Position Register" }),
    (2556933, RmiErrorInfo { code: "RMIT-005", message: "Invalid Speed Override" }),
    (2556934, RmiErrorInfo { code: "RMIT-006", message: "Cannot Execute TP program" }),
    (2556935, RmiErrorInfo { code: "RMIT-007", message: "Controller Servo is Off" }),
    (2556936, RmiErrorInfo { code: "RMIT-008", message: "Teach Pendant is Enabled" }),
    (2556937, RmiErrorInfo { code: "RMIT-009", message: "RMI is Not Running" }),
    (2556938, RmiErrorInfo { code: "RMIT-010", message: "TP Program is Not Paused" }),
    (2556939, RmiErrorInfo { code: "RMIT-011", message: "Cannot Resume TP Program" }),
    (2556940, RmiErrorInfo { code: "RMIT-012", message: "Cannot Reset Controller" }),
    (2556941, RmiErrorInfo { code: "RMIT-013", message: "Invalid RMI Command" }),
    (2556942, RmiErrorInfo { code: "RMIT-014", message: "RMI Command Fail" }),
    (2556943, RmiErrorInfo { code: "RMIT-015", message: "Invalid Controller State" }),
    (2556944, RmiErrorInfo { code: "RMIT-016", message: "Please Cycle Power" }),
    (2556945, RmiErrorInfo { code: "RMIT-017", message: "Invalid Payload Schedule" }),
    (2556946, RmiErrorInfo { code: "RMIT-018", message: "Invalid Motion Option" }),
    (2556947, RmiErrorInfo { code: "RMIT-019", message: "Invalid Vision Register" }),
    (2556948, RmiErrorInfo { code: "RMIT-020", message: "Invalid RMI Instruction" }),
    (2556949, RmiErrorInfo { code: "RMIT-021", message: "Invalid Value" }),
    (2556950, RmiErrorInfo { code: "RMIT-022", message: "Invalid Text String" }),
    (2556951, RmiErrorInfo { code: "RMIT-023", message: "Invalid Position Data" }),
    (2556952, RmiErrorInfo { code: "RMIT-024", message: "RMI is In HOLD State" }),
    (2556953, RmiErrorInfo { code: "RMIT-025", message: "Remote Device Disconnected" }),
    (2556954, RmiErrorInfo { code: "RMIT-026", message: "Robot is Already Connected" }),
    (2556955, RmiErrorInfo { code: "RMIT-027", message: "Wait for Command Done" }),
    (2556956, RmiErrorInfo { code: "RMIT-028", message: "Wait for Instruction Done" }),
    (2556957, RmiErrorInfo { code: "RMIT-029", message: "Invalid sequence ID number" }),
    (2556958, RmiErrorInfo { code: "RMIT-030", message: "Invalid Speed Type" }),
    (2556959, RmiErrorInfo { code: "RMIT-031", message: "Invalid Speed Value" }),
    (2556960, RmiErrorInfo { code: "RMIT-032", message: "Invalid Term Type" }),
    (2556961, RmiErrorInfo { code: "RMIT-033", message: "Invalid Term Value" }),
    (2556962, RmiErrorInfo { code: "RMIT-034", message: "Invalid LCB Port Type" }),
    (2556963, RmiErrorInfo { code: "RMIT-035", message: "Invalid ACC Value" }),
    (2556964, RmiErrorInfo { code: "RMIT-036", message: "Invalid Destination Position" }),
    (2556965, RmiErrorInfo { code: "RMIT-037", message: "Invalid VIA Position" }),
    (2556966, RmiErrorInfo { code: "RMIT-038", message: "Invalid Port Number" }),
    (2556967, RmiErrorInfo { code: "RMIT-039", message: "Invalid Group Number" }),
    (2556968, RmiErrorInfo { code: "RMIT-040", message: "Invalid Group Mask" }),
    (2556969, RmiErrorInfo { code: "RMIT-041", message: "Joint motion with COORD" }),
    (2556970, RmiErrorInfo { code: "RMIT-042", message: "Incremental motion with COORD" }),
    (2556971, RmiErrorInfo { code: "RMIT-043", message: "Robot in Single Step Mode" }),
    (2556972, RmiErrorInfo { code: "RMIT-044", message: "Invalid Position Data Type" }),
    (2556973, RmiErrorInfo { code: "RMIT-045", message: "Not Ready for ASCII Packet" }),
    (2556974, RmiErrorInfo { code: "RMIT-046", message: "ASCII Conversion Failed" }),
    (2556975, RmiErrorInfo { code: "RMIT-047", message: "Invalid ASCII Instruction" }),
    (2556976, RmiErrorInfo { code: "RMIT-048", message: "Invalid Number of Groups" }),
    (2556977, RmiErrorInfo { code: "RMIT-049", message: "Invalid Instruction packet" }),
    (2556978, RmiErrorInfo { code: "RMIT-050", message: "Invalid ASCII packet" }),
    (2556979, RmiErrorInfo { code: "RMIT-051", message: "Invalid ASCII string size" }),
    (2556980, RmiErrorInfo { code: "RMIT-052", message: "Invalid Application Tool" }),
    (2556981, RmiErrorInfo { code: "RMIT-053", message: "Invalid Call Program Name" }),
    (2556982, RmiErrorInfo { code: "RMIT-054", message: "Joint Motion with ALIM" }),
    (2556983, RmiErrorInfo { code: "RMIT-055", message: "ALIM option is not loaded" }),
    (2556984, RmiErrorInfo { code: "RMIT-056", message: "Need to finish S-motion" }),
    (2556985, RmiErrorInfo { code: "RMIT-057", message: "Spline option is not loaded" }),
    (7004, RmiErrorInfo { code: "MEMO-004", message: "Specific program is in use" }),
    (7015, RmiErrorInfo { code: "MEMO-015", message: "Program already exists" }),
];

pub fn decode_error_id(error_id: u32) -> Option<RmiErrorInfo> {
    TABLE.iter().find(|(id, _)| *id == error_id).map(|(_, info)| *info)
}

/// Format an ErrorID for human-readable display.
/// Returns e.g. "RMIT-015 Invalid Controller State (2556943)" or "ErrorID 12345 (unknown)".
pub fn format_error_id(error_id: u32) -> String {
    match decode_error_id(error_id) {
        Some(info) => format!("{} {} ({})", info.code, info.message, error_id),
        None => format!("ErrorID {} (unknown)", error_id),
    }
}

/// Best-effort: scan a raw JSON snippet for `"ErrorID" : <number>` and decode it.
/// Used in fallback log paths where full deserialization failed.
pub fn extract_and_format_error_id(raw_json: &str) -> Option<String> {
    let key = "\"ErrorID\"";
    let idx = raw_json.find(key)?;
    let after = &raw_json[idx + key.len()..];
    let after = after.trim_start_matches(|c: char| c.is_whitespace() || c == ':');
    let end = after.find(|c: char| !c.is_ascii_digit()).unwrap_or(after.len());
    let id: u32 = after[..end].parse().ok()?;
    if id == 0 { return None; }
    Some(format_error_id(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_known_codes() {
        assert_eq!(decode_error_id(2556943).unwrap().code, "RMIT-015");
        assert_eq!(decode_error_id(2556955).unwrap().code, "RMIT-027");
        assert_eq!(decode_error_id(7015).unwrap().code, "MEMO-015");
        assert!(decode_error_id(0).is_none());
        assert!(decode_error_id(99999999).is_none());
    }

    #[test]
    fn extracts_from_raw_json() {
        let raw = r#"{"Command" : "FRC_ReadJointAngles", "ErrorID" : 2556955, "TimeTag": 0}"#;
        let s = extract_and_format_error_id(raw).unwrap();
        assert!(s.contains("RMIT-027"));
        assert!(s.contains("Wait for Command Done"));
    }

    #[test]
    fn ignores_zero_error() {
        let raw = r#"{"ErrorID" : 0, "TimeTag": 100}"#;
        assert!(extract_and_format_error_id(raw).is_none());
    }
}
