//! Robot model definitions.

use serde::{Deserialize, Serialize};

/// Robot model identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RobotModel {
    /// CRX-10iA: 10kg payload, 1070mm reach
    CRX10iA,
    /// CRX-30iA: 30kg payload, 1756mm reach
    CRX30iA,
}

impl RobotModel {
    /// Get all available robot models.
    pub fn all() -> Vec<RobotModel> {
        vec![RobotModel::CRX10iA, RobotModel::CRX30iA]
    }

    /// Get the display name for this robot model.
    pub fn display_name(&self) -> &'static str {
        match self {
            RobotModel::CRX10iA => "CRX-10iA (10kg, 1070mm)",
            RobotModel::CRX30iA => "CRX-30iA (30kg, 1756mm)",
        }
    }

    /// Get the short name for this robot model.
    pub fn short_name(&self) -> &'static str {
        match self {
            RobotModel::CRX10iA => "CRX-10iA",
            RobotModel::CRX30iA => "CRX-30iA",
        }
    }

    /// Get the value string for HTML select.
    pub fn value(&self) -> &'static str {
        match self {
            RobotModel::CRX10iA => "CRX-10iA",
            RobotModel::CRX30iA => "CRX-30iA",
        }
    }
}

impl std::fmt::Display for RobotModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.short_name())
    }
}

impl std::str::FromStr for RobotModel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "CRX-10IA" | "CRX10IA" | "10IA" => Ok(RobotModel::CRX10iA),
            "CRX-30IA" | "CRX30IA" | "30IA" => Ok(RobotModel::CRX30iA),
            _ => Err(format!("Unknown robot model: {}", s)),
        }
    }
}

impl Default for RobotModel {
    fn default() -> Self {
        RobotModel::CRX10iA
    }
}

