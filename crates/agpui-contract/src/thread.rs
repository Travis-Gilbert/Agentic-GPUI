//! Renderer-neutral projection of a `theorem-chat/1` thread.
//!
//! This is a *projection*, not a second part schema. `theoremweb-agent-runtime`
//! owns the authoritative `KnownMessagePart` vocabulary and folds it into these
//! shapes; nothing here parses a stream, holds run state, or decides policy. It
//! exists because SPEC-THEOREM-AGENT-SHELL-1.1 moves the thread into a GPUI
//! leaf, and the leaf must not depend on the Leptos browser runtime. Both
//! renderers therefore agree here, in a crate that depends on neither.
//!
//! The direction of authority is one-way and must stay that way. The runtime
//! projects *into* these types. Nothing projects back, and no field here may
//! acquire meaning the part schema does not already give it.

use serde::{Deserialize, Serialize};

/// Wire schema identifier carried on every projected document.
pub const THREAD_PROJECTION_SCHEMA: &str = "theorem-thread-projection/1";

/// One rendered thread, as the leaf receives it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadDocument {
    /// Always [`THREAD_PROJECTION_SCHEMA`]; a mismatch is a refusal, never a
    /// best-effort render.
    pub schema: String,
    pub thread_id: String,
    #[serde(default)]
    pub title: String,
    pub run_state: ThreadRunState,
    /// Carried from the host's `prefers-reduced-motion`, across the mode
    /// bridge. The leaf cannot read the media query itself.
    #[serde(default)]
    pub reduced_motion: bool,
    /// True while the viewport is pinned to the bottom, so the leaf knows
    /// whether an arriving part should auto-scroll or raise jump-to-latest.
    #[serde(default = "default_true")]
    pub pinned_to_bottom: bool,
    #[serde(default)]
    pub messages: Vec<ThreadMessage>,
}

const fn default_true() -> bool {
    true
}

impl ThreadDocument {
    /// Refuse a document whose schema string is not the pinned one.
    ///
    /// # Errors
    ///
    /// Returns the offending schema string so the caller can report a
    /// deterministic contract error instead of rendering a partial thread.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != THREAD_PROJECTION_SCHEMA {
            return Err(format!(
                "thread projection schema is {:?}, expected {THREAD_PROJECTION_SCHEMA:?}",
                self.schema
            ));
        }
        Ok(())
    }

    /// The message the streaming-text effect should animate, if any.
    ///
    /// Only the final assistant message can stream, and only while a text part
    /// on it is still running. Returning the index rather than a flag keeps the
    /// decision in one place instead of spread across the renderer.
    #[must_use]
    pub fn streaming_message(&self) -> Option<usize> {
        let index = self.messages.len().checked_sub(1)?;
        let message = self.messages.get(index)?;
        message
            .parts
            .iter()
            .any(|part| {
                matches!(
                    part,
                    ThreadPart::Text {
                        status: PartStatus::Running,
                        ..
                    }
                )
            })
            .then_some(index)
    }
}

/// Run lifecycle as the status bar and presence mark read it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadRunState {
    #[default]
    Idle,
    Streaming,
    RequiresApproval,
    Failed,
    Aborted,
}

impl ThreadRunState {
    /// Whether the run is still the model's to finish.
    ///
    /// `RequiresApproval` counts as live: the run has not ended, it is parked
    /// waiting on an answer, and a surface that treats it as settled will
    /// offer to start a second run alongside the first.
    #[must_use]
    pub const fn is_live(self) -> bool {
        matches!(self, Self::Streaming | Self::RequiresApproval)
    }
}

/// Actor grammar from SPEC-THEOREM-AGENT-SHELL-1.0: You clay, Theorem teal,
/// Insight gold. The renderer maps these to tokens; it never picks a colour.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadActor {
    User,
    #[default]
    Theorem,
    Insight,
    System,
}

impl ThreadActor {
    /// Display name shown beside the turn.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::User => "You",
            Self::Theorem => "Theorem",
            Self::Insight => "Insight",
            Self::System => "System",
        }
    }

    /// Single-glyph avatar seed. Kept here so both renderers agree.
    #[must_use]
    pub const fn initial(self) -> &'static str {
        match self {
            Self::User => "Y",
            Self::Theorem => "T",
            Self::Insight => "I",
            Self::System => "S",
        }
    }
}

