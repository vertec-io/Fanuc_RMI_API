//! CSV program parser for robot motion programs.
//!
//! Supports flexible CSV format:
//! - Minimal: x, y, z, speed (required columns)
//! - Full: x, y, z, w, p, r, ext1, ext2, ext3, speed, term_type, uframe, utool
//!
//! Missing columns use program defaults or robot defaults.

use crate::database::ProgramInstruction;
use csv::ReaderBuilder;
use std::collections::HashMap;
use std::io::Read;

/// Error type for CSV parsing.
#[derive(Debug)]
pub enum ParseError {
    CsvError(csv::Error),
    MissingColumn(String),
    InvalidValue { line: usize, column: String, value: String },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::CsvError(e) => write!(f, "CSV error: {}", e),
            ParseError::MissingColumn(col) => write!(f, "Missing required column: {}", col),
            ParseError::InvalidValue { line, column, value } => {
                write!(f, "Invalid value at line {}, column '{}': '{}'", line, column, value)
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

/// Default values for missing CSV columns.
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
        }
    }
}

/// Parse a CSV program from a reader.
///
/// Returns a vector of instructions with line numbers starting at 1.
pub fn parse_csv<R: Read>(reader: R, _defaults: &ProgramDefaults) -> Result<Vec<ProgramInstruction>, ParseError> {
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

    // Check required columns
    for required in ["x", "y", "z", "speed"] {
        if !col_map.contains_key(required) {
            return Err(ParseError::MissingColumn(required.to_string()));
        }
    }

    let mut instructions = Vec::new();
    let mut line_number = 1;

    for result in csv_reader.records() {
        let record = result?;
        
        // Helper to get optional f64 value
        let get_f64 = |col: &str| -> Result<Option<f64>, ParseError> {
            if let Some(&idx) = col_map.get(col) {
                if let Some(val) = record.get(idx) {
                    if val.is_empty() {
                        Ok(None)
                    } else {
                        val.parse::<f64>()
                            .map(Some)
                            .map_err(|_| ParseError::InvalidValue {
                                line: line_number + 1, // +1 for header
                                column: col.to_string(),
                                value: val.to_string(),
                            })
                    }
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        };

        // Helper to get optional i32 value
        let get_i32 = |col: &str| -> Result<Option<i32>, ParseError> {
            if let Some(&idx) = col_map.get(col) {
                if let Some(val) = record.get(idx) {
                    if val.is_empty() {
                        Ok(None)
                    } else {
                        val.parse::<i32>()
                            .map(Some)
                            .map_err(|_| ParseError::InvalidValue {
                                line: line_number + 1,
                                column: col.to_string(),
                                value: val.to_string(),
                            })
                    }
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        };

        // Helper to get optional string value
        let get_str = |col: &str| -> Option<String> {
            col_map.get(col)
                .and_then(|&idx| record.get(idx))
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
        };

        // Required columns (x, y, z, speed)
        let x = get_f64("x")?.ok_or_else(|| ParseError::InvalidValue {
            line: line_number + 1, column: "x".to_string(), value: "".to_string()
        })?;
        let y = get_f64("y")?.ok_or_else(|| ParseError::InvalidValue {
            line: line_number + 1, column: "y".to_string(), value: "".to_string()
        })?;
        let z = get_f64("z")?.ok_or_else(|| ParseError::InvalidValue {
            line: line_number + 1, column: "z".to_string(), value: "".to_string()
        })?;
        let speed = get_f64("speed")?;

        // Optional columns with defaults
        let w = get_f64("w")?;
        let p = get_f64("p")?;
        let r = get_f64("r")?;
        let ext1 = get_f64("ext1")?;
        let ext2 = get_f64("ext2")?;
        let ext3 = get_f64("ext3")?;
        let term_type = get_str("term_type");
        let uframe = get_i32("uframe")?;
        let utool = get_i32("utool")?;

        instructions.push(ProgramInstruction {
            id: 0, // Will be set by database
            program_id: 0, // Will be set when saving
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
            speed,
            term_type,
            uframe,
            utool,
        });

        line_number += 1;
    }

    Ok(instructions)
}

/// Parse CSV from a string.
pub fn parse_csv_string(csv_content: &str, defaults: &ProgramDefaults) -> Result<Vec<ProgramInstruction>, ParseError> {
    parse_csv(csv_content.as_bytes(), defaults)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_csv() {
        let csv = "x,y,z,speed\n100.0,200.0,300.0,50\n150.0,250.0,350.0,100";
        let defaults = ProgramDefaults::default();
        let instructions = parse_csv_string(csv, &defaults).unwrap();

        assert_eq!(instructions.len(), 2);
        assert_eq!(instructions[0].x, 100.0);
        assert_eq!(instructions[0].y, 200.0);
        assert_eq!(instructions[0].z, 300.0);
        assert_eq!(instructions[0].speed, Some(50.0));
        assert_eq!(instructions[0].line_number, 1);

        assert_eq!(instructions[1].x, 150.0);
        assert_eq!(instructions[1].line_number, 2);
    }

    #[test]
    fn test_parse_full_csv() {
        let csv = "x,y,z,w,p,r,ext1,ext2,ext3,speed,term_type,uframe,utool\n\
                   100.0,200.0,300.0,0.0,90.0,0.0,0.0,0.0,0.0,50,CNT,3,1";
        let defaults = ProgramDefaults::default();
        let instructions = parse_csv_string(csv, &defaults).unwrap();

        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0].w, Some(0.0));
        assert_eq!(instructions[0].p, Some(90.0));
        assert_eq!(instructions[0].term_type, Some("CNT".to_string()));
        assert_eq!(instructions[0].uframe, Some(3));
        assert_eq!(instructions[0].utool, Some(1));
    }

    #[test]
    fn test_missing_required_column() {
        let csv = "x,y,speed\n100.0,200.0,50"; // Missing z
        let defaults = ProgramDefaults::default();
        let result = parse_csv_string(csv, &defaults);

        assert!(matches!(result, Err(ParseError::MissingColumn(col)) if col == "z"));
    }

    #[test]
    fn test_optional_columns_empty() {
        let csv = "x,y,z,w,p,r,speed\n100.0,200.0,300.0,,,, 50";
        let defaults = ProgramDefaults::default();
        let instructions = parse_csv_string(csv, &defaults).unwrap();

        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0].w, None);
        assert_eq!(instructions[0].p, None);
        assert_eq!(instructions[0].r, None);
    }
}

