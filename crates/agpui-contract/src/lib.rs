//! `agpui-contract`: the renderer-free half of the AGPUI semantic tree.
//!
//! This is the first of the four AGPUI crates on the rate-of-change axis
//! (`agpui-contract`, `agpui-runtime`, `agpui-registry`, `agpui`). It carries
//! the types, the laws over them, the canonical hash, the diff, and the action
//! and receipt wire contract. It names no renderer, and `tests/boundary.rs`
//! fails the build if one enters its dependency tree.
//!
//! See `docs/AGPUI.md` and `docs/plans/SPEC-AGPUI-SEMANTIC-TREE-1_0.md`.
//!
//! SPEC-AGPUI-HOME-1.0 H2 moved the four UI documents down here from
//! `theorem-surface-contracts`: they are the vocabulary two AGPUI crates both
//! need, so move rule 1 puts them in the contract. They name no renderer and
//! no Theorem type, and they close over each other -- `surface` reads
//! `intent`, `intent` reads `composer` and `thread`, `composer` reads
//! `thread` -- so the set moved whole or not at all.

pub mod semantics;

pub mod composer;
pub mod intent;
pub mod surface;
pub mod thread;

pub use semantics::{
    canonical_hash, hex, redact_sensitive_text, ActionOutcome, ActionReceipt, ActionRefusal,
    DiffSummary, FieldChange, HashNode, HashReadingItem, HashView, Ident, IdentError,
    IdentViolation, LintFinding, LiveRegion, Node, NodeChange, Rect, Role, SemanticAction,
    SemanticGesture, SemanticReadingItem, Snapshot, SnapshotDiff, HASHED_NODE_FIELDS,
    HASHED_READING_FIELDS,
};
