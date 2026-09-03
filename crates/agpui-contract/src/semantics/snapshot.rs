//! The frame-scoped tree, and the lint that reports the defects it can carry.
//!
//! [`Snapshot`] is ported from `gpui-box`,
//! `crates/gpui-kit-semantics/src/lib.rs`, at
//! `e993d0f4e2dbd4a9697db79c6428a623856444a4` (GPUI Box contributors,
//! MIT OR Apache-2.0). `reading_order` and [`Snapshot::lint`] are new in
//! SPEC-AGPUI-SEMANTIC-TREE-1.0 D3.

use std::collections::{BTreeSet, HashSet};

use serde::{Deserialize, Serialize};

use super::ident::Ident;
use super::node::{Node, SemanticReadingItem};
use super::role::Role;

/// One frame's published tree.
///
/// A node absent from the next frame is gone. A frame that publishes nothing
/// reports an empty tree. Registration order is preserved and duplicate ids
/// are kept, so [`Snapshot::lint`] can observe the defect rather than the
/// snapshot hiding it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshot {
    pub generation: u64,
    pub nodes: Vec<Node>,
    /// Rows a virtualized surface knows about and has not materialized.
    ///
    /// A materialized row appears in both `nodes` and `reading_order`; that is
    /// the contract, not a duplicate.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub reading_order: Vec<SemanticReadingItem>,
}

/// A defect one frame's tree carries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "finding", rename_all = "kebab-case")]
pub enum LintFinding {
    /// A node names a parent no node published.
    OrphanParent { id: String, parent: String },
    /// One frame published the same id twice in one half of the tree --
    /// either two nodes, or two reading items.
    DuplicateId { id: String },
    /// An id is outside the [`Ident`] grammar.
    MalformedId { id: String },
    /// No node published [`Role::Window`], so no node's parent chain can
    /// reach one.
    MissingWindow,
    /// A reading item names a parent no node published.
    ReadingItemParentMissing { id: String, parent: String },
    /// This node's parent chain never terminates: walking it revisits an id.
    ///
    /// A chain that closes on itself reaches no root, so it reaches no
    /// [`Role::Window`] either. [`Snapshot::ancestors_of`] and
    /// [`Snapshot::descendants_of`] both refuse to loop on one, which is what
    /// keeps a cycle from hanging a consumer -- and is also why nothing
    /// noticed it. The ancestor set a caller gets back is truncated at the
    /// repeat, so an occlusion or containment decision taken from it is
    /// answered from part of the chain.
    ParentCycle { id: String },
}

