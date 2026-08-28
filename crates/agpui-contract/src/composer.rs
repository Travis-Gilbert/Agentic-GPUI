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

/// Which composer owns the draft.
///
/// Thread and edit composers deliberately do not share cancellation or send
/// policy. Compact is a thread composer with a narrower presentation contract:
/// one line, no attachments, no quote, and no queue chrome.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComposerMode {
    #[default]
    Thread,
    Edit,
    Compact,
}

/// Keyboard submission policy selected by the host.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComposerSubmitMode {
    #[default]
    Enter,
    ControlEnter,
    None,
}

/// Runtime abilities, projected rather than inferred by the renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerCapabilities {
    #[serde(default)]
    pub cancel: bool,
    #[serde(default)]
    pub queue: bool,
    #[serde(default = "default_true")]
    pub attachments: bool,
}

impl Default for ComposerCapabilities {
    fn default() -> Self {
        Self {
            cancel: false,
            queue: false,
            // Compatibility with projections produced before capabilities
            // were explicit: those surfaces already exposed the attach button.
            attachments: true,
        }
    }
}

const fn default_true() -> bool {
    true
}

/// A quote attached to the next user message.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerQuote {
    pub message_id: String,
    pub text: String,
}

/// The queue lane is part of composer chrome, never part of transcript data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComposerQueueLane {
    #[default]
    Queue,
    Steer,
}

/// Renderer-facing identity and text for one pending send.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerQueueItem {
    pub queue_item_id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub lane: ComposerQueueLane,
}

/// Trigger family recognized by the native composer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComposerTriggerKind {
    Slash,
    Mention,
}

impl ComposerTriggerKind {
    #[must_use]
    pub const fn character(self) -> char {
        match self {
            Self::Slash => '/',
            Self::Mention => '@',
        }
    }
}

