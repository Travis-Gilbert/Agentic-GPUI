//! Renderer-free design-token law and deterministic emitters for Theorem.

mod color;
mod emit_css;
mod emit_gpui;
mod grain;
mod prose;
mod texture;
mod tokens;

use std::sync::OnceLock;

pub use color::Rgba;
pub use grain::{bake_grain_png, grain_parameter_hash, GrainBakeReceipt, GrainOracleReceipt};
pub use prose::{ProseHighlightStyle, PROSE_CAPTURES};
pub use texture::{DotGridParams, GrainParams};
pub use tokens::{NeutralLaw, NeutralSample, TokenSet};

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
    #[error("grain artifact I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("grain PNG encoding failed: {0}")]
    Png(#[from] png::EncodingError),
}

impl TokenSet {
    /// Embedded source of record, parsed once per process.
    #[must_use]
    pub fn builtin() -> &'static Self {
        static TOKENS: OnceLock<TokenSet> = OnceLock::new();
        TOKENS.get_or_init(|| {
            Self::from_dtcg_str(include_str!("../../../assets/design/theorem-tokens.json"))
                .expect("embedded theorem token source must be valid")
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;

    use super::*;
    use crate::emit_gpui::SEMANTIC_MAPPING;

    const TOKEN_SOURCE: &str = include_str!("../../../assets/design/theorem-tokens.json");

    fn relative_luminance(color: &Rgba) -> f64 {
        fn linearize(channel: u8) -> f64 {
            let channel = f64::from(channel) / 255.0;
            if channel <= 0.04045 {
                channel / 12.92
            } else {
                ((channel + 0.055) / 1.055).powf(2.4)
            }
        }

        0.2126 * linearize(color.r) + 0.7152 * linearize(color.g) + 0.0722 * linearize(color.b)
    }

    fn contrast_ratio(first: &Rgba, second: &Rgba) -> f64 {
        let first = relative_luminance(first);
        let second = relative_luminance(second);
        (first.max(second) + 0.05) / (first.min(second) + 0.05)
    }

    #[test]
    fn alias_resolution() {
        let tokens = TokenSet::builtin();
        assert_eq!(tokens.color("surface.page").unwrap().hex(), "#FBFAF7");
        assert_eq!(tokens.color("surface.focus").unwrap().hex(), "#C0603F");
        assert_eq!(
            tokens.color("color.ink.inverse").unwrap(),
            tokens.color("color.cream.50").unwrap()
        );
    }

    #[test]
    fn semantic_mapping() {
        let expected = [
            ("background", "#FBFAF7"),
            ("foreground", "#252421"),
            ("surface", "#FFFFFF"),
            ("surface_foreground", "#252421"),
            ("primary", "#C0603F"),
            ("primary_foreground", "#F7F5F0"),
            ("secondary", "#F1EDE5"),
            ("secondary_foreground", "#252421"),
            ("muted", "#E6E1D6"),
            ("muted_foreground", "#89867E"),
            ("accent", "#F1EDE5"),
            ("accent_foreground", "#252421"),
            ("destructive", "#A61B1B"),
            ("destructive_foreground", "#F7F5F0"),
            ("border", "#D4CFC3"),
            ("input", "#D4CFC3"),
            ("ring", "#C0603F"),
        ];
        let colors = TokenSet::builtin().semantic_colors();
        assert_eq!(colors.len(), expected.len());
        for (name, hex) in expected {
            assert_eq!(colors[name].hex(), hex, "semantic color {name}");
        }
    }

    #[test]
    fn gpui_config_schema_shape() {
        let semantic: serde_json::Value =
            serde_json::from_str(&TokenSet::builtin().emit_gpui_semantic_config()).unwrap();
        assert_eq!(semantic["tokens"]["colors"]["primary"], "#C0603F");
        assert_eq!(semantic["tokens"]["typography"]["sans"], "Manrope");
        assert_eq!(semantic["tokens"]["typography"]["mono"], "IBM Plex Mono");
        assert!(semantic["tokens"].get("shadow").is_some());
        assert!(semantic["tokens"].get("shadows").is_none());

        let legacy: serde_json::Value =
            serde_json::from_str(&TokenSet::builtin().emit_gpui_theme_config()).unwrap();
        assert_eq!(legacy["name"], "Theorem");
        assert_eq!(legacy["themes"].as_array().unwrap().len(), 1);
        let theme = &legacy["themes"][0];
        assert_eq!(theme["font.family"], "Manrope");
        assert_eq!(theme["mono_font.family"], "IBM Plex Mono");
        assert_eq!(theme["colors"].as_object().unwrap().len(), 43);
    }

    #[test]
    fn css_freshness() {
        assert_eq!(
            TokenSet::builtin().emit_css(),
            include_str!("../../../assets/design/theorem-tokens.css")
        );
    }

    #[test]
    fn css_alias_count() {
        let semantic_names: BTreeSet<_> = SEMANTIC_MAPPING
            .into_iter()
            .map(|(name, _)| format!("  --{}:", name.replace('_', "-")))
            .collect();
        let count = TokenSet::builtin()
            .emit_css()
            .lines()
            .filter(|line| {
                semantic_names.contains(line.split_once(" var(").unwrap_or((line, "")).0)
            })
            .count();
        assert_eq!(count, 17);
    }

    #[test]
    fn grain_params() {
        let params = TokenSet::builtin().grain();
        assert_eq!(params.color_back.hex(), "#FBFAF7");
        assert_eq!(params.color_front.hex(), "#B6B2A9");
        assert_eq!(params.opacity_page, 0.05);
        assert_eq!(params.opacity_sidebar, 0.07);
        assert_eq!(params.scale, 0.8);
        assert_eq!(params.speed, 0.0);
        assert_eq!(params.contrast, 1.0);
        assert_eq!(params.roughness, 1.0);
        assert_eq!(params.fiber, 0.0);
        assert_eq!(params.fiber_size, 0.2);
        assert_eq!(params.crumples, 0.0);
        assert_eq!(params.crumple_size, 0.35);
        assert_eq!(params.folds, 0.0);
        assert_eq!(params.fold_count, 1.0);
        assert_eq!(params.fade, 0.0);
        assert_eq!(params.drops, 0.0);
        assert_eq!(params.seed, 5.8);
    }

    #[test]
    fn neutral_generation_freshness() {
        let expected = [
            ("color.cream.25", "#FBFAF7"),
            ("color.cream.50", "#F7F5F0"),
            ("color.cream.100", "#F1EDE5"),
            ("color.cream.200", "#E6E1D6"),
            ("color.cream.300", "#D4CFC3"),
            ("color.cream.400", "#B6B2A9"),
            ("color.cream.700", "#595857"),
            ("color.cream.900", "#343434"),
            ("color.ink.primary", "#252421"),
            ("color.ink.muted", "#56544F"),
            ("color.ink.faint", "#89867E"),
        ];
        for (path, hex) in expected {
            assert_eq!(
                TokenSet::builtin().color(path).unwrap().hex(),
                hex,
                "{path}"
            );
        }
    }

    #[test]
    fn neutral_relative_chroma_bound() {
        let tokens = TokenSet::builtin();
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
            let sample = tokens.neutral_sample(path).unwrap();
            assert!(
                sample.chroma / sample.lightness <= 0.02 + 1e-7,
                "relative chroma bound for {path}"
            );
        }
        let peak_lightness = 6.0 / 7.0;
        let peak = tokens.neutral_law().surface_chroma(peak_lightness) / peak_lightness;
        assert!((peak - 0.02).abs() <= 1e-6, "curve peak was {peak}");
    }

    #[test]
    fn neutral_light_half_is_mint() {
        for (path, expected) in [
            ("color.cream.25", "#FBFAF7"),
            ("color.cream.50", "#F7F5F0"),
            ("color.cream.100", "#F1EDE5"),
        ] {
            assert_eq!(TokenSet::builtin().color(path).unwrap().hex(), expected);
        }
    }

    #[test]
    fn neutral_contrast_floors() {
        for (foreground, background, floor) in [
            ("color.ink.primary", "color.cream.25", 4.5),
            ("color.ink.primary", "color.cream.100", 4.5),
            ("color.ink.primary", "surface.raised", 4.5),
            ("color.ink.muted", "color.cream.25", 4.5),
            ("color.ink.faint", "color.cream.25", 3.0),
        ] {
            let foreground = TokenSet::builtin().color(foreground).unwrap();
            let background = TokenSet::builtin().color(background).unwrap();
            let ratio = contrast_ratio(&foreground, &background);
            assert!(ratio >= floor, "contrast {ratio} was below {floor}");
        }
    }

    #[test]
    fn neutral_source_contains_no_authored_hexes() {
        let source: serde_json::Value = serde_json::from_str(TOKEN_SOURCE).unwrap();
        for family in ["cream", "ink"] {
            let values = source["color"][family].as_object().unwrap();
            for (step, token) in values {
                let value = &token["$value"];
                if family == "ink" && step == "inverse" {
                    assert_eq!(value, "{color.cream.50}");
                } else {
                    assert!(
                        value
                            .get("lightness")
                            .and_then(serde_json::Value::as_f64)
                            .is_some(),
                        "{family}.{step} must declare only lightness"
                    );
                }
            }
        }
    }

    #[test]
    fn prose_captures_all_resolve() {
        let tokens = TokenSet::builtin();
        for capture in PROSE_CAPTURES {
            assert!(tokens.prose_highlight_style(capture).is_some(), "{capture}");
        }
        let emitted: serde_json::Value =
            serde_json::from_str(&tokens.emit_prose_highlight_styles()).unwrap();
        assert_eq!(emitted.as_object().unwrap().len(), PROSE_CAPTURES.len());
    }

    #[test]
    fn grain_bake_is_deterministic() {
        let temp = std::env::temp_dir();
        let pid = std::process::id();
        let first = temp.join(format!("theorem-grain-{pid}-first.png"));
        let second = temp.join(format!("theorem-grain-{pid}-second.png"));
        let first_sidecar = temp.join(format!("theorem-grain-{pid}-first.params.json"));
        let second_sidecar = temp.join(format!("theorem-grain-{pid}-second.params.json"));
        let params = TokenSet::builtin().grain();
        let first_receipt = bake_grain_png(&first, params).unwrap();
        let second_receipt = bake_grain_png(&second, params).unwrap();
        assert_eq!(first_receipt, second_receipt);
        assert_eq!(fs::read(&first).unwrap(), fs::read(&second).unwrap());
        assert_eq!(first_receipt.parameter_hash, grain_parameter_hash(params));
        assert_eq!(first_receipt.oracle.package, "@paper-design/shaders");
        assert_eq!(first_receipt.oracle.version, "0.0.77");
        assert_eq!(
            first_receipt.oracle.shader_sha256,
            "b2fa3e8281bf85f9505880056d0cec947454604f4c780e11257ffec416d7e8ef"
        );
        assert_eq!(
            first_receipt.oracle.noise_sha256,
            "5116a06c428a75e2db9bd55062c560bb02600383ee54da007f1628e845b2b73a"
        );
        assert_eq!(
            first_receipt.oracle.bake_contract,
            "paperTexture-static-opaque-page-v1"
        );
        for path in [first, second, first_sidecar, second_sidecar] {
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn grain_hash_covers_every_shader_parameter() {
        let baseline = TokenSet::builtin().grain();
        let baseline_hash = grain_parameter_hash(baseline);
        let mutations: [fn(&mut GrainParams); 17] = [
            |value| value.color_back.r ^= 1,
            |value| value.color_front.r ^= 1,
            |value| value.opacity_page += 0.001,
            |value| value.opacity_sidebar += 0.001,
            |value| value.scale += 0.001,
            |value| value.speed += 0.001,
            |value| value.contrast += 0.001,
            |value| value.roughness += 0.001,
            |value| value.fiber += 0.001,
            |value| value.fiber_size += 0.001,
            |value| value.crumples += 0.001,
            |value| value.crumple_size += 0.001,
            |value| value.folds += 0.001,
            |value| value.fold_count += 0.001,
            |value| value.fade += 0.001,
            |value| value.drops += 0.001,
            |value| value.seed += 0.001,
        ];
        for mutate in mutations {
            let mut changed = baseline;
            mutate(&mut changed);
            assert_ne!(grain_parameter_hash(changed), baseline_hash);
        }
    }

    #[test]
    fn authored_neutral_hex_is_rejected() {
        let source = TOKEN_SOURCE.replacen("{ \"lightness\": 0.9850 }", "\"#FBFAF7\"", 1);
        assert!(matches!(
            TokenSet::from_dtcg_str(&source),
            Err(TokenError::AuthoredNeutral(path)) if path == "color.cream.25"
        ));
    }

    #[test]
    fn malformed_alias_is_rejected_during_parse() {
        let source = TOKEN_SOURCE.replacen("{surface.page}", "{missing.surface}", 1);
        assert!(matches!(
            TokenSet::from_dtcg_str(&source),
            Err(TokenError::MissingToken(path)) if path == "missing.surface"
        ));
    }
}
