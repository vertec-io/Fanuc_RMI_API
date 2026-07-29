use super::Packet;
use crate::instructions::*;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "Instruction")]
#[non_exhaustive]
pub enum Instruction {
    #[serde(rename = "FRC_WaitDIN")]
    FrcWaitDIN(FrcWaitDIN), // Wait for DIN Instruction

    #[serde(rename = "FRC_SetUFrame")]
    FrcSetUFrame(FrcSetUFrame), // Set User Frame Instruction

    #[serde(rename = "FRC_SetUTool")]
    FrcSetUTool(FrcSetUTool), // Set User Tool Instruction

    #[serde(rename = "FRC_WaitTime")]
    FrcWaitTime(FrcWaitTime), // Add Wait Time Instruction

    #[serde(rename = "FRC_SetPayLoad")]
    FrcSetPayLoad(FrcSetPayLoad), // Set Payload Instruction

    #[serde(rename = "FRC_Call")]
    FrcCall(FrcCall), // Call a Program

    #[serde(rename = "FRC_LinearMotion")]
    FrcLinearMotion(FrcLinearMotion), // Add Linear Motion Instruction

    #[serde(rename = "FRC_LinearRelative")]
    FrcLinearRelative(FrcLinearRelative), // Add Linear Incremental Motion Instruction

    #[serde(rename = "FRC_LinearRelativeJRep")]
    FrcLinearRelativeJRep(FrcLinearRelativeJRep), // Add Linear Relative Motion with Joint Representation

    #[serde(rename = "FRC_JointMotion")]
    FrcJointMotion(FrcJointMotion), // Add Joint Motion Instruction

    #[serde(rename = "FRC_JointRelative")]
    FrcJointRelative(FrcJointRelative), // Add Joint Incremental Motion Instruction

    #[serde(rename = "FRC_CircularMotion")]
    FrcCircularMotion(FrcCircularMotion), // Add Circular Motion Instruction

    #[serde(rename = "FRC_CircularRelative")]
    FrcCircularRelative(FrcCircularRelative), // Add Circular Incremental Motion Instruction

    #[serde(rename = "FRC_JointMotionJRep")]
    FrcJointMotionJRep(FrcJointMotionJRep), // Add Joint Motion with Joint Representation

    #[serde(rename = "FRC_JointRelativeJRep")]
    FrcJointRelativeJRep(FrcJointRelativeJRep), // Add Joint Incremental Motion with Joint Representation

    #[serde(rename = "FRC_LinearMotionJRep")]
    FrcLinearMotionJRep(FrcLinearMotionJRep), // Add Linear Motion with Joint Representation

    #[serde(rename = "FRC_SplineMotion")]
    FrcSplineMotion(FrcSplineMotion), // Add Spline Motion Instruction

    #[serde(rename = "FRC_SplineMotionJRep")]
    FrcSplineMotionJRep(FrcSplineMotionJRep), // Add Spline Motion with Joint Representation
}

impl Instruction {
    pub fn get_sequence_id(&self) -> u32 {
        match self {
            Instruction::FrcWaitDIN(resp) => resp.sequence_id,
            Instruction::FrcSetUFrame(resp) => resp.sequence_id,
            Instruction::FrcSetUTool(resp) => resp.sequence_id,
            Instruction::FrcWaitTime(resp) => resp.sequence_id,
            Instruction::FrcSetPayLoad(resp) => resp.sequence_id,
            Instruction::FrcCall(resp) => resp.sequence_id,
            Instruction::FrcLinearMotion(resp) => resp.sequence_id,
            Instruction::FrcLinearRelative(resp) => resp.sequence_id,
            Instruction::FrcLinearRelativeJRep(resp) => resp.sequence_id,
            Instruction::FrcJointMotion(resp) => resp.sequence_id,
            Instruction::FrcJointRelative(resp) => resp.sequence_id,
            Instruction::FrcCircularMotion(resp) => resp.sequence_id,
            Instruction::FrcCircularRelative(resp) => resp.sequence_id,
            Instruction::FrcJointMotionJRep(resp) => resp.sequence_id,
            Instruction::FrcJointRelativeJRep(resp) => resp.sequence_id,
            Instruction::FrcLinearMotionJRep(resp) => resp.sequence_id,
            Instruction::FrcSplineMotion(resp) => resp.sequence_id,
            Instruction::FrcSplineMotionJRep(resp) => resp.sequence_id,
        }
    }

