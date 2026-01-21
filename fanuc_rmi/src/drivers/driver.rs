use serde::{Deserialize, Serialize};
use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::{broadcast, mpsc, Mutex},
    time::sleep,
};

use tracing::{debug, error, info};

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use std::time::Instant;

// Global request ID counter
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

// Prefer importing from the module rather than re-exporting from here
// Prefer downstream crates to reference modules directly (crate::commands, crate::instructions, crate::dto)
use crate::commands::*;
use crate::packets::*;
use crate::FrcError;

use super::DriverState;
use super::FanucDriverConfig;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DriverPacket {
    pub priority: PacketPriority,
    pub packet: SendPacket,
    pub request_id: u64,
}

impl DriverPacket {
    pub fn new(priority: PacketPriority, packet: SendPacket, request_id: u64) -> Self {
        Self { priority, packet, request_id }
    }
}

/// Protocol error information for broadcasting to clients.
#[derive(Debug, Clone)]
pub struct ProtocolError {
    pub error_type: String,
    pub message: String,
    pub raw_data: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FanucDriver {
    pub config: FanucDriverConfig,
    pub log_channel: tokio::sync::broadcast::Sender<String>,
    pub response_tx: tokio::sync::broadcast::Sender<ResponsePacket>,
    /// Broadcast channel for protocol errors (deserialization failures, etc.)
    pub error_tx: tokio::sync::broadcast::Sender<ProtocolError>,
    /// Broadcast channel for sent instruction notifications
    ///
    /// Subscribe to this channel to receive notifications when instructions are assigned
    /// sequence IDs and sent to the controller. This allows correlating send_packet()
    /// calls (via request_id) with actual sequence IDs.
    pub sent_instruction_tx: tokio::sync::broadcast::Sender<SentInstructionInfo>,
    next_available_sequence_number: Arc<std::sync::Mutex<u32>>, // could prop be taken out and just a varible in the send_queue function
    fanuc_write: Arc<Mutex<WriteHalf<TcpStream>>>,
    fanuc_read: Arc<Mutex<ReadHalf<TcpStream>>>,
    queue_tx: mpsc::Sender<DriverPacket>,
    pub connected: Arc<Mutex<bool>>,
    completed_packet_channel: Arc<Mutex<broadcast::Receiver<CompletedPacketReturnInfo>>>,
}

impl FanucDriver {
    /// Establishes a connection to a Fanuc controller (robot hardware).
    ///
    /// This function attempts to connect to the specified Fanuc controller using the provided
    /// configuration. If the initial connection succeeds, it sends a connection packet to the
    /// controller and waits for a response. If the connection packet is successfully sent and
    /// a valid response is received, it establishes a TCP connection with the controller.
    ///
    /// The function also spawns two asynchronous tasks:
    /// 1. One task handles sending packets to the robot.
    /// 2. The other task handles receiving responses from the robot.
    ///
    /// # Arguments
    ///
    /// * `config` - A `FanucDriverConfig` struct containing the address and port of the Fanuc controller.
    ///
    /// # Returns
    ///
    /// If successful, returns a `Result` containing an instance of `FanucDriver` with an active
    /// TCP connection to the Fanuc controller. Otherwise, returns an `FrcError` indicating the
    /// cause of the failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The connection to the initial address fails after the specified number of retries.
    /// - The connection packet cannot be serialized.
    /// - The connection packet cannot be sent.
    /// - No response is received from the controller.
    /// - The response from the controller cannot be parsed.
    /// - The controller returns an unexpected response.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = FanucDriverConfig {
    ///     addr: "192.168.0.1".to_string(),
    ///     port: 12345,
    /// };
    ///
    /// match FanucDriver::connect(config).await {
    ///     Ok(driver) => {
    ///         // Connection established, use the driver instance
    ///     },
    ///     Err(e) => {
    ///         eprintln!("Failed to connect: {:?}", e);
    ///     }
    /// }
    /// ```
    pub async fn connect(config: FanucDriverConfig) -> Result<FanucDriver, FrcError> {
        info!("Connecting fanuc");
        let init_addr = format!("{}:{}", config.addr, config.port);
        let mut stream = connect_with_retries(&init_addr, 3).await?;

        let packet = Communication::FrcConnect {};
        let serialized_packet = serde_json::to_string(&packet).map_err(|_| {
            FrcError::Serialization(
                "Communication: Connect packet didn't serialize correctly".to_string(),
            )
        })? + "\r\n";

        stream
            .write_all(serialized_packet.as_bytes())
            .await
            .map_err(|e| FrcError::FailedToSend(e.to_string()))?;

        let mut buffer = vec![0; 2048];
        let n = stream
            .read(&mut buffer)
            .await
            .map_err(|e| FrcError::FailedToReceive(e.to_string()))?;

        if n == 0 {
            return Err(FrcError::Disconnected());
        }

        let response = String::from_utf8_lossy(&buffer[..n]);
        info!("Sent: {}Received: {}", &serialized_packet, &response);

        let res: CommunicationResponse = serde_json::from_str(&response)
            .map_err(|e| FrcError::Serialization(format!("Could not parse response: {}", e)))?;

        let new_port = if let CommunicationResponse::FrcConnect(res) = res {
            res.port_number
        } else {
            return Err(FrcError::UnrecognizedPacket);
        };

        drop(stream);
        let init_addr = format!("{}:{}", config.addr, new_port);
        let stream = connect_with_retries(&init_addr, 3).await?;

        let (read_half, write_half) = split(stream);
        let read_half = Arc::new(Mutex::new(read_half));
        let write_half = Arc::new(Mutex::new(write_half));
        let (message_channel, _rx) = broadcast::channel(100);
        let (response_tx, _response_rx) = broadcast::channel(1000); // Larger buffer for high-frequency polling
        let (sent_instruction_tx, _sent_rx) = broadcast::channel(100);
        let (queue_tx, queue_rx) = mpsc::channel::<DriverPacket>(1000); //FIXME: there isnt a system on meteorite monitoring number of packets sent
        let next_available_sequence_number = Arc::new(std::sync::Mutex::new(1));

        let connected = Arc::new(Mutex::new(true));

        let (completed_packet_tx, _) = broadcast::channel(100);
        let return_info_rx = completed_packet_tx.subscribe();
        let return_info = completed_packet_tx.subscribe();
        let completed_packet_channel = Arc::new(Mutex::new(return_info_rx));

        // Error channel for protocol errors
        let (error_tx, _) = broadcast::channel(100);

        let driver = Self {
            config,
            log_channel: message_channel,
            response_tx,
            error_tx,
            sent_instruction_tx,
            next_available_sequence_number,
            fanuc_write: write_half,
            fanuc_read: read_half,
            queue_tx,
            connected,
            completed_packet_channel,
        };

        let driver_clone1 = driver.clone();
        let driver_clone2 = driver.clone();

        tokio::spawn(async move {
            if let Err(e) = driver_clone1
                .send_queue_to_controller(queue_rx, return_info)
                .await
            {
                error!("send_queue failed: {}", e);
            }
        });

        tokio::spawn(async move {
            if let Err(e) = driver_clone2.read_responses(completed_packet_tx).await {
                error!("read_queue_responses failed: {}", e);
            }
        });

        Ok(driver)
    }

