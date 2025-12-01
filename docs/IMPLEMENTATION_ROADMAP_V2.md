# FANUC RMI Web Interface - Implementation Roadmap v2.0

**Date**: 2025-11-29  
**Version**: 2.0 (Desktop Application Style)  
**Target Completion**: 3-4 weeks

---

## 🎯 Vision

Transform the FANUC RMI web interface into a **professional desktop-style application** for comprehensive robot control and monitoring.

**Key Design Principles**:
- ✅ **App-like, not webpage-like** - Full-window layout, minimal scrolling
- ✅ **Always-visible essentials** - Jog controls, position, status always accessible  
- ✅ **Dense but clean** - Efficient space usage with modern aesthetic
- ✅ **Tooltips over descriptions** - Keep UI uncluttered
- ✅ **Responsive design** - Adapts to desktop/tablet/mobile
- ✅ **Legacy preserved** - Original simple interface remains accessible at `/`

---

## 📐 Layout Architecture

### Desktop Layout (>1280px)

```
┌────────────────────────────────────────────────────────────────────┐
│ HEADER: [WS ●] [Robot ●] [Robot IP]              [⚙ Settings]     │
├────┬──────────────────────────────────────────────────┬────────────┤
│NAV │              MAIN WORKSPACE (TABS)               │  ALWAYS    │
│BAR │                                                  │  VISIBLE   │
│    │  Tab 1: Dashboard | Tab 2: Control              │            │
│[📊]│                                                  │ Position   │
│DASH│  (Content changes based on active tab)          │ Errors     │
│    │                                                  │ I/O Status │
│[🎮]│                                                  │ Jog Ctrl   │
│CTRL│                                                  │ (poppable) │
│    │                                                  │            │
│[⚙] │                                                  │            │
│SET │                                                  │            │
└────┴──────────────────────────────────────────────────┴────────────┘
```

**Dimensions**:
- Left navbar: 60-80px (icon + label)
- Right sidebar: 280-320px (Dashboard route only)
- Center workspace: Flexible (fills remaining space)
- Header: 56px (fixed)
- Total height: 100vh (no scrolling except logs)

**Right Sidebar Visibility**:
- **Dashboard (DASH)**: Right sidebar VISIBLE (Position, Errors, I/O, Jog)
- **Control (CTRL)**: Right sidebar HIDDEN (full-width workspace)
- **Settings (SET)**: Right sidebar HIDDEN (full-width workspace)

**Jog Controls - Poppable Widget**:
- Default: Embedded in right sidebar (compact)
- Popped: Draggable floating window, larger size
- **Persistence**: State/position persists across ALL navbar routes
- **Visibility**: Only visible in Dashboard (DASH) route
- **Behavior**: When popped, other sidebar panels expand to fill space
- **Interaction**: Free-floating, snaps to edges, clicks don't pass through

---

## 🗂️ Navigation Structure

### Left Navbar (3 Items)

1. **📊 DASH** - Dashboard (Tab 1: Info, Tab 2: Control)
2. **🎮 CTRL** - Control (Program management - future)
3. **⚙ SET** - Settings (Connection, programs, preferences)

### Right Sidebar (Dashboard Route Only)

1. **Position Display** - Current X,Y,Z,W,P,R + active frame/tool
2. **Errors Panel** - Recent errors/warnings
3. **I/O Status** - Digital I/O summary
4. **Jog Controls** - Jog pad (poppable)

**Note**: Right sidebar is ONLY visible in Dashboard (DASH) route. CTRL and SET routes have full-width workspace.

---

## 📋 Phase Overview

| Phase | Duration | Focus |
|-------|----------|-------|
| **Phase 1** | 1-2 days | Backend: Frame/Tool RMI commands, nalgebra integration |
| **Phase 2** | 2-3 days | Backend: Multi-frame transforms, program execution |
| **Phase 3** | 3-4 days | Frontend: Desktop UI layout, Dashboard tabs |
| **Phase 4** | 2-3 days | Frontend: Settings, program management, CSV upload |
| **Phase 5** | 2-3 days | Database, polish, testing, deployment |

