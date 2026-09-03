//! What one frame changed relative to another.
//!
//! New in SPEC-AGPUI-SEMANTIC-TREE-1.0 D6. Nothing here is ported.
//!
//! The diff reports exactly what
//! [`canonical_hash`](super::hash::canonical_hash) covers -- both halves of
//! the tree, `nodes` and `reading_order` -- so a receipt whose hashes differ
//! always reports something, and one whose hashes match can still report
//! `moved`. Bounds are the only thing the hash leaves out, which is why
//! `moved` is the one report that does not imply a hash change.
//!
//! The hash covers two things a per-id walk cannot see. `HashView` keeps both
//! vectors in publication order, so the same ids registered in a different
//! sequence hash differently; and it keeps every entry, so an id published
//! twice hashes differently from the same id published once. Both are
//! reported by [`SnapshotDiff::node_order_changed`] and
//! [`SnapshotDiff::reading_order_changed`] -- without them a receipt could
//! carry two different hashes and an empty summary, which is the one thing
//! this module exists to prevent.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::hash::{HashNode, HashReadingItem, HASHED_NODE_FIELDS, HASHED_READING_FIELDS};
use super::node::{Node, SemanticReadingItem};
use super::snapshot::Snapshot;

/// One field of one node, before and after.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: String,
    pub before: Value,
    pub after: Value,
}

/// Every hashed field that moved on one node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeChange {
    pub id: String,
    pub fields: Vec<FieldChange>,
}

/// The counts a receipt carries instead of the whole diff.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffSummary {
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub moved: usize,
    /// Unmaterialized rows only the later frame published.
    pub reading_added: usize,
    /// Unmaterialized rows only the earlier frame published.
    pub reading_removed: usize,
    /// Unmaterialized rows whose hashed state moved.
    pub reading_changed: usize,
    /// The surviving nodes were registered in a different sequence, or one of
    /// them was registered more than once.
    pub node_order_changed: bool,
    /// The same, for the reading order.
    pub reading_order_changed: bool,
}

impl DiffSummary {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.added == 0
            && self.removed == 0
            && self.changed == 0
            && self.moved == 0
            && self.reading_added == 0
            && self.reading_removed == 0
            && self.reading_changed == 0
            && !self.node_order_changed
            && !self.reading_order_changed
    }
}

/// The difference between two frames' trees.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SnapshotDiff {
    /// Nodes only the later frame published, in its registration order.
    pub added: Vec<Node>,
    /// Ids only the earlier frame published, in its registration order.
    pub removed: Vec<String>,
    /// Nodes whose hashed state moved, in the later frame's registration
    /// order, fields in [`HASHED_NODE_FIELDS`] order.
    pub changed: Vec<NodeChange>,
    /// Nodes whose bounds moved and whose hashed state did not.
    pub moved: Vec<String>,
    /// Reading items only the later frame published, in its publication order.
    pub reading_added: Vec<SemanticReadingItem>,
    /// Ids only the earlier frame's reading order published, in its order.
    pub reading_removed: Vec<String>,
    /// Reading items whose hashed state moved, in the later frame's order,
    /// fields in [`HASHED_READING_FIELDS`] order.
    ///
    /// A row has no bounds until it materializes, so there is no `moved`
    /// counterpart here.
    pub reading_changed: Vec<NodeChange>,
    /// The nodes both frames published appear in a different sequence, or an
    /// id one of them publishes more than once differs inside a repeat.
    ///
    /// Registration order is hashed, and `added`/`removed`/`changed` are all
    /// keyed by id, so this is the only report a pure reshuffle produces. It
    /// is the residual after the per-id walks: inserting a node in the middle
    /// of the tree does not set it, because the ids the two frames share are
    /// still in the same relative order, and a field change inside a
    /// once-published node does not, because `changed` already carries it.
    /// [`residual`] is what is left.
    pub node_order_changed: bool,
    /// The same, for the reading order.
    ///
    /// A frame that publishes one row twice sets this whenever the repeat is
    /// not the same in both frames: the hash carries every entry, and the
    /// per-id walks above deliberately report an id once.
    pub reading_order_changed: bool,
}

