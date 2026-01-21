# PRD: Program Pause/Resume for Fanuc RMI Driver

## Type
Feature

## Introduction

Implement a new "Program Pause/Resume" functionality for the Fanuc RMI driver that is fundamentally different from the existing `pause()` and `continue()` methods.

**Current behavior (`pause()`/`continue()`):** Completely pauses the robot controller and execution queue. The robot stops and cannot be moved at all.

**New behavior (`program_pause()`/`program_resume()`):** Aborts the current RMI_MOVE TP program on the controller, allowing the robot to be jogged or controlled through other means (e.g., teach pendant), while tracking in-flight instructions. On resume, the RMI program is re-initialized and tracked instructions are replayed, allowing seamless continuation.

### Use Case
1. Running a motion program, streaming commands to the Fanuc driver
2. Operator sees an issue (e.g., potential collision, misaligned part)
3. "Program pause" - aborts RMI program but robot remains controllable
4. Jog the robot away from the part using teach pendant or other means
5. Validate everything is correct
6. Jog the robot back to position
7. "Program resume" - replays in-flight instructions and continues normal operation

## Goals

- Implement `program_pause()` method that aborts RMI_MOVE program while preserving in-flight instruction state
- Implement `program_resume()` method that re-initializes and replays tracked instructions
- Track in-flight instructions (packets sent but not completed) for replay capability
- Maintain proper sequence ID management across pause/resume cycles
- Provide clear error handling for edge cases (resume without pause, pause during non-running state)
- Create comprehensive tests using the simulator crate

## User Stories

### US-001: Add ProgramPaused state to DriverState enum
**Description:** As a developer, I need a distinct state to represent when the program is paused (different from Paused) so the driver can differentiate between controller pause and program pause.

**Acceptance Criteria:**
- [ ] Add `ProgramPaused` variant to `DriverState` enum in models.rs
- [ ] Update any match statements that handle DriverState to include the new variant
- [ ] Typecheck passes

### US-002: Track in-flight instructions for replay
**Description:** As a developer, I need to store sent instructions that haven't completed yet so they can be replayed on resume.

**Acceptance Criteria:**
- [ ] Create a data structure to store in-flight instruction packets with their original data
- [ ] Store instruction data when packets are sent to controller (before sequence ID assignment)
- [ ] Remove instructions from tracking when completion responses are received
- [ ] Maximum tracking capacity of 8 instructions (matching RMI buffer limit)
- [ ] Typecheck passes

### US-003: Implement program_pause method
**Description:** As a user of the driver, I want to pause the running program so I can jog the robot while preserving my queued instructions.

