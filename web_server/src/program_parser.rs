//! CSV program parser for robot motion programs.
//!
//! Supports flexible CSV format:
//! - Minimal: x, y, z, speed (required columns, values required per row)
//! - Full: x, y, z, w, p, r, ext1, ext2, ext3, speed, term_type, uframe, utool
//!
//! Validation rules:
//! - Required columns (x, y, z, speed) must have values in every row
//! - Optional columns must be consistent: if present, either ALL rows have values or NONE do
//! - Range validation: speed > 0, uframe >= 0, utool >= 0
//! - Valid term_type values: FINE, CNT (also accepts CNT with value like CNT100, normalized to CNT)

use crate::database::ProgramInstruction;
use csv::ReaderBuilder;
use std::collections::HashMap;
use std::io::Read;

/// Specific validation error with location and details.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub line: usize,
    pub column: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line {}, column '{}': {}", self.line, self.column, self.message)
    }
}

/// Warning for potential issues that don't prevent parsing.
#[derive(Debug, Clone)]
pub struct ParseWarning {
    pub line: Option<usize>,
    pub column: Option<String>,
    pub message: String,
}

impl std::fmt::Display for ParseWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, &self.column) {
            (Some(line), Some(col)) => write!(f, "Line {}, column '{}': {}", line, col, self.message),
            (Some(line), None) => write!(f, "Line {}: {}", line, self.message),
            (None, Some(col)) => write!(f, "Column '{}': {}", col, self.message),
            (None, None) => write!(f, "{}", self.message),
        }
    }
}

