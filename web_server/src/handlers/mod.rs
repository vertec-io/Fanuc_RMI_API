//! API request handlers for WebSocket messages.
//!
//! This module contains handlers organized by functionality:
//! - `connection`: Robot connection management (connect/disconnect/status)
//! - `execution`: Program execution (start/pause/resume/stop)
//! - `programs`: Program CRUD operations
//! - `settings`: Robot settings management
//! - `robot_connections`: Saved connection configurations CRUD
//! - `frame_tool`: Frame and tool data management
//! - `io`: Digital I/O management (DIN/DOUT)

pub mod connection;
pub mod execution;
pub mod frame_tool;
pub mod io;
pub mod programs;
pub mod robot_connections;
pub mod settings;

use crate::api_types::*;
use crate::database::Database;
use crate::program_executor::ProgramExecutor;
use crate::RobotConnection;
use fanuc_rmi::drivers::FanucDriver;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio_tungstenite::tungstenite::Message;

/// Type alias for the WebSocket sender
pub type WsSender = Arc<Mutex<futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    Message
>>>;

/// Handle a client API request and return a response.
pub async fn handle_request(
    request: ClientRequest,
    db: Arc<Mutex<Database>>,
    driver: Option<Arc<FanucDriver>>,
    executor: Option<Arc<Mutex<ProgramExecutor>>>,
    ws_sender: Option<WsSender>,
    robot_connection: Option<Arc<RwLock<RobotConnection>>>,
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

        // Program execution
        ClientRequest::StartProgram { program_id } => {
            execution::start_program(db, driver, executor, program_id, ws_sender).await
        }
        ClientRequest::PauseProgram => execution::pause_program(driver, executor).await,
        ClientRequest::ResumeProgram => execution::resume_program(driver, executor).await,
        ClientRequest::StopProgram => execution::stop_program(driver, executor).await,

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
            frame_tool::set_active_frame_tool(robot_connection, uframe, utool).await
        }
        ClientRequest::ReadFrameData { frame_number } => {
            frame_tool::read_frame_data(robot_connection, frame_number).await
        }
        ClientRequest::ReadToolData { tool_number } => {
            frame_tool::read_tool_data(robot_connection, tool_number).await
        }
        ClientRequest::WriteFrameData { frame_number, x, y, z, w, p, r } => {
            frame_tool::write_frame_data(robot_connection, frame_number, x, y, z, w, p, r).await
        }
        ClientRequest::WriteToolData { tool_number, x, y, z, w, p, r } => {
            frame_tool::write_tool_data(robot_connection, tool_number, x, y, z, w, p, r).await
        }

        // I/O management
        ClientRequest::ReadDin { port_number } => {
            io::read_din(robot_connection, port_number).await
        }
        ClientRequest::WriteDout { port_number, port_value } => {
            io::write_dout(robot_connection, port_number, port_value).await
        }
        ClientRequest::ReadDinBatch { port_numbers } => {
            io::read_din_batch(robot_connection, port_numbers).await
        }
    }
}

