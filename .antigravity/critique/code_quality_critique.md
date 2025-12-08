# Code Quality & Maintainability Critique

## 1. The `FanucDriver` God Object
-   **Problem**: `fanuc_rmi/src/drivers/driver.rs` is **1432 lines long** and growing.
-   **Smell**: It handles connection logic, queue management, packet serialization, thread spawning, and error recovery all in one struct.
-   **Impact**: Extremely hard to unit test. Testing the queue logic requires mocking a TCP stream, which is painful.
-   **Recommendation**: Refactor into smaller components:
    -   `TransportLayer`: Handles the raw TCP read/write.
    -   `PacketQueue`: Handles the priority logic (pure struct, easy to test).
    -   `SequenceManager`: Handles ID assignment.

## 2. Dangerous Error Handling (`unwrap`)
-   **Problem**: `web_server/src/main.rs` contains `unwrap()` on critical startup paths:
    ```rust
    // Line 535
    let ws_listener = tokio::net::TcpListener::bind(&websocket_addr).await.unwrap();
    ```
-   **Impact**: If port 9000 is occupied, the application **crashes immediately** with a panic instead of printing a friendly error log and exiting with a non-zero code. This is unacceptable for production software.
-   **Recommendation**: Use `anyhow` or `thiserror` for top-level error integration and print structured logs.

## 3. Extract Trait Panics
-   **Problem**: `fanuc_rmi/src/extract.rs` implements `expect_inner` which panics:
    ```rust
    fn expect_inner(&self, msg: &str) -> &T {
        self.as_inner().expect(msg)
    }
    ```
-   **Impact**: Library code should **never** panic. It should return `Result`. If a user of this library calls `expect_inner` on the wrong packet type, the entire control process crashes, potentially leaving a robot moving.

## 4. Forgotten TODOs
-   **Problem**: `driver.rs:169`: `//FIXME: there isnt a system on meteorite monitoring number of packets sent`.
-   **Impact**: Indicates abandoned technical debt. "Meteorite" seems to be a legacy project name, implying code was copy-pasted.

## 5. Duplicate logic in `packets`
-   **Problem**: The `packets` directory contains many files that look nearly identical.
-   **Impact**: High code duplication makes it hard to change the protocol structure (e.g., adding a timestamp to all packets).