**Total**: ~12-17 days (2.5-3.5 weeks)

---

## 📦 Technology Stack

**Frontend**:
- Leptos 0.6.15 (Rust WASM framework)
- Tailwind CSS (dark futuristic theme)
- Leptos Router (for `/` legacy vs `/v2` new app)

**Backend**:
- fanuc_rmi (with nalgebra-support feature)
- Axum web server
- SQLite3 database (programs, settings)

**Build**:
- Trunk (WASM bundler)
- bincode (binary WebSocket protocol)

---

## 🎨 Design System (Preserved)

**Colors**:
```css
--bg-app: #0a0a0a;          /* Main background */
--bg-panel: #111111;        /* Panels/cards */
--bg-nested: #1a1a1a;       /* Nested elements */
--accent: #00d9ff;          /* Cyan - highlights */
--text-primary: #ffffff;    /* Primary text */
--text-secondary: #cccccc;  /* Secondary text */
--text-tertiary: #888888;   /* Labels */
--border: #ffffff10;        /* 10% white */
```

**Component Patterns**: See [UI_DESIGN_MOCKUP.md](UI_DESIGN_MOCKUP.md)

---

## 🚀 Phase 1: Backend Foundation (1-2 days)

### Goal
Implement frame/tool awareness, nalgebra integration, and missing RMI commands.

### Tasks

#### 1.1: nalgebra Feature Flag

**File**: `fanuc_rmi/Cargo.toml`

```toml
[dependencies]
nalgebra = { version = "0.33", optional = true }

[features]
nalgebra-support = ["nalgebra"]
```

#### 1.2: Position Type Updates

**File**: `fanuc_rmi/src/lib.rs`

Make `ext1`, `ext2`, `ext3` optional:

```rust
#[cfg_attr(feature = "DTO", mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[serde(default)]
    pub w: f64,
    #[serde(default)]
    pub p: f64,
    #[serde(default)]
    pub r: f64,
    #[serde(default)]
    pub ext1: f64,
    #[serde(default)]
    pub ext2: f64,
    #[serde(default)]
    pub ext3: f64,
}
```

#### 1.3: nalgebra Conversions

**File**: `fanuc_rmi/src/transforms.rs` (new file)

```rust
#[cfg(feature = "nalgebra-support")]
use nalgebra::{Isometry3, Translation3, UnitQuaternion};
use crate::Position;

#[cfg(feature = "nalgebra-support")]
impl From<Position> for Isometry3<f64> {
    fn from(pos: Position) -> Self {
        let translation = Translation3::new(pos.x, pos.y, pos.z);
        let rotation = UnitQuaternion::from_euler_angles(
            pos.w.to_radians(),
            pos.p.to_radians(),
            pos.r.to_radians(),
        );
        Isometry3::from_parts(translation, rotation)
    }
}

#[cfg(feature = "nalgebra-support")]
impl From<Isometry3<f64>> for Position {
    fn from(iso: Isometry3<f64>) -> Self {
        let (w, p, r) = iso.rotation.euler_angles();
        Position {
            x: iso.translation.x,
            y: iso.translation.y,
            z: iso.translation.z,
            w: w.to_degrees(),
            p: p.to_degrees(),
            r: r.to_degrees(),
            ext1: 0.0,  // Not handled by Isometry3
            ext2: 0.0,
            ext3: 0.0,
        }
    }
}
```

**Note**: ext1/ext2/ext3 are NOT handled by Isometry3 (only 6-axis: position + orientation)

#### 1.4: Missing RMI Commands

**Files**: `fanuc_rmi/src/commands/` (new files)

Implement (if not already present):
- `FRC_GetUFrameUTool` - Get active frame/tool numbers
- `FRC_SetUFrameUTool` - Set active frame/tool
- `FRC_ReadUFrameData` - Read frame transformation data
- `FRC_ReadUToolData` - Read tool geometry data

See [RMI_COMMANDS_REFERENCE.md](RMI_COMMANDS_REFERENCE.md) for specifications.