    /// The speed this instruction commands, and whether it is a **joint** move.
    ///
    /// The two motion families take different, non-overlapping speed notations,
    /// so this pairing is what [`check_speed_type`](Self::check_speed_type)
    /// needs. `None` for instructions that command no motion.
    fn speed_spec(&self) -> Option<(crate::SpeedType, bool)> {
        // `true` = joint motion (percentage speed); `false` = a Cartesian path
        // (rate speed). Note the JRep *linear* forms are Cartesian paths merely
        // expressed in joint coordinates — B-84184EN/03 §2.4.15 shows
        // `FRC_LinearMotionJRep` producing `L P[10] 150mm/sec`, so they take a
        // rate, not a percentage.
        match self {
            Instruction::FrcJointMotion(i) => Some((i.speed_type, true)),
            Instruction::FrcJointRelative(i) => Some((i.speed_type, true)),
            Instruction::FrcJointMotionJRep(i) => Some((i.speed_type, true)),
            Instruction::FrcJointRelativeJRep(i) => Some((i.speed_type, true)),

            Instruction::FrcLinearMotion(i) => Some((i.speed_type, false)),
            Instruction::FrcLinearRelative(i) => Some((i.speed_type, false)),
            Instruction::FrcLinearMotionJRep(i) => Some((i.speed_type, false)),
            Instruction::FrcLinearRelativeJRep(i) => Some((i.speed_type, false)),
            Instruction::FrcCircularMotion(i) => Some((i.speed_type, false)),
            Instruction::FrcCircularRelative(i) => Some((i.speed_type, false)),
            Instruction::FrcSplineMotion(i) => Some((i.speed_type, false)),
            Instruction::FrcSplineMotionJRep(i) => Some((i.speed_type, false)),

            Instruction::FrcWaitDIN(_)
            | Instruction::FrcSetUFrame(_)
            | Instruction::FrcSetUTool(_)
            | Instruction::FrcWaitTime(_)
            | Instruction::FrcSetPayLoad(_)
            | Instruction::FrcCall(_) => None,
        }
    }

    /// Reject a speed notation the controller will not accept for this motion
    /// type. Returns the reason, or `None` when the pairing is valid.
    ///
    /// Joint motion takes a **percentage** of maximum speed; a Cartesian path
    /// takes a **rate**. They do not overlap, and the controller's only feedback
    /// for getting it wrong is `RMIT-030 Invalid Speed Type` after the fact
    /// (B-84184EN/03 §2.4.7 and §2.4.9; both confirmed live on an R-30iB).
    ///
    /// This is easy to get wrong precisely because `SpeedType` is one enum
    /// shared by every instruction, so nothing about the type signature hints
    /// that `MMSec` is meaningless on a joint move.
    pub fn check_speed_type(&self) -> Option<String> {
        use crate::SpeedType::*;
        let (speed_type, is_joint) = self.speed_spec()?;
        let name = self.instruction_name();
        match (is_joint, speed_type) {
            // Time-based notations are accepted by both families.
            (_, Time) | (_, MilliSeconds) => None,
            (true, Percent) => None,
            (false, MMSec) | (false, InchMin) => None,
            (true, bad) => Some(format!(
                "{name} is a JOINT motion, which takes a percentage of maximum speed — \
                 `SpeedType::{bad:?}` is a rate and the controller rejects it with \
                 RMIT-030 Invalid Speed Type. Use `SpeedType::Percent` with a value of 1-100 \
                 (a joint move's speed is per-axis, not a tool-tip rate), or `Time`/`mSec`."
            )),
            (false, bad) => Some(format!(
                "{name} follows a Cartesian path, which takes a rate — \
                 `SpeedType::{bad:?}` is a percentage and the controller rejects it with \
                 RMIT-030 Invalid Speed Type. Use `SpeedType::MMSec` (or `InchMin`, \
                 `Time`, `mSec`)."
            )),
        }
    }

