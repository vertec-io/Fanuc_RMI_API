# FANUC RMI API - Project Overview

**Comprehensive Guide to the FANUC Remote Motion Interface API and Web Application**

---

## What is This Project?

The FANUC RMI API is a Rust-based library and web application for controlling FANUC industrial robots via the Remote Motion Interface (RMI) protocol. It provides:

1. **Rust Library** (`fanuc_rmi`): Type-safe, async API for FANUC RMI communication
2. **Web Server** (`web_server`): WebSocket bridge between web clients and robot
3. **Web Application** (`web_app`): Browser-based robot control interface
4. **Simulator** (`sim`): FANUC robot simulator for testing without hardware

---

## Current Status (v0.5.0)

### ✅ What Works

- **Connection Management**: Connect, disconnect, initialize, abort
- **Status Monitoring**: Read robot status, position, joint angles
- **Motion Control**: Linear, joint, and circular motion commands
- **Async API**: Timeout-based async methods for commands
- **Request ID System**: Track async operations
- **Smart Initialization**: `startup_sequence()` method handles robot state
- **Logging Levels**: Error, Warn, Info, Debug for clean output
- **Web Interface**: Basic position display and jog controls

### 🔧 Known Issues (Being Addressed)

- **Position Display Accuracy**: UI shows position in active UFrame, not World Frame
  - Root cause: FRC_ReadCartesianPosition returns position in currently active UFrame
  - Solution: Implement multi-frame display with coordinate transformations

### 📋 Planned Improvements

See [IMPLEMENTATION_ROADMAP.md](./IMPLEMENTATION_ROADMAP.md) for detailed plan to:
- Add frame/tool awareness and management
- Implement multi-frame coordinate display
- Create professional UI with tabs and navigation
- Add advanced features (I/O, logging, multi-robot)

---

## Documentation Structure

### For Developers New to Robotics

Start here to learn fundamental concepts:

1. **[FANUC_ROBOTICS_FUNDAMENTALS.md](./FANUC_ROBOTICS_FUNDAMENTALS.md)**
   - What are coordinate frames, UFrames, UTools?
   - Why do we need multiple frames?
   - How does configuration work?
   - Common pitfalls and best practices

2. **[COORDINATE_FRAMES_GUIDE.md](./COORDINATE_FRAMES_GUIDE.md)**
   - Deep dive into coordinate transformations
   - Rotation matrices and Euler angles
   - Implementing multi-frame display
   - Testing and validation

### For API Users

Reference documentation for using the API:

3. **[RMI_COMMANDS_REFERENCE.md](./RMI_COMMANDS_REFERENCE.md)**
   - Complete list of all RMI commands
   - Request/response formats
   - Rust API examples
   - When to use each command

4. **[FANUC_INITIALIZATION_SEQUENCE.md](./FANUC_INITIALIZATION_SEQUENCE.md)**
   - Proper robot startup sequence
   - Common initialization errors
   - Smart initialization logic

### For Contributors

Planning and implementation documents:

5. **[IMPLEMENTATION_ROADMAP.md](./IMPLEMENTATION_ROADMAP.md)**
   - Phased development plan
   - UI/UX mockups
   - Technical architecture
   - Timeline and milestones

6. **[ISSUE_ANALYSIS_v0.5.0.md](./ISSUE_ANALYSIS_v0.5.0.md)**
   - Post-release issue analysis
   - Root cause investigations
   - Solutions implemented

---

## Quick Start

### Prerequisites

- Rust 1.70 or later
- FANUC robot with RMI support, or use the simulator
- Network connection to robot

### Installation

```bash
# Clone repository
git clone https://github.com/vertec-io/Fanuc_RMI_API.git
cd Fanuc_RMI_API

# Build all packages
cargo build --all

# Run simulator (in one terminal)
cargo run -p sim -- --realtime

# Run web server (in another terminal)
cargo run -p web_server

# Open web browser
# Navigate to http://localhost:9000
```

### Environment Variables

```bash
# Robot connection
export FANUC_ROBOT_ADDR="127.0.0.1"
export FANUC_ROBOT_PORT="16001"

# WebSocket server
export WEBSOCKET_PORT="9000"
```

### Basic Usage Example