impl SnapshotDiff {
    /// Diffs two frames.
    ///
    /// Ids are matched on their first occurrence. A duplicate id is a defect
    /// [`Snapshot::lint`](super::Snapshot::lint) reports; the diff neither
    /// hides it nor reports the same id twice, so a change confined to a
    /// repeat lands in the order flags rather than in `changed`.
    #[must_use]
    pub fn between(before: &Snapshot, after: &Snapshot) -> Self {
        let mut diff = Self::default();

        let mut seen_after: HashSet<&str> = HashSet::new();
        for node in &after.nodes {
            if !seen_after.insert(node.id.as_str()) {
                continue;
            }
            let Some(previous) = before.find(&node.id) else {
                diff.added.push(node.clone());
                continue;
            };
            let fields = changed_fields(previous, node);
            if fields.is_empty() {
                if previous.bounds != node.bounds {
                    diff.moved.push(node.id.clone());
                }
            } else {
                diff.changed.push(NodeChange {
                    id: node.id.clone(),
                    fields,
                });
            }
        }

        let mut seen_before: HashSet<&str> = HashSet::new();
        for node in &before.nodes {
            if seen_before.insert(node.id.as_str()) && !after.contains(&node.id) {
                diff.removed.push(node.id.clone());
            }
        }

        let mut seen_rows_after: HashSet<&str> = HashSet::new();
        for item in &after.reading_order {
            if !seen_rows_after.insert(item.id.as_str()) {
                continue;
            }
            let Some(previous) = before.reading_item(&item.id) else {
                diff.reading_added.push(item.clone());
                continue;
            };
            let fields = changed_reading_fields(previous, item);
            if !fields.is_empty() {
                diff.reading_changed.push(NodeChange {
                    id: item.id.clone(),
                    fields,
                });
            }
        }

        let mut seen_rows_before: HashSet<&str> = HashSet::new();
        for item in &before.reading_order {
            if seen_rows_before.insert(item.id.as_str())
                && after.reading_item(&item.id).is_none()
            {
                diff.reading_removed.push(item.id.clone());
            }
        }

        let added_ids: HashSet<&str> = diff.added.iter().map(|node| node.id.as_str()).collect();
        let removed_ids: HashSet<&str> = diff.removed.iter().map(String::as_str).collect();
        diff.node_order_changed = residual(&before.nodes, &removed_ids, node_id, hashed_fields)
            != residual(&after.nodes, &added_ids, node_id, hashed_fields);

        let rows_added: HashSet<&str> = diff
            .reading_added
            .iter()
            .map(|item| item.id.as_str())
            .collect();
        let rows_removed: HashSet<&str> = diff.reading_removed.iter().map(String::as_str).collect();
        diff.reading_order_changed = residual(
            &before.reading_order,
            &rows_removed,
            reading_item_id,
            hashed_reading_fields,
        ) != residual(
            &after.reading_order,
            &rows_added,
            reading_item_id,
            hashed_reading_fields,
        );

        diff
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.removed.is_empty()
            && self.changed.is_empty()
            && self.moved.is_empty()
            && self.reading_added.is_empty()
            && self.reading_removed.is_empty()
            && self.reading_changed.is_empty()
            && !self.node_order_changed
            && !self.reading_order_changed
    }

    #[must_use]
    pub fn summary(&self) -> DiffSummary {
        DiffSummary {
            added: self.added.len(),
            removed: self.removed.len(),
            changed: self.changed.len(),
            moved: self.moved.len(),
            reading_added: self.reading_added.len(),
            reading_removed: self.reading_removed.len(),
            reading_changed: self.reading_changed.len(),
            node_order_changed: self.node_order_changed,
            reading_order_changed: self.reading_order_changed,
        }
    }
}

