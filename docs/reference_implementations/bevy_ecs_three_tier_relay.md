# Bevy ECS Three-Tier Relay Reference Implementation

This document provides a complete reference implementation of the three-tier message relay system using Bevy ECS and the `eventwork` message bus.

## Important Notes

**Bevy 0.17 Requirement**: This implementation uses Bevy 0.17, which requires Rust nightly (1.88.0+). To set up:

```bash
rustup install nightly
rustup override set nightly  # In your project directory
```

**Eventwork 1.1**: Uses the latest eventwork with automatic message registration (no `NetworkMessage` trait needed).

**Message Bus Strategy**: This example uses **Bevy's built-in event system** for internal message passing (between relay tiers and to consumers). For network transport, you would integrate eventwork's network providers. This hybrid approach is recommended because:
- Bevy events are simpler for internal communication
- Eventwork is designed for network transport
- You get the best of both worlds

If you prefer to use eventwork for both internal and network messaging, you can replace `EventReader`/`EventWriter` with eventwork's message readers/writers throughout.

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         TIER 1: Packet Splitter                         │
│                                                                         │
│  ResponsePacket ──┬──> CommandResponse (internal)                      │
│                   │    └──> OutboundMessage<dto::CommandResponse>      │
│                   │                                                     │
│                   ├──> InstructionResponse (internal)                  │
│                   │    └──> OutboundMessage<dto::InstructionResponse>  │
│                   │                                                     │
│                   └──> CommunicationResponse (internal)                │
│                        └──> OutboundMessage<dto::CommunicationResponse>│
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                    TIER 2: Individual Type Dispatcher                   │
│                                                                         │
│  CommandResponse ──┬──> FrcReadJointAnglesResponse (internal)          │
│                    │    └──> OutboundMessage<dto::FrcReadJointAngles...>│
│                    │                                                    │
│                    ├──> FrcGetStatusResponse (internal)                │
│                    │    └──> OutboundMessage<dto::FrcGetStatus...>     │
│                    │                                                    │
│                    └──> ... (19 more command types)                    │
│                                                                         │
│  InstructionResponse ──┬──> FrcMoveJointResponse (internal)            │
│                        │    └──> OutboundMessage<dto::FrcMoveJoint...> │
│                        │                                                │
│                        └──> ... (15 more instruction types)            │
└─────────────────────────────────────────────────────────────────────────┘
                                    ↓
┌─────────────────────────────────────────────────────────────────────────┐
│                         TIER 3: Consumers                               │
│                                                                         │
│  Option A: Pattern match on enums (Tier 1 or Tier 2)                   │
│  Option B: Subscribe to specific types (Tier 2 only)                   │
└─────────────────────────────────────────────────────────────────────────┘
```

## Dependencies

Add to your `Cargo.toml`:

```toml
[dependencies]
bevy = "0.17"
eventwork = "1.1"  # Core eventwork crate
fanuc_rmi = { version = "0.3", features = ["DTO"] }
bincode = "1.3.3"
serde = { version = "1.0.190", features = ["derive"] }

# Optional: For WebSocket support
# eventwork_websockets = "1.1"
```

**Note**: Bevy 0.17 requires Rust nightly 1.88.0+. See setup instructions above.

## Message Type Definitions

First, define your message types for eventwork:

```rust
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// Wrapper for outbound network messages
// With eventwork 1.1, you just need Serialize + Deserialize - no trait implementation needed!
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutboundMessage<T> {
    pub payload: T,
}

// Internal message types (protocol types) - these are NOT sent over network
// They're used for internal Bevy ECS message passing only
// No Serialize/Deserialize needed for internal-only types, but we include them
// because we might want to convert them to network messages later
```

## Tier 1: Packet-Level Relay

This tier splits `ResponsePacket` into three category enums and sends to both internal and network buses.

**Note**: For this example, we're using Bevy's built-in `EventReader` and `EventWriter` for internal messages, and eventwork's network messaging for external communication. You could also use eventwork for both if you prefer.

```rust
use bevy::prelude::*;
use fanuc_rmi::packets::{ResponsePacket, CommandResponse, InstructionResponse, CommunicationResponse};
use fanuc_rmi::dto;

// Define Bevy events for internal message passing
#[derive(Event, Clone)]
pub struct CommandResponseEvent(pub CommandResponse);

#[derive(Event, Clone)]
pub struct InstructionResponseEvent(pub InstructionResponse);

