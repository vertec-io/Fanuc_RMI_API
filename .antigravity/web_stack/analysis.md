# Web Stack Analysis

## Overview
The web stack enables remote control and monitoring of the robot through a modern, browser-based interface. It consists of a backend (`web_server`) acting as a bridge and a frontend (`web_app`) providing the UI.

## Web Server (`web_server`)
The server is a Rust binary that orchestrates the system.

### Architecture
-   **WebSocket Relay**: It creates a 1:1 relay between WebSocket clients and the `FanucDriver`.
    -   **Incoming (Client -> Robot)**: Deserializes Bincode DTOs -> Converts to Packets -> Sends to Driver.
    -   **Outgoing (Robot -> Client)**: Subscribes to Driver responses -> Converts to DTOs -> Serializes to Bincode -> Broadcasts to *all* clients.
-   **State Management**:
    -   `RobotConnection`: A thread-safe structure (`RwLock`) holding the active driver and configuration.
    -   `ClientManager`: Implements a "Control Lock" system. Only one client can acquire control (Write lock), preventing race conditions between multiple operators.

### Persistence
-   **SQLite**: Uses `rusqlite` to store data in `data/fanuc_rmi.db`.
-   **Schema**:
    -   `programs`: Toolpath programs with metadata.
    -   `program_instructions`: The actual points/commands.
    -   `robot_connections`: Saved IP/Port configurations for different robots.
    -   `robot_settings`: Default values (speed, frames) for each robot.
-   **Migrations**: Includes a migration system in `database.rs` to automatically upgrade the schema when the app updates.

## Web Application (`web_app`)
The frontend is built with **Leptos**, a Rust web framework that compiles to WebAssembly (WASM).

### Key Features
-   **State Sync**: `websocket.rs` maintains a local mirror of the robot state (Using Signals). It listens for binary broadcasts and updates the UI instantly (60fps+).
-   **Components**:
    -   **Jog Controls**: Detailed controls for Cartesian and Joint jogging.
    -   **Program Visualizer**: Shows the currently running line.
    -   **3D View** (implied): References to updating 3D state suggest a 3D visualizer is present or planned.
-   **Performance**: By using Binary WebSockets (Bincode) instead of JSON for the high-frequency robot state, the app minimizes serialization overhead and bandwidth.

## Security & Safety
-   **Control Logic**: The server strictly enforces "Control". If a client without control sends a motion command, the server rejects it.
-   **Timeout**: A background task checks for inactivity and automatically revokes control if a user steps away (safety feature).