    /// Log an error message (always shown if logging feature enabled)
    async fn log_error<T: Into<String>>(&self, message: T) {
        let message = format!("[ERROR] {}", message.into());
        let _ = self.log_channel.send(message.clone());
        #[cfg(feature = "logging")]
        if self.config.log_level >= crate::drivers::driver_config::LogLevel::Error {
            eprintln!("{}", message);
        }
    }

    /// Log a warning message (shown if log_level >= Warn)
    async fn log_warn<T: Into<String>>(&self, message: T) {
        let message = format!("[WARN] {}", message.into());
        let _ = self.log_channel.send(message.clone());
        #[cfg(feature = "logging")]
        if self.config.log_level >= crate::drivers::driver_config::LogLevel::Warn {
            println!("{}", message);
        }
    }

    /// Log an info message (shown if log_level >= Info, which is default)
    async fn log_info<T: Into<String>>(&self, message: T) {
        let message = format!("[INFO] {}", message.into());
        let _ = self.log_channel.send(message.clone());
        #[cfg(feature = "logging")]
        if self.config.log_level >= crate::drivers::driver_config::LogLevel::Info {
            println!("{}", message);
        }
    }

    /// Log a debug message (only shown if log_level == Debug)
    async fn log_debug<T: Into<String>>(&self, message: T) {
        let message = format!("[DEBUG] {}", message.into());
        let _ = self.log_channel.send(message.clone());
        #[cfg(feature = "logging")]
        if self.config.log_level >= crate::drivers::driver_config::LogLevel::Debug {
            println!("{}", message);
        }
    }

    /// Send an abort command to the FANUC controller
    ///
    /// Returns the request ID for tracking this request.
    pub fn send_abort(&self) -> Result<u64, String> {
        let packet = SendPacket::Command(Command::FrcAbort {});
        self.send_packet(packet, PacketPriority::Standard)
    }

    /// Send an abort command and wait for the response
    ///
    /// This is an async convenience method that sends the abort command and waits
    /// for the response from the FANUC controller.
    ///
    /// **Note:** This method waits for the **next** FrcAbortResponse. Do not call
    /// this method concurrently for the same command type. For concurrent usage,
    /// use `send_abort()` and subscribe to `response_tx` manually.
    ///
    /// # Returns
    /// * `Ok(FrcAbortResponse)` - The abort response from the controller
    /// * `Err(String)` - Error if the command could not be sent or timeout (5 seconds)
    ///
    /// # Example
    /// ```no_run
    /// # use fanuc_rmi::drivers::FanucDriver;
    /// # async fn example(driver: &FanucDriver) -> Result<(), String> {
    /// let response = driver.abort().await?;
    /// if response.error_id == 0 {
    ///     println!("Abort successful");
    /// } else {
    ///     println!("Abort failed with error: {}", response.error_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn abort(&self) -> Result<FrcAbortResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let _request_id = self.send_abort()?;

        // Wait up to 5 seconds for response
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(CommandResponse::FrcAbort(abort_response)) = response {
                    return Ok(abort_response);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for abort response".to_string())?;

        // After abort completes, clear the in-flight counter
        // The robot clears its motion queue on abort but doesn't send responses
        // for aborted instructions, so we need to reset our tracking.
        self.clear_in_flight()?;

        result
    }

    /// Clear the driver's in-flight instruction counter.
    ///
    /// This should be called after an abort to reset the driver's tracking,
    /// since the robot clears its motion queue on abort but doesn't send
    /// responses for aborted instructions.
    pub fn clear_in_flight(&self) -> Result<(), String> {
        let packet = SendPacket::DriverCommand(DriverCommand::ClearInFlight);
        // Use High priority to process this command quickly
        self.send_packet(packet, PacketPriority::High)?;
        Ok(())
    }

    /// Send a reset command to the FANUC controller
    ///
    /// Returns the request ID for tracking this request.
    pub fn send_reset(&self) -> Result<u64, String> {
        let packet = SendPacket::Command(Command::FrcReset);
        self.send_packet(packet, PacketPriority::Standard)
    }

    /// Send a reset command and wait for the response
    ///
    /// This is an async convenience method that sends the reset command and waits
    /// for the response from the FANUC controller.
    ///
    /// **Note:** This method waits for the **next** FrcResetResponse. Do not call
    /// this method concurrently for the same command type. For concurrent usage,
    /// use `send_reset()` and subscribe to `response_tx` manually.
    ///
    /// # Returns
    /// * `Ok(FrcResetResponse)` - The reset response from the controller
    /// * `Err(String)` - Error if the command could not be sent or timeout (5 seconds)
    ///
    /// # Example
    /// ```no_run
    /// # use fanuc_rmi::drivers::FanucDriver;
    /// # async fn example(driver: &FanucDriver) -> Result<(), String> {
    /// let response = driver.reset().await?;
    /// if response.error_id == 0 {
    ///     println!("Reset successful");
    /// } else {
    ///     println!("Reset failed with error: {}", response.error_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn reset(&self) -> Result<FrcResetResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let _request_id = self.send_reset()?;

        // Wait up to 5 seconds for response
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(CommandResponse::FrcReset(reset_response)) = response {
                    return Ok(reset_response);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for reset response".to_string())?
    }

