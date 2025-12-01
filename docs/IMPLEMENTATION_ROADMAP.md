# FANUC RMI Web Interface - Implementation Roadmap

**Comprehensive Plan for Building a Full-Featured Robot Control Interface**

---

## Executive Summary

This document outlines the phased implementation plan for transforming the FANUC RMI web interface from a basic position display tool into a comprehensive, professional-grade robot control and monitoring system.

### Vision

Create a web-based interface that provides:
- **Complete visibility** into robot state, position, and configuration
- **Multi-frame coordinate display** with transformation capabilities
- **Intuitive controls** for frame/tool management
- **Professional logging and diagnostics**
- **Extensible architecture** for future features

### Current State

**What Works**:
- Basic WebSocket connection to robot
- Position polling and display (single frame)
- Simple jog controls
- Basic status display

**What's Missing**:
- Frame/tool awareness and management
- Multi-frame coordinate display
- Comprehensive status monitoring
- Error handling and diagnostics
- Settings management
- Professional UI/UX

---

## Phase 1: Foundation - Frame and Tool Awareness

**Goal**: Display which coordinate frame and tool are active, show accurate position data

**Duration**: 1-2 days

### Tasks

#### 1.1: Backend - Add Missing RMI Commands

**Files to modify**:
- `fanuc_rmi/src/commands/` (new files)
- `fanuc_rmi/src/lib.rs`
- `fanuc_rmi/src/drivers/driver.rs`

**New commands to implement**:
- [ ] `FRC_GetUFrameUTool` - Get active frame/tool numbers
- [ ] `FRC_SetUFrameUTool` - Set active frame/tool numbers
- [ ] `FRC_ReadUFrameData` - Read frame transformation data
- [ ] `FRC_ReadUToolData` - Read tool transformation data

**Implementation**:
```rust
// fanuc_rmi/src/commands/frc_getuframeutool.rs
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrcGetUFrameUToolRequest {
    #[serde(rename = "Command")]
    pub command: String,  // "FRC_GetUFrameUTool"
    #[serde(rename = "Group", skip_serializing_if = "Option::is_none")]
    pub group: Option<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FrcGetUFrameUToolResponse {
    #[serde(rename = "ErrorID")]
    pub error_id: u32,
    #[serde(rename = "UFrameNumber")]
    pub u_frame_number: u8,
    #[serde(rename = "UToolNumber")]
    pub u_tool_number: u8,
    #[serde(rename = "Group")]
    pub group: u8,
}
```

**Driver methods**:
```rust
impl FanucDriver {
    pub async fn get_uframe_utool(&self) -> Result<FrcGetUFrameUToolResponse, String> { ... }
    pub async fn set_uframe_utool(&self, uframe: u8, utool: u8) -> Result<FrcSetUFrameUToolResponse, String> { ... }
    pub async fn read_uframe_data(&self, frame_num: u8) -> Result<FrcReadUFrameDataResponse, String> { ... }
    pub async fn read_utool_data(&self, tool_num: u8) -> Result<FrcReadUToolDataResponse, String> { ... }
}
```

#### 1.2: Backend - Frame Manager

**Files to create**:
- `fanuc_rmi/src/frame_manager.rs`

**Purpose**: Cache frame/tool transformations and provide coordinate conversion

**Dependencies**: Add `nalgebra = "0.32"` to Cargo.toml

**Key features**:
- Load all UFrame/UTool data at startup
- Cache transformations
- Provide conversion functions
- Refresh on demand

#### 1.3: Web Server - Integrate Frame Data

**Files to modify**:
- `web_server/src/main.rs`

**Changes**:
- Initialize FrameManager
- Load frame data after robot connection
- Include frame/tool info in position updates
- Add WebSocket messages for frame operations

**New WebSocket messages**:
```json
// Outbound (server -> client)
{
    "type": "position_update",
    "active_uframe": 3,
    "active_utool": 1,
    "position": {
        "x": 6.022,
        "y": -35.690,
        "z": 26.048,
        ...
    },
    "all_frames": {
        "0": {"x": 1006.022, "y": -35.390, "z": 126.048},
        "1": {...},
        "3": {"x": 6.022, "y": -35.690, "z": 26.048}  // Active frame
    }
}

// Inbound (client -> server)
{
    "type": "set_active_frame",
    "uframe": 0,
    "utool": 1
}
```

#### 1.4: Frontend - Display Frame Information

**Technology**: Leptos (Rust WASM framework)

**Files to modify**:
- `web_app/src/components/position_display.rs` - Update to show frame info
- `web_app/src/robot_models.rs` - Add frame/tool fields to model
- `web_app/src/websocket.rs` - Handle new frame data from server

**UI additions**:
- Display active UFrame/UTool numbers
- Show position with frame label
- Add frame selector dropdown
- Visual indicator for active frame

