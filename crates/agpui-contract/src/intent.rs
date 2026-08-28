//! What a leaf asks its host to do.
//!
//! The return channel of [`crate::thread`] and [`crate::composer`]. Documents
//! travel host to leaf and never come back; intents travel leaf to host and
//! carry no renderer state, because a host that had to understand how the
//! thread was drawn in order to act on it would be reading pixels.
//!
//! Both directions live in this crate for the same reason: the leaf and the
//! host are separate binaries built from separate workspaces, and a wire
//! vocabulary defined on one side of that seam is a vocabulary the other side
//! has to guess at. `Serialize` for the leaf, `Deserialize` for the host, one
//! definition for both.

use serde::{Deserialize, Serialize};

use crate::composer::{ComposerQueueLane, ComposerQuote, ComposerTriggerKind};

/// What the composer asks the host to do. The leaf never sends a run itself.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "intent", rename_all = "snake_case")]
pub enum ComposerIntent {
    /// Send this text. The leaf has already cleared its editor; if the host
    /// refuses, it restores the draft by bumping `draft_revision`.
    Submit {
        text: String,
        /// Optional quote metadata captured at the same instant as the text.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        quote: Option<ComposerQuote>,
        /// Queue into the interrupting lane rather than the ordinary FIFO lane.
        #[serde(default, skip_serializing_if = "is_false")]
        steer: bool,
    },
    /// Abort the live run.
    Cancel,
    /// Leave an edit composer without cancelling the thread's active run.
    EndEdit,
    /// The draft moved. The host persists it per scope; it is not a command.
    DraftChanged {
        text: String,
    },
    /// Open the host's file picker. The leaf has no file system.
    PickAttachment,
    RemoveAttachment {
        attachment_id: String,
    },
    /// Clear the quote without mutating the text draft.
    DismissQuote,
    MoveQueueItem {
        queue_item_id: String,
        lane: ComposerQueueLane,
    },
    RemoveQueueItem {
        queue_item_id: String,
    },
    /// A host-supplied slash command or mention was chosen. GPUI has already
    /// applied the item's editor-local insertion text.
    SelectTrigger {
        trigger_item_id: String,
        kind: ComposerTriggerKind,
    },
}

const fn is_false(value: &bool) -> bool {
    !*value
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

/// One drained batch from the thread leaf, tagged by which queue spoke.
///
/// The thread leaf owns two queues - the transcript's and the composer's - so
/// a drained payload has to say which one it came from. The drawer leaf raises
/// a bare command and needs no envelope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum LeafPayload {
    Thread { intents: Vec<ThreadIntent> },
    Composer { intents: Vec<ComposerIntent> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intents_are_internally_tagged_so_a_host_can_match_on_one_key() {
        let wire = serde_json::to_value(ComposerIntent::Submit {
            text: "hello".into(),
            quote: None,
            steer: false,
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

        let wire = serde_json::to_value(ComposerIntent::EndEdit).unwrap();
        assert_eq!(wire, serde_json::json!({"intent": "end_edit"}));
    }

    #[test]
    fn trigger_selection_returns_stable_identity_to_the_host() {
        let wire = serde_json::to_value(ComposerIntent::SelectTrigger {
            trigger_item_id: "mention-ada".to_owned(),
            kind: ComposerTriggerKind::Mention,
        })
        .unwrap();
        assert_eq!(wire["intent"], "select_trigger");
        assert_eq!(wire["trigger_item_id"], "mention-ada");
        assert_eq!(wire["kind"], "mention");
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
}
