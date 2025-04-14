use std::time::Duration;

use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig, FrcLinearMotion, FrcLinearRelative, PacketPriority}, packets::*, Configuration, FrcError, Position, SpeedType, TermType
};

use tokio::time::sleep;
// use fanuc_rmi::{Configuration, Position};
// use std::error::Error;


#[tokio::main]
async fn main() -> Result<(), FrcError > {

    let driver_settings = FanucDriverConfig{
        addr: "127.0.0.1".to_string(),
        port: 16001,
        max_messages: 30
    };
    let driver = FanucDriver::connect(driver_settings.clone()).await;


    let driver = match driver {
        Ok(driver) => {
            println!("Connected successfully");
            driver
        },
        Err(e) => {
            println!("Failed to connect to {:?} : {}",driver_settings, e);
            return Err(e)
        },
    };

    let res = driver.initialize().await;

    driver.add_to_queue(SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
        12,    
        Configuration { u_tool_number: 1, u_frame_number: 2, front: 1, up: 1, left: 1, flip: 1, turn4: 1, turn5: 1, turn6: 1,
        },
        Position { x: 0.0, y: 0.0, z: -10.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0,
        },
        SpeedType::MMSec,
        30.0,
        TermType::FINE,
        1,
    ))),
    PacketPriority::Standard
).await;


    sleep(Duration::from_secs(100)).await;
    driver.abort().await?;
    sleep(Duration::from_secs(10)).await;
    driver.disconnect().await?;


    Ok(())
}