#[derive(Event, Clone)]
pub struct CommunicationResponseEvent(pub CommunicationResponse);

// Define network events (these will be sent via eventwork)
#[derive(Event, Clone)]
pub struct NetworkCommandEvent(pub OutboundMessage<dto::CommandResponse>);

#[derive(Event, Clone)]
pub struct NetworkInstructionEvent(pub OutboundMessage<dto::InstructionResponse>);

#[derive(Event, Clone)]
pub struct NetworkCommunicationEvent(pub OutboundMessage<dto::CommunicationResponse>);

/// Tier 1: Split ResponsePacket into category enums
/// Sends to both internal (protocol) and network (DTO) buses
pub fn tier1_packet_relay(
    mut response_reader: EventReader<ResponsePacket>,

    // Internal message bus (protocol types) - Bevy events
    mut command_writer: EventWriter<CommandResponseEvent>,
    mut instruction_writer: EventWriter<InstructionResponseEvent>,
    mut communication_writer: EventWriter<CommunicationResponseEvent>,

    // Network message bus (DTO types) - will be sent via eventwork
    mut net_command_writer: EventWriter<NetworkCommandEvent>,
    mut net_instruction_writer: EventWriter<NetworkInstructionEvent>,
    mut net_communication_writer: EventWriter<NetworkCommunicationEvent>,
) {
    for packet in response_reader.read() {
        match packet {
            ResponsePacket::CommandResponse(resp) => {
                // Send to internal bus (protocol type)
                command_writer.send(CommandResponseEvent(resp.clone()));

                // Convert to DTO and send to network bus
                let dto: dto::CommandResponse = resp.clone().into();
                net_command_writer.send(NetworkCommandEvent(OutboundMessage { payload: dto }));
            }

            ResponsePacket::InstructionResponse(resp) => {
                instruction_writer.send(InstructionResponseEvent(resp.clone()));
                let dto: dto::InstructionResponse = resp.clone().into();
                net_instruction_writer.send(NetworkInstructionEvent(OutboundMessage { payload: dto }));
            }

            ResponsePacket::CommunicationResponse(resp) => {
                communication_writer.send(CommunicationResponseEvent(resp.clone()));
                let dto: dto::CommunicationResponse = resp.clone().into();
                net_communication_writer.send(NetworkCommunicationEvent(OutboundMessage { payload: dto }));
            }
        }
    }
}
```

## Tier 2: Individual Type Dispatcher

This tier further splits category enums into individual message types. This is optional but provides maximum type safety and granularity.

### Tier 2A: Command Response Dispatcher

```rust
use bevy::prelude::*;
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::commands::*;
use fanuc_rmi::dto;

// Define events for individual command types (internal)
#[derive(Event, Clone)]
pub struct JointAnglesEvent(pub FrcReadJointAnglesResponse);

#[derive(Event, Clone)]
pub struct StatusEvent(pub FrcGetStatusResponse);

#[derive(Event, Clone)]
pub struct TcpSpeedEvent(pub FrcReadTCPSpeedResponse);

#[derive(Event, Clone)]
pub struct CartesianPositionEvent(pub FrcReadCartesianPositionResponse);

// ... define events for all 21 command types

// Define network events for individual types
#[derive(Event, Clone)]
pub struct NetworkJointAnglesEvent(pub OutboundMessage<dto::FrcReadJointAnglesResponse>);

#[derive(Event, Clone)]
pub struct NetworkStatusEvent(pub OutboundMessage<dto::FrcGetStatusResponse>);

// ... define network events for all 21 command types

