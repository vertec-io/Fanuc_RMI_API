use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

use std::{sync::Arc, time::Duration};
use tokio::{ net::TcpStream, sync::Mutex, time::sleep};
use tokio::io::{ AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf, split};
use std::collections::VecDeque;

// use crate::ResponsePacket;
pub use crate::{packets::*, FanucErrorCode};
pub use crate::instructions::*;
pub use crate::commands::*;
pub use crate::{Configuration, Position, SpeedType, TermType, FrcError };

pub enum PacketPriority{
    Low,
    Standard,
    High,
    Immediate,
}
pub struct DriverPacket {
    priority: PacketPriority,
    packet: SendPacket,
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct  FanucDriverConfig {
    pub addr: String,
    pub port: u32,
    pub max_messages: usize,
}

impl Default for FanucDriverConfig {
    fn default() -> Self {
        let addr = "127.0.0.1".to_string(); // Change if the server is running on a different machine
        let port = 16001;
        let max_messages = 30;
        Self {
            addr,
            port,
            max_messages,
        }
    }
}

#[derive( Debug, Clone)]
pub struct FanucDriver {
    pub config: FanucDriverConfig,
    pub messages: Arc<Mutex<VecDeque<String>>>,
    pub latest_sequence: Arc<Mutex<u32>>,
    write_half: Arc<Mutex<WriteHalf<TcpStream>>>,
    read_half: Arc<Mutex<ReadHalf<TcpStream>>>,
    queue_tx: mpsc::Sender<DriverPacket>
}

impl FanucDriver {

    /// connect is a constructor that a config to and it attempts connection and if connection failed returns error and if not returns a driver with tcp connection to a fanuc controllor(the actual robot hardware)
    /// connection calls the start program function that spins up 2 async tasks. one to handle packets sent to the robot one to handle recieving the responses.
    pub async fn connect(config: FanucDriverConfig) -> Result<FanucDriver, FrcError> {
        let init_addr = format!("{}:{}",&config.addr, &config.port);
        let mut stream = connect_with_retries(&init_addr, 3).await?;

        // Create a connection packet
        let packet = Communication::FrcConnect {};
        
        let packet = match serde_json::to_string(&packet) {
            Ok(serialized_packet) => serialized_packet + "\r\n",
            Err(_) => return Err(FrcError::Serialization("Communication: Connect packet didnt serialize correctly".to_string())),
        };

        if let Err(e) = stream.write_all(packet.as_bytes()).await {
            let err = FrcError::FailedToSend(format!("{}",e));
            // self.log_message(err.to_string()).await;
            return Err(err);
        }  

        let mut buffer = vec![0; 2048];

        let n: usize = match stream.read(&mut buffer).await{
            Ok(n)=> n,
            Err(e) => {
                let err = FrcError::FailedToRecieve(format!("{}",e));
                return Err(err);
            }
        };

        if n == 0 {
            let e = FrcError::Disconnected();
            return Err(e);
        }

        let response = String::from_utf8_lossy(&buffer[..n]);
        
        println!("Sent: {}\nReceived: {}", &packet, &response);

        let res: CommunicationResponse  = match serde_json::from_str::<CommunicationResponse>(&response) {
            Ok(response_packet) => response_packet,
            Err(e) => {
                let e = FrcError::Serialization(format!("Could not parse response: {}", e));
                return Err(e)
            }
        };

        let mut new_port = 0;
        match res {
            CommunicationResponse::FrcConnect(res) => new_port = res.port_number,
            _ => ()
        };

        drop(stream);
        let init_addr = format!("{}:{}",config.addr, new_port);
        let stream = connect_with_retries(&init_addr, 3).await?;        

        let (read_half, write_half) = split(stream);
        let read_half = Arc::new(Mutex::new(read_half));
        let write_half = Arc::new(Mutex::new(write_half));
        let messages: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let mut msg = messages.lock().await;
        let (queue_tx, queue_rx) = mpsc::channel::<DriverPacket>(100);

        let latest_sequence = Arc::new(Mutex::new(0));
        msg.push_back("Connected".to_string());
        drop(msg);
        let driver = Self {
            config,
            messages,
            latest_sequence,
            write_half,
            read_half,
            queue_tx,
        };
        
        let mut driver_clone = driver.clone();
        tokio::spawn(async move {
            match driver_clone.start_program(queue_rx).await {
                Ok(_) => println!("Program started successfully"),
                Err(e) => eprintln!("Failed to start program: {:?}", e),
            }
        });

        Ok(driver)
    }


    //log message is an interface to print info needed and log it for api use
    async fn log_message<T: Into<String>>(&self, message:T){
        let message = message.into();
        let messages = self.messages.clone();
        let mut messages = messages.lock().await;

        #[cfg(feature="logging")]
        println!("{}", &message);

        while messages.len() >= self.config.max_messages {
            messages.pop_front();
        }
        messages.push_back(message);
    }