    /// The wire name of this instruction, for messages.
    pub fn instruction_name(&self) -> &'static str {
        match self {
            Instruction::FrcWaitDIN(_) => "FRC_WaitDIN",
            Instruction::FrcSetUFrame(_) => "FRC_SetUFrame",
            Instruction::FrcSetUTool(_) => "FRC_SetUTool",
            Instruction::FrcWaitTime(_) => "FRC_WaitTime",
            Instruction::FrcSetPayLoad(_) => "FRC_SetPayLoad",
            Instruction::FrcCall(_) => "FRC_Call",
            Instruction::FrcLinearMotion(_) => "FRC_LinearMotion",
            Instruction::FrcLinearRelative(_) => "FRC_LinearRelative",
            Instruction::FrcLinearRelativeJRep(_) => "FRC_LinearRelativeJRep",
            Instruction::FrcJointMotion(_) => "FRC_JointMotion",
            Instruction::FrcJointRelative(_) => "FRC_JointRelative",
            Instruction::FrcCircularMotion(_) => "FRC_CircularMotion",
            Instruction::FrcCircularRelative(_) => "FRC_CircularRelative",
            Instruction::FrcJointMotionJRep(_) => "FRC_JointMotionJRep",
            Instruction::FrcJointRelativeJRep(_) => "FRC_JointRelativeJRep",
            Instruction::FrcLinearMotionJRep(_) => "FRC_LinearMotionJRep",
            Instruction::FrcSplineMotion(_) => "FRC_SplineMotion",
            Instruction::FrcSplineMotionJRep(_) => "FRC_SplineMotionJRep",
        }
    }

    /// The `GroupMask` this instruction's position payload actually carries, or
    /// `None` for instructions that command no motion group at all (waits,
    /// frame/tool/payload setters, program calls).
    ///
    /// The controller requires this to equal the session mask established by
    /// `FRC_Initialize`: with two bits set, **every** motion packet must carry
    /// two sets of position data or it is rejected with `RMIT-040 Invalid Group
    /// Mask` (B-84184EN/03 §2.3.1). A single-group payload under a two-group
    /// session is the classic form of that mistake — it looks correct in
    /// isolation and only the controller says otherwise.
    pub fn motion_group_mask(&self) -> Option<u8> {
        match self {
            // No position payload — group-independent, never validated.
            Instruction::FrcWaitDIN(_)
            | Instruction::FrcSetUFrame(_)
            | Instruction::FrcSetUTool(_)
            | Instruction::FrcWaitTime(_)
            | Instruction::FrcSetPayLoad(_)
            | Instruction::FrcCall(_) => None,

            Instruction::FrcLinearMotion(i) => Some(i.groups.group_mask()),
            Instruction::FrcLinearRelative(i) => Some(i.groups.group_mask()),
            Instruction::FrcJointMotion(i) => Some(i.groups.group_mask()),
            Instruction::FrcJointRelative(i) => Some(i.groups.group_mask()),
            Instruction::FrcSplineMotion(i) => Some(i.groups.group_mask()),

            Instruction::FrcLinearRelativeJRep(i) => Some(i.groups.group_mask()),
            Instruction::FrcJointMotionJRep(i) => Some(i.groups.group_mask()),
            Instruction::FrcJointRelativeJRep(i) => Some(i.groups.group_mask()),
            Instruction::FrcLinearMotionJRep(i) => Some(i.groups.group_mask()),
            Instruction::FrcSplineMotionJRep(i) => Some(i.groups.group_mask()),

            Instruction::FrcCircularMotion(i) => Some(i.groups.group_mask()),
            Instruction::FrcCircularRelative(i) => Some(i.groups.group_mask()),
        }
    }
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "Instruction")]
#[non_exhaustive]
pub enum InstructionResponse {
    #[serde(rename = "FRC_WaitDIN")]
    FrcWaitDIN(FrcWaitDINResponse),
    #[serde(rename = "FRC_SetUFrame")]
    FrcSetUFrame(FrcSetUFrameResponse),
    #[serde(rename = "FRC_SetUTool")]
    FrcSetUTool(FrcSetUToolResponse),
    #[serde(rename = "FRC_WaitTime")]
    FrcWaitTime(FrcWaitTimeResponse),
    #[serde(rename = "FRC_SetPayLoad")]
    FrcSetPayLoad(FrcSetPayLoadResponse),
    #[serde(rename = "FRC_Call")]
    FrcCall(FrcCallResponse),
    #[serde(rename = "FRC_LinearMotion")]
    FrcLinearMotion(FrcLinearMotionResponse),
    #[serde(rename = "FRC_LinearRelative")]
    FrcLinearRelative(FrcLinearRelativeResponse),
    #[serde(rename = "FRC_LinearRelativeJRep")]
    FrcLinearRelativeJRep(FrcLinearRelativeJRepResponse),
    #[serde(rename = "FRC_JointMotion")]
    FrcJointMotion(FrcJointMotionResponse),
    #[serde(rename = "FRC_JointRelative")]
    FrcJointRelative(FrcJointRelativeResponse),
    #[serde(rename = "FRC_CircularMotion")]
    FrcCircularMotion(FrcCircularMotionResponse),
    #[serde(rename = "FRC_CircularRelative")]
    FrcCircularRelative(FrcCircularRelativeResponse),
    #[serde(rename = "FRC_JointMotionJRep")]
    FrcJointMotionJRep(FrcJointMotionJRepResponse),
    #[serde(rename = "FRC_JointRelativeJRep")]
    FrcJointRelativeJRep(FrcJointRelativeJRepResponse),
    #[serde(rename = "FRC_LinearMotionJRep")]
    FrcLinearMotionJRep(FrcLinearMotionJRepResponse),
    #[serde(rename = "FRC_SplineMotion")]
    FrcSplineMotion(FrcSplineMotionResponse),
    #[serde(rename = "FRC_SplineMotionJRep")]
    FrcSplineMotionJRep(FrcSplineMotionJRepResponse),
}