#### 1.5: Update web_server and web_app Dependencies

**Files**: `web_server/Cargo.toml`, `web_app/Cargo.toml`

```toml
[dependencies]
fanuc_rmi = { path = "../fanuc_rmi", features = ["DTO", "nalgebra-support"] }
```

### Deliverables

- [ ] nalgebra feature flag implemented
- [ ] Position type with optional ext1/ext2/ext3
- [ ] Isometry3 ↔ Position conversions
- [ ] Missing RMI commands implemented
- [ ] web_server/web_app using nalgebra-support feature

---

## 🔄 Phase 2: Program Execution & Database (2-3 days)

### Goal
Implement CSV program loading, buffered execution, and SQLite database for storage.

### Tasks

#### 2.1: SQLite Database Schema

**File**: `web_server/src/database.rs` (new file)

**Database Location**: `./data/fanuc_rmi.db` (relative to executable)
- Directory created automatically if it doesn't exist
- Added to `.gitignore`

```rust
use rusqlite::{Connection, Result};
use std::path::Path;
use std::fs;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        // Create data directory if it doesn't exist
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent).map_err(|e| {
                rusqlite::Error::InvalidPath(format!("Failed to create directory: {}", e).into())
            })?;
        }

        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS programs (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                default_w REAL DEFAULT 0.0,
                default_p REAL DEFAULT 0.0,
                default_r REAL DEFAULT 0.0,
                default_speed REAL,
                default_term_type TEXT DEFAULT 'CNT',
                default_uframe INTEGER,
                default_utool INTEGER,
                start_x REAL,
                start_y REAL,
                start_z REAL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS program_instructions (
                id INTEGER PRIMARY KEY,
                program_id INTEGER NOT NULL,
                line_number INTEGER NOT NULL,
                x REAL NOT NULL,
                y REAL NOT NULL,
                z REAL NOT NULL,
                w REAL,
                p REAL,
                r REAL,
                ext1 REAL,
                ext2 REAL,
                ext3 REAL,
                speed REAL,
                term_type TEXT,
                uframe INTEGER,
                utool INTEGER,
                FOREIGN KEY (program_id) REFERENCES programs(id)
            );

            CREATE TABLE IF NOT EXISTS robot_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            -- Robot default settings
            INSERT OR IGNORE INTO robot_settings (key, value) VALUES
                ('default_w', '0.0'),
                ('default_p', '0.0'),
                ('default_r', '0.0'),
                ('default_speed', '50.0'),
                ('default_uframe', '0'),
                ('default_utool', '0');"
        )?;
        Ok(Self { conn })
    }

    /// Reset database - IRREVERSIBLE!
    pub fn reset(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "DROP TABLE IF EXISTS programs;
             DROP TABLE IF EXISTS program_instructions;
             DROP TABLE IF EXISTS robot_settings;"
        )?;
        // Recreate tables
        Self::new("./data/fanuc_rmi.db")?;
        Ok(())
    }
}
```

#### 2.2: CSV Program Parser

**File**: `web_server/src/program_parser.rs` (new file)

**Minimal CSV Format**:
```csv
x,y,z,speed
100.0,200.0,300.0,50
150.0,200.0,300.0,100
```

**Full CSV Format** (all columns optional except x,y,z):
```csv
x,y,z,w,p,r,ext1,ext2,ext3,speed,term_type,uframe,utool
100.0,200.0,300.0,0.0,90.0,0.0,0.0,0.0,0.0,50,CNT,3,1
```