/// One turn on the field. Not a card: the renderer draws no per-message
/// container, per the 1.0 thread section.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadMessage {
    pub message_id: String,
    pub actor: ThreadActor,
    #[serde(default)]
    pub parts: Vec<ThreadPart>,
}

/// Lifecycle of a single part, mirroring `MessagePartStatus`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartStatus {
    Pending,
    Running,
    RequiresAction,
    Open,
    #[default]
    Complete,
    Incomplete,
    Failed,
}

impl PartStatus {
    /// Whether this part is still moving, which gates every motion effect.
    #[must_use]
    pub const fn is_live(self) -> bool {
        matches!(self, Self::Pending | Self::Running | Self::Open)
    }
}

/// A projected part. Variant set is a strict image of `KnownMessagePart`, plus
/// [`ThreadPart::Code`], which is not a new part type but a *fold* of fenced
/// blocks already inside a text part, lifted out at projection time so the leaf
/// renders code through `CodeView` instead of re-parsing Markdown per frame.
/// `rename_all` renames *variants*; `rename_all_fields` renames the fields
/// *inside* struct variants, and serde will not infer the second from the
/// first. Without it the struct variants below would emit `part_id` while the
/// newtype variants -- which carry their own `rename_all` -- emit `partId`, so
/// one document would speak two casings. `theorem-chat/1` is camelCase on the
/// wire (`ChatStoredPart`), and this projection follows it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ThreadPart {
    Text {
        part_id: String,
        text: String,
        #[serde(default)]
        status: PartStatus,
    },
    Code {
        part_id: String,
        /// Empty when the fence carried no language. The renderer shows it
        /// plain rather than guessing, matching the CodeView reference.
        #[serde(default)]
        language: String,
        #[serde(default)]
        lines: Vec<CodeLine>,
        /// The raw text, so Copy yields exactly what the model wrote.
        #[serde(default)]
        source: String,
    },
    Reasoning(ReasoningPart),
    Progress {
        part_id: String,
        label: String,
        #[serde(default)]
        status: PartStatus,
    },
    ToolCall {
        part_id: String,
        tool_call_id: String,
        tool_name: String,
        #[serde(default)]
        status: PartStatus,
        /// Already redacted upstream. The leaf never sees raw arguments.
        #[serde(default)]
        arguments_preview: String,
    },
    ToolResult {
        part_id: String,
        tool_call_id: String,
        #[serde(default)]
        status: PartStatus,
        #[serde(default)]
        output_preview: String,
    },
    Approval(ApprovalPart),
    Citation {
        part_id: String,
        source_id: String,
        label: String,
    },
    Attachment {
        part_id: String,
        name: String,
        media_type: String,
        state: String,
    },
    Artifact {
        part_id: String,
        artifact_id: String,
        artifact_revision: String,
        artifact_kind: String,
        renderer_id: String,
        fallback_text: String,
    },
    GraphSelection {
        part_id: String,
        program_id: String,
        graph_revision: String,
        #[serde(default)]
        node_refs: Vec<String>,
    },
    Error {
        part_id: String,
        code: String,
        message: String,
        #[serde(default)]
        recoverable: bool,
    },
    Usage {
        part_id: String,
        input_tokens: u64,
        output_tokens: u64,
        model: String,
    },
    /// A part the runtime retained byte-for-byte but this projection does not
    /// name. It renders as an inert notice and acquires no authority.
    Unknown {
        part_id: String,
        variant: String,
    },
}

impl ThreadPart {
    /// Stable identity, used as the GPUI element id so state survives re-render.
    #[must_use]
    pub fn part_id(&self) -> &str {
        match self {
            Self::Text { part_id, .. }
            | Self::Code { part_id, .. }
            | Self::Progress { part_id, .. }
            | Self::ToolCall { part_id, .. }
            | Self::ToolResult { part_id, .. }
            | Self::Citation { part_id, .. }
            | Self::Attachment { part_id, .. }
            | Self::Artifact { part_id, .. }
            | Self::GraphSelection { part_id, .. }
            | Self::Error { part_id, .. }
            | Self::Usage { part_id, .. }
            | Self::Unknown { part_id, .. } => part_id,
            Self::Reasoning(reasoning) => &reasoning.part_id,
            Self::Approval(approval) => &approval.part_id,
        }
    }
}