    //this is mostly depricated
    pub async fn initialize(&self) -> Result<(), FrcError> {

        let packet: SendPacket =  SendPacket::Command(Command::FrcInitialize(FrcInitialize::default()));
        
        // let packet = match serde_json::to_string(&packet) {
        //     Ok(serialized_packet) => serialized_packet + "\r\n",
        //     Err(_) => return Err(FrcError::Serialization("Initalize packet didnt serialize correctly".to_string())),
        // };

        self.add_to_queue(packet, PacketPriority::Standard).await;
        // if let Err(e) = self.send_packet(packet.clone()).await {
        //     self.log_message(e.to_string()).await;
        //     return Err(e);
        // }; 
        return Ok(());

        // let response = self.recieve::<CommandResponse>().await?;

        // if let CommandResponse::FrcInitialize(ref res) = response {
        //     if res.error_id != 0 {
        //         self.log_message(format!("Error ID: {}", res.error_id)).await;
        //         let error_code = FanucErrorCode::try_from(res.error_id).unwrap_or(FanucErrorCode::UnrecognizedFrcError);
        //         return Err(FrcError::FanucErrorCode(error_code));
        //     }
        // };

 
        // Ok(())

    }
    
    
    pub async fn abort(&self) -> Result<(), FrcError> {

        let packet = SendPacket::Command(Command::FrcAbort {});
        self.add_to_queue(packet, PacketPriority::Standard).await;

        // let packet = match serde_json::to_string(&packet) {
        //     Ok(serialized_packet) => serialized_packet + "\r\n",
        //     Err(_) => return Err(FrcError::Serialization("Abort packet didnt serialize correctly".to_string())),
        // };

        // self.send_packet(packet.clone()).await?;
        // let response = self.recieve::<CommandResponse>().await?;

        // if let CommandResponse::FrcAbort(ref res) = response {
        //     if res.error_id != 0 {
        //         self.log_message(format!("Error ID: {}", res.error_id)).await;
        //         let error_code = FanucErrorCode::try_from(res.error_id).unwrap_or(FanucErrorCode::UnrecognizedFrcError);
        //         return Err(FrcError::FanucErrorCode(error_code));            
        //     }
        // }
        Ok(())
    }

    // pub async fn get_status(&self) -> Result<(), FrcError> {

    //     let packet = Command::FrcGetStatus {};
        
    //     let packet = match serde_json::to_string(&packet) {
    //         Ok(serialized_packet) => serialized_packet + "\r\n",
    //         Err(_) => return Err(FrcError::Serialization("get_status packet didnt serialize correctly".to_string())),
    //     };

    //     self.send_packet(packet.clone()).await?;
    //     let response = self.recieve::<CommandResponse>().await?;        
    //     if let CommandResponse::FrcGetStatus(ref res) = response {
    //         if res.error_id != 0 {
    //             self.log_message(format!("Error ID: {}", res.error_id)).await;
    //             let error_code = FanucErrorCode::try_from(res.error_id).unwrap_or(FanucErrorCode::UnrecognizedFrcError);
    //             return Err(FrcError::FanucErrorCode(error_code)); 
    //         }
    //     }
    //     Ok(())
    // }

    pub async fn disconnect(&self) -> Result<(), FrcError> {

        let packet = Communication::FrcDisconnect {};
        
        let packet = match serde_json::to_string(&packet) {
            Ok(serialized_packet) => serialized_packet + "\r\n",
            Err(_) => return Err(FrcError::Serialization("Disconnect packet didnt serialize correctly".to_string())),
        };

        self.send_packet(packet.clone()).await?;
        // let response = self.recieve::<CommunicationResponse>().await?;        
        // if let CommunicationResponse::FrcDisconnect(ref res) = response {
        //     if res.error_id != 0 {
        //         self.log_message(format!("Error ID: {}", res.error_id)).await;
        //         let error_code = FanucErrorCode::try_from(res.error_id).unwrap_or(FanucErrorCode::UnrecognizedFrcError);
        //         return Err(FrcError::FanucErrorCode(error_code));             
        //     }
        // }

        Ok(())

    }

    async fn send_packet(&self, packet: String) -> Result<(), FrcError> {      
            let mut stream = self.write_half.lock().await;

            if let Err(e) = stream.write_all(packet.as_bytes()).await {
                let err = FrcError::FailedToSend(format!("{}",e));
                self.log_message(err.to_string()).await;
                return Err(err);
            }            
            self.log_message(format!("Sent: {}", packet)).await;
            Ok(())
    }

