use serde::Deserialize;
use serde::Serialize;
use tokio::sync::broadcast;

use tokio::sync::mpsc;

use std::time::Duration;
use std::time::Instant;
use tokio::{ net::TcpStream, sync::Mutex, time::sleep};
use tokio::io::{ AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf, split};
use std::collections::VecDeque;
use std::sync::Arc;

pub use crate::{packets::*, FanucErrorCode};
pub use crate::instructions::*;
pub use crate::commands::*;
pub use crate::{Configuration, Position, SpeedType, TermType, FrcError };

use super::FanucDriverConfig;

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
    next_available_sequence_number: Arc<Mutex<u32>>,   // could prop be taken out and just a varible in the send_queue function
    fanuc_write: Arc<Mutex<WriteHalf<TcpStream>>>, 
    fanuc_read: Arc<Mutex<ReadHalf<TcpStream>>>,    
    queue_tx: mpsc::Sender<DriverPacket>,       
    pub connected: Arc<Mutex<bool>>,
    completed_packet_channel: Arc<Mutex<broadcast::Receiver<CompletedPacketReturnInfo>>>,
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
        let next_available_sequence_number = Arc::new(Mutex::new(1));
        
        let connected = Arc::new(Mutex::new(true));

        let (completed_packet_tx,_)= broadcast::channel(100);
        let return_info_rx = completed_packet_tx.subscribe();
        let return_info = completed_packet_tx.subscribe();
        let completed_packet_channel = Arc::new(Mutex::new(return_info_rx));


        let driver = Self {
            config,
            log_channel: message_channel,
            next_available_sequence_number,
            fanuc_write: write_half,
            fanuc_read: read_half,
            queue_tx,
            connected,
            completed_packet_channel
        };

        let driver_clone1 = driver.clone();
        let driver_clone2 = driver.clone();        
                
        tokio::spawn(async move {
            if let Err(e) = driver_clone1.send_queue_to_controller(queue_rx, return_info).await {
                eprintln!("send_queue failed: {}", e);
            }
        });

        tokio::spawn(async move {
            if let Err(e) = driver_clone2.read_responses(completed_packet_tx).await {
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

    pub async fn abort(&self) {
        let packet = SendPacket::Command(Command::FrcAbort {});
        let _ = self.send_command(packet, PacketPriority::Standard).await;
    }

    pub async fn initialize(&self) {

        let packet: SendPacket =  SendPacket::Command(Command::FrcInitialize(FrcInitialize::default()));
        // self.get_status().await;
        let _ = self.send_command(packet, PacketPriority::Standard,).await;
    }

    pub async fn get_status(&self) {
        let packet: SendPacket =  SendPacket::Command(Command::FrcGetStatus);
        let _ = self.send_command(packet, PacketPriority::Standard,).await;
    }

    pub async fn disconnect(&self){
        let packet = SendPacket::Communication(Communication::FrcDisconnect {});
        let _ = self.send_command(packet, PacketPriority::Standard,).await;
        *self.connected.lock().await = false;
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
        // let driver_packet2 = driver_packet.clone();

        if let Err(e) = sender.send(driver_packet).await {
            println!("Failed to send packet: {}", e);
        }
        else{
            // println!("sent packet to queue: {:?} ", driver_packet2);
        }


        sequence
    }

    //this is an async function that recieves packets and yeets them to the controllor to run
    async fn send_queue_to_controller(&self,mut packets_to_add: mpsc::Receiver<DriverPacket>,mut completed_packet_info:broadcast::Receiver<CompletedPacketReturnInfo>)-> Result<(), FrcError>{

        let mut in_flight: u32 = 0;
        let mut queue = VecDeque::new();

        loop {
            let start_time = Instant::now();

            // Drain all available incoming packets
            while let Ok(new_packet) = packets_to_add.try_recv() {
                match new_packet.priority {
                    PacketPriority::Low | PacketPriority::Standard => queue.push_back(new_packet.packet),
                    PacketPriority::High | PacketPriority::Immediate => queue.push_front(new_packet.packet),
                    PacketPriority::Termination => {
                        queue.clear();
                        queue.push_front(new_packet.packet);

                    }
                }
            }
    
            while let Ok(_pkt) = completed_packet_info.try_recv() {
                in_flight = in_flight.saturating_sub(1);
                // println!("Ack for seq {} received, {} in-flight remaining", pkt.sequence_id, in_flight);
            }

            if packets_to_add.is_closed() && queue.is_empty() {
                break;
            }
    
            while in_flight < 8 {
                if let Some(packet) = queue.pop_front() {
                    match self.send_packet_to_controller(packet.clone()).await {
                        Err(e) => {
                            self.log_message(format!("Failed to send packet: {:?}", e)).await;
                        }
                        Ok(()) => {
                            if packet == SendPacket::Communication(Communication::FrcDisconnect) {
                                // immediate shutdown
                                queue.clear();
                                break;
                            }
                            if let SendPacket::Instruction(instr) = packet {
                                let _seq = instr.get_sequence_id();
                                // println!("Sent seq {} ({} in-flight)", seq, in_flight + 1);
                                in_flight += 1;
                            }
                        }
                    }
                } else {
                    break;
                }
            }
            let current_time = Instant::now();
            let elapsed = current_time.duration_since(start_time);
            let maxtime = Duration::from_millis(16);
            if elapsed < maxtime {
                let sleep_duration = maxtime - elapsed;
                tokio::time::sleep(sleep_duration).await;
            }
            else{
                self.log_message(format!("Send loop duration took {:?} exeeding max time:{:?}", elapsed, maxtime)).await;
            }
            // tokio::time::sleep(Duration::from_millis(10)).await;
        }
    
        self.log_message("Disconnecting from FRC server... closing send queue").await;
        Ok(())
    
    }
    

    // Simplified main loop:
    async fn read_responses(
        &self,
        completed_tx: broadcast::Sender<CompletedPacketReturnInfo>
    ) -> Result<(), FrcError> {
        let mut reader = self.fanuc_read.lock().await;
        let mut buf = vec![0; 2048];
        let mut temp = Vec::new();

        loop {
            let n = match reader.read(&mut buf).await {
                Ok(0) => {
                    *self.connected.lock().await = false;
                    return Err(FrcError::Disconnected())
                },
                Ok(n) => n,
                Err(_) => {
                    *self.connected.lock().await = false;
                    return Err(FrcError::Disconnected())
                },
            };

            temp.extend_from_slice(&buf[..n]);
            for line in extract_lines(&mut temp) {
                self.process_line(line, &completed_tx).await?;
            }

            // tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }


    // Extract handling of each line into an async helper:
    async fn process_line(
        &self,
        line: String,
        completed_tx: &broadcast::Sender<CompletedPacketReturnInfo>
    ) -> Result<(), FrcError> {
        self.log_message(format!("received: {}", line)).await;
        if let Ok(packet) = serde_json::from_str::<ResponsePacket>(&line) {
            match packet {
                ResponsePacket::CommunicationResponse(CommunicationResponse::FrcDisconnect(_)) => {
                    self.log_message("Disconnect packet").await;
                    let mut conn = self.connected.lock().await;
                    *conn = false;
                    return Ok(());
                }
                ResponsePacket::InstructionResponse(pkt) => {
                    let info = CompletedPacketReturnInfo {
                        sequence_id: pkt.get_sequence_id(),
                        error_id:    pkt.get_error_id(),
                    };
                    if let Err(e) = completed_tx.send(info) {
                        self.log_message(format!("Send error: {}", e)).await;
                    }
                }
                // handle other variants similarly...
                _ => {}
            }
        } else {
            self.log_message(format!("Invalid JSON: {}", line)).await;
        }
        Ok(())
    }


    async fn give_sequence_id(&self, mut packet: SendPacket) ->  (SendPacket, u32) {

        let sid = self.next_available_sequence_number.clone();
        let mut sid = sid.lock().await;
        let current_id = *sid;
        // let current_id = 11;

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

    pub async fn wait_on_command_completion(&self, packet_number_to_wait_for: u32) {
        loop {
            let guard = self.completed_packet_channel.clone();
            let mut guard = guard.lock().await;
            match guard.try_recv() {
                Ok(most_recent) => {
                    if most_recent.error_id != 0 {
                        eprintln!("ROBOT MOTION ERROR: {}", most_recent.error_id);
                        break;
                    } else {
                        if most_recent.sequence_id == packet_number_to_wait_for {
                                println!("robot move done #{}", most_recent.sequence_id);
                                break;
                            
                        }
                    }
                }
                Err(broadcast::error::TryRecvError::Empty) => {}
                Err(broadcast::error::TryRecvError::Closed) => println!("Channel closed."),
                Err(broadcast::error::TryRecvError::Lagged(skipped)) => println!("Channel lagged, skipped {} messages.", skipped),
            }
            drop(guard);
        }
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


    // Extract parsing of complete lines into a helper:
fn extract_lines(buffer: &mut Vec<u8>) -> Vec<String> {
    let mut lines = Vec::new();
    while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
        let mut chunk = buffer.drain(..=pos).collect::<Vec<_>>();
        chunk.pop(); // remove the `\n`
        if let Ok(s) = String::from_utf8(chunk) {
            lines.push(s);
        }
    }
    lines
}
