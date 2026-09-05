//! Renderer-free design-token law and deterministic emitters.
//!
//! The law is the mechanism: a DTCG document authors three theme inputs - a
//! base, an accent and a contrast - and this crate derives the whole role
//! registry from them with Linear's adjustment law, then resolves every alias
//! against it. The product is the document. AGPUI owns the first and never the
//! second, which is why nothing here embeds a token file.
//! [`TokenSet::from_dtcg_str`] takes the document as an argument, and
//! [`metrics::ShellMetrics`] is a shape the product fills in.
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
mod regions;
mod texture;
mod theme_law;
mod tokens;

pub use color::Rgba;
pub use emit_gpui::SEMANTIC_MAPPING;
pub use metrics::ShellMetrics;
pub use prose::{ProseHighlightStyle, PROSE_CAPTURES};
pub use regions::{Rect, ShellRegions};
pub use texture::{DotGridParams, GrainParams};
pub use theme_law::{apca_contrast, readable_lightness, CieLch, DerivedTheme, Oklch, ThemeInput};
pub use tokens::TokenSet;

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
    #[error("invalid theme color: {0}")]
    InvalidThemeColor(String),
    #[error("invalid theme file: {0}")]
    InvalidThemeFile(String),
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// A document that is not anybody's palette.
    ///
    /// The law has to be provable without a product. If these tests ran
    /// against Theorem's tokens they would prove that Theorem's file is what
    /// Theorem says it is, which is Theorem's test to run and not this
    /// crate's. The fixture authors a blue base and accent and names its
    /// typefaces after itself, so a value that leaks from here into a product
    /// assertion is obvious on sight.
    const LAW: &str = include_str!("../fixtures/law.tokens.json");

    fn law() -> TokenSet {
        TokenSet::from_dtcg_str(LAW).expect("the law fixture must parse")
    }

    #[test]
    fn an_alias_resolves_to_the_generated_role_it_names() {
        let tokens = law();
        assert_eq!(
            tokens.color("surface.page").unwrap(),
            tokens.color("theme.bgSub").unwrap()
        );
    }

    /// Resolution is transitive, and eager: `texture.grain.colorBack` names
    /// `surface.page`, which names a role that only exists because the theme
    /// was derived first.
    #[test]
    fn an_alias_resolves_through_another_alias() {
        let tokens = law();
        assert_eq!(
            tokens.color("texture.grain.colorBack").unwrap(),
            tokens.color("theme.bgSub").unwrap()
        );
    }

    #[test]
    fn a_dangling_alias_is_rejected_at_parse_rather_than_at_read() {
        let source = LAW.replacen("{theme.bgSub}", "{missing.surface}", 1);
        assert!(matches!(
            TokenSet::from_dtcg_str(&source),
            Err(TokenError::MissingToken(path)) if path == "missing.surface"
        ));
    }

    #[test]
    fn a_document_missing_a_theme_input_is_refused() {
        let source = LAW.replacen("\"accent\"", "\"acccent\"", 1);
        assert!(matches!(
            TokenSet::from_dtcg_str(&source),
            Err(TokenError::MissingToken(path)) if path == "theme.accent"
        ));
    }

    /// Contrast is a dial with ends, not a free number, and the refusal is at
    /// parse so no renderer ever sees a palette derived from an impossible one.
    #[test]
    fn a_contrast_outside_the_dial_is_refused() {
        let source = LAW.replacen("\"$value\": 40", "\"$value\": 140", 1);
        assert!(matches!(
            TokenSet::from_dtcg_str(&source),
            Err(TokenError::InvalidThemeFile(_))
        ));
    }

    /// The whole point of the law: a document authors three values and every
    /// colour role in it is derived. If a fourth authored key ever appears,
    /// something has been hand-set that the law was supposed to compute.
    #[test]
    fn a_document_authors_only_the_three_theme_inputs() {
        let source: serde_json::Value = serde_json::from_str(LAW).unwrap();
        let authored: BTreeSet<_> = source["theme"]
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| !key.starts_with('$'))
            .map(String::as_str)
            .collect();
        assert_eq!(authored, BTreeSet::from(["accent", "base", "contrast"]));
    }

    #[test]
    fn every_semantic_role_resolves_to_the_path_it_maps() {
        let tokens = law();
        let colors = tokens.semantic_colors();
        assert_eq!(colors.len(), SEMANTIC_MAPPING.len());
        for (name, path) in SEMANTIC_MAPPING {
            assert_eq!(colors[name], tokens.color(path).unwrap(), "semantic {name}");
        }
    }

    /// Re-deriving is the law applied twice: the generated roles move with the
    /// new input and everything the document authored outside the theme stays
    /// exactly as written.
    #[test]
    fn re_deriving_moves_the_generated_roles_and_keeps_the_authored_ones() {
        let tokens = law();
        let rederived = tokens.with_theme_input(ThemeInput {
            contrast: 80.0,
            ..tokens.theme_input()
        });

        assert_ne!(
            tokens.color("theme.labelMuted").unwrap(),
            rederived.color("theme.labelMuted").unwrap()
        );
        assert_eq!(
            tokens.string("typography.font.human").unwrap(),
            rederived.string("typography.font.human").unwrap()
        );
    }

    #[test]
    fn prose_captures_are_declared_once_and_counted() {
        let unique: BTreeSet<_> = PROSE_CAPTURES.into_iter().collect();
        assert_eq!(unique.len(), PROSE_CAPTURES.len());
    }

    /// Every capture the highlighter can emit has somewhere to land, which is
    /// the same claim the gate makes about a product's own document.
    #[test]
    fn every_prose_capture_resolves_to_a_style() {
        let tokens = law();
        for capture in PROSE_CAPTURES {
            assert!(
                tokens.prose_highlight_style(capture).is_some(),
                "prose capture {capture} did not resolve"
            );
        }
    }
}
