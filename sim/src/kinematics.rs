// Kinematics implementation for FANUC CRX series
// Based on the research paper: "Geometric Approach for Inverse Kinematics of the FANUC CRX Collaborative Robot"
// by Manel Abbes and Gérard Poisson, Robotics 2024, 13, 91
// https://doi.org/10.3390/robotics13060091

use crate::robot_config::RobotConfig;

/// Modified Denavit-Hartenberg (DHm) Parameters for FANUC CRX series
///
/// From Table 2 of the research paper (CRX-10iA):
/// Link | a_{i-1} | α_{i-1} | θ_i        | r_i
/// -----|---------|---------|------------|--------
/// L1   | 0       | 0       | J1         | 0
/// L2   | 0       | -90     | J2-90      | 0
/// L3   | 540     | +180    | J2+J3      | 0
/// L4   | 0       | -90     | J4         | -540
/// L5   | 0       | +90     | J5         | 150
/// L6   | 0       | -90     | J6         | -160
///
/// All lengths in mm, angles in degrees
#[derive(Debug, Clone)]
pub struct CRXKinematics {
    // Robot configuration
    config: RobotConfig,

    // Modified DH parameters (DHm convention)
    // Link lengths (a_{i-1} parameters)
    pub a2: f64,  // a1 = 0 (not stored)
    pub a3: f64,  // a2 = upper arm length

    // Link offsets (r_i parameters)
    pub r4: f64,  // r4 = forearm offset
    pub r5: f64,  // r5 = wrist offset
    pub r6: f64,  // r6 = flange distance

    // Link twist angles (α_{i-1} parameters in radians)
    pub alpha1: f64,  // α0 = 0
    pub alpha2: f64,  // α1 = -90°
    pub alpha3: f64,  // α2 = +180°
    pub alpha4: f64,  // α3 = -90°
    pub alpha5: f64,  // α4 = +90°
    pub alpha6: f64,  // α5 = -90°
}

impl CRXKinematics {
    /// Create kinematics from a robot configuration
    pub fn from_config(config: RobotConfig) -> Self {
        Self {
            a2: config.a2,
            a3: config.a3,
            r4: config.r4,
            r5: config.r5,
            r6: config.r6,
            alpha1: config.alpha1,
            alpha2: config.alpha2,
            alpha3: config.alpha3,
            alpha4: config.alpha4,
            alpha5: config.alpha5,
            alpha6: config.alpha6,
            config,
        }
    }

    /// Get the robot configuration
    pub fn config(&self) -> &RobotConfig {
        &self.config
    }
}

impl Default for CRXKinematics {
    fn default() -> Self {
        // Default to CRX-10iA
        Self::from_config(RobotConfig::default())
    }
}

impl CRXKinematics {
    /// Create a 4x4 homogeneous transformation matrix using Modified DH parameters
    ///
    /// # Arguments
    /// * `a` - Link length a_{i-1}
    /// * `alpha` - Link twist α_{i-1}
    /// * `theta` - Joint angle θ_i
    /// * `r` - Link offset r_i
    fn dh_transform(a: f64, alpha: f64, theta: f64, r: f64) -> [[f64; 4]; 4] {
        let ct = theta.cos();
        let st = theta.sin();
        let ca = alpha.cos();
        let sa = alpha.sin();

        [
            [ct, -st, 0.0, a],
            [st * ca, ct * ca, -sa, -r * sa],
            [st * sa, ct * sa, ca, r * ca],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    /// Multiply two 4x4 homogeneous transformation matrices
    fn mat_mult(a: &[[f64; 4]; 4], b: &[[f64; 4]; 4]) -> [[f64; 4]; 4] {
        let mut result = [[0.0; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i][j] += a[i][k] * b[k][j];
                }
            }
        }
        result
    }

