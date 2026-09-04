//! The evidence one action produced.
//!
//! New in SPEC-AGPUI-SEMANTIC-TREE-1.0 D8. Nothing here is ported.
//!
//! `generation_after` and `hash_after` are read after one frame has painted,
//! so the receipt describes the frame the action produced rather than the
//! frame it was issued in. A refused action reads both after values from the
//! frame the before values came from, which is why every refusal carries
//! `hash_before == hash_after`.

use serde::{Deserialize, Serialize};

use super::action::{ActionOutcome, ActionRefusal};
use super::diff::DiffSummary;
use super::node::Rect;

/// The receipt for one [`SemanticAction`](super::action::SemanticAction).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionReceipt {
    /// `"{surface_id}:receipt:{n}"`, minted by the dispatcher, monotonic per
    /// surface, never reused within a process.
    pub receipt_id: String,
    /// The caller's id, echoed.
    pub action_id: String,
    pub outcome: ActionOutcome,
    pub generation_before: u64,
    pub generation_after: u64,
    pub hash_before: [u8; 32],
    pub hash_after: [u8; 32],
    /// Where the gesture was aimed, when the target had recorded bounds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_bounds_at_dispatch: Option<Rect>,
    pub diff_summary: DiffSummary,
}

impl ActionReceipt {
    /// The id shape every receipt carries: `"{surface_id}:receipt:{n}"`.
    #[must_use]
    pub fn mint_id(surface_id: &str, ordinal: u64) -> String {
        format!("{surface_id}:receipt:{ordinal}")
    }

    /// A receipt for an action that never reached the surface.
    ///
    /// Both hashes and both generations are the same reading, which is the
    /// invariant every refusal test asserts.
    #[must_use]
    pub fn refused(
        receipt_id: String,
        action_id: String,
        refusal: ActionRefusal,
        generation: u64,
        hash: [u8; 32],
        target_bounds_at_dispatch: Option<Rect>,
    ) -> Self {
        Self {
            receipt_id,
            action_id,
            outcome: ActionOutcome::Refused(refusal),
            generation_before: generation,
            generation_after: generation,
            hash_before: hash,
            hash_after: hash,
            target_bounds_at_dispatch,
            diff_summary: DiffSummary::default(),
        }
    }

    #[must_use]
    pub const fn is_applied(&self) -> bool {
        self.outcome.is_applied()
    }

    /// True when the frame after the action carried different semantic state.
    ///
    /// This is V10's "distinct canonical SHA-256 state hashes before and after
    /// the action".
    #[must_use]
    pub fn changed_state(&self) -> bool {
        self.hash_before != self.hash_after
    }
}

#[cfg(test)]
mod tests {
    use super::ActionReceipt;
    use crate::semantics::action::ActionRefusal;

    #[test]
    fn receipt_ids_carry_the_surface_and_a_monotonic_ordinal() {
        assert_eq!(
            ActionReceipt::mint_id("composer", 1),
            "composer:receipt:1".to_string()
        );
        assert_eq!(
            ActionReceipt::mint_id("thread", 42),
            "thread:receipt:42".to_string()
        );
    }

    #[test]
    fn a_refusal_reads_both_hashes_from_one_frame() {
        let receipt = ActionReceipt::refused(
            ActionReceipt::mint_id("composer", 2),
            "a1".into(),
            ActionRefusal::TargetDisabled,
            9,
            [7u8; 32],
            None,
        );
        assert_eq!(receipt.hash_before, receipt.hash_after);
        assert_eq!(receipt.generation_before, receipt.generation_after);
        assert!(!receipt.changed_state());
        assert!(receipt.diff_summary.is_empty());
        assert!(!receipt.is_applied());
    }

    #[test]
    fn a_receipt_round_trips_through_json() {
        let receipt = ActionReceipt::refused(
            ActionReceipt::mint_id("composer", 2),
            "a1".into(),
            ActionRefusal::GenerationStale {
                expected: 3,
                actual: 4,
            },
            9,
            [7u8; 32],
            None,
        );
        let json = serde_json::to_string(&receipt).expect("serializes");
        let back: ActionReceipt = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(receipt, back);
    }
}
