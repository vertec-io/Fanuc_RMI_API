// Interactive jogging client for FANUC robot with TUI
// Run with: cargo run -p example --bin jog_client_tui
// Make sure the simulator is running: cargo run -p sim

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::Mutex;
use tokio::time::{interval, sleep};
use fanuc_rmi::{
    commands::FrcReadCartesianPosition,
    drivers::{FanucDriver, FanucDriverConfig},
    instructions::FrcLinearRelative,
    packets::*,
    Configuration, Position, SpeedType, TermType,
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
    jog_speed: f64,
    step_distance: f64,
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

#[derive(Debug, Clone)]
struct MotionLog {
    timestamp: SystemTime,
    message: String,
}

struct AppState {
    config: JogConfig,
    motion_log: VecDeque<MotionLog>,
    current_position: Option<(f64, f64, f64)>,
    robot_status: Option<RobotStatus>,
    error_log: VecDeque<String>,
    status_message: String,
    active_jog: Option<char>,
    should_quit: bool,
}

#[derive(Debug, Clone)]
struct RobotStatus {
    servo_ready: i8,
    tp_mode: i8,
    motion_status: i8,
}

impl AppState {
    fn new() -> Self {
        Self {
            config: JogConfig::default(),
            motion_log: VecDeque::new(),
            current_position: None,
            robot_status: None,
            error_log: VecDeque::new(),
            status_message: "Connected and initialized".to_string(),
            active_jog: None,
            should_quit: false,
        }
    }

    fn add_motion(&mut self, message: String) {
        self.motion_log.push_back(MotionLog {
            timestamp: SystemTime::now(),
            message,
        });
        if self.motion_log.len() > 100 {
            self.motion_log.pop_front();
        }
    }

    fn update_position(&mut self, x: f64, y: f64, z: f64) {
        self.current_position = Some((x, y, z));
    }

    fn update_robot_status(&mut self, servo_ready: i8, tp_mode: i8, motion_status: i8) {
        self.robot_status = Some(RobotStatus {
            servo_ready,
            tp_mode,
            motion_status,
        });
    }

    fn add_error(&mut self, message: String) {
        self.error_log.push_back(message);
        if self.error_log.len() > 5 {
            self.error_log.pop_front();
        }
    }

    fn set_status(&mut self, message: String) {
        self.status_message = message;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Connect to robot
    let driver_settings = FanucDriverConfig {
        addr: "127.0.0.1".to_string(),
        port: 16001,
        max_messages: 30,
        log_level: fanuc_rmi::drivers::LogLevel::Info,
    };

    let driver = FanucDriver::connect(driver_settings.clone()).await
        .map_err(|e| format!("Failed to connect: {}", e))?;
    sleep(Duration::from_millis(500)).await;

    // Initialize and wait for response
    match driver.initialize().await {
        Ok(response) => {
            if response.error_id == 0 {
                // Success - continue
            } else {
                return Err(format!("Initialize failed with error: {}", response.error_id).into());
            }
        }
        Err(e) => {
            return Err(format!("Initialize error: {}", e).into());
        }
    }

    // Create shared state
    let app_state = Arc::new(Mutex::new(AppState::new()));

    // Subscribe to response channel and categorize responses
    let mut response_rx = driver.response_tx.subscribe();
    let app_state_clone = Arc::clone(&app_state);
    tokio::spawn(async move {
        while let Ok(response) = response_rx.recv().await {
            let mut state = app_state_clone.lock().await;

            match response {
                ResponsePacket::InstructionResponse(resp) => {
                    let seq_id = resp.get_sequence_id();
                    let error_id = resp.get_error_id();

                    if error_id != 0 {
                        state.add_error(format!("Motion error: Seq:{} Err:{}", seq_id, error_id));
                    } else {
                        let msg = match resp {
                            InstructionResponse::FrcLinearRelative(_) => {
                                format!("Linear move completed (Seq:{})", seq_id)
                            }
                            InstructionResponse::FrcJointMotion(_) => {
                                format!("Joint move completed (Seq:{})", seq_id)
                            }
                            InstructionResponse::FrcWaitTime(_) => {
                                format!("Wait completed (Seq:{})", seq_id)
                            }
                            _ => format!("Motion completed (Seq:{})", seq_id)
                        };
                        state.add_motion(msg);
                    }
                }
                ResponsePacket::CommandResponse(resp) => {
                    match resp {
                        CommandResponse::FrcReadCartesianPosition(r) => {
                            if r.error_id == 0 {
                                state.update_position(r.pos.x as f64, r.pos.y as f64, r.pos.z as f64);
                            }
                        }
                        CommandResponse::FrcGetStatus(s) => {
                            if s.error_id == 0 {
                                state.update_robot_status(s.servo_ready, s.tp_mode, s.rmi_motion_status);
                            }
                        }
                        _ => {
                            // Ignore other command responses
                        }
                    }
                }
                ResponsePacket::CommunicationResponse(_) => {
                    // Ignore communication responses
                }
            }
        }
    });

    // Periodic polling for position and status
    let driver_clone = driver.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        loop {
            interval.tick().await;

            // Request position (uses new() constructor, not default())
            let _ = driver_clone.send_packet(
                SendPacket::Command(Command::FrcReadCartesianPosition(FrcReadCartesianPosition::new(None))),
                PacketPriority::Low
            );

            // Request status (unit variant - no arguments)
            let _ = driver_clone.send_packet(
                SendPacket::Command(Command::FrcGetStatus),
                PacketPriority::Low
            );
        }
    });

    // Run the app
    let res = run_app(&mut terminal, driver, app_state).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    driver: FanucDriver,
    app_state: Arc<Mutex<AppState>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let active_jog = Arc::new(Mutex::new(None::<char>));
    let jog_cancel = Arc::new(Mutex::new(false));

    loop {
        // Render UI
        {
            let state = app_state.lock().await;
            terminal.draw(|f| ui(f, &state))?;
            if state.should_quit {
                break;
            }
        }

        // Handle input with timeout
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let mut state = app_state.lock().await;
                handle_key_event(key.code, &driver, &mut state, &active_jog, &jog_cancel).await?;
            }
        }
    }

    // Cleanup - stop any active jog
    *jog_cancel.lock().await = true;
    *active_jog.lock().await = None;
    sleep(Duration::from_millis(200)).await;

    // Abort and disconnect with response handling
    let _ = driver.abort().await;
    let _ = driver.disconnect().await;

    Ok(())
}

