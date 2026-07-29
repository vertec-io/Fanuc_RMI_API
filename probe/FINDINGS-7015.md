# `FRC_Initialize` → 7015 on COMET1 (192.168.0.100)

Live diagnosis, 2026-07-29, controller RMI v9.0. Run with:

```
cargo run -p fanuc_probe --bin init_diag -- --addr 192.168.0.100:16001
```

## What is proven

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

## Every documented precondition PASSES

Healthy status is stable and byte-identical across every run, including after
toggling the TP enable switch on (fault), off, and resetting:

```json
{"ErrorID":0,"ServoReady":1,"TPMode":0,"RMIMotionStatus":0,"ProgramStatus":0,
 "SingleStepMode":0,"NumberUTool":10,"NumberUFrame":9,"Override":10,
 "UI[2]":1,"UI[8]":1}
```

Checked against `FanucDriver::startup_sequence`, which is the authority here
because it cites FANUC B-84184EN/02 directly:

- `ServoReady: 1` — required 1. **PASS**
- `TPMode: 0` — B-84184EN/02: *0 = teach pendant disabled, 1 = enabled*, and
  RMI works only while the pendant is **disabled**. So 0 is exactly right.
  **PASS**
- `RMIMotionStatus: 0` — 0 means "can initialize", which also rules out the
  "RMI_MOVE is still running" theory. **PASS**

So the controller reports itself fully ready and still refuses Initialize.
That is the actual finding: **7015 here is not explained by any precondition
the manual documents.**

> Correction to an earlier draft of this file, which named `TPMode: 0` as a
> violated precondition. That came from two wrong annotations in this repo —
> `frc_getstatus.rs` said "1 = AUTO mode, 0 = manual" and
> `FANUC_INITIALIZATION_SEQUENCE.md` said "should be 1". Both contradicted the
> driver's own manual-sourced check (`tp_mode != 0`). Both are now fixed.
> FANUC's docs were never wrong; ours were.

`NumberUTool: 10` / `NumberUFrame: 9` are **slot counts** available on the
controller, not the active tool/frame numbers.

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
