# Core Library Analysis (`fanuc_rmi`)

## Overview
The `fanuc_rmi` crate is the foundation of the system, implementing the FANUC Remote Motion Interface (RMI) protocol (spec B-84184EN_02). It provides a type-safe, asynchronous (Tokio-based) API for communicating with FANUC robots.

## Key Components

### 1. The Driver (`FanucDriver`)
Located in `drivers/driver.rs`, the driver is the central nervous system of the library.

**Key Features:**
-   **Async Runtime**: Built on `tokio`, allowing non-blocking I/O. This is essential for maintaining responsive control loops while waiting for network packets.
-   **Split Architecture**: Uses `tokio::io::split` to separate the TCP stream into Read and Write halves, managed by independent tasks. This allows full-duplex communication (sending commands while simultaneously receiving status updates).
-   **Priority Queue**: Implements `PacketPriority` (Low, Standard, High, Immediate, Termination).
    -   *High Priority* is used for status polling (`FrcGetStatus`), ensuring it doesn't get stuck behind a long queue of motion commands.
    -   *Standard Priority* is used for motion commands.
-   **Sequence Management**:
    -   Automatically assigns sequence IDs (`sequence_id`) immediately before sending.
    -   Broadcasts `sent_instruction_tx` to allow callers to correlate their request with the specific sequence ID assigned by the driver.
-   **Backpressure & Safety**:
    -   **Max In-Flight**: Limits outstanding instructions to 8 (matching FANUC's internal buffer).
    -   **Instruction Delay**: Enforces a 2ms delay between instructions to prevent TCP packet packing, which causes "RMI Command Failed" errors on the robot controller.

### 2. Protocol Implementation
The library uses strong typing for all RMI packets.

-   **Enums**: `Command`, `Instruction`, `Communication` enums ensure that only valid packets can be constructed.
-   **Serialization**: Uses `serde` with `serde_json` for wire format, but strictly validates the structure against FANUC's idiosyncratic JSON format.

### 3. DTO System & Macros
To enable the web architecture (where the robot driver runs on the server but logic runs in the browser), the library provides Data Transfer Objects (DTOs).

-   **`fanuc_rmi_macros`**: A procedural macro crate that automatically generates DTO structs from the main packet structs.
-   **Purpose**: These DTOs (`fanuc_rmi::dto`) are mirror images of the packets but derive `Serialize`/`Deserialize` effectively for `bincode`. This allows efficient binary transmission over WebSockets.

### 4. Error Handling
-   **Result Types**: Every operation returns a `Result`.
-   **Recovery**: The driver includes specific logic (`recover_from_hold_state`) to handle the dreaded "Invalid Sequence ID" error, which requires a specific "Reset -> GetStatus -> Sync" dance.
-   **Broadcasting**: Protocol errors are broadcast to a dedicated channel, allowing the UI to show real-time error toasts.

## Engineering Quality
The implementation shows a high degree of engineering maturity:
-   **Thread Safety**: Extensive use of `Arc<Mutex<...>>` and `Arc<RwLock<...>>` where appropriate, with careful scope management to avoid deadlocks.
-   **Resilience**: The connection logic includes retry mechanisms and smart initialization (checking status before aborting/initializing).
