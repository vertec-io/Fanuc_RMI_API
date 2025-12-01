//! API request handlers for WebSocket messages.
//!
//! This module contains handlers organized by functionality:
//! - `connection`: Robot connection management (connect/disconnect/status)
//! - `control`: Control locking (request/release control)
//! - `execution`: Program execution (start/pause/resume/stop)
//! - `programs`: Program CRUD operations
//! - `settings`: Robot settings management
//! - `robot_connections`: Saved connection configurations CRUD
//! - `frame_tool`: Frame and tool data management
//! - `io`: Digital I/O management (DIN/DOUT)

pub mod connection;
pub mod control;
pub mod execution;
pub mod frame_tool;
pub mod io;
pub mod programs;
pub mod robot_connections;
pub mod settings;

use crate::api_types::*;
use crate::database::Database;
use crate::program_executor::ProgramExecutor;
use crate::session::ClientManager;
use crate::RobotConnection;
use fanuc_rmi::drivers::FanucDriver;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

/// Type alias for the WebSocket sender
pub type WsSender = Arc<Mutex<futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    Message
>>>;

/// Check if the client has control of the robot.
/// Returns Ok(()) if the client has control, or an error response if not.
/// Also updates the activity timestamp to prevent timeout.
async fn require_control(
    client_manager: &Option<Arc<ClientManager>>,
    client_id: Option<Uuid>,
) -> Result<(), ServerResponse> {
    match (client_manager, client_id) {
        (Some(cm), Some(id)) => {
            if cm.has_control(id).await {
                // Update activity timestamp to prevent timeout
                cm.touch_control(id).await;
                Ok(())
            } else {
                let holder = cm.get_control_holder().await;
                Err(ServerResponse::ControlDenied {
                    holder_id: holder.map(|h| h.to_string()).unwrap_or_default(),
                    reason: "You do not have control of the robot. Request control first.".to_string(),
                })
            }
        }
        (None, _) => {
            // No client manager = no control locking (single client mode)
            Ok(())
        }
        (Some(_), None) => {
            Err(ServerResponse::Error {
                message: "Client ID not available".to_string(),
            })
        }
    }
}

