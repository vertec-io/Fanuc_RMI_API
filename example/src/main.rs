use std::time::Duration;

use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig, FrcLinearRelative, PacketPriority}, packets::*, Configuration, FrcError, Position, SpeedType, TermType
};

use tokio::time::sleep;
// use fanuc_rmi::{Configuration, Position};
// use std::error::Error;


#[tokio::main]
async fn main() -> Result<(), FrcError > {

    // let driver_settings = FanucDriverConfig::default();

    let driver_settings = FanucDriverConfig{
        addr: "192.168.1.100".to_string(),
        port: 16001,
        max_messages: 30
    };

    println!("going to connect");
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

    let _ = driver.initialize().await;
    sleep(Duration::from_secs(1)).await;

    // // if res.is_err() {
    // //     println!("Already Initialized");
    // //     driver.abort().await?;
    // //     driver.initialize().await?;
    // // };

    // driver.load_gcode().await;
    // let _ = driver.start_program();
    // println!("after startprogram");

    // driver.load_gcode().await;
    driver.add_to_queue(SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
        1,    
        Configuration {
            u_tool_number: 1, u_frame_number: 2, front: 1, up: 1, left: 1, flip: 1, turn4: 1, turn5: 1, turn6: 1,
        },
        Position { x: 30.0, y: 0.0, z: 200.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0,
        },
        SpeedType::MMSec,
        50.0,
        TermType::FINE,
        1,
    ))),PacketPriority::Standard).await;

    // driver.start_program().await?;

    // let dist:f32 = 100.0;
    // let speed: u16 = 31;
    // driver.linear_relative(
    //     1,    
    //     Configuration {
    //         u_tool_number: 1,
    //         u_frame_number: 1,
    //         front: 1,
    //         up: 1,
    //         left: 1,
    //         glip: 1,
    //         turn4: 1,
    //         turn5: 1,
    //         turn6: 1,
    //     },
    //     Position {
    //         x: 0.0,
    //         y: 0.0,
    //         z: dist.clone(),
    //         w: 0.0,
    //         p: 0.0,
    //         r: 0.0,
    //         ext1: 0.0,
    //         ext2: 0.0,
    //         ext3: 0.0,
    //     },
    //     fanuc_rmi::SpeedType::MMSec,
    //     speed.clone(),
    //     fanuc_rmi::TermType::FINE,
    //     1,
    // ).await?;
    
    // driver.linear_relative(
    //     2,    
    //     Configuration {
    //         u_tool_number: 1,
    //         u_frame_number: 1,
    //         front: 1,
    //         up: 1,
    //         left: 1,
    //         glip: 1,
    //         turn4: 1,
    //         turn5: 1,
    //         turn6: 1,
    //     },
    //     Position {
    //         x: 30.0,
    //         y: dist.clone(),
    //         z: 0.0,
    //         w: 0.0,
    //         p: 0.0,
    //         r: 0.0,
    //         ext1: 0.0,
    //         ext2: 0.0,
    //         ext3: 0.0,
    //     },
    //     fanuc_rmi::SpeedType::MMSec,
    //     speed.clone(),
    //     fanuc_rmi::TermType::FINE,
    //     1,
    // ).await?;

    // driver.linear_relative(
    //     3,    
    //     Configuration {
    //         u_tool_number: 1,
    //         u_frame_number: 1,
    //         front: 1,
    //         up: 1,
    //         left: 1,
    //         glip: 1,
    //         turn4: 1,
    //         turn5: 1,
    //         turn6: 1,
    //     },
    //     Position {
    //         x: 0.0,
    //         y: 0.0,
    //         z: -dist.clone(),
    //         w: 0.0,
    //         p: 0.0,
    //         r: 0.0,
    //         ext1: 0.0,
    //         ext2: 0.0,
    //         ext3: 0.0,
    //     },
    //     fanuc_rmi::SpeedType::MMSec,
    //     speed.clone(),
    //     fanuc_rmi::TermType::FINE,
    //     1,
    // ).await?;

    // driver.linear_relative(
    //     4,    
    //     Configuration {
    //         u_tool_number: 1,
    //         u_frame_number: 1,
    //         front: 1,
    //         up: 1,
    //         left: 1,
    //         glip: 1,
    //         turn4: 1,
    //         turn5: 1,
    //         turn6: 1,
    //     },
    //     Position {
    //         x: -30.0,
    //         y: -dist.clone(),
    //         z: 0.0,
    //         w: 0.0,
    //         p: 0.0,
    //         r: 0.0,
    //         ext1: 0.0,
    //         ext2: 0.0,
    //         ext3: 0.0,
    //     },
    //     fanuc_rmi::SpeedType::MMSec,
    //     speed.clone(),
    //     fanuc_rmi::TermType::FINE,
    //     1,
    // ).await?;


    // println!("sleeping to wait for packet responses");
    // // sleep(Duration::from_millis(500)).await;


    sleep(Duration::from_secs(4)).await;
    driver.abort().await?;
    sleep(Duration::from_secs(1)).await;
    driver.disconnect().await?;
    // sleep(Duration::from_millis(5)).await;


    Ok(())
}