    /// Recover from a HOLD state caused by sequence ID errors
    ///
    /// Per FANUC documentation B-84184EN/02 Section 2.4:
    /// "If RMI detects a non-consecutive sequence ID, RMI sends a RMIT-029 Invalid sequence ID
    /// number error ID back to the sender. At this point, RMI goes into a HOLD state. While in
    /// a HOLD state, RMI continues to execute the TP instructions that are already in the TP
    /// program but will not accept new TP instructions until RMI receives the FRC_Reset command.
    /// You can get the correct sequence ID by sending an FRC_GetStatus packet and getting
    /// 'NextSequenceID' : nnnn where the nnnn is the next valid sequence ID."
    ///
    /// This method:
    /// 1. Sends FRC_Reset to clear the HOLD state
    /// 2. Sends FRC_GetStatus to get the correct NextSequenceID
    /// 3. Syncs our sequence counter to match the robot's expected value
    ///
    /// # Returns
    /// * `Ok(())` - Recovery successful
    /// * `Err(String)` - Recovery failed
    ///
    /// # Example
    /// ```no_run
    /// # use fanuc_rmi::drivers::FanucDriver;
    /// # async fn example(driver: &FanucDriver) -> Result<(), String> {
    /// // If you get error 2556957 (Invalid sequence ID number), call this:
    /// driver.recover_from_hold_state().await?;
    /// // Now you can retry your instruction
    /// # Ok(())
    /// # }
    /// ```
    pub async fn recover_from_hold_state(&self) -> Result<(), String> {
        self.log_info("Recovering from HOLD state (sequence ID error)...").await;

        // Step 1: Send FRC_Reset to clear the HOLD state
        self.log_debug("Sending FRC_Reset...").await;
        let reset_response = self.reset().await?;

        if reset_response.error_id != 0 {
            let msg = format!("FRC_Reset failed with error: {}", reset_response.error_id);
            self.log_error(&msg).await;
            return Err(msg);
        }

        // Step 2: Get the correct NextSequenceID from the robot
        self.log_debug("Getting status to sync sequence ID...").await;
        let status = self.get_status().await?;

        if status.error_id != 0 {
            let msg = format!("FRC_GetStatus failed with error: {}", status.error_id);
            self.log_error(&msg).await;
            return Err(msg);
        }

        // Step 3: Sync our sequence counter to match the robot
        let next_seq = status.next_sequence_id;
        self.sync_sequence_counter(next_seq);

        self.log_info(&format!(
            "Recovery complete. Sequence counter synced to: {}",
            next_seq
        )).await;

        Ok(())
    }

    /// Send a pause command to the FANUC controller
    ///
    /// This pauses robot motion. The robot will decelerate and stop at the
    /// current position. Queued motion instructions are preserved.
    ///
    /// Returns the request ID for tracking this request.
    pub fn send_pause(&self) -> Result<u64, String> {
        let packet = SendPacket::Command(Command::FrcPause);
        self.send_packet(packet, PacketPriority::Immediate)
    }

    /// Send a pause command and wait for the response
    ///
    /// This is an async convenience method that sends the pause command and waits
    /// for the response from the FANUC controller. The robot will decelerate and
    /// stop at the current position.
    ///
    /// **Note:** This method waits for the **next** FrcPauseResponse. Do not call
    /// this method concurrently for the same command type. For concurrent usage,
    /// use `send_pause()` and subscribe to `response_tx` manually.
    ///
    /// # Returns
    /// * `Ok(FrcPauseResponse)` - The pause response from the controller
    /// * `Err(String)` - Error if the command could not be sent or timeout (5 seconds)
    ///
    /// # Example
    /// ```no_run
    /// # use fanuc_rmi::drivers::FanucDriver;
    /// # async fn example(driver: &FanucDriver) -> Result<(), String> {
    /// let response = driver.pause().await?;
    /// if response.error_id == 0 {
    ///     println!("Pause successful");
    /// } else {
    ///     println!("Pause failed with error: {}", response.error_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pause(&self) -> Result<FrcPauseResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let _request_id = self.send_pause()?;

        // Wait up to 5 seconds for response
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(CommandResponse::FrcPause(pause_response)) = response {
                    return Ok(pause_response);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for pause response".to_string())?
    }

    /// Send a continue command to the FANUC controller
    ///
    /// This resumes robot motion after a pause. The robot will continue
    /// executing queued motion instructions from where it stopped.
    ///
    /// Returns the request ID for tracking this request.
    pub fn send_continue(&self) -> Result<u64, String> {
        let packet = SendPacket::Command(Command::FrcContinue);
        self.send_packet(packet, PacketPriority::Immediate)
    }

    /// Send a continue command and wait for the response
    ///
    /// This is an async convenience method that sends the continue command and waits
    /// for the response from the FANUC controller. The robot will resume motion
    /// from where it was paused.
    ///
    /// **Note:** This method waits for the **next** FrcContinueResponse. Do not call
    /// this method concurrently for the same command type. For concurrent usage,
    /// use `send_continue()` and subscribe to `response_tx` manually.
    ///
    /// # Returns
    /// * `Ok(FrcContinueResponse)` - The continue response from the controller
    /// * `Err(String)` - Error if the command could not be sent or timeout (5 seconds)
    ///
    /// # Example
    /// ```no_run
    /// # use fanuc_rmi::drivers::FanucDriver;
    /// # async fn example(driver: &FanucDriver) -> Result<(), String> {
    /// let response = driver.continue_motion().await?;
    /// if response.error_id == 0 {
    ///     println!("Continue successful");
    /// } else {
    ///     println!("Continue failed with error: {}", response.error_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn continue_motion(&self) -> Result<FrcContinueResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let _request_id = self.send_continue()?;

        // Wait up to 5 seconds for response
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(CommandResponse::FrcContinue(continue_response)) = response {
                    return Ok(continue_response);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for continue response".to_string())?
    }

    /// Send an initialize command to the FANUC controller
    ///
    /// Returns the request ID for tracking this request.
    pub fn send_initialize(&self) -> Result<u64, String> {
        let packet: SendPacket =
            SendPacket::Command(Command::FrcInitialize(FrcInitialize::default()));
        self.send_packet(packet, PacketPriority::Standard)
    }

    /// Send an initialize command and wait for the response
    ///
    /// This is an async convenience method that sends the initialize command and waits
    /// for the response from the FANUC controller.
    ///
    /// **Note:** This method waits for the **next** FrcInitializeResponse. Do not call
    /// this method concurrently for the same command type. For concurrent usage,
    /// use `send_initialize()` and subscribe to `response_tx` manually.
    ///
    /// # Returns
    /// * `Ok(FrcInitializeResponse)` - The initialize response from the controller
    /// * `Err(String)` - Error if the command could not be sent or timeout (5 seconds)
    ///
    /// # Example
    /// ```no_run
    /// # use fanuc_rmi::drivers::FanucDriver;
    /// # async fn example(driver: &FanucDriver) -> Result<(), String> {
    /// let response = driver.initialize().await?;
    /// if response.error_id == 0 {
    ///     println!("Initialize successful, group_mask: {}", response.group_mask);
    /// } else {
    ///     println!("Initialize failed with error: {}", response.error_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn initialize(&self) -> Result<FrcInitializeResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let _request_id = self.send_initialize()?;