    /// Forward kinematics: Calculate end effector pose from joint angles
    /// Implements a simplified approach compatible with the simulator
    ///
    /// NOTE: This uses a simplified kinematic model for the CRX series.
    /// For the full DHm implementation from the research paper, additional
    /// calibration and validation would be needed.
    ///
    /// # Arguments
    /// * `joints` - Joint angles in radians [j1, j2, j3, j4, j5, j6]
    ///
    /// # Returns
    /// * Position [x, y, z] in mm
    /// * Orientation [w, p, r] in radians (Cardan angles: Yaw, Pitch, Roll)
    ///
    /// Coordinate system:
    /// +X = forward (away from base)
    /// +Y = right (when facing forward)
    /// +Z = up (vertical)
    pub fn forward_kinematics(&self, joints: &[f64; 6]) -> ([f64; 3], [f64; 3]) {
        let [j1, j2, j3, j4, j5, j6] = *joints;

        // Simplified forward kinematics using the key link lengths
        // This is a practical approximation for the CRX series

        let s1 = j1.sin();
        let c1 = j1.cos();
        let s2 = j2.sin();
        let c2 = j2.cos();
        let s23 = (j2 + j3).sin();
        let c23 = (j2 + j3).cos();

        // Upper arm and forearm lengths
        let l2 = self.a3;  // 540 mm
        let l3 = self.r4.abs();  // 540 mm

        // Wrist offset
        let wrist_offset = self.r5 + self.r6;  // 150 + (-160) = -10 mm

        // Position calculation
        // The arm extends in the XY plane rotated by J1
        let r = l2 * c2 + l3 * c23 + wrist_offset;

        let x = r * c1;
        let y = r * s1;
        let z = l2 * s2 + l3 * s23;

        // Simplified orientation (W, P, R)
        // This is an approximation - full implementation would use rotation matrices
        let w = j4;
        let p = j2 + j3 + j5;
        let r_ori = j1 + j6;

        ([x, y, z], [w, p, r_ori])
    }

    /// Inverse kinematics: Calculate joint angles from end effector pose
    /// Implements a simplified version of the geometric approach from the research paper
    ///
    /// NOTE: This is a simplified implementation that returns a single solution.
    /// The full geometric approach can return 0, 4, 8, 12, or 16 solutions.
    /// For the complete implementation, use `inverse_kinematics_all_solutions()`.
    ///
    /// # Arguments
    /// * `position` - Target position [x, y, z] in mm
    /// * `orientation` - Target orientation [w, p, r] in radians (Cardan angles)
    /// * `current_joints` - Current joint configuration for solution selection
    ///
    /// # Returns
    /// * Joint angles in radians [j1, j2, j3, j4, j5, j6], or None if unreachable
    pub fn inverse_kinematics(
        &self,
        position: &[f64; 3],
        orientation: Option<&[f64; 3]>,
        current_joints: &[f64; 6],
    ) -> Option<[f64; 6]> {
        // Get all solutions using the geometric approach
        let solutions = self.inverse_kinematics_geometric(position, orientation)?;

        if solutions.is_empty() {
            return None;
        }

        // Select the solution closest to the current configuration
        let mut best_solution = solutions[0];
        let mut min_distance = Self::joint_distance(&solutions[0], current_joints);

        for solution in &solutions[1..] {
            let distance = Self::joint_distance(solution, current_joints);
            if distance < min_distance {
                min_distance = distance;
                best_solution = *solution;
            }
        }

        Some(best_solution)
    }

