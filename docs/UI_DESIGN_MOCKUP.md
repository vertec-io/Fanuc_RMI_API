# UI Design Mockup - Desktop Application Layout

**Date**: 2025-11-29  
**Version**: 2.0 (App-style, not webpage-style)

---

## Design Principles

✅ **App-like, not webpage-like** - Full-window layout, no scrolling (except logs)  
✅ **Always-visible essentials** - Jog controls and position always on screen  
✅ **Dense but clean** - Efficient space usage, modern aesthetic  
✅ **Tooltips over descriptions** - Keep UI uncluttered  
✅ **Responsive** - Adapts to screen sizes gracefully  

---

## Desktop Layout (>1280px)

```
┌────────────────────────────────────────────────────────────────────────────────┐
│ FANUC RMI Control    [Robot: 192.168.1.100]    [●] Connected    [⚙]          │
├──────────┬────────────────────────────────────────────────────┬────────────────┤
│          │                                                    │                │
│   NAV    │              MAIN WORKSPACE                        │  ALWAYS        │
│   BAR    │                                                    │  VISIBLE       │
│          │                                                    │                │
│  [☰]     │  ┌──────────────────────────────────────────────┐  │  ┌──────────┐  │
│  60px    │  │ Dashboard / Position / Control / Settings    │  │  │ Position │  │
│  wide    │  │                                              │  │  │          │  │
│          │  │  (Content changes based on nav selection)    │  │  │ X: 6.02  │  │
│  ┌────┐  │  │                                              │  │  │ Y:-35.69 │  │
│  │ 📊 │  │  │                                              │  │  │ Z: 26.05 │  │
│  │Dash│  │  │                                              │  │  │          │  │
│  └────┘  │  │                                              │  │  │ UFrame 3 │  │
│          │  │                                              │  │  │ UTool 1  │  │
│  ┌────┐  │  │                                              │  │  └──────────┘  │
│  │ 🎯 │  │  │                                              │  │                │
│  │Pos │  │  │                                              │  │  ┌──────────┐  │
│  └────┘  │  │                                              │  │  │   Jog    │  │
│          │  │                                              │  │  │  Ctrl    │  │
│  ┌────┐  │  │                                              │  │  │          │  │
│  │ 🎮 │  │  │                                              │  │  │    ↑     │  │
│  │Ctrl│  │  │                                              │  │  │  ← ● →   │  │
│  └────┘  │  │                                              │  │  │    ↓     │  │
│          │  │                                              │  │  │          │  │
│  ┌────┐  │  │                                              │  │  │  ▲   ▼   │  │
│  │ ⚙  │  │  │                                              │  │  │  Z+   Z- │  │
│  │Set │  │  │                                              │  │  │          │  │
│  └────┘  │  │                                              │  │  │ Speed:10 │  │
│          │  │                                              │  │  │ Step: 1  │  │
│          │  └──────────────────────────────────────────────┘  │  └──────────┘  │
│          │                                                    │                │
│          │                                                    │  ┌──────────┐  │
│          │                                                    │  │  Status  │  │
│          │                                                    │  │          │  │
│          │                                                    │  │ ● Servo  │  │
│          │                                                    │  │ ● AUTO   │  │
│          │                                                    │  │ ● IDLE   │  │
│          │                                                    │  └──────────┘  │
│          │                                                    │                │
└──────────┴────────────────────────────────────────────────────┴────────────────┘
```

**Dimensions**:
- Left sidebar: 60px (collapsed) or 200px (expanded)
- Right panel: 280px (fixed)
- Center workspace: Flexible (fills remaining space)
- Top bar: 56px (fixed)
- Total height: 100vh (no scrolling)

---

## Tablet Layout (768-1280px)