    async fn recieve<T>(&self) -> Result<T, FrcError>
        where
            T: for<'a> Deserialize<'a> + std::fmt::Debug,
        {
            
            let mut buffer = vec![0; 2048];
            let mut stream = self.read_half.lock().await;

            let n: usize = match stream.read(&mut buffer).await{
                Ok(n)=> n,
                Err(e) => {
                    let err = FrcError::FailedToRecieve(format!("{}",e));
                    self.log_message(err.to_string()).await;
                    return Err(err);
                }
            };


            if n == 0 {
                let e = FrcError::Disconnected();
                self.log_message(e.to_string()).await;
                return Err(e);
            }

            let response = String::from_utf8_lossy(&buffer[..n]);

            self.log_message(format!("Received: {}", &response)).await;

            // Parse JSON response
            match serde_json::from_str::<T>(&response) {
                Ok(response_packet) => Ok(response_packet),
                Err(e) => {
                    let e = FrcError::Serialization(format!("Could not parse response: {}", e));
                    self.log_message(e.to_string()).await;
                    return Err(e);
                }
            }
        }

    pub async fn linear_relative(
        &self,
        sequence_id: u32,    
        configuration: Configuration,
        position: Position,
        speed_type: SpeedType,
        speed: f64,
        term_type: TermType,
        term_value: u8,

    ) -> Result<(), FrcError> {
        let packet = Instruction::FrcLinearRelative(FrcLinearRelative::new(
            sequence_id,    
            configuration,
            position,
            speed_type,
            speed,
            term_type,
            term_value,
        ));
        
        let packet = match serde_json::to_string(&packet) {
            Ok(serialized_packet) => serialized_packet + "\r\n",
            Err(_) => return Err(FrcError::Serialization("linear motion packet didnt serialize correctly".to_string())),
        };

        self.send_packet(packet.clone()).await?;
        let response = self.recieve::<InstructionResponse>().await?;
        if let InstructionResponse::FrcLinearRelative(ref res) = response {
            if res.error_id != 0 {
                self.log_message(format!("Error ID: {}", res.error_id)).await;
                let error_code = FanucErrorCode::try_from(res.error_id).unwrap_or(FanucErrorCode::UnrecognizedFrcError);
                return Err(FrcError::FanucErrorCode(error_code)); 
            }
        }
        Ok(())

    }

    pub async fn start_program(&mut self, queue_rx:Receiver<DriverPacket>) -> Result<(), FrcError> {

        //spins up 2 async concurent functions
        let (res1, res2) = tokio::join!(
            self.send_queue(queue_rx),
            self.read_queue_responses()
        );
        
        match res1 {
            Ok(_) => self.log_message("send_queue completed successfully").await,
            Err(e) => self.log_message(format!("send_queue failed: {}", e)).await,
        }

        match res2 {
            Ok(_) => self.log_message("read_queue_responses completed successfully").await,
            Err(e) => self.log_message(format!("read_queue_responses failed: {}", e)).await,
        }

        Ok(())
    }

    pub async fn add_to_queue(&self, packet: SendPacket, priority: PacketPriority){
        let sender = self.queue_tx.clone();
        let driver_packet = DriverPacket{priority, packet};
        if let Err(e) = sender.send(driver_packet).await {
            // Handle the error properly, e.g., logging it
            println!("Failed to send packet: {}", e);
        }
        else{
            // println!("sent packet to queue: {:?} ", packet2);
        }
    }

    async fn send_queue(&self,mut packets_to_add: mpsc::Receiver<DriverPacket>)-> Result<(), FrcError>{
        let mut queue = VecDeque::new();
        println!("started send loop");
        loop {   
            while let Ok(new_packet) = packets_to_add.try_recv() {
                match new_packet.priority{
                    PacketPriority::Low => queue.push_back(new_packet.packet),
                    PacketPriority::Standard => queue.push_back(new_packet.packet),
                    PacketPriority::High => queue.push_front(new_packet.packet),
                    PacketPriority::Immediate => queue.push_front(new_packet.packet),
                };
            }
            if let Some(packet) = queue.pop_front() {
                // Serialize the packet
                let serialized_packet = match serde_json::to_string(&packet) {
                    Ok(packet_str) => packet_str + "\r\n",
                    Err(e) => {
                        self.log_message(format!("Failed to serialize a packet: {}", e)).await;
                        break;
                    }
                };

                // Send the packet
                if let Err(e) = self.send_packet(serialized_packet).await {
                    self.log_message(format!("Failed to send a packet: {:?}", e)).await;
                }
                if packet == SendPacket::Communication(Communication::FrcDisconnect){break;}
            }
            sleep(Duration::from_millis(40)).await;

        }
        self.log_message("Disconnecting from FRC server... closing send queue").await;

        //when 0 is sent it shuts  off the reciever system so we wait one sec so that the response can be sent back and processed
        sleep(Duration::from_secs(1)).await;

        Ok(())
    }
    

