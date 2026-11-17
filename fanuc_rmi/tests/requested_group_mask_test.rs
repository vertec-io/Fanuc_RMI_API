//! Recovery regressions using a real TCP connection to an isolated fake controller.
#![cfg(feature = "driver")]

use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig},
    instructions::FrcLinearMotion,
    packets::{Instruction, PacketPriority, SendPacket},
    Configuration, Position, SpeedType, TermType,
};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::mpsc,
    time::timeout,
};

async fn fake_controller() -> (FanucDriver, mpsc::UnboundedReceiver<Value>) {
    let handshake = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let data = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = handshake.local_addr().unwrap().port();
    let data_port = data.local_addr().unwrap().port();
    let (tx, rx) = mpsc::unbounded_channel();
    tokio::spawn(async move {
        let (sock, _) = handshake.accept().await.unwrap();
        let mut reader = BufReader::new(sock);
        let mut line = String::new();
        reader.read_line(&mut line).await.unwrap();
        let reply = json!({"Communication":"FRC_Connect","ErrorID":0,
            "PortNumber":data_port,"MajorVersion":2,"MinorVersion":0});
        reader
            .get_mut()
            .write_all(format!("{reply}\r\n").as_bytes())
            .await
            .unwrap();
        let (sock, _) = data.accept().await.unwrap();
        let mut reader = BufReader::new(sock);
        loop {
            line.clear();
            if reader.read_line(&mut line).await.unwrap() == 0 {
                break;
            }
            let packet: Value = serde_json::from_str(line.trim()).unwrap();
            tx.send(packet.clone()).unwrap();
            let reply = match packet["Command"].as_str() {
                Some("FRC_Initialize") => Some(json!({"Command":"FRC_Initialize",
                    "ErrorID":0,"GroupMask":packet["GroupMask"]})),
                Some("FRC_Abort") => Some(json!({"Command":"FRC_Abort","ErrorID":0})),
                _ => None, // Motions deliberately stay in flight until an abort.
            };
            if let Some(reply) = reply {
                reader
                    .get_mut()
                    .write_all(format!("{reply}\r\n").as_bytes())
                    .await
                    .unwrap();
            }
        }
    });
    let driver = FanucDriver::connect(FanucDriverConfig {
        addr: "127.0.0.1".into(),
        port: port.into(),
        ..Default::default()
    })
    .await
    .unwrap();
    (driver, rx)
}

async fn next_packet(rx: &mut mpsc::UnboundedReceiver<Value>) -> Value {
    timeout(Duration::from_secs(5), rx.recv())
        .await
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn requested_mask_survives_reinitialization_and_is_shared_by_clones() {
    let (driver, mut rx) = fake_controller().await;
    assert_eq!(driver.requested_group_mask(), 1);
    assert_eq!(driver.session_group_mask(), 0);
    driver.initialize().await.unwrap();
    assert_eq!(next_packet(&mut rx).await["GroupMask"], 1);
    let clone = driver.clone();
    clone.set_requested_group_mask(3);
    clone.set_requested_group_mask(0);
    assert_eq!(driver.requested_group_mask(), 3);
    assert_eq!(driver.session_group_mask(), 1);
    for _ in 0..2 {
        driver.initialize().await.unwrap();
        assert_eq!(next_packet(&mut rx).await["GroupMask"], 3);
        assert_eq!(clone.session_group_mask(), 3);
    }
    driver.abort().await.unwrap();
    assert_eq!(next_packet(&mut rx).await["Command"], "FRC_Abort");
    assert_eq!(clone.session_group_mask(), 0);
    assert_eq!(clone.requested_group_mask(), 3);
    clone.initialize().await.unwrap();
    assert_eq!(next_packet(&mut rx).await["GroupMask"], 3);
    // An explicit one-time override must not rewrite the persistent selection.
    driver.initialize_with_mask(1).await.unwrap();
    assert_eq!(next_packet(&mut rx).await["GroupMask"], 1);
    clone.initialize().await.unwrap();
    assert_eq!(next_packet(&mut rx).await["GroupMask"], 3);
    driver.program_pause().await.unwrap();
    assert_eq!(next_packet(&mut rx).await["Command"], "FRC_Abort");
    assert_eq!(next_packet(&mut rx).await["GroupMask"], 3);
    driver.program_resume().await.unwrap();
    assert_eq!(next_packet(&mut rx).await["Command"], "FRC_Abort");
    assert_eq!(next_packet(&mut rx).await["GroupMask"], 3);
    driver.shutdown();
}

fn motion() -> SendPacket {
    SendPacket::Instruction(Instruction::FrcLinearMotion(FrcLinearMotion::new(
        0,
        Configuration::default(),
        Position::default(),
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        1,
    )))
}

async fn check_replay(abort_first: bool) {
    let (driver, mut rx) = fake_controller().await;
    driver.initialize().await.unwrap();
    next_packet(&mut rx).await;
    driver
        .send_packet(motion(), PacketPriority::Standard)
        .unwrap();
    assert_eq!(
        next_packet(&mut rx).await["Instruction"],
        "FRC_LinearMotion"
    );
    if abort_first {
        driver.abort().await.unwrap();
        assert_eq!(next_packet(&mut rx).await["Command"], "FRC_Abort");
    }
    driver.program_pause().await.unwrap();
    assert_eq!(next_packet(&mut rx).await["Command"], "FRC_Abort");
    assert_eq!(next_packet(&mut rx).await["Command"], "FRC_Initialize");
    driver.program_resume().await.unwrap();
    assert_eq!(next_packet(&mut rx).await["Command"], "FRC_Abort");
    assert_eq!(next_packet(&mut rx).await["Command"], "FRC_Initialize");
    if abort_first {
        assert!(
            timeout(Duration::from_millis(250), rx.recv())
                .await
                .is_err(),
            "motion discarded by abort must never be replayed"
        );
    } else {
        let replay = next_packet(&mut rx).await;
        assert_eq!(replay["Instruction"], "FRC_LinearMotion");
        assert_eq!(replay["SequenceID"], 1);
    }
    driver.shutdown();
}

#[tokio::test]
async fn abort_then_pause_resume_does_not_replay_discarded_motion() {
    check_replay(true).await;
}

#[tokio::test]
async fn ordinary_pause_resume_still_replays_unfinished_motion() {
    check_replay(false).await;
}