**Leptos Component Example**:
```rust
#[component]
pub fn PositionDisplay() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");
    let position = ws.position;
    let frame_info = ws.frame_info; // NEW

    view! {
        <div class="bg-[#111111] rounded border border-[#ffffff10] p-4">
            <h2 class="text-sm font-semibold text-[#00d9ff] mb-3 flex items-center uppercase tracking-wide">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"/>
                </svg>
                "Position"
            </h2>

            // NEW: Frame info display
            {move || {
                if let Some(info) = frame_info.get() {
                    view! {
                        <div class="mb-3 bg-[#1a1a1a] rounded border border-[#ffffff08] p-2.5">
                            <div class="flex items-center justify-between">
                                <span class="text-[#888888] text-xs">"Active Frame:"</span>
                                <span class="text-[#00d9ff] text-sm font-mono">
                                    {format!("UFrame {}, UTool {}", info.uframe, info.utool)}
                                </span>
                            </div>
                        </div>
                    }.into_view()
                } else {
                    view! { <div></div> }.into_view()
                }
            }}

            // Existing position display with frame label
            {move || {
                if let Some(pos) = position.get() {
                    view! {
                        <div class="space-y-2">
                            <div class="flex justify-between items-center bg-[#1a1a1a] rounded border border-[#ffffff08] p-2.5">
                                <span class="text-[#888888] text-sm font-medium">"X:"</span>
                                <span class="text-base font-mono text-white">{format!("{:.3}", pos.x)} " mm"</span>
                            </div>
                            // ... Y, Z, W, P, R
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <div class="text-center text-[#666666] py-6 text-sm">
                            "Waiting for position data..."
                        </div>
                    }.into_view()
                }
            }}
        </div>
    }
}
```

**Styling Notes**:
- Maintain existing dark theme: `bg-[#0a0a0a]` background, `bg-[#111111]` panels
- Use cyan accent color: `#00d9ff` for highlights and active states
- Keep subtle borders: `border-[#ffffff10]` and `border-[#ffffff08]`
- Preserve Inter font family
- Match existing component structure and spacing

### Deliverables

- [ ] All frame/tool commands implemented and tested
- [ ] FrameManager working with coordinate transformations
- [ ] Web server sending multi-frame data
- [ ] UI displaying active frame and position
- [ ] Documentation updated

### Testing

- [ ] Unit tests for coordinate transformations
- [ ] Integration test with simulator
- [ ] Validation against real robot
- [ ] Verify positions match teach pendant when in same frame

---

## Phase 2: Multi-Frame Display and Frame Management

**Goal**: Allow users to view positions in multiple frames and change active frame

**Duration**: 2-3 days

### Tasks

#### 2.1: Frontend - Multi-Frame Position Display

**UI Component**: Expandable frame viewer

**Mockup**:
```
┌─────────────────────────────────────┐
│ ▼ Position in All Frames            │
├─────────────────────────────────────┤
│ ● UFrame 0 (World)                  │
│   X: 1006.022  Y: -35.390  Z: 126.048│
│                                     │
│ ○ UFrame 1                          │
│   X:  506.022  Y:  64.610  Z:  26.048│
│                                     │
│ ● UFrame 3 (ACTIVE)                 │
│   X:    6.022  Y: -35.690  Z:  26.048│
│                                     │
│ ○ UFrame 5                          │
│   X:  256.022  Y:  14.310  Z:  76.048│
└─────────────────────────────────────┘
```

**Features**:
- Collapsible section
- Highlight active frame
- Click to set as active (with confirmation)
- Real-time updates for all frames

#### 2.2: Frontend - Frame/Tool Management Panel

**New UI Section**: Settings/Configuration tab

