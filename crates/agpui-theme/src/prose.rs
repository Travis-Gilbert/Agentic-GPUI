use std::collections::BTreeMap;

use crate::{Rgba, TokenSet};

pub const PROSE_CAPTURES: [&str; 15] = [
    "prose.noun",
    "prose.verb",
    "prose.adjective",
    "prose.adverb",
    "prose.conjunction",
    "prose.pronoun",
    "prose.sentence.long",
    "prose.sentence.short",
    "prose.word.frequent.1",
    "prose.word.frequent.2",
    "prose.word.frequent.3",
    "prose.word.frequent.4",
    "prose.word.frequent.5",
    "prose.voice.passive",
    "prose.filler",
];

/// A renderer-neutral prose capture style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProseHighlightStyle {
    pub foreground: Option<Rgba>,
    pub background: Option<Rgba>,
    pub underline: bool,
    pub font_weight: Option<u16>,
}

impl TokenSet {
    /// Resolve one capture from the addendum's stable prose contract.
    #[must_use]
    pub fn prose_highlight_style(&self, capture: &str) -> Option<ProseHighlightStyle> {
        let color = |path: &str| self.color(path).ok();
        let translucent = |path: &str, alpha: f32| {
            color(path).map(|mut value| {
                value.a = alpha;
                value
            })
        };
        match capture {
            "prose.noun" => foreground(color("color.agent.600")),
            "prose.verb" => foreground(color("color.human.600")),
            "prose.adjective" => foreground(color("color.insight.gold")),
            "prose.adverb" => foreground(color("color.agent.500")),
            "prose.conjunction" => foreground(color("color.ink.faint")),
            "prose.pronoun" => foreground(color("color.human.500")),
            "prose.sentence.long" => background(translucent("color.human.100", 0.38)),
            "prose.sentence.short" => background(translucent("color.cream.100", 0.62)),
            "prose.word.frequent.1" => background(translucent("color.human.100", 0.18)),
            "prose.word.frequent.2" => background(translucent("color.human.100", 0.28)),
            "prose.word.frequent.3" => background(translucent("color.human.200", 0.34)),
            "prose.word.frequent.4" => background(translucent("color.human.200", 0.44)),
            "prose.word.frequent.5" => background(translucent("color.human.400", 0.36)),
            "prose.voice.passive" => advisory(color("color.agent.600")),
            "prose.filler" => advisory(color("color.human.600")),
            _ => None,
        }
    }

    /// Deterministic JSON projection consumed by renderer adapters.
    #[must_use]
    pub fn emit_prose_highlight_styles(&self) -> String {
        let mut styles = BTreeMap::new();
        for capture in PROSE_CAPTURES {
            let style = self
                .prose_highlight_style(capture)
                .unwrap_or_else(|| panic!("prose capture {capture} must resolve"));
            styles.insert(
                capture,
                serde_json::json!({
                    "foreground": style.foreground.map(|color| color.hex()),
                    "background": style.background.map(|color| color.hex()),
                    "underline": style.underline,
                    "font_weight": style.font_weight,
                }),
            );
        }
        let mut output = serde_json::to_string_pretty(&styles)
            .expect("prose highlight styles contain only serializable values");
        output.push('\n');
        output
    }
}

fn foreground(color: Option<Rgba>) -> Option<ProseHighlightStyle> {
    Some(ProseHighlightStyle {
        foreground: color,
        background: None,
        underline: false,
        font_weight: None,
    })
}

fn background(color: Option<Rgba>) -> Option<ProseHighlightStyle> {
    Some(ProseHighlightStyle {
        foreground: None,
        background: color,
        underline: false,
        font_weight: None,
    })
}

fn advisory(color: Option<Rgba>) -> Option<ProseHighlightStyle> {
    Some(ProseHighlightStyle {
        foreground: color,
        background: None,
        underline: true,
        font_weight: Some(500),
    })
}
