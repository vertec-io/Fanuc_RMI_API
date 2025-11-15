# Jog Functionality Fix - Simulator State Tracking

## Problem

The FANUC simulator was **stateless** - it returned hardcoded values (all zeros for joint angles, Z=500.0 for cartesian position) regardless of any jog commands sent to it. When you jogged the robot via the UI, the simulator would:
1. Accept the `FRC_LinearRelative` instruction
2. Return a success response
3. **NOT update its internal robot state**
4. Continue returning the same hardcoded position values

This meant the Bevy canvas would never see the robot move because the position data never changed.

## Root Cause

In `/home/apino/dev/Fanuc_RMI_API/sim/src/main.rs`, the `FRC_LinearRelative` instruction handler (line 230) was just returning a success response without updating the `robot_state`:

```rust
Some("FRC_LinearRelative") => json!({
    "Instruction": "FRC_LinearRelative",
    "ErrorID": 0,
    "SequenceID": seq
}),
```

## Solution

Modified the `FRC_LinearRelative` handler to:
1. Parse the `Position` data from the instruction (X, Y, Z deltas)
2. Lock the `robot_state` mutex
3. Update `cartesian_position` by adding the relative movement
4. Log the updated position for debugging
5. Return the success response

### Code Changes

**File**: `/home/apino/dev/Fanuc_RMI_API/sim/src/main.rs` (lines 224-256)

```rust
Some("FRC_LinearRelative") => {
    // Parse the Position from the instruction
    if let Some(position) = request_json.get("Position") {
        let dx = position["X"].as_f64().unwrap_or(0.0) as f32;
        let dy = position["Y"].as_f64().unwrap_or(0.0) as f32;
        let dz = position["Z"].as_f64().unwrap_or(0.0) as f32;
        
        // Update robot state with relative movement
        let mut state = robot_state.lock().await;
        state.cartesian_position[0] += dx;
        state.cartesian_position[1] += dy;
        state.cartesian_position[2] += dz;
        
        println!("Updated position: X={:.2}, Y={:.2}, Z={:.2}", 
            state.cartesian_position[0],
            state.cartesian_position[1],
            state.cartesian_position[2]);
    }
    
    json!({
        "Instruction": "FRC_LinearRelative",
        "ErrorID": 0,
        "SequenceID": seq
    })
},
```

## How Jog Commands Work

### From UI to Simulator

1. **User clicks jog button** in Bevy UI (e.g., "Jog +X")
2. **Bevy sends `JogMessage`** via WebSocket to Meteorite server
3. **Meteorite `jog` system** (in `plugins/src/fanuc_driver/systems/jog.rs`):
   - Checks authorization
   - Calculates distance based on jog mode (Step or Continuous)
   - Creates `FrcLinearRelative` instruction with Position delta
   - Sends to FANUC driver
4. **FANUC driver** sends instruction to simulator
5. **Simulator** (NOW FIXED):
   - Parses Position delta from instruction
   - Updates internal `robot_state.cartesian_position`
   - Returns success response
6. **Polling systems** (`poll_joint_angles`, `poll_cartesian_position`):
   - Continue polling every 100ms
   - Receive UPDATED position from simulator
7. **Three-tier relay** processes responses and updates components
8. **Broadcast system** detects `Changed<CurrentCartesianPosition>`
9. **Broadcasts to Bevy clients** via WebSocket
10. **Bevy canvas** receives updated position and moves robot visualization

## Testing

### Running the System

1. **Start Simulator** (Terminal 112):
   ```bash
   cd /home/apino/dev/Fanuc_RMI_API
   cargo run --release --bin sim
   ```

2. **Start Meteorite Server** (Terminal 114):
   ```bash
   cd /home/apino/dev/meteorite
   cargo run --bin server --features ecs
   ```

3. **Start Bevy Frontend** (Terminal 98):
   ```bash
   cd /home/apino/dev/meteorite/app
   trunk serve --port 3000
   ```

4. **Open Browser**: http://localhost:3000

### What to Look For

#### Simulator Console (Terminal 112)
When you jog the robot, you should see:
```
Received on secondary port: {"Instruction":"FRC_LinearRelative","Position":{"X":10.0,"Y":0.0,"Z":0.0},...}
Updated position: X=10.00, Y=0.00, Z=500.00
Sent: {"Instruction":"FRC_LinearRelative","ErrorID":0,"SequenceID":1}
```

#### Server Console (Terminal 114)
You should see jog messages being sent:
```
INFO got a jog message
INFO Sent jog message to fanuc driver
```

#### Bevy Console (Browser DevTools)
You should see updated robot state messages:
```
INFO Received robot state from WebSocket: RobotVisualizationState { ... }
```

#### Bevy Canvas
**The robot should actually move!** Each jog command will update the position, and you'll see the robot move in real-time.

## Current Status

- ✅ Simulator rebuilt with state tracking
- ✅ Simulator running on Terminal 112
- ✅ Server running on Terminal 114 (filtered for jog messages)
- ✅ Bevy frontend running on Terminal 98
- ✅ Ready to test jog functionality!

## Next Steps

1. Open http://localhost:3000 in your browser
2. Navigate to the robot jog controls
3. Click jog buttons (e.g., +X, -X, +Y, -Y, +Z, -Z)
4. Watch the robot move in the Bevy canvas!
5. Check the simulator console to see position updates

## Notes

- The simulator currently only tracks **cartesian position** (X, Y, Z)
- Joint angles are not yet updated (would require inverse kinematics)
- For now, the robot will move in cartesian space, which is sufficient for testing
- Future enhancement: Add inverse kinematics to update joint angles based on cartesian position

