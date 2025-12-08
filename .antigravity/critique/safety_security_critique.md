# Safety & Security Critique

## 1. The "Open Door" Policy (No Auth)
-   **Critical Severity**: There is **Zero Authentication**.
-   **Problem**: The WebSocket server accepts connections from anyone.
-   **Impact**: Any malicious actor on the local network can connect to `ws://server:9000`, send a `RequestControl` packet, and start jogging the 2000kg industrial robot.
-   **Recommendation**: Implement JWT (JSON Web Token) authentication immediately. The Robot Control capability must be scoped to specific users.

## 2. Voluntary Control Locking
-   **High Severity**: The "Control Lock" is enforced by the **server** but requested by the **client UI**.
-   **Problem**: While the server checks `has_control`, the mechanism is purely software-based in the web server. If the web server restarts, does it fail-safe?
-   **Impact**: If the web server crashes while a robot is moving, there is no "Dead Man's Switch".
-   **Recommendation**: Implement a **Heartbeat/Watchdog** at the *Driver* level. If the driver hasn't received a "KeepAlive" packet from the Web Server (and by extension the user) for 200ms, it should automatically send `FRC_Abort` to the robot.

## 3. Lack of TLS
-   **Medium Severity**: The server runs on `http://` and `ws://`.
-   **Problem**: Passwords (if added) and robot commands are sent in cleartext.
-   **Impact**: Vulnerable to Man-in-the-Middle attacks. An attacker could inject packets to override the stop button.
-   **Recommendation**: Enforce HTTPS/WSS.

## 4. No E-Stop Integration
-   **Safety Critical**: Software cannot replace a hardware E-Stop, but the software should be aware of it.
-   **Problem**: There is no dedicated handling for external E-Stop signals in the API.
-   **Impact**: The UI might show "Running" while the robot is actually hardware-halted, confusing the operator.
