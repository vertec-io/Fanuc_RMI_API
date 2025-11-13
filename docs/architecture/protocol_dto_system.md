# Protocol and DTO System Architecture

## Overview

The Fanuc RMI library uses a dual-type system to balance developer ergonomics with network compatibility:

- **Protocol Types**: Used internally, with serde rename attributes for JSON compatibility with FANUC's RMI protocol
- **DTO Types**: Used for network transport, with clean field names for bincode compatibility

## The Problem

FANUC's RMI protocol uses PascalCase field names in JSON:

```json
{
  "Command": "FrcReadJointAngles",
  "ErrorID": 0,
  "TimeTag": 123,
  "JointAngles": { "J1": 0.0, "J2": 0.0, "J3": 0.0, "J4": 0.0, "J5": 0.0, "J6": 0.0 }
}
```

This requires serde rename attributes in Rust:

```rust
#[derive(Serialize, Deserialize)]
pub struct FrcReadJointAnglesResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    
    #[serde(rename = "TimeTag")]
    pub time_tag: i16,
    
    #[serde(rename = "JointAngles")]
    pub joint_angles: JointAngles,
}
```

**Problem**: Bincode serialization includes the serde rename metadata in its binary format, making it incompatible with clients that expect clean field names. Additionally, the `#[serde(tag = "Command")]` attribute on enums creates tagged representations that bloat the binary encoding.

## The Solution: DTO Types

The `#[mirror_dto]` macro automatically generates clean DTO types without serde rename attributes:

```rust
// Protocol type (in fanuc_rmi::commands)
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrcReadJointAnglesResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "TimeTag")]
    pub time_tag: i16,
    #[serde(rename = "JointAngles")]
    pub joint_angles: JointAngles,
}

// Generated DTO type (in fanuc_rmi::dto)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrcReadJointAnglesResponseDto {
    pub error_id: u32,      // No serde rename!
    pub time_tag: i16,      // Clean field names
    pub joint_angles: JointAnglesDto,  // Nested DTOs
}

// Auto-generated conversion
impl From<FrcReadJointAnglesResponse> for FrcReadJointAnglesResponseDto {
    fn from(src: FrcReadJointAnglesResponse) -> Self {
        Self {
            error_id: src.error_id,
            time_tag: src.time_tag,
            joint_angles: src.joint_angles.into(),
        }
    }
}
```

## Type Hierarchy

### Structs (Response Data)

```
Protocol Struct                    DTO Struct
(fanuc_rmi::commands)             (fanuc_rmi::dto)
┌─────────────────────────┐       ┌─────────────────────────┐
│ FrcReadJointAngles      │       │ FrcReadJointAngles      │
│ Response                │──────>│ ResponseDto             │
│                         │ From  │                         │
│ - error_id (renamed)    │       │ - error_id (clean)      │
│ - time_tag (renamed)    │       │ - time_tag (clean)      │
│ - joint_angles (nested) │       │ - joint_angles (DTO)    │
└─────────────────────────┘       └─────────────────────────┘
```

### Enums (Response Collections)

```
Protocol Enum                      DTO Enum
(fanuc_rmi::packets)              (fanuc_rmi::dto)
┌─────────────────────────┐       ┌─────────────────────────┐
│ CommandResponse         │       │ CommandResponse         │
│ (with serde tags)       │──────>│ (CommandResponseDto)    │
│                         │ From  │ (no serde tags)         │
│ - FrcReadJointAngles(   │       │ - FrcReadJointAngles(   │
│     Protocol Struct)    │       │     DTO Struct)         │
│ - FrcGetStatus(...)     │       │ - FrcGetStatus(...)     │
│ - ... 21 variants       │       │ - ... 21 variants       │
└─────────────────────────┘       └─────────────────────────┘
```

### Top-Level Packet Hierarchy

```
┌─────────────────────────────────────────────────────────┐
│                    ResponsePacket                       │
│                                                         │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────┐│
│  │ CommandResponse  │  │InstructionResponse│  │Comms...││
│  │  (21 variants)   │  │  (16 variants)    │  │(2 var.)││
│  └──────────────────┘  └──────────────────┘  └────────┘│
└─────────────────────────────────────────────────────────┘
```

## Generated Code

### What Gets Generated Automatically

When you add `#[cfg_attr(feature = "DTO", crate::mirror_dto)]` to a struct or enum:

1. **DTO Struct/Enum** with clean field names (no serde rename attributes)
2. **Bidirectional From conversions** (Protocol ↔ DTO)
3. **Nested type handling** (automatically converts nested protocol types to DTO types)

### Example: Struct Generation

