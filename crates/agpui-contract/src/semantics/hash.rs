//! The canonical state hash: what changed, with geometry left out.
//!
//! New in SPEC-AGPUI-SEMANTIC-TREE-1.0 D5. Nothing here is ported.
//!
//! A receipt claims a frame produced a change. The claim is only evidence if
//! the number it compares is a function of semantic state and nothing else, so
//! `bounds`, `visible`, `hovered`, `pressed`, `labels`, `describes`,
//! `value_min`, `value_max`, `live_atomic`, and `generation` are excluded on
//! purpose. A hover, a resize, or a re-layout does not change the hash.

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

use super::node::{LiveRegion, Node, SemanticReadingItem};
use super::role::Role;
use super::snapshot::Snapshot;

/// The projection [`canonical_hash`] hashes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HashView {
    /// A marker, always true, so the hashed document says out loud that the
    /// registry's frame counter is not part of it.
    pub generation_independent: bool,
    pub nodes: Vec<HashNode>,
    pub reading_order: Vec<HashReadingItem>,
}

/// One node's semantic state, geometry and pointer transients removed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "this is a projection of Node's ARIA state, one field per state"
)]
pub struct HashNode {
    pub id: String,
    pub role: Role,
    pub parent: Option<String>,
    pub text: Option<String>,
    pub description: Option<String>,
    pub value: Option<String>,
    pub placeholder: Option<String>,
    pub focused: bool,
    pub disabled: bool,
    pub read_only: bool,
    pub selected: Option<bool>,
    pub checked: Option<bool>,
    pub expanded: Option<bool>,
    pub value_now: Option<f32>,
    pub level: Option<u32>,
    pub busy: bool,
    pub invalid: bool,
    pub required: bool,
    pub live: Option<LiveRegion>,
    pub modal: bool,
}

/// One unmaterialized row's state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashReadingItem {
    pub id: String,
    pub parent: Option<String>,
    pub role: Role,
    pub text: Option<String>,
    pub focused: bool,
    pub selected: bool,
}

/// The field names [`HashNode`] carries, in declaration order.
///
/// [`SnapshotDiff`](super::diff::SnapshotDiff) walks this list so a change
/// report names the same fields the hash covers, and in the same order.
pub const HASHED_NODE_FIELDS: [&str; 19] = [
    "role",
    "parent",
    "text",
    "description",
    "value",
    "placeholder",
    "focused",
    "disabled",
    "read_only",
    "selected",
    "checked",
    "expanded",
    "value_now",
    "level",
    "busy",
    "invalid",
    "required",
    "live",
    "modal",
];

/// The field names [`HashReadingItem`] carries, in declaration order.
///
/// `id` is the key rather than a field, so it is not listed: the same rule
/// [`HASHED_NODE_FIELDS`] follows.
pub const HASHED_READING_FIELDS: [&str; 5] =
    ["parent", "role", "text", "focused", "selected"];

impl HashNode {
    pub(crate) fn of(node: &Node) -> Self {
        Self {
            id: node.id.clone(),
            role: node.role,
            parent: node.parent.clone(),
            text: node.text.clone(),
            description: node.description.clone(),
            value: node.value.clone(),
            placeholder: node.placeholder.clone(),
            focused: node.focused,
            disabled: node.disabled,
            read_only: node.read_only,
            selected: node.selected,
            checked: node.checked,
            expanded: node.expanded,
            value_now: finite(node.value_now),
            level: node.level,
            busy: node.busy,
            invalid: node.invalid,
            required: node.required,
            live: node.live,
            modal: node.modal,
        }
    }
}

impl HashReadingItem {
    pub(crate) fn of(item: &SemanticReadingItem) -> Self {
        Self {
            id: item.id.clone(),
            parent: item.parent.clone(),
            role: item.role,
            text: item.text.clone(),
            focused: item.focused,
            selected: item.selected,
        }
    }
}