/// Tier 2: Dispatch CommandResponse to individual types
/// Sends to both internal (protocol) and network (DTO) buses
pub fn tier2_command_dispatcher(
    mut command_reader: EventReader<CommandResponseEvent>,

    // Internal event writers (protocol types) - showing subset for brevity
    mut joint_angles_writer: EventWriter<JointAnglesEvent>,
    mut status_writer: EventWriter<StatusEvent>,
    mut tcp_speed_writer: EventWriter<TcpSpeedEvent>,
    mut cartesian_pos_writer: EventWriter<CartesianPositionEvent>,
    // ... add writers for all 21 command types

    // Network event writers (DTO types) - showing subset
    mut net_joint_angles_writer: EventWriter<NetworkJointAnglesEvent>,
    mut net_status_writer: EventWriter<NetworkStatusEvent>,
    // ... add writers for all 21 command types
) {
    for event in command_reader.read() {
        let response = &event.0;
        match response {
            CommandResponse::FrcReadJointAngles(data) => {
                // Internal
                joint_angles_writer.send(JointAnglesEvent(data.clone()));
                // Network
                let dto: dto::FrcReadJointAnglesResponse = data.clone().into();
                net_joint_angles_writer.send(NetworkJointAnglesEvent(OutboundMessage { payload: dto }));
            }

            CommandResponse::FrcGetStatus(data) => {
                status_writer.send(StatusEvent(data.clone()));
                let dto: dto::FrcGetStatusResponse = data.clone().into();
                net_status_writer.send(NetworkStatusEvent(OutboundMessage { payload: dto }));
            }

            CommandResponse::FrcReadTCPSpeed(data) => {
                tcp_speed_writer.send(TcpSpeedEvent(data.clone()));
                // Network conversion...
            }

            CommandResponse::FrcReadCartesianPosition(data) => {
                cartesian_pos_writer.send(CartesianPositionEvent(data.clone()));
                // Network conversion...
            }

            // Add all 21 command response variants here...
            _ => {
                // Handle remaining variants or log unhandled types
                warn!("Unhandled command response type");
            }
        }
    }
}
```

### Tier 2B: Instruction Response Dispatcher

```rust
use bevy::prelude::*;
use bevy_eventwork::{MessageReader, MessageWriter};
use fanuc_rmi::packets::InstructionResponse;
use fanuc_rmi::instructions::*;
use fanuc_rmi::dto;

/// Tier 2: Dispatch InstructionResponse to individual types
pub fn tier2_instruction_dispatcher(
    mut instruction_reader: MessageReader<InstructionResponse>,

    // Internal message writers (protocol types) - showing subset
    mut move_joint_writer: MessageWriter<FrcMoveJointResponse>,
    mut move_linear_writer: MessageWriter<FrcMoveLinearResponse>,
    mut set_speed_writer: MessageWriter<FrcSetSpeedResponse>,
    // ... add writers for all 16 instruction types

    // Network message writers (DTO types) - showing subset
    mut net_move_joint_writer: MessageWriter<OutboundMessage<dto::FrcMoveJointResponse>>,
    mut net_move_linear_writer: MessageWriter<OutboundMessage<dto::FrcMoveLinearResponse>>,
    mut net_set_speed_writer: MessageWriter<OutboundMessage<dto::FrcSetSpeedResponse>>,
    // ... add writers for all 16 instruction types
) {
    for response in instruction_reader.read() {
        match response {
            InstructionResponse::FrcMoveJoint(data) => {
                move_joint_writer.write(data.clone());
                let dto: dto::FrcMoveJointResponse = data.into();
                net_move_joint_writer.write(OutboundMessage { payload: dto });
            }

            InstructionResponse::FrcMoveLinear(data) => {
                move_linear_writer.write(data.clone());
                let dto: dto::FrcMoveLinearResponse = data.into();
                net_move_linear_writer.write(OutboundMessage { payload: dto });
            }

            InstructionResponse::FrcSetSpeed(data) => {
                set_speed_writer.write(data.clone());
                let dto: dto::FrcSetSpeedResponse = data.into();
                net_set_speed_writer.write(OutboundMessage { payload: dto });
            }

            // Add all 16 instruction response variants here...
            _ => {
                warn!("Unhandled instruction response type");
            }
        }
    }
}
```

### Tier 2C: Communication Response Dispatcher

```rust
use bevy::prelude::*;
use bevy_eventwork::{MessageReader, MessageWriter};
use fanuc_rmi::packets::CommunicationResponse;
use fanuc_rmi::communication::*;
use fanuc_rmi::dto;

