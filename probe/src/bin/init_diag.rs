//! `init_diag` — a focused diagnostic for `FRC_Initialize` failing with
//! `MEMO-015 Program already exists (7015)`.
//!
//! The capability sweep runs its lifecycle probes *after* the initialize
//! discovery, so the one sequence that matters here — clear the controller,
//! THEN initialize — is never actually exercised. This binary does exactly
//! that, and asks the controller for its own error detail via `FRC_ReadError`
//! rather than making us infer from a single error id.
//!
//! Strictly non-motion: it sends only GetStatus / ReadError / Abort / Reset /
//! Initialize. Nothing here commands an axis.
//!
//! ```text
//! cargo run -p fanuc_probe --bin init_diag -- --addr 192.168.0.100:16001
//! ```

use std::time::Duration;

use fanuc_rmi::commands::{FrcInitialize, FrcReadError};
use fanuc_rmi::drivers::{FanucDriver, FanucDriverConfig, LogLevel, RawDirection, RawFrameHook};
use fanuc_rmi::packets::{Command, PacketPriority, SendPacket};
use fanuc_rmi::GroupMask;

/// Send one packet and print whatever comes back, matched on the response's
/// `Command` name so an unrelated async frame can't be mistaken for the reply.
async fn send(driver: &FanucDriver, label: &str, packet: SendPacket, expect: &str) {
    send_within(driver, label, packet, expect, Duration::from_secs(5)).await
}

/// `FRC_Abort` on real hardware can take far longer than an ordinary command to
/// answer, and sending anything else while it is outstanding wedges the session
/// into a run of `RMIT-027 Wait for Command Done`. So abort gets its own,
/// much longer budget rather than the default.
async fn send_within(driver: &FanucDriver, label: &str, packet: SendPacket, expect: &str, budget: Duration) {
    println!("\n--- {label} ---");
    let mut rx = driver.response_tx.subscribe();
    if let Err(e) = driver.send_packet(packet, PacketPriority::Standard) {
        println!("  send failed: {e}");
        return;
    }
    let deadline = tokio::time::Instant::now() + budget;
    loop {
        match tokio::time::timeout_at(deadline, rx.recv()).await {
            Ok(Ok(resp)) => {
                let text = format!("{resp:?}");
                // The raw hook already printed the wire frame; this is the typed
                // parse, which is what tells us the driver understood it.
                if text.contains(expect) {
                    println!("  typed: {text}");
                    return;
                }
            }
            Ok(Err(e)) => {
                println!("  channel error: {e}");
                return;
            }
            Err(_) => {
                println!("  TIMEOUT waiting for {expect}");
                return;
            }
        }
    }
}

async fn status(driver: &FanucDriver, when: &str) {
    send(driver, &format!("FRC_GetStatus ({when})"), SendPacket::Command(Command::FrcGetStatus), "FrcGetStatus").await;
}

async fn read_error(driver: &FanucDriver, when: &str) {
    send(
        driver,
        &format!("FRC_ReadError ({when})"),
        SendPacket::Command(Command::FrcReadError(FrcReadError::new(Some(1)))),
        "FrcReadError",
    )
    .await;
}

async fn initialize(driver: &FanucDriver, mask: GroupMask, label: &str) {
    send(
        driver,
        &format!("FRC_Initialize {label}"),
        SendPacket::Command(Command::FrcInitialize(FrcInitialize::new(Some(mask)))),
        "FrcInitialize",
    )
    .await;
}

#[tokio::main]
async fn main() {
    let addr = std::env::args()
        .skip_while(|a| a != "--addr")
        .nth(1)
        .unwrap_or_else(|| "192.168.0.100:16001".to_string());
    let (host, port) = addr.split_once(':').expect("addr must be host:port");
    let port: u32 = port.parse().expect("port must be a number");

    println!("init_diag → {addr}  (non-motion: GetStatus / ReadError / Abort / Reset / Initialize)");

    let hook = RawFrameHook::new(|f| {
        let arrow = match f.direction {
            RawDirection::Sent => "  >>",
            RawDirection::Received => "  <<",
        };
        println!("{arrow} {}", f.payload);
        if let Some(n) = f.note {
            println!("     note: {n}");
        }
    });

    let config = FanucDriverConfig {
        addr: host.to_string(),
        port,
        max_messages: 30,
        log_level: LogLevel::Warn,
    };
    let driver = match FanucDriver::connect_with_raw_hook(config, hook).await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("connect failed: {e}");
            std::process::exit(1);
        }
    };

    // Baseline: what does the controller think its state is before we touch it?
    status(&driver, "baseline").await;

    // The typed verdict, which is what callers should branch on.
    match driver.get_status().await {
        Ok(s) => {
            println!("\n=== readiness verdict ===");
            println!("  teach pendant disabled : {}", s.teach_pendant_disabled());
            println!("  RMI interface running  : {}", s.rmi_interface_running());
            println!("  RMI_MOVE program       : {}", s.program_state());
            println!("  readiness              : {:?}", s.readiness());
            println!("  -> {}", s.readiness().explain());
        }
        Err(e) => println!("readiness check failed: {e}"),
    }
    read_error(&driver, "baseline — what is the controller's own last error?").await;

    // Reproduce the failure so the sequences below have something to compare to.
    initialize(&driver, GroupMask::GROUP_1, "G1 (baseline, expect 7015)").await;
    read_error(&driver, "immediately after the failed Initialize").await;

    // Sequence A — Abort, then Initialize. The doc only aborts when
    // RMIMotionStatus != 0; ours reads 0, so this path has never run.
    send_within(&driver, "FRC_Abort (60s budget)", SendPacket::Command(Command::FrcAbort), "FrcAbort", Duration::from_secs(60)).await;
    status(&driver, "after Abort").await;
    initialize(&driver, GroupMask::GROUP_1, "G1 after Abort").await;

    // Sequence B — Reset, then Initialize. The sweep sends Reset only *after*
    // its initialize discovery, so Reset→Initialize is untested.
    send_within(&driver, "FRC_Reset (30s budget)", SendPacket::Command(Command::FrcReset), "FrcReset", Duration::from_secs(30)).await;
    status(&driver, "after Reset").await;
    initialize(&driver, GroupMask::GROUP_1, "G1 after Reset").await;

    // Sequence C — Abort + Reset together, then both masks.
    send_within(&driver, "FRC_Abort (again, 60s budget)", SendPacket::Command(Command::FrcAbort), "FrcAbort", Duration::from_secs(60)).await;
    send_within(&driver, "FRC_Reset (again, 30s budget)", SendPacket::Command(Command::FrcReset), "FrcReset", Duration::from_secs(30)).await;
    status(&driver, "after Abort+Reset").await;
    initialize(&driver, GroupMask::GROUP_1, "G1 after Abort+Reset").await;
    initialize(&driver, GroupMask::GROUP_1 | GroupMask::GROUP_2, "G1|G2 after Abort+Reset").await;

    read_error(&driver, "final").await;
    status(&driver, "final").await;

    let _ = driver.disconnect().await;
    println!("\ndone.");
}
