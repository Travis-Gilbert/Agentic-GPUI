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

    /// Takes any string as an identity, encoding whatever the grammar does
    /// not allow rather than carrying it through malformed.
    ///
    /// [`Self::new`] is for an id whose shape the caller already knows.
    /// This is for one that arrived: a part id off the wire, a canvas node
    /// id, a message id a projection derived a control name from. Each
    /// dot-separated piece goes through [`segment`], which leaves an id that
    /// already fits alone -- so `thread.turn.m2` is still `thread.turn.m2` in
    /// a receipt -- and encodes the rest. The separators survive, so a path
    /// stays a path and the whole mapping stays injective.
    ///
    /// Every piece goes through [`segment`], including the ones the grammar
    /// would have admitted whole. There used to be a fast path here that
    /// returned an already-valid id untouched, and it broke the injectivity
    /// this method advertises: the grammar admits `_`, [`segment`] spends it
    /// as the escape tag, and so `A` -- escaped to `hex_41` -- and a raw
    /// `hex_41` arrived at one identity. Two nodes under one name is the
    /// defect this encoder exists to prevent. The cost of dropping the fast
    /// path is that a segment carrying `_` is now escaped rather than carried
    /// through, which is the price of the tag being a byte the readable
    /// branch may not spend.
    pub fn encoded(id: impl AsRef<str>) -> Self {
        let encoded = id.as_ref().split('.').map(segment).collect::<Vec<_>>();
        Self(Arc::from(encoded.join(".").as_str()))
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

/// The escaped form's tag. `_` is what makes the two branches of [`segment`]
/// disjoint, so it is the one byte the readable branch may not contain.
const ESCAPED: &str = "hex_";

/// One arbitrary string carried into one segment, injectively.
///
/// The grammar is `[a-z0-9][a-z0-9_-]*` and ids arrive carrying whatever
/// their producer used, so something has to give. Folding every stray byte to
/// `-` was the first answer and it was lossy in a way a surface cannot
/// survive: `A` and `a`, or `node/a` and `node:a`, are distinct ids to
/// whatever minted them and one segment to the fold. Two controls then publish
/// the same node id, [`Snapshot::lint`](super::Snapshot::lint) reports a
/// duplicate for a tree that was well formed, and an agent can only ever reach
/// the first of them.
///
/// So this is a mapping rather than a fold. A subject already shaped like a
/// segment -- and free of `_` -- is carried through unchanged, which is the
/// case that matters for reading a receipt: `e-one` stays `e-one`. Anything
/// else becomes [`ESCAPED`] followed by the hex of its UTF-8 bytes, which is
/// injective and decodable. The two branches cannot collide because the
/// readable one never contains `_` and the escaped one always does.
#[must_use]
pub fn segment(subject: &str) -> String {
    use std::fmt::Write as _;

    let readable = !subject.is_empty()
        && subject.starts_with(|first: char| first.is_ascii_lowercase() || first.is_ascii_digit())
        && subject
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if readable {
        return subject.to_owned();
    }
    let mut escaped = String::with_capacity(ESCAPED.len() + subject.len() * 2);
    escaped.push_str(ESCAPED);
    for byte in subject.bytes() {
        let _ = write!(escaped, "{byte:02x}");
    }
    escaped
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
    use super::{segment, Ident, IdentViolation, ESCAPED};

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

    /// The escaped branch, read back. Injectivity is the whole claim, and a
    /// round trip is the only way to assert it rather than sample it.
    fn decode(encoded: &str) -> String {
        let hex = encoded
            .strip_prefix(ESCAPED)
            .expect("an escaped segment carries the tag");
        let bytes = (0..hex.len() / 2)
            .map(|ix| u8::from_str_radix(&hex[ix * 2..ix * 2 + 2], 16).expect("two hex digits"))
            .collect();
        String::from_utf8(bytes).expect("the bytes that arrived")
    }

    #[test]
    fn an_id_already_shaped_like_a_segment_is_carried_through() {
        for subject in ["a", "e-one", "node-7", "7"] {
            assert_eq!(segment(subject), subject, "a receipt should stay readable");
        }
    }

    #[test]
    fn ids_the_old_fold_collapsed_together_now_stay_apart() {
        // Every pair here is two ids to whatever minted them and was one
        // segment to the case-folding replacement this grew out of: their
        // controls published the same node id, `lint` reported a duplicate,
        // and only the first was reachable.
        for (left, right) in [("A", "a"), ("node/a", "node:a"), ("x_y", "x-y")] {
            assert_ne!(
                segment(left),
                segment(right),
                "{left:?} and {right:?} are two subjects"
            );
        }
    }

    #[test]
    fn every_encoded_segment_fits_the_grammar() {
        for subject in ["A", "", "-lead", "_lead", "node/a", "n\u{e9}", "x_y", "hex_41"] {
            let encoded = segment(subject);
            assert!(Ident::is_valid(&encoded), "{subject:?} encoded to {encoded:?}");
        }
    }

    #[test]
    fn an_escaped_segment_decodes_back_to_what_arrived() {
        for subject in ["A", "", "-lead", "node/a", "n\u{e9}", "x_y", "hex_41"] {
            assert_eq!(decode(&segment(subject)), subject);
        }
    }

    /// The separators are the point: an id that is already a path stays one,
    /// so only the pieces that broke the grammar are rewritten and the rest
    /// of a receipt stays readable.
    #[test]
    fn encoding_a_whole_id_rewrites_only_the_segments_that_broke_the_grammar() {
        for (raw, encoded) in [
            ("thread.m2.reasoning", "thread.m2.reasoning"),
            ("p1.0", "p1.0"),
            ("Part1.0", "hex_5061727431.0"),
            ("part:1", "hex_706172743a31"),
        ] {
            assert_eq!(Ident::encoded(raw).as_str(), encoded, "{raw}");
        }
    }

    /// The whole-id encoder is injective too, not just [`segment`].
    ///
    /// The defect this holds shut: `encoded` returned an already-valid id
    /// untouched, and the grammar admits the `_` that [`segment`] spends as
    /// its escape tag. `A` escapes to `hex_41`, a raw `hex_41` was carried
    /// through, and the two arrived at one identity -- a duplicate node and an
    /// ambiguous action lookup for every caller of the public wire-id encoder.
    #[test]
    fn an_id_shaped_like_an_escape_is_not_the_id_it_would_decode_to() {
        for (left, right) in [("A", "hex_41"), ("a.A", "a.hex_41"), ("x_y", "x-y")] {
            assert_ne!(
                Ident::encoded(left),
                Ident::encoded(right),
                "{left:?} and {right:?} are two ids"
            );
        }
    }

    #[test]
    fn every_encoded_id_fits_the_grammar() {
        for raw in ["", ".", "a..b", "Part1.0", "part:1", "-lead.TRAIL", "n\u{e9}"] {
            let encoded = Ident::encoded(raw);
            assert!(
                Ident::is_valid(encoded.as_str()),
                "{raw:?} encoded to {:?}",
                encoded.as_str()
            );
        }
    }
}
