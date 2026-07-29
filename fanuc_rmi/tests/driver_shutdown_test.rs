//! Regression test for the RMI **connection leak**.
//!
//! Confirmed on real hardware 2026-07-28: one server process held three
//! simultaneous TCP sessions to a controller's RMI data port — one per
//! abandoned connect attempt — because `FanucDriver` had no teardown and its
//! two background tasks each held a clone of the driver (and therefore of the
//! socket halves). A FANUC serves exactly ONE RMI session, so every leaked
//! attempt burned the only one there was, and the controller wedged.
//!
//! These tests stand up a fake controller that performs the two-stage
//! `FRC_Connect` handshake (request port → per-session data port), then assert
//! that the data socket reaches EOF on the server side once the caller's driver
//! goes away — by drop, and by the explicit `shutdown()` path.
//!
//! The fake controller never answers anything after the handshake, which is the
//! wedged-controller case: the reader task is parked on a read that will never
//! return. That is precisely the shape that used to leak.

#![cfg(feature = "driver")]

use std::time::Duration;

use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig, LogLevel};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// A fake controller: one listener for the handshake, one for the data port.
struct FakeController {
    handshake_port: u16,
    /// Yields the accepted data-port socket once the driver dials it.
    data_rx: tokio::sync::oneshot::Receiver<TcpStream>,
}

async fn spawn_fake_controller() -> FakeController {
    let handshake = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let data = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let handshake_port = handshake.local_addr().unwrap().port();
    let data_port = data.local_addr().unwrap().port();

    let (data_tx, data_rx) = tokio::sync::oneshot::channel();

    // Handshake stage: read FRC_Connect, answer with the data port.
    tokio::spawn(async move {
        let (mut sock, _) = handshake.accept().await.unwrap();
        let mut buf = vec![0u8; 1024];
        let _ = sock.read(&mut buf).await.unwrap();
        let reply = format!(
            "{{\"Communication\":\"FRC_Connect\",\"ErrorID\":0,\"PortNumber\":{},\
             \"MajorVersion\":2,\"MinorVersion\":0}}\r\n",
            data_port
        );
        sock.write_all(reply.as_bytes()).await.unwrap();
        // The driver drops its handshake socket immediately after parsing.
    });

    // Data stage: accept and hand the socket back to the test. Never replies —
    // this is the "controller has gone quiet" case the leak lived in.
    tokio::spawn(async move {
        let (sock, _) = data.accept().await.unwrap();
        let _ = data_tx.send(sock);
    });

    FakeController { handshake_port, data_rx }
}

fn config_for(port: u16) -> FanucDriverConfig {
    FanucDriverConfig {
        addr: "127.0.0.1".to_string(),
        port: port.into(),
        max_messages: 30,
        log_level: LogLevel::Error,
    }
}

/// Assert the controller side of the data socket reaches EOF, i.e. the client
/// really released it.
async fn assert_socket_closed(mut server_side: TcpStream, what: &str) {
    // Traffic the driver already sent (e.g. an FRC_GetStatus that is about to
    // time out) may still be in flight; drain it and keep waiting for EOF.
    let drain = async {
        let mut buf = [0u8; 512];
        loop {
            match server_side.read(&mut buf).await {
                Ok(0) => return, // EOF — the session was released.
                Ok(_) => continue,
                Err(e) => {
                    // A reset also means the client let go of the socket.
                    eprintln!("{what}: socket errored ({e}) — treated as closed");
                    return;
                }
            }
        }
    };

    if tokio::time::timeout(Duration::from_secs(10), drain).await.is_err() {
        panic!(
            "{what}: the data socket was STILL OPEN 10s after the driver went away — \
             this is the connection leak (one leaked socket wedges a real controller)"
        );
    }
}

/// Dropping the last caller-held driver must close the data socket and stop the
/// background tasks. Before the `DriverShutdown` owner existed, the driver's own
/// tasks kept it alive forever and this never reached EOF.
#[tokio::test]
async fn dropping_the_driver_closes_the_data_socket() {
    let fake = spawn_fake_controller().await;

    let driver = FanucDriver::connect(config_for(fake.handshake_port))
        .await
        .expect("fake controller handshake should succeed");

    let server_side = tokio::time::timeout(Duration::from_secs(5), fake.data_rx)
        .await
        .expect("driver should dial the data port")
        .expect("data socket handoff");

    assert!(!driver.is_shut_down());
    drop(driver);

    assert_socket_closed(server_side, "drop").await;
}

/// The failed-connect shape that actually leaked on hardware: `connect()`
/// succeeds, a later setup step fails, and the caller discards the driver. The
/// socket must not outlive the caller's `Err` return.
#[tokio::test]
async fn a_failed_post_connect_step_leaves_no_socket_behind() {
    let fake = spawn_fake_controller().await;

    // Stands in for meteorite's `connect() -> startup_sequence()` flow, where
    // the second half fails (or times out) against a faulted controller.
    async fn connect_then_fail(config: FanucDriverConfig) -> Result<FanucDriver, String> {
        let driver = FanucDriver::connect(config).await.map_err(|e| format!("{e:?}"))?;
        // The fake controller never answers, so this is the real timeout path.
        driver
            .get_status()
            .await
            .map_err(|e| format!("startup failed: {e}"))?;
        Ok(driver)
    }

    let server_side_fut = fake.data_rx;
    let result = connect_then_fail(config_for(fake.handshake_port)).await;
    assert!(result.is_err(), "the fake controller never answers get_status");

    let server_side = tokio::time::timeout(Duration::from_secs(5), server_side_fut)
        .await
        .expect("driver should dial the data port")
        .expect("data socket handoff");

    assert_socket_closed(server_side, "failed post-connect step").await;
}

/// The wedged-link escape hatch: `shutdown()` closes the session without
/// needing the controller to answer anything.
#[tokio::test]
async fn explicit_shutdown_closes_a_wedged_session() {
    let fake = spawn_fake_controller().await;

    let driver = FanucDriver::connect(config_for(fake.handshake_port))
        .await
        .expect("fake controller handshake should succeed");

    let server_side = tokio::time::timeout(Duration::from_secs(5), fake.data_rx)
        .await
        .expect("driver should dial the data port")
        .expect("data socket handoff");

    // Hold a clone, as meteorite does (ECS component + async task), to prove
    // shutdown does not depend on every reference having been released.
    let clone = driver.clone();
    driver.shutdown();
    assert!(clone.is_shut_down());

    assert_socket_closed(server_side, "explicit shutdown").await;
}