/// Tier 2: Dispatch CommunicationResponse to individual types
pub fn tier2_communication_dispatcher(
    mut communication_reader: MessageReader<CommunicationResponse>,

    // Internal message writers (protocol types)
    mut connect_writer: MessageWriter<ConnectResponse>,
    mut disconnect_writer: MessageWriter<DisconnectResponse>,
    mut heartbeat_writer: MessageWriter<HeartbeatResponse>,
    mut get_config_writer: MessageWriter<GetConfigurationResponse>,

    // Network message writers (DTO types)
    mut net_connect_writer: MessageWriter<OutboundMessage<dto::ConnectResponse>>,
    mut net_disconnect_writer: MessageWriter<OutboundMessage<dto::DisconnectResponse>>,
    mut net_heartbeat_writer: MessageWriter<OutboundMessage<dto::HeartbeatResponse>>,
    mut net_get_config_writer: MessageWriter<OutboundMessage<dto::GetConfigurationResponse>>,
) {
    for response in communication_reader.read() {
        match response {
            CommunicationResponse::Connect(data) => {
                connect_writer.write(data.clone());
                let dto: dto::ConnectResponse = data.into();
                net_connect_writer.write(OutboundMessage { payload: dto });
            }

            CommunicationResponse::Disconnect(data) => {
                disconnect_writer.write(data.clone());
                let dto: dto::DisconnectResponse = data.into();
                net_disconnect_writer.write(OutboundMessage { payload: dto });
            }

            CommunicationResponse::Heartbeat(data) => {
                heartbeat_writer.write(data.clone());
                let dto: dto::HeartbeatResponse = data.into();
                net_heartbeat_writer.write(OutboundMessage { payload: dto });
            }

            CommunicationResponse::GetConfiguration(data) => {
                get_config_writer.write(data.clone());
                let dto: dto::GetConfigurationResponse = data.into();
                net_get_config_writer.write(OutboundMessage { payload: dto });
            }
        }
    }
}
```

## Tier 3: Consumer Systems

Consumers can choose between pattern matching on enums (Tier 1) or subscribing to specific types (Tier 2).

### Consumer Approach A: Pattern Matching on Category Enums

Subscribe to category enums and pattern match for the types you care about:

```rust
use bevy::prelude::*;
use fanuc_rmi::packets::CommandResponse;

/// Example: Handle multiple command types with pattern matching
pub fn command_consumer_pattern_match(
    mut command_reader: EventReader<CommandResponseEvent>,
) {
    for event in command_reader.read() {
        let response = &event.0;
        match response {
            CommandResponse::FrcReadJointAngles(angles) => {
                if angles.error_id == 0 {
                    info!("Joint angles: J1={}, J2={}, J3={}",
                        angles.joint_angles.j1,
                        angles.joint_angles.j2,
                        angles.joint_angles.j3
                    );
                } else {
                    error!("Joint angles read error: {}", angles.error_id);
                }
            }

            CommandResponse::FrcGetStatus(status) => {
                info!("Robot status: servo_ready={}, tp_mode={}",
                    status.servo_ready,
                    status.tp_mode
                );
            }

            CommandResponse::FrcReadCartesianPosition(position) => {
                if position.error_id == 0 {
                    info!("Position: X={}, Y={}, Z={}",
                        position.position.x,
                        position.position.y,
                        position.position.z
                    );
                }
            }

            _ => {
                // Ignore other command types
            }
        }
    }
}
```

### Consumer Approach B: Direct Type Subscription

Subscribe to specific message types (requires Tier 2 dispatcher):

```rust
use bevy::prelude::*;
use fanuc_rmi::commands::{FrcReadJointAnglesResponse, FrcGetStatusResponse};

/// Example: Subscribe only to joint angles (no pattern matching needed!)
pub fn joint_angles_consumer(
    mut angles_reader: EventReader<JointAnglesEvent>,
) {
    for event in angles_reader.read() {
        let angles = &event.0;
        // angles is already the right type - no pattern matching!
        if angles.error_id == 0 {
            info!("Joint angles: J1={}, J2={}, J3={}, J4={}, J5={}, J6={}",
                angles.joint_angles.j1,
                angles.joint_angles.j2,
                angles.joint_angles.j3,
                angles.joint_angles.j4,
                angles.joint_angles.j5,
                angles.joint_angles.j6
            );
        } else {
            error!("Joint angles read error: {}", angles.error_id);
        }
    }
}

/// Example: Subscribe only to status (no pattern matching needed!)
pub fn status_consumer(
    mut status_reader: EventReader<StatusEvent>,
) {
    for event in status_reader.read() {
        let status = &event.0;
        // status is already the right type
        info!("Robot status:");
        info!("  Servo Ready: {}", status.servo_ready);
        info!("  TP Mode: {}", status.tp_mode);
        info!("  Remote Mode: {}", status.remote_mode);
        info!("  Cycle Running: {}", status.cycle_running);
    }
}
```

### Consumer Approach C: Generic Error Handler

Use `ExtractInner` to create generic handlers:

```rust
use bevy::prelude::*;
use bevy_eventwork::MessageReader;
use fanuc_rmi::ExtractInner;
use fanuc_rmi::packets::CommandResponse;
use fanuc_rmi::commands::FrcReadJointAnglesResponse;

