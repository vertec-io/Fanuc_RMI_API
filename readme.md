# Fanuc RMI API

A comprehensive Rust library for communicating with and controlling FANUC robots via the Remote Motion Interface (RMI) protocol. Includes driver implementation, web-based control interface, and simulation capabilities.

**Current Version**: 0.5.0
**Status**: Active Development
**License**: See LICENSE file

---

## ⚠️ Important Updates

### Latest Changes (v0.5.0) - 2025-11-25

**API Improvements (No Breaking Changes):**
- ✨ **Async Command Methods**: `abort()`, `initialize()`, `get_status()`, `disconnect()` now wait for responses
- ✨ **Proper Error Handling**: Access FANUC error codes directly from responses
- ✨ **Industry Standard Terminology**: `correlation_id` → `request_id`
- ✨ **Clearer Method Names**: `send_command()` → `send_packet()`
- 🐛 **No More Arbitrary Sleeps**: Async methods eliminate manual delay guessing

**Migration:** See [v0.5.0 Migration Guide](docs/NAMING_MIGRATION_GUIDE_v0.5.0.md) - All old methods still work (deprecated)

### Previous Changes (v0.4.0)

**Breaking Changes:**
- 💥 **Position Precision Fix**: Changed position fields from `f32` to `f64` for sub-millimeter accuracy
- 💥 **Request ID System**: `send_command()` returns request ID (u64) for tracking async operations

See [v0.4.0 Release Notes](docs/releases/RELEASE_NOTES_v0.4.0.md) for details

---

## Features

### Core Library (`fanuc_rmi`)
- ✅ **Full RMI Protocol Support**: Commands, Instructions, and Communications per FANUC B-84184EN_02 spec
- ✅ **Type-Safe API**: Strongly-typed Rust structs for all RMI packets
- ✅ **Async Driver**: Tokio-based async driver with priority queue and sequence ID management
- ✅ **Correlation ID System**: Track requests and responses across async boundaries
- ✅ **DTO Generation**: Automatic DTO types for network serialization (bincode compatible)
- ✅ **ExtractInner Trait**: Generic type-safe extraction from response enums
- ✅ **Error Handling**: Comprehensive error types with FANUC error code mapping

### Web Application
- ✅ **Real-time Monitoring**: Live position, status, and motion tracking via WebSocket
- ✅ **Jog Controls**: Interactive 6-axis cartesian jogging interface
- ✅ **Modern UI**: Clean dark mode design with Leptos + TailwindCSS
- ✅ **WebSocket Bridge**: Bidirectional communication with FANUC driver

### Simulation
- ✅ **RMI Simulator**: Software simulator for testing without hardware
- ✅ **Kinematics**: Forward/inverse kinematics for CRX-10iA and CRX-30iA
- ✅ **State Tracking**: Maintains robot state (position, configuration, status)

---

## Supported Robot Models

- **FANUC CRX-10iA** (10kg payload, 1070mm reach) - Full kinematic parameters from research paper
- **FANUC CRX-30iA** (30kg payload, 1756mm reach) - Scaled parameters
- Other FANUC robots with RMI support (basic compatibility)

---

## Quick Start

### Prerequisites

- **Rust** 1.70+ with Cargo
- **FANUC Robot** with RMI support (or use the included simulator)
- **For Web App**: `wasm32-unknown-unknown` target and `trunk` (optional)

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
fanuc_rmi = { version = "0.5", features = ["driver", "DTO"] }
tokio = { version = "1", features = ["full"] }
```

### Basic Usage

```rust
use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig};
use fanuc_rmi::packets::{SendPacket, PacketPriority, Instruction};
use fanuc_rmi::instructions::FrcLinearRelative;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure and connect
    let config = FanucDriverConfig {
        addr: "192.168.1.100".to_string(),
        port: 18735,
        max_messages: 30,
    };
    let driver = FanucDriver::connect(config).await?;

    // Initialize with response handling (v0.5.0+)
    let init_response = driver.initialize().await?;
    if init_response.error_id == 0 {
        println!("✓ Robot initialized");
    }

    // Send motion and wait for completion
    let instruction = FrcLinearRelative::new(/* ... */);
    let sequence_id = driver.send_and_wait_for_completion(
        SendPacket::Instruction(Instruction::FrcLinearRelative(instruction)),
        PacketPriority::Standard
    ).await?;
    println!("Motion completed: seq {}", sequence_id);

    // Disconnect with confirmation
    driver.disconnect().await?;
    Ok(())
}
```

**New in v0.5.0:** Async methods (`abort()`, `initialize()`, `get_status()`, `disconnect()`) now wait for responses and return FANUC error codes for proper error handling. No more arbitrary sleep delays!

See [docs/examples/](docs/examples/) for more examples.

---

## Running the Simulator

For testing without hardware:

```bash
# Start the RMI simulator
cargo run -p sim -- --realtime

