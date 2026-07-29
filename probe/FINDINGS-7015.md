# `FRC_Initialize` → 7015 on COMET1 (192.168.0.100)

Live diagnosis, 2026-07-29, controller RMI v9.0. Run with:

```
cargo run -p fanuc_probe --bin init_diag -- --addr 192.168.0.100:16001
```

## Reproduction

Software cannot clear this. Every recovery sequence was tried against the live
controller and all returned the same rejection:

| Sequence | `FRC_Initialize` result |
|---|---|
| bare (no preamble) | `7015` |
| `FRC_Abort` → Initialize | `7015` |
| `FRC_Reset` → Initialize (Reset itself returned `ErrorID 0`) | `7015` |
| `FRC_Abort` + `FRC_Reset` → Initialize | `7015` |
| same, GroupMask `0b11` | `7015` |

`FRC_ReadError` returns `"No Error"` immediately after a 7015. The controller
does not log it as a fault — 7015 is a **rejection of the request**, not a
latched error state, which is why nothing on the pendant looks wrong and why
clearing faults changes nothing.

## ROOT CAUSE (B-84184EN/03 §2.3.1, §2.3.7)

Status is stable and byte-identical across every run:

```json
{"ErrorID":0,"ServoReady":1,"TPMode":0,"RMIMotionStatus":0,"ProgramStatus":0,
 "SingleStepMode":0,"NumberUTool":10,"NumberUFrame":9,"Override":10}
```

Decoded against the manual's own field definitions (§2.3.7):

| Field | Value | Manual |
|---|---|---|
| `ServoReady` | 1 | ready for motion — **OK** |
| `TPMode` | 0 | pendant disabled, required by RMI — **OK** |
| `RMIMotionStatus` | 0 | the RMI **interface** is NOT running; `FRC_Initialize` is allowed |
| `ProgramStatus` | 0 | **0 = RMI_MOVE program is Running** (1 = Paused, 2 = Aborted) |

Those last two disagree, and that disagreement is the bug: **the RMI interface
is down while the RMI_MOVE TP program is still running.** `FRC_Initialize` then
tries to create RMI_MOVE, finds it already there, and returns
`7015 MEMO-015 Program already exists`.

§2.3.1 states the consequence explicitly:

> If the FRC_Initialize command is executed successfully, you will have to send
> an FRC_Abort command to terminate the RMI program in order for another TP
> program to run. Otherwise, **even if you manually abort the RMI_MOVE TP
> program through the teach pendant, the RMI still has program control** and
> you will not be able to execute other TP programs.

That is why FCTN → ABORT (ALL) on the pendant did nothing. And §2.3.2:

> Please always end your RMI session with either an FRC_Abort or FRC_Disconnect
> packet. This will ensure you can execute other TP programs after the RMI
> session.

A prior session initialized successfully and then died without sending either —
consistent with the leaked sockets found earlier (one meteorite process holding
three connections to `:16002`). The orphaned program kept control.

The deadlock is real: `FRC_Initialize` fails because the program exists, and
`FRC_Abort` fails (`RMIT-014 RMI Command Fail`) because `RMIMotionStatus == 0`
means this session has no running RMI to abort.

### Two annotations in this repo hid this

- `program_status` was annotated `1 = aborted`. The manual says **0 = Running**.
  So the healthy-looking `0` actually meant "RMI_MOVE is running" the entire
  time.
- `tp_mode` was annotated `1 = AUTO mode, 0 = manual`. The manual says
  **0 = pendant disabled**, which is what RMI requires.

Both are fixed. FANUC's documentation was correct throughout; ours was not.

## Second, unrelated defect found: `FRC_Abort` wedges the session

On the first run `FRC_Abort` returned nothing within 5 s. Every command sent
while it was outstanding then failed with `RMIT-027 Wait for Command Done`, and
`FRC_GetStatus` came back with corrupt payload fields (`ProgramStatus: -4`,
`NumberUFrame: -6`, `UI[8]: -53`). The session self-healed only once the late
abort response finally arrived.

This is almost certainly the "everything is timing out" state seen after using
ABORT on the pendant. Two consequences:

1. **Never send another RMI command while an abort is outstanding.** Anything
   that does will read a wedged controller and may surface garbage field values
   as if they were real.
2. `FRC_Abort` needs a much longer response budget than an ordinary command.
   With a 60 s budget it answered every time (`RMIT-014 RMI Command Fail`,
   expected — there was nothing running to abort).

## Error-id decoding worth fixing elsewhere

`2556943` is **`RMIT-015 Invalid Controller State`**. Meteorite currently
surfaces this id as `(ConnectToRobot)`, which is wrong and sent this
investigation down the wrong path initially.

## Next action (pendant, not software)

Disable the teach pendant (enable switch off) with the mode selector in AUTO,
then re-run the diagnostic and confirm `TPMode` reads `1` before retrying
Initialize.
