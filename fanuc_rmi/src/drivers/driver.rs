use serde::{Deserialize, Serialize};
use tokio::{
    io::{split, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
    sync::{broadcast, mpsc, Mutex, Notify},
    time::sleep,
};

use tracing::{debug, error, info, warn};

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use std::time::Instant;

// Global request ID counter
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

// Prefer importing from the module rather than re-exporting from here
// Prefer downstream crates to reference modules directly (crate::commands, crate::instructions, crate::dto)
use crate::commands::*;
use crate::instructions::FrcCall;
use crate::packets::*;
use crate::{FrcError, GroupMask};

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

/// Direction of a raw protocol frame relative to this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawDirection {
    /// Written to the controller socket.
    Sent,
    /// Read from the controller socket.
    Received,
}

impl RawDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            RawDirection::Sent => "sent",
            RawDirection::Received => "received",
        }
    }
}

/// One raw JSON frame exactly as it crossed the socket, before/after any typed
/// deserialization. Used by diagnostic tooling that must capture the wire form
/// (including frames the typed layer fails to parse).
#[derive(Debug, Clone)]
pub struct RawFrame {
    pub direction: RawDirection,
    /// The JSON text, with the trailing `\r\n` frame terminator stripped.
    pub payload: String,
    /// Set when the driver itself failed to deserialize a received frame
    /// (carries the serde error text).
    pub note: Option<String>,
}

/// Optional observer invoked for every raw frame the driver sends or receives.
///
/// Additive and off by default: [`FanucDriver::connect`] installs an empty hook,
/// so behaviour is unchanged unless a caller opts in via
/// [`FanucDriver::connect_with_raw_hook`]. The callback is synchronous and runs
/// on the driver's I/O path — keep it cheap.
#[derive(Clone, Default)]
pub struct RawFrameHook(Option<Arc<dyn Fn(RawFrame) + Send + Sync>>);

impl std::fmt::Debug for RawFrameHook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.is_some() {
            "RawFrameHook(installed)"
        } else {
            "RawFrameHook(none)"
        })
    }
}

impl RawFrameHook {
    /// Install an observer callback.
    pub fn new(f: impl Fn(RawFrame) + Send + Sync + 'static) -> Self {
        Self(Some(Arc::new(f)))
    }

    /// No observer (the default; zero overhead).
    pub fn none() -> Self {
        Self(None)
    }

    /// True when an observer is installed.
    pub fn is_installed(&self) -> bool {
        self.0.is_some()
    }

    /// Report a frame written to the controller.
    pub fn sent(&self, payload: impl Into<String>) {
        self.emit(RawDirection::Sent, payload, None);
    }

    /// Report a frame read from the controller, with the driver's own
    /// deserialization error when one occurred.
    pub fn received(&self, payload: impl Into<String>, note: Option<String>) {
        self.emit(RawDirection::Received, payload, note);
    }

    fn emit(&self, direction: RawDirection, payload: impl Into<String>, note: Option<String>) {
        if let Some(f) = &self.0 {
            let payload: String = payload.into();
            f(RawFrame {
                direction,
                payload: payload.trim_end_matches(['\r', '\n']).to_string(),
                note,
            });
        }
    }
}

/// How long a cooperatively-cancelled background task is given to notice the
/// cancellation and unwind before it is aborted outright.
const TASK_GRACE: Duration = Duration::from_millis(250);

/// Deterministic teardown for one RMI session.
///
/// # Why this type exists (the connection leak)
///
/// [`FanucDriver`] is `Clone`, and [`FanucDriver::connect`] hands a clone to
/// each of its two background tasks (`send_queue_to_controller` /
/// `read_responses`). Those clones hold the `Arc`s wrapping the socket halves,
/// and the reader parks on `reader.read()` "indefinitely" — so the tasks kept
/// the session alive **forever**, in a reference cycle nothing could break:
///
/// * dropping every caller-held driver freed no socket, and
/// * `send_queue_to_controller`'s only exit is `packets_to_add.is_closed()`,
///   which can never happen while the task's *own* clone holds `queue_tx`.
///
/// Observed on real hardware (2026-07-28): one server process holding three
/// simultaneous sockets to a controller's data port — one per failed connect
/// attempt — which wedged the controller, since a FANUC serves exactly one RMI
/// session.
///
/// # How it breaks the cycle
///
/// Exactly one `Arc<DriverShutdown>` lives on the caller-facing driver
/// (`shutdown_owner: Some(..)`). The clones handed to the background tasks are
/// made with [`FanucDriver::task_clone`], which sets `shutdown_owner: None`. So
/// the tasks can no longer keep the owner alive: when the last caller-held
/// driver drops, this `Drop` runs, cancels the loops, closes the write half and
/// aborts the tasks as a hard backstop. The tasks' clones then drop, which
/// drops both socket halves and releases the fd.
#[derive(Debug)]
struct DriverShutdown {
    /// Set once teardown has begun; the loops check it and exit.
    cancelled: Arc<AtomicBool>,
    /// Wakes loops that are parked on a socket read / sleep.
    notify: Arc<Notify>,
    /// Write half, so teardown can send FIN even if the reader is wedged.
    write: Arc<Mutex<WriteHalf<TcpStream>>>,
    /// Handles for the driver's own background tasks.
    tasks: std::sync::Mutex<Vec<tokio::task::JoinHandle<()>>>,
    /// `addr:port` of the data socket, for logging.
    peer: String,
}

