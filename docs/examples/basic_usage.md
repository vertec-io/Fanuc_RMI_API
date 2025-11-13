# Basic Usage Examples

This document provides framework-agnostic examples of using the Fanuc RMI library.

## Pattern Matching on Enums

### Simple Match

```rust
use fanuc_rmi::packets::CommandResponse;

fn handle_response(response: CommandResponse) {
    match response {
        CommandResponse::FrcReadJointAngles(angles) => {
            println!("Joint 1: {}", angles.joint_angles.j1);
            println!("Joint 2: {}", angles.joint_angles.j2);
            println!("Joint 3: {}", angles.joint_angles.j3);
        }
        CommandResponse::FrcGetStatus(status) => {
            println!("Servo Ready: {}", status.servo_ready);
            println!("TP Mode: {}", status.tp_mode);
        }
        _ => {
            println!("Other response type");
        }
    }
}
```

### Handling Multiple Types

```rust
use fanuc_rmi::packets::CommandResponse;

fn handle_multiple_types(response: CommandResponse) {
    match response {
        CommandResponse::FrcReadJointAngles(angles) => {
            if angles.error_id == 0 {
                println!("Joint angles: {:?}", angles.joint_angles);
            } else {
                eprintln!("Error reading joint angles: {}", angles.error_id);
            }
        }
        CommandResponse::FrcReadCartesianPosition(position) => {
            if position.error_id == 0 {
                println!("Position: X={}, Y={}, Z={}", 
                    position.position.x,
                    position.position.y,
                    position.position.z
                );
            }
        }
        CommandResponse::FrcGetStatus(status) => {
            println!("Robot status: servo_ready={}, tp_mode={}", 
                status.servo_ready,
                status.tp_mode
            );
        }
        _ => {
            // Ignore other types
        }
    }
}
```

### Error Checking Pattern

```rust
use fanuc_rmi::packets::CommandResponse;

fn check_for_errors(response: CommandResponse) -> Result<(), String> {
    match response {
        CommandResponse::FrcReadJointAngles(ref angles) if angles.error_id != 0 => {
            Err(format!("Joint angles read failed: error_id={}", angles.error_id))
        }
        CommandResponse::FrcGetStatus(ref status) if status.error_id != 0 => {
            Err(format!("Status read failed: error_id={}", status.error_id))
        }
        CommandResponse::FrcReadCartesianPosition(ref pos) if pos.error_id != 0 => {
            Err(format!("Position read failed: error_id={}", pos.error_id))
        }
        _ => Ok(()),
    }
}
```

## Using ExtractInner Trait

### Basic Extraction

```rust
use fanuc_rmi::ExtractInner;
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::commands::FrcReadJointAnglesResponse;

fn extract_joint_angles(response: CommandResponse) {
    // Type annotation tells compiler which type to extract
    let angles: Option<&FrcReadJointAnglesResponse> = response.as_inner();
    
    if let Some(angles) = angles {
        println!("Got angles: {:?}", angles);
    } else {
        println!("Response is not joint angles");
    }
}
```

### Taking Ownership

```rust
use fanuc_rmi::ExtractInner;
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::commands::FrcReadJointAnglesResponse;

fn consume_response(response: CommandResponse) {
    // into_inner consumes the response
    let angles: Option<FrcReadJointAnglesResponse> = response.into_inner();
    
    if let Some(angles) = angles {
        // angles is now owned - can be stored or moved
        store_angles_somewhere(angles);
    }
}

fn store_angles_somewhere(angles: FrcReadJointAnglesResponse) {
    // Store or process the owned value
}
```

### Expect Pattern (Panics on Mismatch)

```rust
use fanuc_rmi::ExtractInner;
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::commands::FrcReadJointAnglesResponse;

fn expect_specific_type(response: CommandResponse) {
    // Panics if not the expected type
    let angles: &FrcReadJointAnglesResponse = 
        response.expect_inner("Expected joint angles response");
    
    println!("Angles: {:?}", angles);
}
```

### Multiple Extractions

```rust
use fanuc_rmi::ExtractInner;
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::commands::{FrcReadJointAnglesResponse, FrcGetStatusResponse};

fn try_multiple_types(response: CommandResponse) {
    // Try extracting as joint angles
    if let Some(angles) = response.as_inner::<FrcReadJointAnglesResponse>() {
        println!("Got joint angles: {:?}", angles);
        return;
    }

    // Try extracting as status
    if let Some(status) = response.as_inner::<FrcGetStatusResponse>() {
        println!("Got status: {:?}", status);
        return;
    }

    println!("Unknown response type");
}
```

## Generic Functions with ExtractInner

### Generic Error Checker