/// Generic error checker using ExtractInner
pub fn error_checker_system(
    mut command_reader: MessageReader<CommandResponse>,
) {
    for response in command_reader.read() {
        // Check if it's a joint angles response
        if let Some(angles) = response.as_inner::<FrcReadJointAnglesResponse>() {
            if angles.error_id != 0 {
                error!("Joint angles error: {}", angles.error_id);
            }
        }

        // Could check other types here...
    }
}
```

### Network Consumer Example

Consume DTO messages from the network bus:

```rust
use bevy::prelude::*;
use bevy_eventwork::MessageReader;
use fanuc_rmi::dto;

/// Example: Network consumer that receives DTO messages
pub fn network_consumer(
    mut net_command_reader: MessageReader<OutboundMessage<dto::CommandResponse>>,
) {
    for message in net_command_reader.read() {
        let response = &message.payload;

        match response {
            dto::CommandResponse::FrcReadJointAngles(angles) => {
                // DTO type - same API as protocol type
                info!("Network: Joint angles J1={}", angles.joint_angles.j1);
            }

            dto::CommandResponse::FrcGetStatus(status) => {
                info!("Network: Status servo_ready={}", status.servo_ready);
            }

            _ => {}
        }
    }
}
```

## Complete Application Setup

### Registering All Systems

```rust
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)

        // Register Bevy events for internal message passing
        .add_event::<ResponsePacket>()
        .add_event::<CommandResponseEvent>()
        .add_event::<InstructionResponseEvent>()
        .add_event::<CommunicationResponseEvent>()

        // Register individual type events (if using Tier 2)
        .add_event::<JointAnglesEvent>()
        .add_event::<StatusEvent>()
        .add_event::<TcpSpeedEvent>()
        .add_event::<CartesianPositionEvent>()
        // ... register all individual type events you want to use

        // Register network events
        .add_event::<NetworkCommandEvent>()
        .add_event::<NetworkInstructionEvent>()
        .add_event::<NetworkCommunicationEvent>()
        .add_event::<NetworkJointAnglesEvent>()
        .add_event::<NetworkStatusEvent>()
        // ... register all network events

        // Add relay systems
        .add_systems(Update, (
            tier1_packet_relay,
            tier2_command_dispatcher,
            tier2_instruction_dispatcher,
            tier2_communication_dispatcher,
        ).chain())  // Chain to ensure order

        // Add consumer systems
        .add_systems(Update, (
            // Pattern matching consumers
            command_consumer_pattern_match,

            // Direct type consumers
            joint_angles_consumer,
            status_consumer,
        ))

        // Add network relay system (sends network events via eventwork)
        .add_systems(Update, network_relay_system)

        .run();
}

// System that actually sends network events via eventwork
fn network_relay_system(
    mut net_command_reader: EventReader<NetworkCommandEvent>,
    mut net_instruction_reader: EventReader<NetworkInstructionEvent>,
    // Add your eventwork network provider here
    // net: ResMut<NetworkResource>,
) {
    // Send network events via eventwork
    for event in net_command_reader.read() {
        // net.send_message(&event.0).unwrap();
        // Or broadcast, or send to specific connection, etc.
    }

    for event in net_instruction_reader.read() {
        // net.send_message(&event.0).unwrap();
    }
}
```

### Minimal Setup (Tier 1 Only)

If you don't need Tier 2 individual type dispatching:

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)

        // Register Bevy events
        .add_event::<ResponsePacket>()
        .add_event::<CommandResponseEvent>()
        .add_event::<InstructionResponseEvent>()
        .add_event::<CommunicationResponseEvent>()

        // Register network events
        .add_event::<NetworkCommandEvent>()
        .add_event::<NetworkInstructionEvent>()
        .add_event::<NetworkCommunicationEvent>()

        // Only Tier 1 relay
        .add_systems(Update, tier1_packet_relay)

        // Consumers use pattern matching
        .add_systems(Update, command_consumer_pattern_match)

        // Network relay (if needed)
        .add_systems(Update, network_relay_system)

        .run();
}
```

## Complete Working Example

Here's a complete minimal example that demonstrates the three-tier system:

