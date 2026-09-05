//! Renderer-free design-token law and deterministic emitters.
//!
//! The law is the mechanism: a DTCG document declares neutral steps by
//! lightness only, and this crate derives their chroma from one curve. The
//! product is the document. AGPUI owns the first and never the second, which
//! is why nothing here embeds a token file. [`TokenSet::from_dtcg_str`] takes
//! the document as an argument, and [`metrics::ShellMetrics`] is a shape the
//! product fills in.
//!
//! SPEC-AGPUI-HOME-1.0 H7 moved this out of Theorem's `theorem-design-core`.
//! What stayed behind is what is Theorem's rather than the law's: the token
//! file itself, the const that names it, the PNG grain bake over a vendored
//! shader, and every test that asserts a particular hex.

mod color;
mod emit_css;
mod emit_gpui;
pub mod metrics;
mod prose;
mod texture;
mod tokens;

pub use color::Rgba;
// The semantic role table is part of the emitter's contract, not an
// implementation detail of it: a product that emits CSS from this crate has
// to be able to assert that every role it declares came out the other side.
pub use emit_gpui::SEMANTIC_MAPPING;
pub use metrics::ShellMetrics;
pub use prose::{ProseHighlightStyle, PROSE_CAPTURES};
pub use texture::{DotGridParams, GrainParams};
pub use tokens::{NeutralLaw, NeutralSample, TokenSet};

/// Every way a token document can fail to be one.
///
/// This carries token concerns only. Baking a grain PNG can fail for I/O and
/// encoding reasons that have nothing to do with tokens; that lives with the
/// baker, in the product, and wraps this.
#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("invalid token JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("missing token: {0}")]
    MissingToken(String),
    #[error("invalid token: {0}")]
    InvalidToken(String),
    #[error("invalid color: {0}")]
    InvalidColor(String),
    #[error("token alias cycle at: {0}")]
    AliasCycle(String),
    #[error("neutral step must declare lightness, not a hex color: {0}")]
    AuthoredNeutral(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A document that is not anybody's palette.
    ///
    /// The law has to be provable without a product. If these tests ran
    /// against Theorem's tokens they would prove that Theorem's file is what
    /// Theorem says it is, which is Theorem's test to run and not this
    /// crate's. The fixture's hue is blue and its ramp is round, so a value
    /// that leaks from here into a product assertion is obvious on sight.
    const LAW: &str = include_str!("../fixtures/law.tokens.json");

    fn law() -> TokenSet {
        TokenSet::from_dtcg_str(LAW).expect("the law fixture must parse")
    }

    #[test]
    fn an_alias_resolves_through_to_a_generated_neutral() {
        let tokens = law();
        assert_eq!(
            tokens.color("surface.page").unwrap(),
            tokens.color("color.cream.25").unwrap()
        );
    }

    #[test]
    fn an_alias_resolves_through_another_alias() {
        let tokens = law();
        assert_eq!(
            tokens.color("surface.echo").unwrap(),
            tokens.color("color.cream.50").unwrap()
        );
    }

    #[test]
    fn a_literal_hex_survives_beside_the_generated_steps() {
        assert_eq!(law().color("surface.raised").unwrap().hex(), "#FFFFFF");
    }

    #[test]
    fn neutral_steps_are_generated_rather_than_read_back() {
        let tokens = law();
        for path in ["color.cream.25", "color.cream.900", "color.ink.primary"] {
            let sample = tokens.neutral_sample(path).expect(path);
            assert!(sample.chroma > 0.0, "{path} was generated flat");
            assert_eq!(sample.color, tokens.color(path).unwrap());
        }
    }

    #[test]
    fn every_generated_step_respects_the_relative_chroma_bound() {
        let tokens = law();
        let bound = f64::from(tokens.neutral_law().max_relative_chroma);
        for path in [
            "color.cream.25",
            "color.cream.50",
            "color.cream.100",
            "color.cream.200",
            "color.cream.300",
            "color.cream.400",
            "color.cream.700",
            "color.cream.900",
            "color.ink.primary",
            "color.ink.muted",
            "color.ink.faint",
        ] {
            let sample = tokens.neutral_sample(path).expect(path);
            assert!(
                f64::from(sample.chroma / sample.lightness) <= bound + 1e-7,
                "relative chroma bound for {path}"
            );
        }
    }

    /// The curve's own statement about itself, with no document in it.
    ///
    /// The surface chroma curve is written to peak at exactly the declared
    /// relative bound, at six sevenths lightness. Every step in the test above
    /// is under the bound; this is the one place it is reached, so it is the
    /// one place a change to the curve's shape shows up as a number.
    #[test]
    fn the_surface_curve_peaks_at_the_declared_bound() {
        let law = law().neutral_law();
        let peak_lightness = 6.0 / 7.0;
        let peak = law.surface_chroma(peak_lightness) / peak_lightness;
        assert!(
            (peak - law.max_relative_chroma).abs() <= 1e-6,
            "curve peak was {peak}, bound is {}",
            law.max_relative_chroma
        );
    }

    #[test]
    fn an_authored_neutral_hex_is_rejected() {
        let source = LAW.replacen("{ \"lightness\": 0.98 }", "\"#FBFAF7\"", 1);
        assert!(matches!(
            TokenSet::from_dtcg_str(&source),
            Err(TokenError::AuthoredNeutral(path)) if path == "color.cream.25"
        ));
    }

    #[test]
    fn a_dangling_alias_is_rejected_at_parse_rather_than_at_read() {
        let source = LAW.replacen("{color.cream.25}", "{missing.surface}", 1);
        assert!(matches!(
            TokenSet::from_dtcg_str(&source),
            Err(TokenError::MissingToken(path)) if path == "missing.surface"
        ));
    }

    #[test]
    fn a_document_with_no_neutral_law_is_refused() {
        let source = LAW.replacen("\"hue\"", "\"hueue\"", 1);
        assert!(matches!(
            TokenSet::from_dtcg_str(&source),
            Err(TokenError::MissingToken(path)) if path == "color.neutral.hue"
        ));
    }

    #[test]
    fn prose_captures_are_declared_once_and_counted() {
        let unique: std::collections::BTreeSet<_> = PROSE_CAPTURES.into_iter().collect();
        assert_eq!(unique.len(), PROSE_CAPTURES.len());
    }
}