impl InstructionResponse {
    pub fn get_sequence_id(&self) -> u32 {
        match self {
            InstructionResponse::FrcWaitDIN(resp) => resp.sequence_id,
            InstructionResponse::FrcSetUFrame(resp) => resp.sequence_id,
            InstructionResponse::FrcSetUTool(resp) => resp.sequence_id,
            InstructionResponse::FrcWaitTime(resp) => resp.sequence_id,
            InstructionResponse::FrcSetPayLoad(resp) => resp.sequence_id,
            InstructionResponse::FrcCall(resp) => resp.sequence_id,
            InstructionResponse::FrcLinearMotion(resp) => resp.sequence_id,
            InstructionResponse::FrcLinearRelative(resp) => resp.sequence_id,
            InstructionResponse::FrcLinearRelativeJRep(resp) => resp.sequence_id,
            InstructionResponse::FrcJointMotion(resp) => resp.sequence_id,
            InstructionResponse::FrcJointRelative(resp) => resp.sequence_id,
            InstructionResponse::FrcCircularMotion(resp) => resp.sequence_id,
            InstructionResponse::FrcCircularRelative(resp) => resp.sequence_id,
            InstructionResponse::FrcJointMotionJRep(resp) => resp.sequence_id,
            InstructionResponse::FrcJointRelativeJRep(resp) => resp.sequence_id,
            InstructionResponse::FrcLinearMotionJRep(resp) => resp.sequence_id,
            InstructionResponse::FrcSplineMotion(resp) => resp.sequence_id,
            InstructionResponse::FrcSplineMotionJRep(resp) => resp.sequence_id,
        }
    }
}
impl InstructionResponse {
    pub fn get_error_id(&self) -> u32 {
        match self {
            InstructionResponse::FrcWaitDIN(resp) => resp.error_id,
            InstructionResponse::FrcSetUFrame(resp) => resp.error_id,
            InstructionResponse::FrcSetUTool(resp) => resp.error_id,
            InstructionResponse::FrcWaitTime(resp) => resp.error_id,
            InstructionResponse::FrcSetPayLoad(resp) => resp.error_id,
            InstructionResponse::FrcCall(resp) => resp.error_id,
            InstructionResponse::FrcLinearMotion(resp) => resp.error_id,
            InstructionResponse::FrcLinearRelative(resp) => resp.error_id,
            InstructionResponse::FrcLinearRelativeJRep(resp) => resp.error_id,
            InstructionResponse::FrcJointMotion(resp) => resp.error_id,
            InstructionResponse::FrcJointRelative(resp) => resp.error_id,
            InstructionResponse::FrcCircularMotion(resp) => resp.error_id,
            InstructionResponse::FrcCircularRelative(resp) => resp.error_id,
            InstructionResponse::FrcJointMotionJRep(resp) => resp.error_id,
            InstructionResponse::FrcJointRelativeJRep(resp) => resp.error_id,
            InstructionResponse::FrcLinearMotionJRep(resp) => resp.error_id,
            InstructionResponse::FrcSplineMotion(resp) => resp.error_id,
            InstructionResponse::FrcSplineMotionJRep(resp) => resp.error_id,
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletedPacketReturnInfo {
    pub sequence_id: u32,
    pub error_id: u32,
}

/// Information about an instruction that was sent to the FANUC controller
///
/// This struct is broadcast on the `sent_instruction_tx` channel when an instruction
/// is assigned a sequence ID and sent to the controller. It allows callers to correlate
/// their send requests (identified by request_id) with the actual sequence IDs
/// assigned by the driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SentInstructionInfo {
    /// The request ID returned from send_packet()
    pub request_id: u64,
    /// The sequence ID assigned to the instruction by the driver
    pub sequence_id: u32,
    /// When the instruction was sent
    pub timestamp: std::time::Instant,
}

#[cfg_attr(feature = "DTO", crate::mirror_dto)]

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum OnOff {
    ON,
    OFF,
}

impl Packet for Instruction {}

// ExtractInner trait implementations for InstructionResponse
impl_extract_inner!(InstructionResponse, FrcWaitDIN, FrcWaitDINResponse);
impl_extract_inner!(InstructionResponse, FrcSetUFrame, FrcSetUFrameResponse);
impl_extract_inner!(InstructionResponse, FrcSetUTool, FrcSetUToolResponse);
impl_extract_inner!(InstructionResponse, FrcWaitTime, FrcWaitTimeResponse);
impl_extract_inner!(InstructionResponse, FrcSetPayLoad, FrcSetPayLoadResponse);
impl_extract_inner!(InstructionResponse, FrcCall, FrcCallResponse);
impl_extract_inner!(InstructionResponse, FrcLinearMotion, FrcLinearMotionResponse);
impl_extract_inner!(InstructionResponse, FrcLinearRelative, FrcLinearRelativeResponse);
impl_extract_inner!(InstructionResponse, FrcLinearRelativeJRep, FrcLinearRelativeJRepResponse);
impl_extract_inner!(InstructionResponse, FrcJointMotion, FrcJointMotionResponse);
impl_extract_inner!(InstructionResponse, FrcJointRelative, FrcJointRelativeResponse);
impl_extract_inner!(InstructionResponse, FrcCircularMotion, FrcCircularMotionResponse);
impl_extract_inner!(InstructionResponse, FrcCircularRelative, FrcCircularRelativeResponse);
impl_extract_inner!(InstructionResponse, FrcJointMotionJRep, FrcJointMotionJRepResponse);
impl_extract_inner!(InstructionResponse, FrcJointRelativeJRep, FrcJointRelativeJRepResponse);
impl_extract_inner!(InstructionResponse, FrcLinearMotionJRep, FrcLinearMotionJRepResponse);
impl_extract_inner!(InstructionResponse, FrcSplineMotion, FrcSplineMotionResponse);
impl_extract_inner!(InstructionResponse, FrcSplineMotionJRep, FrcSplineMotionJRepResponse);


#[cfg(test)]
mod speed_type_tests {
    use super::*;
    use crate::instructions::{FrcJointRelativeJRep, FrcLinearRelative};
    use crate::{Configuration, JointAngles, Position, SpeedType, TermType};

    fn joint(speed_type: SpeedType) -> Instruction {
        Instruction::FrcJointRelativeJRep(FrcJointRelativeJRep::single(
            1,
            JointAngles::default(),
            speed_type,
            5.0,
            TermType::FINE,
            0,
        ))
    }

    fn linear(speed_type: SpeedType) -> Instruction {
        Instruction::FrcLinearRelative(FrcLinearRelative::single(
            1,
            Configuration::default(),
            Position::default(),
            speed_type,
            5.0,
            TermType::FINE,
            0,
        ))
    }

    /// The exact mistake that cost a live debugging cycle: `mmSec` on a joint
    /// move, which the controller answers with RMIT-030.
    #[test]
    fn joint_motion_rejects_a_rate() {
        let reason = joint(SpeedType::MMSec).check_speed_type().expect("must be refused");
        assert!(reason.contains("percentage"), "{reason}");
        assert!(reason.contains("RMIT-030"), "{reason}");
        assert!(joint(SpeedType::InchMin).check_speed_type().is_some());
    }

    #[test]
    fn joint_motion_accepts_percent_and_the_time_forms() {
        assert!(joint(SpeedType::Percent).check_speed_type().is_none());
        assert!(joint(SpeedType::Time).check_speed_type().is_none());
        assert!(joint(SpeedType::MilliSeconds).check_speed_type().is_none());
    }

    /// The mirror-image mistake, which is just as wrong.
    #[test]
    fn a_cartesian_path_rejects_a_percentage() {
        let reason = linear(SpeedType::Percent).check_speed_type().expect("must be refused");
        assert!(reason.contains("rate"), "{reason}");
        assert!(linear(SpeedType::MMSec).check_speed_type().is_none());
        assert!(linear(SpeedType::InchMin).check_speed_type().is_none());
    }

    /// Instructions that command no motion have no speed to validate, and must
    /// not be refused.
    #[test]
    fn non_motion_instructions_are_not_speed_checked() {
        let wait = Instruction::FrcWaitTime(crate::instructions::FrcWaitTime::new(1, 0.5));
        assert!(wait.check_speed_type().is_none());
    }

    /// `FRC_LinearMotionJRep` is a Cartesian PATH expressed in joint
    /// coordinates (§2.4.15 shows it producing `L P[10] 150mm/sec`), so it takes
    /// a rate. Easy to misfile as joint motion because of the name.
    #[test]
    fn jrep_linear_is_a_cartesian_path_not_a_joint_move() {
        let i = Instruction::FrcLinearMotionJRep(
            crate::instructions::FrcLinearMotionJRep::single(
                1,
                JointAngles::default(),
                SpeedType::MMSec,
                150.0,
                TermType::FINE,
                0,
            ),
        );
        assert!(i.check_speed_type().is_none());
        let i = Instruction::FrcLinearMotionJRep(
            crate::instructions::FrcLinearMotionJRep::single(
                1,
                JointAngles::default(),
                SpeedType::Percent,
                50.0,
                TermType::FINE,
                0,
            ),
        );
        assert!(i.check_speed_type().is_some());
    }
}