impl Snapshot {
    #[must_use]
    pub fn find(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|node| node.id == id)
    }

    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.find(id).is_some()
    }

    /// The reading item with this id, when the surface published one.
    #[must_use]
    pub fn reading_item(&self, id: &str) -> Option<&SemanticReadingItem> {
        self.reading_order.iter().find(|item| item.id == id)
    }

    #[must_use]
    pub fn ids(&self) -> Vec<&str> {
        self.nodes.iter().map(|node| node.id.as_str()).collect()
    }

    #[must_use]
    pub fn under(&self, prefix: &str) -> Vec<&Node> {
        self.nodes
            .iter()
            .filter(|node| node.id.starts_with(prefix))
            .collect()
    }

    #[must_use]
    pub fn children_of(&self, parent: &str) -> Vec<&Node> {
        self.nodes
            .iter()
            .filter(|node| node.parent.as_deref() == Some(parent))
            .collect()
    }

    #[must_use]
    pub fn descendants_of(&self, parent: &str) -> Vec<&Node> {
        let mut found = Vec::new();
        let mut frontier = vec![parent];
        let mut visited = BTreeSet::from([parent.to_string()]);
        while let Some(next) = frontier.pop() {
            for node in self.children_of(next) {
                if visited.insert(node.id.clone()) {
                    found.push(node);
                    frontier.push(&node.id);
                }
            }
        }
        found
    }

    /// The ids on this node's parent chain, nearest first, stopping at a
    /// missing parent or a cycle.
    #[must_use]
    pub fn ancestors_of(&self, id: &str) -> Vec<&str> {
        let mut chain = Vec::new();
        let mut visited = BTreeSet::from([id.to_string()]);
        let mut current = id;
        while let Some(parent) = self.find(current).and_then(|node| node.parent.as_deref()) {
            if !visited.insert(parent.to_string()) {
                break;
            }
            let Some(node) = self.find(parent) else {
                break;
            };
            chain.push(node.id.as_str());
            current = node.id.as_str();
        }
        chain
    }

    /// Re-applies redaction.
    ///
    /// The probe already redacts recorded text and values; this covers nodes a
    /// host constructed directly, and is idempotent.
    #[must_use]
    pub fn redacted(mut self) -> Self {
        for node in &mut self.nodes {
            node.redact();
        }
        for item in &mut self.reading_order {
            item.redact();
        }
        self
    }

    /// Whether walking this node's parent chain revisits an id.
    ///
    /// Separate from [`Snapshot::ancestors_of`], which stops at the repeat and
    /// reports the prefix: the caller there wants the chain it can trust, and
    /// this one wants to know the chain is untrustworthy.
    fn chain_repeats(&self, id: &str) -> bool {
        let mut visited = BTreeSet::from([id.to_string()]);
        let mut current = id;
        while let Some(parent) = self.find(current).and_then(|node| node.parent.as_deref()) {
            if !visited.insert(parent.to_string()) {
                return true;
            }
            current = parent;
        }
        false
    }

    /// Reports the defects one frame's tree carries.
    ///
    /// Ordering is deterministic: node findings in registration order (each
    /// node's malformed id, then its duplicate, then its orphan parent or the
    /// cycle its chain closes into), then
    /// reading-item findings in publication order (malformed id, then
    /// duplicate, then missing parent), then [`LintFinding::MissingWindow`]
    /// last. An empty result is the contract a surface's story test asserts.
    ///
    /// The two halves keep separate duplicate sets. A row that materializes
    /// between frames is legitimately both a reading item and a node, so an
    /// id appearing once in each is not a duplicate; the same id published
    /// twice inside either half is.
    #[must_use]
    pub fn lint(&self) -> Vec<LintFinding> {
        let mut findings = Vec::new();
        let published: HashSet<&str> = self.nodes.iter().map(|node| node.id.as_str()).collect();
        let mut seen: HashSet<&str> = HashSet::new();
        for node in &self.nodes {
            if !Ident::is_valid(&node.id) {
                findings.push(LintFinding::MalformedId {
                    id: node.id.clone(),
                });
            }
            if !seen.insert(node.id.as_str()) {
                findings.push(LintFinding::DuplicateId {
                    id: node.id.clone(),
                });
            }
            if let Some(parent) = &node.parent {
                if !published.contains(parent.as_str()) {
                    findings.push(LintFinding::OrphanParent {
                        id: node.id.clone(),
                        parent: parent.clone(),
                    });
                } else if self.chain_repeats(&node.id) {
                    findings.push(LintFinding::ParentCycle {
                        id: node.id.clone(),
                    });
                }
            }
        }
        let mut seen_rows: HashSet<&str> = HashSet::new();
        for item in &self.reading_order {
            if !Ident::is_valid(&item.id) {
                findings.push(LintFinding::MalformedId {
                    id: item.id.clone(),
                });
            }
            if !seen_rows.insert(item.id.as_str()) {
                findings.push(LintFinding::DuplicateId {
                    id: item.id.clone(),
                });
            }
            if let Some(parent) = &item.parent {
                if !published.contains(parent.as_str()) {
                    findings.push(LintFinding::ReadingItemParentMissing {
                        id: item.id.clone(),
                        parent: parent.clone(),
                    });
                }
            }
        }
        if !self.nodes.iter().any(|node| node.role == Role::Window) {
            findings.push(LintFinding::MissingWindow);
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::{LintFinding, Snapshot};
    use crate::semantics::node::{Node, Rect, SemanticReadingItem};
    use crate::semantics::role::Role;

    fn node(id: &str, parent: Option<&str>) -> Node {
        Node {
            id: id.into(),
            role: Role::Region,
            parent: parent.map(str::to_string),
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            },
            visible: true,
            ..Node::default()
        }
    }

    fn window(id: &str) -> Node {
        Node {
            role: Role::Window,
            ..node(id, None)
        }
    }

    #[test]
    fn descendants_follow_the_declared_parent_chain() {
        let snapshot = Snapshot {
            generation: 1,
            nodes: vec![
                node("root", None),
                node("child", Some("root")),
                node("grandchild", Some("child")),
            ],
            reading_order: Vec::new(),
        };
        assert_eq!(
            snapshot
                .descendants_of("root")
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            vec!["child", "grandchild"]
        );
    }

    #[test]
    fn descendants_do_not_loop_on_a_malformed_parent_cycle() {
        let snapshot = Snapshot {
            generation: 1,
            nodes: vec![node("a", Some("b")), node("b", Some("a"))],
            reading_order: Vec::new(),
        };
        assert_eq!(
            snapshot
                .descendants_of("a")
                .iter()
                .map(|node| node.id.as_str())
                .collect::<Vec<_>>(),
            vec!["b"]
        );
    }

    #[test]
    fn ancestors_stop_at_a_cycle() {
        let snapshot = Snapshot {
            generation: 1,
            nodes: vec![node("a", Some("b")), node("b", Some("a"))],
            reading_order: Vec::new(),
        };
        assert_eq!(snapshot.ancestors_of("a"), vec!["b"]);
    }

    #[test]
    fn adjacent_bounds_do_not_overlap() {
        let left = Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 100.0,
        };
        let right = Rect { x: 100.0, ..left };
        assert!(!left.overlaps(right));
    }

    #[test]
    fn snapshot_redaction_is_idempotent() {
        let snapshot = Snapshot {
            generation: 1,
            nodes: vec![Node {
                id: "secret".into(),
                text: Some("sk-live-key".into()),
                value: Some("Bearer abc".into()),
                ..Node::default()
            }],
            reading_order: Vec::new(),
        }
        .redacted();
        assert_eq!(
            snapshot.find("secret").unwrap().text.as_deref(),
            Some("[REDACTED]")
        );
        assert_eq!(
            snapshot.clone().redacted().find("secret").unwrap().text,
            snapshot.find("secret").unwrap().text
        );
    }

    #[test]
    fn a_well_formed_tree_lints_clean() {
        let snapshot = Snapshot {
            generation: 3,
            nodes: vec![window("composer"), node("composer-send", Some("composer"))],
            reading_order: vec![SemanticReadingItem {
                id: "thread.m4000".into(),
                parent: Some("composer".into()),
                role: Role::Row,
                ..SemanticReadingItem::default()
            }],
        };
        assert_eq!(snapshot.lint(), Vec::new());
    }

    #[test]
    fn lint_reports_orphan_duplicate_grammar_and_a_missing_window() {
        let snapshot = Snapshot {
            generation: 1,
            nodes: vec![
                node("Composer.Send", None),
                node("row", Some("nowhere")),
                node("row", None),
            ],
            reading_order: vec![SemanticReadingItem {
                id: "thread.m1".into(),
                parent: Some("gone".into()),
                ..SemanticReadingItem::default()
            }],
        };
        assert_eq!(
            snapshot.lint(),
            vec![
                LintFinding::MalformedId {
                    id: "Composer.Send".into()
                },
                LintFinding::OrphanParent {
                    id: "row".into(),
                    parent: "nowhere".into()
                },
                LintFinding::DuplicateId { id: "row".into() },
                LintFinding::ReadingItemParentMissing {
                    id: "thread.m1".into(),
                    parent: "gone".into()
                },
                LintFinding::MissingWindow,
            ]
        );
    }

    #[test]
    fn a_parent_chain_that_closes_on_itself_is_reported() {
        // Every declared parent is published, so `OrphanParent` sees nothing
        // wrong; the defect is that neither `panel` nor `row` reaches a root.
        let snapshot = Snapshot {
            generation: 1,
            nodes: vec![
                window("leaf"),
                node("panel", Some("row")),
                node("row", Some("panel")),
            ],
            reading_order: Vec::new(),
        };
        assert_eq!(
            snapshot.lint(),
            vec![
                LintFinding::ParentCycle {
                    id: "panel".into()
                },
                LintFinding::ParentCycle { id: "row".into() },
            ]
        );
        // The reason it matters: the chain a caller gets back is the prefix
        // before the repeat, not the whole chain, and nothing else says so.
        assert_eq!(snapshot.ancestors_of("row"), vec!["panel"]);
    }

    #[test]
    fn a_node_hanging_off_a_cycle_is_reported_too() {
        let snapshot = Snapshot {
            generation: 1,
            nodes: vec![
                window("leaf"),
                node("a", Some("b")),
                node("b", Some("a")),
                node("leaf-child", Some("leaf")),
                node("dangling", Some("a")),
            ],
            reading_order: Vec::new(),
        };
        assert_eq!(
            snapshot.lint(),
            vec![
                LintFinding::ParentCycle { id: "a".into() },
                LintFinding::ParentCycle { id: "b".into() },
                LintFinding::ParentCycle {
                    id: "dangling".into()
                },
            ]
        );
    }

    #[test]
    fn a_chain_that_ends_at_a_parentless_node_is_not_a_cycle() {
        // Most nodes in this repository declare no parent at all: they are in
        // the window their registry is for. A chain that terminates is fine
        // however short it is, and only a chain that never terminates is not.
        let snapshot = Snapshot {
            generation: 1,
            nodes: vec![
                window("leaf"),
                node("panel", None),
                node("row", Some("panel")),
            ],
            reading_order: Vec::new(),
        };
        assert_eq!(snapshot.lint(), Vec::new());
    }

    #[test]
    fn the_same_row_published_twice_is_a_duplicate() {
        // `canonical_hash` keeps every reading entry, so a frame that
        // publishes one row twice hashes differently from one that publishes
        // it once. Only the node half was checked for duplicates before, so
        // this frame linted clean while carrying an id an action could not
        // address unambiguously.
        let row = SemanticReadingItem {
            id: "thread.m1".into(),
            parent: Some("thread".into()),
            role: Role::Row,
            ..SemanticReadingItem::default()
        };
        let snapshot = Snapshot {
            generation: 1,
            nodes: vec![window("thread")],
            reading_order: vec![row.clone(), row],
        };
        assert_eq!(
            snapshot.lint(),
            vec![LintFinding::DuplicateId {
                id: "thread.m1".into()
            }]
        );
    }

    #[test]
    fn a_row_that_shares_an_id_with_a_node_is_not_a_duplicate() {
        // A row materializing between frames is legitimately both, so the two
        // halves keep separate duplicate sets.
        let snapshot = Snapshot {
            generation: 1,
            nodes: vec![window("thread"), node("thread.m1", Some("thread"))],
            reading_order: vec![SemanticReadingItem {
                id: "thread.m1".into(),
                parent: Some("thread".into()),
                role: Role::Row,
                ..SemanticReadingItem::default()
            }],
        };
        assert_eq!(snapshot.lint(), Vec::new());
    }
}
