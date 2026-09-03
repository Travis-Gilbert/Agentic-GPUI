//! What one frame changed relative to another.
//!
//! New in SPEC-AGPUI-SEMANTIC-TREE-1.0 D6. Nothing here is ported.
//!
//! The diff is keyed by id and reports exactly the fields
//! [`canonical_hash`](super::hash::canonical_hash) covers, so a receipt whose
//! hashes differ always has a non-empty `changed`, `added`, or `removed`, and
//! one whose hashes match can still report `moved`.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::hash::{HashNode, HASHED_NODE_FIELDS};
use super::node::Node;
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
}

impl DiffSummary {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.added == 0 && self.removed == 0 && self.changed == 0 && self.moved == 0
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

        diff
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.removed.is_empty()
            && self.changed.is_empty()
            && self.moved.is_empty()
    }

    #[must_use]
    pub fn summary(&self) -> DiffSummary {
        DiffSummary {
            added: self.added.len(),
            removed: self.removed.len(),
            changed: self.changed.len(),
            moved: self.moved.len(),
        }
    }
}

fn hashed_fields(node: &Node) -> Map<String, Value> {
    match serde_json::to_value(HashNode::of(node)) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn changed_fields(before: &Node, after: &Node) -> Vec<FieldChange> {
    let before_fields = hashed_fields(before);
    let after_fields = hashed_fields(after);
    HASHED_NODE_FIELDS
        .iter()
        .filter_map(|field| {
            let old = before_fields.get(*field).cloned().unwrap_or(Value::Null);
            let new = after_fields.get(*field).cloned().unwrap_or(Value::Null);
            (old != new).then(|| FieldChange {
                field: (*field).to_owned(),
                before: old,
                after: new,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::SnapshotDiff;
    use crate::semantics::node::{Node, Rect};
    use crate::semantics::role::Role;
    use crate::semantics::snapshot::Snapshot;

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
}
