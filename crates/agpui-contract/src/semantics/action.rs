//! What an agent asks a surface to do, and every named reason it is refused.
//!
//! New in SPEC-AGPUI-SEMANTIC-TREE-1.0 D7. Nothing here is ported.
//!
//! These types are the wire contract for both paths a head reaches a surface
//! by: the in-process call from the agent minted inside the Theorem binary,
//! and the AG-UI frames an external head sends. `theorem-surface-contracts`
//! re-exports them so both carry one struct.

use serde::{Deserialize, Deserializer, Serialize};

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
    ///
    /// Never empty. The echo is the only thing tying a receipt to the request
    /// that produced it, and an empty id ties it to every other empty one, so
    /// a wire caller that sends one is refused here and an in-process caller
    /// that constructs one is refused by the dispatcher with
    /// [`ActionRefusal::ActionUnidentified`] before the gesture is delivered.
    #[serde(deserialize_with = "nonempty")]
    pub action_id: String,
    /// Never empty, for the same reason [`Self::action_id`] is not.
    ///
    /// A dispatcher that does not own the named surface answers from one
    /// shared unknown-surface bucket, so an empty one is not refused as
    /// malformed -- it is quietly treated as some other host's surface, and
    /// the caller gets a plausible receipt for a frame nobody rendered.
    #[serde(deserialize_with = "nonempty")]
    pub surface_id: String,
    /// An [`Ident`](super::ident::Ident).
    pub target: String,
    pub gesture: SemanticGesture,
    /// When set, the action is refused unless the coordinator is still on this
    /// generation. Callers that snapshot, decide, then act use it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expect_generation: Option<u64>,
}

/// Rejects an empty string where an identifier is required.
fn nonempty<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.is_empty() {
        return Err(serde::de::Error::custom("an identifier is never empty"));
    }
    Ok(value)
}

impl SemanticAction {
    /// Whether this action carries the two identifiers it is never without.
    ///
    /// Deserialization refuses an empty one of either, so this is the
    /// in-process half: a caller that builds the struct by hand reaches the
    /// dispatcher without passing through serde.
    ///
    /// `surface_id` is checked here and not left to the dispatcher's own
    /// surface lookup, because that lookup cannot tell the two cases apart.
    /// An empty surface names no surface, so it falls into the same
    /// unknown-surface bucket as a well-formed id belonging to another host,
    /// and the caller is handed a plausible
    /// [`ActionRefusal::SurfaceUnknown`] receipt for a malformed request. The
    /// field's own documentation already states the invariant; this is where
    /// the in-process path keeps it.
    #[must_use]
    pub fn is_identified(&self) -> bool {
        !self.action_id.is_empty() && !self.surface_id.is_empty()
    }

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
    /// The action carried no `action_id`, or no `surface_id`.
    ///
    /// A receipt echoes the id it was asked under, and that echo is the only
    /// thing tying it to the request. An empty id ties a receipt to every
    /// other empty one, so the gesture is refused before delivery rather than
    /// applied under a name that names nothing. An empty `surface_id` is the
    /// same fault answered by a different receipt if it is let through: it
    /// names no surface, so it lands in the unknown-surface bucket and comes
    /// back as [`Self::SurfaceUnknown`], which says a real surface was named
    /// and is not here. The wire half is on
    /// [`SemanticAction::action_id`] and [`SemanticAction::surface_id`],
    /// neither of which will deserialize empty; this is the refusal an
    /// in-process caller gets.
    ActionUnidentified,
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
            ActionRefusal::ActionUnidentified,
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

    /// Neither identifier on the wire may be empty.
    ///
    /// The defect for `action_id`: a frame carrying `""` deserialized,
    /// dispatched, and produced a receipt echoing the empty id --
    /// valid-looking, and impossible to match to the request that caused it or
    /// to tell apart from every other such receipt. For `surface_id` it is
    /// quieter: a dispatcher that does not own the named surface answers from
    /// one shared unknown bucket, so an empty one reads as somebody else's
    /// surface rather than as a malformed frame.
    #[test]
    fn an_action_without_an_id_is_not_a_wire_action() {
        for json in [
            "{\"action_id\":\"\",\"surface_id\":\"composer\",\"target\":\"composer-send\",\"gesture\":\"activate\"}",
            "{\"action_id\":\"a1\",\"surface_id\":\"\",\"target\":\"composer-send\",\"gesture\":\"activate\"}",
        ] {
            let error = serde_json::from_str::<SemanticAction>(json)
                .expect_err("an empty identifier is refused");
            assert!(error.to_string().contains("never empty"), "{error}");
        }
        assert!(!SemanticAction::activate("", "composer", "composer-send").is_identified());
        // The in-process half of the surface invariant. Without it the
        // constructor's empty surface reaches the dispatcher's own lookup,
        // which cannot tell "named nothing" from "named a surface this host
        // does not own" and answers both with `SurfaceUnknown`.
        assert!(!SemanticAction::activate("a1", "", "composer-send").is_identified());
        assert!(SemanticAction::activate("a1", "composer", "composer-send").is_identified());
    }
}
