/// Robot configuration module for different FANUC CRX models
/// 
/// This module provides configuration data for different CRX robot models
/// based on the Modified Denavit-Hartenberg (DHm) parameters from the
/// research paper "Geometric Approach for Inverse Kinematics of the FANUC CRX
/// Collaborative Robot" by Manel Abbes and Gérard Poisson (Robotics 2024, 13, 91).

use serde::{Deserialize, Serialize};

/// Robot model identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RobotModel {
    /// CRX-10iA: 10kg payload, 1070mm reach
    CRX10iA,
    /// CRX-30iA: 30kg payload, 1756mm reach
    CRX30iA,
}

impl RobotModel {
    /// Get all available robot models
    pub fn all() -> Vec<RobotModel> {
        vec![RobotModel::CRX10iA, RobotModel::CRX30iA]
    }

    /// Get the display name for this robot model
    pub fn display_name(&self) -> &'static str {
        match self {
            RobotModel::CRX10iA => "CRX-10iA (10kg, 1070mm)",
            RobotModel::CRX30iA => "CRX-30iA (30kg, 1756mm)",
        }
    }

    /// Get the short name for this robot model
    pub fn short_name(&self) -> &'static str {
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

/// Robot configuration with DHm parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotConfig {
    /// Robot model
    pub model: RobotModel,
    
    /// Maximum payload in kg
    pub max_payload: f64,
    
    /// Maximum reach in mm
    pub max_reach: f64,
    
    // Modified DH parameters (DHm convention)
    // Link lengths (a_{i-1} parameters)
    pub a2: f64,  // a1 = 0 (not stored)
    pub a3: f64,  // a2 = link 3 length (upper arm)
    
    // Link offsets (r_i parameters)
    pub r4: f64,  // r4 = link 4 offset (forearm)
    pub r5: f64,  // r5 = link 5 offset (wrist)
    pub r6: f64,  // r6 = link 6 offset (flange)
    
    // Link twist angles (α_{i-1} parameters) in radians
    pub alpha1: f64,  // α0 = 0
    pub alpha2: f64,  // α1 = -90°
    pub alpha3: f64,  // α2 = +180°
    pub alpha4: f64,  // α3 = -90°
    pub alpha5: f64,  // α4 = +90°
    pub alpha6: f64,  // α5 = -90°
}

impl RobotConfig {
    /// Create configuration for CRX-10iA
    /// 
    /// Parameters from Table 2 of the research paper:
    /// - Upper arm (a3): 540 mm
    /// - Forearm (r4): -540 mm
    /// - Wrist offset (r5): 150 mm
    /// - Flange distance (r6): -160 mm
    /// - Maximum reach: ~1070 mm
    pub fn crx_10ia() -> Self {
        Self {
            model: RobotModel::CRX10iA,
            max_payload: 10.0,
            max_reach: 1070.0,
            a2: 0.0,
            a3: 540.0,
            r4: -540.0,
            r5: 150.0,
            r6: -160.0,
            alpha1: 0.0,
            alpha2: -90.0_f64.to_radians(),
            alpha3: 180.0_f64.to_radians(),
            alpha4: -90.0_f64.to_radians(),
            alpha5: 90.0_f64.to_radians(),
            alpha6: -90.0_f64.to_radians(),
        }
    }

    /// Create configuration for CRX-30iA
    /// 
    /// Scaled parameters based on reach ratio (1756mm / 1070mm = 1.641):
    /// - Upper arm (a3): 886 mm (540 * 1.641)
    /// - Forearm (r4): -886 mm (-540 * 1.641)
    /// - Wrist offset (r5): 246 mm (150 * 1.641)
    /// - Flange distance (r6): -263 mm (-160 * 1.641)
    /// - Maximum reach: ~1756 mm
    pub fn crx_30ia() -> Self {
        const SCALE_FACTOR: f64 = 1.641121495327103; // 1756 / 1070
        
        Self {
            model: RobotModel::CRX30iA,
            max_payload: 30.0,
            max_reach: 1756.0,
            a2: 0.0,
            a3: 540.0 * SCALE_FACTOR,
            r4: -540.0 * SCALE_FACTOR,
            r5: 150.0 * SCALE_FACTOR,
            r6: -160.0 * SCALE_FACTOR,
            alpha1: 0.0,
            alpha2: -90.0_f64.to_radians(),
            alpha3: 180.0_f64.to_radians(),
            alpha4: -90.0_f64.to_radians(),
            alpha5: 90.0_f64.to_radians(),
            alpha6: -90.0_f64.to_radians(),
        }
    }

    /// Create configuration for a specific robot model
    pub fn from_model(model: RobotModel) -> Self {
        match model {
            RobotModel::CRX10iA => Self::crx_10ia(),
            RobotModel::CRX30iA => Self::crx_30ia(),
        }
    }
}

impl Default for RobotConfig {
    fn default() -> Self {
        Self::crx_10ia()
    }
}