```rust
use bevy::prelude::*;
use fanuc_rmi::packets::{ResponsePacket, CommandResponse};
use fanuc_rmi::commands::FrcReadJointAnglesResponse;
use fanuc_rmi::dto;
use serde::{Deserialize, Serialize};

// Message wrapper for network transport
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct OutboundMessage<T> {
    pub payload: T,
}

// Event definitions
#[derive(Event, Clone)]
pub struct CommandResponseEvent(pub CommandResponse);

#[derive(Event, Clone)]
pub struct JointAnglesEvent(pub FrcReadJointAnglesResponse);

#[derive(Event, Clone)]
pub struct NetworkCommandEvent(pub OutboundMessage<dto::CommandResponse>);

#[derive(Event, Clone)]
pub struct NetworkJointAnglesEvent(pub OutboundMessage<dto::FrcReadJointAnglesResponse>);

// Tier 1 relay
fn tier1_relay(
    mut response_reader: EventReader<ResponsePacket>,
    mut command_writer: EventWriter<CommandResponseEvent>,
    mut net_command_writer: EventWriter<NetworkCommandEvent>,
) {
    for packet in response_reader.read() {
        if let ResponsePacket::CommandResponse(resp) = packet {
            command_writer.send(CommandResponseEvent(resp.clone()));
            let dto: dto::CommandResponse = resp.clone().into();
            net_command_writer.send(NetworkCommandEvent(OutboundMessage { payload: dto }));
        }
    }
}

// Tier 2 relay (partial - just joint angles for example)
fn tier2_relay(
    mut command_reader: EventReader<CommandResponseEvent>,
    mut joint_angles_writer: EventWriter<JointAnglesEvent>,
    mut net_joint_angles_writer: EventWriter<NetworkJointAnglesEvent>,
) {
    for event in command_reader.read() {
        if let CommandResponse::FrcReadJointAngles(data) = &event.0 {
            joint_angles_writer.send(JointAnglesEvent(data.clone()));
            let dto: dto::FrcReadJointAnglesResponse = data.clone().into();
            net_joint_angles_writer.send(NetworkJointAnglesEvent(OutboundMessage { payload: dto }));
        }
    }
}

// Consumer: Direct type subscription (Tier 3)
fn joint_angles_consumer(
    mut angles_reader: EventReader<JointAnglesEvent>,
) {
    for event in angles_reader.read() {
        let angles = &event.0;
        info!("Joint angles: J1={}, J2={}, J3={}",
            angles.joint_angles.j1,
            angles.joint_angles.j2,
            angles.joint_angles.j3
        );
    }
}

// Network relay (sends network events via eventwork)
fn network_relay(
    mut net_command_reader: EventReader<NetworkCommandEvent>,
    mut net_joint_angles_reader: EventReader<NetworkJointAnglesEvent>,
    // Add your eventwork network provider here
    // net: ResMut<NetworkResource>,
) {
    for event in net_command_reader.read() {
        // Serialize and send via eventwork
        // let bytes = bincode::serialize(&event.0).unwrap();
        // net.send(bytes).unwrap();
        info!("Would send command response over network");
    }

    for event in net_joint_angles_reader.read() {
        // let bytes = bincode::serialize(&event.0).unwrap();
        // net.send(bytes).unwrap();
        info!("Would send joint angles over network");
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)

        // Register Bevy events
        .add_event::<ResponsePacket>()
        .add_event::<CommandResponseEvent>()
        .add_event::<JointAnglesEvent>()
        .add_event::<NetworkCommandEvent>()
        .add_event::<NetworkJointAnglesEvent>()

        // Add relay systems
        .add_systems(Update, (
            tier1_relay,
            tier2_relay,
        ).chain())

        // Add consumers
        .add_systems(Update, (
            joint_angles_consumer,
            network_relay,
        ))

        .run();
}
```

## Integrating Eventwork for Network Transport

The examples above show the relay architecture using Bevy events. To actually send messages over the network using eventwork, you would:

### 1. Add Eventwork Plugin

```rust
use eventwork::{EventworkPlugin, NetworkProvider, tcp::TcpProvider};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EventworkPlugin::<TcpProvider>::default())
        // ... rest of setup
}
```

### 2. Listen for Network Messages

```rust
use eventwork::{NetworkEvent, ConnectionId};

fn network_send_system(
    mut net_command_reader: EventReader<NetworkCommandEvent>,
    net: Res<NetworkProvider<TcpProvider>>,
) {
    for event in net_command_reader.read() {
        // Serialize the DTO
        let bytes = bincode::serialize(&event.0).unwrap();

        // Send to all connected clients
        for connection_id in net.connections() {
            net.send_message(connection_id, &bytes).unwrap();
        }
    }
}
```

