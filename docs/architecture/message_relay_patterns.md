# Message Relay Patterns

## Overview

This document describes architectural patterns for routing FANUC RMI messages in your application. These patterns are framework-agnostic but examples use Bevy ECS for illustration.

## Three-Tier Architecture

The recommended architecture has three tiers, each providing different levels of message granularity:

```
┌─────────────────────────────────────────────────────────────┐
│ TIER 1: Packet-Level Routing                               │
│ ResponsePacket → CommandResponse/InstructionResponse/etc.   │
│ Granularity: By message category (Command, Instruction)     │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ TIER 2: Individual Type Routing (Optional)                 │
│ CommandResponse → FrcReadJointAngles/FrcGetStatus/etc.      │
│ Granularity: By specific message type                       │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ TIER 3: Consumers                                           │
│ Application systems that process messages                   │
│ Choice: Pattern match enums OR subscribe to specific types  │
└─────────────────────────────────────────────────────────────┘
```

## Tier 1: Packet-Level Routing

### Purpose
Split the top-level `ResponsePacket` into three category enums:
- `CommandResponse` (21 variants)
- `InstructionResponse` (16 variants)
- `CommunicationResponse` (4 variants)

### Benefits
- Reduces noise (consumers only see relevant message categories)
- Enables category-level pattern matching
- Single conversion point for network transport

### Pattern

```rust
// Internal message bus
MessageReader<ResponsePacket> → MessageWriter<CommandResponse>
                              → MessageWriter<InstructionResponse>
                              → MessageWriter<CommunicationResponse>

// Network transport (optional)
MessageReader<ResponsePacket> → MessageWriter<OutboundMessage<dto::CommandResponse>>
                              → MessageWriter<OutboundMessage<dto::InstructionResponse>>
                              → MessageWriter<OutboundMessage<dto::CommunicationResponse>>
```

### When to Use
- **Always** - This is the foundation tier
- Provides basic message categorization
- Required for both internal and network consumers

## Tier 2: Individual Type Routing

### Purpose
Further split category enums into individual message types.

### Benefits
- Consumers can subscribe to exactly what they need
- No pattern matching required in consumer code
- Type-safe - compiler ensures correct handling
- Parallel processing - each type can be processed independently

### Pattern

```rust
// Internal message bus
MessageReader<CommandResponse> → MessageWriter<FrcReadJointAnglesResponse>
                               → MessageWriter<FrcGetStatusResponse>
                               → MessageWriter<FrcReadTCPSpeedResponse>
                               → ... (21 total)

// Network transport (optional)
MessageReader<CommandResponse> → MessageWriter<OutboundMessage<dto::FrcReadJointAnglesResponse>>
                               → MessageWriter<OutboundMessage<dto::FrcGetStatusResponse>>
                               → ... (21 total)
```

### When to Use
- When you have many consumers that each care about specific message types
- When you want to avoid pattern matching in consumer code
- When you want maximum type safety

### When to Skip
- Simple applications with few consumers
- When consumers naturally handle multiple related types together
- When the overhead of many message channels is not justified

## Tier 3: Consumer Patterns

Consumers can choose between two approaches:

### Approach A: Pattern Matching on Enums

Subscribe to category enums and pattern match:

```rust
fn my_consumer(mut reader: MessageReader<CommandResponse>) {
    for response in reader.read() {
        match response {
            CommandResponse::FrcReadJointAngles(angles) => {
                // Handle joint angles
            }
            CommandResponse::FrcGetStatus(status) => {
                // Handle status
            }
            _ => {} // Ignore others
        }
    }
}
```

**Use when:**
- Handling 2-3 related message types together
- Logic naturally groups multiple types
- Fewer systems preferred over more systems

### Approach B: Direct Type Subscription

Subscribe to specific message types:

```rust
fn my_consumer(mut reader: MessageReader<FrcReadJointAnglesResponse>) {
    for angles in reader.read() {
        // angles is already the right type - no pattern matching!
        println!("J1: {}", angles.joint_angles.j1);
    }
}
```

**Use when:**
- Only care about one specific message type
- Want maximum type safety
- Prefer single-responsibility systems
- Tier 2 routing is enabled

## Network Transport Pattern

### Dual Message Bus Strategy

For applications that need both internal processing and network distribution:

