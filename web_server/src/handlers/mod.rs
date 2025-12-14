//! API request handlers for WebSocket messages.
//!
//! This module contains handlers organized by functionality:
//! - `configurations`: Robot configuration management (named configs per robot)
//! - `connection`: Robot connection management (connect/disconnect/status)
//! - `control`: Control locking (request/release control)
//! - `execution`: Program execution (start/pause/resume/stop)
//! - `programs`: Program CRUD operations
//! - `settings`: Robot settings management
//! - `robot_connections`: Saved connection configurations CRUD
//! - `frame_tool`: Frame and tool data management
//! - `io`: Digital I/O management (DIN/DOUT/AIN/AOUT/GIN/GOUT)
//! - `io_config`: I/O display configuration management
//! - `robot_control`: Robot control commands (abort/reset/initialize)

pub mod configurations;
pub mod connection;
pub mod control;
pub mod execution;
pub mod frame_tool;
pub mod io;
pub mod io_config;
pub mod programs;
pub mod robot_connections;
pub mod robot_control;
pub mod settings;

use crate::api_types::*;
use crate::database::Database;
use crate::program_executor::ProgramExecutor;
use crate::session::ClientManager;
use crate::RobotConnection;
use fanuc_rmi::drivers::FanucDriver;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

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
        ClientRequest::UpdateProgramSettings {
            program_id, start_x, start_y, start_z, end_x, end_y, end_z, move_speed
        } => {
            programs::update_program_settings(
                db, program_id, start_x, start_y, start_z, end_x, end_y, end_z, move_speed
            ).await
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
        ClientRequest::LoadProgram { program_id } => {
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            execution::load_program(db, executor, program_id, robot_connection, client_manager).await
        }
        ClientRequest::UnloadProgram => {
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            execution::unload_program(driver, executor, client_manager).await
        }
        ClientRequest::StartProgram { program_id } => {
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            execution::start_program(db, driver, executor, program_id, robot_connection, client_manager).await
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
            execution::stop_program(driver, executor, robot_connection, client_manager).await
        }
        ClientRequest::GetExecutionState => execution::get_execution_state(executor).await,

        // Robot control commands (requires control)
        ClientRequest::RobotAbort => {
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            robot_control::robot_abort(driver, executor, robot_connection, client_manager).await
        }
        ClientRequest::RobotReset => {
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            robot_control::robot_reset(driver).await
        }
        ClientRequest::RobotInitialize { group_mask } => {
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            robot_control::robot_initialize(driver, robot_connection, client_manager, group_mask.unwrap_or(1)).await
        }

        // Robot connection management
        ClientRequest::GetConnectionStatus => {
            connection::get_connection_status(robot_connection).await
        }
        ClientRequest::ConnectRobot { robot_addr, robot_port } => {
            // Requires control - changes which robot the server is connected to
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            connection::connect_robot(robot_connection, robot_addr, robot_port).await
        }
        ClientRequest::ConnectToSavedRobot { connection_id } => {
            // Requires control - changes which robot the server is connected to
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            connection::connect_to_saved_robot(db, robot_connection, client_manager, connection_id).await
        }
        ClientRequest::DisconnectRobot => {
            // Requires control - disconnects the robot
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
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
        ClientRequest::CreateRobotWithConfigurations {
            name, description, ip_address, port,
            default_speed, default_speed_type, default_term_type, default_w, default_p, default_r,
            default_cartesian_jog_speed, default_cartesian_jog_step,
            default_joint_jog_speed, default_joint_jog_step,
            configurations,
        } => {
            robot_connections::create_robot_with_configurations(
                db,
                &name,
                description.as_deref(),
                &ip_address,
                port,
                default_speed,
                &default_speed_type,
                &default_term_type,
                default_w,
                default_p,
                default_r,
                default_cartesian_jog_speed,
                default_cartesian_jog_step,
                default_joint_jog_speed,
                default_joint_jog_step,
                configurations,
            ).await
        }
        ClientRequest::UpdateRobotConnection { id, name, description, ip_address, port } => {
            robot_connections::update_robot_connection(db, id, &name, description.as_deref(), &ip_address, port).await
        }
        ClientRequest::UpdateRobotConnectionDefaults {
            id, default_speed, default_speed_type, default_term_type,
            default_w, default_p, default_r,
        } => {
            robot_connections::update_robot_connection_defaults(
                db, id, default_speed, &default_speed_type, &default_term_type,
                default_w, default_p, default_r,
            ).await
        }
        ClientRequest::DeleteRobotConnection { id } => {
            robot_connections::delete_robot_connection(db, id).await
        }
        ClientRequest::UpdateRobotJogDefaults { id, cartesian_jog_speed, cartesian_jog_step, joint_jog_speed, joint_jog_step } => {
            robot_connections::update_robot_jog_defaults(db, id, cartesian_jog_speed, cartesian_jog_step, joint_jog_speed, joint_jog_step).await
        }
        ClientRequest::UpdateJogControls { cartesian_jog_speed, cartesian_jog_step, joint_jog_speed, joint_jog_step } => {
            // Requires control - changes active jog controls (from Control panel)
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            robot_connections::update_jog_controls(robot_connection, client_manager, cartesian_jog_speed, cartesian_jog_step, joint_jog_speed, joint_jog_step).await
        }
        ClientRequest::ApplyJogSettings { cartesian_jog_speed, cartesian_jog_step, joint_jog_speed, joint_jog_step } => {
            // Requires control - applies jog defaults (from Configuration panel)
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            robot_connections::apply_jog_settings(robot_connection, client_manager, cartesian_jog_speed, cartesian_jog_step, joint_jog_speed, joint_jog_step).await
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
            frame_tool::set_active_frame_tool(robot_connection, client_manager, uframe, utool).await
        }
        ClientRequest::ReadFrameData { frame_number } => {
            frame_tool::read_frame_data(robot_connection, frame_number).await
        }
        ClientRequest::ReadToolData { tool_number } => {
            frame_tool::read_tool_data(robot_connection, tool_number).await
        }
        ClientRequest::WriteFrameData { frame_number, data } => {
            // Requires control - modifies robot data
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            frame_tool::write_frame_data(robot_connection, frame_number, data.into()).await
        }
        ClientRequest::WriteToolData { tool_number, data } => {
            // Requires control - modifies robot data
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            frame_tool::write_tool_data(robot_connection, tool_number, data.into()).await
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
            let response = io::write_dout(robot_connection, port_number, port_value).await;
            // Broadcast successful I/O changes to all clients
            if matches!(response, ServerResponse::DoutValue { .. }) {
                if let Some(ref cm) = client_manager {
                    cm.broadcast_all(&response).await;
                }
            }
            response
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
            let response = io::write_aout(robot_connection, port_number, port_value).await;
            // Broadcast successful I/O changes to all clients
            if matches!(response, ServerResponse::AoutValue { .. }) {
                if let Some(ref cm) = client_manager {
                    cm.broadcast_all(&response).await;
                }
            }
            response
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
            let response = io::write_gout(robot_connection, port_number, port_value).await;
            // Broadcast successful I/O changes to all clients
            if matches!(response, ServerResponse::GoutValue { .. }) {
                if let Some(ref cm) = client_manager {
                    cm.broadcast_all(&response).await;
                }
            }
            response
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

        // I/O Configuration
        ClientRequest::GetIoConfig { robot_connection_id } => {
            io_config::get_io_config(db, robot_connection_id).await
        }
        ClientRequest::UpdateIoConfig {
            robot_connection_id,
            io_type,
            io_index,
            display_name,
            is_visible,
            display_order,
        } => {
            io_config::update_io_config(
                db,
                robot_connection_id,
                io_type,
                io_index,
                display_name,
                is_visible,
                display_order,
            ).await
        }

        // Robot Configurations
        ClientRequest::ListRobotConfigurations { robot_connection_id } => {
            configurations::list_robot_configurations(db, robot_connection_id).await
        }
        ClientRequest::GetRobotConfiguration { id } => {
            configurations::get_robot_configuration(db, id).await
        }
        ClientRequest::CreateRobotConfiguration {
            robot_connection_id,
            name,
            is_default,
            u_frame_number,
            u_tool_number,
            front,
            up,
            left,
            flip,
            turn4,
            turn5,
            turn6,
        } => {
            configurations::create_robot_configuration(
                db,
                robot_connection_id,
                name,
                is_default,
                u_frame_number,
                u_tool_number,
                front,
                up,
                left,
                flip,
                turn4,
                turn5,
                turn6,
            ).await
        }
        ClientRequest::UpdateRobotConfiguration {
            id,
            name,
            is_default,
            u_frame_number,
            u_tool_number,
            front,
            up,
            left,
            flip,
            turn4,
            turn5,
            turn6,
        } => {
            configurations::update_robot_configuration(
                db,
                id,
                name,
                is_default,
                u_frame_number,
                u_tool_number,
                front,
                up,
                left,
                flip,
                turn4,
                turn5,
                turn6,
            ).await
        }
        ClientRequest::DeleteRobotConfiguration { id } => {
            configurations::delete_robot_configuration(db, id).await
        }
        ClientRequest::SetDefaultRobotConfiguration { id } => {
            configurations::set_default_robot_configuration(db, id).await
        }
        ClientRequest::GetActiveConfiguration => {
            configurations::get_active_configuration(robot_connection).await
        }
        ClientRequest::LoadConfiguration { configuration_id } => {
            configurations::load_configuration(db, robot_connection, client_manager, configuration_id).await
        }
        ClientRequest::SaveCurrentConfiguration { configuration_name } => {
            // Requires control - saves configuration to database
            if let Err(e) = require_control(&client_manager, client_id).await {
                return e;
            }
            configurations::save_current_configuration(db, robot_connection, client_manager, configuration_name).await
        }
    }
}