/// One host-supplied slash command or mention result.
///
/// The renderer may insert `insert_text`, but the stable ID is returned to the
/// host so execution never depends on reading editor text back out of GPUI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerTriggerItem {
    pub trigger_item_id: String,
    pub kind: ComposerTriggerKind,
    pub label: String,
    #[serde(default)]
    pub detail: String,
    #[serde(default)]
    pub insert_text: String,
}

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
    #[serde(default)]
    pub mode: ComposerMode,
    #[serde(default)]
    pub submit_mode: ComposerSubmitMode,
    #[serde(default)]
    pub capabilities: ComposerCapabilities,
    /// Host-owned send disablement. Edit composers intentionally ignore it.
    #[serde(default)]
    pub is_send_disabled: bool,
    /// True from send admission until every attachment/upload sibling settles.
    #[serde(default)]
    pub is_sending: bool,
    /// Some runtimes settle their top-level run flag before the trailing
    /// assistant message; cancellation remains available until both settle.
    #[serde(default)]
    pub trailing_assistant_running: bool,
    #[serde(default)]
    pub quote: Option<ComposerQuote>,
    #[serde(default)]
    pub queue: Vec<ComposerQueueItem>,
    #[serde(default)]
    pub trigger_items: Vec<ComposerTriggerItem>,
    /// Bumped for a host-authorized focus event (mount, thread switch,
    /// run-start, or scroll-to-bottom). Edit composers ignore it.
    #[serde(default)]
    pub focus_revision: u64,
    /// Host modality projection; GPUI has no CSS media query to infer this.
    #[serde(default)]
    pub touch_primary: bool,
    /// Native picker/drop accept expression. Empty means unrestricted.
    #[serde(default)]
    pub attachment_accept: String,
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
    /// 1. **A host refusal outranks anything the draft says.** The user can
    ///    type into an expired session; they cannot send from one.
    /// 2. **The in-flight transaction lock outranks a second send.** It stays
    ///    set until all sibling attachment operations settle.
    /// 3. **Failed outranks pending.** They need different actions - remove
    ///    versus wait — so reporting the weaker one hides the actionable one.
    ///    Same fold as a failed part dominating a turn's status.
    /// 4. **Empty precedes runtime policy**, because it is the only block the
    ///    user clears by
    ///    typing, and reporting it while an upload is still running would be a
    ///    lie about what is holding the send.
    /// 5. **A live run blocks only without queue capability.** Cancellation is
    ///    a separate control; with queue capability, the send remains a send.
    #[must_use]
    pub fn affordance(&self, live_draft: &str) -> SendAffordance {
        if !self.refusal.trim().is_empty() {
            return SendAffordance::Blocked(SendBlock::Refused);
        }
        if self.is_sending {
            return SendAffordance::Blocked(SendBlock::InFlight);
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
        if self.mode != ComposerMode::Edit && self.is_send_disabled {
            return SendAffordance::Blocked(SendBlock::Disabled);
        }
        if self.run_state.is_live() && !self.allows_queue() {
            return SendAffordance::Blocked(SendBlock::RunActive);
        }
        SendAffordance::Send
    }

    /// Whether Escape/the stop control may cancel right now.
    #[must_use]
    pub const fn can_cancel(&self) -> bool {
        if matches!(self.mode, ComposerMode::Edit) {
            return true;
        }
        self.capabilities.cancel && (self.run_state.is_live() || self.trailing_assistant_running)
    }

    #[must_use]
    pub const fn allows_queue(&self) -> bool {
        self.capabilities.queue && matches!(self.mode, ComposerMode::Thread)
    }

    #[must_use]
    pub const fn allows_attachments(&self) -> bool {
        self.capabilities.attachments && !matches!(self.mode, ComposerMode::Compact)
    }

    #[must_use]
    pub const fn allows_quote(&self) -> bool {
        !matches!(self.mode, ComposerMode::Compact)
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
    InFlight,
    Disabled,
    RunActive,
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
            Self::InFlight => "Sending the current message",
            Self::Disabled => "Sending is disabled",
            Self::RunActive => "Wait for the current response",
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
    fn a_live_run_blocks_send_without_queue_and_cancel_is_separate() {
        for state in [ThreadRunState::Streaming, ThreadRunState::RequiresApproval] {
            let mut subject = document();
            subject.run_state = state;
            subject.capabilities.cancel = true;
            assert_eq!(
                subject.affordance("a whole new question"),
                SendAffordance::Blocked(SendBlock::RunActive)
            );
            assert!(subject.can_cancel());
        }

        let mut trailing = document();
        trailing.capabilities.cancel = true;
        trailing.trailing_assistant_running = true;
        assert!(trailing.can_cancel());

        let mut idle = document();
        idle.capabilities.cancel = true;
        assert!(!idle.can_cancel());
    }

    #[test]
    fn queue_capability_keeps_send_live_during_a_run() {
        let mut subject = document();
        subject.run_state = ThreadRunState::Streaming;
        subject.capabilities.queue = true;
        assert_eq!(subject.affordance("next"), SendAffordance::Send);
    }

    #[test]
    fn in_flight_send_lock_beats_a_second_send() {
        let mut subject = document();
        subject.is_sending = true;
        assert_eq!(
            subject.affordance("second"),
            SendAffordance::Blocked(SendBlock::InFlight)
        );
    }

    #[test]
    fn edit_ignores_thread_disable_and_escape_exits_edit() {
        let mut subject = document();
        subject.mode = ComposerMode::Edit;
        subject.is_send_disabled = true;
        assert_eq!(subject.affordance("replacement"), SendAffordance::Send);
        assert!(subject.can_cancel());
    }

    #[test]
    fn quote_only_is_empty_and_compact_removes_expansive_chrome() {
        let mut subject = document();
        subject.quote = Some(ComposerQuote {
            message_id: "m1".to_owned(),
            text: "quoted".to_owned(),
        });
        assert_eq!(
            subject.affordance(""),
            SendAffordance::Blocked(SendBlock::Empty)
        );
        subject.mode = ComposerMode::Compact;
        assert!(!subject.allows_attachments());
        assert!(!subject.allows_quote());
        assert!(!subject.allows_queue());
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