/// Handle a client API request and return a response.
pub async fn handle_request(
    request: ClientRequest,
    db: Arc<Mutex<Database>>,
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    ws_sender: Option<WsSender>,
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
    client_manager: Option<Arc<ClientManager>>,
    client_id: Option<uuid::Uuid>,
) -> ServerResponse {
    match request {
        // Program management
        ClientRequest::ListPrograms => programs::list_programs(db).await,
        ClientRequest::GetProgram { id } => programs::get_program(db, id).await,
        ClientRequest::CreateProgram { name, description } => {
            programs::create_program(db, &name, description.as_deref()).await
        }
        ClientRequest::DeleteProgram { id } => programs::delete_program(db, id).await,
        ClientRequest::UploadCsv { program_id, csv_content, start_position } => {
            programs::upload_csv(db, program_id, &csv_content, start_position).await
        }

        // Settings management
        ClientRequest::GetSettings => settings::get_settings(db).await,
        ClientRequest::UpdateSettings {
            default_w, default_p, default_r,
            default_speed, default_term_type,
            default_uframe, default_utool,
        } => {
            settings::update_settings(
                db, default_w, default_p, default_r,
                default_speed, &default_term_type,
                default_uframe, default_utool,
            ).await
        }
        ClientRequest::ResetDatabase => settings::reset_database(db).await,

        // Program execution (requires control)
        ClientRequest::StartProgram { program_id } => {
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            execution::start_program(db, driver, executor, program_id, ws_sender, client_manager).await
        }
        ClientRequest::PauseProgram => {
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            execution::pause_program(driver, executor, client_manager).await
        }
        ClientRequest::ResumeProgram => {
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            execution::resume_program(driver, executor, client_manager).await
        }
        ClientRequest::StopProgram => {
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            execution::stop_program(driver, executor, client_manager).await
        }
        ClientRequest::GetExecutionState => execution::get_execution_state(executor).await,

        // Robot connection management
        ClientRequest::GetConnectionStatus => {
            connection::get_connection_status(robot_connection).await
        }
        ClientRequest::ConnectRobot { robot_addr, robot_port } => {
            connection::connect_robot(robot_connection, robot_addr, robot_port).await
        }
        ClientRequest::ConnectToSavedRobot { connection_id } => {
            connection::connect_to_saved_robot(db, robot_connection, connection_id).await
        }
        ClientRequest::DisconnectRobot => {
            connection::disconnect_robot(robot_connection).await
        }

        // Saved robot connections CRUD
        ClientRequest::ListRobotConnections => {
            robot_connections::list_robot_connections(db).await
        }
        ClientRequest::GetRobotConnection { id } => {
            robot_connections::get_robot_connection(db, id).await
        }
        ClientRequest::CreateRobotConnection { name, description, ip_address, port } => {
            robot_connections::create_robot_connection(db, &name, description.as_deref(), &ip_address, port).await
        }
        ClientRequest::UpdateRobotConnection { id, name, description, ip_address, port } => {
            robot_connections::update_robot_connection(db, id, &name, description.as_deref(), &ip_address, port).await
        }
        ClientRequest::UpdateRobotConnectionDefaults { id, default_speed, default_term_type, default_uframe, default_utool, default_w, default_p, default_r } => {
            robot_connections::update_robot_connection_defaults(db, id, default_speed, default_term_type.as_deref(), default_uframe, default_utool, default_w, default_p, default_r).await
        }
        ClientRequest::DeleteRobotConnection { id } => {
            robot_connections::delete_robot_connection(db, id).await
        }

        // Frame/Tool management
        ClientRequest::GetActiveFrameTool => {
            frame_tool::get_active_frame_tool(robot_connection).await
        }
        ClientRequest::SetActiveFrameTool { uframe, utool } => {
            // Requires control - changes robot configuration
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            frame_tool::set_active_frame_tool(robot_connection, uframe, utool).await
        }
        ClientRequest::ReadFrameData { frame_number } => {
            frame_tool::read_frame_data(robot_connection, frame_number).await
        }
        ClientRequest::ReadToolData { tool_number } => {
            frame_tool::read_tool_data(robot_connection, tool_number).await
        }
        ClientRequest::WriteFrameData { frame_number, x, y, z, w, p, r } => {
            // Requires control - modifies robot data
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            frame_tool::write_frame_data(robot_connection, frame_number, x, y, z, w, p, r).await
        }
        ClientRequest::WriteToolData { tool_number, x, y, z, w, p, r } => {
            // Requires control - modifies robot data
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            frame_tool::write_tool_data(robot_connection, tool_number, x, y, z, w, p, r).await
        }

        // I/O management - Digital
        ClientRequest::ReadDin { port_number } => {
            io::read_din(robot_connection, port_number).await
        }
        ClientRequest::WriteDout { port_number, port_value } => {
            // Requires control - modifies robot outputs
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            io::write_dout(robot_connection, port_number, port_value).await
        }
        ClientRequest::ReadDinBatch { port_numbers } => {
            io::read_din_batch(robot_connection, port_numbers).await
        }

        // I/O management - Analog
        ClientRequest::ReadAin { port_number } => {
            io::read_ain(robot_connection, port_number).await
        }
        ClientRequest::WriteAout { port_number, port_value } => {
            // Requires control - modifies robot outputs
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            io::write_aout(robot_connection, port_number, port_value).await
        }

        // I/O management - Group
        ClientRequest::ReadGin { port_number } => {
            io::read_gin(robot_connection, port_number).await
        }
        ClientRequest::WriteGout { port_number, port_value } => {
            // Requires control - modifies robot outputs
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            io::write_gout(robot_connection, port_number, port_value).await
        }

        // Control locking
        ClientRequest::RequestControl => {
            control::request_control(client_manager, client_id).await
        }
        ClientRequest::ReleaseControl => {
            control::release_control(client_manager, client_id).await
        }
        ClientRequest::GetControlStatus => {
            control::get_control_status(client_manager, client_id).await
        }
    }
}