```rust
// Source code in fanuc_rmi/src/commands/frc_readjointangles.rs
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrcReadJointAnglesResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "TimeTag")]
    pub time_tag: i16,
    #[serde(rename = "JointAngles")]
    pub joint_angles: JointAngles,
}

// Generated (when feature = "DTO" is enabled)
#[cfg(feature = "DTO")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FrcReadJointAnglesResponseDto {
    pub error_id: u32,
    pub time_tag: i16,
    pub joint_angles: JointAnglesDto,  // Nested types become DTOs
}

#[cfg(feature = "DTO")]
impl From<FrcReadJointAnglesResponse> for FrcReadJointAnglesResponseDto {
    fn from(src: FrcReadJointAnglesResponse) -> Self {
        Self {
            error_id: src.error_id,
            time_tag: src.time_tag,
            joint_angles: src.joint_angles.into(),  // Nested conversion
        }
    }
}

#[cfg(feature = "DTO")]
impl From<FrcReadJointAnglesResponseDto> for FrcReadJointAnglesResponse {
    fn from(src: FrcReadJointAnglesResponseDto) -> Self {
        Self {
            error_id: src.error_id,
            time_tag: src.time_tag,
            joint_angles: src.joint_angles.into(),
        }
    }
}

// In fanuc_rmi/src/commands/mod.rs - dto submodule
#[cfg(feature = "DTO")]
pub mod dto {
    pub use super::frc_readjointangles::FrcReadJointAnglesResponseDto as FrcReadJointAnglesResponse;
    // ... other re-exports
}
```

### Example: Enum Generation

```rust
// Source code in fanuc_rmi/src/packets/command.rs
#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "Command")]
pub enum CommandResponse {
    #[serde(rename = "FRC_ReadJointAngles")]
    FrcReadJointAngles(FrcReadJointAnglesResponse),

    #[serde(rename = "FRC_GetStatus")]
    FrcGetStatus(FrcGetStatusResponse),

    // ... 19 more variants
}

// Generated (when feature = "DTO" is enabled)
#[cfg(feature = "DTO")]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum CommandResponseDto {
    FrcReadJointAngles(FrcReadJointAnglesResponseDto),  // DTO variant
    FrcGetStatus(FrcGetStatusResponseDto),              // DTO variant
    // ... 19 more variants (no serde rename!)
}

#[cfg(feature = "DTO")]
impl From<CommandResponse> for CommandResponseDto {
    fn from(src: CommandResponse) -> Self {
        match src {
            CommandResponse::FrcReadJointAngles(inner) => {
                CommandResponseDto::FrcReadJointAngles(inner.into())
            }
            CommandResponse::FrcGetStatus(inner) => {
                CommandResponseDto::FrcGetStatus(inner.into())
            }
            // ... all variants
        }
    }
}

#[cfg(feature = "DTO")]
impl From<CommandResponseDto> for CommandResponse {
    fn from(src: CommandResponseDto) -> Self {
        match src {
            CommandResponseDto::FrcReadJointAngles(inner) => {
                CommandResponse::FrcReadJointAngles(inner.into())
            }
            CommandResponseDto::FrcGetStatus(inner) => {
                CommandResponse::FrcGetStatus(inner.into())
            }
            // ... all variants
        }
    }
}

// In fanuc_rmi/src/dto/mod.rs - top-level re-export
#[cfg(feature = "DTO")]
pub use crate::packets::CommandResponseDto as CommandResponse;
```

## Usage Patterns

### Internal Systems (Use Protocol Types)

Internal application code should use protocol types directly:

```rust
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::commands::FrcReadJointAnglesResponse;

fn handle_response(response: CommandResponse) {
    match response {
        CommandResponse::FrcReadJointAngles(angles) => {
            // angles is protocol type
            println!("Error ID: {}", angles.error_id);
            println!("J1: {}", angles.joint_angles.j1);
        }
        CommandResponse::FrcGetStatus(status) => {
            println!("Servo Ready: {}", status.servo_ready);
        }
        _ => {}
    }
}
```

### Network Transport (Convert to DTO at Boundary)

When sending data over the network, convert to DTO at the boundary:

```rust
use fanuc_rmi::dto;

fn send_over_network(response: fanuc_rmi::packets::CommandResponse) {
    // Convert protocol to DTO
    let dto: dto::CommandResponse = response.into();

    // Serialize with bincode (clean, efficient binary)
    let encoded = bincode::serialize(&dto).unwrap();

    // Send over network...
    websocket.send(encoded);
}
```

### Client Code (Uses DTO Module)

Client code uses the DTO module, which re-exports types with the same names:

```rust
// Client doesn't know about "Dto" suffix
use fanuc_rmi::dto::CommandResponse;
use fanuc_rmi::dto::FrcReadJointAnglesResponse;

fn receive_from_network(data: &[u8]) {
    // Deserialize from bincode
    let response: CommandResponse = bincode::deserialize(data).unwrap();

    match response {
        CommandResponse::FrcReadJointAngles(angles) => {
            // angles is DTO type, but client uses same API
            println!("Error ID: {}", angles.error_id);
            println!("J1: {}", angles.joint_angles.j1);
        }
        _ => {}
    }
}
```