### 3. Receive Network Messages

```rust
fn network_receive_system(
    mut network_events: EventReader<NetworkEvent>,
    mut response_writer: EventWriter<ResponsePacket>,
) {
    for event in network_events.read() {
        match event {
            NetworkEvent::Message(connection_id, data) => {
                // Deserialize from bincode
                if let Ok(dto_response) = bincode::deserialize::<dto::CommandResponse>(data) {
                    // Convert DTO back to protocol type
                    let protocol_response: CommandResponse = dto_response.into();

                    // Send into internal relay system
                    response_writer.send(ResponsePacket::CommandResponse(protocol_response));
                }
            }
            NetworkEvent::Connected(connection_id) => {
                info!("Client connected: {:?}", connection_id);
            }
            NetworkEvent::Disconnected(connection_id) => {
                info!("Client disconnected: {:?}", connection_id);
            }
            _ => {}
        }
    }
}
```

### 4. Complete Network Integration

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(EventworkPlugin::<TcpProvider>::default())

        // Register all events
        .add_event::<ResponsePacket>()
        .add_event::<CommandResponseEvent>()
        .add_event::<NetworkCommandEvent>()
        // ... etc

        // Add relay systems
        .add_systems(Update, (
            tier1_relay,
            tier2_relay,
        ).chain())

        // Add network systems
        .add_systems(Update, (
            network_receive_system,  // Receives from network, sends to internal relay
            network_send_system,     // Receives from internal relay, sends to network
        ))

        // Add consumers
        .add_systems(Update, joint_angles_consumer)

        .run();
}
```

This creates a complete bidirectional flow:
```
Network → network_receive_system → ResponsePacket → tier1_relay → CommandResponse → tier2_relay → JointAnglesEvent → consumer
                                                                                                                        ↓
Network ← network_send_system ← NetworkCommandEvent ← tier1_relay ← CommandResponse ← (internal processing)
```

## Performance Considerations

### Message Cloning

The relay systems clone messages when sending to multiple buses. This is necessary because:
- Internal bus needs protocol types
- Network bus needs DTO types
- Both need ownership of the data

**Optimization**: If you only need one bus (internal OR network), you can avoid cloning:

```rust
// Internal only (no cloning)
fn tier1_relay_internal_only(
    mut response_reader: MessageReader<ResponsePacket>,
    mut command_writer: MessageWriter<CommandResponse>,
) {
    for packet in response_reader.read() {
        if let ResponsePacket::CommandResponse(resp) = packet {
            command_writer.write(resp);  // No clone!
        }
    }
}

// Network only (no cloning)
fn tier1_relay_network_only(
    mut response_reader: MessageReader<ResponsePacket>,
    mut net_command_writer: MessageWriter<OutboundMessage<dto::CommandResponse>>,
) {
    for packet in response_reader.read() {
        if let ResponsePacket::CommandResponse(resp) = packet {
            let dto: dto::CommandResponse = resp.into();  // Consumes resp
            net_command_writer.write(OutboundMessage { payload: dto });
        }
    }
}
```

### System Ordering

Use `.chain()` to ensure relay systems run in order:

```rust
.add_systems(Update, (
    tier1_packet_relay,
    tier2_command_dispatcher,
    tier2_instruction_dispatcher,
).chain())
```

This ensures:
1. Tier 1 processes packets first
2. Tier 2 processes the results
3. Consumers see all messages in the same frame

### Selective Tier 2 Dispatching

You don't need to dispatch ALL types in Tier 2. Only dispatch the types you actually use:

```rust
fn tier2_selective_dispatcher(
    mut command_reader: MessageReader<CommandResponse>,

    // Only dispatch the types we care about
    mut joint_angles_writer: MessageWriter<FrcReadJointAnglesResponse>,
    mut status_writer: MessageWriter<FrcGetStatusResponse>,
) {
    for response in command_reader.read() {
        match response {
            CommandResponse::FrcReadJointAngles(data) => {
                joint_angles_writer.write(data);
            }
            CommandResponse::FrcGetStatus(data) => {
                status_writer.write(data);
            }
            _ => {
                // Other types stay in the enum - consumers can pattern match
            }
        }
    }
}
```

## Best Practices

### 1. Choose the Right Tier for Your Needs

| Application Size | Recommended Approach |
|-----------------|---------------------|
| Small (< 10 message types) | Tier 1 only, pattern matching |
| Medium (10-30 types) | Tier 1 + selective Tier 2 |
| Large (> 30 types) | Full three-tier |

### 2. Use Type Safety

Prefer direct type subscription over pattern matching when possible:

```rust
// ✅ Good: Type-safe, no pattern matching
fn consumer(mut reader: MessageReader<FrcReadJointAnglesResponse>) {
    for angles in reader.read() {
        // angles is guaranteed to be the right type
    }
}