```rust
use fanuc_rmi::FanucDriver;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create driver
    let driver = FanucDriver::new("127.0.0.1:16001").await?;
    
    // Connect and initialize
    driver.connect().await?;
    driver.startup_sequence().await?;
    
    // Read position
    let pos = driver.read_cartesian_position().await?;
    println!("Position: X={}, Y={}, Z={}", pos.pos.x, pos.pos.y, pos.pos.z);
    
    // Move robot
    driver.linear_motion(
        500.0, 0.0, 400.0,  // X, Y, Z
        0.0, 0.0, 180.0,    // W, P, R
        100,                // Speed (mm/sec)
        "FINE",             // Term type
        0,                  // Term value
    ).await?;
    
    // Cleanup
    driver.abort().await?;
    driver.disconnect().await?;
    
    Ok(())
}
```

---

## Architecture

### System Overview

```
┌─────────────┐
│  Web Browser│
│  (web_app)  │
└──────┬──────┘
       │ WebSocket
       │
┌──────▼──────┐
│ Web Server  │
│ (Rust)      │
└──────┬──────┘
       │ TCP/IP
       │ JSON
┌──────▼──────┐
│ FANUC Robot │
│ Controller  │
└─────────────┘
```

### Component Responsibilities

**fanuc_rmi (Library)**:
- Protocol type definitions
- Serialization/deserialization
- Driver with async methods
- Request ID management
- Sequence ID tracking

**web_server**:
- WebSocket server
- Robot connection management
- Message routing
- State synchronization

**web_app**:
- User interface
- Real-time position display
- Jog controls
- Settings management

**sim (Simulator)**:
- Emulates FANUC robot
- For testing without hardware
- Supports all RMI commands

---

## Key Concepts

### Request IDs vs Sequence IDs

- **Request ID**: Tracks async operations, assigned by client
- **Sequence ID**: Identifies motion instructions, must be consecutive

### Coordinate Frames

- **World Frame**: Robot base coordinate system
- **User Frames (UFrames)**: Custom work coordinate systems (0-9)
- **User Tools (UTools)**: Tool geometry and TCP (0-10)

### Motion Types

- **Linear Motion**: Straight line path
- **Joint Motion**: Fastest path (not straight)
- **Circular Motion**: Arc through 3 points

### Termination Types

- **FINE**: Stop at point
- **CNT**: Blend through point (corner rounding)

---

## Development Workflow

### Making Changes

1. Create feature branch
2. Make changes
3. Run tests: `cargo test --all`
4. Build: `cargo build --all`
5. Test with simulator
6. Test with real robot (if available)
7. Update documentation
8. Create pull request

### Testing Strategy

- **Unit tests**: Test individual functions
- **Integration tests**: Test with simulator
- **Validation tests**: Test with real robot
- **Manual testing**: Use web interface

### Code Style

- Follow Rust conventions
- Use `cargo fmt` for formatting
- Use `cargo clippy` for linting
- Document public APIs

---

## Troubleshooting

### Common Issues

**Issue**: Position on UI doesn't match teach pendant

**Solution**: Check which UFrame is active. UI shows position in active UFrame, TP may show World Frame. See [FANUC_ROBOTICS_FUNDAMENTALS.md](./FANUC_ROBOTICS_FUNDAMENTALS.md) for details.

---

**Issue**: Initialization fails with error 7015

**Solution**: RMI_MOVE program is selected on TP. Press SELECT, choose different program, press ENTER. See [FANUC_INITIALIZATION_SEQUENCE.md](./FANUC_INITIALIZATION_SEQUENCE.md).

---

**Issue**: Cannot change UFrame/UTool

**Solution**: Robot must be stopped (RMIMotionStatus == 0). Wait for motion to complete before changing frames.

---

**Issue**: Sequence ID errors

**Solution**: Sequence IDs must be consecutive. Use driver's automatic sequence ID management.

---

## Contributing

We welcome contributions! Please:

1. Read the documentation
2. Check existing issues
3. Create issue for discussion
4. Submit pull request

### Areas for Contribution

- Additional RMI commands
- UI improvements
- Documentation
- Testing
- Bug fixes

---

## License

[Add license information]

---

## Contact

[Add contact information]

---

## Version History

### v0.5.0 (2025-11-29)
- Renamed correlation_id to request_id
- Added async methods for commands
- Implemented smart startup_sequence()
- Added logging levels
- Fixed simulator responses
- Updated all documentation

### v0.4.0 (2025-11-25)
- Changed position precision from f32 to f64
- Added request ID system
- Fixed sequence ID errors
- Improved error handling

### v0.3.0 and earlier
- Initial development
- Basic RMI protocol implementation
- Web interface prototype

---

**Document Version**: 1.0  
**Last Updated**: 2025-11-29  
**Author**: FANUC RMI API Development Team

