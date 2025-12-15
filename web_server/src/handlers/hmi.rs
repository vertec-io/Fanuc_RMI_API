//! HMI Panel and Extended I/O Configuration handlers.

use crate::database::{Database, DbHmiPanel, DbIoPortConfig};
use std::sync::Arc;
use tokio::sync::Mutex;
use web_common::{
    HmiPanel, HmiPanelWithPorts, IoPortConfig, IoType, ServerResponse, WidgetType,
};

/// Convert IoType enum to database string.
fn io_type_to_str(io_type: IoType) -> &'static str {
    match io_type {
        IoType::DIN => "DIN",
        IoType::DOUT => "DOUT",
        IoType::AIN => "AIN",
        IoType::AOUT => "AOUT",
        IoType::GIN => "GIN",
        IoType::GOUT => "GOUT",
    }
}

/// Convert database string to IoType enum.
fn str_to_io_type(s: &str) -> IoType {
    match s {
        "DIN" => IoType::DIN,
        "DOUT" => IoType::DOUT,
        "AIN" => IoType::AIN,
        "AOUT" => IoType::AOUT,
        "GIN" => IoType::GIN,
        "GOUT" => IoType::GOUT,
        _ => IoType::DIN, // Default fallback
    }
}

/// Convert WidgetType enum to database string.
fn widget_type_to_str(wt: WidgetType) -> &'static str {
    match wt {
        WidgetType::Auto => "auto",
        WidgetType::Button => "button",
        WidgetType::Toggle => "toggle",
        WidgetType::Led => "led",
        WidgetType::Gauge => "gauge",
        WidgetType::Slider => "slider",
        WidgetType::Bar => "bar",
        WidgetType::Numeric => "numeric",
        WidgetType::MultiState => "multistate",
    }
}

/// Convert database string to WidgetType enum.
fn str_to_widget_type(s: &str) -> WidgetType {
    match s {
        "button" => WidgetType::Button,
        "toggle" => WidgetType::Toggle,
        "led" => WidgetType::Led,
        "gauge" => WidgetType::Gauge,
        "slider" => WidgetType::Slider,
        "bar" => WidgetType::Bar,
        "numeric" => WidgetType::Numeric,
        "multistate" => WidgetType::MultiState,
        _ => WidgetType::Auto,
    }
}

/// Convert DbIoPortConfig to IoPortConfig.
fn db_to_io_port_config(db: DbIoPortConfig) -> IoPortConfig {
    IoPortConfig {
        io_type: str_to_io_type(&db.io_type),
        io_index: db.io_index as u16,
        display_name: db.display_name,
        description: db.description,
        category: db.category,
        widget_type: str_to_widget_type(&db.widget_type),
        color_on: db.color_on,
        color_off: db.color_off,
        icon: db.icon,
        min_value: db.min_value,
        max_value: db.max_value,
        unit: db.unit,
        decimal_places: db.decimal_places as u8,
        warning_low: db.warning_low,
        warning_high: db.warning_high,
        alarm_low: db.alarm_low,
        alarm_high: db.alarm_high,
        warning_enabled: db.warning_enabled,
        alarm_enabled: db.alarm_enabled,
        hmi_enabled: db.hmi_enabled,
        hmi_x: db.hmi_x.map(|x| x as u16),
        hmi_y: db.hmi_y.map(|y| y as u16),
        hmi_width: db.hmi_width as u16,
        hmi_height: db.hmi_height as u16,
        hmi_panel_id: db.hmi_panel_id,
        is_visible: db.is_visible,
        display_order: db.display_order.map(|o| o as u16),
    }
}

