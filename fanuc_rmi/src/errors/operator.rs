//! Operator-facing faults: a machine-readable cause plus the physical steps to
//! clear it.
//!
//! An RMI error id on its own is useless to the person standing at the cell.
//! Some failures cannot be cleared over the wire at all — they need someone to
//! act on the teach pendant — so the recovery procedure is authored **here**,
//! once, and every consumer of this crate gets the same instructions rather
//! than re-deriving them (or, as happened on COMET1, guessing wrong for a week).

use serde::{Deserialize, Serialize};

/// A physical action an operator must take at the controller.
///
/// UIs should match on this to present a dedicated dialog: it is stable and
/// machine-readable, unlike the message text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperatorAction {
    /// The controller holds an RMI_MOVE program that must be removed before a
    /// new RMI session can be created.
    DeleteRmiMoveProgram,
    /// RMI_MOVE is currently selected on the pendant and must be deselected.
    DeselectRmiMoveProgram,
    /// The teach pendant enable switch is on; RMI needs it off.
    DisableTeachPendant,
    /// Servo or controller faults are present and must be cleared.
    ClearControllerFault,
}

impl OperatorAction {
    /// Short title for a dialog header.
    pub fn title(self) -> &'static str {
        match self {
            Self::DeleteRmiMoveProgram => "Delete the RMI_MOVE program",
            Self::DeselectRmiMoveProgram => "Select a different program",
            Self::DisableTeachPendant => "Disable the teach pendant",
            Self::ClearControllerFault => "Clear the controller fault",
        }
    }

    /// One line naming the underlying condition, for the dialog body.
    pub fn cause(self) -> &'static str {
        match self {
            Self::DeleteRmiMoveProgram => {
                "A previous connection ended without releasing the robot, so its motion program \
                 still has control of the controller. The robot cannot accept a new connection \
                 until that program is removed. Aborting from the teach pendant does not \
                 release it."
            }
            Self::DeselectRmiMoveProgram => {
                "The robot's remote motion program is currently selected on the teach pendant, \
                 which blocks a new connection."
            }
            Self::DisableTeachPendant => {
                "The teach pendant is switched on. The robot only accepts remote motion while \
                 the pendant is off."
            }
            Self::ClearControllerFault => {
                "The controller is reporting a fault and is not ready for motion."
            }
        }
    }

    /// Numbered steps to perform at the pendant. Rendered as an ordered list.
    pub fn steps(self) -> &'static [&'static str] {
        match self {
            Self::DeleteRmiMoveProgram => &[
                "Press SELECT on the teach pendant to open the program list.",
                "Highlight RMI_MOVE.",
                "Press DELETE (a soft button along the bottom of the screen) and confirm.",
                "Reconnect. The robot recreates the program automatically.",
            ],
            Self::DeselectRmiMoveProgram => &[
                "Press SELECT on the teach pendant to open the program list.",
                "Highlight any program other than RMI_MOVE.",
                "Press ENTER to select it.",
                "Reconnect.",
            ],
            Self::DisableTeachPendant => &[
                "Set the teach pendant enable switch to OFF.",
                "Confirm the controller is in AUTO.",
                "Reconnect.",
            ],
            Self::ClearControllerFault => &[
                "Read the alarm on the teach pendant screen and resolve its cause.",
                "Press RESET on the teach pendant.",
                "Reconnect.",
            ],
        }
    }
}

/// A controller failure with its decoded id and, where one exists, the operator
/// procedure that clears it.
///
/// Implements `Display` and converts into `String`, so callers that only want a
/// message keep working unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmiFault {
    /// Raw `ErrorID` from the controller. `0` when the failure was local
    /// (a timeout or transport error) rather than reported by the robot.
    pub error_id: u32,
    /// Decoded id, e.g. `"MEMO-015 Program already exists (7015)"`.
    pub summary: String,
    /// What the operator must physically do, when that is known.
    pub action: Option<OperatorAction>,
}

impl RmiFault {
    /// Build from a controller error id, attaching operator guidance when the
    /// id has a known physical remedy.
    pub fn from_error_id(error_id: u32) -> Self {
        Self {
            error_id,
            summary: crate::format_error_id(error_id),
            action: action_for(error_id),
        }
    }

    /// A failure with no controller error id (timeout, transport, local check).
    pub fn local(message: impl Into<String>) -> Self {
        Self { error_id: 0, summary: message.into(), action: None }
    }

    /// Attach an operator action to a locally-detected failure.
    pub fn with_action(mut self, action: OperatorAction) -> Self {
        self.action = Some(action);
        self
    }

    /// True when this needs someone at the cell — the signal a UI should use to
    /// escalate from a toast to a dialog.
    pub fn needs_operator(&self) -> bool {
        self.action.is_some()
    }

    /// Full message including the numbered steps, for a log line where no
    /// dialog is available.
    pub fn full_text(&self) -> String {
        match self.action {
            None => self.summary.clone(),
            Some(action) => {
                let steps = action
                    .steps()
                    .iter()
                    .enumerate()
                    .map(|(i, s)| format!("  {}. {s}", i + 1))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!("{}\n{}\n{}\n{steps}", self.summary, action.cause(), action.title())
            }
        }
    }
}

/// Map a controller error id to the physical action that clears it.
fn action_for(error_id: u32) -> Option<OperatorAction> {
    match error_id {
        // MEMO-015 "Program already exists". The manual (B-84184EN/03 §2.3.1)
        // attributes this to RMI_MOVE being selected on the pendant, and that is
        // worth trying first — but it is not the only cause. Verified on an
        // R-30iB: it also occurs with RMI_MOVE deselected, after a cold reboot,
        // surviving FRC_Abort and FRC_Reset, when a previous session died
        // without releasing the program. Deleting RMI_MOVE was the fix, so that
        // is the action named here.
        7015 => Some(OperatorAction::DeleteRmiMoveProgram),
        // MEMO-004 "Specific program is in use" — deselecting is the remedy.
        7004 => Some(OperatorAction::DeselectRmiMoveProgram),
        _ => None,
    }
}

impl std::fmt::Display for RmiFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.summary)
    }
}

impl std::error::Error for RmiFault {}

/// Lets `?` lift the driver's existing `Result<_, String>` helpers (get_status,
/// abort, initialize) into a fault. These are local/transport failures, so they
/// carry no controller error id and no operator action.
impl From<String> for RmiFault {
    fn from(message: String) -> Self {
        Self::local(message)
    }
}

/// Lets existing `-> Result<_, String>` callers keep using `?` unchanged.
impl From<RmiFault> for String {
    fn from(fault: RmiFault) -> String {
        fault.full_text()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_already_exists_asks_for_a_delete() {
        let fault = RmiFault::from_error_id(7015);
        assert!(fault.needs_operator());
        assert_eq!(fault.action, Some(OperatorAction::DeleteRmiMoveProgram));
        assert!(fault.summary.contains("MEMO-015"));
    }

    #[test]
    fn an_unremarkable_error_carries_no_operator_action() {
        let fault = RmiFault::from_error_id(2556943);
        assert!(!fault.needs_operator());
        assert!(fault.summary.contains("RMIT-015"));
    }

    /// The String conversion must carry the steps, since a caller that only
    /// logs the message is exactly the case where no dialog will be shown.
    #[test]
    fn string_conversion_includes_the_recovery_steps() {
        let text: String = RmiFault::from_error_id(7015).into();
        assert!(text.contains("SELECT"));
        assert!(text.contains("RMI_MOVE"));
        assert!(text.contains("recreates the program"));
    }
}
