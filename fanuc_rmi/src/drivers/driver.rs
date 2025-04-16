use serde::Deserialize;
use serde::Serialize;
use tokio::sync::broadcast;

use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;

use std::time::Duration;
use tokio::{ net::TcpStream, sync::Mutex, time::sleep};
use tokio::io::{ AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf, split};
use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::mpsc as other_mpsc;

pub use crate::{packets::*, FanucErrorCode};
pub use crate::instructions::*;
pub use crate::commands::*;
pub use crate::{Configuration, Position, SpeedType, TermType, FrcError };

use super::FanucDriverConfig;


//FIXME: get sequence id system working well

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
    pub log_channel: tokio::sync::broadcast::Sender<String>,
    latest_sequence: Arc<Mutex<u32>>,
    fanuc_write: Arc<Mutex<WriteHalf<TcpStream>>>, 
    fanuc_read: Arc<Mutex<ReadHalf<TcpStream>>>,    
    queue_tx: mpsc::Sender<DriverPacket>,       
    pub connected: Arc<Mutex<bool>>,
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
        let latest_sequence = Arc::new(Mutex::new(1));
        let connected = Arc::new(Mutex::new(true));
        let (completed_packet_tx, return_info_rx) = mpsc::channel::<CompletedPacketReturnInfo>(100);
        let completed_packet_channel = Arc::new(Mutex::new(return_info_rx));

        let driver = Self {
            config,
            log_channel: message_channel,
            latest_sequence,
            fanuc_write: write_half,
            fanuc_read: read_half,
            queue_tx,
            connected,
            completed_packet_channel
        };

        let driver_clone1 = driver.clone();
        let driver_clone2 = driver.clone();

        let (packets_done_tx, packets_done_rx): (other_mpsc::Sender<u32>, other_mpsc::Receiver<u32>) = other_mpsc::channel();
        
                
        tokio::spawn(async move {
            if let Err(e) = driver_clone1.send_queue_to_controller(queue_rx, packets_done_rx).await {
                eprintln!("send_queue failed: {}", e);
            }
        });

        tokio::spawn(async move {
            if let Err(e) = driver_clone2.read_responses(completed_packet_tx, packets_done_tx).await {
                eprintln!("read_queue_responses failed: {}", e);
            }
        });


        Ok(driver)
    }


    async fn log_message<T: Into<String>>(&self, message:T){
        let message = message.into();
        let _ = self.log_channel.send(message.clone());
        #[cfg(feature="logging")]
        println!("{:?}", message);
    }

    pub async fn abort(&self) -> Result<(), FrcError> {
        let packet = SendPacket::Command(Command::FrcAbort {});
        let _ = self.send_command(packet, PacketPriority::Standard).await;
        Ok(())
    }

    pub async fn initialize(&self) -> Result<(), FrcError> {

        let packet: SendPacket =  SendPacket::Command(Command::FrcInitialize(FrcInitialize::default()));

        let _ = self.send_command(packet, PacketPriority::Standard,).await;

        return Ok(());

    }

    pub async fn disconnect(&self) -> Result<(), FrcError> {
        let packet = SendPacket::Communication(Communication::FrcDisconnect {});
        let _ = self.send_command(packet, PacketPriority::Standard,).await;
        Ok(())
    }

    async fn send_packet_to_controller(&self, packet: SendPacket) -> Result<(), FrcError> {
        /*
        this is specifically for sending packets to the controller. It takes a packet and sends it over tcp to the controller.
        Note: not a public function
        */        

        let mut stream = self.fanuc_write.lock().await;
        
        let serialized_packet = match serde_json::to_string(&packet) {
            Ok(packet_str) => packet_str + "\r\n",
            Err(e) => {
                self.log_message(format!("Failed to serialize a packet: {}", e)).await;
                return Err(FrcError::Serialization(e.to_string()));
            }
        };
    
        if let Err(e) = stream.write_all(serialized_packet.as_bytes()).await {
            let err = FrcError::FailedToSend(format!("{}", e));
            self.log_message(err.to_string()).await;
            return Err(err);
        }
            
        self.log_message(format!("Sent: {}", serialized_packet)).await;
        Ok(())
    }


    pub async fn send_command(&self, packet: SendPacket, priority: PacketPriority) -> u32{
        
        /*
        This is the method meteorite will use to send a command to the driver this is the abstraction layer that will be called to send a packet and will returna sequence id.
        */

        let sender = self.queue_tx.clone();
        
        let (packet_with_sequence, sequence) = self.give_sequence_id(packet).await;
        let driver_packet = DriverPacket{priority, packet: packet_with_sequence };


        if let Err(e) = sender.send(driver_packet).await {
            println!("Failed to send packet: {}", e);
        }
        else{
            // println!("sent packet to queue: {:?} ", packet2);
        }


        sequence
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


    //this is an async function that recieves packets and yeets them to the controllor to run
    async fn send_queue_to_controller(&self,mut packets_to_add: mpsc::Receiver<DriverPacket>, packets_done_rx:other_mpsc::Receiver<u32>)-> Result<(), FrcError>{
        let mut packets_sent_number:u32 = 0;
        let mut packets_done_number:u32 = 0;
        let mut queue = VecDeque::new();

        loop {
            // Drain all available incoming packets
            if let Ok(new_packet) = packets_to_add.try_recv() {
                match new_packet.priority {
                    PacketPriority::Low | PacketPriority::Standard => queue.push_back(new_packet.packet),
                    PacketPriority::High | PacketPriority::Immediate => queue.push_front(new_packet.packet),
                    PacketPriority::Termination => {
                        queue.clear();
                        queue.push_front(new_packet.packet);
                    }
                }
            }
    
            // Drain completed packets
            while let Ok(num) = packets_done_rx.try_recv() {
                packets_done_number = num;
            }
    
            let packets_in_queue = packets_sent_number - packets_done_number;
    
            if packets_to_add.is_closed() && queue.is_empty() {
                break;
            }
    
            if packets_in_queue >= 8 {
                tokio::time::sleep(Duration::from_millis(10)).await;
                continue;
            }
    
            if let Some(packet) = queue.pop_front() {
                if let Err(e) = self.send_packet_to_controller(packet.clone()).await {
                    self.log_message(format!("Failed to send a packet: {:?}", e)).await;
                }
                packets_sent_number += 1;
                
                if packet == SendPacket::Communication(Communication::FrcDisconnect) {
                    break;
                }
            }
    
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    
        self.log_message("Disconnecting from FRC server... closing send queue").await;
        Ok(())
    
    }
    

    async fn read_responses(&self, completed_packet_tx: mpsc::Sender<CompletedPacketReturnInfo>, packets_done_tx:other_mpsc::Sender<u32>) -> Result<(), FrcError> {

        let mut packets_done_number:u32 = 0;
        let mut reader = self.fanuc_read.lock().await;
        let mut buffer = vec![0; 2048];
        let mut temp_buffer = Vec::new();

        loop {
            match reader.read(&mut buffer).await {
                Ok(0) => {
                    self.log_message("Connection closed by peer.").await;
                    let mut connected = self.connected.lock().await;
                    *connected = false;
                    return Err(FrcError::Disconnected());
                }
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
                        packets_done_number += 1;
                        if let Err(e) = packets_done_tx.send(packets_done_number) {
                            self.log_message(format!("Failed to send packets_done_number: {}", e)).await;
                        }
                        

                        let response_packet: Option<ResponsePacket> = match serde_json::from_str::<ResponsePacket>(&response_str) {
                            Ok(response_packet) => {
                                Some(response_packet)},
                            Err(e) => {
                                self.log_message(format!("Could not parse response into RPE: {}", e)).await;
                                None
                            }
                        };

                        // here is packet response handling logic. may be relocated soon
                        match response_packet {

                            Some(ResponsePacket::CommunicationResponse(CommunicationResponse::FrcDisconnect(_))) => {
                                self.log_message("Received a FrcDisconnect packet.").await;
                                let mut connected = self.connected.lock().await;
                                *connected = false;
                                return Ok(());
                            },
                            Some(ResponsePacket::CommandResponse(CommandResponse::FrcInitialize(resp))) => {
                                let id = resp.error_id;
                                if id != 0 {
                                    self.log_message(format!("Init response returned error id: {}. Attempting recovery.", id)).await;
                                    let _ = self.queue_tx.send(DriverPacket::new(PacketPriority::Standard,SendPacket::Command(Command::FrcAbort))).await;
                                    tokio::time::sleep(Duration::from_millis(100)).await;
                                    let _ = self.queue_tx.send(DriverPacket::new(PacketPriority::Standard,SendPacket::Command(Command::FrcInitialize(FrcInitialize::default())))).await;
                                } else {
                                    self.log_message("Init successful.").await;
                                }
                                continue;
                            }
                            Some(ResponsePacket::InstructionResponse(packet))=>{
                                let sequence_id:u32 = packet.get_sequence_id();
                                let error_id:u32 = packet.get_error_id();
                                if let Err(e) = completed_packet_tx.send(CompletedPacketReturnInfo { sequence_id, error_id }).await {
                                    self.log_message(format!("Failed to forward completed packet info: {}", e)).await;
                                }
                            },
                            None=>{
                                self.log_message(format!("Failed to parse response: {}", response_str)).await;
                                println!("Failed to parse response: {}", response_str);
                            }
                            _ => {
                                println!("Received a different type of packet.");
                                // Handle other types of packets here
                            }
                        }
                    }
                }
                Err(e) => {
                    self.log_message(format!("Read error: {}", e)).await;
                    let mut connected = self.connected.lock().await;
                    *connected = false;
                    return Err(FrcError::Disconnected());
                }

            
            }
            
        }


    }

   

    async fn give_sequence_id(&self, mut packet: SendPacket) ->  (SendPacket, u32) {

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

        }
        return (packet, current_id)  

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
