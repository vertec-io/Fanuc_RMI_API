# Log Levels Usage Guide

This guide explains how to control logging verbosity in the FANUC RMI driver.

---

## Overview

The driver has a flexible logging system with 4 levels:

| Level | Terminal Output | Use Case |
|-------|----------------|----------|
| `Error` | Errors only | Production - only critical failures |
| `Warn` | Warnings + Errors | Production - performance issues + failures |
| `Info` | Info + Warn + Error | **Default** - important events |
| `Debug` | Everything | Development - every packet sent/received |

**Important**: All messages are **always** sent to the `log_channel` regardless of the log level. The log level only controls what gets printed to the terminal (when the `logging` feature is enabled).

---

## Quick Start

### Default Behavior (Info Level)

```rust
use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Default config uses Info level
    let config = FanucDriverConfig {
        addr: "192.168.1.100".to_string(),
        port: 18735,
        max_messages: 30,
        log_level: Default::default(), // Info level
    };

    let driver = FanucDriver::connect(config).await?;
    
    // You'll see:
    // - Connection events
    // - Initialization messages
    // - Errors and warnings
    // 
    // You WON'T see:
    // - Every packet sent/received (too verbose)
    
    Ok(())
}
```

---

## Setting Log Levels

### Error Level (Quietest)

Only show critical errors:

```rust
use fanuc_rmi::drivers::{FanucDriverConfig, LogLevel};

let config = FanucDriverConfig {
    addr: "192.168.1.100".to_string(),
    port: 18735,
    max_messages: 30,
    log_level: LogLevel::Error,
};

// Or use builder pattern
let config = FanucDriverConfig::new(
    "192.168.1.100".to_string(),
    18735,
    30
).with_log_level(LogLevel::Error);
```

**Terminal Output:**
```
[ERROR] Failed to serialize packet: ...
[ERROR] Read error: Connection reset by peer
```

---

### Warn Level

Show warnings and errors:

```rust
let config = FanucDriverConfig {
    addr: "192.168.1.100".to_string(),
    port: 18735,
    max_messages: 30,
    log_level: LogLevel::Warn,
};
```

**Terminal Output:**
```
[WARN] Send loop duration took 12ms exceeding max time: 8ms
[ERROR] Failed to send packet: ...
```

---

### Info Level (Default)

Show important events, warnings, and errors:

```rust
let config = FanucDriverConfig {
    addr: "192.168.1.100".to_string(),
    port: 18735,
    max_messages: 30,
    log_level: LogLevel::Info, // This is the default
};
```

**Terminal Output:**
```
[INFO] Initialized sequence counter to 42 from FRC_GetStatus
[INFO] Received disconnect packet
[INFO] Disconnecting from FRC server... closing send queue
[WARN] Send loop duration took 12ms exceeding max time: 8ms
[ERROR] Failed to send packet: ...
```

---

### Debug Level (Most Verbose)

Show **everything** including every packet sent and received:

```rust
let config = FanucDriverConfig {
    addr: "192.168.1.100".to_string(),
    port: 18735,
    max_messages: 30,
    log_level: LogLevel::Debug,
};
```

**Terminal Output:**
```
[DEBUG] Received: {"PacketType":"CommandResponse","Data":{"FrcGetStatus":{...}}}
[DEBUG] Sent response to backend: CommandResponse(FrcGetStatus(...))
[DEBUG] Received: {"PacketType":"InstructionResponse","Data":{...}}
[DEBUG] Sent response to backend: InstructionResponse(...)
[INFO] Initialized sequence counter to 42 from FRC_GetStatus
[WARN] Send loop duration took 12ms exceeding max time: 8ms
[ERROR] Failed to send packet: ...
```

⚠️ **Warning**: Debug level generates **massive** amounts of output in high-frequency applications (e.g., polling at 125Hz). Only use for debugging specific issues.

---

## Subscribing to Log Channel

All messages are sent to the log channel regardless of terminal log level:

```rust
use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig, LogLevel};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = FanucDriverConfig {
        addr: "192.168.1.100".to_string(),
        port: 18735,
        max_messages: 30,
        log_level: LogLevel::Error, // Quiet terminal
    };

    let driver = FanucDriver::connect(config).await?;
    
    // Subscribe to log channel
    let mut log_rx = driver.log_channel.subscribe();
    
    // Spawn task to handle logs
    tokio::spawn(async move {
        while let Ok(message) = log_rx.recv().await {
            // Custom log handling - send to UI, file, database, etc.
            println!("Custom log: {}", message);
        }
    });
    
    // Terminal is quiet (Error level), but log channel receives everything
    
    Ok(())
}
```

---

## Disabling Terminal Logging Completely

Compile without the `logging` feature:

```toml
[dependencies]
fanuc_rmi = { version = "0.4", features = ["driver"] }
# Note: "logging" feature NOT included
```

With this configuration:
- ✅ Log channel still works (all messages sent)
- ❌ Nothing printed to terminal (no `println!` calls)

---

## Best Practices

### Production Applications

```rust
// Quiet terminal, custom log handling
let config = FanucDriverConfig::new(addr, port, max_messages)
    .with_log_level(LogLevel::Error);

let driver = FanucDriver::connect(config).await?;
let mut log_rx = driver.log_channel.subscribe();

// Send logs to your application's logging system
tokio::spawn(async move {
    while let Ok(msg) = log_rx.recv().await {
        // Parse log level from message prefix
        if msg.starts_with("[ERROR]") {
            error!("{}", msg);
        } else if msg.starts_with("[WARN]") {
            warn!("{}", msg);
        } else if msg.starts_with("[INFO]") {
            info!("{}", msg);
        } else {
            debug!("{}", msg);
        }
    }
});
```

### Development/Debugging

```rust
// Verbose terminal output for debugging
let config = FanucDriverConfig::new(addr, port, max_messages)
    .with_log_level(LogLevel::Debug);

let driver = FanucDriver::connect(config).await?;
// See every packet in terminal
```

### Testing

```rust
// Info level for test output
let config = FanucDriverConfig::new(addr, port, max_messages)
    .with_log_level(LogLevel::Info);

let driver = FanucDriver::connect(config).await?;
// See important events without flooding test output
```

---

## Migration from v0.4.0

If you're upgrading from v0.4.0, no changes required! The default behavior is unchanged (Info level).

To reduce verbosity:

```rust
// Before (v0.4.0) - saw every packet
let config = FanucDriverConfig { ... };

// After (v0.4.1+) - only see important events
let config = FanucDriverConfig {
    log_level: LogLevel::Info, // or Error/Warn for even less
    ..config
};
```


