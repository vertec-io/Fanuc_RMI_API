# fanuc_rmi v0.6.1

Released November 17, 2026.

## Driver fixes for Meteorite

- `ClearInFlight` clears both the in-flight counter and the tracked instruction
  bodies. A later program pause/resume cannot replay motion discarded by an abort.
- `FanucDriver::set_requested_group_mask(u8)` selects the groups used by
  `initialize()`, `startup_sequence()`, and recovery reinitialization. The value is
  shared across driver clones and survives aborts. Zero is ignored; Group 1 remains
  the default. `requested_group_mask()` returns this selection.
- `session_group_mask()` continues to report the controller's echoed reservation.
  Changing the requested mask does not change an active session. Explicit
  `initialize_with_mask()` / `send_initialize_with_mask()` calls are one-time
  overrides and do not update the persistent selection.

## Meteorite update

Pin the merged commit for this release in the workspace git dependency and remove
its local `[patch]` override and vendored copy. For arm + positioner sessions:

```rust
let driver = FanucDriver::connect(config).await?;
driver.set_requested_group_mask(0b11);
driver.startup_sequence().await?;
```

## Verification

TCP fake-controller regressions cover discarded-motion suppression, preserved
normal pause/resume replay, default/zero mask behavior, clone sharing, repeated
initialization, abort recovery, pause/resume recovery, and one-time overrides.
`cargo test -p fanuc_rmi --all-features` passes (98 tests, 12 ignored).
Removing each fix independently makes its regression fail; ordinary pause/resume
replay still passes. The broader simulator library check has two existing inverse
kinematics failures (`test_inverse_kinematics_roundtrip` and
`test_multiple_ik_solutions`), reproduced with the original upstream driver.
No physical robot verification was performed.

The library version is 0.6.1; simulator and other workspace package versions are
unchanged. This is a GitHub source release for pinned git dependencies.
