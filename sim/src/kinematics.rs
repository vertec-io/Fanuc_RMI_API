// Kinematics implementation for FANUC CRX-30iA

/// DH Parameters for FANUC CRX-30iA
/// Based on standard FANUC CRX series kinematics
/// 
/// Link lengths (mm):
/// - d1: Base height = 245 mm
/// - a2: Upper arm = 650 mm  
/// - a3: Forearm = 650 mm
/// - d4: Wrist offset = 95 mm
/// - d6: Flange = 95 mm
#[derive(Debug, Clone)]
pub struct CRXKinematics {
    // DH parameters (modified DH convention)
    pub d1: f64,  // Base height
    pub a2: f64,  // Upper arm length
    pub a3: f64,  // Forearm length
    pub d4: f64,  // Wrist 1 offset
    pub d6: f64,  // Flange distance
}

impl Default for CRXKinematics {
    fn default() -> Self {
        Self {
            d1: 245.0,   // Base height (mm)
            a2: 650.0,   // Upper arm (mm)
            a3: 650.0,   // Forearm (mm)
            d4: 95.0,    // Wrist offset (mm)
            d6: 95.0,    // Flange (mm)
        }
    }
}

impl CRXKinematics {
    /// Forward kinematics: Calculate end effector position from joint angles
    ///
    /// # Arguments
    /// * `joints` - Joint angles in radians [j1, j2, j3, j4, j5, j6]
    ///
    /// # Returns
    /// * Position [x, y, z] in mm
    /// * Orientation [w, p, r] in radians (Euler angles)
    ///
    /// Coordinate system:
    /// +X = forward (away from base)
    /// +Y = right (when facing forward)
    /// +Z = up (vertical)
    pub fn forward_kinematics(&self, joints: &[f64; 6]) -> ([f64; 3], [f64; 3]) {
        let [j1, j2, j3, j4, j5, j6] = *joints;

        // Calculate position using standard DH convention
        let s1 = j1.sin();
        let c1 = j1.cos();
        let s2 = j2.sin();
        let c2 = j2.cos();
        let s23 = (j2 + j3).sin();
        let c23 = (j2 + j3).cos();

        // Position in cylindrical coordinates (r, theta, z)
        // r is the radial distance from the Z axis
        let r = self.a2 * c2 + self.a3 * c23 + (self.d4 + self.d6);

        // Convert to Cartesian coordinates
        // J1 rotates around Z axis, so:
        // X = r * cos(j1) (forward/backward)
        // Y = r * sin(j1) (left/right)
        // Z = base height + vertical components
        let x = r * c1;
        let y = r * s1;
        let z = self.d1 + self.a2 * s2 + self.a3 * s23;

        // Simplified orientation (W, P, R)
        let w = j4;
        let p = j2 + j3 + j5;
        let r_ori = j1 + j6;

        ([x, y, z], [w, p, r_ori])
    }
    
    /// Inverse kinematics: Calculate joint angles from end effector position
    ///
    /// # Arguments
    /// * `position` - Target position [x, y, z] in mm
    /// * `orientation` - Target orientation [w, p, r] in radians (optional, uses current if None)
    /// * `current_joints` - Current joint configuration for solution selection
    ///
    /// # Returns
    /// * Joint angles in radians [j1, j2, j3, j4, j5, j6]
    pub fn inverse_kinematics(
        &self,
        position: &[f64; 3],
        orientation: Option<&[f64; 3]>,
        current_joints: &[f64; 6],
    ) -> Option<[f64; 6]> {
        let [x, y, z] = *position;

        // J1: Base rotation (around Z axis)
        // atan2(y, x) gives the angle in the XY plane
        let j1 = y.atan2(x);

        // Radial distance from Z axis (in XY plane)
        let r = (x * x + y * y).sqrt();

        // Subtract wrist offset to get wrist center radial distance
        let wrist_offset = self.d4 + self.d6;
        let r_wrist = r - wrist_offset;

        // Wrist center height above base
        let z_wrist = z - self.d1;

        // Distance from shoulder to wrist center
        let d = (r_wrist * r_wrist + z_wrist * z_wrist).sqrt();

        // Check if target is reachable
        let max_reach = self.a2 + self.a3;
        let min_reach = (self.a2 - self.a3).abs();

        if d > max_reach || d < min_reach {
            eprintln!("IK FAILED: Target unreachable! d={:.2} is outside range [{:.2}, {:.2}]", d, min_reach, max_reach);
            return None;
        }

        // J3: Elbow angle (law of cosines)
        let cos_j3 = (d * d - self.a2 * self.a2 - self.a3 * self.a3) / (2.0 * self.a2 * self.a3);
        let cos_j3 = cos_j3.clamp(-1.0, 1.0);

        // Choose elbow up or down based on current configuration
        let j3 = if current_joints[2] >= 0.0 {
            cos_j3.acos()  // Elbow up
        } else {
            -cos_j3.acos() // Elbow down
        };

        // J2: Shoulder angle
        let alpha = z_wrist.atan2(r_wrist);
        let beta = (self.a3 * j3.sin()).atan2(self.a2 + self.a3 * j3.cos());
        let j2 = alpha - beta;

        // Wrist orientation (simplified - assumes orientation follows arm)
        let j4 = orientation.map(|o| o[0]).unwrap_or(current_joints[3]);
        let j5 = orientation.map(|o| o[1]).unwrap_or(current_joints[4]) - (j2 + j3);
        let j6 = orientation.map(|o| o[2]).unwrap_or(current_joints[5]) - j1;

        Some([j1, j2, j3, j4, j5, j6])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_forward_kinematics_zero_position() {
        let kin = CRXKinematics::default();
        let joints = [0.0; 6];
        let (pos, _ori) = kin.forward_kinematics(&joints);
        
        // At zero position, robot should be extended forward
        assert!(pos[0] > 1000.0); // X should be positive
        assert!(pos[1].abs() < 1.0); // Y should be near zero
        assert!(pos[2] > 200.0); // Z should be above base
    }
}

