//! `agpui-contract`: the renderer-free half of the AGPUI semantic tree.
//!
//! This is the first of the four AGPUI crates on the rate-of-change axis
//! (`agpui-contract`, `agpui-runtime`, `agpui-registry`, `agpui`). It carries
//! the types, the laws over them, the canonical hash, the diff, and the action
//! and receipt wire contract. It names no renderer, and `tests/boundary.rs`
//! fails the build if one enters its dependency tree.
//!
//! See `docs/AGPUI-CRATES.md` and `docs/plans/SPEC-AGPUI-SEMANTIC-TREE-1_0.md`.

pub mod semantics;

pub use semantics::{
    canonical_hash, hex, redact_sensitive_text, ActionOutcome, ActionReceipt, ActionRefusal,
    DiffSummary, FieldChange, HashNode, HashReadingItem, HashView, Ident, IdentError,
    IdentViolation, LintFinding, LiveRegion, Node, NodeChange, Rect, Role, SemanticAction,
    SemanticGesture, SemanticReadingItem, Snapshot, SnapshotDiff, HASHED_NODE_FIELDS,
    HASHED_READING_FIELDS,
};
