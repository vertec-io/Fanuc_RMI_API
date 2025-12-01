//! Coordinate transformation utilities for FANUC RMI.
//!
//! This module provides conversions between FANUC Position types and
//! nalgebra geometric types when the `nalgebra-support` feature is enabled.
//!
//! # Feature Flag
//!
//! Enable nalgebra support in your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! fanuc_rmi = { version = "0.5", features = ["nalgebra-support"] }
//! ```
//!
//! # Examples
//!
//! ```rust,ignore
//! use fanuc_rmi::Position;
//! use nalgebra::Isometry3;
//!
//! // Convert Position to Isometry3
//! let pos = Position {
//!     x: 100.0, y: 200.0, z: 300.0,
//!     w: 0.0, p: 90.0, r: 0.0,
//!     ext1: 0.0, ext2: 0.0, ext3: 0.0,
//! };
//! let iso: Isometry3<f64> = pos.into();
//!
//! // Convert Isometry3 back to Position
//! let pos2: Position = iso.into();
//! ```
//!
//! # Notes
//!
//! - Only the 6-axis values (X, Y, Z, W, P, R) are handled by Isometry3
//! - ext1, ext2, ext3 are set to 0.0 when converting from Isometry3
//! - W, P, R use FANUC's W-P-R Euler angle convention (degrees)

use crate::Position;

#[cfg(feature = "nalgebra-support")]
use nalgebra::{Isometry3, Translation3, UnitQuaternion};

/// Convert Position to nalgebra Isometry3.
///
/// The translation is taken directly from X, Y, Z.
/// The rotation is constructed from W, P, R using Euler angles (in degrees).
///
/// Note: ext1, ext2, ext3 are NOT preserved in the Isometry3 representation.
#[cfg(feature = "nalgebra-support")]
impl From<Position> for Isometry3<f64> {
    fn from(pos: Position) -> Self {
        let translation = Translation3::new(pos.x, pos.y, pos.z);
        
        // Convert degrees to radians for nalgebra
        // FANUC uses W-P-R (Yaw-Pitch-Roll) convention
        let rotation = UnitQuaternion::from_euler_angles(
            pos.w.to_radians(), // Roll (around X)
            pos.p.to_radians(), // Pitch (around Y)
            pos.r.to_radians(), // Yaw (around Z)
        );
        
        Isometry3::from_parts(translation, rotation)
    }
}

/// Convert nalgebra Isometry3 to Position.
///
/// The translation becomes X, Y, Z.
/// The rotation is extracted as Euler angles and converted to degrees.
///
/// Note: ext1, ext2, ext3 are set to 0.0 (not preserved from Isometry3).
#[cfg(feature = "nalgebra-support")]
impl From<Isometry3<f64>> for Position {
    fn from(iso: Isometry3<f64>) -> Self {
        // Extract Euler angles (returns radians)
        let (roll, pitch, yaw) = iso.rotation.euler_angles();
        
        Position {
            x: iso.translation.x,
            y: iso.translation.y,
            z: iso.translation.z,
            w: roll.to_degrees(),
            p: pitch.to_degrees(),
            r: yaw.to_degrees(),
            // External axes are not represented in Isometry3
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        }
    }
}

/// Convert a reference to Position to Isometry3.
#[cfg(feature = "nalgebra-support")]
impl From<&Position> for Isometry3<f64> {
    fn from(pos: &Position) -> Self {
        (*pos).into()
    }
}

#[cfg(all(test, feature = "nalgebra-support"))]
mod tests {
    use super::*;
    
    #[test]
    fn test_position_to_isometry_translation() {
        let pos = Position {
            x: 100.0,
            y: 200.0,
            z: 300.0,
            w: 0.0,
            p: 0.0,
            r: 0.0,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        };
        
        let iso: Isometry3<f64> = pos.into();
        
        assert!((iso.translation.x - 100.0).abs() < 1e-10);
        assert!((iso.translation.y - 200.0).abs() < 1e-10);
        assert!((iso.translation.z - 300.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_roundtrip_conversion() {
        let original = Position {
            x: 123.456,
            y: -789.012,
            z: 345.678,
            w: 45.0,
            p: 30.0,
            r: -60.0,
            ext1: 1.0, // Will be lost in conversion
            ext2: 2.0,
            ext3: 3.0,
        };
        
        let iso: Isometry3<f64> = original.into();
        let converted: Position = iso.into();
        
        // Translation should be preserved
        assert!((converted.x - original.x).abs() < 1e-10);
        assert!((converted.y - original.y).abs() < 1e-10);
        assert!((converted.z - original.z).abs() < 1e-10);
        
        // Rotation should be preserved (within floating point tolerance)
        assert!((converted.w - original.w).abs() < 1e-6);
        assert!((converted.p - original.p).abs() < 1e-6);
        assert!((converted.r - original.r).abs() < 1e-6);
        
        // External axes are NOT preserved
        assert_eq!(converted.ext1, 0.0);
        assert_eq!(converted.ext2, 0.0);
        assert_eq!(converted.ext3, 0.0);
    }
}

