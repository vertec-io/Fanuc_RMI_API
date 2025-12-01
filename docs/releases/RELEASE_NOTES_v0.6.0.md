# Release Notes v0.6.0 - Web Interface Overhaul

**Release Date**: 2025-12-01  
**Status**: ✅ Complete

---

## 🎯 Overview

Major web interface overhaul with a professional desktop-style UI for robot control and program management. This release transforms the web application into a production-grade control interface.

---

## ✨ New Features

### Desktop-Style Layout
- **Left Navbar**: Navigation between Dashboard, Programs, and Settings views
- **Main Workspace**: Routed content area with tab support
- **Right Panel**: Always-visible position display, error log, and jog controls
- **Top Bar**: Connection status, robot info, and quick actions

### Program Management
- **File Menu**: New, Open, Save As, Upload CSV, Close
- **View Menu**: Toggle Program Browser visibility
- **Program Browser**: List of saved programs with metadata
- **Program Display**: G-code style line-by-line view with execution highlighting
- **Execution Controls**: Run, Pause, Stop buttons with real-time progress

### Jog Controls
- **Docked Mode**: Embedded in right panel
- **Floating Mode**: Draggable window (header-only drag)
- **Persistent Settings**: Speed and step values persist between modes
- **Axis Controls**: X, Y, Z, W, P, R with +/- buttons

### Settings View
- **Connection Settings**: Robot IP, port, connect/disconnect
- **Saved Connections**: CRUD for robot connection profiles
- **Default Settings**: Speed, termination type, frame/tool defaults

### Backend Improvements
- **SQLite Database**: Persistent storage for programs, settings, and connections
- **Buffered Execution**: 5-instruction buffer for smooth motion
- **Progress Tracking**: Real-time line-by-line progress updates
- **CSV Parser**: Support for minimal (x,y,z,speed) and full (13 column) formats

---

## 🔧 Technical Details

### Frontend (web_app)
- Leptos 0.8.x with signal-based reactivity
- leptos_router for view navigation
- leptos-use for drag functionality
- Tailwind CSS dark theme

### Backend (web_server)
- Axum async web server
- SQLite with rusqlite
- tokio-tungstenite WebSocket
- bincode + serde_json serialization

---

## 📁 File Structure

```
web_app/
├── src/
│   ├── lib.rs              # App entry point
│   ├── websocket.rs        # WebSocket manager
│   └── components/
│       ├── jog_controls.rs
│       ├── position_display.rs
│       ├── robot_status.rs
│       ├── error_log.rs
│       └── layout/
│           ├── mod.rs           # LayoutContext
│           ├── left_navbar.rs
│           ├── top_bar.rs
│           ├── right_panel.rs
│           └── main_workspace.rs

web_server/
├── src/
│   ├── main.rs             # Server entry
│   ├── api_handler.rs      # Request processing
│   ├── api_types.rs        # ClientRequest/ServerResponse
│   ├── database.rs         # SQLite operations
│   ├── program_parser.rs   # CSV parsing
│   └── program_executor.rs # Buffered execution
```

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

- Frame/Tool RMI commands not exposed in web API (available in library)
- No responsive design for tablet/mobile
- Programs are read-only after upload (no inline editing)

---

## 📚 Related Documentation

- [Web Interface Implementation](../WEB_INTERFACE_IMPLEMENTATION.md)
- [Implementation Roadmap V2](../IMPLEMENTATION_ROADMAP_V2.md)
- [UI Design Mockup](../UI_DESIGN_MOCKUP.md)