/// One approval request, in whichever state the host last reported it.
///
/// Field-for-field the upstream `KnownMessagePart::ToolApproval`. The renderer
/// reads [`ApprovalPart::outcome`] rather than the raw pair, because
/// `(status, approved)` has combinations that mean nothing and a renderer that
/// branches on them directly will eventually render one of them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalPart {
    pub part_id: String,
    pub approval_id: String,
    pub tool_call_id: String,
    /// The action in the host's own words, used as the prompt title.
    pub summary: String,
    pub risk: String,
    #[serde(default)]
    pub status: PartStatus,
    #[serde(default)]
    pub approved: Option<bool>,
    /// Why it ended this way, when the host said. For an unanswered request
    /// this is the only place the difference between "it expired" and "a later
    /// request replaced it" survives, so a renderer must show it verbatim.
    #[serde(default)]
    pub reason: Option<String>,
}

impl ApprovalPart {
    /// Collapse `(status, approved)` into the states a reader can act on.
    #[must_use]
    pub const fn outcome(&self) -> ApprovalOutcome {
        match (self.status, self.approved) {
            (PartStatus::RequiresAction | PartStatus::Pending | PartStatus::Open, _) => {
                ApprovalOutcome::Awaiting
            }
            (_, Some(true)) => ApprovalOutcome::Approved,
            (_, Some(false)) => ApprovalOutcome::Declined,
            (PartStatus::Failed, None) => ApprovalOutcome::Failed,
            (_, None) => ApprovalOutcome::Unanswered,
        }
    }
}

/// What became of an approval request.
///
/// The gpui-box `ApprovalPrompt` reference separates *expired* from *replaced*
/// with two different marks. `theorem-chat/1` cannot carry that distinction:
/// both arrive as `status: incomplete, approved: null` and differ only in the
/// host's `reason` prose. Rather than guess which one a request was, this
/// projection has one [`Self::Unanswered`] state and the renderer prints the
/// reason beside it, so the difference reaches the reader as words instead of
/// as a colour the wire never justified. Splitting the state is an upstream
/// change to the part schema, not a change here.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalOutcome {
    /// Still the reader's move.
    #[default]
    Awaiting,
    Approved,
    Declined,
    /// Settled without an answer: expired, or replaced by a later request.
    Unanswered,
    /// The approval machinery itself failed.
    Failed,
}

impl ApprovalOutcome {
    /// Whether the prompt still offers its choices.
    #[must_use]
    pub const fn is_open(self) -> bool {
        matches!(self, Self::Awaiting)
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Awaiting => "Waiting for approval",
            Self::Approved => "Approved",
            Self::Declined => "Declined",
            Self::Unanswered => "Not answered",
            Self::Failed => "This approval failed",
        }
    }
}

/// Chain-of-thought state.
///
/// The four states are the whole vocabulary, and the distinction between
/// [`ReasoningState::Withheld`] and [`ReasoningState::Absent`] is deliberate:
/// "the provider refused to hand reasoning over" and "this run produced none"
/// are different facts, and collapsing them would let a silent provider read as
/// an honest one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningPart {
    pub part_id: String,
    pub state: ReasoningState,
    /// Wall-clock the provider reported, when it reported one. The renderer
    /// never measures this itself; a locally timed number would drift from what
    /// the provider actually spent.
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
    /// Whether the host wants the body open. View state, echoed back on toggle.
    #[serde(default)]
    pub expanded: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ReasoningState {
    /// Reasoning is arriving now. Renders as "Thinking…" with a live mark.
    Streaming { text: String },
    /// Provider-emitted reasoning, complete. Renders as "Thought for N s" with
    /// an expandable body.
    Present { text: String },
    /// The provider declined to emit reasoning, and said why. Renders in the
    /// caution register with the reason beside it, never hidden.
    Withheld { reason: String },
    /// The run finished and produced no reasoning at all.
    #[default]
    Absent,
}

/// One line of code, carrying its own file-relative number.
///
/// Numbers come from the file, marks from the host, colour from the caller —
/// the three-way split the CodeView reference states in its own caption.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeLine {
    /// The file's own line number, which is not the index in `lines` whenever
    /// the block is an excerpt.
    pub number: u32,
    #[serde(default)]
    pub mark: LineMark,
    #[serde(default)]
    pub spans: Vec<CodeSpan>,
}

impl CodeLine {
    /// The line's plain text, for Copy and for the accessibility mirror.
    #[must_use]
    pub fn text(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect()
    }
}

