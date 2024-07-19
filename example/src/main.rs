use std::time::Duration;

use fanuc_rmi::{drivers::{FanucDriver, FanucDriverConfig}, instructions::FrcLinearRelative, Configuration, FrcError, Instruction, PacketEnum, Position, SpeedType, TermType};
use tokio::time::sleep;
// use fanuc_rmi::{Configuration, Position};
// use std::error::Error;


#[tokio::main]
async fn main() -> Result<(), FrcError > {

    let driver_settings = FanucDriverConfig::default();
    let driver = FanucDriver::connect(driver_settings.clone()).await;

    // let mut driver = FanucDriver::new("192.168.1.100".to_string(), 16001);

    let mut driver = match driver {
        Ok(driver) => {
            println!("Connected successfully");
            driver
        },
        Err(e) => {
            println!("Failed to connect to {:?} : {}",driver_settings, e);
            return Err(e)
        },
    };

    // driver.get_status().await?;
    let res = driver.initialize().await;
    if res.is_err() {
        println!("Already Initialized");
        driver.abort().await?;
        driver.initialize().await?;
    };

    // driver.load_gcode().await;
    // let _ = driver.start_program();
    // println!("after startprogram");

    driver.load_gcode().await;
    // driver.add_to_queue(PacketEnum::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
    //     4,    
    //     Configuration { u_tool_number: 1, u_frame_number: 1, front: 1, up: 1, left: 1, glip: 1, turn4: 1, turn5: 1, turn6: 1,
    //     },
    //     Position { x: -30.0, y: -100.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0,
    //     },
    //     SpeedType::MMSec,
    //     30,
    //     TermType::FINE,
    //     1,
    // )))).await;

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
    println!("sleeping to wait for background task");
    sleep(Duration::from_millis(500)).await;

    driver.abort().await?;
    driver.disconnect().await?;
    sleep(Duration::from_millis(5)).await;


    Ok(())
}