**Mockup**:
```
┌─────────────────────────────────────┐
│ Frame & Tool Configuration          │
├─────────────────────────────────────┤
│ Active Coordinate Frame:            │
│   UFrame: [3 ▼]  UTool: [1 ▼]       │
│   [Apply Changes]                   │
│                                     │
│ ⚠ Warning: Changing frames affects  │
│   motion commands. Only change when │
│   robot is stopped.                 │
│                                     │
│ Frame Details:                      │
│ ┌─────────────────────────────────┐ │
│ │ UFrame 3: Fixture A             │ │
│ │ Origin: X=1000, Y=300, Z=100    │ │
│ │ Rotation: W=0, P=0, R=90        │ │
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

**Features**:
- Dropdowns populated from robot status
- Safety check before applying
- Display frame transformation data
- Visual feedback on success/error

#### 2.3: Backend - Safety Checks

**Implementation**: Verify robot is stopped before allowing frame changes

```rust
pub async fn safe_set_uframe_utool(&self, uframe: u8, utool: u8) -> Result<(), String> {
    // Check robot is stopped
    let status = self.get_status().await?;
    if status.rmi_motion_status != 0 {
        return Err("Cannot change frame while robot is moving".to_string());
    }
    
    // Apply change
    let response = self.set_uframe_utool(uframe, utool).await?;
    if response.error_id != 0 {
        return Err(format!("Failed to set frame: error {}", response.error_id));
    }
    
    Ok(())
}
```

### Deliverables

- [ ] Multi-frame position display working
- [ ] Frame/tool selector with safety checks
- [ ] Frame transformation data displayed
- [ ] Error handling for invalid operations

### Testing

- [ ] Verify multi-frame positions are mathematically correct
- [ ] Test frame switching when robot stopped
- [ ] Verify error when trying to change frame during motion
- [ ] Compare with teach pendant in different frames

---

## Phase 3: Professional UI/UX - Navigation and Layout

**Goal**: Create a professional, organized interface with proper navigation

**Duration**: 3-4 days

### Tasks

#### 3.1: UI Architecture - Application Layout (Desktop-Style)

**Technology**: Leptos with reactive signals, full-window layout, responsive design

**Design Philosophy**:
- ✅ **App-like, not webpage-like** - No scrolling, full-window layout
- ✅ **Always-visible essentials** - Jog controls and position always on screen
- ✅ **Contextual panels** - Tabs/accordions for secondary features
- ✅ **Dense but clean** - Efficient use of space, modern aesthetic
- ✅ **Tooltips for descriptions** - Keep UI clean, show help on hover
- ✅ **Responsive** - Collapsible sidebars, adaptive layouts

**New Structure** (Desktop/Tablet):
```
┌──────────────────────────────────────────────────────────────────┐
│ FANUC RMI Control  [Robot: 192.168.1.100]  [●] Connected  [⚙]  │
├────────┬─────────────────────────────────────────────┬───────────┤
│        │                                             │           │
│  Nav   │         MAIN WORKSPACE                      │  ALWAYS   │
│  Bar   │                                             │  VISIBLE  │
│        │  ┌────────────────────────────────────┐     │           │
│ [📊]   │  │ [Position] [I/O] [Logs] [Advanced] │     │  ┌─────┐  │
│ Dash   │  ├────────────────────────────────────┤     │  │ Pos │  │
│        │  │                                    │     │  │ X:  │  │
│ [🎯]   │  │  Tab Content Area                  │     │  │ Y:  │  │
│ Pos    │  │  (Position details, I/O grid,      │     │  │ Z:  │  │
│        │  │   logs, advanced features)         │     │  └─────┘  │
│ [🎮]   │  │                                    │     │           │
│ Ctrl   │  │                                    │     │  ┌─────┐  │
│        │  │                                    │     │  │ Jog │  │
│ [⚙]   │  │                                    │     │  │ Pad │  │
│ Set    │  │                                    │     │  │ ↑   │  │
│        │  └────────────────────────────────────┘     │  │← ↓→ │  │
│        │                                             │  └─────┘  │
│        │                                             │           │
└────────┴─────────────────────────────────────────────┴───────────┘
```

**Layout Zones**:

1. **Left Sidebar (Collapsible)** - Main navigation
   - Dashboard (overview)
   - Position (detailed position/frame info)
   - Control (motion commands, programs)
   - Settings (connection, config)
   - Width: 60px (icons only) or 200px (expanded)
   - Collapses to icon-only on smaller screens

2. **Center Workspace** - Contextual content based on nav selection
   - Tabs for sub-features within each section
   - Full height, no scrolling
   - Responsive grid layouts

3. **Right Panel (Always Visible)** - Critical controls
   - **Current Position** (compact, always visible)
   - **Jog Control Pad** (always accessible)
   - **Robot Status** (servo, mode, motion)
   - Width: 280px
   - Stacks vertically on mobile

**Responsive Breakpoints**:
- **Desktop (>1280px)**: Full 3-column layout
- **Tablet (768-1280px)**: Collapsed sidebar (icons only), narrower right panel
- **Mobile (<768px)**: Bottom nav bar, jog controls in modal/drawer

**Leptos Implementation**:
```rust
// web_app/src/components/app_layout.rs
#[derive(Clone, Copy, PartialEq)]
enum NavSection {
    Dashboard,
    Position,
    Control,
    Settings,
}

#[component]
pub fn AppLayout() -> impl IntoView {
    let (active_section, set_active_section) = create_signal(NavSection::Dashboard);
    let (sidebar_expanded, set_sidebar_expanded) = create_signal(true);

    view! {
        <div class="h-screen w-screen bg-[#0a0a0a] flex flex-col overflow-hidden">
            // Top bar
            <TopBar/>

            // Main layout
            <div class="flex-1 flex overflow-hidden">
                // Left sidebar (collapsible)
                <Sidebar
                    active_section=active_section
                    set_active_section=set_active_section
                    expanded=sidebar_expanded
                    set_expanded=set_sidebar_expanded
                />

                // Center workspace
                <div class="flex-1 overflow-y-auto">
                    {move || match active_section.get() {
                        NavSection::Dashboard => view! { <DashboardView/> }.into_view(),
                        NavSection::Position => view! { <PositionView/> }.into_view(),
                        NavSection::Control => view! { <ControlView/> }.into_view(),
                        NavSection::Settings => view! { <SettingsView/> }.into_view(),
                    }}
                </div>

                // Right panel (always visible)
                <RightPanel/>
            </div>
        </div>
    }
}