    async fn read_queue_responses(&self) -> Result<(), FrcError> {
        
        let mut reader = self.read_half.lock().await;

        // let mut numbers_to_look_for: VecDeque<u32> = VecDeque::new();
        let mut buffer = vec![0; 2048];
        let mut temp_buffer = Vec::new();
        println!("started recieve loop");

        loop {
            tokio::select! {
                result = reader.read(&mut buffer) => {

                    match result {
                        Ok(0) => break Ok(()), // Connection closed
                        Ok(n) => {
                            // Append new data to temp_buffer
                            temp_buffer.extend_from_slice(&buffer[..n]);

                            while let Some(pos) = temp_buffer.iter().position(|&x| x == b'\n') {
                                // Split the buffer into the current message and the rest
                                let request: Vec<u8> = temp_buffer.drain(..=pos).collect();
                                // Remove the newline character
                                let request = &request[..request.len() - 1];

                                let response_str = String::from_utf8_lossy(request);
                                self.log_message(format!("recieved: {}", response_str.clone())).await;

                                let response_packet: Option<ResponsePacket> = match serde_json::from_str::<ResponsePacket>(&response_str) {
                                    Ok(response_packet) => Some(response_packet),
                                    Err(e) => {
                                        self.log_message(format!("Could not parse response into RPE: {}", e)).await;
                                        None
                                    }
                                };

                                // here is packet response handling logic. may be relocated soon
                                match response_packet {
                                    Some(ResponsePacket::CommunicationResponse(CommunicationResponse::FrcDisconnect(_))) => {
                                        println!("Received a FrcDisconnect packet.");
                                        break
                                    },
                                    Some(ResponsePacket::CommandResponse(CommandResponse::FrcInitialize(frc_initialize_response))) => {
                                        let id = frc_initialize_response.error_id;
                                        // if id != 0 {
                                        //     self.add_to_queue(SendPacket::Command(Command::FrcAbort), PacketPriority::Standard).await;
                                        //     self.add_to_queue(SendPacket::Command(Command::FrcInitialize(FrcInitialize::default())), PacketPriority::Standard).await;
                                        // }
                                        println!("Received a init packet. with eid :{}", id);
                                        break
                                    },
                                    _ => {
                                        // println!("Received a different type of packet.");
                                        // Handle other types of packets here
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            self.log_message(format!("Failed to read from stream: {}", e)).await;
                        }
                    }
                    sleep(Duration::from_millis(1)).await;
                }
            }
        }
    }

    //this is just a debug helper function to load the queue automatically
    pub async fn load_gcode(&self){
        let mut latest_sequence = self.latest_sequence.lock().await;
        let starting_sequence = *latest_sequence;
        *latest_sequence = starting_sequence+4;


        self.add_to_queue(SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
                1,    
                Configuration {
                    u_tool_number: 1, u_frame_number: 1, front: 1, up: 1, left: 1, flip: 1, turn4: 1, turn5: 1, turn6: 1,
                },
                Position { x: 0.0, y: 0.0, z: 100.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0,
                },
                SpeedType::MMSec,
                30.0,
                TermType::FINE,
                1,
            ))),
            PacketPriority::Standard
        ).await;
        self.add_to_queue(SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
                2,    
                Configuration {
                    u_tool_number: 1, u_frame_number: 1, front: 1, up: 1, left: 1, flip: 1, turn4: 1, turn5: 1, turn6: 1,
                },
                Position { x: 30.0, y: 100.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0,
                },
                SpeedType::MMSec,
                30.0,
                TermType::FINE,
                1,
            ))),
            PacketPriority::Standard
        ).await;
        self.add_to_queue(SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
                3,    
                Configuration { u_tool_number: 1, u_frame_number: 1, front: 1, up: 1, left: 1, flip: 1, turn4: 1, turn5: 1, turn6: 1,
                },
                Position { x: 0.0, y: 0.0, z: -100.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0,
                },
                SpeedType::MMSec,
                30.0,
                TermType::FINE,
                1,
            ))),
            PacketPriority::Standard
        ).await;
        self.add_to_queue(SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
                4,    
                Configuration { u_tool_number: 1, u_frame_number: 1, front: 1, up: 1, left: 1, flip: 1, turn4: 1, turn5: 1, turn6: 1,
                },
                Position { x: 0.0, y: -100.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0,
                },
                SpeedType::MMSec,
                30.0,
                TermType::FINE,
                1,
            ))),
            PacketPriority::Standard
        ).await;
        println!("added 4 packets to queue");
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
    return Err(FrcError::Disconnected())
}