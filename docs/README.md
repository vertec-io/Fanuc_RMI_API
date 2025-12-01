# Fanuc RMI Documentation

Welcome to the Fanuc RMI library documentation. This library provides Rust types and utilities for communicating with FANUC robots via the RMI (Robot Messaging Interface) protocol.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
fanuc_rmi = { version = "0.5", features = ["driver", "DTO"] }
```

Basic usage:

```rust
use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = FanucDriverConfig {
        addr: "192.168.1.100".to_string(),
        port: 18735,
        max_messages: 30,
    };
    let driver = FanucDriver::connect(config).await?;

    // Initialize and check response
    let init = driver.initialize().await?;
    if init.error_id == 0 {
        println!("Robot initialized successfully");
    }

    Ok(())
}
```

## Documentation Structure

### 🌐 Web Interface (v0.6.0)

- **[Web Interface Implementation](WEB_INTERFACE_IMPLEMENTATION.md)** - Complete web app architecture and features
- **[Implementation Roadmap V2](IMPLEMENTATION_ROADMAP_V2.md)** - Design specification and roadmap
- **[UI Design Mockup](UI_DESIGN_MOCKUP.md)** - Visual mockups and design system

### 📚 Core Documentation

**Current & Active:**
- **[v0.5.0 Implementation Summary](V0.5.0_IMPLEMENTATION_SUMMARY.md)** - Async command methods and error handling
- **[Naming Migration Guide v0.5.0](NAMING_MIGRATION_GUIDE_v0.5.0.md)** - Migration guide for v0.5.0 changes
- **[Position Precision Fix](POSITION_PRECISION_FIX.md)** - Detailed explanation of f32→f64 precision improvement
- **[Robot Configuration](ROBOT_CONFIGURATION.md)** - Supported robot models for the simulator and kinematic parameters
- **[RMI Commands Reference](RMI_COMMANDS_REFERENCE.md)** - Complete RMI protocol command reference

**Legacy/Historical:**
- **[Sequence ID Migration Guide](SEQUENCE_ID_MIGRATION_GUIDE.md)** - Migration guide for correlation ID system (v0.3.0+)
- **[Implementation Summary](IMPLEMENTATION_SUMMARY.md)** - Historical implementation notes

### 🏗️ Architecture

Understanding the core design and patterns:

- **[Protocol and DTO System](architecture/protocol_dto_system.md)** - How protocol types and DTO types work together, what gets generated automatically, and when to use each
- **[Message Relay Patterns](architecture/message_relay_patterns.md)** - Three-tier architecture for routing messages in your application

### 📖 Examples

Framework-agnostic code examples:

- **[Basic Usage](examples/basic_usage.md)** - Pattern matching, ExtractInner trait, generic functions, network serialization
- **[Correlation ID Usage](examples/correlation_id_usage.rs)** - Complete examples of all correlation ID patterns

### 🔧 Reference Implementations

Complete working examples for specific frameworks:

- **[Bevy ECS Three-Tier Relay](reference_implementations/bevy_ecs_three_tier_relay.md)** - Complete reference implementation showing how to build a three-tier message relay system with dual internal/network buses using Bevy ECS

### 📝 Release Notes

- **[v0.6.0](releases/RELEASE_NOTES_v0.6.0.md)** - Major web interface overhaul with desktop-style UI
- **[v0.5.0](releases/RELEASE_NOTES_v0.5.0.md)** - Async command methods, proper error handling
- **[v0.4.0](releases/RELEASE_NOTES_v0.4.0.md)** - Position precision fix, request ID system
- **[v0.3.0](releases/RELEASE_NOTES_v0.3.0.md)** - ExtractInner trait, DTO enums, comprehensive documentation
- **[v0.2.0 Web App](releases/RELEASE_NOTES_v0.2.0_WEB_APP.md)** - Web application redesign with dark mode UI

### 🔍 Historical Fixes

Documentation of past issues and their solutions (for reference):

- **[Configuration Fix](historical-fixes/CONFIGURATION_FIX_SUMMARY.md)** - Configuration struct compatibility fix
- **[Sequence ID Fixes](historical-fixes/FINAL_SEQUENCE_ID_FIX.md)** - Complete sequence ID bug fixes (3 root causes)
- **[Sequence ID Fix Summary](historical-fixes/SEQUENCE_ID_FIX_SUMMARY.md)** - Initial sequence ID investigation
- **[Jog Functionality Fix](historical-fixes/JOG_FUNCTIONALITY_FIX.md)** - Simulator state tracking implementation
- **[IK Implementation](historical-fixes/FULL_IK_IMPLEMENTATION_SUMMARY.md)** - 7-step geometric IK solver
- **[Kinematics Update](historical-fixes/KINEMATICS_UPDATE_SUMMARY.md)** - CRX-10iA kinematics alignment

## Key Concepts

### Protocol Types vs DTO Types

The library uses two parallel type systems:

| Aspect | Protocol Types | DTO Types |
|--------|---------------|-----------|
| **Purpose** | Internal processing, JSON with robot | Network transport, bincode serialization |
| **Location** | `fanuc_rmi::packets`, `fanuc_rmi::commands` | `fanuc_rmi::dto` |
| **Field Names** | With serde rename (`ErrorID` → `error_id`) | Clean (no rename) |
| **Use Case** | Application logic | Network boundaries |

**Example:**

```rust
// Internal processing - use protocol types
use fanuc_rmi::packets::CommandResponse;

