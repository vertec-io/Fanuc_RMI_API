# FANUC RMI Examples

This directory contains example applications demonstrating how to use the FANUC RMI library.

## Examples

### 1. Interactive Jogging Client with TUI (`jog_client_tui.rs`) ⭐ **RECOMMENDED**

A modern terminal user interface (TUI) for jogging a FANUC robot with real-time response display.

**Features:**
- Clean split-screen interface
- Real-time configuration display
- Scrolling response log showing all packets from the simulator
- No line breaks or scrolling - everything updates in place
- Visual feedback for active jogs

### 2. Interactive Jogging Client - Simple (`jog_client.rs`)

A simpler command-line interface for jogging. Responses print to console mixed with prompts.

## Prerequisites

- Rust toolchain installed
- FANUC simulator running (see instructions below)

## Quick Start

### Step 1: Start the Simulator

Open a terminal and run the simulator in **realtime mode** (recommended for realistic behavior):

```bash
cargo run -p sim -- --realtime
```

Or run in **immediate mode** (instant responses, good for rapid testing):

```bash
cargo run -p sim
```

You should see:
```
🤖 Starting FANUC Simulator in REALTIME mode
   (Simulates actual robot timing, return packets sent after execution)

🤖 FANUC Simulator started on 0.0.0.0:16001
   Waiting for connections...
```

### Step 2: Start the Jog Client

Open a **second terminal** and run the **TUI jog client** (recommended):

```bash
cargo run -p example --bin jog_client_tui
```

You'll see a split-screen interface with:
- **Top panel**: Configuration (speed, distance, mode, active jog, status)
- **Middle panel**: Response log showing all packets from the simulator
- **Bottom panel**: Help text with all available commands

Or run the simple command-line version:

```bash
cargo run -p example --bin jog_client
```

You should see the interactive menu:
```
╔════════════════════════════════════════╗
║         JOGGING CONFIGURATION          ║
╠════════════════════════════════════════╣
║ Jog Speed:         10.00 mm/s        ║
║ Step Distance:      1.00 mm          ║
║ Motion Mode:        Step             ║
╚════════════════════════════════════════╝
```

## Jog Client Controls

### Motion Controls

| Key | Direction | Axis |
|-----|-----------|------|
| `k` | Up        | +Z   |
| `j` | Down      | -Z   |
| `h` | Left      | -Y   |
| `l` | Right     | +Y   |
| `f` | Forward   | +X   |
| `b` | Backward  | -X   |

### Configuration Controls

| Key | Action |
|-----|--------|
| `s` | Set jog speed (mm/s) |
| `d` | Set step distance (mm) |
| `m` | Toggle motion mode (Step ↔ Continuous) |
| `q` | Quit |

## Motion Modes

### Step Mode (Default)

- Sends a single motion command per key press
- Uses **FINE** termination (robot stops precisely at target)
- Good for precise positioning

**Example:**
1. Press `k` → Robot moves up 1mm and stops
2. Press `k` again → Robot moves up another 1mm and stops

### Continuous Mode

- Streams motion commands while key is held
- Uses **CNT** termination (blended motion, no stopping between moves)
- Automatically adjusts send frequency based on motion duration
- Sends **FINE** termination move when you release the key (press same key again)

**Example:**
1. Press `m` to switch to Continuous mode
2. Press `f` → Robot starts moving forward continuously
3. Press `f` again → Robot sends final FINE move and stops smoothly

**Adaptive Frequency:**
The client automatically calculates the optimal send rate to prevent buffer overflow:
- 1mm @ 10mm/s = 100ms motion → sends every 100ms
- 1mm @ 50mm/s = 20ms motion → sends every 50ms (minimum)

## Simulator Modes

### Immediate Mode (Default)

```bash
cargo run -p sim
```

- Positions update instantly
- Return packets sent immediately
- Good for rapid testing and development

### Realtime Mode (Recommended)

```bash
cargo run -p sim -- --realtime
```

- Calculates motion duration based on distance and speed
- Delays return packets until motion completes
- Simulates actual FANUC robot controller behavior
- Shows timing information in logs

**Example output:**
```
🎯 FRC_LinearRelative: ΔX=+1.0 ΔY=+0.0 ΔZ=+0.0 | Speed=10.0mm/s | Term=CNT CNT=100 | Pos=[1110.2, 0.0, 245.0]
   ⏱️  Motion will take 0.10s to complete
```

## Example Workflow

### Basic Jogging

1. Start simulator: `cargo run -p sim -- --realtime`
2. Start jog client: `cargo run -p example --bin jog_client`
3. Press `k` to move up
4. Press `j` to move down
5. Press `q` to quit

### Continuous Jogging

1. Start simulator: `cargo run -p sim -- --realtime`
2. Start jog client: `cargo run -p example --bin jog_client`
3. Press `m` to switch to Continuous mode
4. Press `f` to start moving forward
5. Watch the robot move continuously in the simulator logs
6. Press `f` again to stop
7. Press `q` to quit

### High-Speed Jogging

1. Start simulator: `cargo run -p sim -- --realtime`
2. Start jog client: `cargo run -p example --bin jog_client`
3. Press `s` and enter `50` for 50mm/s speed
4. Press `m` to switch to Continuous mode
5. Press `f` to move forward at high speed
6. Notice the faster send frequency: `📊 Continuous mode: sending every 50ms`

## Troubleshooting

### "Connection refused" error

Make sure the simulator is running first before starting the jog client.

### Robot not moving

Check that the simulator shows "Client connected" message when you start the jog client.

### Buffer overflow warnings

This shouldn't happen with the adaptive frequency, but if it does:
- Reduce jog speed with `s` command
- Increase step distance with `d` command

## Technical Details

### TermType and TermValue

The jog client uses two termination types:

- **FINE** (Step mode): Robot stops precisely at each target position
- **CNT** (Continuous mode): Robot blends motion smoothly without stopping
  - CNT value 1-100 controls smoothness (100 = maximum smoothness)
  - The client uses CNT100 for smooth continuous motion
  - CNT1 for the final termination move (tight stop)

### Buffer Management

- FANUC controllers support up to 8 concurrent motion instructions
- The client uses a conservative limit of 6 instructions
- Adaptive send frequency prevents buffer overflow
- Send interval matches or exceeds motion duration

## See Also

- [FANUC RMI Library Documentation](../fanuc_rmi/src/lib.rs)
- [Simulator Source](../sim/src/main.rs)

