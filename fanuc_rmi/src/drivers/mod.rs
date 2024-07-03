use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::task;
use std::{error::Error, io, sync::Arc, time::Duration};
use tokio::{io::AsyncWriteExt, io::AsyncReadExt, net::TcpStream, sync::Mutex, time::sleep};
use std::collections::VecDeque;


use crate::packets::*;
use crate::instructions::*;
use crate::commands::*;
use crate::PacketEnum;
use crate::{Configuration, Position, SpeedType, TermType, FrcError };

pub struct FanucDriver {
    addr: String,
    initialize_port: u32,
    connection_port: Option<String>,
    tcp_stream: Option<Arc<Mutex<TcpStream>>>,
    // Instruction_packet_queue: VecDeque<i32>

}

impl FanucDriver {
    pub fn new(addr: String, initialize_port: u32) -> FanucDriver {
        Self {
            addr,
            initialize_port,
            connection_port: None,
            tcp_stream: None,
            
        }
    }

    pub async fn connect(&mut self) -> Result<(), Box<dyn Error>> {
        let init_addr = format!("{}:{}", self.addr, self.initialize_port);
        let stream = connect_with_retries(&init_addr, 3).await?;
        println!("Connected to the server at {}", init_addr);
        self.tcp_stream = Some(Arc::new(Mutex::new(stream)));

        // Create a connection packet
        let packet = Communication::FrcConnect {};
        
        let packet = match serde_json::to_string(&packet) {
            Ok(serialized_packet) => serialized_packet + "\r\n",
            Err(_) => return Err(Box::new(FrcError::Serialization("Communication: Connect packet didnt serialize correctly".to_string()))),
        };
        // Send a connection request packet to start the handshake
        let response = self.send::<CommunicationResponse>(packet).await?;

        //FIXME: this should prob have a defined behavior to handle not getting a port number back
        self.connection_port = match response {
            CommunicationResponse::FrcConnect(res) => Some(res.port_number.to_string()),
            _ => None,
        };


        // Close the initial connection
        self.close_connection();

        match &self.connection_port {
            Some(port) => {
                // Connect to the new port
                let new_addr = format!("{}:{}",&self.addr, port);
                let stream = connect_with_retries(&new_addr, 3).await?;
                println!("Connected to the secondary server at {}", new_addr);
                self.tcp_stream = Some(Arc::new(Mutex::new(stream)));
            },
            None => {return Err(Box::new(io::Error::new(io::ErrorKind::InvalidData, "Port number is missing in the response")));}
        }

        Ok(())
    }

    pub fn close_connection(&mut self) {
        self.tcp_stream = None;
    }



    pub async fn initialize(&self) -> Result<(), Box<dyn Error>> {

        let packet = Command::FrcInitialize(FrcInitialize::default());
        
        let packet = match serde_json::to_string(&packet) {
            Ok(serialized_packet) => serialized_packet + "\r\n",
            Err(_) => return Err(Box::new(FrcError::Serialization("Initalize packet didnt serialize correctly".to_string()))),
        };

        let response = self.send::<CommandResponse>(packet).await?;
        if let CommandResponse::FrcInitialize(ref res) = response {
            if res.error_id != 0 {
                println!("Error ID: {}", res.error_id);
                // return Err(Box::new(io::Error::new(io::ErrorKind::Interrupted, format!("Fanuc threw a Error #{} on a initialization packet", res.error_id))));
                return Err(Box::new(FrcError::FanucErrorCode(res.error_id)));
            }
        }
        Ok(())

    }
    
    
    
    pub async fn abort(&self) -> Result<(), Box<dyn Error>> {

        let packet = Command::FrcAbort {};
        
        let packet = match serde_json::to_string(&packet) {
            Ok(serialized_packet) => serialized_packet + "\r\n",
            Err(_) => return Err(Box::new(FrcError::Serialization("Abort packet didnt serialize correctly".to_string()))),
        };


        let response = self.send::<CommandResponse>(packet).await?;
        if let CommandResponse::FrcAbort(ref res) = response {
            if res.error_id != 0 {
                println!("Error ID: {}", res.error_id);
                return Err(Box::new(io::Error::new(io::ErrorKind::Interrupted, format!("Fanuc threw a Error #{} on a abort packet", res.error_id))));
            }
        }
        Ok(())
    }