# In another terminal, run examples against localhost:18735
cargo run -p example --bin jog_client
```

The simulator emulates FANUC RMI protocol with:
- State tracking (position, configuration, status)
- Forward/inverse kinematics for CRX robots
- Realistic response timing

---

## Web Application

Modern web-based control interface with real-time monitoring.

### Features
- 🎮 **Interactive Jog Controls**: 6-axis cartesian jogging (X, Y, Z)
- 📊 **Real-time Position Display**: Live coordinates with sub-millimeter precision
- 🔴 **Status Indicators**: Servo ready, TP mode, motion status
- 📝 **Motion Log**: History of completed movements
- 🎨 **Clean Dark UI**: Professional industrial control aesthetic

### Quick Start

**Option 1: Using Trunk (Recommended)**
```bash
# Terminal 1: Start simulator
cargo run -p sim -- --realtime

# Terminal 2: Start WebSocket server
cargo run -p web_server

# Terminal 3: Build and serve web app
cd web_app && trunk serve --release
```

**Option 2: Manual Build**
```bash
# Build WASM
cd web_app && trunk build --release

# Serve (any HTTP server)
python3 -m http.server 8000 --directory dist
```

Open browser to `http://localhost:8080` (trunk) or `http://localhost:8000` (manual)

See [web_app/README.md](web_app/README.md) and [web_server/README.md](web_server/README.md) for details.

---

## Documentation

📚 **[Complete Documentation](docs/README.md)**

### Key Documents
- **[Sequence ID Migration Guide](docs/SEQUENCE_ID_MIGRATION_GUIDE.md)** - ⚠️ Required reading for v0.3.0+
- **[Position Precision Fix](docs/POSITION_PRECISION_FIX.md)** - f32→f64 precision improvement
- **[Robot Configuration](docs/ROBOT_CONFIGURATION.md)** - Supported robot models
- **[Protocol & DTO System](docs/architecture/protocol_dto_system.md)** - Architecture overview
- **[Basic Usage Examples](docs/examples/basic_usage.md)** - Code examples

### API Documentation
```bash
cargo doc --open --no-deps -p fanuc_rmi
```

---

## Project Structure

```
Fanuc_RMI_API/
├── fanuc_rmi/          # Core library (RMI protocol types and driver)
├── fanuc_rmi_macros/   # Procedural macros for DTO generation
├── example/            # CLI examples (jog client, TUI, etc.)
├── web_app/            # Leptos WASM web application
├── web_server/         # WebSocket server (driver ↔ web app bridge)
├── sim/                # RMI simulator with kinematics
├── docs/               # Documentation
└── research/           # Research papers and specifications
```

---

## Contributing

Contributions welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes with clear commit messages
4. Add tests if applicable
5. Update documentation
6. Push to your fork and open a pull request

### Development Guidelines
- Follow Rust API guidelines
- Add tests for new features
- Update documentation
- Run `cargo fmt` and `cargo clippy` before committing

---

## License

See [LICENSE](LICENSE) file for details.

---

## Acknowledgments

- FANUC Corporation for RMI specification (B-84184EN_02)
- Research paper: "Geometric Approach for Inverse Kinematics of the FANUC CRX Collaborative Robot" by Manel Abbes and Gérard Poisson (Robotics 2024, 13, 91)

---

## Support

- 📖 [Documentation](docs/README.md)
- 🐛 [Issue Tracker](https://github.com/vertec-io/Fanuc_RMI_API/issues)
- 💬 Open an issue for questions or bug reports