/// Host-supplied gutter mark. Not a diff engine: the host decides, this renders.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineMark {
    #[default]
    None,
    Added,
    Removed,
    Changed,
    Highlighted,
}

/// One coloured run inside a line.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeSpan {
    pub text: String,
    #[serde(default)]
    pub role: SyntaxRole,
}

/// The single highlight vocabulary.
///
/// Both the code renderer and the Markdown renderer's inline-code highlighter
/// emit these roles, and one resolver in the styling adapter turns them into
/// colours. Adding a colour anywhere else re-opens the second-palette problem
/// this enum exists to close.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyntaxRole {
    #[default]
    Plain,
    Keyword,
    Literal,
    StringLiteral,
    Comment,
    Type,
    Function,
    Punctuation,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_part(part_id: &str, status: PartStatus) -> ThreadPart {
        ThreadPart::Text {
            part_id: part_id.to_owned(),
            text: "hello".to_owned(),
            status,
        }
    }

    #[test]
    fn validate_refuses_a_foreign_schema() {
        let document = ThreadDocument {
            schema: "theorem-thread-projection/2".to_owned(),
            ..ThreadDocument::default()
        };
        let error = document.validate().expect_err("a v2 schema must refuse");
        assert!(error.contains("theorem-thread-projection/2"), "{error}");
    }

    #[test]
    fn only_the_last_message_streams() {
        let document = ThreadDocument {
            schema: THREAD_PROJECTION_SCHEMA.to_owned(),
            messages: vec![
                ThreadMessage {
                    message_id: "m1".to_owned(),
                    actor: ThreadActor::Theorem,
                    parts: vec![text_part("p1", PartStatus::Running)],
                },
                ThreadMessage {
                    message_id: "m2".to_owned(),
                    actor: ThreadActor::Theorem,
                    parts: vec![text_part("p2", PartStatus::Complete)],
                },
            ],
            ..ThreadDocument::default()
        };
        assert_eq!(
            document.streaming_message(),
            None,
            "a running part on an earlier message must not animate"
        );
    }

    #[test]
    fn a_running_final_message_streams() {
        let document = ThreadDocument {
            schema: THREAD_PROJECTION_SCHEMA.to_owned(),
            messages: vec![ThreadMessage {
                message_id: "m1".to_owned(),
                actor: ThreadActor::Theorem,
                parts: vec![text_part("p1", PartStatus::Running)],
            }],
            ..ThreadDocument::default()
        };
        assert_eq!(document.streaming_message(), Some(0));
    }

    #[test]
    fn withheld_and_absent_do_not_collapse() {
        let withheld = ReasoningState::Withheld {
            reason: "This connection does not hand over reasoning.".to_owned(),
        };
        assert_ne!(withheld, ReasoningState::Absent);
        let encoded = serde_json::to_string(&withheld).expect("withheld serializes");
        assert!(encoded.contains("\"state\":\"withheld\""), "{encoded}");
        let absent = serde_json::to_string(&ReasoningState::Absent).expect("absent serializes");
        assert!(absent.contains("\"state\":\"absent\""), "{absent}");
    }

    #[test]
    fn code_line_text_rejoins_its_spans() {
        let line = CodeLine {
            number: 41,
            mark: LineMark::Added,
            spans: vec![
                CodeSpan {
                    text: "let ".to_owned(),
                    role: SyntaxRole::Keyword,
                },
                CodeSpan {
                    text: "verified".to_owned(),
                    role: SyntaxRole::Plain,
                },
            ],
        };
        assert_eq!(line.text(), "let verified");
    }

    #[test]
    fn a_document_round_trips_through_json() {
        let document = ThreadDocument {
            schema: THREAD_PROJECTION_SCHEMA.to_owned(),
            thread_id: "t1".to_owned(),
            title: "First run".to_owned(),
            run_state: ThreadRunState::Streaming,
            reduced_motion: true,
            pinned_to_bottom: false,
            messages: vec![ThreadMessage {
                message_id: "m1".to_owned(),
                actor: ThreadActor::User,
                parts: vec![
                    text_part("p1", PartStatus::Complete),
                    ThreadPart::Reasoning(ReasoningPart {
                        part_id: "p2".to_owned(),
                        state: ReasoningState::Present {
                            text: "Read both files first.".to_owned(),
                        },
                        elapsed_ms: Some(8400),
                        expanded: true,
                    }),
                ],
            }],
        };
        let encoded = serde_json::to_string(&document).expect("document serializes");
        let decoded: ThreadDocument = serde_json::from_str(&encoded).expect("document decodes");
        assert_eq!(decoded, document);
        decoded.validate().expect("the pinned schema validates");
    }

    fn approval(status: PartStatus, approved: Option<bool>) -> ApprovalPart {
        ApprovalPart {
            part_id: "a1".to_owned(),
            approval_id: "ap1".to_owned(),
            tool_call_id: "tc1".to_owned(),
            summary: "Write to /work/report/summary.md".to_owned(),
            risk: "write".to_owned(),
            status,
            approved,
            reason: None,
        }
    }

    #[test]
    fn an_open_request_is_awaiting_whatever_approved_says() {
        // A host that leaves a stale `approved` on a re-opened request must not
        // make the prompt render as already settled.
        assert_eq!(
            approval(PartStatus::RequiresAction, Some(true)).outcome(),
            ApprovalOutcome::Awaiting
        );
        assert!(approval(PartStatus::RequiresAction, None)
            .outcome()
            .is_open());
    }

    #[test]
    fn a_settled_request_reads_its_answer() {
        assert_eq!(
            approval(PartStatus::Complete, Some(true)).outcome(),
            ApprovalOutcome::Approved
        );
        assert_eq!(
            approval(PartStatus::Complete, Some(false)).outcome(),
            ApprovalOutcome::Declined
        );
    }

    #[test]
    fn expired_and_replaced_both_land_on_unanswered() {
        // The wire cannot tell them apart, so neither can this projection. The
        // difference survives in `reason`, which the renderer prints verbatim.
        let mut expired = approval(PartStatus::Incomplete, None);
        expired.reason = Some("This request expired before it was answered".to_owned());
        let mut replaced = approval(PartStatus::Incomplete, None);
        replaced.reason = Some("Replaced by a later request".to_owned());

        assert_eq!(expired.outcome(), ApprovalOutcome::Unanswered);
        assert_eq!(replaced.outcome(), ApprovalOutcome::Unanswered);
        assert_ne!(expired.reason, replaced.reason);
        assert!(!expired.outcome().is_open());
    }

    #[test]
    fn a_broken_approval_is_not_a_declined_one() {
        assert_eq!(
            approval(PartStatus::Failed, None).outcome(),
            ApprovalOutcome::Failed
        );
    }

    #[test]
    fn every_part_field_speaks_one_casing() {
        // A struct variant and a newtype variant in the same document. Serde
        // renames variant *fields* only when told to, so this pins the thing
        // that silently drifts: `part_id` beside `partId` in one payload.
        let document = ThreadDocument {
            schema: THREAD_PROJECTION_SCHEMA.to_owned(),
            thread_id: "t".to_owned(),
            messages: vec![ThreadMessage {
                message_id: "m".to_owned(),
                actor: ThreadActor::Theorem,
                parts: vec![
                    text_part("struct-variant", PartStatus::Complete),
                    ThreadPart::Reasoning(ReasoningPart {
                        part_id: "newtype-variant".to_owned(),
                        state: ReasoningState::Present {
                            text: "why".to_owned(),
                        },
                        elapsed_ms: Some(1),
                        expanded: false,
                    }),
                ],
            }],
            ..ThreadDocument::default()
        };

        let wire = serde_json::to_string(&document).expect("the projection serializes");
        assert!(!wire.contains("part_id"), "snake_case leaked into {wire}");
        assert_eq!(wire.matches("\"partId\"").count(), 2);
        assert!(wire.contains("\"threadId\""));
        assert!(wire.contains("\"messageId\""));
        assert_eq!(
            serde_json::from_str::<ThreadDocument>(&wire).expect("it parses back"),
            document
        );
    }

    #[test]
    fn a_tool_call_keeps_its_multiword_fields_camel() {
        let wire = serde_json::to_string(&ThreadPart::ToolCall {
            part_id: "p".to_owned(),
            tool_call_id: "tc".to_owned(),
            tool_name: "graph.read".to_owned(),
            status: PartStatus::Running,
            arguments_preview: "{}".to_owned(),
        })
        .expect("the part serializes");
        for field in ["partId", "toolCallId", "toolName", "argumentsPreview"] {
            assert!(wire.contains(field), "{field} missing from {wire}");
        }
    }
}