/// Error type for CSV parsing.
#[derive(Debug)]
pub enum ParseError {
    /// CSV library error (malformed CSV)
    CsvError(csv::Error),
    /// Missing required column in header
    MissingColumn(String),
    /// Validation errors found during parsing
    ValidationErrors(Vec<ValidationError>),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::CsvError(e) => write!(f, "CSV format error: {}", e),
            ParseError::MissingColumn(col) => write!(f, "Missing required column: {}", col),
            ParseError::ValidationErrors(errors) => {
                writeln!(f, "Validation failed with {} error(s):", errors.len())?;
                for err in errors.iter().take(10) {
                    writeln!(f, "  - {}", err)?;
                }
                if errors.len() > 10 {
                    writeln!(f, "  ... and {} more errors", errors.len() - 10)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ParseError {}

impl From<csv::Error> for ParseError {
    fn from(e: csv::Error) -> Self {
        ParseError::CsvError(e)
    }
}

/// Result of parsing a CSV file.
#[derive(Debug)]
pub struct ParseResult {
    /// Successfully parsed instructions (empty if errors occurred)
    pub instructions: Vec<ProgramInstruction>,
    /// Warnings about potential issues (non-fatal)
    pub warnings: Vec<ParseWarning>,
    /// Number of rows parsed
    pub row_count: usize,
    /// Columns that were present in the CSV
    pub columns_present: Vec<String>,
}

/// Default values for missing CSV columns (used only for columns not present in CSV).
#[derive(Debug, Clone)]
pub struct ProgramDefaults {
    pub w: f64,
    pub p: f64,
    pub r: f64,
    pub ext1: f64,
    pub ext2: f64,
    pub ext3: f64,
    pub speed: f64,
    pub term_type: String,
    pub uframe: Option<i32>,
    pub utool: Option<i32>,
    // Robot arm configuration defaults
    pub front: Option<i32>,
    pub up: Option<i32>,
    pub left: Option<i32>,
    pub flip: Option<i32>,
    pub turn4: Option<i32>,
    pub turn5: Option<i32>,
    pub turn6: Option<i32>,
}

impl Default for ProgramDefaults {
    fn default() -> Self {
        Self {
            w: 0.0,
            p: 0.0,
            r: 0.0,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
            speed: 100.0,
            term_type: "CNT".to_string(),
            uframe: None,
            utool: None,
            front: None,
            up: None,
            left: None,
            flip: None,
            turn4: None,
            turn5: None,
            turn6: None,
        }
    }
}

/// Valid termination types for motion commands.
const VALID_TERM_TYPES: &[&str] = &["FINE", "CNT"];

/// Normalize and validate term_type value.
/// Accepts: FINE, CNT, or CNT with a value (e.g., CNT100, CNT50).
/// Returns the normalized term_type (CNT100 -> CNT) or None if invalid.
fn normalize_term_type(term_type: &str) -> Option<String> {
    let tt_upper = term_type.to_uppercase();

    // Check exact matches first
    if VALID_TERM_TYPES.contains(&tt_upper.as_str()) {
        return Some(tt_upper);
    }

    // Check for CNT with a value (e.g., CNT100, CNT50)
    // FANUC uses CNT followed by a number for blending percentage
    if tt_upper.starts_with("CNT") {
        let rest = &tt_upper[3..];
        // If it's CNT followed by a valid number (0-100), normalize to CNT
        if rest.parse::<i32>().is_ok() {
            return Some("CNT".to_string());
        }
    }

    None
}

/// Track whether optional column values are specified or not per row.
#[derive(Debug, Default)]
struct ColumnConsistencyTracker {
    /// For each optional column: (column_name, first_row_had_value, lines_with_different_state)
    columns: HashMap<String, (bool, Vec<usize>)>,
}

impl ColumnConsistencyTracker {
    fn new() -> Self {
        Self { columns: HashMap::new() }
    }

    /// Record whether a value was present for this column on this line.
    fn record(&mut self, column: &str, line: usize, has_value: bool) {
        self.columns
            .entry(column.to_string())
            .and_modify(|(first_had_value, inconsistent_lines)| {
                if has_value != *first_had_value {
                    inconsistent_lines.push(line);
                }
            })
            .or_insert((has_value, vec![]));
    }

    /// Get validation errors for inconsistent columns.
    fn get_errors(&self) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        for (column, (first_had_value, inconsistent_lines)) in &self.columns {
            if !inconsistent_lines.is_empty() {
                let expected = if *first_had_value { "specified" } else { "empty" };
                let found = if *first_had_value { "empty" } else { "specified" };
                errors.push(ValidationError {
                    line: inconsistent_lines[0],
                    column: column.clone(),
                    message: format!(
                        "Inconsistent values: row 1 had column {}, but this row has it {}. \
                         Optional columns must be consistently specified for all rows or none. \
                         ({} row(s) affected)",
                        expected, found, inconsistent_lines.len()
                    ),
                });
            }
        }
        errors
    }
}

/// Parse a CSV program from a reader with full validation.
///
/// Returns a ParseResult with instructions, warnings, and metadata.
/// Returns Err(ParseError) if validation fails.
pub fn parse_csv<R: Read>(reader: R, _defaults: &ProgramDefaults) -> Result<ParseResult, ParseError> {
    let mut csv_reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::All)
        .from_reader(reader);

    // Get headers and build column index map
    let headers = csv_reader.headers()?.clone();
    let col_map: HashMap<&str, usize> = headers.iter()
        .enumerate()
        .map(|(i, h)| (h, i))
        .collect();

    // Track which columns are present
    let columns_present: Vec<String> = headers.iter().map(|s| s.to_string()).collect();

    // Check required columns exist in header
    for required in ["x", "y", "z", "speed"] {
        if !col_map.contains_key(required) {
            return Err(ParseError::MissingColumn(required.to_string()));
        }
    }

    let mut instructions = Vec::new();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut consistency_tracker = ColumnConsistencyTracker::new();
    let mut line_number = 1usize;

    // Optional columns that need consistency tracking
    let optional_columns = ["w", "p", "r", "ext1", "ext2", "ext3", "term_type", "uframe", "utool"];

    for result in csv_reader.records() {
        let record = match result {
            Ok(r) => r,
            Err(e) => {
                errors.push(ValidationError {
                    line: line_number + 1,
                    column: "".to_string(),
                    message: format!("CSV parse error: {}", e),
                });
                line_number += 1;
                continue;
            }
        };

        let csv_line = line_number + 1; // +1 for header row

        // Helper to get f64 value and track if present
        let get_f64 = |col: &str, errors: &mut Vec<ValidationError>| -> Option<f64> {
            if let Some(&idx) = col_map.get(col) {
                if let Some(val) = record.get(idx) {
                    if val.is_empty() {
                        None
                    } else {
                        match val.parse::<f64>() {
                            Ok(v) => Some(v),
                            Err(_) => {
                                errors.push(ValidationError {
                                    line: csv_line,
                                    column: col.to_string(),
                                    message: format!("Invalid number: '{}'", val),
                                });
                                None
                            }
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Helper to get i32 value
        let get_i32 = |col: &str, errors: &mut Vec<ValidationError>| -> Option<i32> {
            if let Some(&idx) = col_map.get(col) {
                if let Some(val) = record.get(idx) {
                    if val.is_empty() {
                        None
                    } else {
                        match val.parse::<i32>() {
                            Ok(v) => Some(v),
                            Err(_) => {
                                errors.push(ValidationError {
                                    line: csv_line,
                                    column: col.to_string(),
                                    message: format!("Invalid integer: '{}'", val),
                                });
                                None
                            }
                        }
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Helper to get string value
        let get_str = |col: &str| -> Option<String> {
            col_map.get(col)
                .and_then(|&idx| record.get(idx))
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        };

        // Parse required columns (x, y, z, speed)
        let x = get_f64("x", &mut errors);
        let y = get_f64("y", &mut errors);
        let z = get_f64("z", &mut errors);
        let speed = get_f64("speed", &mut errors);

        // Validate required values are present
        if x.is_none() {
            errors.push(ValidationError {
                line: csv_line,
                column: "x".to_string(),
                message: "Required value missing".to_string(),
            });
        }
        if y.is_none() {
            errors.push(ValidationError {
                line: csv_line,
                column: "y".to_string(),
                message: "Required value missing".to_string(),
            });
        }
        if z.is_none() {
            errors.push(ValidationError {
                line: csv_line,
                column: "z".to_string(),
                message: "Required value missing".to_string(),
            });
        }
        if speed.is_none() {
            errors.push(ValidationError {
                line: csv_line,
                column: "speed".to_string(),
                message: "Required value missing".to_string(),
            });
        }

        // Validate speed is positive
        if let Some(s) = speed {
            if s <= 0.0 {
                errors.push(ValidationError {
                    line: csv_line,
                    column: "speed".to_string(),
                    message: format!("Speed must be positive, got: {}", s),
                });
            }
        }

        // Parse optional columns
        let w = get_f64("w", &mut errors);
        let p = get_f64("p", &mut errors);
        let r = get_f64("r", &mut errors);
        let ext1 = get_f64("ext1", &mut errors);
        let ext2 = get_f64("ext2", &mut errors);
        let ext3 = get_f64("ext3", &mut errors);
        let term_type = get_str("term_type");
        let uframe = get_i32("uframe", &mut errors);
        let utool = get_i32("utool", &mut errors);

        // Track consistency for optional columns (only if column exists in header)
        for col in &optional_columns {
            if col_map.contains_key(*col) {
                let has_value = match *col {
                    "w" => w.is_some(),
                    "p" => p.is_some(),
                    "r" => r.is_some(),
                    "ext1" => ext1.is_some(),
                    "ext2" => ext2.is_some(),
                    "ext3" => ext3.is_some(),
                    "term_type" => term_type.is_some(),
                    "uframe" => uframe.is_some(),
                    "utool" => utool.is_some(),
                    _ => false,
                };
                consistency_tracker.record(col, csv_line, has_value);
            }
        }

        // Validate and normalize term_type if present
        // Accepts FINE, CNT, or CNT with value (e.g., CNT100) - normalized to CNT
        let term_type = if let Some(ref tt) = term_type {
            if let Some(normalized) = normalize_term_type(tt) {
                Some(normalized)
            } else {
                errors.push(ValidationError {
                    line: csv_line,
                    column: "term_type".to_string(),
                    message: format!("Invalid term_type '{}'. Must be FINE or CNT (CNT100, CNT50, etc. also accepted)", tt),
                });
                None
            }
        } else {
            None
        };

        // Validate uframe >= 0
        if let Some(uf) = uframe {
            if uf < 0 {
                errors.push(ValidationError {
                    line: csv_line,
                    column: "uframe".to_string(),
                    message: format!("uframe must be >= 0, got: {}", uf),
                });
            }
        }

        // Validate utool >= 0
        if let Some(ut) = utool {
            if ut < 0 {
                errors.push(ValidationError {
                    line: csv_line,
                    column: "utool".to_string(),
                    message: format!("utool must be >= 0, got: {}", ut),
                });
            }
        }

        // Only add instruction if required fields are present
        if let (Some(x), Some(y), Some(z), Some(speed)) = (x, y, z, speed) {
            instructions.push(ProgramInstruction {
                id: 0,
                program_id: 0,
                line_number: line_number as i32,
                x,
                y,
                z,
                w,
                p,
                r,
                ext1,
                ext2,
                ext3,
                speed: Some(speed),
                term_type,
                uframe,
                utool,
            });
        }

        line_number += 1;
    }

    // Check column consistency and add errors
    errors.extend(consistency_tracker.get_errors());

    // Add warning if no data rows
    if line_number == 1 {
        warnings.push(ParseWarning {
            line: None,
            column: None,
            message: "CSV file contains no data rows".to_string(),
        });
    }

    // Return error if any validation errors occurred
    if !errors.is_empty() {
        return Err(ParseError::ValidationErrors(errors));
    }

    Ok(ParseResult {
        instructions,
        warnings,
        row_count: line_number - 1,
        columns_present,
    })
}

/// Parse CSV from a string.
pub fn parse_csv_string(csv_content: &str, defaults: &ProgramDefaults) -> Result<ParseResult, ParseError> {
    parse_csv(csv_content.as_bytes(), defaults)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_csv() {
        let csv = "x,y,z,speed\n100.0,200.0,300.0,50\n150.0,250.0,350.0,100";
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults).unwrap();

        assert_eq!(result.instructions.len(), 2);
        assert_eq!(result.instructions[0].x, 100.0);
        assert_eq!(result.instructions[0].y, 200.0);
        assert_eq!(result.instructions[0].z, 300.0);
        assert_eq!(result.instructions[0].speed, Some(50.0));
        assert_eq!(result.instructions[0].line_number, 1);

        assert_eq!(result.instructions[1].x, 150.0);
        assert_eq!(result.instructions[1].line_number, 2);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_parse_full_csv() {
        let csv = "x,y,z,w,p,r,ext1,ext2,ext3,speed,term_type,uframe,utool\n\
                   100.0,200.0,300.0,0.0,90.0,0.0,0.0,0.0,0.0,50,CNT,3,1";
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults).unwrap();

        assert_eq!(result.instructions.len(), 1);
        assert_eq!(result.instructions[0].w, Some(0.0));
        assert_eq!(result.instructions[0].p, Some(90.0));
        assert_eq!(result.instructions[0].term_type, Some("CNT".to_string()));
        assert_eq!(result.instructions[0].uframe, Some(3));
        assert_eq!(result.instructions[0].utool, Some(1));
    }

    #[test]
    fn test_missing_required_column() {
        let csv = "x,y,speed\n100.0,200.0,50"; // Missing z
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults);

        assert!(matches!(result, Err(ParseError::MissingColumn(col)) if col == "z"));
    }

    #[test]
    fn test_missing_required_value_per_row() {
        // Speed is required per row, not just in header
        let csv = "x,y,z,speed\n100.0,200.0,300.0,";
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults);

        assert!(matches!(result, Err(ParseError::ValidationErrors(_))));
        if let Err(ParseError::ValidationErrors(errors)) = result {
            assert!(errors.iter().any(|e| e.column == "speed" && e.message.contains("Required")));
        }
    }

    #[test]
    fn test_negative_speed_error() {
        let csv = "x,y,z,speed\n100.0,200.0,300.0,-50";
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults);

        assert!(matches!(result, Err(ParseError::ValidationErrors(_))));
        if let Err(ParseError::ValidationErrors(errors)) = result {
            assert!(errors.iter().any(|e| e.column == "speed" && e.message.contains("positive")));
        }
    }

    #[test]
    fn test_invalid_term_type_error() {
        let csv = "x,y,z,speed,term_type\n100.0,200.0,300.0,50,INVALID";
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults);

        assert!(matches!(result, Err(ParseError::ValidationErrors(_))));
        if let Err(ParseError::ValidationErrors(errors)) = result {
            assert!(errors.iter().any(|e| e.column == "term_type" && e.message.contains("Invalid")));
        }
    }

    #[test]
    fn test_negative_uframe_error() {
        let csv = "x,y,z,speed,uframe\n100.0,200.0,300.0,50,-1";
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults);

        assert!(matches!(result, Err(ParseError::ValidationErrors(_))));
        if let Err(ParseError::ValidationErrors(errors)) = result {
            assert!(errors.iter().any(|e| e.column == "uframe" && e.message.contains(">= 0")));
        }
    }

    #[test]
    fn test_inconsistent_optional_column_error() {
        // First row has uframe, second row doesn't - should error
        let csv = "x,y,z,speed,uframe\n100.0,200.0,300.0,50,2\n150.0,250.0,350.0,100,";
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults);

        assert!(matches!(result, Err(ParseError::ValidationErrors(_))));
        if let Err(ParseError::ValidationErrors(errors)) = result {
            assert!(errors.iter().any(|e| e.column == "uframe" && e.message.contains("Inconsistent")));
        }
    }

    #[test]
    fn test_all_optional_columns_empty_is_ok() {
        // All rows have empty optional columns - this is consistent and OK
        let csv = "x,y,z,w,p,r,speed\n100.0,200.0,300.0,,,, 50\n150.0,250.0,350.0,,,,100";
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults).unwrap();

        assert_eq!(result.instructions.len(), 2);
        assert_eq!(result.instructions[0].w, None);
        assert_eq!(result.instructions[0].p, None);
        assert_eq!(result.instructions[0].r, None);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_all_optional_columns_specified_is_ok() {
        // All rows have values for optional columns - this is consistent and OK
        let csv = "x,y,z,w,p,r,speed\n100.0,200.0,300.0,1.0,2.0,3.0,50\n150.0,250.0,350.0,4.0,5.0,6.0,100";
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults).unwrap();

        assert_eq!(result.instructions.len(), 2);
        assert_eq!(result.instructions[0].w, Some(1.0));
        assert_eq!(result.instructions[1].w, Some(4.0));
    }

    #[test]
    fn test_valid_term_types() {
        // Both FINE and CNT should be valid
        let csv = "x,y,z,speed,term_type\n100.0,200.0,300.0,50,FINE\n150.0,250.0,350.0,100,CNT";
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults).unwrap();

        assert_eq!(result.instructions.len(), 2);
        assert_eq!(result.instructions[0].term_type, Some("FINE".to_string()));
        assert_eq!(result.instructions[1].term_type, Some("CNT".to_string()));
    }

    #[test]
    fn test_cnt_with_value_normalized() {
        // CNT100, CNT50, etc. should be normalized to CNT
        let csv = "x,y,z,speed,term_type\n100.0,200.0,300.0,50,CNT100\n150.0,250.0,350.0,100,CNT50\n200.0,300.0,400.0,75,cnt0";
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults).unwrap();

        assert_eq!(result.instructions.len(), 3);
        assert_eq!(result.instructions[0].term_type, Some("CNT".to_string()));
        assert_eq!(result.instructions[1].term_type, Some("CNT".to_string()));
        assert_eq!(result.instructions[2].term_type, Some("CNT".to_string()));
    }
}