impl DriverShutdown {
    /// Idempotent teardown: cancel the loops, close the socket, abort the tasks.
    ///
    /// Safe to call from `Drop` (does no awaiting itself — the socket close and
    /// the backstop abort run on a detached task).
    fn tear_down(&self, reason: &str) {
        if self.cancelled.swap(true, Ordering::SeqCst) {
            return; // already torn down
        }
        info!("FanucDriver teardown ({}) for session {}", reason, self.peer);

        // 1. Cooperative: wake both loops so they can exit on their own.
        self.notify.notify_waiters();

        // 2. Take the task handles now; the detached closer aborts them if the
        //    cooperative exit does not land within the grace window.
        let handles: Vec<_> = self
            .tasks
            .lock()
            .map(|mut t| t.drain(..).collect())
            .unwrap_or_default();

        let write = self.write.clone();
        let peer = self.peer.clone();

        match tokio::runtime::Handle::try_current() {
            Ok(rt) => {
                rt.spawn(async move {
                    // Shut the write half down: sends FIN immediately, so a
                    // healthy controller drops the RMI session even if we never
                    // get to abort the reader.
                    match tokio::time::timeout(TASK_GRACE, write.lock()).await {
                        Ok(mut w) => {
                            if let Err(e) = w.shutdown().await {
                                debug!("FanucDriver {}: write-half shutdown: {}", peer, e);
                            }
                        }
                        Err(_) => {
                            warn!(
                                "FanucDriver {}: write half still locked at teardown — \
                                 aborting tasks without a clean FIN",
                                peer
                            );
                        }
                    }

                    // Give the loops the rest of the grace window to unwind.
                    tokio::time::sleep(TASK_GRACE).await;

                    // 3. Hard backstop. `read_responses` parks on a socket read
                    //    that a wedged controller may never satisfy; abort is
                    //    what guarantees its driver clone — and therefore the
                    //    socket — is released.
                    for h in handles {
                        if !h.is_finished() {
                            h.abort();
                        }
                    }
                    info!("FanucDriver {}: session closed, background tasks stopped", peer);
                });
            }
            Err(_) => {
                // No runtime (e.g. dropped after the runtime shut down). Abort
                // synchronously — the runtime teardown closes the fds anyway.
                for h in handles {
                    h.abort();
                }
            }
        }
    }
}

impl Drop for DriverShutdown {
    fn drop(&mut self) {
        self.tear_down("driver dropped");
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
    /// Shared storage for in-flight instructions during program pause/resume.
    /// When program_pause is called, in-flight instructions are stored here.
    /// When program_resume is called, instructions are read from here for replay.
    program_pause_instructions: Arc<std::sync::Mutex<Vec<Instruction>>>,
    /// Optional raw-frame observer (see [`RawFrameHook`]). Empty unless the
    /// caller connected via [`FanucDriver::connect_with_raw_hook`].
    pub raw_hook: RawFrameHook,
    /// Set once the session is being torn down. Read by both background loops
    /// (and by [`FanucDriver::is_shut_down`]); present on *every* clone.
    cancelled: Arc<AtomicBool>,
    /// Wakes the background loops out of a parked read / sleep at teardown.
    /// Present on every clone.
    cancel_notify: Arc<Notify>,
    /// Sole owner of the session teardown — see [`DriverShutdown`].
    ///
    /// `Some` on every caller-held clone, `None` on the clones handed to the
    /// driver's own background tasks. That asymmetry is what lets the last
    /// caller-held driver drop actually close the socket instead of deadlocking
    /// on a reference cycle with its own tasks.
    shutdown_owner: Option<Arc<DriverShutdown>>,
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
        Self::connect_with_raw_hook(config, RawFrameHook::none()).await
    }

