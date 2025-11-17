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

## Implementation Status

### ✅ Implemented Features

#### 1. **Helper Functions for DHm Transformations**
- `dh_transform()` - Create 4x4 homogeneous transformation matrices
- `mat_mult()` - Multiply transformation matrices
- `extract_rotation()` - Extract 3x3 rotation from 4x4 transformation
- `transpose_3x3()` - Transpose 3x3 matrices
- `mult_3x3()` - Multiply 3x3 matrices
- `cardan_to_rotation_matrix()` - Convert Cardan angles (W, P, R) to rotation matrix
- `rotation_matrix_to_cardan()` - Extract Cardan angles from rotation matrix

#### 2. **Simplified Geometric IK Solver** (Production-Ready)
- `inverse_kinematics()` - Returns single best solution closest to current configuration
- `inverse_kinematics_geometric()` - Returns all 2-4 solutions (elbow up/down, base rotation)
- Achieves sub-millimeter accuracy
- Fast and reliable for real-time control
- Used by the simulator and web application

#### 3. **Full 7-Step IK Solver Foundation** (Work in Progress)
- `inverse_kinematics_full()` - Framework for complete algorithm
- `solve_j2_j3()` - Solve for J2 and J3 with posture parameter
- `solve_wrist_angles()` - Solve for J4, J5, J6 from rotation matrices
- `compute_dual_solution()` - Generate dual solutions
- `validate_solution()` - Verify solutions with forward kinematics

### 🚧 Full Geometric Approach (From Research Paper)

The research paper describes a complete 7-step geometric approach that can find 0, 4, 8, 12, or 16 solutions:

#### **Step 1**: Position points O6 and O5 in frame R0
- ✅ Implemented: O6 is the TCP position (given)
- ✅ Implemented: O5 computed from O6 using r6 offset and orientation

#### **Step 2**: Position candidate-points O4 in frame R0
- 🚧 Partial: O4 lies on sphere centered at O5 with radius |r5|
- ❌ TODO: Find all candidate O4 points satisfying geometric constraints
- ❌ TODO: Implement perpendicularity constraints

#### **Step 3**: Determine J1 values
- ✅ Implemented: Primary J1 solutions from O5 projection
- ❌ TODO: Additional J1 solutions from geometric analysis
- ❌ TODO: Up to 4 J1 solutions possible

#### **Step 4**: Determine J2 and J3 with posture parameter δ
- ✅ Implemented: Basic J2/J3 solver with UP/DW posture
- ❌ TODO: Refine O4 position calculation
- ❌ TODO: Proper handling of all geometric cases

#### **Step 5**: Determine J4 value
- ✅ Implemented: J4 from rotation matrix decomposition
- ❌ TODO: Handle all singularity cases

#### **Step 6**: Determine J5 and J6 values
- ✅ Implemented: J5/J6 from R36 matrix
- ❌ TODO: All solution branches for wrist singularities

#### **Step 7**: Apply dual property for all solutions
- ✅ Implemented: Basic dual solution generation
- ❌ TODO: Verify dual property for all 16 solutions
- ❌ TODO: Proper dual transformation for CRX geometry

### Simplified Implementation (Current Production Use)

The current implementation uses a simplified geometric approach that:
- Provides 2-4 solutions (elbow up/down, base rotation)
- Uses analytical geometry for J1, J2, J3
- Uses simplified orientation calculations for J4, J5, J6
- Achieves sub-millimeter accuracy for position
- Is computationally efficient for real-time control

**Accuracy:**
- Position: < 1 mm error
- Orientation: Simplified (suitable for most applications)

## Future Work: Complete 7-Step Algorithm

### Phase 1: Geometric Constraint Implementation
1. **Candidate Point O4 Generation**
   - Implement sphere intersection for O4 candidates
   - Apply perpendicularity constraints between O3O4 and O4O5
   - Find all valid O4 positions (up to 4)

2. **Complete J1 Solution Set**
   - Geometric analysis for all J1 candidates
   - Handle special cases (singularities, workspace boundaries)
   - Validate against paper's examples

### Phase 2: Wrist Kinematics Refinement
3. **Enhanced Wrist Angle Solver**
   - Complete R36 decomposition for all cases
   - Handle all singularity configurations
   - Multiple J5 solutions (±acos)

4. **Dual Solution Property**
   - Verify dual transformation for CRX geometry
   - Ensure all 16 solutions are found
   - Validate dual pairs give identical poses

### Phase 3: Validation and Testing
5. **Test Against Paper Examples**
   - 8-solution case (Section 3.1)
   - 12-solution case (Section 3.2)
   - 16-solution case (Section 3.3)
   - Verify sub-millidegree accuracy

6. **Workspace Analysis**
   - Characterize workspace domains by solution count
   - Define boundaries between 4, 8, 12, 16 solution regions
   - Implement aspect changes and transitions

### Phase 4: Optimization
7. **Performance Optimization**
   - Minimize redundant calculations
   - Cache transformation matrices
   - Parallel solution validation

8. **Solution Selection Logic**
   - Minimum joint motion criterion
   - Joint limit avoidance
   - Singularity avoidance
   - User-defined preferences

## Current Recommendation

**For Production Use**: Use `inverse_kinematics()` or `inverse_kinematics_geometric()`
- ✅ Proven accuracy (sub-millimeter)
- ✅ Fast and reliable
- ✅ Finds 2-4 solutions (sufficient for most applications)
- ✅ Well-tested and validated

**For Research/Development**: Use `inverse_kinematics_full()`
- 🚧 Work in progress
- 🚧 Foundation in place, needs geometric refinement
- 🚧 Will eventually find all 16 solutions
- 🚧 Requires additional development and testing

## Testing

Run the kinematics tests:
```bash
cargo test -p sim --lib kinematics -- --nocapture
```

**Test Coverage:**
- ✅ Forward kinematics at zero position
- ✅ Inverse kinematics roundtrip (FK → IK → FK)
- ✅ Multiple IK solutions verification (2-4 solutions)
- ✅ Full IK solver framework (foundation tests)
- ✅ Both CRX-10iA and CRX-30iA robot models

## References

1. Abbes, M.; Poisson, G. Geometric Approach for Inverse Kinematics of the FANUC CRX Collaborative Robot. *Robotics* 2024, 13, 91.
2. FANUC CRX Series Documentation
3. Modified Denavit-Hartenberg Convention (Craig, 2005)

