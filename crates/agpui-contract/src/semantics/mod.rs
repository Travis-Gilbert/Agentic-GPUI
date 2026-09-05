//! The semantic tree: the DOM a native GPUI window does not have.
//!
//! An agent, a test, and a screen reader address the same nodes from the same
//! probe. This module is renderer-free on purpose — `tests/boundary.rs`
//! proves no GPUI package is reachable from this crate — so the same types
//! serialize onto the AG-UI wire that the in-process host passes by value.
//!
//! The probe, the coordinator, and the dispatcher that turns a
//! [`SemanticAction`] into real platform input live in `agpui`, which is where
//! GPUI is nameable.

pub mod action;
pub mod diff;
pub mod hash;
pub mod ident;
pub mod node;
pub mod receipt;
pub mod redact;
pub mod role;
pub mod snapshot;

pub use action::{ActionOutcome, ActionRefusal, SemanticAction, SemanticGesture};
pub use diff::{DiffSummary, FieldChange, NodeChange, SnapshotDiff};
pub use hash::{canonical_hash, hex, HashNode, HashReadingItem, HashView, HASHED_NODE_FIELDS, HASHED_READING_FIELDS};
pub use ident::{segment, Ident, IdentError, IdentViolation};
pub use node::{LiveRegion, Node, Rect, SemanticReadingItem};
pub use receipt::ActionReceipt;
pub use redact::redact_sensitive_text;
pub use role::Role;
pub use snapshot::{LintFinding, Snapshot};
