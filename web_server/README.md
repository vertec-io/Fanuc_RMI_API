# FANUC RMI WebSocket Server

A WebSocket server that bridges the FANUC RMI driver and web clients, enabling real-time robot control and monitoring through a web browser.

## Overview

This server acts as a middleware layer between:
- **FANUC Robot** (via RMI protocol on TCP)
- **Web Clients** (via WebSocket with binary encoding)

It handles bidirectional communication, converting between the FANUC RMI protocol and WebSocket messages using efficient binary serialization (bincode).

## Features

- **WebSocket Server**: Listens on port 9000 for web client connections
- **FANUC Driver Integration**: Connects to robot controller on port 16001
- **Binary Protocol**: Uses bincode for efficient message serialization
- **Broadcast Support**: Sends robot updates to all connected clients
- **Periodic Polling**: Automatically requests position and status every 100ms
- **DTO Conversion**: Converts between DTO types (for web) and protocol types (for driver)

## Architecture

```
┌─────────────┐         ┌──────────────┐         ┌─────────────┐
│   Web App   │◄───────►│  WebSocket   │◄───────►│   FANUC     │
│  (Browser)  │  WS:9000│    Server    │ TCP:16001│   Robot     │
└─────────────┘         └──────────────┘         └─────────────┘
     Binary                  Bincode                  RMI
    Messages                Conversion              Protocol
```

## Message Flow

### Outbound (Robot → Web)
1. Robot sends response via RMI protocol
2. Server receives protocol packet
3. Server converts to DTO packet using `.into()`
4. Server serializes with bincode
5. Server broadcasts binary message to all WebSocket clients

### Inbound (Web → Robot)
1. Web client sends binary message via WebSocket
2. Server deserializes with bincode to DTO packet
3. Server converts to protocol packet using `.into()`
4. Server sends to robot via FANUC driver

## Configuration

### Robot Connection
- **Host**: `127.0.0.1` (localhost for simulator, change for real robot)
- **Port**: `16001` (standard RMI port)

### WebSocket Server
- **Host**: `127.0.0.1`
- **Port**: `9000`

### Polling Interval
- **Position**: Every 100ms
- **Status**: Every 100ms

## Running

### Prerequisites
- FANUC robot or simulator running on port 16001

### Start Server
```bash
cargo run -p web_server
```

### Expected Output
```
Connecting to robot at 127.0.0.1:16001
✓ Connected to robot
🚀 WebSocket server listening on ws://127.0.0.1:9000
```

## Message Types

### Commands (Web → Robot)
- `FrcReadCartesianPosition` - Request current position
- `FrcGetStatus` - Request robot status
- `FrcLinearRelative` - Execute relative linear movement
- And all other RMI commands...

### Responses (Robot → Web)
- `CommandResponse::FrcReadCartesianPosition` - Position data
- `CommandResponse::FrcGetStatus` - Status data
- `InstructionResponse::FrcLinearRelative` - Movement completion
- And all other RMI responses...

## Error Handling

The server includes comprehensive error handling:
- Connection failures are logged
- Failed message sends are logged but don't crash the server
- Broadcast channel errors are handled gracefully
- WebSocket errors are logged per-client

## Development

### Dependencies
- `tokio` - Async runtime
- `tokio-tungstenite` - WebSocket implementation
- `futures-util` - Stream utilities
- `bincode` - Binary serialization
- `fanuc_rmi` - FANUC driver and DTO types
- `tracing` - Logging

### Code Structure
- `main.rs` - Server initialization and message routing
- Uses `fanuc_rmi::drivers::FanucDriver` for robot communication
- Uses `tokio::sync::broadcast` for client broadcasting

## Troubleshooting

### "Connection refused" on port 16001
- Ensure the robot or simulator is running
- Check firewall settings
- Verify the IP address is correct

### "Address already in use" on port 9000
- Another instance may be running
- Kill the process: `lsof -ti:9000 | xargs kill -9`
- Or change the port in the code

### WebSocket clients not receiving updates
- Check browser console for connection errors
- Verify WebSocket URL is `ws://127.0.0.1:9000`
- Ensure binary message handling is implemented

## Version History

### v0.2.0 (Current)
- Updated documentation
- Improved error handling
- Added comprehensive logging

### v0.1.0
- Initial release
- Basic WebSocket server
- FANUC driver integration
- Binary message protocol

## License

See the main repository LICENSE file.