#[component]
fn TopBar() -> impl IntoView {
    let ws = use_context::<WebSocketManager>().expect("WebSocketManager not found");

    view! {
        <div class="h-14 bg-[#111111] border-b border-[#ffffff10] flex items-center justify-between px-4">
            <div class="flex items-center space-x-3">
                <h1 class="text-lg font-semibold text-white">"FANUC RMI Control"</h1>
                <div class="text-xs text-[#888888] bg-[#1a1a1a] px-2 py-1 rounded">
                    "Robot: 192.168.1.100"
                </div>
            </div>

            <div class="flex items-center space-x-4">
                // Connection status
                <div class="flex items-center space-x-2">
                    <div class=move || if ws.connected.get() {
                        "w-2 h-2 rounded-full bg-[#00d9ff] animate-pulse"
                    } else {
                        "w-2 h-2 rounded-full bg-[#666666]"
                    }></div>
                    <span class="text-xs text-[#888888]">
                        {move || if ws.connected.get() { "Connected" } else { "Disconnected" }}
                    </span>
                </div>

                // Settings icon
                <button class="text-[#888888] hover:text-white transition-colors">
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
                    </svg>
                </button>
            </div>
        </div>
    }
}

#[component]
fn Sidebar(
    active_section: ReadSignal<NavSection>,
    set_active_section: WriteSignal<NavSection>,
    expanded: ReadSignal<bool>,
    set_expanded: WriteSignal<bool>,
) -> impl IntoView {
    view! {
        <div class=move || if expanded.get() {
            "w-52 bg-[#111111] border-r border-[#ffffff10] flex flex-col transition-all duration-200"
        } else {
            "w-16 bg-[#111111] border-r border-[#ffffff10] flex flex-col transition-all duration-200"
        }>
            // Toggle button
            <button
                class="h-12 flex items-center justify-center text-[#888888] hover:text-white border-b border-[#ffffff10]"
                on:click=move |_| set_expanded.update(|e| *e = !*e)
            >
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"/>
                </svg>
            </button>

            // Nav items
            <NavItem
                icon="📊"
                label="Dashboard"
                active=move || active_section.get() == NavSection::Dashboard
                on_click=move |_| set_active_section.set(NavSection::Dashboard)
                expanded=expanded
            />
            <NavItem
                icon="🎯"
                label="Position"
                active=move || active_section.get() == NavSection::Position
                on_click=move |_| set_active_section.set(NavSection::Position)
                expanded=expanded
            />
            <NavItem
                icon="🎮"
                label="Control"
                active=move || active_section.get() == NavSection::Control
                on_click=move |_| set_active_section.set(NavSection::Control)
                expanded=expanded
            />
            <NavItem
                icon="⚙"
                label="Settings"
                active=move || active_section.get() == NavSection::Settings
                on_click=move |_| set_active_section.set(NavSection::Settings)
                expanded=expanded
            />
        </div>
    }
}

#[component]
fn NavItem(
    icon: &'static str,
    label: &'static str,
    active: impl Fn() -> bool + 'static,
    on_click: impl Fn(ev::MouseEvent) + 'static,
    expanded: ReadSignal<bool>,
) -> impl IntoView {
    view! {
        <button
            class=move || if active() {
                "h-12 flex items-center px-4 bg-[#00d9ff] text-black font-medium transition-colors"
            } else {
                "h-12 flex items-center px-4 text-[#888888] hover:bg-[#1a1a1a] hover:text-white transition-colors"
            }
            on:click=on_click
            title=label
        >
            <span class="text-lg">{icon}</span>
            {move || if expanded.get() {
                view! { <span class="ml-3 text-sm">{label}</span> }.into_view()
            } else {
                view! { <span></span> }.into_view()
            }}
        </button>
    }
}