// ❌ Less ideal: Pattern matching, can forget variants
fn consumer(mut reader: MessageReader<CommandResponse>) {
    for response in reader.read() {
        if let CommandResponse::FrcReadJointAngles(angles) = response {
            // Easy to forget to handle other variants
        }
    }
}
```

### 3. Separate Internal and Network Concerns

- **Internal systems**: Use protocol types, subscribe to specific types
- **Network systems**: Use DTO types, handle serialization at the boundary

```rust
// ✅ Good: Internal system uses protocol types
fn internal_system(mut reader: MessageReader<FrcReadJointAnglesResponse>) {
    // Process protocol type
}

// ✅ Good: Network system uses DTO types
fn network_system(mut reader: MessageReader<OutboundMessage<dto::FrcReadJointAnglesResponse>>) {
    // Serialize and send DTO type
}
```

### 4. Handle Errors Consistently

Create dedicated error handling systems:

```rust
fn error_handler(mut command_reader: MessageReader<CommandResponse>) {
    for response in command_reader.read() {
        match response {
            CommandResponse::FrcReadJointAngles(ref angles) if angles.error_id != 0 => {
                error!("Joint angles error: {}", angles.error_id);
            }
            CommandResponse::FrcGetStatus(ref status) if status.error_id != 0 => {
                error!("Status error: {}", status.error_id);
            }
            // ... check all types
            _ => {}
        }
    }
}
```

### 5. Use Logging Effectively

Add logging to relay systems for debugging:

```rust
fn tier1_relay_with_logging(
    mut response_reader: MessageReader<ResponsePacket>,
    mut command_writer: MessageWriter<CommandResponse>,
) {
    for packet in response_reader.read() {
        match packet {
            ResponsePacket::CommandResponse(resp) => {
                debug!("Tier 1: Routing command response");
                command_writer.write(resp);
            }
            ResponsePacket::InstructionResponse(resp) => {
                debug!("Tier 1: Routing instruction response");
                // ...
            }
            _ => {}
        }
    }
}
```

## Troubleshooting

### Messages Not Arriving at Consumers

**Problem**: Consumer system not receiving messages.

**Solutions**:
1. Verify message type is registered: `.add_plugins(EventworkPlugin::<YourType>::default())`
2. Check system ordering: Use `.chain()` to ensure relays run before consumers
3. Verify relay is writing to the correct message type
4. Add logging to relay systems to confirm messages are being sent

### Type Mismatch Errors

**Problem**: Compiler errors about type mismatches.

**Solutions**:
1. Ensure you're using protocol types internally and DTO types for network
2. Check that conversions are using `.into()` correctly
3. Verify feature flag: `features = ["DTO"]` in Cargo.toml

### Performance Issues

**Problem**: High CPU usage or latency.

**Solutions**:
1. Reduce cloning: Use single bus if possible
2. Use selective Tier 2 dispatching: Only dispatch types you need
3. Consider batching: Process multiple messages per frame
4. Profile with `cargo flamegraph` to identify bottlenecks

## Summary

The three-tier relay system provides:

✅ **Tier 1**: Category-level routing (Command, Instruction, Communication)
✅ **Tier 2**: Individual type routing (FrcReadJointAngles, FrcGetStatus, etc.)
✅ **Tier 3**: Flexible consumption (pattern matching OR direct subscription)
✅ **Dual buses**: Internal (protocol) and network (DTO) at every tier
✅ **Type safety**: Compiler-enforced correctness
✅ **Flexibility**: Choose the right granularity for your needs

## See Also

- [Protocol and DTO System](../architecture/protocol_dto_system.md) - Understanding the type system
- [Message Relay Patterns](../architecture/message_relay_patterns.md) - Architectural patterns
- [Basic Usage Examples](../examples/basic_usage.md) - Framework-agnostic examples


