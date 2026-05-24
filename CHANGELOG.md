# Changelog

All notable changes to the `sim` crate in this repository will be documented
in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-05-23

This release lands the COMET1 Simulator System integration work
(PRD `comet1-simulator-system`, US-004a through US-004d). The `sim` binary
is now suitable for use as a long-running FANUC RMI simulator behind the
Meteorite launcher, with configurable networking, full joint-motion support,
flow control under burst loads, an out-of-band I/O stimulus surface, and
two long-standing protocol-completeness fixes.

### Added

- **Configurable bind address and primary port** (US-004a). New CLI flags
  `--bind <ADDR>` (default `0.0.0.0`) and `--port <PORT>` (default `16001`)
  let the simulator listen on a specific interface and primary control port
  instead of the previous hard-coded `0.0.0.0:16001`.
- **`--quiet` flag** (US-004a) suppresses the per-tick informational logging
  on the primary control port so the simulator can run cleanly in the
  background under a launcher.
- **Secondary-port reuse** (US-004a). When a client `FRC_Disconnect`s, the
  secondary motion port it was using is returned to the `PortAllocator` and
  handed back out on the next `FRC_Connect`, instead of leaking ports
  monotonically until `u16` exhaustion.
- **All joint-motion instruction variants** (US-004b). `FRC_JointMotion`,
  `FRC_JointMotionJRep`, and `FRC_JointRelativeJRep` are now dispatched
  through the same motion executor as `FRC_LinearMotion` /
  `FRC_LinearRelative`, with proper sequence-ID validation and motion
  interpolation. Previously only the linear variants were implemented.
- **In-flight instruction cap** (US-004b). A `Semaphore` limits the number
  of concurrent motion instructions per session to 8, providing back-pressure
  against burst clients and matching the real controller's queue depth.
- **HTTP I/O stimulus sidecar** (US-004c). An `axum` 0.8 sidecar (default
  bound on the next port above the primary control port) exposes four
  endpoints for out-of-band test stimulus:
  - `POST /sim/din   { port: u32, value: bool }` — drive a digital input
  - `POST /sim/ain   { port: u32, value: f64 }`  — drive an analog input
  - `POST /sim/gin   { port: u32, value: u32 }`  — drive a group input
  - `POST /sim/fault { error_id: u32 }`          — arm a one-shot fault that
    fires on the next dispatched Command / Instruction / Communication
- **`FRC_ReadError` command** (US-004d). Returns the currently latched
  one-shot fault `error_id` from `RobotState` (or `0` when no fault is
  armed) along with the request's `Count` and an empty `ErrorData` string.
  Previously this command fell through to the `Unknown` arm and returned
  `ErrorID = 2556950` (`InvalidTextString`).
- **`CHANGELOG.md`** (US-004d) — this file.

### Changed

- **Bumped `sim` crate version from `0.1.1` to `0.2.0`** (US-004d). The new
  CLI flags, the joint-motion variants, the I/O sidecar, and the protocol
  completeness fixes together constitute a minor-version-worthy expansion
  of the public surface.

### Fixed

- **`FRC_Connect` now returns `ErrorID = 0` on success** (US-004d). Real
  FANUC controllers return `0` for a successful handshake; the simulator
  previously returned `1`, which tripped strict clients that check
  `ErrorID == 0` for success.

[0.2.0]: https://github.com/raven-space-systems/Fanuc_RMI_API/releases/tag/sim-v0.2.0