#[component]
fn RightPanel() -> impl IntoView {
    view! {
        <div class="w-72 bg-[#111111] border-l border-[#ffffff10] flex flex-col overflow-y-auto">
            // Always visible: Position
            <div class="p-3 border-b border-[#ffffff10]">
                <CompactPositionDisplay/>
            </div>

            // Always visible: Jog Controls
            <div class="p-3 border-b border-[#ffffff10]">
                <JogControls/>
            </div>

            // Always visible: Robot Status
            <div class="p-3">
                <CompactRobotStatus/>
            </div>
        </div>
    }
}
```

#### 3.2: Dashboard View

**Purpose**: At-a-glance overview, all info visible without scrolling

**Layout**: Responsive grid (2-3 columns on desktop, stacks on mobile)

**Content Cards**:

1. **Robot Status Card** (always visible)
   - Connection indicator (pulsing dot)
   - Servo Ready status
   - TP Mode (AUTO/MANUAL/etc)
   - Motion Status (IDLE/MOVING/PAUSED)
   - Override percentage with visual bar
   - Tooltips: Hover for detailed status descriptions

2. **Active Frame/Tool Card**
   - Current UFrame number with name (if available)
   - Current UTool number with name (if available)
   - Quick dropdown to change frame (with safety warning)
   - Tooltip: Explains what frames/tools are

3. **Position Overview Card**
   - Current X, Y, Z in active frame
   - Compact display (3 rows)
   - Click to navigate to detailed Position view
   - Tooltip: Shows full 6-axis position

4. **Recent Activity Card**
   - Last 5 commands sent (scrollable mini-list)
   - Timestamp, command type, status (✓ or ✗)
   - Click to see full logs
   - Auto-scrolls to latest

5. **Error Summary Card** (conditional - only if errors exist)
   - Active errors/warnings count
   - Most recent error message
   - Click to see full error log
   - Red border if critical errors

6. **Quick Actions Card**
   - [Initialize] [Abort] [Continue] buttons
   - Tooltips: Explain what each action does
   - Disabled states when not applicable
   - Confirmation modals for destructive actions

**Grid Layout**:
```
┌──────────────┬──────────────┬──────────────┐
│ Robot Status │ Frame/Tool   │ Position     │
├──────────────┼──────────────┼──────────────┤
│ Recent       │ Errors       │ Quick        │
│ Activity     │ (if any)     │ Actions      │
└──────────────┴──────────────┴──────────────┘
```

**No scrolling** - All cards fit in viewport, dense but clean

#### 3.3: Position View

**Purpose**: Detailed position/frame information, coordinate transformations

**Layout**: Two-column layout (left: current position, right: frame data)

**Left Column - Current Position**:

1. **Cartesian Position Panel**
   - Full 6-axis display (X, Y, Z, W, P, R)
   - Active frame indicator at top
   - Precision: 3 decimal places
   - Copy button to copy values
   - Tooltip: Explains each axis

2. **Configuration Panel**
   - Front/Back, Up/Down, Left/Right
   - Flip, Turn4, Turn5, Turn6
   - Visual indicators (icons or colors)
   - Tooltip: Explains configuration parameters

3. **Joint Angles Panel** (accordion - collapsed by default)
   - All 6 joint angles
   - Compact 2-column grid
   - Tooltip: Joint limits and current percentage

**Right Column - Frame Management**:

1. **Active Frame Selector**
   - Dropdown: UFrame 0-9
   - Shows current active frame
   - Change button (with safety warning modal)
   - Tooltip: "Changing frame affects motion commands"

2. **Frame Data Viewer** (accordion)
   - Expandable list of all UFrames (0-9)
   - Each shows: X, Y, Z, W, P, R transformation
   - Click to expand/collapse
   - Tooltip: Shows what each frame represents

3. **Multi-Frame Display** (accordion)
   - Toggle: "Show position in all frames"
   - When enabled, shows current position transformed to each frame
   - Compact table view
   - Tooltip: "Mathematically transformed coordinates"

**No scrolling needed** - Accordions keep it compact

#### 3.4: Control View

**Purpose**: Advanced motion control, program execution, teaching

**Note**: Basic jog controls are ALWAYS visible in right panel, not here

**Layout**: Full-width workspace with sections

**Content**:

1. **Motion Command Builder** (top section)
   - Dropdown: Command type (Linear, Joint, Circular, Arc, etc.)
   - Input fields for target position (X, Y, Z, W, P, R)
   - Speed/acceleration settings with units
   - Term type selector (FINE, CNT with value)
   - Configuration selector (Front/Back, Up/Down, etc.)
   - [Send Command] button with confirmation
   - Preview panel: Shows command JSON/structure
   - Tooltip: Explains each parameter in detail

2. **Program Execution Panel** (middle section)
   - Program selector dropdown (if programs are stored)
   - [Execute] [Pause] [Resume] [Stop] buttons
   - Progress indicator (visual bar)
   - Current line/instruction display
   - Estimated time remaining
   - Tooltip: Program execution status and controls

3. **Teaching Panel** (accordion - collapsed by default)
   - [Teach Current Position] button
   - Taught points list (scrollable, max 100 points)
   - Each point shows: Name, X, Y, Z, timestamp
   - Edit/delete/reorder taught points
   - Export taught points as program file
   - Tooltip: "Capture current position for later use"

4. **Motion Queue Viewer** (bottom section)
   - Shows pending motion commands (last 10)
   - Columns: Seq ID, Type, Status, Timestamp
   - Status indicators: Pending, Executing, Complete, Error
   - [Clear Queue] button (with confirmation)
   - Auto-scrolls to latest
   - Tooltip: "Commands waiting to execute on robot"

#### 3.5: Settings Tab

**Content**: Robot connection and configuration

```
┌─────────────────────────────────────┐
│ Robot Connection                    │
├─────────────────────────────────────┤
│ Robot Address:                      │
│   [127.0.0.1    ] : [16001]         │
│                                     │
│ WebSocket Port:                     │
│   [9000]                            │
│                                     │
│ [Connect] [Disconnect]              │
│                                     │
│ Status: ● Connected                 │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Frame & Tool Configuration          │
├─────────────────────────────────────┤
│ Active Frame: [3 ▼]                 │
│ Active Tool:  [1 ▼]                 │
│                                     │
│ [Apply Changes]                     │
│                                     │
│ ⚠ Only change when robot stopped    │
└─────────────────────────────────────┘

