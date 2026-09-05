use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::{Rgba, TokenSet};

/// Every semantic role this crate emits, paired with the token path it reads.
///
/// Public because the emitter's contract is the table, not the function: a
/// product that emits from this crate has to be able to assert that every role
/// it declares came out the other side, and those assertions live with the
/// product's token file rather than here.
pub const SEMANTIC_MAPPING: [(&str, &str); 17] = [
    ("background", "theme.bgBase"),
    ("foreground", "theme.labelTitle"),
    ("surface", "theme.bgBase"),
    ("surface_foreground", "theme.labelBase"),
    ("primary", "theme.controlPrimary"),
    ("primary_foreground", "theme.controlPrimaryLabel"),
    ("secondary", "theme.controlSecondary"),
    ("secondary_foreground", "theme.controlSecondaryLabel"),
    ("muted", "theme.bgShade"),
    ("muted_foreground", "theme.labelFaint"),
    ("accent", "theme.bgSelected"),
    ("accent_foreground", "theme.labelTitle"),
    ("destructive", "theme.errorBase"),
    ("destructive_foreground", "theme.errorForeground"),
    ("border", "theme.bgBorder"),
    ("input", "theme.bgBorder"),
    ("ring", "theme.focus"),
];

const LEGACY_MAPPING: [(&str, &str); 43] = [
    ("background", "theme.bgBase"),
    ("foreground", "theme.labelTitle"),
    ("border", "theme.bgBorder"),
    ("input.border", "theme.bgBorder"),
    ("ring", "theme.focus"),
    ("primary.background", "theme.controlPrimary"),
    ("primary.foreground", "theme.controlPrimaryLabel"),
    ("primary.hover.background", "theme.controlPrimaryHover"),
    ("primary.active.background", "theme.controlPrimaryHover"),
    ("secondary.background", "theme.controlSecondary"),
    ("secondary.foreground", "theme.controlSecondaryLabel"),
    ("secondary.hover.background", "theme.controlSecondaryHover"),
    (
        "secondary.active.background",
        "theme.controlSecondarySelected",
    ),
    ("muted.background", "theme.bgShade"),
    ("muted.foreground", "theme.labelFaint"),
    ("accent.background", "theme.bgSelected"),
    ("accent.foreground", "theme.labelTitle"),
    ("danger.background", "theme.errorBase"),
    ("danger.foreground", "theme.errorForeground"),
    ("success.background", "theme.successBase"),
    ("warning.background", "theme.warningBase"),
    ("sidebar.background", "theme.bgSub"),
    ("sidebar.foreground", "theme.labelBase"),
    ("sidebar.border", "theme.bgBorderFaint"),
    ("sidebar.accent.background", "theme.bgSelected"),
    ("sidebar.primary.background", "theme.controlPrimary"),
    ("title_bar.background", "theme.bgBase"),
    ("title_bar.border", "theme.bgBorderFaint"),
    ("status_bar.background", "theme.bgSub"),
    ("status_bar.border", "theme.bgBorderFaint"),
    ("tab_bar.background", "theme.bgSub"),
    ("tab.foreground", "theme.labelMuted"),
    ("tab.active.background", "theme.chromeTabBgActive"),
    ("tab.active.foreground", "theme.labelTitle"),
    ("list.background", "theme.bgBase"),
    ("list.hover.background", "theme.bgBaseHover"),
    ("table.background", "theme.bgBase"),
    ("popover.background", "theme.bgBase"),
    ("popover.foreground", "theme.labelTitle"),
    ("caret", "theme.controlPrimary"),
    ("selection.background", "theme.bgSelected"),
    ("link", "theme.labelLink"),
    ("window.border", "theme.bgBorderStrong"),
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
        let shadow_channel = if self.derived_theme().is_light {
            0.0
        } else {
            1.0
        };
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
                "shadow": {
                    "sm": [shadow(0.0, 1.0, 1.0, 0.0, shadow_channel, 0.06)],
                    "md": [
                        shadow(0.0, 3.0, 4.0, -1.0, shadow_channel, 0.08),
                        shadow(0.0, 1.0, 1.0, 0.0, shadow_channel, 0.06)
                    ],
                    "lg": [
                        shadow(0.0, 9.0, 24.0, -3.0, shadow_channel, 0.10),
                        shadow(0.0, 3.0, 4.0, -2.0, shadow_channel, 0.08)
                    ]
                }
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
        let mode = if self.derived_theme().is_light {
            "light"
        } else {
            "dark"
        };
        let theme = json!({
            "is_default": true,
            "name": "Theorem",
            "mode": mode,
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

fn shadow(x: f32, y: f32, blur: f32, spread: f32, channel: f32, alpha: f32) -> Value {
    let channel = if channel > 0.5 { 255 } else { 0 };
    let alpha = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
    json!({
        "color": format!("#{channel:02X}{channel:02X}{channel:02X}{alpha:02X}"),
        "offset": { "x": x, "y": y },
        "blur_radius": blur,
        "spread_radius": spread,
        "inset": false
    })
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