/// JSON has no NaN and no infinity, so a non-finite slider value is recorded
/// as absent rather than making the whole snapshot unhashable.
///
/// Negative zero is folded onto positive zero for the receipt invariant, not
/// for tidiness: `serde_json` writes `-0.0` and `0.0` as different bytes, so
/// they hash differently, while `SnapshotDiff` compares them as
/// `serde_json::Value` numbers, where IEEE equality calls them the same. A
/// slider crossing zero would otherwise mint a receipt whose hashes moved and
/// whose summary was empty.
fn finite(value: Option<f32>) -> Option<f32> {
    value
        .filter(|number| number.is_finite())
        .map(|number| if number == 0.0 { 0.0 } else { number })
}

impl HashView {
    /// The hashed projection of one snapshot, in registration order.
    #[must_use]
    pub fn of(snapshot: &Snapshot) -> Self {
        Self {
            generation_independent: true,
            nodes: snapshot.nodes.iter().map(HashNode::of).collect(),
            reading_order: snapshot
                .reading_order
                .iter()
                .map(HashReadingItem::of)
                .collect(),
        }
    }
}

impl Snapshot {
    /// The projection [`canonical_hash`] hashes.
    #[must_use]
    pub fn hash_view(&self) -> HashView {
        HashView::of(self)
    }
}

/// SHA-256 over the canonical JSON of [`HashView`].
///
/// Canonical means the struct is serialized straight to a compact string, with
/// no whitespace and with fields in declaration order. It never passes through
/// [`serde_json::Value`], and that is the whole point: `Value`'s object
/// representation and its numbers are both decided by `serde_json` *features*,
/// which Cargo unifies across whatever else is in the build.
/// `preserve_order` swaps a sorted `BTreeMap` for an insertion-ordered
/// `IndexMap`, and `arbitrary_precision` changes how a float becomes a
/// `Number` -- so a `Value` round trip makes the hash a function of the
/// consumer's dependency graph rather than of the tree.
///
/// This is not hypothetical here. `rustyredcore_THG`'s `workspace-hack` turns
/// both features on, `apps/theoremweb` does not, and the two trees are exactly
/// the two consumers acceptance 12 requires to agree. Before this went through
/// the struct directly they hashed the same snapshot to different digests.
/// `the_hash_is_pinned_across_workspaces` holds the line, and the `agpui`
/// crate asserts the same constant from the other tree.
///
/// [`HashView`] carries no map-typed field, so declaration order is total.
///
/// # Panics
///
/// Unreachable. [`HashView`] has only finite floats, which `serde_json` cannot
/// fail to represent.
#[must_use]
pub fn canonical_hash(snapshot: &Snapshot) -> [u8; 32] {
    let canonical = serde_json::to_string(&snapshot.hash_view())
        .expect("HashView has only finite floats");
    Sha256::digest(canonical.as_bytes()).into()
}

