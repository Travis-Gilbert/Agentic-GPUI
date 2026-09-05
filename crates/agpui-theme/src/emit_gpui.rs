use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::{Rgba, TokenSet};

pub const SEMANTIC_MAPPING: [(&str, &str); 17] = [
    ("background", "surface.page"),
    ("foreground", "color.ink.primary"),
    ("surface", "surface.raised"),
    ("surface_foreground", "color.ink.primary"),
    ("primary", "color.human.500"),
    ("primary_foreground", "color.ink.inverse"),
    ("secondary", "color.cream.100"),
    ("secondary_foreground", "color.ink.primary"),
    ("muted", "color.cream.200"),
    ("muted_foreground", "color.ink.faint"),
    ("accent", "color.cream.100"),
    ("accent_foreground", "color.ink.primary"),
    ("destructive", "color.status.error"),
    ("destructive_foreground", "color.ink.inverse"),
    ("border", "surface.border"),
    ("input", "color.cream.300"),
    ("ring", "surface.focus"),
];

const LEGACY_MAPPING: [(&str, &str); 43] = [
    ("background", "surface.page"),
    ("foreground", "color.ink.primary"),
    ("border", "surface.border"),
    ("input.border", "surface.border"),
    ("ring", "surface.focus"),
    ("primary.background", "color.human.500"),
    ("primary.foreground", "color.ink.inverse"),
    ("primary.hover.background", "color.human.600"),
    ("primary.active.background", "color.human.700"),
    ("secondary.background", "color.cream.100"),
    ("secondary.foreground", "color.ink.primary"),
    ("secondary.hover.background", "color.cream.200"),
    ("secondary.active.background", "color.cream.300"),
    ("muted.background", "color.cream.200"),
    ("muted.foreground", "color.ink.faint"),
    ("accent.background", "color.cream.100"),
    ("accent.foreground", "color.ink.primary"),
    ("danger.background", "color.status.error"),
    ("danger.foreground", "color.ink.inverse"),
    ("success.background", "color.status.success"),
    ("warning.background", "color.status.warning"),
    ("sidebar.background", "color.cream.100"),
    ("sidebar.foreground", "color.ink.primary"),
    ("sidebar.border", "color.cream.300"),
    ("sidebar.accent.background", "color.cream.200"),
    ("sidebar.primary.background", "color.human.500"),
    ("title_bar.background", "color.cream.50"),
    ("title_bar.border", "color.cream.300"),
    ("status_bar.background", "color.cream.50"),
    ("status_bar.border", "color.cream.300"),
    ("tab_bar.background", "color.cream.100"),
    ("tab.foreground", "color.ink.muted"),
    ("tab.active.background", "color.cream.25"),
    ("tab.active.foreground", "color.ink.primary"),
    ("list.background", "color.cream.25"),
    ("list.hover.background", "color.cream.100"),
    ("table.background", "surface.raised"),
    ("popover.background", "surface.raised"),
    ("popover.foreground", "color.ink.primary"),
    ("caret", "color.human.500"),
    ("selection.background", "color.human.200"),
    ("link", "color.agent.600"),
    ("window.border", "color.cream.400"),
];

impl TokenSet {
    #[must_use]
    pub fn semantic_colors(&self) -> BTreeMap<&'static str, Rgba> {
        SEMANTIC_MAPPING
            .into_iter()
            .map(|(semantic, path)| {
                (
                    semantic,
                    self.color(path).unwrap_or_else(|error| {
                        panic!("semantic token {semantic} from {path} must resolve: {error}")
                    }),
                )
            })
            .collect()
    }

    #[must_use]
    pub fn emit_gpui_semantic_config(&self) -> String {
        let colors: BTreeMap<_, _> = self
            .semantic_colors()
            .into_iter()
            .map(|(name, color)| (name, color.hex()))
            .collect();
        let config = json!({
            "tokens": {
                "colors": colors,
                "radius": { "none": 0, "sm": 3, "md": 4, "lg": 6, "xl": 8, "full": 9999 },
                "spacing": { "xxs": 2, "xs": 4, "sm": 8, "md": 12, "lg": 16, "xl": 24, "xxl": 32 },
                "typography": {
                    "sans": first_font_family(self, "typography.font.human"),
                    "mono": first_font_family(self, "typography.font.mono"),
                    "xs": { "size": 11.5, "line_height": 16 },
                    "sm": { "size": 13, "line_height": 18 },
                    "md": { "size": 14, "line_height": 22 },
                    "lg": { "size": 16, "line_height": 22 },
                    "xl": { "size": 22, "line_height": 28 },
                    "mono_md": { "size": 13, "line_height": 18 }
                },
                "shadow": {}
            }
        });
        pretty_json(&config)
    }

    #[must_use]
    pub fn emit_gpui_theme_config(&self) -> String {
        let colors: BTreeMap<_, _> = LEGACY_MAPPING
            .into_iter()
            .map(|(name, path)| {
                (
                    name,
                    self.color(path)
                        .unwrap_or_else(|error| panic!("legacy token {name} must resolve: {error}"))
                        .hex(),
                )
            })
            .collect();
        let theme = json!({
            "is_default": true,
            "name": "Theorem",
            "mode": "light",
            "colors": colors,
            "font.family": first_font_family(self, "typography.font.human"),
            "font.size": 13,
            "mono_font.family": first_font_family(self, "typography.font.mono"),
            "mono_font.size": 12,
            "radius": 4,
            "radius.lg": 6,
            "shadow": true
        });
        pretty_json(&json!({ "name": "Theorem", "themes": [theme] }))
    }
}

fn pretty_json(value: &Value) -> String {
    let mut output = serde_json::to_string_pretty(value)
        .expect("GPUI theme config contains only serializable values");
    output.push('\n');
    output
}

fn first_font_family(tokens: &TokenSet, path: &str) -> String {
    tokens
        .string(path)
        .unwrap_or_else(|error| panic!("font token {path} must resolve: {error}"))
        .split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches('\'')
        .to_owned()
}
