# Implementation Notes - Important Corrections

**Date**: 2025-11-29  
**Status**: Critical corrections to implementation roadmap

---

## ⚠️ Critical Correction: Technology Stack

### Frontend Framework

**INCORRECT** (Original roadmap):
- HTML/CSS/JavaScript
- Files: `index.html`, `style.css`, `script.js`

**CORRECT** (Actual implementation):
- **Leptos 0.6.15** - Rust WASM framework with reactive signals
- **Tailwind CSS** - Utility-first CSS via CDN
- **Bincode** - Binary serialization for WebSocket communication
- **Trunk** - WASM build tool

### Current Web App Structure

```
web_app/
├── Cargo.toml              # Leptos dependencies
├── index.html              # Minimal HTML shell
├── src/
│   ├── lib.rs              # Main app component
│   ├── websocket.rs        # WebSocket manager
│   ├── robot_models.rs     # Data models
│   └── components/
│       ├── mod.rs
│       ├── robot_status.rs
│       ├── position_display.rs
│       ├── jog_controls.rs
│       ├── error_log.rs
│       ├── motion_log.rs
│       └── settings.rs
└── dist/                   # Build output (WASM)
```

---

## 🎨 Design System (Must Preserve)

The existing Leptos app has a **dark futuristic aesthetic** that MUST be maintained:

### Color Palette

```css
/* Backgrounds */
--bg-primary: #0a0a0a;      /* Near black */
--bg-panel: #111111;        /* Dark gray panels */
--bg-nested: #1a1a1a;       /* Nested elements */

/* Accent */
--accent: #00d9ff;          /* Cyan - for highlights, active states */

/* Text */
--text-primary: #ffffff;    /* White */
--text-secondary: #cccccc;  /* Light gray */
--text-tertiary: #888888;   /* Medium gray */
--text-disabled: #666666;   /* Dark gray */

/* Borders */
--border-subtle: #ffffff10; /* 10% white opacity */
--border-nested: #ffffff08; /* 8% white opacity */
```

### Typography

- **Font Family**: Inter, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif
- **Headers**: Uppercase, tracking-wide, cyan color (`#00d9ff`)
- **Values**: Monospace font for numbers
- **Labels**: Small, gray (`#888888`)

### Component Patterns

**Panel Structure**:
```rust
view! {
    <div class="bg-[#111111] rounded border border-[#ffffff10] p-4">
        <h2 class="text-sm font-semibold text-[#00d9ff] mb-3 flex items-center uppercase tracking-wide">
            // SVG icon
            "Panel Title"
        </h2>
        // Content
    </div>
}
```

**Nested Elements**:
```rust
<div class="bg-[#1a1a1a] rounded border border-[#ffffff08] p-2.5">
    <span class="text-[#888888] text-sm">"Label:"</span>
    <span class="text-white font-mono">"Value"</span>
</div>
```

**Buttons**:
```rust
<button class="bg-[#1a1a1a] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-3 px-4 rounded transition-colors">
    "Button Text"
</button>
```

**Active States**:
- Background changes to cyan (`#00d9ff`)
- Text changes to black for contrast
- Smooth transitions

---

## 📋 Requirement: Preserve Legacy App

### Strategy

1. **Keep existing app functional** during development
2. **Create routing structure**:
   - `/` or `/legacy` - Current simple interface (PRESERVED)
   - `/v2` or `/advanced` - New desktop-style application

### Recommended Approach: Routing with Legacy Preservation

**Clean separation** - Keep legacy app intact, build new app separately:

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

// Legacy app (existing simple interface)
#[component]
fn LegacyApp() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-[#0a0a0a]">
            <div class="container mx-auto px-6 py-6">
                <Header/>

                // Link to new app
                <div class="mt-4">
                    <a href="/v2" class="text-[#00d9ff] hover:underline text-sm">
                        "Try the new advanced interface →"
                    </a>
                </div>

                // Existing layout
                <div class="grid grid-cols-1 lg:grid-cols-3 gap-4 mt-6">
                    // ... existing components
                </div>
            </div>
        </div>
    }
}

// New desktop-style app
#[component]
fn NewApp() -> impl IntoView {
    view! {
        <div class="h-screen w-screen bg-[#0a0a0a] flex flex-col overflow-hidden">
            <TopBar/>
            <div class="flex-1 flex overflow-hidden">
                <Sidebar/>
                <MainWorkspace/>
                <RightPanel/>  // Always visible: Position, Jog, Status
            </div>
        </div>
    }
}
```

**Benefits**:
- Clean separation of legacy and new code
- Legacy app remains untouched and functional
- Easy to switch between interfaces
- Can deprecate legacy later
- Shared WebSocket context

---

## 🔧 Files to Modify (Corrected)

### Phase 1: Frame Awareness

**Backend**:
- `fanuc_rmi/src/commands/` - New command files
- `fanuc_rmi/src/lib.rs` - Export new commands
- `fanuc_rmi/src/drivers/driver.rs` - Add driver methods
- `fanuc_rmi/src/frame_manager.rs` - NEW FILE
- `web_server/src/main.rs` - Integrate FrameManager

**Frontend (Leptos)**:
- `web_app/src/robot_models.rs` - Add frame/tool fields
- `web_app/src/websocket.rs` - Handle frame data
- `web_app/src/components/position_display.rs` - Show frame info
- `web_app/src/components/mod.rs` - Export new components

### Phase 2: Multi-Frame Display

**Frontend (Leptos)**:
- `web_app/src/components/multi_frame_display.rs` - NEW FILE
- `web_app/src/components/frame_selector.rs` - NEW FILE

### Phase 3: Tabbed Interface

**Frontend (Leptos)**:
- `web_app/src/lib.rs` - Add view mode toggle
- `web_app/src/components/tabs.rs` - NEW FILE
- `web_app/src/components/dashboard_tab.rs` - NEW FILE
- `web_app/src/components/position_tab.rs` - NEW FILE
- `web_app/src/components/control_tab.rs` - NEW FILE
- `web_app/src/components/io_tab.rs` - NEW FILE
- `web_app/src/components/settings_tab.rs` - NEW FILE
- `web_app/src/components/logs_tab.rs` - NEW FILE

---

## 🚀 Build and Development

### Building the Web App

```bash
# Install trunk if not already installed
cargo install trunk

# Development build with auto-reload
cd web_app
trunk serve

# Production build
trunk build --release
```

### Running the Full Stack

```bash
# Terminal 1: Simulator
cargo run -p sim -- --realtime

# Terminal 2: Web Server
cargo run -p web_server

# Terminal 3: Web App (development)
cd web_app && trunk serve

# Open browser to http://localhost:8080 (trunk default)
# WebSocket connects to ws://localhost:9000
```

---

## ✅ Key Takeaways

1. **Use Leptos**, not vanilla HTML/CSS/JS
2. **Preserve the dark futuristic aesthetic** - it's already excellent
3. **Keep legacy app accessible** at `/` or `/legacy` route
4. **Desktop-style app, not webpage** - Full-window layout, no scrolling (except logs)
5. **Always-visible essentials** - Jog controls, position, status in right panel
6. **Use Leptos reactive signals** for state management
7. **Maintain existing component structure** and patterns
8. **Use Tailwind utility classes** matching existing style
9. **Binary WebSocket protocol** (bincode) is already implemented
10. **Tooltips over descriptions** - Keep UI clean and uncluttered
11. **Responsive design** - Collapsible sidebar, adaptive layouts
12. **Dense but modern** - Efficient space usage with accordions

---

**Document Version**: 1.0  
**Last Updated**: 2025-11-29  
**Author**: FANUC RMI API Development Team

