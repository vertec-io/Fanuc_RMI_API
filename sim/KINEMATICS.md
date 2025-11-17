# FANUC CRX Kinematics Implementation

## Overview

This kinematics implementation is based on the research paper:

**"Geometric Approach for Inverse Kinematics of the FANUC CRX Collaborative Robot"**  
by Manel Abbes and Gérard Poisson  
Published in *Robotics* 2024, 13, 91  
DOI: https://doi.org/10.3390/robotics13060091

## Robot Model: FANUC CRX-10iA

The FANUC CRX-10iA is a 6 Degrees of Freedom (DoF) collaborative robot with a 6R serial architecture and a non-spherical wrist. This architecture makes traditional inverse kinematics methods (like Pieper's or Paul's) inapplicable, requiring a geometric numerical approach.

### Modified Denavit-Hartenberg (DHm) Parameters

From Table 2 of the research paper:

| Link | a_{i-1} (mm) | α_{i-1} (deg) | θ_i        | r_i (mm) |
|------|--------------|---------------|------------|----------|
| L1   | 0            | 0             | J1         | 0        |
| L2   | 0            | -90           | J2-90      | 0        |
| L3   | 540          | +180          | J2+J3      | 0        |
| L4   | 0            | -90           | J4         | -540     |
| L5   | 0            | +90           | J5         | 150      |
| L6   | 0            | -90           | J6         | -160     |

**Key Dimensions:**
- Upper arm length (a3): 540 mm
- Forearm length (|r4|): 540 mm
- Wrist offset (r5 + r6): -10 mm
- Maximum reach: ~1070 mm

## Implementation

### Forward Kinematics

The forward kinematics implementation uses a simplified geometric approach that is compatible with the simulator and provides accurate position calculations:

```rust
pub fn forward_kinematics(&self, joints: &[f64; 6]) -> ([f64; 3], [f64; 3])
```

**Inputs:**
- Joint angles [J1, J2, J3, J4, J5, J6] in radians

**Outputs:**
- Position [X, Y, Z] in mm
- Orientation [W, P, R] in radians (Cardan angles: Yaw, Pitch, Roll)

**Coordinate System:**
- +X: forward (away from base)
- +Y: right (when facing forward)
- +Z: up (vertical)

### Inverse Kinematics

The inverse kinematics implementation provides two methods:

#### 1. Single Solution Method
```rust
pub fn inverse_kinematics(
    &self,
    position: &[f64; 3],
    orientation: Option<&[f64; 3]>,
    current_joints: &[f64; 6],
) -> Option<[f64; 6]>
```

Returns the IK solution closest to the current joint configuration.

#### 2. Multiple Solutions Method
```rust
pub fn inverse_kinematics_geometric(
    &self,
    position: &[f64; 3],
    orientation: Option<&[f64; 3]>,
) -> Option<Vec<[f64; 6]>>
```

Returns all valid IK solutions (typically 2-4 solutions for the simplified model).

**Solution Types:**
- **Elbow Up**: J3 > 0
- **Elbow Down**: J3 < 0
- **Base Rotation**: J1 and J1 ± 180°

## Research Paper vs. Implementation

### Full Geometric Approach (Paper)

The research paper describes a 7-step geometric approach that can find 0, 4, 8, 12, or 16 solutions:

1. **Step 1**: Position points O6 and O5 in frame R0
2. **Step 2**: Position candidate-points O4 in frame R0
3. **Step 3**: Position candidate-points O3 in frame R0
4. **Step 4**: Perpendicularity of O3O4 and O4O5 vectors
5. **Step 5**: Determination of J1, J2, J3
6. **Step 6**: Determination of J4, J5, J6
7. **Step 7**: Finding all IK solutions by dual property

### Simplified Implementation (Current)

The current implementation uses a simplified geometric approach that:
- Provides 2-4 solutions (elbow up/down, base rotation)
- Uses analytical geometry for J1, J2, J3
- Uses simplified orientation calculations for J4, J5, J6
- Achieves sub-millimeter accuracy for position
- Is computationally efficient for real-time control

**Accuracy:**
- Position: < 1 mm error
- Orientation: Simplified (suitable for most applications)

## Future Enhancements

To implement the full geometric approach from the paper:

1. **Complete DHm Transformation**: Implement full 4x4 homogeneous transformation matrices
2. **Rotation Matrix Decomposition**: Proper Cardan angle extraction from rotation matrices
3. **All 16 Solutions**: Implement Steps 2-7 to find all possible configurations
4. **Dual Property**: Apply Equation (23) to generate dual solutions
5. **Workspace Analysis**: Characterize domains with specific numbers of solutions

## Testing

Run the kinematics tests:
```bash
cargo test -p sim --lib kinematics -- --nocapture
```

**Test Coverage:**
- Forward kinematics at zero position
- Inverse kinematics roundtrip (FK → IK → FK)
- Multiple IK solutions verification

## References

1. Abbes, M.; Poisson, G. Geometric Approach for Inverse Kinematics of the FANUC CRX Collaborative Robot. *Robotics* 2024, 13, 91.
2. FANUC CRX Series Documentation
3. Modified Denavit-Hartenberg Convention (Craig, 2005)

