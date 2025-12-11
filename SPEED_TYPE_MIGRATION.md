# Speed Type Migration Plan

## Overview
Adding `default_speed_type` field to robot connections and updating default values for safety.

## Changes Required

### 1. Database (web_server/src/database.rs)
- ✅ Added `default_speed_type` to migrations
- ✅ Updated `RobotConnection` struct
- ✅ Updated `create_robot_connection()` signature
- ✅ Updated `get_robot_connection()` query
- ✅ Updated `list_robot_connections()` query
- ✅ Updated `update_robot_connection_defaults()` signature
- ✅ Updated default values: (10, 1, 0.1, 0.25) for jog defaults

### 2. API Types (web_server/src/api_types.rs)
- [ ] Add `default_speed_type` to `RobotConnectionDto`
- [ ] Add `default_speed_type` to `CreateRobotWithConfigurations` request
- [ ] Add `default_speed_type` to `UpdateRobotConnectionDefaults` request
- [ ] Add `configurations` array to `RobotConnectionCreated` response

### 3. Handlers (web_server/src/handlers/robot_connections.rs)
- [ ] Update `create_robot_connection()` to include speed_type
- [ ] Update `create_robot_with_configurations()` to include speed_type
- [ ] Update `update_robot_connection_defaults()` to include speed_type
- [ ] Update default values to (10, 1, 0.1, 0.25)

### 4. Frontend WebSocket (web_app/src/websocket.rs)
- [ ] Add `default_speed_type` to `RobotConnectionDto`
- [ ] Add `default_speed_type` to `create_robot_with_configurations()` method
- [ ] Add `default_speed_type` to `UpdateRobotConnectionDefaults` request

### 5. Frontend Wizard (web_app/src/components/robot_creation_wizard.rs)
- [ ] Add speed_type dropdown to MotionDefaultsStep
- [ ] Update default values to (10, 1, 0.1, 0.25)
- [ ] Add speed_type to form state
- [ ] Include speed_type in submission

### 6. Frontend Settings (web_app/src/components/layout/workspace/settings.rs)
- [ ] Redesign robot details panel to show configurations
- [ ] Add configuration list/switcher
- [ ] Add ability to edit configurations
- [ ] Show speed_type in motion defaults

## Speed Type Values
From FANUC RMI documentation:
- `mmSec`: mm/sec (linear speed)
- `InchMin`: 0.1 inch/min
- `Time`: 0.1 seconds (time-based)
- `mSec`: milliseconds

## Default Values (Updated for Safety)
- Cartesian jog speed: 10.0 mm/sec (was 50.0)
- Cartesian jog step: 1.0 mm (was 10.0)
- Joint jog speed: 0.1 (was 10.0)
- Joint jog step: 0.25 degrees (was 1.0)
- Speed type: "mmSec" (new field)
- Speed: 100.0 mm/sec
- Term type: "CNT"

## API Response Fix
The `RobotConnectionCreated` response currently only returns the connection, but should also return the configurations that were created. This will allow the frontend to properly display the new robot with its configurations.

```rust
RobotConnectionCreated {
    id: i64,
    connection: RobotConnectionDto,
    configurations: Vec<RobotConfigurationDto>,  // ADD THIS
}
```