fn ui(f: &mut Frame, state: &AppState) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),  // Config panel
            Constraint::Min(10),    // Data panels
            Constraint::Length(10), // Help panel
        ])
        .split(f.area());

    // Config panel at top
    render_config_panel(f, main_chunks[0], state);

    // Split middle section into columns
    let data_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(40),  // Motion log
            Constraint::Percentage(30),  // Position & Status
            Constraint::Percentage(30),  // Errors
        ])
        .split(main_chunks[1]);

    // Render data panels
    render_motion_log(f, data_chunks[0], state);
    render_robot_data(f, data_chunks[1], state);
    render_errors(f, data_chunks[2], state);

    // Help panel at bottom
    render_help_panel(f, main_chunks[2], state);
}

fn render_config_panel(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let config_text = vec![
        Line::from(vec![
            Span::styled("Jog Speed: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:.2} mm/s", state.config.jog_speed)),
        ]),
        Line::from(vec![
            Span::styled("Step Distance: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:.2} mm", state.config.step_distance)),
        ]),
        Line::from(vec![
            Span::styled("Motion Mode: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                format!("{}", state.config.mode),
                Style::default().fg(if state.config.mode == MotionMode::Continuous {
                    Color::Yellow
                } else {
                    Color::Green
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("Active Jog: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                state.active_jog.map(|c| c.to_string()).unwrap_or_else(|| "None".to_string()),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Cyan)),
            Span::raw(&state.status_message),
        ]),
    ];

    let config_block = Paragraph::new(config_text)
        .block(Block::default().borders(Borders::ALL).title("Configuration"));
    f.render_widget(config_block, area);
}

fn render_motion_log(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let items: Vec<ListItem> = state
        .motion_log
        .iter()
        .rev() // Show newest first
        .map(|log| {
            let elapsed = log.timestamp.elapsed().unwrap_or(Duration::ZERO);
            let time_str = if elapsed.as_secs() < 60 {
                format!("{}s", elapsed.as_secs())
            } else {
                format!("{}m", elapsed.as_secs() / 60)
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", time_str),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled("✓ ", Style::default().fg(Color::Green)),
                Span::raw(&log.message),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Motion Log")
            .border_style(Style::default().fg(Color::Green))
    );
    f.render_widget(list, area);
}

fn render_robot_data(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let mut lines = vec![];

    // Position data
    lines.push(Line::from(vec![
        Span::styled("Position", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]));

    if let Some((x, y, z)) = state.current_position {
        lines.push(Line::from(vec![
            Span::styled("  X: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:.1} mm", x)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Y: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:.1} mm", y)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Z: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:.1} mm", z)),
        ]));
    } else {
        lines.push(Line::from("  No data"));
    }

    lines.push(Line::from(""));

    // Robot status
    lines.push(Line::from(vec![
        Span::styled("Robot Status", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
    ]));

    if let Some(status) = &state.robot_status {
        lines.push(Line::from(vec![
            Span::styled("  Servo: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                if status.servo_ready != 0 { "Ready" } else { "Not Ready" },
                Style::default().fg(if status.servo_ready != 0 { Color::Green } else { Color::Red })
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  TP Mode: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", status.tp_mode)),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Motion: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{}", status.motion_status)),
        ]));
    } else {
        lines.push(Line::from("  No data"));
    }

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Robot Data")
            .border_style(Style::default().fg(Color::Yellow))
    );
    f.render_widget(paragraph, area);
}

fn render_errors(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let items: Vec<ListItem> = if state.error_log.is_empty() {
        vec![ListItem::new(Line::from(vec![
            Span::styled("No errors", Style::default().fg(Color::Green)),
        ]))]
    } else {
        state
            .error_log
            .iter()
            .rev()
            .map(|error| {
                ListItem::new(Line::from(vec![
                    Span::styled("⚠ ", Style::default().fg(Color::Red)),
                    Span::raw(error),
                ]))
            })
            .collect()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Errors")
            .border_style(Style::default().fg(Color::Red))
    );
    f.render_widget(list, area);
}

fn render_help_panel(f: &mut Frame, area: ratatui::layout::Rect, state: &AppState) {
    let help_text = vec![
        Line::from(Span::styled("Motion Controls:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  k=Up(+Z)  j=Down(-Z)  h=Left(-Y)  l=Right(+Y)  f=Forward(+X)  b=Backward(-X)"),
        Line::from(""),
        Line::from(Span::styled("Configuration:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  s=Set Speed  d=Set Distance  m=Toggle Mode (Step/Continuous)"),
        Line::from(""),
        Line::from(Span::styled("Other:", Style::default().add_modifier(Modifier::BOLD))),
        Line::from("  q=Quit"),
        Line::from(""),
        Line::from(Span::styled(
            if state.config.mode == MotionMode::Continuous {
                "Continuous Mode: Press direction key to start, press again to stop"
            } else {
                "Step Mode: Press direction key for single move"
            },
            Style::default().fg(Color::Yellow),
        )),
    ];

    let help_block = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help_block, area);
}

async fn handle_key_event(
    key: KeyCode,
    driver: &FanucDriver,
    state: &mut AppState,
    active_jog: &Arc<Mutex<Option<char>>>,
    jog_cancel: &Arc<Mutex<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    match key {
        KeyCode::Char('q') => {
            state.should_quit = true;
        }
        KeyCode::Char('m') => {
            // Stop any active jog when switching modes
            *jog_cancel.lock().await = true;
            *active_jog.lock().await = None;
            state.active_jog = None;
            sleep(Duration::from_millis(100)).await;
            *jog_cancel.lock().await = false;

            state.config.mode = match state.config.mode {
                MotionMode::Step => MotionMode::Continuous,
                MotionMode::Continuous => MotionMode::Step,
            };
            state.set_status(format!("Mode changed to {}", state.config.mode));
        }
        KeyCode::Char('s') => {
            // For now, cycle through preset speeds
            state.config.jog_speed = match state.config.jog_speed as i32 {
                10 => 20.0,
                20 => 50.0,
                50 => 100.0,
                _ => 10.0,
            };
            state.set_status(format!("Speed set to {:.0} mm/s", state.config.jog_speed));
        }
        KeyCode::Char('d') => {
            // Cycle through preset distances
            state.config.step_distance = match state.config.step_distance as i32 {
                1 => 5.0,
                5 => 10.0,
                10 => 20.0,
                _ => 1.0,
            };
            state.set_status(format!("Distance set to {:.0} mm", state.config.step_distance));
        }
        KeyCode::Char(c @ ('k' | 'j' | 'h' | 'l' | 'f' | 'b')) => {
            match state.config.mode {
                MotionMode::Step => {
                    handle_jog_step(driver, c, &state.config).await?;
                    state.set_status(format!("Step {} ({:.1}mm)", get_direction_name(c), state.config.step_distance));
                }
                MotionMode::Continuous => {
                    let current_jog = *active_jog.lock().await;
                    if current_jog == Some(c) {
                        // Stop jogging
                        *jog_cancel.lock().await = true;
                        *active_jog.lock().await = None;
                        state.active_jog = None;
                        sleep(Duration::from_millis(100)).await;
                        *jog_cancel.lock().await = false;
                        state.set_status(format!("Stopped {}", get_direction_name(c)));
                    } else if current_jog.is_none() {
                        // Start jogging
                        *active_jog.lock().await = Some(c);
                        state.active_jog = Some(c);
                        handle_jog_continuous_start(driver, c, &state.config, active_jog, jog_cancel).await?;
                        state.set_status(format!("Continuous {} started", get_direction_name(c)));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
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

async fn handle_jog_step(
    driver: &FanucDriver,
    key: char,
    config: &JogConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let position = get_direction_vector(key, config.step_distance);
    let instruction = FrcLinearRelative::new(
        0,
        Configuration::default(),
        position,
        SpeedType::MMSec,
        config.jog_speed,
        TermType::FINE,
        1,
    );

    let packet = SendPacket::Instruction(Instruction::FrcLinearRelative(instruction));
    driver.send_packet(packet, PacketPriority::Immediate)
        .map_err(|e| format!("Failed to send step command: {}", e))?;

    Ok(())
}

async fn handle_jog_continuous_start(
    driver: &FanucDriver,
    key: char,
    config: &JogConfig,
    active_jog: &Arc<Mutex<Option<char>>>,
    jog_cancel: &Arc<Mutex<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let driver_clone = driver.clone();
    let config_clone = config.clone();
    let active_jog_clone = Arc::clone(active_jog);
    let jog_cancel_clone = Arc::clone(jog_cancel);
    let key_clone = key;

    tokio::spawn(async move {
        let motion_duration_ms = (config_clone.step_distance / config_clone.jog_speed * 1000.0) as u64;
        let send_interval_ms = motion_duration_ms.max(50);

        let mut tick = interval(Duration::from_millis(send_interval_ms));
        let step_per_tick = config_clone.step_distance;

        loop {
            tick.tick().await;

            // Check if cancelled or no longer active
            let cancelled = *jog_cancel_clone.lock().await;
            let current_jog = *active_jog_clone.lock().await;

            if cancelled || current_jog != Some(key_clone) {
                // Send final FINE move to stop smoothly
                let position = get_direction_vector(key_clone, step_per_tick * 0.1);
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
                let _ = driver_clone.send_packet(packet, PacketPriority::Immediate);
                break;
            }

            // Send CNT move
            let position = get_direction_vector(key_clone, step_per_tick);
            let instruction = FrcLinearRelative::new(
                0,
                Configuration::default(),
                position,
                SpeedType::MMSec,
                config_clone.jog_speed,
                TermType::CNT,
                100,
            );
            let packet = SendPacket::Instruction(Instruction::FrcLinearRelative(instruction));
            let _ = driver_clone.send_packet(packet, PacketPriority::Immediate);
        }
    });

    Ok(())
}



