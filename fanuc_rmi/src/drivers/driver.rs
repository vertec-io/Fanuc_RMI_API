use serde::Deserialize;
use serde::Serialize;
use tokio::sync::broadcast;

// use tokio::sync::broadcast::Sender;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

use std::{sync::Arc, time::Duration};
use tokio::{ net::TcpStream, sync::Mutex, time::sleep};
use tokio::io::{ AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf, split};
use std::collections::VecDeque;
use std::sync::{Arc as StdArc, RwLock};

// use crate::ResponsePacket;
pub use crate::{packets::*, FanucErrorCode};
pub use crate::instructions::*;
pub use crate::commands::*;
pub use crate::{Configuration, Position, SpeedType, TermType, FrcError };

use super::FanucDriverConfig;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum PacketPriority{
    Low,
    Standard,
    High,
    Immediate,
    Termination,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DriverPacket {
    pub priority: PacketPriority,
    pub packet: SendPacket,
}

impl DriverPacket {
    pub fn new(priority:PacketPriority, packet: SendPacket) -> Self {
        Self {
            priority,
            packet,
        }
    }
}

#[derive( Debug, Clone)]
pub struct FanucDriver {
    pub config: FanucDriverConfig,
    pub message_channel: tokio::sync::broadcast::Sender<String>,
    pub latest_sequence: Arc<Mutex<u32>>,
    write_half: Arc<Mutex<WriteHalf<TcpStream>>>,
    read_half: Arc<Mutex<ReadHalf<TcpStream>>>,
    queue_tx: mpsc::Sender<DriverPacket>,
    pub connected: StdArc<RwLock<bool>>,
    pub completed_packet_channel: Arc<Mutex<Receiver<CompletedPacketReturnInfo>>>,

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
    /// ```rust
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
        let init_addr = format!("{}:{}", config.addr, config.port);
        let mut stream = connect_with_retries(&init_addr, 3).await?;

        let packet = Communication::FrcConnect {};
        let serialized_packet = serde_json::to_string(&packet)
            .map_err(|_| FrcError::Serialization("Communication: Connect packet didn't serialize correctly".to_string()))? + "\r\n";

        stream.write_all(serialized_packet.as_bytes()).await
            .map_err(|e| FrcError::FailedToSend(e.to_string()))?;

        let mut buffer = vec![0; 2048];
        let n = stream.read(&mut buffer).await
            .map_err(|e| FrcError::FailedToRecieve(e.to_string()))?;

        if n == 0 {
            return Err(FrcError::Disconnected());
        }

        let response = String::from_utf8_lossy(&buffer[..n]);
        println!("Sent: {}\nReceived: {}", &serialized_packet, &response);

        let res: CommunicationResponse = serde_json::from_str(&response)
            .map_err(|e| FrcError::Serialization(format!("Could not parse response: {}", e)))?;

        let new_port = if let CommunicationResponse::FrcConnect(res) = res {
            res.port_number
        } else {
            return Err(FrcError::UnrecognizedPacket);
        };

        drop(stream);
        let init_addr = format!("{}:{}", config.addr, new_port);
        let stream = connect_with_retries(&init_addr, 3).await?;        

        let (read_half, write_half) = split(stream);
        let read_half = Arc::new(Mutex::new(read_half));
        let write_half = Arc::new(Mutex::new(write_half));
        let (message_channel, _rx) = broadcast::channel(100);
        let (queue_tx, queue_rx) = mpsc::channel::<DriverPacket>(100);
        let latest_sequence = Arc::new(Mutex::new(0));
        let connected = StdArc::new(RwLock::new(true));
        let (completed_packet_tx, return_info_rx) = mpsc::channel::<CompletedPacketReturnInfo>(100);
        let completed_packet_channel = Arc::new(Mutex::new(return_info_rx));

        let driver = Self {
            config,
            message_channel,
            latest_sequence,
            write_half,
            read_half,
            queue_tx,
            connected,
            completed_packet_channel
        };

        let mut driver_clone = driver.clone();
        tokio::spawn(async move {
            if let Err(e) = driver_clone.start_program(queue_rx, completed_packet_tx).await {
                eprintln!("Failed to start program: {:?}", e);
            } else {
                println!("Program started successfully");
            }
        });

        Ok(driver)
    }


    async fn log_message<T: Into<String>>(&self, message:T){
        let message = message.into();
        let _ = self.message_channel.send(message.clone());
        #[cfg(feature="logging")]
        println!("{:?}", message);
    }

    pub async fn abort(&self) -> Result<(), FrcError> {
        let packet = SendPacket::Command(Command::FrcAbort {});
        self.add_to_queue(packet, PacketPriority::Standard).await;
        Ok(())
    }

     pub async fn initialize(&self) -> Result<(), FrcError> {

        let packet: SendPacket =  SendPacket::Command(Command::FrcInitialize(FrcInitialize::default()));

        self.add_to_queue(packet, PacketPriority::Standard).await;

        return Ok(());

    }

    pub async fn disconnect(&self) -> Result<(), FrcError> {

        let packet = Communication::FrcDisconnect {};
        
        let packet = match serde_json::to_string(&packet) {
            Ok(serialized_packet) => serialized_packet + "\r\n",
            Err(_) => return Err(FrcError::Serialization("Disconnect packet didnt serialize correctly".to_string())),
        };

        self.send_packet(packet.clone()).await?;


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
/// Starts the program by spawning two asynchronous tasks: one to handle sending packets to the robot,
    /// and another to handle receiving responses from the robot.
    ///
    /// This function joins two futures: `send_queue` and `read_queue_responses`, both of which are responsible
    /// for managing the communication with the Fanuc controller. It logs the outcome of each task and returns
    /// an error if either task fails.
    ///
    /// # Arguments
    ///
    /// * `queue_rx` - A `Receiver<DriverPacket>` for receiving packets to be sent to the robot.
    ///
    /// # Returns
    ///
    /// If successful, returns `Ok(())`. Otherwise, returns an `FrcError` indicating the cause of the failure.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - `send_queue` fails to send packets to the robot.
    /// - `read_queue_responses` fails to read responses from the robot.
    ///
    /// # Example
    ///
    /// ```rust
    /// let (queue_tx, queue_rx) = mpsc::channel::<DriverPacket>(100);
    /// let mut driver = FanucDriver::connect(config).await?;
    /// driver.start_program(queue_rx).await?;
    /// ```
    pub async fn start_program(&mut self, queue_rx: Receiver<DriverPacket>, completed_packet_tx: mpsc::Sender<CompletedPacketReturnInfo>) -> Result<(), FrcError> {
        let current_packets_in_controller_queue: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));

        let (res1, res2) = tokio::join!(
            self.send_queue(queue_rx, current_packets_in_controller_queue.clone()),
            self.read_queue_responses(current_packets_in_controller_queue.clone(), completed_packet_tx)
        );
        
        if let Err(e) = res1 {
            self.log_message(format!("send_queue failed: {}", e)).await;
            return Err(e);
        } else {
            self.log_message("send_queue completed successfully").await;
        }

        if let Err(e) = res2 {
            self.log_message(format!("read_queue_responses failed: {}", e)).await;
            return Err(e);
        } else {
            self.log_message("read_queue_responses completed successfully").await;
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

    //this is an async function that recieves packets and yeets them to the controllor to run
    async fn send_queue(&self,mut packets_to_add: mpsc::Receiver<DriverPacket>, current_packets_in_controllor_queue:Arc<Mutex<i32>>)-> Result<(), FrcError>{
        
        let mut queue = VecDeque::new();

        println!("started send loop");

        let sid = self.latest_sequence.clone();
        let mut sid = sid.lock().await;
        *sid = 1; // Ensure this happens only once, ideally during initialization
        drop(sid); // Release the lock
        
        loop {   
            while let Ok(new_packet) = packets_to_add.try_recv() {
                match new_packet.priority{
                    PacketPriority::Low => queue.push_back(new_packet.packet),
                    PacketPriority::Standard => queue.push_back(new_packet.packet),
                    PacketPriority::High => queue.push_front(new_packet.packet),
                    PacketPriority::Immediate => queue.push_front(new_packet.packet),
                    PacketPriority::Termination => {
                        queue.clear();
                        queue.push_front(new_packet.packet)
                    },
                };
            }

            //this will delays us from sending too many packets to the controller
            {
            let current_packets: tokio::sync::MutexGuard<i32> = current_packets_in_controllor_queue.lock().await;
            if *current_packets >=8 {continue;}
            }
            if let Some(packet) = queue.pop_front() {
                
                //this will give the instruction packets a sequence number
                let packet: SendPacket = self.give_sequence_id(packet).await;                


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

                //this is a custom scope so that the mutex guard unlocks immediatly after it is operated on
                {
                let mut current_packets: tokio::sync::MutexGuard<i32> = current_packets_in_controllor_queue.lock().await;
                *current_packets += 1; // Dereference and increment the value
                // println!("just incremented to:{}",current_packets);
                }

                
                if packet == SendPacket::Communication(Communication::FrcDisconnect){break;}
            }
            sleep(Duration::from_millis(10)).await;

        }
        self.log_message("Disconnecting from FRC server... closing send queue").await;

        //when 0 is sent it shuts  off the reciever system so we wait one sec so that the response can be sent back and processed
        // sleep(Duration::from_secs(1)).await;

        let current_packets: tokio::sync::MutexGuard<i32> = current_packets_in_controllor_queue.lock().await;
        self.log_message(format!("driver send queue ended with {} in controller", *current_packets)).await;


        Ok(())
    }
    

    async fn read_queue_responses(&self, current_packets_in_controllor_queue:Arc<Mutex<i32>>, completed_packet_tx: mpsc::Sender<CompletedPacketReturnInfo>) -> Result<(), FrcError> {
        
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
                                    Ok(response_packet) => {

                                        //this decrements the number of packets in the controllor queue when we recieve a response
                                        {
                                        let mut current_packets: tokio::sync::MutexGuard<i32> = current_packets_in_controllor_queue.lock().await;
                                        *current_packets -= 1; // Dereference and increment the value
                                        // println!("just decremented to:{}",current_packets);
                                        }

                                        Some(response_packet)},
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
                                        if id != 0 {
                                            self.add_to_queue(SendPacket::Command(Command::FrcAbort), PacketPriority::Standard).await;
                                            sleep(Duration::from_millis(1)).await;

                                            self.add_to_queue(SendPacket::Command(Command::FrcInitialize(FrcInitialize::default())), PacketPriority::Standard).await;
                                        }
                                        println!("Received a init packet. with eid :{}", id);
                                        break
                                    },
                                    Some(ResponsePacket::InstructionResponse(packet))=>{
                                        let sequence_id:u32 = packet.get_sequence_id();
                                        let error_id:u32 = packet.get_error_id();
                                        match completed_packet_tx.send(CompletedPacketReturnInfo{sequence_id, error_id}).await{
                                            Ok(_) => println!("returning some info"),
                                            Err(e) => println!("couldnt send returned packet info{}",e),
                                        }
                                    
                                    }
                                    _ => {
                                        // println!("Received a different type of packet.");
                                        // Handle other types of packets here
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // let mut connected = self.connected.clone();
                            let mut err_occured = false;
                            match self.connected.write() {
                                Ok(mut connected) => {
                                    *connected = false;
                                },
                                Err(_) => {err_occured = true;},
                            };
                          
                            if err_occured {
                                self.log_message(format!("The driver stream disconnected in a poisoned state and driver failed to set connection status to false")).await;
                            }
                            
                            self.log_message(format!("Failed to read from stream: {}", e)).await;
                            break Err(FrcError::Disconnected())
                        }
                    }
                    sleep(Duration::from_millis(1)).await;
                }
            }
        }

        // println!("ended here");
        // let current_packets: tokio::sync::MutexGuard<i32> = current_packets_in_controllor_queue.lock().await;
        // self.log_message(format!("driver send queue ended with {} in controller", *current_packets)).await;
        // Ok(())
    }

    //this is just a debug helper function to load the queue automatically
    pub async fn load_gcode(&self){


        self.add_to_queue(SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
                10,    
                Configuration {
                    u_tool_number: 1, u_frame_number: 2, front: 1, up: 1, left: 1, flip: 1, turn4: 1, turn5: 1, turn6: 1,
                },
                Position { x: 0.0, y: 0.0, z: 10.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0,
                },
                SpeedType::MMSec,
                30.0,
                TermType::FINE,
                1,
            ))),
            PacketPriority::Standard
        ).await;
        self.add_to_queue(SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
                11,    
                Configuration {
                    u_tool_number: 1, u_frame_number: 2, front: 1, up: 1, left: 1, flip: 1, turn4: 1, turn5: 1, turn6: 1,
                },
                Position { x: 0.0, y: 10.0, z: 0.0, w: 0.0, p: 0.0, r: 0.0, ext1: 0.0, ext2: 0.0, ext3: 0.0,
                },
                SpeedType::MMSec,
                30.0,
                TermType::FINE,
                1,
            ))),
            PacketPriority::Standard
        ).await;
        self.add_to_queue(SendPacket::Instruction(Instruction::FrcLinearRelative(FrcLinearRelative::new(
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




        println!("added 3 packets to queue");
    }

    async fn give_sequence_id(&self, mut packet: SendPacket) -> SendPacket {

        let sid = self.latest_sequence.clone();
        let mut sid = sid.lock().await;
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
            }

            *sid += 1;
            // println!("");
            // println!("Sequence after increment: {}", sid);
            // println!("");


        }
        packet
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