fn process(response: CommandResponse) {
    // Work with protocol type
}

// Network transport - convert to DTO
use fanuc_rmi::dto;

fn send_over_network(response: CommandResponse) {
    let dto: dto::CommandResponse = response.into();
    let bytes = bincode::serialize(&dto).unwrap();
    // Send bytes...
}
```

### Three-Tier Message Relay

The recommended architecture for routing messages:

```
Tier 1: ResponsePacket → CommandResponse/InstructionResponse/CommunicationResponse
Tier 2: CommandResponse → FrcReadJointAngles/FrcGetStatus/etc. (optional)
Tier 3: Consumers (pattern match OR direct type subscription)
```

**Benefits:**
- Type safety
- Reduced noise (consumers only see relevant messages)
- Flexible granularity (choose enum or individual types)
- Network transport at every tier

### ExtractInner Trait

Type-safe extraction from enums:

```rust
use fanuc_rmi::ExtractInner;
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::commands::FrcReadJointAnglesResponse;

fn extract(response: CommandResponse) {
    // Type annotation tells compiler which type to extract
    let angles: Option<&FrcReadJointAnglesResponse> = response.as_inner();
    
    if let Some(angles) = angles {
        println!("Got angles: {:?}", angles);
    }
}
```

## Common Use Cases

### Simple Application (Pattern Matching)

```rust
use fanuc_rmi::packets::CommandResponse;

fn handle_responses(response: CommandResponse) {
    match response {
        CommandResponse::FrcReadJointAngles(angles) => {
            println!("Angles: {:?}", angles.joint_angles);
        }
        CommandResponse::FrcGetStatus(status) => {
            println!("Status: {:?}", status);
        }
        _ => {}
    }
}
```

### Network Application (DTO Conversion)

```rust
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::dto;

fn relay_to_network(response: CommandResponse) {
    // Convert to DTO at boundary
    let dto: dto::CommandResponse = response.into();
    
    // Serialize with bincode
    let bytes = bincode::serialize(&dto).unwrap();
    
    // Send over WebSocket, TCP, etc.
    websocket.send(bytes);
}
```

### Bevy ECS Application (Three-Tier Relay)

See the complete [Bevy ECS Three-Tier Relay](reference_implementations/bevy_ecs_three_tier_relay.md) reference implementation.

## Feature Flags

### `DTO` Feature

Enables DTO type generation and conversions:

```toml
[dependencies]
fanuc_rmi = { version = "0.3", features = ["DTO"] }
```

When enabled:
- Generates `*Dto` types for all protocol types
- Provides `From` conversions between protocol and DTO types
- Re-exports DTO types in `fanuc_rmi::dto` module with clean names

When disabled:
- Only protocol types are available
- Smaller compile times and binary size
- Use when you don't need network serialization

## Next Steps

1. **New to the library?** Start with [Basic Usage](examples/basic_usage.md)
2. **Building a network application?** Read [Protocol and DTO System](architecture/protocol_dto_system.md)
3. **Using Bevy ECS?** Check out the [Bevy ECS Three-Tier Relay](reference_implementations/bevy_ecs_three_tier_relay.md)
4. **Need architectural guidance?** See [Message Relay Patterns](architecture/message_relay_patterns.md)

## Contributing

Found an issue or have a suggestion? Please open an issue on GitHub.

## License

See the LICENSE file in the repository root.