│ ☑ Show all frames in position view  │
│ ☑ Auto-refresh position (125Hz)     │
│ ☐ Show joint angles                 │
│ ☑ Enable debug logging              │
└─────────────────────────────────────┘
```

**Replaced with**:

**Left Column - Connection & Robot**:

1. **Robot Connection Panel**
   - Robot address input, WebSocket port
   - Connect/Disconnect buttons, status indicator
   - Auto-reconnect toggle, connection history
   - Tooltip: Network settings

2. **Frame & Tool Configuration** (accordion)
   - Active Frame/Tool selectors
   - Apply button with safety warning
   - Tooltip: Affects motion commands

3. **Robot Information** (accordion, read-only)
   - Model, firmware, RMI version, groups

**Right Column - Display & Preferences**:

1. **Display Settings Panel**
   - Precision, units, formats
   - Tooltip: Customize display

2. **UI Preferences** (accordion)
   - Theme, sidebar, tooltips, confirmations

3. **Advanced Settings** (accordion)
   - Timeouts, retry, update rate, debug mode
   - Reset to defaults button

**No scrolling needed** - Accordions keep it compact

---

#### 3.6: I/O View (NEW)

**Purpose**: Monitor and control digital I/O

**Layout**: Grid layout for I/O banks

**Content**:

1. **Digital Inputs Panel** (left side)
   - Grid of input indicators (DI 1-64)
   - Visual indicators: Green (ON), Gray (OFF)
   - Labels for each input (if configured)
   - Real-time updates
   - Tooltip: Shows input number and state

2. **Digital Outputs Panel** (right side)
   - Grid of output controls (DO 1-64)
   - Toggle switches for each output
   - Visual indicators: Cyan (ON), Gray (OFF)
   - Labels for each output (if configured)
   - [Set] [Clear] [Pulse] buttons per output
   - Confirmation for critical outputs
   - Tooltip: Shows output number and state

3. **Group I/O Panel** (accordion - if supported)
   - Batch operations
   - Set multiple outputs at once
   - Read input patterns
   - Tooltip: Advanced I/O operations

**Dense grid layout** - All I/O visible without scrolling (on desktop)

---

#### 3.7: Logs View (Separate from Dashboard)

**Purpose**: Detailed logging and debugging

**Layout**: Full-width log viewer with filters

**Content**:

1. **Log Viewer** (main area)
   - Virtual scrolling for performance (1000s of logs)
   - Columns: Timestamp, Level, Source, Message
   - Color-coded by level (ERROR=red, WARN=yellow, INFO=white, DEBUG=gray)
   - Auto-scroll toggle (follows latest)
   - Search/filter box
   - Tooltip: Click to see full message details

2. **Filter Panel** (top bar)
   - Level filter: [All] [ERROR] [WARN] [INFO] [DEBUG]
   - Source filter: [All] [WebSocket] [Commands] [Motion] [I/O]
   - Time range: [Last 1h] [Last 24h] [All]
   - [Clear Logs] button
   - [Export Logs] button (download as .txt)

3. **Log Details Modal** (click on log entry)
   - Full message with stack trace (if error)
   - Related logs (same source/time)
   - Copy button
   - Tooltip: Detailed log information

**Scrollable** - This is the ONE view where scrolling is expected

### Deliverables

- [ ] Professional desktop-style application UI
- [ ] Full-window layout (no scrolling except logs view)
- [ ] Always-visible essentials (jog controls, position, status in right panel)
- [ ] Collapsible sidebar navigation (4 main sections)
- [ ] Responsive design (desktop/tablet/mobile breakpoints)
- [ ] Tooltips for all controls (clean, uncluttered UI)
- [ ] Dense but modern aesthetic (efficient space usage)
- [ ] Accordions for secondary features (keep views compact)
- [ ] Legacy route preserved at `/` or `/legacy`
- [ ] Smooth transitions and animations

### Testing

- [ ] Test all tabs load correctly
- [ ] Verify navigation works
- [ ] Test on different screen sizes
- [ ] Verify all controls functional

---

## Phase 4: Advanced Features

**Goal**: Add professional features for production use

**Duration**: 3-5 days

### Tasks

#### 4.1: Logging System

**Backend**:
- Structured logging with levels
- Log rotation
- Export logs to file

**Frontend**:
- Real-time log streaming via WebSocket
- Filtering by level
- Search functionality
- Export to file

#### 4.2: Error Handling and Recovery

**Features**:
- Graceful error handling
- User-friendly error messages
- Automatic reconnection
- Error recovery suggestions

**Example**:
```
┌─────────────────────────────────────┐
│ ⚠ Error: Motion Failed              │
├─────────────────────────────────────┤
│ Error Code: 7015                    │
│ Message: RMI_MOVE program selected  │
│                                     │
│ Suggested Action:                   │
│ 1. Press SELECT on teach pendant    │
│ 2. Choose different program         │
│ 3. Press ENTER                      │
│ 4. Click [Retry] below              │
│                                     │
│ [Retry] [Dismiss]                   │
└─────────────────────────────────────┘
```

#### 4.3: I/O Monitoring and Control

**Features**:
- Real-time digital I/O display
- Click to toggle outputs
- Input state monitoring
- I/O configuration

**UI**:
```
┌─────────────────────────────────────┐
│ Digital Inputs                      │
├─────────────────────────────────────┤
│ DI[1]: ● ON   DI[2]: ○ OFF          │
│ DI[3]: ○ OFF  DI[4]: ● ON           │
│ DI[5]: ○ OFF  DI[6]: ○ OFF          │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Digital Outputs                     │
├─────────────────────────────────────┤
│ DO[1]: [●] ON   DO[2]: [○] OFF      │
│ DO[3]: [○] OFF  DO[4]: [●] ON       │
│ DO[5]: [○] OFF  DO[6]: [○] OFF      │
│                                     │
│ Click to toggle                     │
└─────────────────────────────────────┘
```

#### 4.4: Motion Recording and Playback

**Features**:
- Record waypoints
- Save to file
- Load and replay
- Edit waypoint list

**Use Case**: Teach robot path by jogging, record positions, replay

#### 4.5: Multi-Robot Support

**Features**:
- Connect to multiple robots
- Switch between robots
- Synchronized display
- Independent control

**UI**:
```
┌─────────────────────────────────────┐
│ Robot Selection                     │
├─────────────────────────────────────┤
│ ● Robot 1 (127.0.0.1:16001)         │
│ ○ Robot 2 (192.168.1.100:16001)     │
│ ○ Robot 3 (192.168.1.101:16001)     │
│                                     │
│ [+ Add Robot]                       │
└─────────────────────────────────────┘
```

### Deliverables

- [ ] Comprehensive logging system
- [ ] Error handling with recovery
- [ ] I/O monitoring and control
- [ ] Motion recording feature
- [ ] Multi-robot support

### Testing

- [ ] Test error scenarios
- [ ] Verify logging works
- [ ] Test I/O control
- [ ] Validate motion recording
- [ ] Test multi-robot switching

---

## Phase 5: Polish and Documentation

**Goal**: Production-ready release with complete documentation

**Duration**: 2-3 days

### Tasks

#### 5.1: Performance Optimization

- [ ] Optimize WebSocket message frequency
- [ ] Reduce unnecessary re-renders
- [ ] Implement efficient state management
- [ ] Profile and optimize hot paths

#### 5.2: User Documentation

**Create**:
- [ ] User manual (how to use the interface)
- [ ] Quick start guide
- [ ] Troubleshooting guide
- [ ] Video tutorials (optional)

#### 5.3: Developer Documentation

**Update**:
- [ ] API documentation
- [ ] Architecture diagrams
- [ ] Contribution guidelines
- [ ] Code examples

#### 5.4: Testing and Validation

- [ ] End-to-end testing
- [ ] User acceptance testing
- [ ] Performance testing
- [ ] Security review

#### 5.5: Deployment

- [ ] Build production bundle
- [ ] Create Docker image
- [ ] Deployment documentation
- [ ] Release notes

### Deliverables

- [ ] Optimized, production-ready code
- [ ] Complete user documentation
- [ ] Updated developer docs
- [ ] Deployment package
- [ ] Release v1.0.0

---

## Technology Stack

### Backend

- **Language**: Rust
- **Framework**: Tokio async runtime
- **WebSocket**: tokio-tungstenite
- **Serialization**: serde, bincode (binary protocol)
- **Math**: nalgebra (for coordinate transformations)
- **Logging**: tracing, tracing-subscriber

### Frontend

- **Framework**: Leptos 0.6.15 (Rust WASM framework with reactive signals)
- **Styling**: Tailwind CSS (via CDN)
- **Font**: Inter (Google Fonts)
- **WebSocket**: web-sys WebSocket API
- **Serialization**: bincode (binary DTO protocol)
- **Build Tool**: Trunk (WASM bundler)
- **Crate Type**: cdylib + rlib for WASM compilation

### Design System (Existing - Must Preserve)

- **Background**: `#0a0a0a` (near black)
- **Panels**: `#111111` (dark gray)
- **Nested panels**: `#1a1a1a` (slightly lighter)
- **Accent color**: `#00d9ff` (cyan) - for highlights, active states, headers
- **Text colors**:
  - Primary: `white`
  - Secondary: `#cccccc`
  - Tertiary: `#888888`
  - Disabled: `#666666`
