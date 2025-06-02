use std::time::Duration;
use tokio::time::sleep;
use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig, FrcLinearRelative, PacketPriority}, packets::*, Configuration, FrcError, Position, SpeedType, TermType
};

#[tokio::main]
async fn main() -> Result<(), FrcError > {

    let driver_settings = FanucDriverConfig{
        addr: "127.0.0.1".to_string(),
        port: 16001,
        max_messages: 30
    };
    let mut driver = FanucDriver::connect(driver_settings.clone()).await.unwrap();
    driver.disconnect().await;
    driver = FanucDriver::connect(driver_settings.clone()).await.unwrap();
    driver.initialize().await;


    let mut x = 1;
    while x < 40
     {
        x = driver.send_command(SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
            0,    
            Configuration { u_tool_number: 1, u_frame_number: 2, front: 1, up: 1, left: 1, flip: 1, turn4: 1, turn5: 1, turn6: 1,
            },
            Position { x: 0.0, y: 0.0, z: -10.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0,
            },
            SpeedType::MMSec,
            30.0,
            TermType::FINE,
            1,
        ))), PacketPriority::Standard).await;
        println!("{}", x);
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    driver.send_command(SendPacket::DriverCommand(DriverCommand::Pause), PacketPriority::Standard).await;
    tokio::time::sleep(Duration::from_secs(10)).await;
    // sleep(Duration::from_secs(10)).await;
    

    driver.send_command(SendPacket::DriverCommand(DriverCommand::Unpause), PacketPriority::Standard).await;

    while x < 80
     {
        x = driver.send_command(SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
            0,    
            Configuration { u_tool_number: 1, u_frame_number: 2, front: 1, up: 1, left: 1, flip: 1, turn4: 1, turn5: 1, turn6: 1,
            },
            Position { x: 0.0, y: 0.0, z: -10.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0,
            },
            SpeedType::MMSec,
            30.0,
            TermType::FINE,
            1,
        ))), PacketPriority::Standard).await;
        println!("{}", x);
    }





    driver.abort().await;
    driver.disconnect().await;
    // this main needs to stay in scope long enough for the background threads to send the data. if it goes out of scope before then the background processes get terminated
    sleep(Duration::from_secs(1000)).await;
    Ok(())
}