        // Wait up to 5 seconds for response
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(CommandResponse::FrcInitialize(init_response)) = response {
                    return Ok(init_response);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for initialize response".to_string())??;

        // Per FANUC documentation B-84184EN/02 Section 2.4:
        // "Start your SequenceID number from 1 after the FRC_Initialize packet."
        // Reset sequence counter after successful initialization.
        if result.error_id == 0 {
            self.reset_sequence_counter();
        }

        Ok(result)
    }

    /// Send a get status command to the FANUC controller
    ///
    /// Returns the request ID for tracking this request.
    pub fn send_get_status(&self) -> Result<u64, String> {
        let packet: SendPacket = SendPacket::Command(Command::FrcGetStatus);
        self.send_packet(packet, PacketPriority::Standard)
    }

    /// Send a get status command and wait for the response
    ///
    /// This is an async convenience method that sends the get status command and waits
    /// for the response from the FANUC controller.
    ///
    /// **Note:** This method waits for the **next** FrcGetStatusResponse. Do not call
    /// this method concurrently for the same command type. For concurrent usage,
    /// use `send_get_status()` and subscribe to `response_tx` manually.
    ///
    /// # Returns
    /// * `Ok(FrcGetStatusResponse)` - The status response from the controller
    /// * `Err(String)` - Error if the command could not be sent or timeout (5 seconds)
    ///
    /// # Example
    /// ```no_run
    /// # use fanuc_rmi::drivers::FanucDriver;
    /// # async fn example(driver: &FanucDriver) -> Result<(), String> {
    /// let status = driver.get_status().await?;
    /// if status.error_id == 0 {
    ///     println!("Servo ready: {}", status.servo_ready);
    ///     println!("Next sequence ID: {}", status.next_sequence_id);
    /// } else {
    ///     println!("Get status failed with error: {}", status.error_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_status(&self) -> Result<FrcGetStatusResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let _request_id = self.send_get_status()?;

        // Wait up to 5 seconds for response
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(CommandResponse::FrcGetStatus(status_response)) = response {
                    return Ok(status_response);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for get status response".to_string())?
    }

    /// Send a disconnect communication to the FANUC controller
    ///
    /// Returns the request ID for tracking this request.
    pub async fn send_disconnect(&self) -> Result<u64, String> {
        let packet = SendPacket::Communication(Communication::FrcDisconnect {});
        let request_id = self.send_packet(packet, PacketPriority::Standard)?;
        *self.connected.lock().await = false;
        Ok(request_id)
    }

    /// Send a disconnect communication and wait for the response
    ///
    /// This is an async convenience method that sends the disconnect communication and waits
    /// for the response from the FANUC controller.
    ///
    /// **Note:** This method waits for the **next** FrcDisconnectResponse. Do not call
    /// this method concurrently. For concurrent usage, use `send_disconnect()` and
    /// subscribe to `response_tx` manually.
    ///
    /// # Returns
    /// * `Ok(FrcDisconnectResponse)` - The disconnect response from the controller
    /// * `Err(String)` - Error if the command could not be sent or timeout (5 seconds)
    ///
    /// # Example
    /// ```no_run
    /// # use fanuc_rmi::drivers::FanucDriver;
    /// # async fn example(driver: &FanucDriver) -> Result<(), String> {
    /// let response = driver.disconnect().await?;
    /// if response.error_id == 0 {
    ///     println!("Disconnect successful");
    /// } else {
    ///     println!("Disconnect failed with error: {}", response.error_id);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn disconnect(&self) -> Result<FrcDisconnectResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let _request_id = self.send_disconnect().await?;

        // Wait up to 5 seconds for response
        tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommunicationResponse(CommunicationResponse::FrcDisconnect(disconnect_response)) = response {
                    return Ok(disconnect_response);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for disconnect response".to_string())?
    }

    /// Smart initialization sequence that checks robot status before initializing
    ///
    /// This method implements the proper FANUC RMI initialization sequence according to
    /// the B-84184EN_02 manual. It:
    /// 1. Checks the current robot status using FRC_GetStatus
    /// 2. Verifies the robot is ready (servo ready, AUTO mode)
    /// 3. Only aborts if RMI is already running (avoids "RMI Command Failed" error)
    /// 4. Initializes the RMI system
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Initialization successful
    /// * `Err(String)` - Initialization failed with error message
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// * Status check fails
    /// * Robot is not ready (servo errors)
    /// * Robot is not in AUTO mode
    /// * Abort fails (if needed)
    /// * Initialize fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig, LogLevel};
    /// # async fn example() -> Result<(), String> {
    /// let config = FanucDriverConfig {
    ///     addr: "192.168.1.100".to_string(),
    ///     port: 16001,
    ///     max_messages: 30,
    ///     log_level: LogLevel::Info,
    /// };
    ///
    /// let driver = FanucDriver::connect(config).await.map_err(|e| e.to_string())?;
    ///
    /// // Smart initialization - checks status first
    /// driver.startup_sequence().await?;
    ///
    /// // Robot is now ready for motion commands
    /// # Ok(())
    /// # }
    /// ```
    pub async fn startup_sequence(&self) -> Result<(), String> {
        self.log_info("Starting robot initialization sequence...").await;

        // Step 1: Get current status
        self.log_debug("Checking robot status...").await;
        let status = self.get_status().await?;

        if status.error_id != 0 {
            let msg = format!("Get status failed with error: {}", status.error_id);
            self.log_error(&msg).await;
            return Err(msg);
        }

        // Step 2: Check if controller is ready
        if status.servo_ready != 1 {
            let msg = "Controller not ready (servo errors present)".to_string();
            self.log_error(&msg).await;
            return Err(msg);
        }

        // Per FANUC documentation B-84184EN/02:
        // TPMode: if it is 0, the teach pendant is disabled. If it is 1, the teach pendant is enabled.
        // The Remote Motion interface only works when the teach pendant is disabled.
        if status.tp_mode != 0 {
            let msg = "Teach pendant is enabled (tp_mode=1). RMI requires teach pendant to be disabled (tp_mode=0). Switch to AUTO mode.".to_string();
            self.log_error(&msg).await;
            return Err(msg);
        }

        self.log_info(&format!(
            "Robot status: servo_ready={}, tp_mode={}, rmi_motion_status={}",
            status.servo_ready, status.tp_mode, status.rmi_motion_status
        )).await;

        // Step 3: Abort if RMI is already running
        // According to B-84184EN_02: FRC_Abort only works when RMI_MOVE is running
        if status.rmi_motion_status != 0 {
            self.log_info("RMI already running, aborting first...").await;
            let abort_response = self.abort().await?;

            if abort_response.error_id != 0 {
                let msg = format!("Abort failed with error: {}", abort_response.error_id);
                self.log_error(&msg).await;
                return Err(msg);
            }

            self.log_info("Abort successful").await;
        } else {
            self.log_info("RMI not running, skipping abort").await;
        }

        // Step 4: Initialize
        self.log_info("Initializing RMI...").await;
        let init_response = self.initialize().await?;

        if init_response.error_id != 0 {
            let msg = format!("Initialize failed with error: {}", init_response.error_id);
            self.log_error(&msg).await;

            // Special handling for error 7015 (RMI_MOVE program selected)
            if init_response.error_id == 7015 {
                self.log_error("Error 7015: RMI_MOVE program is selected on teach pendant").await;
                self.log_error("Solution: Press SELECT on TP, choose another program, then retry").await;
            }

            return Err(msg);
        }

        // Note: initialize() already resets sequence counter to 1 on success
        // per FANUC documentation B-84184EN/02 Section 2.4
        self.log_info(&format!(
            "Initialization successful (group_mask: {}, sequence counter reset to 1)",
            init_response.group_mask
        )).await;

        Ok(())
    }