/// Convert IoPortConfig to DbIoPortConfig.
fn io_port_config_to_db(config: &IoPortConfig, robot_connection_id: i64) -> DbIoPortConfig {
    DbIoPortConfig {
        id: 0, // Will be set by database
        robot_connection_id,
        io_type: io_type_to_str(config.io_type).to_string(),
        io_index: config.io_index as i32,
        display_name: config.display_name.clone(),
        description: config.description.clone(),
        category: config.category.clone(),
        widget_type: widget_type_to_str(config.widget_type).to_string(),
        color_on: config.color_on.clone(),
        color_off: config.color_off.clone(),
        icon: config.icon.clone(),
        min_value: config.min_value,
        max_value: config.max_value,
        unit: config.unit.clone(),
        decimal_places: config.decimal_places as i32,
        warning_low: config.warning_low,
        warning_high: config.warning_high,
        alarm_low: config.alarm_low,
        alarm_high: config.alarm_high,
        warning_enabled: config.warning_enabled,
        alarm_enabled: config.alarm_enabled,
        hmi_enabled: config.hmi_enabled,
        hmi_x: config.hmi_x.map(|x| x as i32),
        hmi_y: config.hmi_y.map(|y| y as i32),
        hmi_width: config.hmi_width as i32,
        hmi_height: config.hmi_height as i32,
        hmi_panel_id: config.hmi_panel_id,
        is_visible: config.is_visible,
        display_order: config.display_order.map(|o| o as i32),
    }
}

/// Convert DbHmiPanel to HmiPanel.
fn db_to_hmi_panel(db: DbHmiPanel) -> HmiPanel {
    HmiPanel {
        id: db.id,
        robot_connection_id: db.robot_connection_id,
        name: db.name,
        description: db.description,
        grid_columns: db.grid_columns as u16,
        grid_rows: db.grid_rows as u16,
        background_color: db.background_color,
        is_default: db.is_default,
    }
}

/// Convert HmiPanel to DbHmiPanel.
fn hmi_panel_to_db(panel: &HmiPanel) -> DbHmiPanel {
    DbHmiPanel {
        id: panel.id,
        robot_connection_id: panel.robot_connection_id,
        name: panel.name.clone(),
        description: panel.description.clone(),
        grid_columns: panel.grid_columns as i32,
        grid_rows: panel.grid_rows as i32,
        background_color: panel.background_color.clone(),
        is_default: panel.is_default,
    }
}

// ========== I/O Port Config Handlers ==========

/// Get all I/O port configurations for a robot.
pub async fn get_io_port_configs(
    db: Arc<Mutex<Database>>,
    robot_connection_id: i64,
) -> ServerResponse {
    let db = db.lock().await;
    match db.get_io_port_configs(robot_connection_id) {
        Ok(configs) => {
            let configs: Vec<IoPortConfig> = configs.into_iter().map(db_to_io_port_config).collect();
            ServerResponse::IoPortConfigs { configs }
        }
        Err(e) => ServerResponse::Error {
            message: format!("Failed to get I/O port configs: {}", e),
        },
    }
}

/// Save/update a single I/O port configuration.
pub async fn save_io_port_config(
    db: Arc<Mutex<Database>>,
    robot_connection_id: i64,
    config: IoPortConfig,
) -> ServerResponse {
    let db_config = io_port_config_to_db(&config, robot_connection_id);
    let db = db.lock().await;
    match db.upsert_io_port_config(robot_connection_id, &db_config) {
        Ok(_) => ServerResponse::IoPortConfigSaved { config },
        Err(e) => ServerResponse::Error {
            message: format!("Failed to save I/O port config: {}", e),
        },
    }
}

/// Save/update multiple I/O port configurations.
pub async fn save_io_port_configs(
    db: Arc<Mutex<Database>>,
    robot_connection_id: i64,
    configs: Vec<IoPortConfig>,
) -> ServerResponse {
    let db = db.lock().await;
    for config in &configs {
        let db_config = io_port_config_to_db(config, robot_connection_id);
        if let Err(e) = db.upsert_io_port_config(robot_connection_id, &db_config) {
            return ServerResponse::Error {
                message: format!("Failed to save I/O port config: {}", e),
            };
        }
    }
    // Return all configs after save
    match db.get_io_port_configs(robot_connection_id) {
        Ok(configs) => {
            let configs: Vec<IoPortConfig> = configs.into_iter().map(db_to_io_port_config).collect();
            ServerResponse::IoPortConfigs { configs }
        }
        Err(e) => ServerResponse::Error {
            message: format!("Failed to get I/O port configs after save: {}", e),
        },
    }
}