    pub async fn get_status(&self) -> Result<(), Box<dyn Error>> {

        let packet = Command::FrcGetStatus {};
        
        let packet = match serde_json::to_string(&packet) {
            Ok(serialized_packet) => serialized_packet + "\r\n",
            Err(_) => return Err(Box::new(FrcError::Serialization("get_status packet didnt serialize correctly".to_string()))),
        };

        let response = self.send::<CommandResponse>(packet).await?;
        if let CommandResponse::FrcGetStatus(ref res) = response {
            if res.error_id != 0 {
                println!("Error ID: {}", res.error_id);
                return Err(Box::new(io::Error::new(io::ErrorKind::Interrupted, format!("Fanuc threw a Error #{} on a FrcGetStatus return packet", res.error_id))));
            }
        }
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<(), Box<dyn Error>> {

        let packet = Communication::FrcDisconnect {};
        
        let packet = match serde_json::to_string(&packet) {
            Ok(serialized_packet) => serialized_packet + "\r\n",
            Err(_) => return Err(Box::new(FrcError::Serialization("Disconnect packet didnt serialize correctly".to_string()))),
        };

        let response = self.send::<CommunicationResponse>(packet).await?;
        if let CommunicationResponse::FrcDisconnect(ref res) = response {
            if res.error_id != 0 {
                println!("Error ID: {}", res.error_id);
                return Err(Box::new(io::Error::new(io::ErrorKind::Interrupted, format!("Fanuc threw a Error #{} on a Disconect packet", res.error_id))));
            }
        }
        self.close_connection();

        Ok(())

    }

    //this need to be updated and need clearification on location and config input
    pub async fn linear_motion(
        &self,
        sequenceid: u32,    
        config: Configuration,
        pos: Position,
        speed_t: SpeedType,
        speed: u16,
        term_t: TermType,
        term_va: u8,

    ) -> Result<(), Box<dyn Error>> {
        let packet = Instruction::FrcLinearMotion(FrcLinearMotion::new(
            sequenceid,    
            config,
            pos,
            speed_t,
            speed,
            term_t,
            term_va,

        ));
        
        let packet = match serde_json::to_string(&packet) {
            Ok(serialized_packet) => serialized_packet + "\r\n",
            Err(_) => return Err(Box::new(FrcError::Serialization("linear motion packet didnt serialize correctly".to_string()))),
        };

        let response = self.send::<CommunicationResponse>(packet).await?;
        if let CommunicationResponse::FrcDisconnect(ref res) = response {
            if res.error_id != 0 {
                println!("Error ID: {}", res.error_id);
                return Err(Box::new(io::Error::new(io::ErrorKind::Interrupted, format!("Fanuc threw a Error #{} on a linear motion on return packet", res.error_id))));
            }
        }
        Ok(())

    }

    async fn send<T>(&self, packet: String) -> Result<T, Box<dyn Error>>
    where
        T: for<'a> Deserialize<'a> + std::fmt::Debug,
    {
        match &self.tcp_stream {
            Some(stream) => {
                let mut stream = stream.lock().await;

                stream.write_all(packet.as_bytes()).await?;
                println!("Sent: {}", packet);

                // Read response
                let mut buffer = vec![0; 2048];
                let n = stream.read(&mut buffer).await?;
                if n == 0 {
                    return Err(Box::new(io::Error::new(io::ErrorKind::Other, "Connection closed by peer")));
                }

                let response = String::from_utf8_lossy(&buffer[..n]);
                
                println!("Received: {}", response);

                // Parse JSON response
                match serde_json::from_str::<T>(&response) {
                    Ok(response_packet) => {
                        // Successfully parsed JSON into the generic type T
                        Ok(response_packet)
                    }
                    Err(e) => {
                        // Failed to parse JSON
                        println!("Could not parse response: {}", e);
                        Err(Box::new(io::Error::new(io::ErrorKind::Other, "could not parse response")))
                    }
                }
            }
            None => Err(Box::new(io::Error::new(io::ErrorKind::NotConnected, "Cannot send without initializing an open TCP stream"))),
        }
    }

    async fn load_gcode(&self) -> Result<VecDeque<PacketEnum>, Box<dyn Error>> {
        //here is where we will convert the gcode to the packets we need and return a queue

        //apply math magic to generate g code here


        let mut queue: VecDeque<PacketEnum> = VecDeque::new();
        queue.push_back(PacketEnum::Instruction(Instruction::FrcLinearMotion(FrcLinearMotion::new(
            1,    
        Configuration {
            u_tool_number: 1,
            u_frame_number: 1,
            front: 1,
            up: 1,
            left: 1,
            glip: 1,
            turn4: 1,
            turn5: 1,
            turn6: 1,
        },
        Position {
            x: 0.0,
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
        20,
        TermType::CNT,
        1,

        ))));

        queue.push_back(PacketEnum::Instruction(Instruction::FrcLinearMotion(FrcLinearMotion::new(
            2,    
        Configuration {
            u_tool_number: 1,
            u_frame_number: 1,
            front: 1,
            up: 1,
            left: 1,
            glip: 1,
            turn4: 1,
            turn5: 1,
            turn6: 1,
        },
        Position {
            x: 0.0,
            y: 100.0,
            z: 0.0,
            w: 0.0,
            p: 0.0,
            r: 0.0,
            ext1: 0.0,
            ext2: 0.0,
            ext3: 0.0,
        },
        SpeedType::MMSec,
        20,
        TermType::CNT,
        1,
        ))));

        Ok(queue)
        

    }

// FIXME: not done yet just geting the structure down
    pub async fn start_proccess(&self) -> Result<(), Box<dyn Error>> {
        //my vision is that you will call the start proccess function and feed it a input and it will start a queue of instructions and send and handle the request

        //implement a queue of packets and send them and get a response, using the buffer on the controllor
        
        //this function will be async so that just sits off to the side and handles the robot

        //dequeues here still have a O(1)access and removal so it just give us more functionality


        let mut sequencenum:u32 = 1;

        let mut queue: VecDeque<PacketEnum> = self.load_gcode().await?;


        let (tx, mut rx) = mpsc::channel(32);

        // Create a shared TCP stream wrapped in an Arc<Mutex<TcpStream>>

        let tcp_stream = match &self.tcp_stream {
            Some(stream) => Arc::clone(stream),
            None => {
                return Err(Box::new(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "Cannot send without initializing an open TCP stream",
                )));
            }
        };
        // let tcp_stream = self.tcp_stream;
    
        // Create a shared flag to indicate when the consumer is ready for the next command
        let consumer_ready_flag = Arc::new(Mutex::new(true));
    
        // Clone the ready flag for the producer
        let producer_ready_flag = Arc::clone(&consumer_ready_flag);

        let producer = task::spawn({
            let tx = tx.clone();
            let producer_ready_flag = Arc::clone(&producer_ready_flag);
            async move {
                for i in 0..7 {
                    // Wait until the consumer is ready for the next command
                    {
                        let mut ready = producer_ready_flag.lock().await;
                        while !*ready {
                            tokio::task::yield_now().await;
                        }
                        *ready = false; // Mark as not ready
                    }
                    println!("queue is :{}", queue.len() );
                    // Create a mock command
                    let command = format!("Command {}", i).into_bytes();
    
                    // Send the command to the channel
                    let packet = serde_json::to_string(&queue.pop_front().unwrap()).unwrap();
                    println!("deserialized to  :{}", packet );


                    let command = packet.into_bytes();
                    if tx.send(command).await.is_err() {
                        println!("Receiver dropped");
                        return;
                    }
                }
            }
        });
    

        // Spawn a consumer task
        let consumer = task::spawn({
            let tcp_stream= Arc::clone(&tcp_stream);

            let consumer_ready_flag = Arc::clone(&consumer_ready_flag);
            async move {
                while let Some(command) = rx.recv().await {
                    let mut stream = tcp_stream.lock().await;
    
                    // Write the command to the TCP stream
                    if let Err(e) = stream.write_all(&command).await {
                        println!("Failed to send command: {}", e);
                        continue;
                    } else {
                        println!("Sent: {:?}", String::from_utf8_lossy(&command));
                    }
    
                    // Read the response from the TCP stream
                    let mut buffer = vec![0; 1024];
                    match stream.read(&mut buffer).await {
                        Ok(n) if n == 0 => {
                            println!("Connection closed by server");
                            break;
                        }
                        Ok(n) => {
                            let response = &buffer[..n];
                            println!("Received on consumer: {:?}", String::from_utf8_lossy(response));
                        }
                        Err(e) => {
                            println!("Failed to read response: {}", e);
                            break;
                        }
                    }
    
                    // Mark the consumer as ready for the next command
                    {
                        let mut ready = consumer_ready_flag.lock().await;
                        *ready = true;
                    }
                }
            }
        });
    
        // Wait for both tasks to complete
        producer.await.unwrap();
        consumer.await.unwrap();
            



        Ok(())

    }

}



impl Default for FanucDriver {
    fn default() -> Self {
        let addr = "127.0.0.1".to_string(); // Change if the server is running on a different machine
        let initialize_port = 16001;
        Self::new(addr, initialize_port)
    }
}

async fn connect_with_retries(addr: &str, retries: u32) -> Result<TcpStream, Box<dyn Error>> {
    for attempt in 0..retries {
        match TcpStream::connect(addr).await {
            Ok(stream) => return Ok(stream),
            Err(e) => {
                eprintln!("Failed to connect (attempt {}): {}", attempt + 1, e);
                if attempt + 1 == retries {
                    return Err(Box::new(e));
                }
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
    Err("Exceeded maximum connection attempts".into())
}