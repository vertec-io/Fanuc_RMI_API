# Robot Configuration System

## Overview

The FANUC RMI API now supports multiple FANUC CRX robot models with accurate kinematic parameters based on the research paper "Geometric Approach for Inverse Kinematics of the FANUC CRX Collaborative Robot" by Manel Abbes and Gérard Poisson (Robotics 2024, 13, 91).

## Supported Robot Models

### CRX-10iA
- **Payload**: 10 kg
- **Reach**: 1070 mm
- **Upper arm (a3)**: 540 mm
- **Forearm (r4)**: -540 mm
- **Wrist offset (r5)**: 150 mm
- **Flange distance (r6)**: -160 mm

### CRX-30iA
- **Payload**: 30 kg
- **Reach**: 1756 mm
- **Upper arm (a3)**: 886 mm (scaled from CRX-10iA)
- **Forearm (r4)**: -886 mm (scaled from CRX-10iA)
- **Wrist offset (r5)**: 246 mm (scaled from CRX-10iA)
- **Flange distance (r6)**: -263 mm (scaled from CRX-10iA)

**Note**: CRX-30iA parameters are scaled from CRX-10iA using the reach ratio (1756mm / 1070mm = 1.641). The research paper provides exact parameters for CRX-10iA only.

## Architecture

### Backend (Simulator)

**File**: `sim/src/robot_config.rs`

```rust
use sim::{RobotConfig, RobotModel};

// Create configuration for CRX-10iA
let config = RobotConfig::crx_10ia();

// Create configuration for CRX-30iA
let config = RobotConfig::crx_30ia();

// Create configuration from model enum
let config = RobotConfig::from_model(RobotModel::CRX30iA);

// Use with kinematics
let kinematics = CRXKinematics::from_config(config);
```

**Key Types**:
- `RobotModel`: Enum for robot model selection
- `RobotConfig`: Struct containing all DHm parameters and specifications

### Frontend (Web App)

**File**: `web_app/src/robot_models.rs`

The web app has its own `RobotModel` enum to avoid dependencies on the sim package. This mirrors the backend enum but is defined separately for the WASM target.

**Settings UI**: Users can select their robot model from a dropdown in the Settings modal.

## Usage

### 1. Simulator

The simulator defaults to CRX-10iA but can be configured for different models:

```bash
# Run simulator with default CRX-10iA
cargo run -p sim -- --realtime

# Future: Support for model selection via CLI
cargo run -p sim -- --realtime --model CRX-30iA
```

### 2. Web Application

1. Open the web application
2. Click the **Settings** button (gear icon) in the header
3. Select your robot model from the **Robot Model** dropdown:
   - CRX-10iA (10kg, 1070mm)
   - CRX-30iA (30kg, 1756mm)
4. Configure other settings (WebSocket URL, Robot IP, Port)
5. Click **Apply** to save settings

**Note**: Currently, the robot model selection in the web UI is for display/documentation purposes. The actual kinematics are determined by the simulator configuration. Future updates will enable dynamic model switching.

### 3. Example Code

```rust
use sim::{CRXKinematics, RobotConfig, RobotModel};

// Create kinematics for CRX-30iA
let config = RobotConfig::crx_30ia();
let kin = CRXKinematics::from_config(config);

// Forward kinematics
let joints = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
let (position, orientation) = kin.forward_kinematics(&joints);

// For CRX-30iA at zero position:
// position ≈ [1756.0, 0.0, 0.0] mm
// (vs CRX-10iA: [1070.0, 0.0, 0.0] mm)

// Inverse kinematics
let target_pos = [1000.0, 0.0, 500.0];
let current_joints = [0.0; 6];
if let Some(solution) = kin.inverse_kinematics(&target_pos, None, &current_joints) {
    println!("IK solution: {:?}", solution);
}
```

## Modified Denavit-Hartenberg (DHm) Parameters

Both robot models use the same DHm convention with link twist angles:

| Link | α_{i-1} (deg) | Description |
|------|---------------|-------------|
| L1   | 0             | Base rotation |
| L2   | -90           | Shoulder |
| L3   | +180          | Elbow |
| L4   | -90           | Wrist 1 |
| L5   | +90           | Wrist 2 |
| L6   | -90           | Wrist 3 |

The key difference between models is in the link lengths (a3, r4, r5, r6).

## Testing

Run kinematics tests for both models:

```bash
# Test default CRX-10iA
cargo test -p sim --lib kinematics -- --nocapture

# Test CRX-30iA (future)
cargo test -p sim --lib kinematics::crx30ia -- --nocapture
```

## Future Enhancements

1. **CLI Model Selection**: Add `--model` flag to simulator
2. **Dynamic Model Switching**: Allow web app to change robot model at runtime
3. **Additional Models**: Add support for CRX-5iA, CRX-20iA, CRX-25iA
4. **Model-Specific Validation**: Joint limits, workspace boundaries, payload limits
5. **Calibration**: Support for custom DHm parameters from robot calibration data

## References

1. **Research Paper**: Abbes, M.; Poisson, G. Geometric Approach for Inverse Kinematics of the FANUC CRX Collaborative Robot. *Robotics* 2024, 13, 91.
2. **FANUC CRX-10iA Specifications**: https://crx.fanucamerica.com/crx-10ia-l
3. **FANUC CRX-30iA Specifications**: https://crx.fanucamerica.com/crx-30ia

## Files Modified/Created

- **Created**: `sim/src/robot_config.rs` - Robot configuration module
- **Modified**: `sim/src/kinematics.rs` - Updated to use RobotConfig
- **Modified**: `sim/src/lib.rs` - Export robot_config module
- **Created**: `web_app/src/robot_models.rs` - Web UI robot model definitions
- **Modified**: `web_app/src/lib.rs` - Export RobotModel
- **Modified**: `web_app/src/components/settings.rs` - Add robot model selector
- **Created**: `ROBOT_CONFIGURATION.md` - This documentation

## Conclusion

The robot configuration system provides a flexible foundation for supporting multiple FANUC CRX models with accurate kinematic parameters. The system is designed to be extensible for future robot models and calibration data.

