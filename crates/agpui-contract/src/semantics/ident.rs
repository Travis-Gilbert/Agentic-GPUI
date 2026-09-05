//! One identity for the element and for the semantic tree, and the law it
//! obeys.
//!
//! Ported from `gpui-box`, `crates/gpui-kit/src/foundation/ident.rs`, at
//! `e993d0f4e2dbd4a9697db79c6428a623856444a4` (GPUI Box contributors,
//! MIT OR Apache-2.0).
//!
//! Two changes from the port, both from SPEC-AGPUI-SEMANTIC-TREE-1.0 D2:
//!
//! * the backing string is `Arc<str>` rather than `gpui::SharedString`, so
//!   this module has no renderer in its dependency tree. `element_id` and
//!   `indexed_element_id` moved to `agpui::IdentElementExt`, which is where
//!   `gpui::ElementId` is nameable;
//! * [`Ident::parse`] is new. `Ident::new` still accepts anything, because a
//!   host that mints an id at runtime should not panic in a paint; `parse` is
//!   what the story leaf, the oracle, and [`super::Snapshot::lint`] use.

use std::fmt;
use std::sync::Arc;

/// One identity used for both the GPUI element and the semantic tree.
///
/// Ids come from business identity, never list position, so an assertion that
/// targets `thread.turn.m2.reasoning` keeps working when the turn moves.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident(Arc<str>);

/// Why a string is not a legal [`Ident`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentError {
    /// The rejected string, verbatim.
    pub id: String,
    /// Which rule it broke.
    pub reason: IdentViolation,
}

/// The single rule an id can break.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentViolation {
    /// The whole string was empty.
    Empty,
    /// A dot-separated segment was empty, so the id had a leading, trailing,
    /// or doubled `.`.
    EmptySegment,
    /// A segment started with something other than `[a-z0-9]`.
    SegmentStart,
    /// A segment carried a byte outside `[a-z0-9_-]`.
    SegmentByte,
}

impl fmt::Display for IdentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let reason = match self.reason {
            IdentViolation::Empty => "an id is never empty",
            IdentViolation::EmptySegment => "every dot-separated segment carries at least one byte",
            IdentViolation::SegmentStart => "a segment starts with [a-z0-9]",
            IdentViolation::SegmentByte => "a segment continues with [a-z0-9_-]",
        };
        write!(formatter, "{:?} is not an ident: {reason}", self.id)
    }
}

impl std::error::Error for IdentError {}

impl Ident {
    /// Takes any string as an identity.
    ///
    /// Paint paths use this: an id assembled from a record key at render time
    /// must not be able to panic a frame. [`Snapshot::lint`](super::Snapshot::lint)
    /// is what reports the ones outside the grammar.
    pub fn new(id: impl AsRef<str>) -> Self {
        Self(Arc::from(id.as_ref()))
    }

    /// Takes a string only when it satisfies the grammar
    /// `segment ( "." segment )*`, `segment = [a-z0-9] [a-z0-9_-]*`.
    ///
    /// # Errors
    ///
    /// Returns the offending string and the rule it broke.
    pub fn parse(id: impl AsRef<str>) -> Result<Self, IdentError> {
        let raw = id.as_ref();
        check(raw).map(|()| Self(Arc::from(raw)))
    }

    /// True when a string satisfies the grammar.
    #[must_use]
    pub fn is_valid(id: &str) -> bool {
        check(id).is_ok()
    }

    /// Derives a child identity, for example the disclosure inside a block.
    #[must_use]
    pub fn child(&self, suffix: impl AsRef<str>) -> Self {
        Self(Arc::from(format!("{}.{}", self.0, suffix.as_ref()).as_str()))
    }

    /// The identity as one shared string.
    #[must_use]
    pub fn semantic_id(&self) -> Arc<str> {
        Arc::clone(&self.0)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn check(id: &str) -> Result<(), IdentError> {
    let fail = |reason| {
        Err(IdentError {
            id: id.to_owned(),
            reason,
        })
    };
    if id.is_empty() {
        return fail(IdentViolation::Empty);
    }
    for segment in id.split('.') {
        let mut bytes = segment.bytes();
        let Some(first) = bytes.next() else {
            return fail(IdentViolation::EmptySegment);
        };
        if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
            return fail(IdentViolation::SegmentStart);
        }
        for byte in bytes {
            if !byte.is_ascii_lowercase() && !byte.is_ascii_digit() && byte != b'_' && byte != b'-' {
                return fail(IdentViolation::SegmentByte);
            }
        }
    }
    Ok(())
}

impl From<&str> for Ident {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Ident {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<Ident> for String {
    fn from(value: Ident) -> Self {
        value.0.to_string()
    }
}

impl AsRef<str> for Ident {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{Ident, IdentViolation};

    #[test]
    fn child_ids_are_prefixed_by_their_owner() {
        let block = Ident::new("thread.m2.reasoning");
        assert_eq!(block.child("body").as_str(), "thread.m2.reasoning.body");
    }

    #[test]
    fn the_grammar_accepts_the_ids_the_surfaces_already_publish() {
        for id in [
            "composer",
            "composer-send",
            "thread.m2.reasoning",
            "records.row.r-118",
            "chip-remove-a1",
            "t2",
        ] {
            assert!(Ident::parse(id).is_ok(), "{id} should parse");
        }
    }

    #[test]
    fn the_grammar_refuses_case_position_and_punctuation() {
        for (id, reason) in [
            ("", IdentViolation::Empty),
            ("Composer.Send", IdentViolation::SegmentStart),
            ("composer.Send", IdentViolation::SegmentStart),
            ("composer.", IdentViolation::EmptySegment),
            (".composer", IdentViolation::EmptySegment),
            ("composer..send", IdentViolation::EmptySegment),
            ("-composer", IdentViolation::SegmentStart),
            ("composer send", IdentViolation::SegmentByte),
            ("composer/send", IdentViolation::SegmentByte),
        ] {
            let error = Ident::parse(id).expect_err("the grammar refuses this id");
            assert_eq!(error.reason, reason, "{id}");
            assert_eq!(error.id, id);
        }
    }

    #[test]
    fn new_accepts_what_parse_refuses_so_a_paint_never_panics() {
        assert_eq!(Ident::new("Composer.Send").as_str(), "Composer.Send");
        assert!(!Ident::is_valid("Composer.Send"));
    }
}
