# Fanuc_RMI_API

Fanuc_RMI_API is a Rust-based library and example implementations designed to facilitate communication and control of FANUC robots via Remote Method Interface (RMI). This project is in early development and may change dramatically over time, but we hope to stabalize as we build.

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
cargo run --bin sim
```

This will launch a simulation server to emulate FANUC robot controller behavior.

---

## Contributing

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature-name`).
3. Commit changes (`git commit -m "Add feature"`).
4. Push to the branch (`git push origin feature-name`).
5. Open a pull request.