    /// Same as [`FanucDriver::connect`], but installs a [`RawFrameHook`] that
    /// observes every raw JSON frame sent to and received from the controller —
    /// including the `FRC_Connect` handshake and frames the typed layer fails to
    /// deserialize. Intended for diagnostic/capture tooling.
    pub async fn connect_with_raw_hook(
        config: FanucDriverConfig,
        raw_hook: RawFrameHook,
    ) -> Result<FanucDriver, FrcError> {
        info!("Connecting fanuc");
        let init_addr = format!("{}:{}", config.addr, config.port);
        let mut stream = connect_with_retries(&init_addr, 3).await?;

        let packet = Communication::FrcConnect {};
        let serialized_packet = serde_json::to_string(&packet).map_err(|_| {
            FrcError::Serialization(
                "Communication: Connect packet didn't serialize correctly".to_string(),
            )
        })? + "\r\n";

        raw_hook.sent(&serialized_packet);

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

        let parsed: Result<CommunicationResponse, _> = serde_json::from_str(&response);
        raw_hook.received(response.to_string(), parsed.as_ref().err().map(|e| e.to_string()));
        let res: CommunicationResponse = parsed
            .map_err(|e| FrcError::Serialization(format!("Could not parse response: {}", e)))?;

        let new_port = if let CommunicationResponse::FrcConnect(res) = res {
            res.port_number
        } else {
            return Err(FrcError::UnrecognizedPacket);
        };

        drop(stream);
        let data_addr = format!("{}:{}", config.addr, new_port);
        let stream = connect_with_retries(&data_addr, 3).await?;

        // ── The DATA socket is now open. ────────────────────────────────────
        // Everything below MUST leave this socket closed if it returns `Err`.
        // That is now structural rather than a rule to remember: the socket
        // halves are handed straight to the driver, whose `shutdown_owner`
        // (`DriverShutdown`) closes them and stops the background tasks on
        // drop. So an early `return Err(..)` from here on — or a caller who
        // drops the driver because a later `initialize()` / `startup_sequence()`
        // failed, or whose whole connect future is cancelled by a timeout —
        // releases the session instead of leaking it.
        //
        // Leaking it is exactly what happened before: a controller serves one
        // RMI session, so every abandoned attempt burned the only one there is.

        let (read_half, write_half) = split(stream);
        let read_half = Arc::new(Mutex::new(read_half));
        let write_half = Arc::new(Mutex::new(write_half));
        let (message_channel, _rx) = broadcast::channel(100);
        let (response_tx, _response_rx) = broadcast::channel(1000); // Larger buffer for high-frequency polling
        let (sent_instruction_tx, _sent_rx) = broadcast::channel(100);
        let (queue_tx, queue_rx) = mpsc::channel::<DriverPacket>(1000); //FIXME: no backpressure/monitoring on the number of packets queued to the controller
        let next_available_sequence_number = Arc::new(std::sync::Mutex::new(1));

        let connected = Arc::new(Mutex::new(true));

        let (completed_packet_tx, _) = broadcast::channel(100);
        let return_info_rx = completed_packet_tx.subscribe();
        let return_info = completed_packet_tx.subscribe();
        let completed_packet_channel = Arc::new(Mutex::new(return_info_rx));

        // Error channel for protocol errors
        let (error_tx, _) = broadcast::channel(100);

        let cancelled = Arc::new(AtomicBool::new(false));
        let cancel_notify = Arc::new(Notify::new());

        let shutdown_owner = Arc::new(DriverShutdown {
            cancelled: cancelled.clone(),
            notify: cancel_notify.clone(),
            write: write_half.clone(),
            tasks: std::sync::Mutex::new(Vec::new()),
            peer: data_addr,
        });

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
            program_pause_instructions: Arc::new(std::sync::Mutex::new(Vec::new())),
            raw_hook,
            cancelled,
            cancel_notify,
            shutdown_owner: Some(shutdown_owner.clone()),
        };

        // `task_clone`, NOT `clone`: the background tasks must not own the
        // teardown, or nothing could ever close the socket (see DriverShutdown).
        let driver_clone1 = driver.task_clone();
        let driver_clone2 = driver.task_clone();

        let send_task = tokio::spawn(async move {
            if let Err(e) = driver_clone1
                .send_queue_to_controller(queue_rx, return_info)
                .await
            {
                error!("send_queue failed: {}", e);
            }
        });

        let read_task = tokio::spawn(async move {
            if let Err(e) = driver_clone2.read_responses(completed_packet_tx).await {
                error!("read_queue_responses failed: {}", e);
            }
        });

        if let Ok(mut tasks) = shutdown_owner.tasks.lock() {
            tasks.push(send_task);
            tasks.push(read_task);
        }

