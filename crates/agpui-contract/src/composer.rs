//! Renderer-neutral projection of the composer.
//!
//! Sibling of [`crate::thread`], and the same one-way rule applies: the host
//! folds its authoritative state *into* these shapes and nothing folds back.
//! `theoremweb-agent-runtime::ComposerState` remains the owner of drafts,
//! attachment records, and the run command; this crate carries only what a
//! renderer needs to draw the band and decide what the primary button does.
//!
//! # Why the send rule lives here
//!
//! Send-readiness has two inputs that live in different places. Attachment
//! readiness and run state are the host's; the live draft is the editor's,
//! because the editor owns the text between keystrokes and a round trip per
//! character would leave the button one frame behind the cursor.
//!
//! Splitting the *rule* across those two places is how a surface ends up with a
//! button that lights up when the host says no. So the rule is written once,
//! here, as [`ComposerDocument::affordance`], and takes the live draft as an
//! argument. Both sides call it; neither re-derives it.

use serde::{Deserialize, Serialize};

use crate::thread::ThreadRunState;

/// Wire schema identifier carried on every projected composer.
pub const COMPOSER_PROJECTION_SCHEMA: &str = "theorem-composer-projection/1";

/// The composer band, as the leaf receives it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerDocument {
    /// Always [`COMPOSER_PROJECTION_SCHEMA`]; a mismatch is a refusal.
    pub schema: String,
    pub thread_id: String,
    /// The host's persisted draft for this scope.
    #[serde(default)]
    pub draft: String,
    /// Bumped by the host every time it means [`Self::draft`] to *replace*
    /// what the editor holds.
    ///
    /// Without this the leaf has to guess, and both guesses are wrong. Taking
    /// the host's draft on every projection would overwrite the user's
    /// keystrokes each time an upload reported progress. Taking it only when
    /// the thread changes loses the case that matters most: a send that the
    /// host rejects, where the host still has the text and the editor has
    /// already cleared itself. So the host says when it means it, and the leaf
    /// obeys exactly then.
    #[serde(default)]
    pub draft_revision: u64,
    /// Empty falls back to the leaf's own default, so a host that ships no
    /// copy still renders a prompt rather than a blank field.
    #[serde(default)]
    pub placeholder: String,
    #[serde(default)]
    pub run_state: ThreadRunState,
    /// Carried from the host's `prefers-reduced-motion` across the mode
    /// bridge. The leaf cannot read the media query itself.
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default)]
    pub attachments: Vec<ComposerAttachment>,
    /// Non-empty when the host refuses composition outright — expired
    /// identity, a read-only thread, a revoked scope. The string is printed
    /// verbatim where the controls would be, because a composer that is merely
    /// inert reads as a bug and a composer that explains itself does not.
    #[serde(default)]
    pub refusal: String,
}

impl ComposerDocument {
    /// Refuse a document whose schema string is not the pinned one.
    ///
    /// # Errors
    ///
    /// Returns the offending schema string so the caller reports a
    /// deterministic contract error instead of drawing a partial composer.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != COMPOSER_PROJECTION_SCHEMA {
            return Err(format!(
                "composer projection schema is {:?}, expected {COMPOSER_PROJECTION_SCHEMA:?}",
                self.schema
            ));
        }
        Ok(())
    }

    /// What the primary button does, given the text the editor currently holds.
    ///
    /// The order of the guards is the whole content of this function, so it is
    /// spelled out rather than left to reading order:
    ///
    /// 1. **A live run wins.** While the model is producing, or while it is
    ///    parked on an approval, the only thing the primary button may do is
    ///    stop it. Queueing a second message into a live run has no
    ///    representation on `theorem-chat/1` and would be a message the host
    ///    could not send.
    /// 2. **A host refusal outranks anything the draft says.** The user can
    ///    type into an expired session; they cannot send from one.
    /// 3. **Failed outranks pending.** They need different actions — remove
    ///    versus wait — so reporting the weaker one hides the actionable one.
    ///    Same fold as a failed part dominating a turn's status.
    /// 4. **Empty is last**, because it is the only block the user clears by
    ///    typing, and reporting it while an upload is still running would be a
    ///    lie about what is holding the send.
    #[must_use]
    pub fn affordance(&self, live_draft: &str) -> SendAffordance {
        if self.run_state.is_live() {
            return SendAffordance::Stop;
        }
        if !self.refusal.trim().is_empty() {
            return SendAffordance::Blocked(SendBlock::Refused);
        }
        if self
            .attachments
            .iter()
            .any(|attachment| attachment.state.has_failed())
        {
            return SendAffordance::Blocked(SendBlock::AttachmentFailed);
        }
        if self
            .attachments
            .iter()
            .any(|attachment| !attachment.state.is_send_ready())
        {
            return SendAffordance::Blocked(SendBlock::AttachmentPending);
        }
        if live_draft.trim().is_empty() && self.attachments.is_empty() {
            return SendAffordance::Blocked(SendBlock::Empty);
        }
        SendAffordance::Send
    }
}

