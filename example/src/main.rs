use std::time::Duration;
use tokio::time::sleep;
use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig}, instructions::FrcLinearRelative, packets::*, Configuration, FrcError, Position, SpeedType, TermType
};

#[tokio::main]
async fn main() -> Result<(), FrcError > {

    let driver_settings = FanucDriverConfig{
        addr: "127.0.0.1".to_string(),
        port: 16001,
        max_messages: 30
    };

    let driver = FanucDriver::connect(driver_settings.clone()).await.unwrap();
    tokio::time::sleep(Duration::from_secs(1)).await;
    driver.initialize();
    tokio::time::sleep(Duration::from_secs(1)).await;





    match driver.send_command(
        SendPacket::Instruction(Instruction::FrcLinearRelative(
            FrcLinearRelative::new(
                0,
                Configuration {
                    front: 1,
                    up: 1,
                    left: 0,
                    turn4: 0,
                    turn5: 0,
                    turn6: 0,
                },
                Position {
                    x: 10.0,
                    y: 0.0,
                    z: 0.0,
                    w: 0.0,
                    p: 0.0,
                    r: 0.0,
                    ext1: 0.0,
                    ext2: 0.0,
                    ext3: 0.0,
                },
                SpeedType::MMSec,
                30.0,
                TermType::FINE,
                1,
            )
        )),
        PacketPriority::Standard,
    ) {
        Ok(_id) => {
            // driver.wait_on_command_completion(_id).await
        },
        Err(e) => return Err(FrcError::FailedToSend(e)),
    };





    driver.abort();
    driver.disconnect().await;
    // this main needs to stay in scope long enough for the background threads to send the data. if it goes out of scope before then the background processes get terminated
    sleep(Duration::from_secs(1000)).await;
    Ok(())
}

