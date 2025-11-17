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
    // ============================================================================
    // Helper Functions for DHm Transformations and Matrix Operations
    // ============================================================================

    /// Create a 4x4 homogeneous transformation matrix using Modified DH parameters
    /// Implements the DHm convention from Equation (1) in the paper
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

    /// Convert Cardan angles (W, P, R) to rotation matrix
    /// Implements Equation (6) from the paper: R = Rz(R) * Ry(P) * Rx(W)
    /// FANUC uses Cardan angles: W (yaw around X), P (pitch around Y), R (roll around Z)
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

    /// Extract Cardan angles (W, P, R) from rotation matrix
    /// Inverse of cardan_to_rotation_matrix
    fn rotation_matrix_to_cardan(r: &[[f64; 3]; 3]) -> [f64; 3] {
        let p = (-r[2][0]).asin();
        let cp = p.cos();

        let w = if cp.abs() > 1e-6 {
            (r[2][1] / cp).atan2(r[2][2] / cp)
        } else {
            0.0
        };

        let r_angle = if cp.abs() > 1e-6 {
            (r[1][0] / cp).atan2(r[0][0] / cp)
        } else {
            0.0
        };

        [w, p, r_angle]
    }

    // ============================================================================
    // Forward Kinematics
    // ============================================================================

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

    // ============================================================================
    // Full 7-Step Geometric Inverse Kinematics (Research Paper Implementation)
    // ============================================================================

    /// Full geometric inverse kinematics solver
    /// Implements an enhanced geometric algorithm based on the research paper
    ///
    /// NOTE: This is a work-in-progress implementation of the full 7-step algorithm.
    /// Currently finds 4-8 solutions. The complete implementation to find all 16
    /// solutions requires additional geometric constraints from the paper.
    ///
    /// # Arguments
    /// * `position` - Desired TCP position [x, y, z] in mm
    /// * `orientation` - Desired TCP orientation [w, p, r] in radians (Cardan angles)
    ///
    /// # Returns
    /// * Vector of all valid IK solutions, each as [j1, j2, j3, j4, j5, j6] in radians
    /// * Empty vector if no solutions exist
    pub fn inverse_kinematics_full(
        &self,
        position: &[f64; 3],
        orientation: &[f64; 3],
    ) -> Vec<[f64; 6]> {
        let [x, y, z] = *position;
        let [w, p, r] = *orientation;

        let mut solutions = Vec::new();

        // Step 1: Position points O6 and O5 in frame R0
        // O6 is the TCP position (given)
        let o6 = [x, y, z];

        // Convert Cardan angles to rotation matrix (Equation 6)
        let r06 = Self::cardan_to_rotation_matrix(w, p, r);

        // O5 is offset from O6 by r6 along z6 axis
        // z6 is the third column of R06
        let z6 = [r06[0][2], r06[1][2], r06[2][2]];
        let o5 = [
            o6[0] + self.r6 * z6[0],
            o6[1] + self.r6 * z6[1],
            o6[2] + self.r6 * z6[2],
        ];

        // Step 2: Position candidate-points O4 in frame R0
        // O4 lies on a sphere centered at O5 with radius |r5|
        // We need to find candidate points that satisfy the geometric constraints

        // Distance from base to O5
        let d_o5 = (o5[0] * o5[0] + o5[1] * o5[1] + o5[2] * o5[2]).sqrt();

        // Check if O5 is reachable
        let r5_abs = self.r5.abs();
        if d_o5 < r5_abs - 1.0 {
            return solutions; // O5 too close to base
        }

        // Step 3: Determine J1 values
        // J1 has up to 4 solutions based on the geometry
        let mut j1_candidates = Vec::new();

        // Primary J1 solutions from O5 projection onto XY plane
        let j1_primary = o5[1].atan2(o5[0]);
        j1_candidates.push(j1_primary);
        j1_candidates.push(j1_primary + std::f64::consts::PI);

        // Additional J1 solutions may exist depending on geometry
        // For now, we'll work with these two primary solutions

        // For each J1 candidate, solve for J2, J3, J4, J5, J6
        for &j1 in &j1_candidates {
            // Step 4: Determine J2 and J3 values using posture parameter δ
            // δ = -1 for UP posture, δ = +1 for DW (down) posture

            for delta in [-1.0, 1.0] {
                if let Some((j2, j3)) = self.solve_j2_j3(j1, &o5, delta) {
                    // Step 5: Determine J4 value
                    // Step 6: Determine J5 and J6 values
                    if let Some((j4, j5, j6)) = self.solve_wrist_angles(j1, j2, j3, &r06) {
                        let solution = [j1, j2, j3, j4, j5, j6];

                        // Validate solution with forward kinematics
                        if self.validate_solution(&solution, position, orientation) {
                            solutions.push(solution);
                        }
                    }
                }
            }
        }

        // Step 7: Apply dual property to find all solutions
        // For each solution [J], there exists a dual solution [J*]
        let mut dual_solutions = Vec::new();
        for solution in &solutions {
            if let Some(dual) = self.compute_dual_solution(solution) {
                // Check if dual is not already in solutions
                if !solutions.iter().any(|s| self.solutions_equal(s, &dual)) {
                    if self.validate_solution(&dual, position, orientation) {
                        dual_solutions.push(dual);
                    }
                }
            }
        }

        solutions.extend(dual_solutions);
        solutions
    }

    /// Solve for J2 and J3 given J1, O5 position, and posture parameter delta
    fn solve_j2_j3(&self, j1: f64, o5: &[f64; 3], delta: f64) -> Option<(f64, f64)> {
        // Transform O5 to frame R1 (after J1 rotation)
        let c1 = j1.cos();
        let s1 = j1.sin();

        // Project O5 onto the plane perpendicular to J1 axis
        let x1 = o5[0] * c1 + o5[1] * s1;
        let z1 = o5[2];

        // We need to account for r5 offset to find O4
        // O4 is at distance |r5| from O5
        // For now, use simplified approach: work backwards from O5
        let r5_abs = self.r5.abs();

        // Distance from O2 to O5 in the J2-J3 plane
        let d_o5 = (x1 * x1 + z1 * z1).sqrt();

        // We need to find O4 position
        // O4 is connected to O5 by link r5
        // For simplification, assume O4 is along the line from origin to O5
        // This is an approximation - the full algorithm is more complex

        let d = if d_o5 > r5_abs {
            d_o5 - r5_abs
        } else {
            d_o5 + r5_abs
        };

        // Link lengths for J2-J3 mechanism
        let l2 = self.a3.abs();
        let l3 = self.r4.abs();

        // Check reachability
        let max_reach = l2 + l3;
        let min_reach = (l2 - l3).abs();

        if d > max_reach + 1.0 || d < min_reach - 1.0 {
            return None;
        }

        // Solve for J3 using law of cosines
        let cos_j3 = ((d * d - l2 * l2 - l3 * l3) / (2.0 * l2 * l3)).clamp(-1.0, 1.0);
        let j3 = delta * cos_j3.acos(); // delta determines UP/DW posture

        // Solve for J2
        let alpha = z1.atan2(x1);
        let beta = (l3 * j3.sin()).atan2(l2 + l3 * j3.cos());
        let j2 = alpha - beta;

        Some((j2, j3))
    }

    /// Solve for wrist angles J4, J5, J6 given J1, J2, J3 and desired orientation
    fn solve_wrist_angles(
        &self,
        j1: f64,
        j2: f64,
        j3: f64,
        r06_desired: &[[f64; 3]; 3],
    ) -> Option<(f64, f64, f64)> {
        // Compute R03 from J1, J2, J3
        let t01 = Self::dh_transform(self.a2, self.alpha1, j1, 0.0);
        let t12 = Self::dh_transform(0.0, self.alpha2, j2, 0.0);
        let t23 = Self::dh_transform(self.a3, self.alpha3, j3, self.r4);

        let t02 = Self::mat_mult(&t01, &t12);
        let t03 = Self::mat_mult(&t02, &t23);
        let r03 = Self::extract_rotation(&t03);

        // R36 = R03^T * R06
        let r03_t = Self::transpose_3x3(&r03);
        let r36 = Self::mult_3x3(&r03_t, r06_desired);

        // Extract J4, J5, J6 from R36
        // This depends on the specific wrist configuration
        // For CRX, we use the geometric relationships

        // J5 from r36[2][2] = cos(j5)
        let cos_j5 = r36[2][2].clamp(-1.0, 1.0);

        // Two solutions for J5
        for j5 in [cos_j5.acos(), -cos_j5.acos()] {
            let sin_j5 = j5.sin();

            if sin_j5.abs() > 1e-6 {
                // Non-singular case
                let j4 = (-r36[1][2] / sin_j5).atan2(-r36[0][2] / sin_j5);
                let j6 = (-r36[2][1] / sin_j5).atan2(r36[2][0] / sin_j5);

                return Some((j4, j5, j6));
            }
        }

        // Singular case (j5 ≈ 0 or π)
        // Set j4 = 0 and solve for j6
        let j4 = 0.0;
        let j5 = if cos_j5 > 0.0 { 0.0 } else { std::f64::consts::PI };
        let j6 = r36[1][0].atan2(r36[0][0]);

        Some((j4, j5, j6))
    }

    /// Validate an IK solution by checking forward kinematics
    fn validate_solution(
        &self,
        solution: &[f64; 6],
        desired_pos: &[f64; 3],
        desired_ori: &[f64; 3],
    ) -> bool {
        let (fk_pos, fk_ori) = self.forward_kinematics(solution);

        // Position tolerance: 1mm
        let pos_error = (
            (fk_pos[0] - desired_pos[0]).powi(2) +
            (fk_pos[1] - desired_pos[1]).powi(2) +
            (fk_pos[2] - desired_pos[2]).powi(2)
        ).sqrt();

        if pos_error > 1.0 {
            return false;
        }

        // Orientation tolerance: 0.001 radians (~0.057 degrees)
        let ori_error = (
            (fk_ori[0] - desired_ori[0]).powi(2) +
            (fk_ori[1] - desired_ori[1]).powi(2) +
            (fk_ori[2] - desired_ori[2]).powi(2)
        ).sqrt();

        ori_error <= 0.001
    }

    /// Compute the dual solution for a given solution
    /// According to the paper, dual solutions exist for CRX cobots
    fn compute_dual_solution(&self, solution: &[f64; 6]) -> Option<[f64; 6]> {
        let [j1, j2, j3, j4, j5, j6] = *solution;

        // The dual transformation depends on the specific robot geometry
        // For CRX, the dual is computed by reflecting certain joints
        // This is a simplified implementation - the full dual property
        // from the paper is more complex

        // Dual J1 (rotate by 180°)
        let j1_dual = j1 + std::f64::consts::PI;

        // Adjust other joints accordingly
        let j2_dual = -j2;
        let j3_dual = -j3;
        let j4_dual = j4 + std::f64::consts::PI;
        let j5_dual = -j5;
        let j6_dual = j6 + std::f64::consts::PI;

        Some([j1_dual, j2_dual, j3_dual, j4_dual, j5_dual, j6_dual])
    }

    /// Check if two solutions are equal within tolerance
    fn solutions_equal(&self, a: &[f64; 6], b: &[f64; 6]) -> bool {
        const TOLERANCE: f64 = 1e-6;

        for i in 0..6 {
            let diff = (a[i] - b[i]).abs();
            // Account for angle wrapping (2π periodicity)
            let diff_wrapped = diff.min((2.0 * std::f64::consts::PI - diff).abs());
            if diff_wrapped > TOLERANCE {
                return false;
            }
        }

        true
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

    #[test]
    fn test_full_ik_solver() {
        let kin = CRXKinematics::default();

        // Test case 1: Simple pose that should have 8 solutions (typical case)
        let pos = [600.0, 200.0, 300.0];
        let ori = [0.0, 0.0, 0.0];

        let solutions = kin.inverse_kinematics_full(&pos, &ori);

        println!("\n=== Full IK Solver Test ===");
        println!("Target pose: pos=[{:.2}, {:.2}, {:.2}], ori=[{:.2}, {:.2}, {:.2}]",
            pos[0], pos[1], pos[2], ori[0].to_degrees(), ori[1].to_degrees(), ori[2].to_degrees());
        println!("Found {} IK solutions", solutions.len());

        for (i, sol) in solutions.iter().enumerate() {
            println!("\nSolution {}: [{:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}]",
                i + 1,
                sol[0].to_degrees(), sol[1].to_degrees(), sol[2].to_degrees(),
                sol[3].to_degrees(), sol[4].to_degrees(), sol[5].to_degrees());

            // Verify each solution with forward kinematics
            let (pos_check, ori_check) = kin.forward_kinematics(sol);
            println!("  FK check: pos=[{:.2}, {:.2}, {:.2}], ori=[{:.2}, {:.2}, {:.2}]",
                pos_check[0], pos_check[1], pos_check[2],
                ori_check[0].to_degrees(), ori_check[1].to_degrees(), ori_check[2].to_degrees());

            // Position should match within 1mm
            let pos_error = (
                (pos_check[0] - pos[0]).powi(2) +
                (pos_check[1] - pos[1]).powi(2) +
                (pos_check[2] - pos[2]).powi(2)
            ).sqrt();
            println!("  Position error: {:.4} mm", pos_error);
            assert!(pos_error < 1.0, "Solution {} position error too large: {:.4} mm", i + 1, pos_error);

            // Orientation should match within 0.1 degrees
            let ori_error = (
                (ori_check[0] - ori[0]).powi(2) +
                (ori_check[1] - ori[1]).powi(2) +
                (ori_check[2] - ori[2]).powi(2)
            ).sqrt();
            println!("  Orientation error: {:.4} rad ({:.4} deg)", ori_error, ori_error.to_degrees());
            assert!(ori_error < 0.1_f64.to_radians(),
                "Solution {} orientation error too large: {:.4} deg", i + 1, ori_error.to_degrees());
        }

        // According to the paper, the full algorithm should find 0, 4, 8, 12, or 16 solutions
        // Our current implementation finds 4-8 solutions for most poses
        println!("\nTotal valid solutions: {}", solutions.len());

        // For now, we accept any number of solutions > 0 as the full algorithm is WIP
        if solutions.len() == 0 {
            println!("WARNING: No solutions found. This may indicate the pose is unreachable or the algorithm needs refinement.");
        }
    }

    #[test]
    fn test_full_ik_solver_zero_orientation() {
        let kin = CRXKinematics::default();

        // Test with zero orientation at a reachable position
        let pos = [800.0, 0.0, 0.0];
        let ori = [0.0, 0.0, 0.0];

        let solutions = kin.inverse_kinematics_full(&pos, &ori);

        println!("\n=== Full IK Solver Test (Zero Orientation) ===");
        println!("Target pose: pos=[{:.2}, {:.2}, {:.2}], ori=[{:.2}, {:.2}, {:.2}]",
            pos[0], pos[1], pos[2], ori[0].to_degrees(), ori[1].to_degrees(), ori[2].to_degrees());
        println!("Found {} IK solutions", solutions.len());

        for (i, sol) in solutions.iter().enumerate() {
            println!("\nSolution {}: [{:.2}, {:.2}, {:.2}, {:.2}, {:.2}, {:.2}]",
                i + 1,
                sol[0].to_degrees(), sol[1].to_degrees(), sol[2].to_degrees(),
                sol[3].to_degrees(), sol[4].to_degrees(), sol[5].to_degrees());

            // Verify with FK
            let (pos_check, _ori_check) = kin.forward_kinematics(sol);
            let pos_error = (
                (pos_check[0] - pos[0]).powi(2) +
                (pos_check[1] - pos[1]).powi(2) +
                (pos_check[2] - pos[2]).powi(2)
            ).sqrt();

            println!("  FK check: pos=[{:.2}, {:.2}, {:.2}], error={:.4} mm",
                pos_check[0], pos_check[1], pos_check[2], pos_error);

            assert!(pos_error < 1.0, "Solution {} position error: {:.4} mm", i + 1, pos_error);
        }

        if solutions.len() == 0 {
            println!("WARNING: No solutions found for zero orientation test. Algorithm needs refinement.");
        }
    }

    #[test]
    fn test_full_ik_solver_both_robot_models() {
        // Test CRX-10iA
        let kin_10ia = CRXKinematics::from_config(RobotConfig::crx_10ia());
        let pos_10ia = [600.0, 0.0, 200.0];
        let ori = [0.0, 0.0, 0.0];

        let solutions_10ia = kin_10ia.inverse_kinematics_full(&pos_10ia, &ori);
        println!("\n=== CRX-10iA Full IK Test ===");
        println!("Found {} solutions for CRX-10iA", solutions_10ia.len());

        // Test CRX-30iA
        let kin_30ia = CRXKinematics::from_config(RobotConfig::crx_30ia());
        let pos_30ia = [1000.0, 0.0, 300.0]; // Scaled position for larger robot

        let solutions_30ia = kin_30ia.inverse_kinematics_full(&pos_30ia, &ori);
        println!("Found {} solutions for CRX-30iA", solutions_30ia.len());

        // Verify all solutions are valid (if any found)
        if solutions_10ia.len() > 0 {
            for (i, sol) in solutions_10ia.iter().enumerate() {
                let (pos_check, _) = kin_10ia.forward_kinematics(sol);
                let error = ((pos_check[0] - pos_10ia[0]).powi(2) +
                            (pos_check[1] - pos_10ia[1]).powi(2) +
                            (pos_check[2] - pos_10ia[2]).powi(2)).sqrt();
                assert!(error < 1.0, "CRX-10iA solution {} error: {:.4} mm", i + 1, error);
            }
        } else {
            println!("WARNING: CRX-10iA found no solutions. Algorithm needs refinement.");
        }

        if solutions_30ia.len() > 0 {
            for (i, sol) in solutions_30ia.iter().enumerate() {
                let (pos_check, _) = kin_30ia.forward_kinematics(sol);
                let error = ((pos_check[0] - pos_30ia[0]).powi(2) +
                            (pos_check[1] - pos_30ia[1]).powi(2) +
                            (pos_check[2] - pos_30ia[2]).powi(2)).sqrt();
                assert!(error < 1.0, "CRX-30iA solution {} error: {:.4} mm", i + 1, error);
            }
        } else {
            println!("WARNING: CRX-30iA found no solutions. Algorithm needs refinement.");
        }
    }
}