/// One attachment chip.
///
/// Field-for-field the renderer-facing half of
/// `theoremweb-agent-runtime::AttachmentRecord`. Byte identity, admission
/// receipts, and provenance stay on the host: a renderer that could read them
/// would eventually be asked to decide with them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerAttachment {
    pub attachment_id: String,
    pub name: String,
    #[serde(default)]
    pub media_type: String,
    #[serde(default)]
    pub state: AttachmentState,
}

/// Lifecycle of one attachment, mirroring the runtime's own vocabulary.
///
/// `Removed` has no variant here on purpose: a removed record never projects,
/// so a renderer cannot be handed one and cannot decide what to do with it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum AttachmentState {
    #[default]
    Pending,
    Uploading {
        progress_percent: u8,
    },
    Scanning,
    Extracting,
    Ready,
    Failed {
        message: String,
    },
}

impl AttachmentState {
    /// Whether this attachment may ride along on a run request.
    #[must_use]
    pub const fn is_send_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    #[must_use]
    pub const fn has_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// The line printed under the file name.
    ///
    /// A ready attachment gets nothing: readiness is the common case and
    /// labelling it puts a badge on every chip that is working correctly. A
    /// failed one prints the host's own message, in place, rather than a
    /// generic word — the same rule the approval and thinking surfaces follow.
    #[must_use]
    pub fn detail(&self) -> Option<String> {
        match self {
            Self::Ready => None,
            Self::Pending => Some("Queued".to_owned()),
            Self::Uploading { progress_percent } => Some(format!("Uploading {progress_percent}%")),
            Self::Scanning => Some("Scanning".to_owned()),
            Self::Extracting => Some("Reading".to_owned()),
            Self::Failed { message } => Some(message.clone()),
        }
    }

    /// Fraction of the upload bar to fill, when there is a bar to fill.
    #[must_use]
    pub fn progress(&self) -> Option<f32> {
        match self {
            Self::Uploading { progress_percent } => {
                Some(f32::from((*progress_percent).min(100)) / 100.0)
            }
            _ => None,
        }
    }
}

/// What the composer's primary button does right now.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SendAffordance {
    Send,
    Blocked(SendBlock),
    /// A run is live; the button aborts it.
    Stop,
}

impl SendAffordance {
    #[must_use]
    pub const fn is_send(self) -> bool {
        matches!(self, Self::Send)
    }

    /// Accessible label for the button in this state.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Send => "Send",
            Self::Stop => "Stop",
            Self::Blocked(_) => "Send unavailable",
        }
    }
}

/// Why send is refused. Every variant names a different next action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SendBlock {
    /// The host refuses composition; its own reason is on the document.
    Refused,
    AttachmentFailed,
    AttachmentPending,
    Empty,
}

impl SendBlock {
    /// What the user would do about it.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Refused => "This thread cannot accept messages",
            Self::AttachmentFailed => "Remove the attachment that failed",
            Self::AttachmentPending => "Waiting for an attachment",
            Self::Empty => "Write a message",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> ComposerDocument {
        ComposerDocument {
            schema: COMPOSER_PROJECTION_SCHEMA.to_owned(),
            thread_id: "thread-1".to_owned(),
            ..ComposerDocument::default()
        }
    }

    fn attachment(state: AttachmentState) -> ComposerAttachment {
        ComposerAttachment {
            attachment_id: "a1".to_owned(),
            name: "notes.pdf".to_owned(),
            media_type: "application/pdf".to_owned(),
            state,
        }
    }

    #[test]
    fn a_foreign_schema_is_refused_rather_than_rendered() {
        let mut subject = document();
        subject.schema = "composer/2".to_owned();
        assert!(subject.validate().is_err());
        assert!(document().validate().is_ok());
    }

    #[test]
    fn an_empty_draft_cannot_be_sent() {
        assert_eq!(
            document().affordance("   \n  "),
            SendAffordance::Blocked(SendBlock::Empty)
        );
    }

    #[test]
    fn an_attachment_alone_is_enough_to_send() {
        let mut subject = document();
        subject.attachments.push(attachment(AttachmentState::Ready));
        assert_eq!(
            subject.affordance(""),
            SendAffordance::Send,
            "a file with no covering note is a message"
        );
    }

    #[test]
    fn a_live_run_turns_the_button_into_stop_whatever_the_draft_says() {
        for state in [ThreadRunState::Streaming, ThreadRunState::RequiresApproval] {
            let mut subject = document();
            subject.run_state = state;
            assert_eq!(subject.affordance(""), SendAffordance::Stop);
            assert_eq!(
                subject.affordance("a whole new question"),
                SendAffordance::Stop
            );
        }
    }