**Parser Logic**:
- Required: `x`, `y`, `z`
- Optional: `w`, `p`, `r` (default: program's default rotation, or robot's default if not set)
- Optional: `ext1`, `ext2`, `ext3` (default: 0.0)
- Optional: `speed` (default: program's default speed, or robot's default if not set)
- Optional: `term_type` (default: `CNT` for all except last, `FINE` for last)
- Optional: `uframe`, `utool` (default: program's default, or active frame/tool if not set)

**Start Position (Home)**:
- **Default**: First line of program (X,Y,Z from line 1)
- **Before CSV upload**: Defaults to (0, 0, 0)
- **User can override**: During CSV upload, user can specify custom start position
- Robot moves to start position before executing program

#### 2.3: Program Execution Engine

**File**: `web_server/src/program_executor.rs` (new file)

**Buffered Streaming Strategy**:
1. Send first 5 instructions to robot
2. As each instruction completes, send next one
3. Maintain buffer of 5 instructions until completion

**Termination Handling**:
- All instructions use `CNT` termination (continuous motion)
- **Last instruction uses `FINE`** termination (complete stop)
- This ensures smooth motion without requiring NoBlend flag (RMI v5+)

```rust
async fn execute_program(
    driver: &FanucDriver,
    instructions: Vec<Instruction>,
) -> Result<(), String> {
    let buffer_size = 5;
    let mut sent = 0;

    // Send initial batch
    for i in 0..buffer_size.min(instructions.len()) {
        let mut instr = instructions[i].clone();
        // Set FINE for last instruction
        if i == instructions.len() - 1 {
            instr.term_type = TermType::FINE;
        }
        driver.send_packet(
            SendPacket::Instruction(instr),
            PacketPriority::Standard
        )?;
        sent += 1;
    }

    // Stream remaining
    while sent < instructions.len() {
        driver.wait_on_request_completion(/* ... */).await?;
        let mut instr = instructions[sent].clone();
        if sent == instructions.len() - 1 {
            instr.term_type = TermType::FINE;
        }
        driver.send_packet(
            SendPacket::Instruction(instr),
            PacketPriority::Standard
        )?;
        sent += 1;
    }

    Ok(())
}
```

#### 2.4: Example Program - Spiral Cylinder

**File**: `examples/spiral_cylinder.csv`

```csv
x,y,z
50.0,0.0,0.0
49.24,7.82,4.0
46.98,15.45,8.0
43.30,22.70,12.0
...
(100 points total, spiraling up to Z=200mm)
```

**Generator**: Create Rust utility to generate spiral programs

### Deliverables

- [ ] SQLite database schema
- [ ] CSV parser (minimal and full formats)
- [ ] Program execution engine (buffered streaming)
- [ ] Example spiral cylinder program
- [ ] Database CRUD operations for programs

---

## 🎨 Phase 3: Frontend - Desktop UI (3-4 days)

### Goal
Build desktop-style application layout with Dashboard tabs and always-visible sidebar.

### Tasks

#### 3.1: Routing Setup

**File**: `web_app/Cargo.toml`

```toml
[dependencies]
leptos_router = "0.6"
```

**File**: `web_app/src/lib.rs`

```rust
use leptos::*;
use leptos_router::*;

#[component]
pub fn App() -> impl IntoView {
    let ws_manager = WebSocketManager::new();
    provide_context(ws_manager);

    view! {
        <Router>
            <Routes>
                <Route path="/" view=LegacyApp/>
                <Route path="/v2" view=NewApp/>
            </Routes>
        </Router>
    }
}

#[component]
fn LegacyApp() -> impl IntoView {
    // Existing simple interface (preserved)
    view! {
        <div class="min-h-screen bg-[#0a0a0a]">
            // ... existing layout
            <a href="/v2" class="text-[#00d9ff]">
                "Try the new interface →"
            </a>
        </div>
    }
}

#[component]
fn NewApp() -> impl IntoView {
    view! {
        <div class="h-screen w-screen bg-[#0a0a0a] flex flex-col overflow-hidden">
            <TopBar/>
            <div class="flex-1 flex overflow-hidden">
                <Sidebar/>
                <MainWorkspace/>
                <RightPanel/>
            </div>
        </div>
    }
}
```

#### 3.2: Header Component

**File**: `web_app/src/components/top_bar.rs` (new file)

```rust
#[component]
pub fn TopBar() -> impl IntoView {
    view! {
        <div class="h-14 bg-[#111111] border-b border-[#ffffff10] flex items-center justify-between px-4">
            <div class="flex items-center space-x-4">
                <h1 class="text-lg font-semibold text-white">"FANUC RMI CONTROL"</h1>

                // WebSocket status
                <div class="flex items-center space-x-2">
                    <div class="w-2 h-2 rounded-full bg-[#00ff88]"></div>
                    <span class="text-xs text-[#cccccc]">"Connected"</span>
                </div>

                // Robot status
                <div class="flex items-center space-x-2">
                    <span class="text-xs text-[#888888]">"Robot:"</span>
                    <span class="text-xs text-white">"192.168.1.100"</span>
                    <div class="w-2 h-2 rounded-full bg-[#00ff88]"></div>
                </div>
            </div>

            // Settings button
            <button class="p-2 hover:bg-[#1a1a1a] rounded">
                // ⚙ icon
            </button>
        </div>
    }
}
```

#### 3.3: Left Navbar

**File**: `web_app/src/components/sidebar.rs` (new file)

```rust
#[component]
pub fn Sidebar() -> impl IntoView {
    let (active_route, set_active_route) = create_signal("dash");

    view! {
        <div class="w-20 bg-[#111111] border-r border-[#ffffff10] flex flex-col items-center py-4 space-y-4">
            <NavButton icon="📊" label="DASH" route="dash" active=active_route/>
            <NavButton icon="🎮" label="CTRL" route="ctrl" active=active_route/>
            <NavButton icon="⚙" label="SET" route="set" active=active_route/>
        </div>
    }
}
```

#### 3.4: Right Panel (Always Visible)

**File**: `web_app/src/components/right_panel.rs` (new file)

```rust
#[component]
pub fn RightPanel() -> impl IntoView {
    let (jog_popped, set_jog_popped) = create_signal(false);

    view! {
        <div class="w-80 bg-[#111111] border-l border-[#ffffff10] flex flex-col space-y-4 p-4 overflow-y-auto">
            <CompactPositionDisplay/>
            <ErrorsPanel/>
            <IOStatusPanel/>

            {move || if !jog_popped.get() {
                view! { <JogControls on_pop=move |_| set_jog_popped.set(true)/> }.into_view()
            } else {
                view! { <div class="text-xs text-[#888888] text-center">"Jog controls popped out"</div> }.into_view()
            }}
        </div>
    }
}
```

#### 3.5: Poppable Jog Controls

**File**: `web_app/src/components/jog_controls_popped.rs` (new file)

```rust
#[component]
pub fn JogControlsPopped(
    on_close: impl Fn() + 'static,
    initial_x: f64,
    initial_y: f64,
) -> impl IntoView {
    let (pos_x, set_pos_x) = create_signal(initial_x);
    let (pos_y, set_pos_y) = create_signal(initial_y);

    // Draggable logic, snap to edges

    view! {
        <div
            class="fixed bg-[#111111] border-2 border-[#00d9ff] rounded-lg p-6 shadow-2xl z-50"
            style=move || format!("left: {}px; top: {}px;", pos_x.get(), pos_y.get())
        >
            <div class="flex justify-between items-center mb-4">
                <h3 class="text-sm font-semibold text-[#00d9ff] uppercase">"Jog Controls"</h3>
                <button on:click=move |_| on_close() class="text-[#888888] hover:text-white">
                    "✕"
                </button>
            </div>

            // Larger jog pad
            <JogPad size="large"/>
        </div>
    }
}
```

**Persistence**: Store jog popped state and position in localStorage or context

#### 3.6: Dashboard Tab 1 - Info

**File**: `web_app/src/views/dashboard_info.rs` (new file)

**Layout**: Grid of panels

```rust
#[component]
pub fn DashboardInfo() -> impl IntoView {
    view! {
        <div class="grid grid-cols-2 gap-4 p-4">
            // Top row
            <FrameManagementPanel/>
            <ToolManagementPanel/>

            // Middle row
            <MultiFrameDisplay/>
            <MultiToolDisplay/>

            // Bottom row (full width)
            <div class="col-span-2">
                <JointAnglesPanel/>
            </div>

            // Logs (collapsible accordion)
            <div class="col-span-2">
                <LogsAccordion/>
            </div>
        </div>
    }
}
```

**Frame Management Panel**:
- High-level list of all UFrames (0-9)
- Shows: Number, Name (if configured), Active indicator
- Minimal, compact display

**Multi-Frame Display**:
- Detailed data for each frame
- Active frame expanded by default
- Others collapsed (accordion)
- Shows: X,Y,Z,W,P,R transformation, current position in that frame

**Tool Management Panel**:
- High-level list of all UTools (0-10)
- Shows: Number, Name, Active indicator

**Multi-Tool Display**:
- Detailed tool geometry for each tool
- Active tool expanded by default
- Shows: X,Y,Z,W,P,R offsets, TCP position

#### 3.7: Dashboard Tab 2 - Control

**File**: `web_app/src/views/dashboard_control.rs` (new file)

**Layout**: Vertical sections

```rust
#[component]
pub fn DashboardControl() -> impl IntoView {
    view! {
        <div class="flex flex-col space-y-4 p-4">
            // Quick actions
            <QuickActionsBar/>

            // Command section
            <CommandSection/>

            // Command log
            <CommandLogPanel/>

            // Program loader
            <ProgramLoaderPanel/>

            // Program visual display
            <ProgramVisualDisplay/>
        </div>
    }
}
```

**Quick Actions Bar**:
```rust
<div class="flex space-x-2">
    <button>"Connect/Disconnect"</button>
    <button>"Abort"</button>
    <button>"Initialize"</button>
    <button>"Reset"</button>
    <button>"..."</button>
</div>
```

**Command Section**:
```rust
<div class="flex items-center space-x-2">
    <select class="flex-1">"Recent commands dropdown"</select>
    <button>"Composer"</button>
    <button>"Send"</button>
</div>
```

**Command Composer Modal**:
- Step 1: Select command/instruction type
- Step 2: Fill in fields
  - Configuration: Dropdowns (UFrame, UTool, Front/Up/Left, etc.)
  - Position: Cartesian inputs (X,Y,Z,W,P,R)
    - Smart behavior: Unspecified axes use current position (absolute) or 0 (relative)
    - [Use Current Position] button to pre-fill
- Step 3: Preview command
- [Send] or [Save to Recent]

**Program Visual Display**:
- G-code style line-by-line view
- Columns: Line #, X, Y, Z, W, P, R, Speed, Term
- Highlights currently executing line
- Scrollable

### Deliverables

- [ ] Routing setup (legacy `/` vs new `/v2`)
- [ ] Desktop layout (header, sidebar, workspace, right panel)
- [ ] Poppable jog controls with persistence
- [ ] Dashboard Tab 1 (Info) with frame/tool panels
- [ ] Dashboard Tab 2 (Control) with command composer
- [ ] Program visual display

---

## ⚙ Phase 4: Settings & Program Management (2-3 days)

### Goal
Implement settings view, program management, and CSV upload.

### Tasks

#### 4.1: Settings View

**File**: `web_app/src/views/settings.rs` (new file)

**Layout**: Two-column

```rust
#[component]
pub fn SettingsView() -> impl IntoView {
    view! {
        <div class="grid grid-cols-2 gap-4 p-4">
            // Left column
            <div class="space-y-4">
                <ConnectionSettingsPanel/>
                <RobotInfoPanel/>
            </div>

            // Right column
            <div class="space-y-4">
                <ProgramManagementPanel/>
                <DisplayPreferencesPanel/>
            </div>
        </div>
    }
}
```

**Program Management Panel**:
- List of saved programs
- [Upload CSV] button
- [Delete] [Edit] buttons
- Program metadata: Name, Description, Line count, Created date

**CSV Upload Flow**:
1. User clicks [Upload CSV]
2. File picker opens
3. Parse CSV (validate format)
4. Show preview modal:
   - Program name input
   - Description input
   - Default settings (speed, term_type, uframe, utool, rotation)
   - Start position (home) input
   - Preview first 10 lines
5. [Save] to database

#### 4.2: Program Metadata Display

When displaying a program, show:
- **Program Name**
- **Description**
- **Default Rotation** (W, P, R) - used when not specified in CSV
- **Default Speed** - used when not specified
- **Default Term Type** - CNT (except last = FINE)
- **Default UFrame/UTool** - active frame/tool
- **Start Position** (Home) - X, Y, Z to move to before starting
- **Line Count**
- **Created/Updated dates**

#### 4.3: WebSocket Message Updates

**File**: `web_server/src/websocket_messages.rs`

Add new message types:
- `LoadProgram { program_id: u64 }`
- `RunProgram`
- `PauseProgram`
- `StopProgram`
- `ClearErrors`
- `UnloadProgram`
- `ProgramStatus { name: String, current_line: usize, total_lines: usize, status: String }`

### Deliverables

- [ ] Settings view with program management
- [ ] CSV upload with preview and metadata
- [ ] Program CRUD operations (UI)
- [ ] WebSocket messages for program control
- [ ] Program metadata display

---

## 🎁 Phase 5: Polish & Deployment (2-3 days)

### Goal
Final polish, testing, documentation, and deployment.

### Tasks

#### 5.1: Responsive Design

**Breakpoints**:
- Desktop (>1280px): Full 3-column layout
- Tablet (768-1280px): Collapsed sidebar (icons only), narrower right panel
- Mobile (<768px): Bottom nav bar, jog controls in modal

#### 5.2: Tooltips

Add tooltips to ALL controls using Leptos tooltip component or custom implementation.

#### 5.3: Testing

- [ ] Test all RMI commands
- [ ] Test program execution (spiral cylinder)
- [ ] Test CSV upload (minimal and full formats)
- [ ] Test jog controls (embedded and popped)
- [ ] Test responsive design (desktop/tablet/mobile)
- [ ] Test database operations

#### 5.4: Documentation

- [ ] Update README with new features
- [ ] Add screenshots/GIFs of new UI
- [ ] Document CSV format
- [ ] Document program execution behavior
- [ ] Document database schema

#### 5.5: Deployment

- [ ] Build optimized WASM bundle
- [ ] Configure web_server for production
- [ ] Set up SQLite database path
- [ ] Environment variables for configuration

### Deliverables

- [ ] Responsive design implemented
- [ ] Tooltips on all controls
- [ ] Comprehensive testing complete
- [ ] Documentation updated
- [ ] Production deployment ready

---

## 📊 Success Criteria

- ✅ Legacy app preserved at `/`
- ✅ New desktop-style app at `/v2`
- ✅ Full-window layout (no scrolling except logs)
- ✅ Always-visible essentials (jog, position, status)
- ✅ Poppable jog controls with persistence
- ✅ Frame/tool awareness and management
- ✅ Program execution with buffered streaming
- ✅ CSV upload with flexible format
- ✅ SQLite database for programs/settings
- ✅ Responsive design (desktop/tablet/mobile)
- ✅ Tooltips on all controls
- ✅ Dark futuristic aesthetic preserved

---

## 📝 Notes

### CNT Termination & Last Instruction

**Problem**: CNT instructions don't execute until next instruction arrives.

**Solution**: Use `FINE` termination for the last instruction in a program.

**Future Enhancement**: Add `NoBlend` flag support (RMI v5+) to allow CNT instructions to execute without waiting.

### nalgebra Integration

- Only handles 6-axis (X,Y,Z,W,P,R)
- ext1/ext2/ext3 are NOT handled by Isometry3
- Conversions are feature-gated behind `nalgebra-support`

### Database

- SQLite3 for simplicity (single-file database)
- Stores programs, instructions, and settings
- Can migrate to PostgreSQL later if needed

---

**See Also**:
- [UI_DESIGN_MOCKUP.md](UI_DESIGN_MOCKUP.md) - Visual mockups and component patterns
- [FANUC_ROBOTICS_FUNDAMENTALS.md](FANUC_ROBOTICS_FUNDAMENTALS.md) - Robotics concepts
- [RMI_COMMANDS_REFERENCE.md](RMI_COMMANDS_REFERENCE.md) - Complete RMI command reference