```rust
use fanuc_rmi::ExtractInner;
use fanuc_rmi::packets::CommandResponse;

// Trait for types that have an error_id field
trait HasErrorId {
    fn error_id(&self) -> u32;
}

// Implement for specific response types
impl HasErrorId for fanuc_rmi::commands::FrcReadJointAnglesResponse {
    fn error_id(&self) -> u32 {
        self.error_id
    }
}

impl HasErrorId for fanuc_rmi::commands::FrcGetStatusResponse {
    fn error_id(&self) -> u32 {
        self.error_id
    }
}

// Generic function that works with any extractable type that has error_id
fn check_error<T>(response: &CommandResponse) -> Result<(), String>
where
    CommandResponse: ExtractInner<T>,
    T: HasErrorId,
{
    if let Some(data) = response.as_inner::<T>() {
        if data.error_id() != 0 {
            return Err(format!("Error ID: {}", data.error_id()));
        }
    }
    Ok(())
}

// Usage
fn example() {
    let response: CommandResponse = get_response();

    // Check for errors in joint angles response
    if let Err(e) = check_error::<fanuc_rmi::commands::FrcReadJointAnglesResponse>(&response) {
        eprintln!("Joint angles error: {}", e);
    }
}
```

### Generic Processor

```rust
use fanuc_rmi::ExtractInner;
use fanuc_rmi::packets::CommandResponse;

fn process_if_present<T, F>(response: CommandResponse, processor: F)
where
    CommandResponse: ExtractInner<T>,
    F: FnOnce(&T),
{
    if let Some(data) = response.as_inner::<T>() {
        processor(data);
    }
}

// Usage
fn example() {
    let response: CommandResponse = get_response();

    process_if_present(response, |angles: &fanuc_rmi::commands::FrcReadJointAnglesResponse| {
        println!("J1: {}", angles.joint_angles.j1);
    });
}
```

## Network Serialization

### Converting to DTO for Network Transport

```rust
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::dto;

fn send_over_network(response: CommandResponse) {
    // Convert protocol type to DTO
    let dto_response: dto::CommandResponse = response.into();

    // Serialize with bincode (efficient binary format)
    let encoded = bincode::serialize(&dto_response).unwrap();

    // Send over network (WebSocket, TCP, etc.)
    send_bytes(encoded);
}

fn send_bytes(data: Vec<u8>) {
    // Your network code here
}
```

### Receiving from Network

```rust
use fanuc_rmi::dto;

fn receive_from_network(data: &[u8]) {
    // Deserialize from bincode
    let dto_response: dto::CommandResponse = bincode::deserialize(data).unwrap();

    // Use the DTO directly (same API as protocol types)
    match dto_response {
        dto::CommandResponse::FrcReadJointAngles(angles) => {
            println!("Received joint angles: {:?}", angles);
        }
        dto::CommandResponse::FrcGetStatus(status) => {
            println!("Received status: {:?}", status);
        }
        _ => {}
    }
}
```

### Converting DTO Back to Protocol

```rust
use fanuc_rmi::dto;
use fanuc_rmi::packets::CommandResponse;

fn dto_to_protocol(dto_response: dto::CommandResponse) {
    // Convert DTO back to protocol type
    let protocol_response: CommandResponse = dto_response.into();

    // Now can use with internal systems
    process_internally(protocol_response);
}

fn process_internally(response: CommandResponse) {
    // Your internal processing
}
```

## Working with Different Response Categories

### Command Responses

```rust
use fanuc_rmi::packets::CommandResponse;

fn handle_command_response(response: CommandResponse) {
    match response {
        CommandResponse::FrcReadJointAngles(angles) => { /* ... */ }
        CommandResponse::FrcReadCartesianPosition(pos) => { /* ... */ }
        CommandResponse::FrcGetStatus(status) => { /* ... */ }
        CommandResponse::FrcReadTCPSpeed(speed) => { /* ... */ }
        // ... 17 more command variants
        _ => {}
    }
}
```

### Instruction Responses

```rust
use fanuc_rmi::packets::InstructionResponse;

fn handle_instruction_response(response: InstructionResponse) {
    match response {
        InstructionResponse::FrcMoveJoint(result) => { /* ... */ }
        InstructionResponse::FrcMoveLinear(result) => { /* ... */ }
        InstructionResponse::FrcSetSpeed(result) => { /* ... */ }
        // ... 13 more instruction variants
        _ => {}
    }
}
```

### Communication Responses

```rust
use fanuc_rmi::packets::CommunicationResponse;

fn handle_communication_response(response: CommunicationResponse) {
    match response {
        CommunicationResponse::Connect(result) => {
            println!("Connection result: {:?}", result);
        }
        CommunicationResponse::Disconnect(result) => {
            println!("Disconnection result: {:?}", result);
        }
        CommunicationResponse::Heartbeat(result) => {
            println!("Heartbeat: {:?}", result);
        }
        CommunicationResponse::GetConfiguration(config) => {
            println!("Configuration: {:?}", config);
        }
    }
}
```

## See Also

- [Protocol and DTO System](../architecture/protocol_dto_system.md) - Understanding the type system
- [Message Relay Patterns](../architecture/message_relay_patterns.md) - Architectural patterns
- [Bevy ECS Three-Tier Relay](../reference_implementations/bevy_ecs_three_tier_relay.md) - Complete implementation


