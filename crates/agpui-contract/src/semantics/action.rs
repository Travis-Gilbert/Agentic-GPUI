//! What an agent asks a surface to do, and every named reason it is refused.
//!
//! New in SPEC-AGPUI-SEMANTIC-TREE-1.0 D7. Nothing here is ported.
//!
//! These types are the wire contract for both paths a head reaches a surface
//! by: the in-process call from the agent minted inside the Theorem binary,
//! and the AG-UI frames an external head sends. `theorem-surface-contracts`
//! re-exports them so both carry one struct.

use serde::{Deserialize, Serialize};

use super::role::Role;

/// The seven things an agent can ask of a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SemanticGesture {
    /// Press it, as a pointer would.
    Activate,
    /// Move the keyboard to it.
    Focus,
    /// Replace an editable control's text.
    SetValue(String),
    /// Activate, and require `checked` to have flipped.
    Toggle,
    /// Activate, and require `selected` to have become true.
    Select,
    /// Activate when `expanded` is not already the requested state, and
    /// require it afterwards.
    Expand(bool),
    /// Ask the host to bring an unmaterialized row into the frame.
    ScrollIntoView,
}

/// One request against one node of one surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticAction {
    /// Minted by the caller and echoed on the receipt.
    pub action_id: String,
    pub surface_id: String,
    /// An [`Ident`](super::ident::Ident).
    pub target: String,
    pub gesture: SemanticGesture,
    /// When set, the action is refused unless the coordinator is still on this
    /// generation. Callers that snapshot, decide, then act use it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_generation: Option<u64>,
}

impl SemanticAction {
    /// The common case: activate a control by id.
    #[must_use]
    pub fn activate(
        action_id: impl Into<String>,
        surface_id: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            action_id: action_id.into(),
            surface_id: surface_id.into(),
            target: target.into(),
            gesture: SemanticGesture::Activate,
            expect_generation: None,
        }
    }

    #[must_use]
    pub fn with_gesture(mut self, gesture: SemanticGesture) -> Self {
        self.gesture = gesture;
        self
    }

    #[must_use]
    pub const fn expecting_generation(mut self, generation: u64) -> Self {
        self.expect_generation = Some(generation);
        self
    }
}

/// Why a gesture was not delivered.
///
/// The set is closed. A dispatcher that cannot name its reason has no business
/// issuing a receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionRefusal {
    /// No node and no reading item published this id.
    TargetAbsent,
    /// The node published zero area, or a later-painted node covers the point
    /// the gesture would have landed on. A person could not click it either.
    TargetNotVisible,
    TargetDisabled,
    /// The id is in `reading_order` and not in `nodes`: the surface knows the
    /// row and has not materialized an element for it.
    TargetNotMaterialized { hint: SemanticGesture },
    GestureUnsupported { role: Role },
    /// The node registered no focus handle, so the keyboard cannot reach it.
    NoFocusHandle,
    GenerationStale { expected: u64, actual: u64 },
    /// The gesture was delivered and the state it promises did not appear.
    ///
    /// `field` is a [`Node`](super::node::Node) field name. It is a `String`
    /// and not the `&'static str` the spec sketches because this type crosses
    /// the AG-UI wire, and a borrowed string has no `Deserialize`.
    PostconditionUnmet { field: String },
    /// `ScrollIntoView` with no host hook installed.
    NoScrollHook,
    /// The action named a surface this dispatcher does not own.
    SurfaceUnknown,
    /// The node exists and belongs to a different published surface than the
    /// one the action named.
    ///
    /// Distinct from [`Self::TargetAbsent`] because the recovery is different:
    /// the id is real and the retry is the same gesture against
    /// `surface_id`, not a re-read of the tree.
    TargetOutsideSurface { surface_id: String },
    /// The window publishes more than one surface root, and the target's
    /// declared parent chain reaches none of them.
    ///
    /// A window's snapshot holds every surface painted into it, so a lookup by
    /// id alone can find a node belonging to another one.
    /// [`Self::TargetOutsideSurface`] catches that when the node says where it
    /// belongs. This is the case where it does not say: with one root
    /// published an unscoped node can only be part of it, so it is allowed
    /// through; with several, "unscoped" means "could be any of them", and the
    /// gesture would activate a control the receipt cannot honestly place
    /// under the surface the action named. The recovery is a declared parent
    /// on the target, not a different action: this is a surface that has not
    /// declared its chain.
    TargetUnscoped,
    /// The dispatcher was built for one window and handed another.
    ///
    /// Every lookup a dispatcher performs -- the snapshot, the target, the
    /// focus handle -- reads the window it was built for, while platform
    /// input is delivered to the window it is given. When those differ the
    /// gesture would land in unrelated UI and the receipt would describe the
    /// other window, so there is no reading of the result that is true. The
    /// recovery is a dispatcher built for the window the caller means. This
    /// is a host bug rather than something the action said wrong, and it is
    /// on the wire because the receipt is the only channel a dispatch has.
    WindowMismatch,
}

