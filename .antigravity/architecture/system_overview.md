# System Architecture Overview

## High-Level Design

The `Fanuc_RMI_API` system is designed as a layered architecture that acts as a bridge between modern web clients and industrial FANUC robots. It transforms high-level user intentions (via a web interface) into low-level RMI (Remote Motion Interface) protocol packets.

### Core Components

1.  **Web Application (`web_app`)**:
    -   **Tech Stack**: Leptos (Rust WASM framework), TailwindCSS.
    -   **Role**: The user interface. It renders the state of the robot, allows jogging, and manages programs.
    -   **Communication**: Connects to `web_server` via WebSockets. Sends JSON for management tasks and binary (bincode) for real-time robot control.

2.  **Web Server (`web_server`)**:
    -   **Tech Stack**: Rust, Tokio (async runtime), Tungstenite (WebSockets), SQLite (via SQLx or similar).
    -   **Role**: The central coordinator.
        -   **Bridge**: Translates WebSocket messages into `FanucDriver` calls.
        -   **State Authority**: Maintains the "source of truth" for robot connection status, active configuration (UFrames, UTools), and who has "control".
        -   **Broadcaster**: Multiplexes robot status updates (position, errors) to all connected clients.
        -   **Persistence**: Stores robot profiles and programs in a SQLite database.

3.  **Core Library (`fanuc_rmi`)**:
    -   **Tech Stack**: Pure Rust.
    -   **Role**: The protocol implementation.
        -   **Driver**: Manages the TCP connection to the physical robot. Uses a priority queue to interleave high-priority status checks with standard motion commands.
        -   **Protocol**: Defines the binary structure of RMI packets.
        -   **DTOs**: Provides serializable data structures that mirror the protocol packets, used for network transmission between the web server and web app.

4.  **Simulator (`sim`)**:
    -   **Role**: A software mock of a FANUC robot. It listens on the RMI port (default 18735) and responds to commands, simulating physics/kinematics for testing without hardware.

## Data Flow & Communication

### 1. Robot Control Loop (Real-Time)
This flow handles jogging and motion commands.
1.  **User**: Clicks "Jog X+" in the Web App.
2.  **Web App**: Serializes a `SendPacket` DTO into binary (using `bincode`) and sends it over WebSocket.
3.  **Web Server**:
    -   Receives binary message.
    -   Checks if the client has "Control" (Mutex lock).
    -   Deserializes the DTO into a `fanuc_rmi` packet.
    -   Calls `driver.send_packet(packet)`.
4.  **Fanuc Driver**:
    -   Queues the packet.
    -   Sends it over TCP to the Robot.
    -   Waits for response (async).

### 2. Status Monitoring (Broadcast)
This flow keeps the UI in sync with the robot.
1.  **Web Server (Background Task)**:
    -   Every 100ms, queues `FrcReadCartesianPosition`, `FrcReadJointAngles`, and `FrcGetStatus` commands (High Priority).
2.  **Fanuc Driver**:
    -   Sends commands to Robot.
    -   Receives responses via TCP.
    -   Publishes responses to a broadast channel (`response_tx`).
3.  **Web Server (Broadcast Task)**:
    -   Subscribes to `response_tx`.
    -   Converts responses to DTOs.
    -   Serializes to binary.
    -   Broadcasts to **ALL** connected WebSocket clients.
4.  **Web App**:
    -   Receives binary.
    -   Updates UI (3D view, position readout).

### 3. Management & API
This flow handles non-real-time tasks like saving programs or changing settings.
1.  **User**: Clicks "Save Program".
2.  **Web App**: Sends a JSON message (`ClientRequest`).
3.  **Web Server**:
    -   Parses JSON.
    -    Executes logic (e.g., writes to SQLite).
    -   Sends JSON response (`ServerResponse`).

## Concurrency Model

The system relies heavily on Rust's async/await model (Tokio) to handle high-concurrency without blocking the control loop.

-   **Driver Isolation**: The `FanucDriver` runs in its own task, managing the TCP stream. It communicates with the rest of the application via `mpsc` (Multi-Producer, Single-Consumer) channels for sending and `broadcast` channels for receiving.
-   **Locking**:
    -   `RobotConnection` is protected by `RwLock` because it's read frequently (for status polling) and written rarely (connection/disconnection).
    -   `Database` and `ProgramExecutor` are protected by `Mutex`.
-   **Safety**: The "Control" mechanism in `ClientManager` ensures that **only one user** can command the robot at a time, preventing dangerous race conditions where two operators might fight for control.
