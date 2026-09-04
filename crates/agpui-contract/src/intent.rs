//! What a leaf asks its host to do.
//!
//! The return channel of [`crate::thread`], [`crate::composer`], and the native
//! document editor. Projection documents travel host to leaf and never come
//! back; intents travel leaf to host and carry no renderer state, because a
//! host that had to understand how a surface was drawn in order to act on it
//! would be reading pixels.
//!
//! Both directions live in this crate for the same reason: the leaf and the
//! host are separate binaries built from separate workspaces, and a wire
//! vocabulary defined on one side of that seam is a vocabulary the other side
//! has to guess at. `Serialize` for the leaf, `Deserialize` for the host, one
//! definition for both.

use serde::{Deserialize, Serialize};

/// What the composer asks the host to do. The leaf never sends a run itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "intent", rename_all = "snake_case")]
pub enum ComposerIntent {
    /// Send this text. The leaf has already cleared its editor; if the host
    /// refuses, it restores the draft by bumping `draft_revision`.
    Submit {
        text: String,
    },
    /// Abort the live run.
    Cancel,
    /// The draft moved. The host persists it per scope; it is not a command.
    DraftChanged {
        text: String,
    },
    /// Open the host's file picker. The leaf has no file system.
    PickAttachment,
    /// Open Settings through the host's renderer-neutral application reducer.
    OpenSettings,
    RemoveAttachment {
        attachment_id: String,
    },
}

/// Something the reader asked for that only the host can do.
///
/// Every variant names the wire identity the host needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "intent", rename_all = "snake_case")]
pub enum ThreadIntent {
    /// Answer an open approval. `approved` is the reader's literal answer; the
    /// leaf never supplies one on their behalf.
    Approval { approval_id: String, approved: bool },
    /// Run a failed tool call again.
    RetryTool { tool_call_id: String },
    /// Mount an artifact in the right drawer.
    OpenArtifact {
        artifact_id: String,
        artifact_revision: String,
    },
    /// Focus a graph selection in the drawer's graph leaf.
    FocusGraph {
        program_id: String,
        graph_revision: String,
        node_refs: Vec<String>,
    },
    /// Open a cited source.
    OpenCitation { source_id: String },
}

/// A canonical document change the GPUI editor asks its host to commit.
///
/// The document leaf owns transient editor entities and a local Loro working
/// session. The product host remains the durable authority, so a canonical
/// snapshot or an optimistic code proposal crosses this renderer-neutral
/// contract instead of the leaf calling a graph API itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DocumentIntent {
    /// Authored prose changed and the complete canonical Loro snapshot is
    /// ready for the host's revision endpoint.
    CanonicalChanged {
        document_id: String,
        block_id: String,
        snapshot: Vec<u8>,
        /// Complete agent-readable projection of the same post-edit state.
        markdown_projection: String,
    },
    /// A code editor proposes a replacement against the exact text it opened.
    /// The filesystem or Git remains write authority and may refuse it.
    ProposeCodeEdit {
        document_id: String,
        block_id: String,
        expected: String,
        replacement: String,
    },
}

/// One drained batch from a leaf, tagged by which queue spoke.
///
/// The thread leaf owns two queues - the transcript's and the composer's - so
/// a drained payload has to say which one it came from. The drawer leaf raises
/// a typed canvas command and uses the same envelope so the host can match on
/// `source`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum LeafPayload {
    Thread { intents: Vec<ThreadIntent> },
    Composer { intents: Vec<ComposerIntent> },
    Drawer { command: serde_json::Value },
    Document { intents: Vec<DocumentIntent> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intents_are_internally_tagged_so_a_host_can_match_on_one_key() {
        let wire = serde_json::to_value(ComposerIntent::Submit {
            text: "hello".into(),
        })
        .unwrap();
        assert_eq!(wire["intent"], "submit");
        assert_eq!(wire["text"], "hello");

        let wire = serde_json::to_value(ThreadIntent::Approval {
            approval_id: "a1".into(),
            approved: true,
        })
        .unwrap();
        assert_eq!(wire["intent"], "approval");
        assert_eq!(wire["approved"], true);
    }

    #[test]
    fn a_fieldless_intent_still_round_trips_as_an_object() {
        // `Cancel` has no fields, and an externally tagged enum would render
        // it as the bare string "cancel" - which a host matching on `.intent`
        // could not read.
        let wire = serde_json::to_value(ComposerIntent::Cancel).unwrap();
        assert_eq!(wire, serde_json::json!({"intent": "cancel"}));
        assert_eq!(
            serde_json::from_value::<ComposerIntent>(wire).unwrap(),
            ComposerIntent::Cancel
        );

        let wire = serde_json::to_value(ComposerIntent::OpenSettings).unwrap();
        assert_eq!(wire, serde_json::json!({"intent": "open_settings"}));
    }

    #[test]
    fn a_drained_payload_names_the_queue_that_spoke() {
        let payload = LeafPayload::Composer {
            intents: vec![ComposerIntent::Cancel],
        };
        let wire = serde_json::to_value(&payload).unwrap();
        assert_eq!(wire["source"], "composer");
        assert_eq!(
            serde_json::from_value::<LeafPayload>(wire).unwrap(),
            payload
        );
    }

    #[test]
    fn a_drawer_command_arrives_in_the_same_envelope() {
        let payload = LeafPayload::Drawer {
            command: serde_json::json!({
                "schema": "theorem-surface-command/1",
                "action": "select",
                "programId": "program-1"
            }),
        };
        let wire = serde_json::to_value(&payload).unwrap();
        assert_eq!(wire["source"], "drawer");
        assert_eq!(wire["command"]["action"], "select");
        assert_eq!(
            serde_json::from_value::<LeafPayload>(wire).unwrap(),
            payload
        );
    }

    #[test]
    fn document_edits_cross_the_leaf_boundary_as_typed_payloads() {
        let payload = LeafPayload::Document {
            intents: vec![DocumentIntent::CanonicalChanged {
                document_id: "note:one".into(),
                block_id: "block:body".into(),
                snapshot: vec![0, 1, 2, 255],
                markdown_projection: "# One\n\nEdited body.".into(),
            }],
        };

        let wire = serde_json::to_value(&payload).unwrap();
        assert_eq!(wire["source"], "document");
        assert_eq!(wire["intents"][0]["kind"], "canonical_changed");
        assert_eq!(wire["intents"][0]["document_id"], "note:one");
        assert_eq!(
            wire["intents"][0]["markdown_projection"],
            "# One\n\nEdited body."
        );
        assert_eq!(
            serde_json::from_value::<LeafPayload>(wire).unwrap(),
            payload
        );
    }
}