    #[test]
    fn a_settled_run_returns_the_button_to_send() {
        for state in [
            ThreadRunState::Idle,
            ThreadRunState::Failed,
            ThreadRunState::Aborted,
        ] {
            let mut subject = document();
            subject.run_state = state;
            assert_eq!(
                subject.affordance("again"),
                SendAffordance::Send,
                "{state:?} is over; the user is allowed to try again"
            );
        }
    }

    #[test]
    fn a_host_refusal_outranks_a_perfectly_good_draft() {
        let mut subject = document();
        subject.refusal = "Your session expired".to_owned();
        assert_eq!(
            subject.affordance("ready to go"),
            SendAffordance::Blocked(SendBlock::Refused)
        );
    }

    #[test]
    fn a_failed_attachment_outranks_a_pending_one() {
        let mut subject = document();
        subject.attachments = vec![
            attachment(AttachmentState::Uploading {
                progress_percent: 40,
            }),
            attachment(AttachmentState::Failed {
                message: "Too large".to_owned(),
            }),
        ];
        assert_eq!(
            subject.affordance("hello"),
            SendAffordance::Blocked(SendBlock::AttachmentFailed),
            "reporting the upload hides the one the user can actually act on"
        );
    }

    #[test]
    fn every_unready_state_holds_the_send() {
        for state in [
            AttachmentState::Pending,
            AttachmentState::Uploading {
                progress_percent: 99,
            },
            AttachmentState::Scanning,
            AttachmentState::Extracting,
        ] {
            let mut subject = document();
            subject.attachments = vec![attachment(state.clone())];
            assert!(
                !subject.affordance("hello").is_send(),
                "{state:?} would send bytes the host has not admitted"
            );
        }
    }

    #[test]
    fn a_ready_attachment_carries_no_badge() {
        assert!(
            AttachmentState::Ready.detail().is_none(),
            "labelling the working case puts a badge on every healthy chip"
        );
        assert!(AttachmentState::Pending.detail().is_some());
    }

    #[test]
    fn a_failure_prints_the_hosts_own_words() {
        let state = AttachmentState::Failed {
            message: "Archives are not accepted".to_owned(),
        };
        assert_eq!(state.detail().as_deref(), Some("Archives are not accepted"));
    }

    #[test]
    fn only_an_upload_has_a_progress_bar() {
        assert_eq!(
            AttachmentState::Uploading {
                progress_percent: 50
            }
            .progress(),
            Some(0.5)
        );
        assert_eq!(AttachmentState::Scanning.progress(), None);
        assert_eq!(
            AttachmentState::Uploading {
                progress_percent: 200
            }
            .progress(),
            Some(1.0),
            "a bar past its own end draws outside the chip"
        );
    }

    #[test]
    fn every_block_names_a_different_next_action() {
        let reasons = [
            SendBlock::Refused,
            SendBlock::AttachmentFailed,
            SendBlock::AttachmentPending,
            SendBlock::Empty,
        ]
        .map(SendBlock::reason);
        let mut unique = reasons.to_vec();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            reasons.len(),
            "two blocks reading the same leaves the user without a next move"
        );
    }

    #[test]
    fn the_wire_stays_tagged_and_camel_cased() {
        let mut subject = document();
        subject.attachments = vec![attachment(AttachmentState::Uploading {
            progress_percent: 12,
        })];
        let wire = serde_json::to_value(&subject).unwrap();
        assert_eq!(wire["schema"], COMPOSER_PROJECTION_SCHEMA);
        assert_eq!(wire["attachments"][0]["attachmentId"], "a1");
        assert_eq!(wire["attachments"][0]["state"]["state"], "uploading");
        assert_eq!(wire["attachments"][0]["state"]["progressPercent"], 12);
        let back: ComposerDocument = serde_json::from_value(wire).unwrap();
        assert_eq!(back, subject);
    }

    #[test]
    fn a_draft_revision_survives_the_wire() {
        let mut subject = document();
        subject.draft = "half a question".to_owned();
        subject.draft_revision = 7;
        let wire = serde_json::to_value(&subject).unwrap();
        assert_eq!(wire["draftRevision"], 7);
        let back: ComposerDocument = serde_json::from_value(wire).unwrap();
        assert_eq!(back.draft_revision, 7);
    }

    #[test]
    fn an_absent_optional_field_does_not_break_the_parse() {
        let minimal = serde_json::json!({
            "schema": COMPOSER_PROJECTION_SCHEMA,
            "threadId": "thread-1",
        });
        let parsed: ComposerDocument = serde_json::from_value(minimal).unwrap();
        assert!(parsed.validate().is_ok());
        assert_eq!(parsed.run_state, ThreadRunState::Idle);
        assert_eq!(
            parsed.affordance(""),
            SendAffordance::Blocked(SendBlock::Empty)
        );
    }
}