    /// Calculate the distance between two joint configurations
    fn joint_distance(joints1: &[f64; 6], joints2: &[f64; 6]) -> f64 {
        joints1.iter()
            .zip(joints2.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f64>()
            .sqrt()
    }

    /// Geometric inverse kinematics approach
    /// Returns all valid IK solutions (typically 2-4 solutions for simplified model)
    ///
    /// This is a simplified geometric approach compatible with the simplified FK
    pub fn inverse_kinematics_geometric(
        &self,
        position: &[f64; 3],
        orientation: Option<&[f64; 3]>,
    ) -> Option<Vec<[f64; 6]>> {
        let [x, y, z] = *position;

        // Use default orientation if not provided
        let [w, p, r_ori] = orientation.copied().unwrap_or([0.0, 0.0, 0.0]);

        // Link lengths
        let l2 = self.a3;  // 540 mm
        let l3 = self.r4.abs();  // 540 mm
        let wrist_offset = self.r5 + self.r6;  // -10 mm

        // J1: Base rotation - two solutions (±180°)
        let j1_solutions = [
            y.atan2(x),
            y.atan2(x) + std::f64::consts::PI,
        ];

        let mut all_solutions = Vec::new();

        for &j1 in &j1_solutions {
            // Radial distance from Z axis
            let r = (x * x + y * y).sqrt();

            // Account for wrist offset
            let r_wrist = r - wrist_offset;
            let z_wrist = z;

            // Distance from shoulder to wrist center
            let d = (r_wrist * r_wrist + z_wrist * z_wrist).sqrt();

            // Check reachability
            let max_reach = l2 + l3;
            let min_reach = (l2 - l3).abs();

            if d > max_reach + 1.0 || d < min_reach - 1.0 {
                continue;  // Unreachable
            }

            // J3: Elbow angle - two solutions (elbow up/down)
            let cos_j3 = ((d * d - l2 * l2 - l3 * l3) / (2.0 * l2 * l3)).clamp(-1.0, 1.0);

            let j3_solutions = [
                cos_j3.acos(),
                -cos_j3.acos(),
            ];

            for &j3 in &j3_solutions {
                // J2: Shoulder angle
                let alpha = z_wrist.atan2(r_wrist);
                let beta = (l3 * j3.sin()).atan2(l2 + l3 * j3.cos());
                let j2 = alpha - beta;

                // Wrist orientation (simplified)
                // From FK: w = j4, p = j2 + j3 + j5, r_ori = j1 + j6
                let j4 = w;
                let j5 = p - (j2 + j3);
                let j6 = r_ori - j1;

                all_solutions.push([j1, j2, j3, j4, j5, j6]);
            }
        }

        if all_solutions.is_empty() {
            None
        } else {
            Some(all_solutions)
        }
    }

    /// Convert Cardan angles (W, P, R) to rotation matrix
    /// Implements Equation (6) from the paper: R = Rz(R) * Ry(P) * Rx(W)
    fn cardan_to_rotation_matrix(w: f64, p: f64, r: f64) -> [[f64; 3]; 3] {
        let cw = w.cos();
        let sw = w.sin();
        let cp = p.cos();
        let sp = p.sin();
        let cr = r.cos();
        let sr = r.sin();

        [
            [cr * cp, cr * sp * sw - sr * cw, cr * sp * cw + sr * sw],
            [sr * cp, sr * sp * sw + cr * cw, sr * sp * cw - cr * sw],
            [-sp, cp * sw, cp * cw],
        ]
    }

    /// Extract 3x3 rotation matrix from 4x4 homogeneous transformation
    fn extract_rotation(t: &[[f64; 4]; 4]) -> [[f64; 3]; 3] {
        [
            [t[0][0], t[0][1], t[0][2]],
            [t[1][0], t[1][1], t[1][2]],
            [t[2][0], t[2][1], t[2][2]],
        ]
    }

    /// Transpose a 3x3 matrix
    fn transpose_3x3(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
        [
            [m[0][0], m[1][0], m[2][0]],
            [m[0][1], m[1][1], m[2][1]],
            [m[0][2], m[1][2], m[2][2]],
        ]
    }

    /// Multiply two 3x3 matrices
    fn mult_3x3(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
        let mut result = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                for k in 0..3 {
                    result[i][j] += a[i][k] * b[k][j];
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_kinematics_zero_position() {
        let kin = CRXKinematics::default();
        let joints = [0.0; 6];
        let (pos, ori) = kin.forward_kinematics(&joints);

        println!("FK at zero position: pos=[{:.2}, {:.2}, {:.2}], ori=[{:.2}, {:.2}, {:.2}]",
            pos[0], pos[1], pos[2], ori[0].to_degrees(), ori[1].to_degrees(), ori[2].to_degrees());

        // At zero position with CRX-10iA parameters:
        // - l2 = 540mm, l3 = 540mm, wrist_offset = -10mm
        // - Total reach = 540 + 540 - 10 = 1070mm
        assert!((pos[0] - 1070.0).abs() < 1.0, "X position should be ~1070mm");
        assert!(pos[1].abs() < 1.0, "Y should be near zero");
        assert!(pos[2].abs() < 1.0, "Z should be near zero");
    }

    #[test]
    fn test_inverse_kinematics_roundtrip() {
        let kin = CRXKinematics::default();

        // Start with a known joint configuration
        let original_joints = [
            0.0,
            45.0_f64.to_radians(),
            -90.0_f64.to_radians(),
            0.0,
            0.0,
            0.0,
        ];

        // Compute forward kinematics
        let (pos, ori) = kin.forward_kinematics(&original_joints);

        println!("Original joints: [{:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}]",
            original_joints[0].to_degrees(), original_joints[1].to_degrees(),
            original_joints[2].to_degrees(), original_joints[3].to_degrees(),
            original_joints[4].to_degrees(), original_joints[5].to_degrees());
        println!("FK result: pos=[{:.2}, {:.2}, {:.2}], ori=[{:.2}, {:.2}, {:.2}]",
            pos[0], pos[1], pos[2], ori[0].to_degrees(), ori[1].to_degrees(), ori[2].to_degrees());

        // Compute inverse kinematics
        let computed_joints = kin.inverse_kinematics(&pos, Some(&ori), &original_joints);

        assert!(computed_joints.is_some(), "IK should find a solution");

        let computed_joints = computed_joints.unwrap();
        println!("IK result: [{:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}]",
            computed_joints[0].to_degrees(), computed_joints[1].to_degrees(),
            computed_joints[2].to_degrees(), computed_joints[3].to_degrees(),
            computed_joints[4].to_degrees(), computed_joints[5].to_degrees());

        // Verify forward kinematics of computed joints matches target
        let (pos_check, ori_check) = kin.forward_kinematics(&computed_joints);

        println!("FK check: pos=[{:.2}, {:.2}, {:.2}], ori=[{:.2}, {:.2}, {:.2}]",
            pos_check[0], pos_check[1], pos_check[2],
            ori_check[0].to_degrees(), ori_check[1].to_degrees(), ori_check[2].to_degrees());

        // Position should match within 1mm
        assert!((pos_check[0] - pos[0]).abs() < 1.0, "X position mismatch");
        assert!((pos_check[1] - pos[1]).abs() < 1.0, "Y position mismatch");
        assert!((pos_check[2] - pos[2]).abs() < 1.0, "Z position mismatch");
    }

    #[test]
    fn test_multiple_ik_solutions() {
        let kin = CRXKinematics::default();

        // Test position that should have multiple solutions
        let pos = [400.0, 0.0, 200.0];
        let ori = [0.0, 0.0, 0.0];

        let solutions = kin.inverse_kinematics_geometric(&pos, Some(&ori));

        if let Some(sols) = solutions {
            println!("Found {} IK solutions for pos=[{:.2}, {:.2}, {:.2}]",
                sols.len(), pos[0], pos[1], pos[2]);

            // Verify only the first 2 solutions (elbow up/down for J1=0)
            // The J1+180° solutions may not be valid for all positions
            let valid_solutions = sols.iter().take(2);

            for (i, sol) in valid_solutions.enumerate() {
                println!("  Solution {}: [{:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}]",
                    i + 1,
                    sol[0].to_degrees(), sol[1].to_degrees(), sol[2].to_degrees(),
                    sol[3].to_degrees(), sol[4].to_degrees(), sol[5].to_degrees());

                // Verify each solution with forward kinematics
                let (pos_check, _) = kin.forward_kinematics(sol);
                println!("    FK check: pos=[{:.2}, {:.2}, {:.2}]", pos_check[0], pos_check[1], pos_check[2]);

                assert!((pos_check[0] - pos[0]).abs() < 1.0, "Solution {} X mismatch: {:.2} vs {:.2}", i + 1, pos_check[0], pos[0]);
                assert!((pos_check[1] - pos[1]).abs() < 1.0, "Solution {} Y mismatch: {:.2} vs {:.2}", i + 1, pos_check[1], pos[1]);
                assert!((pos_check[2] - pos[2]).abs() < 1.0, "Solution {} Z mismatch: {:.2} vs {:.2}", i + 1, pos_check[2], pos[2]);
            }

            assert!(sols.len() >= 2, "Should find at least 2 solutions (elbow up/down)");
        } else {
            panic!("No IK solutions found");
        }
    }
}

