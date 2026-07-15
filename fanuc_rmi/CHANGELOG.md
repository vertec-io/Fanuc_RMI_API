# Changelog — `fanuc_rmi`

All notable changes to the `fanuc_rmi` library crate are documented here.
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.0] - 2026-07-14

Reshape the motion layer to the **real RMI multi-group wire format** documented
in the FANUC RMI Operators Manual (B-84184EN/03, §1.4.1 and §2.4.7.1). A single
motion instruction packet can now drive more than one motion group — e.g. a
robot arm as Group 1 and a coordinated positioner as Group 2 — which the prior
scalar `group: Option<GroupMask>` field could not express and never actually
produced on the wire.

### Breaking

- **Motion-instruction payload reshape.** Every motion instruction
  (`FrcLinearMotion`, `FrcLinearRelative`, `FrcJointMotion`, `FrcJointRelative`,
  `FrcCircularMotion`, `FrcCircularRelative`, `FrcJointMotionJRep`,
  `FrcJointRelativeJRep`, `FrcLinearMotionJRep`, `FrcLinearRelativeJRep`) had its
  inline `configuration`/`position` (or `joint_angles`) fields plus the incorrect
  `group: Option<GroupMask>` field replaced by a single `#[serde(flatten)] groups`
  field:
  - Cartesian-native instructions carry `groups: CartesianGroups`.
  - Joint-representation (JRep) instructions carry `groups: JointGroups`.
  - Circular instructions carry `groups: CircularGroups` (each group has a
    destination **and** a via: `Configuration`/`Position` +
    `ViaConfiguration`/`ViaPosition`).

  `CartesianGroups`/`JointGroups`/`CircularGroups` each serialize to the **flat**
  single-group form (data at the top level) or the **wrapped** multi-group form
  (`"G1": {…}, "G2": {…}` + top-level `"COORD"`), matching the manual exactly.

- **`GroupMask` removed from motion packets.** `GroupMask` belongs on
  `FRC_Initialize` (§2.3.1) only; motion packets never carried it. The old
  `.with_group(GroupMask)` builder and `group` field are gone.

- **`Instruction` / `InstructionResponse` are now `#[non_exhaustive]`** and gained
  `FrcSplineMotion` / `FrcSplineMotionJRep` variants (see below). Downstream
  exhaustive `match`es must add a wildcard arm.

- **Full canonical optional-key set.** Each motion instruction now exposes its
  documented optional keys (`COORD`, `ACC`, `OffsetPRNumber`, `VisionPRNumber`,
  `WristJoint`, `MROT`, `ALIM`/`ALIMREG`, `LCBType`/`LCBValue`, `PortType`/
  `PortNumber`/`PortValue`, `ToolOffsetPRNumber`, `NoBlend`), each `Option` and
  omitted from the wire when `None`. Joint and circular instructions omit
  `ALIM`/`ALIMREG`; spline instructions carry only the reduced set the manual
  documents (no `COORD`/`WristJoint`/`MROT`/`ALIM`/`NoBlend`).

### Added

- **`FrcSplineMotion` / `FrcSplineMotionJRep`** (§2.4.17 / §2.4.18), including
  their two-group forms and response types.
- **Multi-group payload types** in `fanuc_rmi::instructions` (and DTO mirrors in
  `fanuc_rmi::dto`): `GroupBlock` (`Cartesian` | `Joint`), `CartesianGroups`,
  `JointGroups`, `CircGroupBlock`, `CircularGroups`, with constructors
  `single(...)`, `multi(...)`, and `arm_and_group2(...)`.
- **Convenience constructors** on each motion instruction: `single(...)`
  (flat single-group; `new(...)` is a signature-compatible alias for the common
  Cartesian/JRep cases), `with_groups(...)` (explicit payload), and
  `coordinated(...)` (arm + Group 2, sets `COORD=ON`; spline uses `two_group(...)`
  with no `COORD`).

### Fixed

- **`JointAngle` naming (lenient-in / correct-out).** JRep and joint-group
  serialization now emit the singular `"JointAngle"` key the real controller and
  the manual use; deserialization still accepts the legacy/simulator plural
  `"JointAngles"`. Previously JRep instructions emitted the plural form.

### Migration

Replace direct struct literals with the constructors. The simplest port keeps
single-group behavior identical:

```rust
// before (0.5)
FrcLinearMotion { sequence_id, configuration, position, speed_type, speed,
                  term_type, term_value, group: None }

// after (0.6)
FrcLinearMotion::single(sequence_id, configuration, position, speed_type, speed,
                        term_type, term_value)
```

For a coordinated arm + Group-2 positioner move:

```rust
FrcLinearMotion::coordinated(
    sequence_id, arm_configuration, arm_position,
    GroupBlock::joint(JointAngles { j1: 45.0, j2: 10.0, ..Default::default() }),
    speed_type, speed, term_type, term_value,
) // serializes G1 (arm) + G2 (positioner) + COORD=ON
```

Building the binary DTO from a protocol instruction still works via `.into()`:
`let dto: FrcLinearMotionDto = instr.into();`.
