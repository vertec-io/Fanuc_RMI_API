use serde::{Deserialize, Serialize};
use std::net::ToSocketAddrs;

/// Log level for filtering driver messages
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Only errors (connection failures, serialization errors, etc.)
    Error = 0,
    /// Warnings and errors (timeouts, retries, etc.)
    Warn = 1,
    /// Important info, warnings, and errors (connection events, initialization, etc.)
    Info = 2,
    /// All messages including debug (every packet sent/received)
    Debug = 3,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

/// ```rust,ignore
/// // Create a new configuration with a DNS name or IP address
/// let config = FanucDriverConfig::new("example.com".to_string(), 16001, 30);
/// let config = FanucDriverConfig::new("127.0.0.1".to_string(), 16001, 30);
///
/// // Validate the configuration
/// if let Err(e) = config.validate() {
///     println!("Configuration error: {}", e);
///     return;
/// }
///
/// // Resolve the address to a `SocketAddr`
/// match config.resolve() {
///     Ok(resolved_address) => {
///         println!("Resolved address: {}", resolved_address);
///         // Now you can use the resolved address to establish a network connection
///         // For example: open a TCP or UDP socket connection
///     }
///     Err(e) => {
///         println!("Failed to resolve address: {}", e);
///     }
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FanucDriverConfig {
    pub addr: String,
    pub port: u32,
    pub max_messages: usize,
    /// Log level for terminal output (when "logging" feature is enabled)
    ///
    /// - `Error`: Only critical errors (connection failures, serialization errors)
    /// - `Warn`: Warnings and errors (timeouts, performance issues)
    /// - `Info`: Important events, warnings, and errors (default - connection, initialization)
    /// - `Debug`: All messages including every packet sent/received (very verbose)
    ///
    /// Note: All messages are always sent to the log_channel regardless of this setting.
    /// This only controls what gets printed to the terminal.
    #[serde(default)]
    pub log_level: LogLevel,
}

impl FanucDriverConfig {
    pub fn new(addr: String, port: u32, max_messages: usize) -> Self {
        Self {
            addr,
            port,
            max_messages,
            log_level: LogLevel::default(),
        }
    }

    pub fn with_log_level(mut self, log_level: LogLevel) -> Self {
        self.log_level = log_level;
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.addr.is_empty() {
            return Err("Address cannot be empty.".to_string());
        }
        if self.port == 0 {
            return Err("Port number must be greater than 0.".to_string());
        }
        if self.max_messages == 0 {
            return Err("Maximum messages must be greater than 0.".to_string());
        }
        Ok(())
    }

    /// Generates a connection URL from the address and port.
    pub fn connection_url(&self) -> String {
        format!("{}:{}", self.addr, self.port)
    }

    /// Resolves the address to a `SocketAddr` if possible.
    ///
    /// Returns the resolved address as a `String`, or an error message if it cannot be resolved.
    pub fn resolve(&self) -> Result<String, String> {
        resolve_address(&self.addr, self.port)
    }
}

impl Default for FanucDriverConfig {
    fn default() -> Self {
        Self {
            addr: "127.0.0.1".to_string(),
            port: 16001,
            max_messages: 30,
            log_level: LogLevel::default(),
        }
    }
}

/// Resolves a DNS name or IP address to a `SocketAddr`.
///
/// Returns the resolved address as a `String`, or an error message if it cannot be resolved.
fn resolve_address(addr: &str, port: u32) -> Result<String, String> {
    let address_with_port = format!("{}:{}", addr, port);
    match address_with_port.to_socket_addrs() {
        Ok(mut iter) => match iter.next() {
            Some(socket_addr) => Ok(socket_addr.to_string()),
            None => Err("Could not resolve address".to_string()),
        },
        Err(_) => Err("Invalid address format".to_string()),
    }
}