//! One assertion target, and the geometry it was published at.
//!
//! Ported from `gpui-box`, `crates/gpui-kit-semantics/src/lib.rs`, at
//! `e993d0f4e2dbd4a9697db79c6428a623856444a4` (GPUI Box contributors,
//! MIT OR Apache-2.0). [`SemanticReadingItem`] is new in
//! SPEC-AGPUI-SEMANTIC-TREE-1.0 D1 and is not from the port.

use serde::{Deserialize, Serialize};

use super::redact::redact_sensitive_text;
use super::role::Role;

/// The urgency a live region announces at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LiveRegion {
    Polite,
    Assertive,
}

/// The bounds GPUI actually produced for an element, in window pixels.
///
/// Geometry is recorded, never asserted on by the canonical hash: a resize is
/// not a state change. See [`super::hash::HashView`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    #[must_use]
    pub fn area(self) -> f32 {
        self.width.max(0.0) * self.height.max(0.0)
    }

    #[must_use]
    pub fn center(self) -> (f32, f32) {
        (self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        self.x < other.x + other.width
            && other.x < self.x + self.width
            && self.y < other.y + other.height
            && other.y < self.y + self.height
    }

    /// True when the point is inside the rect, edges on the left and top
    /// included.
    ///
    /// The dispatcher uses this to decide whether platform input aimed at one
    /// node's centre would land on a later-painted sibling instead.
    #[must_use]
    pub fn contains(self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// True when `other` lies wholly inside this rect.
    ///
    /// Distinct from [`Self::contains`] on a point: a container encloses
    /// everything it holds, while something laid over one control generally
    /// covers its middle without enclosing all of it. The dispatcher uses the
    /// difference to tell a frame from an overlay.
    #[must_use]
    pub fn encloses(self, other: Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x + other.width <= self.x + self.width
            && other.y + other.height <= self.y + self.height
    }
}

/// A single assertion target published for one frame.
///
/// Fields added after the initial protocol are omitted from serialized
/// snapshots unless a component sets them, so recorded baselines stay stable
/// as new roles gain state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is one ARIA state a screen reader and an agent both read; \
              collapsing them into a bitset would hide the field names the diff reports"
)]
pub struct Node {
    pub id: String,
    pub role: Role,
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub describes: Option<String>,
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub bounds: Rect,
    pub visible: bool,
    pub focused: bool,
    pub disabled: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub read_only: bool,
    /// `None` on a node that is not selectable at all, which is most of them.
    ///
    /// Optional for the same reason [`Self::checked`] and [`Self::expanded`]
    /// are: a dispatcher preflights a gesture by asking whether the node
    /// carries the state it moves, and a plain `bool` answers "yes, false" for
    /// every node in the tree. `Select` aimed at an ordinary button therefore
    /// clicked it -- navigating, sending, whatever the button does -- and only
    /// then reported the postcondition unmet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<bool>,
    pub hovered: bool,
    pub pressed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expanded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_min: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_max: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_now: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u32>,
    #[serde(skip_serializing_if = "is_false")]
    pub busy: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub invalid: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live: Option<LiveRegion>,
    #[serde(skip_serializing_if = "is_false")]
    pub live_atomic: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub modal: bool,
}

impl Node {
    /// Re-applies redaction to the three free-text fields.
    ///
    /// The probe already redacts what it records; this covers nodes a host
    /// constructed directly, and is idempotent.
    pub fn redact(&mut self) {
        if let Some(text) = &mut self.text {
            *text = redact_sensitive_text(text);
        }
        if let Some(description) = &mut self.description {
            *description = redact_sensitive_text(description);
        }
        if let Some(value) = &mut self.value {
            *value = redact_sensitive_text(value);
        }
    }
}

/// A row a surface knows about but has not materialized.
///
/// A surface publishes one per row it can name and did not build: the rows
/// outside a virtual list's window, and the rows behind a closed disclosure.
/// An action aimed at a reading item is refused with
/// [`ActionRefusal::TargetNotMaterialized`](super::action::ActionRefusal::TargetNotMaterialized)
/// rather than reported absent, because the surface does know the row exists,
/// and the refusal carries the gesture that would materialize it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SemanticReadingItem {
    pub id: String,
    pub parent: Option<String>,
    pub role: Role,
    pub text: Option<String>,
    pub focused: bool,
    pub selected: bool,
}

impl SemanticReadingItem {
    pub fn redact(&mut self) {
        if let Some(text) = &mut self.text {
            *text = redact_sensitive_text(text);
        }
    }
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if hands the field by reference"
)]
pub(crate) const fn is_false(value: &bool) -> bool {
    !*value
}
