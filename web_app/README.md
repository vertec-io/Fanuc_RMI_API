# FANUC RMI Web Application

A modern, minimalistic web-based control interface for FANUC robots using the RMI (Remote Motion Interface) protocol. Built with Leptos (Rust WASM framework) and featuring real-time WebSocket communication with binary encoding.

## Features

- **Real-time Monitoring**: Live position updates and robot status via WebSocket
- **Jog Controls**: Interactive buttons for precise robot movement (X, Y, Z axes)
- **Binary Protocol**: Efficient bincode serialization for low-latency communication
- **Modern UI**: Clean, minimalistic dark mode design inspired by industrial control software
- **Responsive Layout**: Adapts to different screen sizes
- **Motion Logging**: Real-time log of completed movements
- **Error Tracking**: Dedicated error log panel

## Architecture

The web application consists of two components:

### 1. Web App (Frontend)
- **Framework**: Leptos 0.6.15 (Rust → WebAssembly)
- **Styling**: Tailwind CSS with custom dark theme
- **Communication**: WebSocket client with binary message handling
- **Location**: `web_app/`

### 2. Web Server (Backend)
- **Framework**: Tokio + tokio-tungstenite
- **Purpose**: WebSocket server bridging FANUC driver and web clients
- **Port**: 9000 (WebSocket)
- **Location**: `web_server/`

## Design Philosophy

The UI follows a clean, minimalistic aesthetic:

- **Dark Mode**: Deep blacks (#0a0a0a) with subtle grays
- **Accent Color**: Cyan (#00d9ff) for primary actions and highlights
- **Typography**: Inter font family for modern, readable text
- **Spacing**: Moderate padding and consistent gaps
- **Borders**: Subtle 1px borders with low opacity
- **No Clutter**: Removed heavy gradients, glows, and excessive animations

Inspired by shadcn/ui and sci-fi command centers (Star Wars/Star Trek), the design is professional and suitable for industrial/technical software.

## Building

### Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- `wasm-bindgen-cli` installed

```bash
# Install WASM target
rustup target add wasm32-unknown-unknown

# Install wasm-bindgen-cli
cargo install wasm-bindgen-cli
```

### Build Steps

1. **Build the WASM app**:
```bash
cargo build --target wasm32-unknown-unknown --release -p web_app
```

2. **Generate JavaScript bindings**:
```bash
wasm-bindgen --target web --out-dir web_app/pkg --no-typescript target/wasm32-unknown-unknown/release/web_app.wasm
```

Or use the provided build script:
```bash
cd web_app && ./build.sh
```

## Running

### 1. Start the Robot Simulator
```bash
cargo run -p sim -- --realtime
```

### 2. Start the WebSocket Server
```bash
cargo run -p web_server
```

### 3. Serve the Web App
```bash
cd web_app && python3 -m http.server 8000
```

### 4. Open in Browser
Navigate to: `http://localhost:8000/`

## Usage

### Jog Controls
- **X+/X-**: Move along X axis
- **Y+/Y-**: Move along Y axis  
- **Z+/Z-**: Move along Z axis
- **Speed**: Adjust movement speed (mm/s)
- **Step**: Set step distance (mm)

### Monitoring
- **Position**: Real-time X, Y, Z coordinates
- **Robot Status**: Servo ready, TP mode, motion status
- **Motion Log**: History of completed movements
- **Errors**: Any errors encountered during operation

## Components

- `src/lib.rs` - Main app component and header
- `src/websocket.rs` - WebSocket client manager
- `src/components/robot_status.rs` - Robot status display
- `src/components/position_display.rs` - Position coordinates
- `src/components/jog_controls.rs` - Movement control buttons
- `src/components/error_log.rs` - Error message display
- `src/components/motion_log.rs` - Motion event history

## Version History

### v0.2.0 (Current)
- Redesigned UI with clean, minimalistic dark mode aesthetic
- Improved typography and spacing
- Removed heavy visual effects (gradients, glows, backdrop blur)
- Added Inter font family
- Custom scrollbar styling
- Professional color scheme with cyan accent

### v0.1.0
- Initial release
- Basic WebSocket communication
- Jog controls and monitoring
- Real-time position updates
- Binary encoding with bincode

## License

See the main repository LICENSE file.

