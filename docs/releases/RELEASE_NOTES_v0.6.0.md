# Release Notes v0.6.0 - Web Interface Overhaul

**Release Date**: 2025-12-01
**Status**: ✅ Complete (Superseded by v0.7.0)

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

## 📚 Related Documentation

- [Web Interface Implementation](../WEB_INTERFACE_IMPLEMENTATION.md)
- [v0.7.0 Release Notes](RELEASE_NOTES_v0.7.0.md) - Latest version