```
┌──────────────┐
│ ResponsePacket│
└───────┬──────┘
        │
        ├─────────────────────────────────────┐
        │                                     │
        ▼                                     ▼
┌───────────────┐                    ┌────────────────┐
│ Internal Bus  │                    │ Network Bus    │
│ (Protocol)    │                    │ (DTO)          │
└───────┬───────┘                    └────────┬───────┘
        │                                     │
        ▼                                     ▼
┌───────────────┐                    ┌────────────────┐
│ Internal      │                    │ WebSocket/     │
│ Consumers     │                    │ Network        │
└───────────────┘                    └────────────────┘
```

### Benefits
- Single conversion point (efficient)
- Internal consumers use protocol types (clean)
- Network consumers use DTO types (bincode-compatible)
- Both buses available at every tier

### Implementation

```rust
// Tier 1: Split packet and send to both buses
fn tier1_relay(
    mut packet_reader: MessageReader<ResponsePacket>,
    // Internal bus (protocol types)
    mut command_writer: MessageWriter<CommandResponse>,
    mut instruction_writer: MessageWriter<InstructionResponse>,
    // Network bus (DTO types)
    mut net_command_writer: MessageWriter<OutboundMessage<dto::CommandResponse>>,
    mut net_instruction_writer: MessageWriter<OutboundMessage<dto::InstructionResponse>>,
) {
    for packet in packet_reader.read() {
        match packet {
            ResponsePacket::CommandResponse(resp) => {
                // Internal
                command_writer.write(resp.clone());
                // Network
                let dto: dto::CommandResponse = resp.into();
                net_command_writer.write(OutboundMessage { payload: dto });
            }
            ResponsePacket::InstructionResponse(resp) => {
                instruction_writer.write(resp.clone());
                let dto: dto::InstructionResponse = resp.into();
                net_instruction_writer.write(OutboundMessage { payload: dto });
            }
            // ... etc
        }
    }
}
```

## Consumer Choice Matrix

| Consumer Type | Data Granularity | Approach | Tier | Example Use Case |
|---------------|------------------|----------|------|------------------|
| **Internal** | Multiple types | Pattern match enum | Tier 1 | Error handler for all commands |
| **Internal** | Single type | Direct type access | Tier 2 | Joint angles display |
| **Network** | All commands | Send entire enum | Tier 1 | WebSocket broadcast all |
| **Network** | Multiple types | Pattern match DTO enum | Tier 1 | Route to different topics |
| **Network** | Single type | Direct DTO access | Tier 2 | Dedicated joint angles stream |

## Comparison: Tier 1 Only vs. Full Three-Tier

### Tier 1 Only (Simpler)

```rust
// Setup
app.add_systems(Update, tier1_relay);

// Consumer (pattern matching)
fn my_system(mut reader: MessageReader<CommandResponse>) {
    for response in reader.read() {
        if let CommandResponse::FrcReadJointAngles(angles) = response {
            // Use angles
        }
    }
}
```

**Pros:**
- Simple setup (one relay system)
- Fewer message channels
- Good for small applications

**Cons:**
- Consumers must pattern match
- Less type-safe (can forget to handle variants)
- All consumers see all messages (more noise)

### Full Three-Tier (More Flexible)

```rust
// Setup
app.add_systems(Update, (
    tier1_relay,
    tier2_command_relay,
    tier2_instruction_relay,
));

// Consumer (direct type)
fn my_system(mut reader: MessageReader<FrcReadJointAnglesResponse>) {
    for angles in reader.read() {
        // No pattern matching needed!
        println!("J1: {}", angles.joint_angles.j1);
    }
}
```

**Pros:**
- No pattern matching in consumers
- Maximum type safety
- Consumers only see relevant messages
- Single-responsibility systems

**Cons:**
- More setup code
- More message channels
- Overkill for simple applications

## Recommendations

### For Small Applications (< 10 message types used)
- Use **Tier 1 only**
- Pattern match in consumers
- Simple and straightforward

### For Medium Applications (10-30 message types)
- Use **Tier 1 + selective Tier 2**
- Add Tier 2 routing for frequently-used types
- Pattern match for rarely-used types

### For Large Applications (> 30 message types)
- Use **Full Three-Tier**
- Maximum type safety and clarity
- Worth the setup overhead

### For Network Applications
- Always use **dual message bus** (internal + network)
- Convert to DTO at Tier 1 or Tier 2
- Let network consumers use `fanuc_rmi::dto` module

## See Also

- [Protocol and DTO System](./protocol_dto_system.md) - Understanding the type system
- [Bevy ECS Three-Tier Relay](../reference_implementations/bevy_ecs_three_tier_relay.md) - Complete implementation
- [Basic Usage Examples](../examples/basic_usage.md) - Code examples


