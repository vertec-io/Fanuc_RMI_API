# Architecture & Scalability Critique

## 1. The "Web Server as Middleman" Bottleneck
The current architecture forces all traffic through a single `web_server` binary.
-   **Problem**: The `web_server` handles WebSocket connections, database I/O, *and* robot driver management in a single process.
-   **Impact**: If the SQLite database locks up (common with write-heavy loads) or a WebSocket client sends a flood of data, the robot control loop (which shares the same Tokio runtime) could jitter.
-   **Recommendation**:
    -   Split the `FanucDriver` into its own microservice (e.g., `rmi-service`) that exposes a gRPC or ZeroMQ interface.
    -   The `web_server` should merely be a UI gateway, not the process responsible for maintaining the sensitive TCP connection to the robot.

## 2. Brittle Binary Protocol
The project uses `bincode` for client-server communication.
-   **Problem**: `bincode` is **not versioned**. If the server adds a field to a DTO struct and the client (running WASM cached in a browser) is on the old version, deserialization will fail silently or catastrophically.
-   **Impact**: "It works on my machine" bugs where users have to clear their browser cache to fix protocol errors.
-   **Recommendation**: Use a schema-based format like **Protobuf** or **FlatBuffers** for the websocket protocol to ensure forward/backward compatibility.

## 3. Lack of Horizontal Scalability
-   **Problem**: The architecture assumes a single server instance managing multiple robots.
-   **Impact**: You cannot run multiple instances of `web_server` behind a load balancer because they rely on local in-memory state (`Arc<ClientManager>`) for control locking.
-   **Recommendation**: Move state to an external store (e.g., Redis) or use a consistent hashing approach if you want to scale to controlling hundreds of robots.

## 4. Database as Single Point of Failure
-   **Problem**: `driver.rs` and `web_server` seem to share the same execution context, but the database is embedded (SQLite).
-   **Impact**: SQLite is great for embedded, but if you deploy this effectively as a centralized server, file locking issues will plague performance.