        Ok(driver)
    }

    /// Clone for one of the driver's *own* background tasks.
    ///
    /// Identical to `Clone` except it does not carry the [`DriverShutdown`]
    /// owner, so a background task can never keep the session alive after the
    /// last caller-held driver is gone. Deliberately private.
    fn task_clone(&self) -> Self {
        let mut c = self.clone();
        c.shutdown_owner = None;
        c
    }

    /// True once the session has been torn down (or is being torn down).
    ///
    /// Callers holding a driver across a reconnect can use this to tell a stale
    /// handle from a live one without poking the socket.
    pub fn is_shut_down(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Explicitly close this session **now**, without waiting for the last
    /// clone to drop.
    ///
    /// Idempotent. Cancels the background loops, sends FIN on the write half and
    /// aborts the tasks as a backstop, which releases the socket. Unlike
    /// [`FanucDriver::disconnect`] this does not need the controller to answer
    /// (or even to be reachable), so it is the right call when the link is
    /// wedged — a `disconnect()` that times out still leaves the socket open.
    ///
    /// Prefer `disconnect().await` first when the link is healthy, so the
    /// controller is told the session is ending, then `shutdown()`.
    pub fn shutdown(&self) {
        match &self.shutdown_owner {
            Some(owner) => owner.tear_down("explicit shutdown"),
            None => {
                // A task-side clone: cannot own teardown, but it can still ask
                // the loops to stop.
                self.cancelled.store(true, Ordering::SeqCst);
                self.cancel_notify.notify_waiters();
            }
        }
    }

    /// Best-effort graceful close: tell the controller, then close regardless.
    ///
    /// Sends `FRC_Disconnect` and waits up to `timeout` for the acknowledgement,
    /// then always calls [`FanucDriver::shutdown`]. A failed or timed-out
    /// acknowledgement is logged, not propagated — the socket still closes,
    /// which is the whole point.
    pub async fn disconnect_and_close(&self, timeout: Duration) {
        match tokio::time::timeout(timeout, self.disconnect()).await {
            Ok(Ok(resp)) => info!("FANUC acknowledged disconnect: {:?}", resp),
            Ok(Err(e)) => warn!("FANUC disconnect failed ({}) — closing the socket anyway", e),
            Err(_) => warn!(
                "FANUC disconnect timed out after {:?} — closing the socket anyway",
                timeout
            ),
        }
        self.shutdown();
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
    /// This also pauses the internal driver queue to prevent sending more
    /// instructions while paused.
    ///
    /// Returns the request ID for tracking this request.
    pub fn send_pause(&self) -> Result<u64, String> {
        // Pause the robot first for immediate pause
        let packet = SendPacket::Command(Command::FrcPause);
        self.send_packet(packet, PacketPriority::Immediate)?;
        
        // Then pause the driver to stop sending more instructions
        let pause_queue_packet = SendPacket::DriverCommand(DriverCommand::Pause);
        self.send_packet(pause_queue_packet, PacketPriority::Immediate)
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
    /// This also unpauses the internal driver queue to resume sending
    /// queued instructions.
    ///
    /// Returns the request ID for tracking this request.
    pub fn send_continue(&self) -> Result<u64, String> {

        
        // Send FrcContinue to the robot first
        let packet = SendPacket::Command(Command::FrcContinue);
        let result = self.send_packet(packet, PacketPriority::Immediate)?;

        // Unpause the internal driver queue to resume sending instructions
        let unpause_queue_packet = SendPacket::DriverCommand(DriverCommand::Unpause);
        self.send_packet(unpause_queue_packet, PacketPriority::Immediate)?;

        Ok(result)
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
        // Make sure we don't lose track of the sequences
        match self.get_status().await {
            Ok(status) => {
                self.sync_sequence_counter(status.next_sequence_id);
            }
            Err(err) => {
                return Err(format!("Failed to get status before continuing: {}", err));
            }
        }
        
        let mut response_rx = self.response_tx.subscribe();
        // send_continue() already unpauses the driver queue
        let _request_id = self.send_continue()?;

        // Wait up to 5 seconds for response
        let response = tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(CommandResponse::FrcContinue(continue_response)) = response {
                    return Ok(continue_response);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for continue response".to_string())??;

        Ok(response)
    }

    /// Pause the running program while allowing the robot to be jogged.
    ///
    /// This is fundamentally different from `pause()`:
    /// - `pause()`: Pauses the robot controller completely. The robot stops and cannot move.
    /// - `program_pause()`: Aborts the RMI_MOVE program but allows the robot to be jogged
    ///   using the teach pendant or other control methods.
    ///
    /// When program_pause is called:
    /// 1. FRC_Abort is sent to terminate the current RMI_MOVE program
    /// 2. In-flight instructions (sent but not completed) are preserved for replay
    /// 3. The internal queue is preserved (not cleared)
    /// 4. The driver enters ProgramPaused state
    ///
    /// Use `program_resume()` to re-initialize and continue execution.
    ///
    /// # Use Case
    /// 1. Running a motion program
    /// 2. See an issue (e.g., potential collision)
    /// 3. Call `program_pause()` - robot stops but can be jogged
    /// 4. Jog robot away from danger using teach pendant
    /// 5. Fix the issue
    /// 6. Jog robot back to position
    /// 7. Call `program_resume()` - program continues from where it left off
    ///
    /// # Returns
    /// * `Ok(())` - Program pause successful
    /// * `Err(String)` - Error if abort failed or driver not in running state
    ///
    /// # Example
    /// ```no_run
    /// # use fanuc_rmi::drivers::FanucDriver;
    /// # async fn example(driver: &FanucDriver) -> Result<(), String> {
    /// // Pause the program to jog the robot
    /// driver.program_pause().await?;
    ///
    /// // ... user jogs robot with teach pendant ...
    ///
    /// // Resume the program
    /// driver.program_resume().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn program_pause(&self) -> Result<(), String> {
        self.log_info("Program pause: snapshotting unexecuted set before abort...").await;

        // Step 1: Snapshot the unexecuted program set (in-flight + queued) and stop
        // transmitting this program's points, BEFORE the abort. Ordering matters:
        // this ProgramPause command is enqueued ahead of the FRC_Abort below on the
        // same FIFO channel, so by the time the abort reaches the controller the send
        // loop has already drained this program's queued points into the replay set —
        // none can leak into the next TP program carrying a stale sequence id. This is
        // driver-internal and is not sent to the controller.
        let pause_packet = SendPacket::DriverCommand(DriverCommand::ProgramPause);
        self.send_packet(pause_packet, PacketPriority::Immediate)?;

        // Step 2: Abort the RMI_MOVE program on the controller.
        self.log_info("Aborting RMI program...").await;
        let abort_response = self.abort().await?;
        if abort_response.error_id != 0 {
            let msg = format!("Program pause abort failed with error: {}", abort_response.error_id);
            self.log_error(&msg).await;
            return Err(msg);
        }

        // Step 3: Re-initialize — fresh TP program with the sequence counter reset to
        // 1 — so the robot can accept jog / rewind moves while paused.
        self.log_info("Re-initializing for jog capability...").await;
        let init_response = self.initialize().await?;
        if init_response.error_id != 0 {
            let msg = format!("Program pause re-initialize failed with error: {}", init_response.error_id);
            self.log_error(&msg).await;
            return Err(msg);
        }

        self.log_info("Program pause complete. Robot can now be jogged.").await;
        Ok(())
    }

    /// Resume a paused program by re-initializing and replaying preserved instructions.
    ///
    /// This should be called after `program_pause()` to resume normal operation.
    ///
    /// When program_resume is called:
    /// 1. FRC_Initialize is sent to create a new RMI_MOVE program
    /// 2. Sequence counter is reset to 1
    /// 3. Preserved in-flight instructions are replayed
    /// 4. The internal queue resumes processing
    /// 5. The driver returns to Running state
    ///
    /// # Returns
    /// * `Ok(())` - Program resume successful
    /// * `Err(String)` - Error if initialize failed or driver not in program-paused state
    ///
    /// # Example
    /// ```no_run
    /// # use fanuc_rmi::drivers::FanucDriver;
    /// # async fn example(driver: &FanucDriver) -> Result<(), String> {
    /// // After program_pause and jogging...
    /// driver.program_resume().await?;
    /// // Robot now continues executing the motion program
    /// # Ok(())
    /// # }
    /// ```
    pub async fn program_resume(&self) -> Result<(), String> {
        self.log_info("Program resume: preparing to continue motion program...").await;

        // Step 1: Get preserved instructions before we clear them
        // Note: We extract the result synchronously and drop the MutexGuard before any await
        let lock_result = self.program_pause_instructions.lock()
            .map(|stored| stored.clone())
            .map_err(|e| format!("Failed to get preserved instructions: {}", e));

        let instructions_to_replay: Vec<Instruction> = match lock_result {
            Ok(instructions) => instructions,
            Err(msg) => {
                self.log_error(&msg).await;
                return Err(msg);
            }
        };

        self.log_info(&format!(
            "Program resume: {} instructions to replay",
            instructions_to_replay.len()
        )).await;

        // Step 2: Abort current RMI_MOVE (used for jogging during pause)
        self.log_info("Aborting jog session...").await;
        let abort_response = self.abort().await?;
        if abort_response.error_id != 0 {
            let msg = format!("Program resume abort failed with error: {}", abort_response.error_id);
            self.log_error(&msg).await;
            return Err(msg);
        }

        // Step 3: Initialize to create fresh RMI_MOVE program with reset sequence counter
        self.log_info("Re-initializing for motion program...").await;
        let init_response = self.initialize().await?;
        if init_response.error_id != 0 {
            let msg = format!("Program resume initialize failed with error: {}", init_response.error_id);
            self.log_error(&msg).await;
            return Err(msg);
        }

        // Step 4: Send ProgramResume command with instructions to replay
        let resume_packet = SendPacket::DriverCommand(DriverCommand::ProgramResume {
            instructions_to_replay,
        });
        self.send_packet(resume_packet, PacketPriority::Immediate)?;

        // Step 5: Clear the stored instructions since we've replayed them
        if let Ok(mut stored) = self.program_pause_instructions.lock() {
            stored.clear();
        }

        self.log_info("Program resume complete. Motion program continuing.").await;
        Ok(())
    }

    /// Send an initialize command to the FANUC controller
    ///
    /// Returns the request ID for tracking this request.
    pub fn send_initialize(&self) -> Result<u64, String> {
        let packet: SendPacket =
            SendPacket::Command(Command::FrcInitialize(FrcInitialize::default()));
        self.send_packet(packet, PacketPriority::Standard)
    }

    /// Send an initialize command reserving a specific set of motion groups.
    ///
    /// `group_mask` is a bitmask (e.g. `GroupMask::GROUP_1 | GroupMask::GROUP_2`
    /// → `0b11`). Use this when a coordinated Group-2 positioner is present so
    /// every subsequent motion packet may carry a G2 block. Returns the request
    /// ID for tracking.
    pub fn send_initialize_with_mask(&self, group_mask: u8) -> Result<u64, String> {
        let packet: SendPacket =
            SendPacket::Command(Command::FrcInitialize(FrcInitialize::from_bits(group_mask)));
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
        self.initialize_with_mask(GroupMask::default().bits()).await
    }

    /// Initialize reserving a specific set of motion groups and wait for the
    /// response. See [`send_initialize_with_mask`](Self::send_initialize_with_mask);
    /// this is the awaiting convenience wrapper used when a coordinated Group-2
    /// positioner is present (`group_mask = 0b11`).
    pub async fn initialize_with_mask(&self, group_mask: u8) -> Result<FrcInitializeResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let _request_id = self.send_initialize_with_mask(group_mask)?;

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

    // ------------------------------------------------------------------
    // FANUC Group-2 (coordinated positioner) support helpers
    //
    // A FANUC positioner is a *second motion group* (Group 2). The RMI motion
    // protocol is documented single-group and its motion packets carry no group
    // field, so coordinated positioner motion is **not** reachable through
    // `FRC_LinearMotion` / `FRC_JointMotion` / etc. The supported architecture is:
    //
    //   1. RUN a controller-resident TP/COORD program by name — `run_tp_program`
    //      (wraps the `FRC_Call` instruction). That program owns the coordinated
    //      motion between the robot and the positioner.
    //   2. PASS PARAMETERS to it through registers / group I/O — `write_register`,
    //      `read_register`, `write_position_register`, `read_position_register`,
    //      and the existing `FRC_WriteGOUT` / `FRC_ReadGIN` commands.
    //   3. READ BACK Group-2 state for visualization — `read_group_joint_angles`,
    //      `read_group_cartesian_position` (the RMI read commands *do* carry a
    //      group field).
    // ------------------------------------------------------------------

    /// Run a controller-resident TP program by name (the coordinated-motion
    /// channel for a Group-2 positioner).
    ///
    /// This wraps the `FRC_Call` instruction. Because `FRC_Call` is an
    /// *instruction*, it flows through the ordered instruction queue and is
    /// assigned a sequence ID by the driver. The program named must already exist
    /// on the controller; for coordinated motion it is a TP/COORD program that
    /// drives both the robot and the positioner (Group 2).
    ///
    /// Returns the request ID; await [`Self::wait_on_request_completion`] with it
    /// to block until the program finishes.
    pub fn run_tp_program(&self, program_name: &str) -> Result<u64, String> {
        let packet = SendPacket::Instruction(Instruction::FrcCall(FrcCall::new(
            0, // sequence_id is assigned by the driver on dispatch
            program_name.to_string(),
        )));
        self.send_packet(packet, PacketPriority::Standard)
    }

    /// Read the joint angles of a specific motion group (1-based group number).
    ///
    /// Use `group = 2` to read a coordinated positioner's axes for visualization.
    /// The RMI `FRC_ReadJointAngles` command carries a `Group` field, so this
    /// works even though the *motion* protocol is single-group.
    pub async fn read_group_joint_angles(
        &self,
        group: u8,
    ) -> Result<FrcReadJointAnglesResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let packet =
            SendPacket::Command(Command::FrcReadJointAngles(FrcReadJointAngles::new(Some(group))));
        let _request_id = self.send_packet(packet, PacketPriority::Standard)?;

        tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(CommandResponse::FrcReadJointAngles(resp)) =
                    response
                {
                    return Ok(resp);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for read joint angles response".to_string())?
    }

    /// Read the Cartesian position of a specific motion group (1-based group
    /// number). Use `group = 2` for a coordinated positioner.
    pub async fn read_group_cartesian_position(
        &self,
        group: u8,
    ) -> Result<FrcReadCartesianPositionResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let packet = SendPacket::Command(Command::FrcReadCartesianPosition(
            FrcReadCartesianPosition::new(Some(group)),
        ));
        let _request_id = self.send_packet(packet, PacketPriority::Standard)?;

        tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(
                    CommandResponse::FrcReadCartesianPosition(resp),
                ) = response
                {
                    return Ok(resp);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for read cartesian position response".to_string())?
    }

    /// Write a numeric register (`R[n]`) — a parameter channel for a
    /// coordinated-motion TP program.
    ///
    /// See [`crate::commands::FrcWriteRegister`] for the portability caveat
    /// (numeric register commands are not in the base RMI set; UNTESTED across
    /// controllers — prefer group I/O or position registers where portability
    /// matters).
    pub async fn write_register(
        &self,
        register_number: u16,
        value: f32,
    ) -> Result<FrcWriteRegisterResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let packet = SendPacket::Command(Command::FrcWriteRegister(FrcWriteRegister::new(
            register_number,
            value,
        )));
        let _request_id = self.send_packet(packet, PacketPriority::Standard)?;

        tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(CommandResponse::FrcWriteRegister(resp)) =
                    response
                {
                    return Ok(resp);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for write register response".to_string())?
    }

    /// Read a numeric register (`R[n]`). See [`Self::write_register`] for the
    /// portability caveat.
    pub async fn read_register(
        &self,
        register_number: u16,
    ) -> Result<FrcReadRegisterResponse, String> {
        let mut response_rx = self.response_tx.subscribe();
        let packet =
            SendPacket::Command(Command::FrcReadRegister(FrcReadRegister::new(register_number)));
        let _request_id = self.send_packet(packet, PacketPriority::Standard)?;

        tokio::time::timeout(Duration::from_secs(5), async {
            while let Ok(response) = response_rx.recv().await {
                if let ResponsePacket::CommandResponse(CommandResponse::FrcReadRegister(resp)) =
                    response
                {
                    return Ok(resp);
                }
            }
            Err("Response channel closed".to_string())
        })
        .await
        .map_err(|_| "Timeout waiting for read register response".to_string())?
    }

    /// Initialize the RMI session reserving a specific set of motion groups.
    ///
    /// Convenience over [`Self::initialize`] for the coordinated case: pass
    /// `GroupMask::GROUP_1 | GroupMask::GROUP_2` to reserve both the robot and a
    /// positioner. **Reserving Group 2 does not make RMI motion packets drive it**
    /// (see [`GroupMask`]); it only declares the groups the session owns so that
    /// group reads and a coordinated TP program can operate.
    pub fn send_initialize_groups(&self, groups: GroupMask) -> Result<u64, String> {
        let packet =
            SendPacket::Command(Command::FrcInitialize(FrcInitialize::new(Some(groups))));
        self.send_packet(packet, PacketPriority::Standard)
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
                let msg = format!("Abort failed: {}", crate::format_error_id(abort_response.error_id));
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
            let msg = format!("Initialize failed: {}", crate::format_error_id(init_response.error_id));
            self.log_error(&msg).await;

            // 7015 is MEMO-015 "Program already exists". The manual attributes it
            // to RMI_MOVE being selected on the teach pendant, but that is not the
            // only cause: it has been reproduced on an R-30iB with every documented
            // precondition satisfied (servo_ready=1, tp_mode=0, rmi_motion_status=0),
            // after a cold reboot, with RMI_MOVE deselected, and it survives
            // FRC_Abort, FRC_Reset, and abort+reset. So suggest the documented
            // remedy without asserting it as the diagnosis.
            if init_response.error_id == 7015 {
                self.log_error(
                    "The controller already holds an RMI_MOVE program it will not replace. \
                     Documented remedy: press SELECT on the TP, choose a program other than \
                     RMI_MOVE, press ENTER, then retry. If RMI_MOVE is already deselected, \
                     this is a controller-side program state that RMI commands cannot clear.",
                )
                .await;
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

        self.raw_hook.sent(&serialized_packet);

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
                let raw_hook = self.raw_hook.clone();

                tokio::spawn(async move {
                    let serialized_packet = match serde_json::to_string(&packet) {
                        Ok(packet_str) => packet_str + "\r\n",
                        Err(e) => {
                            let _ = log_channel.send(format!("ERROR: Failed to serialize command: {}", e));
                            return;
                        }
                    };

                    raw_hook.sent(&serialized_packet);

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
        // Track in-flight instructions for program pause/resume replay
        // Stores (sequence_id, instruction) pairs for instructions sent but not yet completed
        let mut in_flight_instructions: VecDeque<(u32, Instruction)> = VecDeque::new();

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

            // Cooperative teardown: the session is closing, stop pumping.
            if self.cancelled.load(Ordering::SeqCst) {
                break;
            }

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
                        DriverCommand::ProgramPause => {
                            // A pause is the seam between two TP programs. Snapshot the
                            // ENTIRE unexecuted set of the program being aborted so it can
                            // be replayed (re-sequenced from 1) into the next one:
                            //   1. in-flight — transmitted-but-not-completed (earliest first)
                            //   2. queued    — accepted-but-not-yet-transmitted (later points)
                            //
                            // Draining the queued points here is the fix for RMIT-029
                            // ("invalid sequence id"): if they were left in the queue they
                            // would be streamed into the NEXT TP program carrying THIS
                            // program's now-stale sequence ids, which the controller rejects.
                            // program_resume() replays this whole set from seq 1, in order,
                            // so nothing is lost and nothing carries a stale id.
                            //
                            // Only program-motion Instructions are drained; FRC commands and
                            // other non-instruction packets stay in the queue so abort /
                            // initialize / jog / rewind still flow while paused.
                            println!("ProgramPause: transitioning to ProgramPaused state");
                            if let Ok(mut stored) = self.program_pause_instructions.lock() {
                                stored.clear();
                                for (_, instr) in in_flight_instructions.iter() {
                                    stored.push(instr.clone());
                                }
                                let mut drained_from_queue = 0usize;
                                queue.retain(|pkt| match &pkt.packet {
                                    SendPacket::Instruction(instr) => {
                                        stored.push(instr.clone());
                                        drained_from_queue += 1;
                                        false
                                    }
                                    _ => true,
                                });
                                println!(
                                    "  Preserving {} in-flight + {} queued instruction(s) for replay",
                                    in_flight_instructions.len(),
                                    drained_from_queue
                                );
                            }

                            state = DriverState::ProgramPaused;
                            // Reset counter since robot's buffer was cleared by abort
                            in_flight = 0;
                            // Clear local tracking since we've stored them
                            in_flight_instructions.clear();
                        }
                        DriverCommand::ProgramResume { instructions_to_replay } => {
                            // Program resume: Re-queue instructions for replay, then set state to Running
                            println!("ProgramResume: replaying {} instructions", instructions_to_replay.len());

                            // Clear tracked in-flight since we're starting fresh
                            in_flight_instructions.clear();

                            // Re-queue instructions at the front (high priority) so they execute before
                            // any other queued instructions. Insert in reverse order to maintain order.
                            for instr in instructions_to_replay.iter().rev() {
                                let replay_packet = DriverPacket {
                                    priority: PacketPriority::High,
                                    packet: SendPacket::Instruction(instr.clone()),
                                    request_id: REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst),
                                };
                                queue.push_front(replay_packet);
                            }

                            state = DriverState::Running;
                            println!("ProgramResume: state set to Running, queue size: {}", queue.len());
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
                // Remove completed instruction from in-flight tracking
                // Find and remove by sequence_id
                if let Some(pos) = in_flight_instructions.iter().position(|(seq, _)| *seq == pkt.sequence_id) {
                    in_flight_instructions.remove(pos);
                }
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

            // Send packets with backpressure (when Running or ProgramPaused, not when Paused)
            // ProgramPaused allows jog commands and other instructions to be sent
            while in_flight < MAX_IN_FLIGHT && (state == DriverState::Running || state == DriverState::ProgramPaused) {
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
                            Instruction::FrcSplineMotion(ref mut instr) => instr.sequence_id = current_id,
                            Instruction::FrcSplineMotionJRep(ref mut instr) => instr.sequence_id = current_id,
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
                                let seq = instr.get_sequence_id();
                                in_flight += 1;

                                // Only track in-flight instructions when Running (not when ProgramPaused)
                                // Instructions sent during ProgramPaused are jog commands, not program instructions
                                if state == DriverState::Running {
                                    in_flight_instructions.push_back((seq, instr));
                                }

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

            // Maintain consistent loop timing. Racing the sleep against the
            // teardown notify means a `shutdown()` is picked up within a
            // millisecond instead of a full loop interval.
            let elapsed = Instant::now().duration_since(start_time);
            if elapsed < LOOP_INTERVAL {
                tokio::select! {
                    _ = tokio::time::sleep(LOOP_INTERVAL - elapsed) => {}
                    _ = self.cancel_notify.notified() => break,
                }
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

            if self.cancelled.load(Ordering::SeqCst) {
                break;
            }

            // Read without timeout — we want to stay connected indefinitely.
            //
            // But NOT past a teardown: this read is the reason a dropped driver
            // used to hold its socket forever (a wedged controller sends
            // nothing, so the read never returns and the task never released
            // its driver clone). Racing it against the cancel notify is what
            // lets `shutdown()` / `Drop` unblock the reader promptly; the
            // abort backstop in `DriverShutdown` covers the rest.
            let n = tokio::select! {
                biased;
                _ = self.cancel_notify.notified() => break,
                res = reader.read(&mut buf) => match res {
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
                },
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
                tokio::select! {
                    _ = tokio::time::sleep(LOOP_INTERVAL - elapsed) => {}
                    _ = self.cancel_notify.notified() => break,
                }
            }
        }

        // Reached only via teardown (`shutdown()` / driver drop). Returning
        // here drops the read-half guard and, with it, this task's driver
        // clone — which is what actually frees the socket.
        *self.connected.lock().await = false;
        self.log_info("Read loop exiting: session torn down").await;
        Ok(())
    }

    // Extract handling of each line into an async helper:
    async fn process_line(
        &self,
        line: String,
        completed_tx: &broadcast::Sender<CompletedPacketReturnInfo>,
    ) -> Result<(), FrcError> {
        // HOT PATH: Only log at debug level to avoid flooding terminal
        self.log_debug(format!("Received: {}", line)).await;

        let parsed = serde_json::from_str::<ResponsePacket>(&line);
        // Surface the raw frame (and the serde error, if any) to a diagnostic
        // observer before any typed handling. No-op unless a hook is installed.
        self.raw_hook
            .received(line.clone(), parsed.as_ref().err().map(|e| e.to_string()));

        match parsed {
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
                let decoded = crate::extract_and_format_error_id(&line);
                let error_msg = match &decoded {
                    Some(d) => format!("Invalid JSON ({}) [{}]: {}", e, d, line),
                    None => format!("Invalid JSON ({}): {}", e, line),
                };
                self.log_error(error_msg.clone()).await;

                // Broadcast protocol error to subscribers
                let protocol_error = ProtocolError {
                    error_type: "protocol".to_string(),
                    message: match &decoded {
                        Some(d) => format!("Failed to parse robot response [{}]: {}", d, e),
                        None => format!("Failed to parse robot response: {}", e),
                    },
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
                Instruction::FrcSplineMotion(ref mut instr) => {
                    instr.sequence_id = current_id;
                }
                Instruction::FrcSplineMotionJRep(ref mut instr) => {
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