/// The lowercase hex of a canonical hash, for receipts a person reads.
#[must_use]
pub fn hex(hash: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    hash.iter().fold(String::with_capacity(64), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

#[cfg(test)]
mod tests {
    use super::{canonical_hash, hex, HASHED_NODE_FIELDS};
    use crate::semantics::node::{Node, Rect};
    use crate::semantics::role::Role;
    use crate::semantics::snapshot::Snapshot;

    fn button(id: &str) -> Node {
        Node {
            id: id.into(),
            role: Role::Button,
            visible: true,
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: 20.0,
                height: 20.0,
            },
            ..Node::default()
        }
    }

    fn snapshot(nodes: Vec<Node>) -> Snapshot {
        Snapshot {
            generation: 1,
            nodes,
            reading_order: Vec::new(),
        }
    }

    #[test]
    fn geometry_and_pointer_transients_do_not_change_the_hash() {
        let before = snapshot(vec![button("composer-send")]);
        let mut moved = button("composer-send");
        moved.bounds = Rect {
            x: 400.0,
            y: 91.5,
            width: 28.0,
            height: 28.0,
        };
        moved.hovered = true;
        moved.pressed = true;
        moved.visible = false;
        moved.labels = Some("send".into());
        moved.describes = Some("composer".into());
        moved.value_min = Some(0.0);
        moved.value_max = Some(1.0);
        moved.live_atomic = true;
        let after = Snapshot {
            generation: 97,
            ..snapshot(vec![moved])
        };
        assert_eq!(canonical_hash(&before), canonical_hash(&after));
    }

    #[test]
    fn a_disabled_flip_changes_the_hash() {
        let before = snapshot(vec![button("composer-send")]);
        let mut disabled = button("composer-send");
        disabled.disabled = true;
        assert_ne!(canonical_hash(&before), canonical_hash(&snapshot(vec![disabled])));
    }

    #[test]
    fn registration_order_is_part_of_the_hash() {
        let one = snapshot(vec![button("a"), button("b")]);
        let other = snapshot(vec![button("b"), button("a")]);
        assert_ne!(canonical_hash(&one), canonical_hash(&other));
    }

    #[test]
    fn a_non_finite_slider_value_still_hashes() {
        let mut broken = button("slider");
        broken.role = Role::Slider;
        broken.value_now = Some(f32::NAN);
        let mut absent = button("slider");
        absent.role = Role::Slider;
        assert_eq!(
            canonical_hash(&snapshot(vec![broken])),
            canonical_hash(&snapshot(vec![absent]))
        );
    }

    /// The receipt invariant, at the one value where the hash and the diff
    /// disagreed about equality.
    ///
    /// `serde_json` writes `-0.0` and `0.0` as different bytes, so they hash
    /// differently, while `SnapshotDiff` compares them as `Value` numbers,
    /// where IEEE equality calls them the same. A slider crossing zero minted
    /// a receipt whose hashes moved and whose summary was empty.
    #[test]
    fn a_slider_crossing_zero_does_not_move_the_hash_behind_an_empty_diff() {
        let mut negative = button("volume");
        negative.role = Role::Slider;
        negative.value_now = Some(-0.0);
        let mut positive = negative.clone();
        positive.value_now = Some(0.0);

        assert_eq!(
            canonical_hash(&snapshot(vec![negative.clone()])),
            canonical_hash(&snapshot(vec![positive.clone()])),
            "signed zero moved the hash that the diff reads as unchanged"
        );

        // The sign is the only thing folded: a real move still shows.
        let mut moved = negative;
        moved.value_now = Some(0.5);
        assert_ne!(
            canonical_hash(&snapshot(vec![positive])),
            canonical_hash(&snapshot(vec![moved]))
        );
    }

    #[test]
    fn the_hash_is_stable_across_runs() {
        let hash = canonical_hash(&snapshot(vec![button("composer-send")]));
        assert_eq!(hex(&hash).len(), 64);
        assert_eq!(hash, canonical_hash(&snapshot(vec![button("composer-send")])));
    }

    #[test]
    fn every_hashed_field_is_named_once() {
        let mut sorted = HASHED_NODE_FIELDS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), HASHED_NODE_FIELDS.len());
    }

    /// The digest of one fixed snapshot, written down.
    ///
    /// `agpui` asserts this same constant from `apps/theoremweb`, which builds
    /// this crate with a different `serde_json` feature set. That is the whole
    /// oracle: two trees, one number. The fixture is spelled out on both sides
    /// on purpose -- sharing it through a helper would let a change to the
    /// helper move both sides together and prove nothing.
    #[test]
    fn the_hash_is_pinned_across_workspaces() {
        let node = Node {
            id: "composer-send".into(),
            role: Role::Button,
            parent: Some("composer".into()),
            text: Some("Send".into()),
            value_now: Some(0.1),
            bounds: Rect {
                x: 1.,
                y: 2.,
                width: 3.,
                height: 4.,
            },
            visible: true,
            ..Node::default()
        };
        let snapshot = Snapshot {
            generation: 7,
            nodes: vec![node],
            reading_order: Vec::new(),
        };
        assert_eq!(
            hex(&canonical_hash(&snapshot)),
            "88ecd6acc565570650efabe9ac0634d511f42e1a184bd61b1aba2b785d6d40b1"
        );
    }
}
