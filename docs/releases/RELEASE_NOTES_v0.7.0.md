# Release Notes v0.7.0 - I/O Panel & Modular Refactoring

**Release Date**: 2025-12-01  
**Status**: ✅ Production Ready

---

## 🎯 Overview

This release adds comprehensive I/O support, implements reserved future features, and completes the modular refactoring of both frontend and backend codebases.

---

## ✨ New Features

### I/O Panel
- **Digital Inputs (DIN)**: Real-time reading of digital input ports
- **Digital Outputs (DOUT)**: Read/write with toggle controls
- **Batch Reading**: Efficient multi-port reads for I/O status
- **Refresh Control**: Manual refresh button for I/O state
- **Cache Management**: Clear and update I/O cache

### 6-DOF Position Display
- **Full Orientation**: Now shows W, P, R rotation angles alongside X, Y, Z position
- **Real-time Updates**: Live position and orientation from robot

### Toast Notifications
- **Success Toasts**: Auto-dismiss after 5 seconds (green accent)
- **Error Toasts**: Auto-dismiss after 8 seconds (red accent)
- **Position**: Bottom-left to avoid jog controls overlay
- **Dismissable**: Click to dismiss early

### Execution Progress
- **Progress Bar**: Visual progress during program execution
- **Status Indicator**: Shows running/paused/error states
- **Error Display**: Inline error messages in progress bar

### Settings Enhancements
- **Danger Zone Panel**: Database reset with confirmation dialog
- **Safety Confirmation**: Two-step confirmation to prevent accidents

### Dashboard Changes
- **Control Tab Default**: Control tab is now the default (previously Info)
- **Tab Order**: Control, Info (swapped from Info, Control)

---

## 🔧 Technical Improvements

### Frontend Modular Refactoring
Refactored `control.rs` (~1063 lines) into modular structure:
```
control/
├── mod.rs              # Main ControlPanel component
├── quick_commands.rs   # Quick command buttons (Home, Zero, etc.)
├── command_input.rs    # Command input field with send
├── command_log.rs      # Console output panel with clear
├── program_display.rs  # Program table and progress bar
├── load_modal.rs       # Load program modal
└── composer.rs         # Motion composer panel
```

### Backend Modular Handlers
```
handlers/
├── mod.rs               # Handler exports
├── connection.rs        # Robot connection management
├── execution.rs         # Program execution control
├── programs.rs          # Program CRUD operations
├── settings.rs          # Settings management
├── robot_connections.rs # Saved connection profiles
├── frame_tool.rs        # Frame/Tool RMI commands
└── io.rs                # Digital I/O (NEW)
```

### Dead Code Cleanup
- Removed `#[allow(dead_code)]` from all actively used methods
- Connected all reserved signals to UI components
- Only remaining dead code is in `sim` kinematics module (unrelated)

---

## 📦 API Changes

### New ClientRequest Variants
- `ReadDin { port_number }` - Read single DIN
- `WriteDout { port_number, port_value }` - Write DOUT
- `ReadDinBatch { port_numbers }` - Read multiple DINs

### New ServerResponse Variants
- `DinValue { port_number, port_value }` - Single DIN value
- `DinBatch { values }` - Batch DIN values

---

## 🚀 Running the Application

```bash
# Terminal 1: Start simulator
cargo run -p sim -- --realtime

# Terminal 2: Start web server
cargo run -p web_server

# Terminal 3: Build and serve frontend
cd web_app && trunk serve --open
```

Open browser to `http://localhost:8080`

---

## 📋 Known Limitations

- No responsive design for tablet/mobile
- Programs are read-only after upload (no inline editing)
- Analog I/O (AIN/AOUT) not yet implemented
- Group I/O not yet implemented

---

## 📚 Related Documentation

- [Web Interface Implementation](../WEB_INTERFACE_IMPLEMENTATION.md)
- [Implementation Roadmap V2](../IMPLEMENTATION_ROADMAP_V2.md)
- [v0.6.0 Release Notes](RELEASE_NOTES_v0.6.0.md) - Previous version