/// Whether the frame after the action is the frame the action produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionOutcome {
    Applied,
    Refused(ActionRefusal),
}

impl ActionOutcome {
    #[must_use]
    pub const fn is_applied(&self) -> bool {
        matches!(self, Self::Applied)
    }

    #[must_use]
    pub const fn refusal(&self) -> Option<&ActionRefusal> {
        match self {
            Self::Applied => None,
            Self::Refused(refusal) => Some(refusal),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionOutcome, ActionRefusal, SemanticAction, SemanticGesture};
    use crate::semantics::role::Role;

    #[test]
    fn a_gesture_round_trips_through_json() {
        for gesture in [
            SemanticGesture::Activate,
            SemanticGesture::Focus,
            SemanticGesture::SetValue("draft".into()),
            SemanticGesture::Toggle,
            SemanticGesture::Select,
            SemanticGesture::Expand(true),
            SemanticGesture::ScrollIntoView,
        ] {
            let json = serde_json::to_string(&gesture).expect("serializes");
            let back: SemanticGesture = serde_json::from_str(&json).expect("deserializes");
            assert_eq!(gesture, back, "{json}");
        }
    }

    #[test]
    fn a_refusal_round_trips_through_json() {
        for refusal in [
            ActionRefusal::TargetAbsent,
            ActionRefusal::TargetNotVisible,
            ActionRefusal::TargetDisabled,
            ActionRefusal::TargetNotMaterialized {
                hint: SemanticGesture::ScrollIntoView,
            },
            ActionRefusal::GestureUnsupported { role: Role::Button },
            ActionRefusal::NoFocusHandle,
            ActionRefusal::GenerationStale {
                expected: 3,
                actual: 4,
            },
            ActionRefusal::PostconditionUnmet {
                field: "checked".into(),
            },
            ActionRefusal::NoScrollHook,
            ActionRefusal::SurfaceUnknown,
            ActionRefusal::TargetOutsideSurface {
                surface_id: "thread".into(),
            },
            ActionRefusal::TargetUnscoped,
            ActionRefusal::WindowMismatch,
        ] {
            let json = serde_json::to_string(&ActionOutcome::Refused(refusal.clone()))
                .expect("serializes");
            let back: ActionOutcome = serde_json::from_str(&json).expect("deserializes");
            assert_eq!(back.refusal(), Some(&refusal), "{json}");
        }
    }

    #[test]
    fn the_wire_names_are_kebab_case() {
        assert_eq!(
            serde_json::to_string(&SemanticGesture::ScrollIntoView).expect("serializes"),
            "\"scroll-into-view\""
        );
        assert_eq!(
            serde_json::to_string(&ActionOutcome::Refused(ActionRefusal::TargetDisabled))
                .expect("serializes"),
            "{\"refused\":\"target-disabled\"}"
        );
    }

    #[test]
    fn an_action_omits_an_absent_generation_expectation() {
        let action = SemanticAction::activate("a1", "composer", "composer-send");
        assert_eq!(
            serde_json::to_string(&action).expect("serializes"),
            "{\"action_id\":\"a1\",\"surface_id\":\"composer\",\"target\":\"composer-send\",\"gesture\":\"activate\"}"
        );
        assert_eq!(action.expecting_generation(7).expect_generation, Some(7));
    }
}