- **Borders**: `#ffffff10` (10% white), `#ffffff08` (8% white)
- **Font**: Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif
- **Animations**: Subtle pulse for connection indicator
- **Hover states**: Cyan background (`#00d9ff`) with black text

### Infrastructure

- **Version Control**: Git
- **CI/CD**: GitHub Actions (optional)
- **Containerization**: Docker (optional)

---

## Dependencies and Prerequisites

### New Rust Dependencies

**Backend (fanuc_rmi, web_server)**:
```toml
[dependencies]
nalgebra = "0.32"  # For coordinate transformations
tracing = "0.1"    # For structured logging
tracing-subscriber = "0.3"  # Log formatting
```

**Frontend (web_app)**:
```toml
[dependencies]
leptos_router = { version = "0.6.15", features = ["csr"] }  # For routing (optional, if using routes)
# All other dependencies already present
```

### Development Tools

- Rust 1.70+
- Node.js (for frontend tooling, optional)
- FANUC robot or simulator
- Git

---

## Risk Assessment and Mitigation

### Risk 1: Coordinate Transformation Accuracy

**Risk**: Math errors in frame transformations could cause incorrect position display

**Mitigation**:
- Extensive unit testing
- Validation against real robot
- Cross-check with teach pendant
- Use well-tested nalgebra library