**Acceptance Criteria:**
- [ ] Method returns error if driver is not in Running state
- [ ] Sends FRC_Abort command to terminate RMI_MOVE program
- [ ] Waits for abort response before completing
- [ ] Preserves the internal instruction queue (does not clear it)
- [ ] Preserves tracked in-flight instructions for later replay
- [ ] Sets driver state to ProgramPaused
- [ ] Resets in_flight counter to 0 (robot's buffer is cleared by abort)
- [ ] Typecheck passes

### US-004: Implement program_resume method
**Description:** As a user of the driver, I want to resume the paused program so the robot continues executing from where it left off.

**Acceptance Criteria:**
- [ ] Method returns error if driver is not in ProgramPaused state
- [ ] Sends FRC_Initialize command to create new RMI_MOVE program
- [ ] Resets sequence counter to 1 (as required after initialize)
- [ ] Replays all tracked in-flight instructions in original order
- [ ] Clears in-flight tracking after successful replay
- [ ] Sets driver state to Running
- [ ] Resumes processing the internal instruction queue
- [ ] Typecheck passes

### US-005: Handle instruction completions during program pause
**Description:** As a developer, I need to properly handle any late-arriving completion responses during the pause transition.

**Acceptance Criteria:**
- [ ] Completion responses received after abort do not cause errors
- [ ] In-flight tracking correctly accounts for completions received during abort
- [ ] No duplicate instruction replay occurs
- [ ] Typecheck passes

### US-006: Add integration tests with simulator
**Description:** As a developer, I want comprehensive tests to verify program pause/resume works correctly.

**Acceptance Criteria:**
- [ ] Test basic program_pause followed by program_resume
- [ ] Test that in-flight instructions are correctly replayed on resume
- [ ] Test that internal queue instructions continue after replay
- [ ] Test error case: program_pause when not running
- [ ] Test error case: program_resume when not program-paused
- [ ] Test multiple pause/resume cycles in sequence
- [ ] All tests pass with simulator (cargo run -p sim -- --realtime)
- [ ] Typecheck passes

## Functional Requirements

- FR-1: Add `ProgramPaused` variant to `DriverState` enum to distinguish from `Paused` state
- FR-2: Create `InFlightTracker` structure to store instruction packets awaiting completion (max 8)
- FR-3: `program_pause()` must send FRC_Abort and wait for response before returning
- FR-4: `program_pause()` must preserve both the internal queue and in-flight instruction data
- FR-5: `program_pause()` must set state to ProgramPaused and reset in_flight counter to 0
- FR-6: `program_resume()` must send FRC_Initialize and reset sequence counter to 1
- FR-7: `program_resume()` must replay all tracked in-flight instructions before resuming queue processing
- FR-8: `program_resume()` must assign new sequential sequence IDs to replayed instructions
- FR-9: Both methods must return appropriate errors for invalid state transitions
- FR-10: The `send_queue_to_controller` loop must respect ProgramPaused state (stop sending)

## Non-Goals

- No position tracking or verification (user is responsible for robot position on resume)
- No automatic position restoration after jogging
- No modification to existing `pause()`/`continue()` behavior
- No support for partial instruction replay (all tracked instructions are replayed)
- No timeout handling for pause duration
- No integration with teach pendant or other control modes

## Technical Considerations

### FANUC RMI Protocol Requirements
From the FANUC B-84184EN_02 specification:
- Section 3.4: "you have to manage the SKIP or JUMP instruction in your application program by calling FRC_Abort to terminate the current RMI_MOVE TP program, and use FRC_Initialize to create the program again"
- FRC_Abort clears the robot's instruction buffer (up to 8 instructions)
- FRC_Initialize creates a new RMI_MOVE program and resets sequence counter
- Sequence IDs must be consecutive starting from 1 after initialize

### Existing Code to Leverage
- `startup_sequence()` method (driver.rs:795-871) provides template for abort + initialize flow
- `abort()` method (driver.rs:257-313) handles FRC_Abort with response waiting
- `initialize()` method (driver.rs:585-644) handles FRC_Initialize with sequence reset
- `SentInstructionInfo` struct already tracks sent packets for completion matching

### Implementation Approach
1. Modify `SentInstructionInfo` or create parallel tracking to store original instruction data
2. Store instruction data before sending (not just after)
3. On pause: abort, preserve tracking, set ProgramPaused state
4. On resume: initialize, replay tracked instructions, clear tracking, set Running state
5. Ensure `send_queue_to_controller` respects ProgramPaused state

### Concurrency Considerations
- The `send_queue_to_controller` runs as an async task
- State changes must be properly synchronized via existing atomic/mutex mechanisms
- In-flight tracking must handle concurrent completion responses

## Success Metrics

- Program pause/resume cycle completes without errors
- All in-flight instructions are replayed in correct order on resume
- Internal queue continues processing after replay
- No instruction loss or duplication
- Clear error messages for invalid state transitions
- All simulator tests pass

## Open Questions

- Should there be a timeout or warning if paused for extended period?
- Should we emit events/callbacks for pause/resume state changes?
- Should program_resume verify robot is in a safe state before replaying?

## Merge Target

`main` - Merge to main branch when complete.
Auto-merge: No (ask for confirmation first)
