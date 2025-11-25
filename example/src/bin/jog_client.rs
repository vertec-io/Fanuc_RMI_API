// Interactive jogging client for FANUC robot
// Run with: cargo run -p example --bin jog_client
// Make sure the simulator is running: cargo run -p sim

use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::{interval, sleep};
use fanuc_rmi::{
    drivers::{FanucDriver, FanucDriverConfig},
    instructions::FrcLinearRelative,
    packets::*,
    Configuration, FrcError, Position, SpeedType, TermType,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MotionMode {
    Step,
    Continuous,
}

impl std::fmt::Display for MotionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MotionMode::Step => write!(f, "Step"),
            MotionMode::Continuous => write!(f, "Continuous"),
        }
    }
}

#[derive(Debug, Clone)]
struct JogConfig {
    jog_speed: f64,      // mm/s
    step_distance: f64,  // mm
    mode: MotionMode,
}

impl Default for JogConfig {
    fn default() -> Self {
        Self {
            jog_speed: 10.0,
            step_distance: 1.0,
            mode: MotionMode::Step,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), FrcError> {
    println!("=== FANUC Interactive Jogging Client ===\n");

    // Connect to robot
    let driver_settings = FanucDriverConfig {
        addr: "127.0.0.1".to_string(),
        port: 16001,
        max_messages: 30,
    };

    println!("Connecting to robot at {}:{}...", driver_settings.addr, driver_settings.port);
    let driver = FanucDriver::connect(driver_settings.clone()).await?;
    sleep(Duration::from_millis(500)).await;

    // Subscribe to response channel and print responses
    let mut response_rx = driver.response_tx.subscribe();
    tokio::spawn(async move {
        while let Ok(response) = response_rx.recv().await {
            println!("📥 Response: {:?}", response);
        }
    });

    println!("Initializing robot...");
    driver.initialize();
    sleep(Duration::from_millis(500)).await;

    let config = Arc::new(Mutex::new(JogConfig::default()));
    let active_jog = Arc::new(Mutex::new(None::<char>));

    println!("\n✓ Connected and initialized!\n");
    
    // Main loop
    loop {
        display_status(&config).await;
        print_help();

        print!("\nCommand: ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        let cmd = input.chars().next().unwrap();

        match cmd {
            'q' => {
                println!("\nShutting down...");
                driver.abort();
                sleep(Duration::from_millis(100)).await;
                driver.disconnect().await;
                sleep(Duration::from_millis(500)).await;
                break;
            }
            's' => {
                if let Err(e) = set_jog_speed(&config).await {
                    println!("Error: {}", e);
                }
            }
            'd' => {
                if let Err(e) = set_step_distance(&config).await {
                    println!("Error: {}", e);
                }
            }
            'm' => {
                toggle_mode(&config).await;
            }
            'j' | 'k' | 'h' | 'l' | 'f' | 'b' => {
                let cfg = config.lock().await.clone();
                match cfg.mode {
                    MotionMode::Step => {
                        if let Err(e) = handle_jog_step(&driver, cmd, &cfg).await {
                            println!("Jog error: {}", e);
                        }
                    }
                    MotionMode::Continuous => {
                        // Check if already jogging in this direction
                        let current_jog = *active_jog.lock().await;
                        if current_jog.is_none() {
                            if let Err(e) = handle_jog_continuous_start(&driver, cmd, &cfg, &active_jog).await {
                                println!("Jog error: {}", e);
                            }
                        } else if current_jog == Some(cmd) {
                            // Same key pressed again - stop jogging
                            *active_jog.lock().await = None;
                        } else {
                            // Different key - stop current and start new
                            *active_jog.lock().await = None;
                            sleep(Duration::from_millis(200)).await; // Wait for stop
                            if let Err(e) = handle_jog_continuous_start(&driver, cmd, &cfg, &active_jog).await {
                                println!("Jog error: {}", e);
                            }
                        }
                    }
                }
            }
            _ => {
                println!("Unknown command: '{}'", cmd);
            }
        }
    }

    println!("Disconnected.");
    Ok(())
}

async fn display_status(config: &Arc<Mutex<JogConfig>>) {
    let cfg = config.lock().await;
    println!("\n╔════════════════════════════════════════╗");
    println!("║         JOGGING CONFIGURATION          ║");
    println!("╠════════════════════════════════════════╣");
    println!("║ Jog Speed:      {:>8.2} mm/s        ║", cfg.jog_speed);
    println!("║ Step Distance:  {:>8.2} mm          ║", cfg.step_distance);
    println!("║ Motion Mode:    {:>12}           ║", cfg.mode);
    println!("╚════════════════════════════════════════╝");
}

fn print_help() {
    println!("\n┌─────────────────────────────────────────┐");
    println!("│ MOTION CONTROLS:                        │");
    println!("│  k = Up    (+Z)    j = Down   (-Z)      │");
    println!("│  h = Left  (-Y)    l = Right  (+Y)      │");
    println!("│  f = Forward (+X)  b = Backward (-X)    │");
    println!("│                                         │");
    println!("│ CONFIGURATION:                          │");
    println!("│  s = Set jog speed                      │");
    println!("│  d = Set step distance                  │");
    println!("│  m = Toggle motion mode (Step/Cont.)    │");
    println!("│                                         │");
    println!("│ OTHER:                                  │");
    println!("│  q = Quit                               │");
    println!("└─────────────────────────────────────────┘");
}

async fn set_jog_speed(config: &Arc<Mutex<JogConfig>>) -> Result<(), String> {
    print!("Enter jog speed (mm/s): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let speed: f64 = input.trim().parse()
        .map_err(|_| "Invalid number".to_string())?;

    if speed <= 0.0 || speed > 1000.0 {
        return Err("Speed must be between 0 and 1000 mm/s".to_string());
    }

    config.lock().await.jog_speed = speed;
    println!("✓ Jog speed set to {:.2} mm/s", speed);
    Ok(())
}

async fn set_step_distance(config: &Arc<Mutex<JogConfig>>) -> Result<(), String> {
    print!("Enter step distance (mm): ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let distance: f64 = input.trim().parse()
        .map_err(|_| "Invalid number".to_string())?;

    if distance <= 0.0 || distance > 100.0 {
        return Err("Distance must be between 0 and 100 mm".to_string());
    }

    config.lock().await.step_distance = distance;
    println!("✓ Step distance set to {:.2} mm", distance);
    Ok(())
}

async fn toggle_mode(config: &Arc<Mutex<JogConfig>>) {
    let mut cfg = config.lock().await;
    cfg.mode = match cfg.mode {
        MotionMode::Step => MotionMode::Continuous,
        MotionMode::Continuous => MotionMode::Step,
    };
    println!("✓ Motion mode set to: {}", cfg.mode);
}

fn get_direction_vector(key: char, distance: f64) -> Position {
    match key {
        'k' => Position { z: distance, ..Default::default() },  // Up (+Z)
        'j' => Position { z: -distance, ..Default::default() }, // Down (-Z)
        'h' => Position { y: -distance, ..Default::default() }, // Left (-Y)
        'l' => Position { y: distance, ..Default::default() },  // Right (+Y)
        'f' => Position { x: distance, ..Default::default() },  // Forward (+X)
        'b' => Position { x: -distance, ..Default::default() }, // Backward (-X)
        _ => Position::default(),
    }
}

async fn handle_jog_step(
    driver: &FanucDriver,
    key: char,
    config: &JogConfig,
) -> Result<(), String> {
    // Step mode: single FINE move with Immediate priority
    let position = get_direction_vector(key, config.step_distance);
    let instruction = FrcLinearRelative::new(
        0, // sequence_id will be assigned by driver
        Configuration::default(),
        position,
        SpeedType::MMSec,
        config.jog_speed,
        TermType::FINE,
        1, // term_value (ignored for FINE)
    );

    let packet = SendPacket::Instruction(Instruction::FrcLinearRelative(instruction));
    let _seq_id = driver.send_command(packet, PacketPriority::Immediate)
        .map_err(|e| format!("Failed to send step command: {}", e))?;

    println!("→ Step {} ({:.2} mm)", get_direction_name(key), config.step_distance);
    Ok(())
}

async fn handle_jog_continuous_start(
    driver: &FanucDriver,
    key: char,
    config: &JogConfig,
    active_jog: &Arc<Mutex<Option<char>>>,
) -> Result<(), String> {
    // Mark this direction as active
    *active_jog.lock().await = Some(key);

    println!("→ Continuous {} started ({:.2} mm/s)", get_direction_name(key), config.jog_speed);

    // Start streaming CNT moves at 10Hz until key is released
    let driver_clone = driver.clone();
    let config_clone = config.clone();
    let active_jog_clone = active_jog.clone();

    tokio::spawn(async move {
        // Calculate appropriate send interval to avoid buffer overflow
        // We want to send the next command before the current one completes, but not too early
        // Motion duration = distance / speed
        // We'll send at a rate that keeps ~3-4 commands in the buffer
        let motion_duration_ms = (config_clone.step_distance / config_clone.jog_speed * 1000.0) as u64;
        let send_interval_ms = motion_duration_ms.max(50); // At least 50ms between sends

        println!("   📊 Continuous mode: sending every {}ms (motion takes {}ms)",
                 send_interval_ms, motion_duration_ms);

        let mut tick = interval(Duration::from_millis(send_interval_ms));
        let step_per_tick = config_clone.step_distance; // Distance per tick

        loop {
            tick.tick().await;

            // Check if this jog is still active
            let current_jog = *active_jog_clone.lock().await;
            if current_jog != Some(key) {
                // Key was released, send FINE termination move
                let position = get_direction_vector(key, 0.1); // Small final move
                let instruction = FrcLinearRelative::new(
                    0,
                    Configuration::default(),
                    position,
                    SpeedType::MMSec,
                    config_clone.jog_speed,
                    TermType::FINE,
                    1,
                );

                let packet = SendPacket::Instruction(Instruction::FrcLinearRelative(instruction));
                let _ = driver_clone.send_command(packet, PacketPriority::Termination);
                println!("→ Continuous {} stopped", get_direction_name(key));
                break;
            }

            // Send CNT move
            let position = get_direction_vector(key, step_per_tick);
            let instruction = FrcLinearRelative::new(
                0,
                Configuration::default(),
                position,
                SpeedType::MMSec,
                config_clone.jog_speed,
                TermType::CNT,
                100, // High CNT value for smooth continuous motion
            );

            let packet = SendPacket::Instruction(Instruction::FrcLinearRelative(instruction));
            if let Err(e) = driver_clone.send_command(packet, PacketPriority::Immediate) {
                eprintln!("Failed to send continuous move: {}", e);
                break;
            }
        }
    });

    Ok(())
}

fn get_direction_name(key: char) -> &'static str {
    match key {
        'k' => "Up (+Z)",
        'j' => "Down (-Z)",
        'h' => "Left (-Y)",
        'l' => "Right (+Y)",
        'f' => "Forward (+X)",
        'b' => "Backward (-X)",
        _ => "Unknown",
    }
}

