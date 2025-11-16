# Fanuc_RMI_API

Fanuc_RMI_API is a Rust-based library and example implementations designed to facilitate communication and control of FANUC robots via Remote Motion Interface (RMI). This project is in early development and may change dramatically over time, but we hope to stabalize as we build.

---

## Features

- **Robot Communication**: Establish TCP communication with FANUC robot controllers.
- **Command Execution**: Supports various commands for robot operation such as movement, configuration, and control.
- **Custom Instructions**: Create and execute custom instructions tailored to your use case.
- **Simulation**: Includes a simulation module for testing without hardware.
- **Error Handling**: Comprehensive error reporting and handling mechanisms.

---

## Modules

1. **Commands**: Implements robot commands like `initialize`, `read_error`, `abort`, and more.
2. **Communication**: Handles TCP communication and packet exchange with the robot controller.
3. **Drivers**: Provides configuration and management of robot drivers.
4. **Instructions**: Contains motion-related instructions such as linear and joint motions.
5. **Packets**: Defines the structure for communication and instruction packets.
6. **Errors**: Comprehensive error types and utilities for debugging and recovery.
7. **Web App**: Modern web-based control interface with real-time monitoring (Leptos + WebSocket).
8. **Web Server**: WebSocket server bridging FANUC driver and web clients.

---

## Getting Started

### Prerequisites

- Rust and Cargo installed.
- Access to a FANUC robot with RMI support (alternatively see below for our very basic simulation RMI simulation server).

### Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/vertec-io/Fanuc_RMI_API.git
   cd Fanuc_RMI_API
   ```

2. Build the project:

   ```bash
   cargo build
   ```

---

## Usage

### Example

An example implementation to connect and execute basic commands:

```rust
use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig},
    FrcError,
};

#[tokio::main]
async fn main() -> Result<(), FrcError> {
    let config = FanucDriverConfig {
        addr: "127.0.0.1".to_string(),
        port: 16001,
        max_messages: 30,
    };

    let driver = FanucDriver::connect(config).await?;
    driver.initialize().await?;
    driver.abort().await?;
    driver.disconnect().await?;
    Ok(())
}
```

Run this example with:
```bash
cargo run --example main
```

---

## Simulation

For testing and development without access to hardware, use the simulation module:

```bash
cargo run --bin sim -- --realtime
```

This will launch a simulation server to emulate FANUC robot controller behavior.

---

## Web Application

A modern web-based control interface for FANUC robots with real-time monitoring and jog controls.

### Features
- **Real-time Position Monitoring**: Live X, Y, Z coordinates via WebSocket
- **Jog Controls**: Interactive buttons for precise robot movement
- **Robot Status**: Servo ready, TP mode, motion status indicators
- **Motion Log**: Real-time history of completed movements
- **Clean UI**: Minimalistic dark mode design inspired by industrial control software

### Quick Start

1. **Start the simulator**:
```bash
cargo run -p sim -- --realtime
```

2. **Start the WebSocket server**:
```bash
cargo run -p web_server
```

3. **Build and serve the web app**:
```bash
# Build WASM
cargo build --target wasm32-unknown-unknown --release -p web_app
wasm-bindgen --target web --out-dir web_app/pkg --no-typescript target/wasm32-unknown-unknown/release/web_app.wasm

# Serve
cd web_app && python3 -m http.server 8000
```

4. **Open in browser**: `http://localhost:8000/`

For detailed documentation, see:
- [Web App README](web_app/README.md)
- [Web Server README](web_server/README.md)

---

## Contributing

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature-name`).
3. Commit changes (`git commit -m "Add feature"`).
4. Push to the branch (`git push origin feature-name`).
5. Open a pull request.