## Extraction Traits

The library provides `ExtractInner<T>` for type-safe extraction from enums:

```rust
use fanuc_rmi::ExtractInner;
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::commands::FrcReadJointAnglesResponse;

fn handle_response(response: CommandResponse) {
    // Extract specific type with type annotation
    let angles: Option<&FrcReadJointAnglesResponse> = response.as_inner();

    if let Some(angles) = angles {
        println!("Got angles: {:?}", angles);
    }
}
```

This enables generic code:

```rust
fn extract_and_process<T>(response: CommandResponse)
where
    CommandResponse: ExtractInner<T>,
{
    if let Some(data) = response.as_inner::<T>() {
        // Process data
    }
}
```

The `ExtractInner` trait provides three methods:

- `as_inner(&self) -> Option<&T>` - Borrow the inner value
- `into_inner(self) -> Option<T>` - Take ownership of the inner value
- `expect_inner(&self, msg: &str) -> &T` - Panic if not the expected type

## Module Organization

```
fanuc_rmi/
├── src/
│   ├── lib.rs                    # Root module
│   ├── extract.rs                # ExtractInner trait
│   ├── packets/
│   │   ├── mod.rs
│   │   ├── command.rs            # Command & CommandResponse enums
│   │   ├── instruction.rs        # Instruction & InstructionResponse enums
│   │   └── communication.rs      # Communication & CommunicationResponse enums
│   ├── commands/
│   │   ├── mod.rs                # Re-exports + dto submodule
│   │   ├── frc_readjointangles.rs
│   │   └── ...                   # 21 command files
│   ├── instructions/
│   │   ├── mod.rs                # Re-exports + dto submodule
│   │   └── ...                   # 16 instruction files
│   └── dto/
│       └── mod.rs                # Top-level DTO re-exports
```

### DTO Re-export Strategy

Each module has a `dto` submodule that re-exports DTO types with the same names:

```rust
// fanuc_rmi/src/commands/mod.rs
#[cfg(feature = "DTO")]
pub mod dto {
    pub use super::frc_readjointangles::FrcReadJointAnglesResponseDto as FrcReadJointAnglesResponse;
    pub use super::frc_getstatus::FrcGetStatusResponseDto as FrcGetStatusResponse;
    // ... all command DTOs
}
```

The top-level `dto` module aggregates everything:

```rust
// fanuc_rmi/src/dto/mod.rs
pub use crate::commands::dto::*;
pub use crate::instructions::dto::*;
pub use crate::packets::CommandResponseDto as CommandResponse;
pub use crate::packets::InstructionResponseDto as InstructionResponse;
pub use crate::packets::CommunicationResponseDto as CommunicationResponse;
```

This allows clients to use:

```rust
use fanuc_rmi::dto::CommandResponse;           // Enum
use fanuc_rmi::dto::FrcReadJointAnglesResponse; // Struct
```

Instead of:

```rust
use fanuc_rmi::dto::CommandResponseDto;                // Awkward!
use fanuc_rmi::dto::FrcReadJointAnglesResponseDto;     // Awkward!
```

## Summary

| Aspect | Protocol Types | DTO Types |
|--------|---------------|-----------|
| **Location** | `fanuc_rmi::packets`, `fanuc_rmi::commands`, `fanuc_rmi::instructions` | `fanuc_rmi::dto` |
| **Field Names** | With serde rename (`#[serde(rename = "ErrorID")]`) | Clean (no rename) |
| **Enum Tags** | With serde tags (`#[serde(tag = "Command")]`) | No tags (cleaner binary) |
| **Use Case** | Internal processing, JSON communication with robot | Network transport, bincode serialization |
| **Serialization** | JSON (FANUC protocol) | Bincode (efficient binary) |
| **Generated** | Manual (source code) | Automatic (via `#[mirror_dto]` macro) |
| **Naming** | `FrcReadJointAnglesResponse` | `FrcReadJointAnglesResponseDto` (re-exported as `FrcReadJointAnglesResponse`) |
| **Conversions** | N/A | Bidirectional `From` trait implementations |

## Best Practices

1. **Use protocol types internally** - Keep your application logic using protocol types
2. **Convert at the boundary** - Only convert to DTO when sending over network
3. **Let clients use dto module** - Clients import from `fanuc_rmi::dto` for clean names
4. **Enable DTO feature** - Add `features = ["DTO"]` to your Cargo.toml when you need network serialization
5. **Use ExtractInner for type safety** - Leverage the extraction traits for cleaner code

## See Also

- [Message Relay Patterns](./message_relay_patterns.md) - Architectural patterns for message routing
- [Basic Usage Examples](../examples/basic_usage.md) - Code examples
- [Bevy ECS Three-Tier Relay](../reference_implementations/bevy_ecs_three_tier_relay.md) - Complete reference implementation