/// The part of one frame's sequence the per-id walks above do not carry.
///
/// Ids only one frame published drop out first, so an insertion or a deletion
/// does not read as a reshuffle. Every surviving id keeps its position, so a
/// reshuffle still shows. The first occurrence of an id carries no fields --
/// `changed` and `reading_changed` already report what moved inside it -- and
/// every occurrence after the first carries its hashed fields, because those
/// walks report an id once while the hash covers them all. Without that last
/// part, `[a(x), a(y)]` becoming `[a(x), a(z)]` would hash differently and
/// diff empty, and the receipt would say the frame did not move.
fn residual<'a, T>(
    entries: &'a [T],
    skip: &HashSet<&str>,
    id: fn(&'a T) -> &'a str,
    hashed: fn(&'a T) -> Map<String, Value>,
) -> Vec<(&'a str, Option<Map<String, Value>>)> {
    let mut seen: HashSet<&str> = HashSet::new();
    let mut kept = Vec::new();
    for entry in entries {
        let entry_id = id(entry);
        if skip.contains(entry_id) {
            continue;
        }
        let repeat = !seen.insert(entry_id);
        kept.push((entry_id, repeat.then(|| hashed(entry))));
    }
    kept
}

fn node_id(node: &Node) -> &str {
    node.id.as_str()
}

fn reading_item_id(item: &SemanticReadingItem) -> &str {
    item.id.as_str()
}

fn hashed_fields(node: &Node) -> Map<String, Value> {
    match serde_json::to_value(HashNode::of(node)) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn hashed_reading_fields(item: &SemanticReadingItem) -> Map<String, Value> {
    match serde_json::to_value(HashReadingItem::of(item)) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn changed_reading_fields(
    before: &SemanticReadingItem,
    after: &SemanticReadingItem,
) -> Vec<FieldChange> {
    let before_fields = hashed_reading_fields(before);
    let after_fields = hashed_reading_fields(after);
    field_changes(&HASHED_READING_FIELDS, &before_fields, &after_fields)
}

fn field_changes(
    names: &[&str],
    before: &Map<String, Value>,
    after: &Map<String, Value>,
) -> Vec<FieldChange> {
    names
        .iter()
        .filter_map(|field| {
            let old = before.get(*field).cloned().unwrap_or(Value::Null);
            let new = after.get(*field).cloned().unwrap_or(Value::Null);
            (old != new).then(|| FieldChange {
                field: (*field).to_owned(),
                before: old,
                after: new,
            })
        })
        .collect()
}

fn changed_fields(before: &Node, after: &Node) -> Vec<FieldChange> {
    let before_fields = hashed_fields(before);
    let after_fields = hashed_fields(after);
    field_changes(&HASHED_NODE_FIELDS, &before_fields, &after_fields)
}

#[cfg(test)]
mod tests {
    use super::SnapshotDiff;
    use crate::semantics::hash::canonical_hash;
    use crate::semantics::node::{Node, Rect, SemanticReadingItem};
    use crate::semantics::role::Role;
    use crate::semantics::snapshot::Snapshot;

    fn row(id: &str) -> SemanticReadingItem {
        SemanticReadingItem {
            id: id.into(),
            role: Role::Row,
            text: Some("a message".into()),
            ..SemanticReadingItem::default()
        }
    }

    fn with_rows(nodes: Vec<Node>, reading_order: Vec<SemanticReadingItem>) -> Snapshot {
        Snapshot {
            generation: 1,
            nodes,
            reading_order,
        }
    }

    fn node(id: &str) -> Node {
        Node {
            id: id.into(),
            role: Role::Button,
            visible: true,
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
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
    fn a_bounds_only_change_is_a_move_not_a_change() {
        let before = snapshot(vec![node("composer-send")]);
        let mut shifted = node("composer-send");
        shifted.bounds.y = 41.0;
        let diff = SnapshotDiff::between(&before, &snapshot(vec![shifted]));
        assert_eq!(diff.moved, vec!["composer-send".to_string()]);
        assert!(diff.changed.is_empty());
        assert!(!diff.is_empty());
    }

    #[test]
    fn a_removed_chip_is_the_only_thing_the_diff_reports() {
        let before = snapshot(vec![node("composer"), node("composer-chip-r-118")]);
        let after = snapshot(vec![node("composer")]);
        let diff = SnapshotDiff::between(&before, &after);
        assert_eq!(diff.removed, vec!["composer-chip-r-118".to_string()]);
        assert!(diff.added.is_empty());
        assert!(diff.changed.is_empty());
        assert!(diff.moved.is_empty());
    }

    #[test]
    fn a_changed_node_names_its_fields_in_declaration_order() {
        let before = snapshot(vec![node("composer-send")]);
        let mut after_node = node("composer-send");
        after_node.disabled = true;
        after_node.text = Some("Send blocked".into());
        let diff = SnapshotDiff::between(&before, &snapshot(vec![after_node]));
        let names: Vec<&str> = diff.changed[0]
            .fields
            .iter()
            .map(|change| change.field.as_str())
            .collect();
        assert_eq!(names, vec!["text", "disabled"]);
        assert_eq!(diff.changed[0].fields[1].after, serde_json::json!(true));
    }

    #[test]
    fn ordering_follows_registration_not_the_id() {
        let before = snapshot(vec![node("z"), node("a")]);
        let after = snapshot(vec![node("m"), node("b")]);
        let diff = SnapshotDiff::between(&before, &after);
        assert_eq!(
            diff.added.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
            vec!["m", "b"]
        );
        assert_eq!(diff.removed, vec!["z".to_string(), "a".to_string()]);
    }

    #[test]
    fn a_duplicate_id_is_reported_once_and_not_hidden() {
        let before = snapshot(vec![node("row")]);
        let after = snapshot(vec![node("row"), node("row")]);
        let diff = SnapshotDiff::between(&before, &after);

        // The per-id walks still report an id once: the second registration
        // is not a second `added` entry, and the field walk does not compare
        // the node against itself.
        assert!(diff.added.is_empty());
        assert!(diff.changed.is_empty());
        assert!(diff.moved.is_empty());

        // This assertion used to be `diff.is_empty()`, which enshrined the
        // defect: `canonical_hash` keeps both entries, so the two frames hash
        // differently and a receipt reporting nothing would be claiming a
        // change it could not name.
        assert_ne!(canonical_hash(&before), canonical_hash(&after));
        assert!(diff.node_order_changed, "{diff:?}");
        assert!(!diff.is_empty());

        assert_eq!(after.lint().len(), 2, "the lint still reports the duplicate");
    }

    #[test]
    fn an_identical_frame_diffs_empty_and_summarises_to_zero() {
        let one = snapshot(vec![node("a"), node("b")]);
        let diff = SnapshotDiff::between(&one, &one.clone());
        assert!(diff.is_empty());
        assert!(diff.summary().is_empty());
    }

    #[test]
    fn a_row_the_thread_scrolled_past_is_reported_like_any_other_removal() {
        let before = with_rows(vec![node("thread")], vec![row("thread.m1"), row("thread.m2")]);
        let after = with_rows(vec![node("thread")], vec![row("thread.m2"), row("thread.m3")]);
        let diff = SnapshotDiff::between(&before, &after);
        assert_eq!(
            diff.reading_added
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec!["thread.m3"]
        );
        assert_eq!(diff.reading_removed, vec!["thread.m1".to_string()]);
        assert!(diff.added.is_empty() && diff.removed.is_empty());
    }

    #[test]
    fn a_selected_row_that_never_materialized_still_reports_the_field() {
        let before = with_rows(vec![node("thread")], vec![row("thread.m1")]);
        let mut chosen = row("thread.m1");
        chosen.selected = true;
        let after = with_rows(vec![node("thread")], vec![chosen]);
        let diff = SnapshotDiff::between(&before, &after);
        assert_eq!(diff.reading_changed.len(), 1);
        assert_eq!(diff.reading_changed[0].id, "thread.m1");
        assert_eq!(
            diff.reading_changed[0]
                .fields
                .iter()
                .map(|field| field.field.as_str())
                .collect::<Vec<_>>(),
            vec!["selected"]
        );
    }

    #[test]
    fn no_reading_order_change_escapes_a_receipt_whose_hashes_moved() {
        // The contract this module states: what `canonical_hash` covers, the
        // diff reports. `reading_order` is half of what it covers, so a frame
        // that only moves a row used to hash differently and summarize empty.
        let base = with_rows(vec![node("thread")], vec![row("thread.m1")]);
        let mut renamed = row("thread.m1");
        renamed.text = Some("an edited message".into());
        let mut refocused = row("thread.m1");
        refocused.focused = true;
        for after in [
            with_rows(vec![node("thread")], vec![renamed]),
            with_rows(vec![node("thread")], vec![refocused]),
            with_rows(vec![node("thread")], Vec::new()),
            with_rows(vec![node("thread")], vec![row("thread.m9")]),
        ] {
            assert_ne!(canonical_hash(&base), canonical_hash(&after));
            assert!(
                !SnapshotDiff::between(&base, &after).summary().is_empty(),
                "hashes moved and the summary claimed nothing changed"
            );
        }
    }
    /// The other half of the invariant: publication order and multiplicity
    /// are hashed, and a per-id walk cannot see either.
    ///
    /// Every pair here hashed differently and summarized empty before
    /// `node_order_changed` / `reading_order_changed` existed, which is a
    /// receipt claiming a frame changed while reporting nothing that did.
    #[test]
    fn no_reshuffle_escapes_a_receipt_whose_hashes_moved() {
        let base = with_rows(
            vec![node("thread"), node("composer")],
            vec![row("thread.m1"), row("thread.m2")],
        );
        for after in [
            // The same nodes, registered the other way round.
            with_rows(
                vec![node("composer"), node("thread")],
                vec![row("thread.m1"), row("thread.m2")],
            ),
            // The same rows, published the other way round.
            with_rows(
                vec![node("thread"), node("composer")],
                vec![row("thread.m2"), row("thread.m1")],
            ),
            // One node registered twice.
            with_rows(
                vec![node("thread"), node("composer"), node("thread")],
                vec![row("thread.m1"), row("thread.m2")],
            ),
            // One row published twice.
            with_rows(
                vec![node("thread"), node("composer")],
                vec![row("thread.m1"), row("thread.m2"), row("thread.m1")],
            ),
        ] {
            assert_ne!(canonical_hash(&base), canonical_hash(&after));
            assert!(
                !SnapshotDiff::between(&base, &after).summary().is_empty(),
                "hashes moved and the summary claimed nothing changed"
            );
        }
    }

    /// The order report is the residual after the set difference, so an
    /// ordinary insertion must not set it -- otherwise every frame that adds
    /// a row also claims the tree was reshuffled and the signal is noise.
    #[test]
    fn inserting_in_the_middle_is_not_a_reshuffle() {
        let before = with_rows(
            vec![node("thread"), node("composer")],
            vec![row("thread.m1"), row("thread.m3")],
        );
        let after = with_rows(
            vec![node("thread"), node("banner"), node("composer")],
            vec![row("thread.m1"), row("thread.m2"), row("thread.m3")],
        );
        let diff = SnapshotDiff::between(&before, &after);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.reading_added.len(), 1);
        assert!(!diff.node_order_changed, "an insertion is not a move");
        assert!(!diff.reading_order_changed, "an insertion is not a move");

        // And the reverse: a deletion is not a reshuffle either.
        let back = SnapshotDiff::between(&after, &before);
        assert_eq!(back.removed.len(), 1);
        assert_eq!(back.reading_removed.len(), 1);
        assert!(!back.node_order_changed);
        assert!(!back.reading_order_changed);
    }

    #[test]
    fn a_swap_names_the_half_that_moved() {
        let before = with_rows(vec![node("a"), node("b")], vec![row("r1"), row("r2")]);
        let nodes_swapped = with_rows(vec![node("b"), node("a")], vec![row("r1"), row("r2")]);
        let diff = SnapshotDiff::between(&before, &nodes_swapped);
        assert!(diff.node_order_changed);
        assert!(
            !diff.reading_order_changed,
            "the reading order did not move and must not be reported"
        );

        let rows_swapped = with_rows(vec![node("a"), node("b")], vec![row("r2"), row("r1")]);
        let diff = SnapshotDiff::between(&before, &rows_swapped);
        assert!(!diff.node_order_changed);
        assert!(diff.reading_order_changed);
    }

    /// The per-id walk reports `thread.m1` once, so a change confined to the
    /// second publication of it has nowhere else to land. The hash carries
    /// both entries, and two hashes with an empty summary is the one thing
    /// this module exists to prevent.
    #[test]
    fn a_change_inside_a_repeated_row_is_reported() {
        let mut first_draft = row("thread.m1");
        first_draft.text = Some("the first draft".into());
        let before = with_rows(vec![node("thread")], vec![row("thread.m1"), first_draft]);

        let mut second_draft = row("thread.m1");
        second_draft.text = Some("the second draft".into());
        let after = with_rows(vec![node("thread")], vec![row("thread.m1"), second_draft]);

        assert_ne!(canonical_hash(&before), canonical_hash(&after));
        let diff = SnapshotDiff::between(&before, &after);
        assert!(
            diff.reading_changed.is_empty(),
            "the repeat is not the id's first occurrence: {diff:#?}"
        );
        assert!(diff.reading_order_changed, "{diff:#?}");
        assert!(!diff.is_empty());
    }

    /// The same hole on the other half of the tree.
    #[test]
    fn a_change_inside_a_repeated_node_is_reported() {
        let before = snapshot(vec![node("composer-send"), node("composer-send")]);
        let mut repeat = node("composer-send");
        repeat.disabled = true;
        let after = snapshot(vec![node("composer-send"), repeat]);

        assert_ne!(canonical_hash(&before), canonical_hash(&after));
        let diff = SnapshotDiff::between(&before, &after);
        assert!(
            diff.changed.is_empty(),
            "the repeat is not the id's first occurrence: {diff:#?}"
        );
        assert!(diff.node_order_changed, "{diff:#?}");

        // And a node published once keeps reporting through `changed`: the
        // residual carries no fields for a first occurrence.
        let mut disabled = node("composer-send");
        disabled.disabled = true;
        let single = SnapshotDiff::between(
            &snapshot(vec![node("composer-send")]),
            &snapshot(vec![disabled]),
        );
        assert_eq!(single.changed.len(), 1);
        assert!(!single.node_order_changed, "{single:#?}");
    }

    /// The residual must not fire on a repeat that did not move: two readings
    /// of one frame hash the same, so their diff has to be empty.
    #[test]
    fn an_unchanged_repeat_is_not_a_reshuffle() {
        let frame = with_rows(
            vec![node("thread"), node("thread")],
            vec![row("thread.m1"), row("thread.m1")],
        );
        let diff = SnapshotDiff::between(&frame, &frame);
        assert!(diff.is_empty(), "{diff:#?}");
    }
}
