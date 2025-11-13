# Fanuc RMI Documentation

Welcome to the Fanuc RMI library documentation. This library provides Rust types and utilities for communicating with FANUC robots via the RMI (Robot Messaging Interface) protocol.

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
fanuc_rmi = { version = "0.3", features = ["DTO"] }
```

Basic usage:

```rust
use fanuc_rmi::packets::CommandResponse;

fn handle_response(response: CommandResponse) {
    match response {
        CommandResponse::FrcReadJointAngles(angles) => {
            println!("Joint 1: {}", angles.joint_angles.j1);
        }
        _ => {}
    }
}
```

## Documentation Structure

### Architecture

Understanding the core design and patterns:

- **[Protocol and DTO System](architecture/protocol_dto_system.md)** - How protocol types and DTO types work together, what gets generated automatically, and when to use each
- **[Message Relay Patterns](architecture/message_relay_patterns.md)** - Three-tier architecture for routing messages in your application

### Examples

Framework-agnostic code examples:

- **[Basic Usage](examples/basic_usage.md)** - Pattern matching, ExtractInner trait, generic functions, network serialization

### Reference Implementations

Complete working examples for specific frameworks:

- **[Bevy ECS Three-Tier Relay](reference_implementations/bevy_ecs_three_tier_relay.md)** - Complete reference implementation showing how to build a three-tier message relay system with dual internal/network buses using Bevy ECS

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