    /// Reset the sequence counter to 1
    ///
    /// Per FANUC documentation B-84184EN/02 Section 2.4:
    /// "Start your SequenceID number from 1 after the FRC_Initialize packet."
    ///
    /// This should be called after successful FRC_Initialize.
    pub fn reset_sequence_counter(&self) {
        if let Ok(mut seq_id) = self.next_available_sequence_number.lock() {
            *seq_id = 1;
        }
    }

    /// Sync the sequence counter to match the robot's NextSequenceID
    ///
    /// This is useful after recovering from a HOLD state or reconnecting to
    /// an existing RMI session.
    pub fn sync_sequence_counter(&self, next_sequence_id: u32) {
        if let Ok(mut seq_id) = self.next_available_sequence_number.lock() {
            *seq_id = next_sequence_id;
        }
    }

    async fn send_packet_to_controller(&self, packet: SendPacket) -> Result<(), FrcError> {
        /*
        this is specifically for sending packets to the controller. It takes a packet and sends it over tcp to the controller.
        Note: not a public function
        */

        let mut stream = self.fanuc_write.lock().await;

        let serialized_packet = match serde_json::to_string(&packet) {
            Ok(packet_str) => packet_str + "\r\n",
            Err(e) => {
                self.log_error(format!("Failed to serialize packet: {}", e))
                    .await;
                return Err(FrcError::Serialization(e.to_string()));
            }
        };

        // Add timeout to write operation - this is still important to prevent blocking
        // indefinitely if the connection is stalled
        const WRITE_TIMEOUT: Duration = Duration::from_secs(5);

        match tokio::time::timeout(
            WRITE_TIMEOUT,
            stream.write_all(serialized_packet.as_bytes())
        ).await {
            Ok(result) => {
                if let Err(e) = result {
                    let err = FrcError::FailedToSend(format!("{}", e));
                    self.log_error(err.to_string()).await;
                    return Err(err);
                }
            },
            Err(_) => {
                let err = FrcError::FailedToSend("Write operation timed out".to_string());
                self.log_error(err.to_string()).await;
                return Err(err);
            }
        }

        Ok(())
    }

    /// Send a packet to the FANUC controller
    ///
    /// Returns a request ID that can be used to track when the packet is sent
    /// and correlate it with responses. For Instructions, subscribe to `sent_instruction_tx`
    /// to receive notifications when the instruction is assigned a sequence ID and sent.
    ///
    /// # Arguments
    /// * `packet` - The packet to send (Communication, Command, or Instruction)
    /// * `priority` - The priority level for queue insertion
    ///
    /// # Returns
    /// * `Ok(request_id)` - A unique request ID for this send request
    /// * `Err(String)` - Error message if the packet could not be queued
    ///
    /// # Example
    /// ```rust,ignore
    /// let request_id = driver.send_packet(packet, PacketPriority::Standard)?;
    /// // Subscribe to sent_instruction_tx to get the sequence ID when it's assigned
    /// ```
    pub fn send_packet(
        &self,
        packet: SendPacket,
        priority: PacketPriority,
    ) -> Result<u64, String> {
        // Generate unique request ID
        let request_id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);

        // Log what command/communication is being sent (at debug level)
        // This uses the driver's log_debug method which respects config.log_level
        let self_clone = self.clone();
        let packet_clone = packet.clone();
        tokio::spawn(async move {
            match &packet_clone {
                SendPacket::Command(cmd) => {
                    self_clone.log_debug(format!("📤 Sending command: {:?}", cmd)).await;
                }
                SendPacket::Communication(comm) => {
                    self_clone.log_debug(format!("📤 Sending communication: {:?}", comm)).await;
                }
                SendPacket::Instruction(instr) => {
                    self_clone.log_debug(format!("📤 Sending instruction: {:?}", instr)).await;
                }
                SendPacket::DriverCommand(_) => {
                    self_clone.log_debug("📤 Sending driver command".to_string()).await;
                }
            }
        });

        // Commands and Communications bypass the instruction queue entirely.
        // Only Instructions need backpressure (the 8-slot buffer limit applies to TP instructions only).
        // Commands are processed immediately by the controller and don't consume buffer slots.
        match &packet {
            SendPacket::Command(_) | SendPacket::Communication(_) => {
                // Send directly to controller - bypass instruction queue
                let fanuc_write = Arc::clone(&self.fanuc_write);
                let log_channel = self.log_channel.clone();

                tokio::spawn(async move {
                    let serialized_packet = match serde_json::to_string(&packet) {
                        Ok(packet_str) => packet_str + "\r\n",
                        Err(e) => {
                            let _ = log_channel.send(format!("ERROR: Failed to serialize command: {}", e));
                            return;
                        }
                    };

                    let mut stream = fanuc_write.lock().await;
                    if let Err(e) = stream.write_all(serialized_packet.as_bytes()).await {
                        let _ = log_channel.send(format!("ERROR: Failed to send command: {}", e));
                    }
                });
            }
            SendPacket::Instruction(_) | SendPacket::DriverCommand(_) => {
                // Instructions go through the queue with backpressure
                let sender = self.queue_tx.clone();

                let driver_packet = DriverPacket {
                    priority,
                    packet,
                    request_id,
                };

                if let Err(e) = sender.try_send(driver_packet) {
                    println!("Failed to send packet: {}", e);
                    return Err(format!("Failed to send packet: {}", e));
                }
            }
        }