/// Delete an I/O port configuration.
pub async fn delete_io_port_config(
    db: Arc<Mutex<Database>>,
    robot_connection_id: i64,
    io_type: IoType,
    io_index: u16,
) -> ServerResponse {
    let db = db.lock().await;
    match db.delete_io_port_config(robot_connection_id, io_type_to_str(io_type), io_index as i32) {
        Ok(deleted) => {
            if deleted {
                ServerResponse::IoPortConfigDeleted { io_type, io_index }
            } else {
                ServerResponse::Error {
                    message: format!("I/O port config not found: {:?}[{}]", io_type, io_index),
                }
            }
        }
        Err(e) => ServerResponse::Error {
            message: format!("Failed to delete I/O port config: {}", e),
        },
    }
}

// ========== HMI Panel Handlers ==========

/// Get all HMI panels for a robot.
pub async fn get_hmi_panels(
    db: Arc<Mutex<Database>>,
    robot_connection_id: i64,
) -> ServerResponse {
    let db = db.lock().await;
    match db.get_hmi_panels(robot_connection_id) {
        Ok(panels) => {
            let panels: Vec<HmiPanel> = panels.into_iter().map(db_to_hmi_panel).collect();
            ServerResponse::HmiPanels { panels }
        }
        Err(e) => ServerResponse::Error {
            message: format!("Failed to get HMI panels: {}", e),
        },
    }
}

/// Get an HMI panel with its configured ports.
pub async fn get_hmi_panel_with_ports(
    db: Arc<Mutex<Database>>,
    panel_id: i64,
) -> ServerResponse {
    let db = db.lock().await;

    // Get the panel by ID
    match db.get_hmi_panel_by_id(panel_id) {
        Ok(Some(db_panel)) => {
            // Get ports for this panel
            match db.get_io_port_configs_for_panel(panel_id) {
                Ok(ports) => {
                    let panel = HmiPanelWithPorts {
                        panel: db_to_hmi_panel(db_panel),
                        ports: ports.into_iter().map(db_to_io_port_config).collect(),
                    };
                    ServerResponse::HmiPanelWithPorts { panel }
                }
                Err(e) => ServerResponse::Error {
                    message: format!("Failed to get panel ports: {}", e),
                },
            }
        }
        Ok(None) => ServerResponse::Error {
            message: format!("HMI panel not found: {}", panel_id),
        },
        Err(e) => ServerResponse::Error {
            message: format!("Failed to get HMI panel: {}", e),
        },
    }
}

/// Save/update an HMI panel.
pub async fn save_hmi_panel(
    db: Arc<Mutex<Database>>,
    panel: HmiPanel,
) -> ServerResponse {
    let db_panel = hmi_panel_to_db(&panel);
    let db = db.lock().await;
    match db.upsert_hmi_panel(&db_panel) {
        Ok(id) => {
            let saved_panel = HmiPanel {
                id,
                ..panel
            };
            ServerResponse::HmiPanelSaved { panel: saved_panel }
        }
        Err(e) => ServerResponse::Error {
            message: format!("Failed to save HMI panel: {}", e),
        },
    }
}

/// Delete an HMI panel.
pub async fn delete_hmi_panel(
    db: Arc<Mutex<Database>>,
    panel_id: i64,
) -> ServerResponse {
    let db = db.lock().await;
    match db.delete_hmi_panel(panel_id) {
        Ok(deleted) => {
            if deleted {
                ServerResponse::HmiPanelDeleted { panel_id }
            } else {
                ServerResponse::Error {
                    message: format!("HMI panel not found: {}", panel_id),
                }
            }
        }
        Err(e) => ServerResponse::Error {
            message: format!("Failed to delete HMI panel: {}", e),
        },
    }
}