```
┌──────────────────────────────────────────────────────────────┐
│ FANUC RMI    [192.168.1.100]  [●]  [⚙]                      │
├────┬─────────────────────────────────────────┬───────────────┤
│    │                                         │               │
│ [☰]│         MAIN WORKSPACE                  │  ESSENTIALS   │
│    │                                         │               │
│ 📊 │                                         │  ┌─────────┐  │
│    │                                         │  │ Pos     │  │
│ 🎯 │                                         │  │ X: 6.02 │  │
│    │                                         │  └─────────┘  │
│ 🎮 │                                         │               │
│    │                                         │  ┌─────────┐  │
│ ⚙  │                                         │  │  Jog    │  │
│    │                                         │  │  ↑←↓→   │  │
│    │                                         │  └─────────┘  │
│    │                                         │               │
└────┴─────────────────────────────────────────┴───────────────┘
```

**Changes**:
- Sidebar: Icon-only (48px)
- Right panel: Narrower (220px)
- Compact jog controls
- Smaller fonts

---

## Mobile Layout (<768px)

```
┌──────────────────────────────────────┐
│ FANUC RMI  [●]  [⚙]                 │
├──────────────────────────────────────┤
│                                      │
│                                      │
│         MAIN WORKSPACE               │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
│                                      │
├──────────────────────────────────────┤
│ [📊] [🎯] [🎮] [⚙]    [Jog ▲]       │
└──────────────────────────────────────┘
```

**Changes**:
- Bottom navigation bar
- Jog controls in modal/drawer (tap [Jog ▲] to open)
- Position in collapsible header
- Full-width workspace
- Stacked layout

---

## Color Palette (Existing - Preserved)

```css
/* Backgrounds */
--bg-app: #0a0a0a;          /* Main background */
--bg-panel: #111111;        /* Panels/cards */
--bg-nested: #1a1a1a;       /* Nested elements */
--bg-hover: #222222;        /* Hover states */

/* Accent */
--accent: #00d9ff;          /* Cyan - active states, highlights */
--accent-hover: #00b8e6;    /* Darker cyan for hover */

/* Text */
--text-primary: #ffffff;    /* Primary text */
--text-secondary: #cccccc;  /* Secondary text */
--text-tertiary: #888888;   /* Labels, placeholders */
--text-disabled: #666666;   /* Disabled text */

/* Borders */
--border-primary: #ffffff10;  /* 10% white */
--border-secondary: #ffffff08; /* 8% white */

/* Status Colors */
--status-success: #00ff88;  /* Green */
--status-warning: #ffaa00;  /* Orange */
--status-error: #ff4444;    /* Red */
--status-info: #00d9ff;     /* Cyan */
```

---

## Component Patterns

### Panel/Card
```rust
<div class="bg-[#111111] rounded border border-[#ffffff10] p-4">
    <h3 class="text-sm font-semibold text-[#00d9ff] mb-3 uppercase tracking-wide">
        "Panel Title"
    </h3>
    // Content
</div>
```

### Nested Element
```rust
<div class="bg-[#1a1a1a] rounded border border-[#ffffff08] p-2.5">
    <span class="text-[#888888] text-sm">"Label:"</span>
    <span class="text-white font-mono">"Value"</span>
</div>
```

### Button (Primary)
```rust
<button class="bg-[#00d9ff] hover:bg-[#00b8e6] text-black font-semibold py-2 px-4 rounded transition-colors">
    "Action"
</button>
```

### Button (Secondary)
```rust
<button class="bg-[#1a1a1a] hover:bg-[#00d9ff] border border-[#ffffff08] hover:border-[#00d9ff] text-white hover:text-black font-semibold py-2 px-4 rounded transition-colors">
    "Action"
</button>
```

### Tooltip
```rust
<div class="group relative">
    <button>"Hover me"</button>
    <div class="absolute hidden group-hover:block bg-[#111111] border border-[#ffffff10] rounded p-2 text-xs text-[#cccccc] whitespace-nowrap bottom-full mb-2">
        "Tooltip text"
    </div>
</div>
```

---

**Next**: See IMPLEMENTATION_ROADMAP.md for detailed feature breakdown