        Ok(request_id)
    }

    /// Send a packet to the FANUC controller
    ///
    /// **DEPRECATED:** Use `send_packet()` instead. This method name is misleading
    /// as it sends any packet type (Communication, Command, or Instruction), not just Commands.
    #[deprecated(since = "0.5.0", note = "Use send_packet instead - send_command is misleading as it sends any packet type")]
    pub fn send_command(
        &self,
        packet: SendPacket,
        priority: PacketPriority,
    ) -> Result<u64, String> {
        self.send_packet(packet, priority)
    }

    //this is an async function that receives packets and forwards them to the controller
    async fn send_queue_to_controller(
        &self,
        mut packets_to_add: mpsc::Receiver<DriverPacket>,
        mut completed_packet_info: broadcast::Receiver<CompletedPacketReturnInfo>,
    ) -> Result<(), FrcError> {
        let mut in_flight: u32 = 0;
        let mut queue: VecDeque<DriverPacket> = VecDeque::new();
        let mut state = DriverState::default();

        // Standard loop interval
        const LOOP_INTERVAL: Duration = Duration::from_millis(8);
        // Maximum in-flight packets (backpressure)
        const MAX_IN_FLIGHT: u32 = 8;
        // Per FANUC documentation B-84184EN/02 Section 3.2:
        // "For each of the 8 instructions, please wait at least 2 milliseconds before
        // sending the next instruction. This is due to TCP/IP packs several RMI packets
        // together in one TCP/IP packet if these RMI packets arrive around the same time.
        // It is possible that during the packing, an RMI packet could be broken into two
        // parts and carried by two TCP/IP packets. RMI will return an error in this case."
        const INSTRUCTION_DELAY: Duration = Duration::from_millis(2);

        loop {
            let start_time = Instant::now();

            // Drain all available incoming packets
            while let Ok(new_packet) = packets_to_add.try_recv() {
                match (new_packet.packet.clone(), &state) {
                    (SendPacket::DriverCommand(DriverCommand::Pause), DriverState::Running) => {
                        state = DriverState::Paused
                    }
                    (SendPacket::DriverCommand(DriverCommand::Unpause), DriverState::Paused) => {
                        state = DriverState::Running
                    }
                    _ => {}
                }

                // Handle driver commands (these don't get sent to robot)
                if let SendPacket::DriverCommand(cmd) = &new_packet.packet {
                    match cmd {
                        DriverCommand::ClearInFlight => {
                            // Reset the in-flight counter. This is needed after abort
                            // because the robot clears its queue but doesn't send responses
                            // for aborted instructions.
                            let old_in_flight = in_flight;
                            in_flight = 0;
                            println!("ClearInFlight: reset in_flight counter from {} to 0", old_in_flight);
                        }
                        _ => {
                            println!("GOT A DRIVER COMMAND: {:?}", cmd);
                        }
                    }
                    continue;
                }

                match new_packet.priority {
                    PacketPriority::Low | PacketPriority::Standard => {
                        queue.push_back(new_packet)
                    }
                    PacketPriority::High | PacketPriority::Immediate => {
                        queue.push_front(new_packet)
                    }
                    PacketPriority::Termination => {
                        queue.clear();
                        queue.push_front(new_packet);
                    }
                }
            }

            // Process completed packets
            while let Ok(pkt) = completed_packet_info.try_recv() {
                in_flight = in_flight.saturating_sub(1);
                // Log if error occurred
                if pkt.error_id != 0 {
                    self.log_error(format!(
                        "Error in packet {}: error_id={}",
                        pkt.sequence_id, pkt.error_id
                    )).await;
                }
            }

            if packets_to_add.is_closed() && queue.is_empty() {
                break;
            }

            // Send packets with backpressure
            while in_flight < MAX_IN_FLIGHT && state == DriverState::Running {
                if let Some(mut driver_packet) = queue.pop_front() {
                    // Assign sequence ID right before sending (ensures consecutive IDs in send order)
                    if let SendPacket::Instruction(ref mut instruction) = driver_packet.packet {
                        let current_id = {
                            // Lock, increment, and immediately drop the guard
                            match self.next_available_sequence_number.lock() {
                                Ok(mut sid) => {
                                    let id = *sid;
                                    *sid += 1;
                                    id
                                }
                                Err(poisoned) => {
                                    // Can't await here, so just log to stderr and break
                                    eprintln!("Sequence ID mutex poisoned: {}", poisoned);
                                    break;
                                }
                            }
                        }; // MutexGuard dropped here

                        // Assign sequence ID to instruction
                        match instruction {
                            Instruction::FrcWaitDIN(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcSetUFrame(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcSetUTool(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcWaitTime(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcSetPayLoad(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcCall(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcLinearMotion(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcLinearRelative(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcLinearRelativeJRep(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcJointMotion(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcJointRelative(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcCircularMotion(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcCircularRelative(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcJointMotionJRep(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcJointRelativeJRep(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcLinearMotionJRep(ref mut instr) => instr.sequence_id = current_id,
                        }

                        // Broadcast sent instruction info
                        let _ = self.sent_instruction_tx.send(SentInstructionInfo {
                            request_id: driver_packet.request_id,
                            sequence_id: current_id,
                            timestamp: Instant::now(),
                        });
                    }

                    match self.send_packet_to_controller(driver_packet.packet.clone()).await {
                        Err(e) => {
                            self.log_error(format!("Failed to send packet: {:?}", e))
                                .await;
                        }
                        Ok(()) => {
                            if driver_packet.packet == SendPacket::Communication(Communication::FrcDisconnect) {
                                // immediate shutdown
                                queue.clear();
                                break;
                            }
                            if let SendPacket::Instruction(instr) = driver_packet.packet {
                                let _seq = instr.get_sequence_id();
                                in_flight += 1;

                                // Per FANUC documentation B-84184EN/02 Section 3.2:
                                // Wait at least 2ms between consecutive instructions to prevent
                                // TCP/IP packet fragmentation issues that cause RMI errors.
                                tokio::time::sleep(INSTRUCTION_DELAY).await;
                            }
                        }
                    }
                } else {
                    break;
                }
            }

            // Maintain consistent loop timing
            let elapsed = Instant::now().duration_since(start_time);
            if elapsed < LOOP_INTERVAL {
                tokio::time::sleep(LOOP_INTERVAL - elapsed).await;
            } else {
                self.log_warn(format!(
                    "Send loop duration took {:?} exceeding max time:{:?}",
                    elapsed, LOOP_INTERVAL
                )).await;
            }
        }

        self.log_info("Disconnecting from FRC server... closing send queue")
            .await;
        Ok(())
    }

    // Simplified main loop:
    async fn read_responses(
        &self,
        completed_tx: broadcast::Sender<CompletedPacketReturnInfo>,
    ) -> Result<(), FrcError> {
        let mut reader = self.fanuc_read.lock().await;
        let mut buf = vec![0; 2048];
        let mut temp = Vec::new();

        // Standard loop interval for processing
        const LOOP_INTERVAL: Duration = Duration::from_millis(10);

        loop {
            // Maintain a consistent loop interval for processing
            let start_time = Instant::now();

            // Read without timeout - we want to stay connected indefinitely
            let n = match reader.read(&mut buf).await {
                Ok(0) => {
                    // Connection closed by peer
                    *self.connected.lock().await = false;
                    return Err(FrcError::Disconnected());
                }
                Ok(n) => n,
                Err(e) => {
                    self.log_error(format!("Read error: {}", e)).await;
                    *self.connected.lock().await = false;
                    return Err(FrcError::FailedToReceive(e.to_string()));
                }
            };

            temp.extend_from_slice(&buf[..n]);
            for line in extract_lines(&mut temp) {
                if let Err(e) = self.process_line(line, &completed_tx).await {
                    self.log_error(format!("Error processing line: {:?}", e)).await;
                    // Continue processing other lines even if one fails
                }
            }

            let elapsed = Instant::now().duration_since(start_time);
            if elapsed < LOOP_INTERVAL {
                tokio::time::sleep(LOOP_INTERVAL - elapsed).await;
            }
        }
    }

    // Extract handling of each line into an async helper:
    async fn process_line(
        &self,
        line: String,
        completed_tx: &broadcast::Sender<CompletedPacketReturnInfo>,
    ) -> Result<(), FrcError> {
        // HOT PATH: Only log at debug level to avoid flooding terminal
        self.log_debug(format!("Received: {}", line)).await;

        match serde_json::from_str::<ResponsePacket>(&line) {
            Ok(packet) => {
                // Log InstructionResponse at info level for debugging
                if matches!(packet, ResponsePacket::InstructionResponse(_)) {
                    info!("📥 Received InstructionResponse: {:?}", packet);
                }

                // Send the response to the response_channel for all responses
                if let Err(e) = self.response_tx.send(packet.clone()) {
                    self.log_error(format!("Failed to send to response channel: {}", e))
                        .await;
                    info!(
                        "Failed to send message to response channel {:?}: {:?}",
                        packet.clone(),
                        e
                    );
                } else {
                    // Log InstructionResponse broadcast at info level
                    if matches!(packet, ResponsePacket::InstructionResponse(_)) {
                        info!("📤 Broadcast InstructionResponse to {} subscribers", self.response_tx.receiver_count());
                    }
                    // HOT PATH: Only log at debug level for other types
                    self.log_debug(format!(
                        "Sent response to backend: {:?}",
                        packet.clone()
                    ))
                    .await;
                    debug!("Sent message to response channel: {:?}", packet.clone())
                }

                match packet {
                    ResponsePacket::CommunicationResponse(CommunicationResponse::FrcDisconnect(_)) => {
                        self.log_info("Received disconnect packet").await;
                        let mut conn = self.connected.lock().await;
                        *conn = false;
                        return Ok(());
                    }
                    ResponsePacket::InstructionResponse(pkt) => {
                        let info = CompletedPacketReturnInfo {
                            sequence_id: pkt.get_sequence_id(),
                            error_id: pkt.get_error_id(),
                        };
                        if let Err(e) = completed_tx.send(info) {
                            self.log_error(format!("Failed to send completion info: {}", e)).await;
                        }
                    }
                    ResponsePacket::CommandResponse(CommandResponse::FrcGetStatus(_status_response)) => {
                        // Per FANUC documentation B-84184EN/02 Section 2.4:
                        // "Start your SequenceID number from 1 after the FRC_Initialize packet."
                        //
                        // We do NOT automatically sync the sequence counter from FRC_GetStatus.
                        // The sequence counter should:
                        // 1. Be reset to 1 after FRC_Initialize (done in startup_sequence)
                        // 2. Increment consecutively with each instruction
                        // 3. Only be synced during explicit error recovery (e.g., after FRC_Reset)
                        //
                        // Automatic syncing during normal operation causes race conditions and
                        // can result in duplicate or skipped sequence IDs.
                        //
                        // Use sync_sequence_counter() explicitly when recovering from errors.
                    }
                    ResponsePacket::CommandResponse(CommandResponse::FrcSetOverRide(
                        frc_set_override_response,
                    )) => {
                        info!("Got set override response: {:?}", frc_set_override_response);
                    }
                    // handle other variants similarly...
                    _ => {}
                }
            }
            Err(e) => {
                let error_msg = format!("Invalid JSON ({}): {}", e, line);
                self.log_error(error_msg.clone()).await;

                // Broadcast protocol error to subscribers
                let protocol_error = ProtocolError {
                    error_type: "protocol".to_string(),
                    message: format!("Failed to parse robot response: {}", e),
                    raw_data: Some(line.to_string()),
                };
                if let Err(send_err) = self.error_tx.send(protocol_error) {
                    // No subscribers - that's okay, just log it
                    debug!("No error channel subscribers: {}", send_err);
                }
            }
        }
        Ok(())
    }

    // DEPRECATED: Sequence IDs are now assigned in send_queue_to_controller()
    // This ensures consecutive sequence IDs in send order, not queue insertion order.
    // Keeping this function for reference but it's no longer used.
    #[allow(dead_code)]
    fn give_sequence_id(&self, mut packet: SendPacket) -> Result<(SendPacket, u32), String> {
        let sid = self.next_available_sequence_number.clone();

        let mut sid = match sid.lock() {
            Ok(guard) => guard,
            Err(poisoned) => return Err(format!("Mutex poisoned: {}", poisoned)),
        };

        let current_id = *sid;

        if let SendPacket::Instruction(ref mut instruction) = packet {
            match instruction {
                Instruction::FrcWaitDIN(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcSetUFrame(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcSetUTool(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcWaitTime(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcSetPayLoad(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcCall(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcLinearMotion(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcLinearRelative(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcLinearRelativeJRep(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcJointMotion(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcJointRelative(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcCircularMotion(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcCircularRelative(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcJointMotionJRep(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcJointRelativeJRep(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcLinearMotionJRep(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
            }

            *sid += 1;
        }
        return Ok((packet, current_id));
    }

    /// Wait for an instruction to complete by sequence ID
    ///
    /// This is the renamed version of `wait_on_command_completion` for clarity.
    /// Polls the completed packet channel until an instruction with the given
    /// sequence ID (or higher) completes.
    ///
    /// # Arguments
    /// * `sequence_id` - The sequence ID to wait for
    ///
    /// # Behavior
    /// - Breaks immediately if an error occurs (error_id != 0)
    /// - Breaks when sequence_id >= the target sequence ID
    /// - Polls every 10ms
    pub async fn wait_on_instruction_completion(&self, sequence_id: u32) {
        const WAIT_INTERVAL: Duration = Duration::from_millis(10);

        loop {
            let start_time = Instant::now();

            let guard = self.completed_packet_channel.clone();
            let mut guard = guard.lock().await;
            match guard.try_recv() {
                Ok(most_recent) => {
                    if most_recent.error_id != 0 {
                        eprintln!("ROBOT MOTION ERROR: {}", most_recent.error_id);
                        break;
                    } else {
                        if most_recent.sequence_id >= sequence_id {
                            println!("robot move done #{}", most_recent.sequence_id);
                            break;
                        }
                    }
                }
                Err(broadcast::error::TryRecvError::Empty) => {}
                Err(broadcast::error::TryRecvError::Closed) => info!("Channel closed."),
                Err(broadcast::error::TryRecvError::Lagged(skipped)) => {
                    info!("Channel lagged, skipped {} messages.", skipped)
                }
            }
            drop(guard);

            // Maintain consistent loop timing
            let elapsed = Instant::now().duration_since(start_time);
            if elapsed < WAIT_INTERVAL {
                tokio::time::sleep(WAIT_INTERVAL - elapsed).await;
            }
        }
    }

    /// Deprecated: Use `wait_on_instruction_completion` instead
    ///
    /// This function is kept for backward compatibility but will be removed in a future version.
    #[deprecated(since = "0.1.0", note = "Use wait_on_instruction_completion instead")]
    pub async fn wait_on_command_completion(&self, packet_number_to_wait_for: u32) {
        self.wait_on_instruction_completion(packet_number_to_wait_for).await;
    }

    /// Wait for an instruction to complete using its request ID
    ///
    /// This is a convenience function that:
    /// 1. Subscribes to sent_instruction_tx to get the sequence ID
    /// 2. Waits for the instruction with that sequence ID to complete
    ///
    /// # Arguments
    /// * `request_id` - The request ID returned from send_packet()
    ///
    /// # Returns
    /// * `Ok(sequence_id)` - The sequence ID that was assigned to the instruction
    /// * `Err(String)` - Error if the sent notification was not received
    ///
    /// # Example
    /// ```rust,ignore
    /// let request_id = driver.send_packet(packet, PacketPriority::Standard)?;
    /// let sequence_id = driver.wait_on_request_completion(request_id).await?;
    /// println!("Instruction {} completed", sequence_id);
    /// ```
    pub async fn wait_on_request_completion(&self, request_id: u64) -> Result<u32, String> {
        // Subscribe to sent notifications
        let mut sent_rx = self.sent_instruction_tx.subscribe();

        // Wait for our instruction to be sent and get its sequence ID
        let sequence_id = loop {
            match sent_rx.recv().await {
                Ok(sent_info) if sent_info.request_id == request_id => {
                    break sent_info.sequence_id;
                }
                Ok(_) => continue, // Not our instruction
                Err(e) => return Err(format!("Failed to receive sent notification: {}", e)),
            }
        };

        // Wait for completion
        self.wait_on_instruction_completion(sequence_id).await;

        Ok(sequence_id)
    }

    /// Wait for an instruction to complete using its request ID
    ///
    /// **DEPRECATED:** Use `wait_on_request_completion()` instead. "request_id" is industry
    /// standard terminology (HTTP/2, gRPC, AWS SDK).
    #[deprecated(since = "0.5.0", note = "Use wait_on_request_completion instead - request_id is industry standard terminology")]
    pub async fn wait_on_correlation_completion(&self, correlation_id: u64) -> Result<u32, String> {
        self.wait_on_request_completion(correlation_id).await
    }

    /// Send an instruction and wait for it to complete
    ///
    /// This is a convenience function that combines send_packet() and
    /// wait_on_request_completion() into a single call.
    ///
    /// # Arguments
    /// * `packet` - The packet to send (should be an Instruction)
    /// * `priority` - The priority level for queue insertion
    ///
    /// # Returns
    /// * `Ok(sequence_id)` - The sequence ID that was assigned to the instruction
    /// * `Err(String)` - Error if send or wait failed
    ///
    /// # Example
    /// ```rust,ignore
    /// let sequence_id = driver.send_and_wait_for_completion(
    ///     SendPacket::Instruction(instruction),
    ///     PacketPriority::Standard
    /// ).await?;
    /// println!("Instruction {} completed", sequence_id);
    /// ```
    pub async fn send_and_wait_for_completion(
        &self,
        packet: SendPacket,
        priority: PacketPriority,
    ) -> Result<u32, String> {
        let request_id = self.send_packet(packet, priority)?;
        self.wait_on_request_completion(request_id).await
    }
}
async fn connect_with_retries(addr: &str, retries: u32) -> Result<TcpStream, FrcError> {
    for attempt in 0..retries {
        match TcpStream::connect(addr).await {
            Ok(stream) => return Ok(stream),
            Err(e) => {
                eprintln!("Failed to connect (attempt {}): {}", attempt + 1, e);
                if attempt + 1 == retries {
                    return Err(FrcError::Disconnected());
                }
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
    return Err(FrcError::Disconnected());
}

// Extract parsing of complete lines into a helper:
fn extract_lines(buffer: &mut Vec<u8>) -> Vec<String> {
    let mut lines = Vec::new();
    while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
        let mut chunk = buffer.drain(..=pos).collect::<Vec<_>>();
        chunk.pop(); // remove the `\n`
        if let Ok(s) = String::from_utf8(chunk) {
            lines.push(s);
        }
    }
    lines
}