### Risk 2: Real-Time Performance

**Risk**: High-frequency position updates (125Hz) could overwhelm WebSocket

**Mitigation**:
- Implement throttling
- Use binary WebSocket messages (not JSON)
- Profile and optimize
- Make update rate configurable

### Risk 3: Robot Safety

**Risk**: UI bugs could send dangerous commands to robot

**Mitigation**:
- Extensive safety checks in backend
- Require confirmation for critical actions
- Implement emergency stop
- Thorough testing before production use

### Risk 4: Complexity Creep

**Risk**: Feature additions could make UI overwhelming

**Mitigation**:
- Phased implementation
- User feedback at each phase
- Keep advanced features optional
- Maintain simple default view

---

## Success Metrics

### Important: Preserving Legacy App

**Requirement**: The existing Leptos web app must be preserved as a "legacy" route during development.

**Implementation Strategy**:

1. **Create routing structure** using Leptos Router
2. **Legacy route**: `/` or `/legacy` - Current simple interface
3. **New route**: `/v2` or `/advanced` - New comprehensive interface
4. **Shared components**: WebSocketManager, robot_models remain shared
5. **Gradual migration**: Users can switch between interfaces during development

**Leptos Router Setup**:
```rust
// web_app/src/lib.rs
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
    // Current simple interface (existing code)
    view! {
        <div class="min-h-screen bg-[#0a0a0a]">
            // ... existing layout
        </div>
    }
}

#[component]
fn NewApp() -> impl IntoView {
    // New comprehensive interface with tabs
    view! {
        <div class="min-h-screen bg-[#0a0a0a]">
            // ... new tabbed layout
        </div>
    }
}
```

**Benefits**:
- No disruption to existing functionality
- Easy A/B testing
- Gradual feature rollout
- Fallback if issues arise

---

## Success Metrics

### Phase 1 Success Criteria

- [ ] Frame/tool information displayed correctly
- [ ] Position matches teach pendant when in same frame
- [ ] All new commands working
- [ ] Legacy app still functional

### Phase 2 Success Criteria

- [ ] Multi-frame display mathematically correct
- [ ] Frame switching works safely
- [ ] User can match TP display by selecting correct frame
- [ ] Both legacy and new routes working

### Phase 3 Success Criteria

- [ ] Professional, intuitive UI matching existing aesthetic
- [ ] All tabs functional
- [ ] Positive user feedback
- [ ] Smooth routing between legacy and new interface

### Phase 4 Success Criteria

- [ ] All advanced features working
- [ ] No critical bugs
- [ ] Performance acceptable (< 100ms latency)
- [ ] Feature parity with legacy app

### Phase 5 Success Criteria

- [ ] Production-ready release
- [ ] Complete documentation
- [ ] Successful deployment
- [ ] Decision made on legacy app deprecation timeline

---

## Timeline Summary

| Phase | Duration | Cumulative |
|-------|----------|------------|
| Phase 1: Foundation | 1-2 days | 2 days |
| Phase 2: Multi-Frame | 2-3 days | 5 days |
| Phase 3: UI/UX | 3-4 days | 9 days |
| Phase 4: Advanced | 3-5 days | 14 days |
| Phase 5: Polish | 2-3 days | 17 days |

**Total Estimated Time**: 3-4 weeks (17 working days)

---

## Next Steps

### Immediate Actions

1. **Review this roadmap** with stakeholders
2. **Prioritize phases** based on business needs
3. **Set up development environment**
4. **Create feature branch** for Phase 1
5. **Begin implementation**

### Decision Points

Before starting each phase, decide:
- [ ] Are previous phase deliverables acceptable?
- [ ] Should we proceed as planned or adjust?
- [ ] Any new requirements to incorporate?
- [ ] Any features to defer or remove?

---

## Appendix: Related Documents

- [FANUC_ROBOTICS_FUNDAMENTALS.md](./FANUC_ROBOTICS_FUNDAMENTALS.md) - Learn robotics concepts
- [COORDINATE_FRAMES_GUIDE.md](./COORDINATE_FRAMES_GUIDE.md) - Frame transformation details
- [RMI_COMMANDS_REFERENCE.md](./RMI_COMMANDS_REFERENCE.md) - All RMI commands
- [FANUC_INITIALIZATION_SEQUENCE.md](./FANUC_INITIALIZATION_SEQUENCE.md) - Startup procedures
- B-84184EN_02.pdf - Official FANUC RMI manual

---

**Document Version**: 1.0
**Last Updated**: 2025-11-29
**Author**: FANUC RMI API Development Team
**Status**: Ready for Review



