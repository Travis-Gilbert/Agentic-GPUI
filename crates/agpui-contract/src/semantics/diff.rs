//! What one frame changed relative to another.
//!
//! New in SPEC-AGPUI-SEMANTIC-TREE-1.0 D6. Nothing here is ported.
//!
//! The diff is keyed by id and reports exactly the fields
//! [`canonical_hash`](super::hash::canonical_hash) covers -- both halves of
//! the tree, `nodes` and `reading_order` -- so a receipt whose hashes differ
//! always reports something, and one whose hashes match can still report
//! `moved`. Bounds are the only thing the hash leaves out, which is why
//! `moved` is the one report that does not imply a hash change.

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
}

impl SnapshotDiff {
    /// Diffs two frames.
    ///
    /// Ids are matched on their first occurrence. A duplicate id is a defect
    /// [`Snapshot::lint`](super::Snapshot::lint) reports; the diff neither
    /// hides it nor reports the same id twice.
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
        }
    }
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
        assert!(diff.is_empty(), "{diff:?}");
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
}
