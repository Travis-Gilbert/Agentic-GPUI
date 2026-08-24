use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::color::{oklch_to_rgba, Oklch};
use crate::{Rgba, TokenError};

/// The only law allowed to generate cream and ink colors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeutralLaw {
    pub hue: f32,
    pub surface_a: f32,
    pub surface_exponent: f32,
    pub ink_slope: f32,
    pub max_relative_chroma: f32,
}

impl NeutralLaw {
    #[must_use]
    pub fn surface_chroma(&self, lightness: f32) -> f32 {
        self.surface_a * (1.0 - lightness) * lightness.powf(self.surface_exponent)
    }

    #[must_use]
    pub fn ink_chroma(&self, lightness: f32) -> f32 {
        self.ink_slope * lightness
    }

    fn bounded_chroma(&self, lightness: f32, chroma: f32) -> f32 {
        chroma.min(self.max_relative_chroma * lightness)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeutralSample {
    pub lightness: f32,
    pub chroma: f32,
    pub color: Rgba,
}

/// Fully resolved renderer-neutral token set.
#[derive(Clone, Debug)]
pub struct TokenSet {
    law: NeutralLaw,
    raw: BTreeMap<String, Value>,
    neutrals: BTreeMap<String, NeutralSample>,
}

impl TokenSet {
    /// Parse a DTCG source and derive all neutral colors.
    ///
    /// # Errors
    ///
    /// Returns a typed error for malformed JSON, missing law decisions,
    /// authored neutral hexes, or invalid token values.
    pub fn from_dtcg_str(source: &str) -> Result<Self, TokenError> {
        let root: Value = serde_json::from_str(source)?;
        let mut raw = BTreeMap::new();
        collect_tokens(&root, "", &mut raw);
        let number = |path: &str| -> Result<f32, TokenError> {
            raw.get(path)
                .and_then(Value::as_f64)
                .map(|value| value as f32)
                .ok_or_else(|| TokenError::MissingToken(path.to_owned()))
        };
        let law = NeutralLaw {
            hue: number("color.neutral.hue")?,
            surface_a: number("color.neutral.surfaceA")?,
            surface_exponent: number("color.neutral.surfaceExponent")?,
            ink_slope: number("color.neutral.inkSlope")?,
            max_relative_chroma: number("color.neutral.maxRelativeChroma")?,
        };

        let mut neutrals = BTreeMap::new();
        for path in [
            "color.cream.25",
            "color.cream.50",
            "color.cream.100",
            "color.cream.200",
            "color.cream.300",
            "color.cream.400",
            "color.cream.700",
            "color.cream.900",
        ] {
            let lightness = neutral_lightness(&raw, path)?;
            let chroma = law.bounded_chroma(lightness, law.surface_chroma(lightness));
            insert_neutral(&mut neutrals, path, lightness, chroma, law.hue);
        }
        for path in ["color.ink.primary", "color.ink.muted", "color.ink.faint"] {
            let lightness = neutral_lightness(&raw, path)?;
            let chroma = law.bounded_chroma(lightness, law.ink_chroma(lightness));
            insert_neutral(&mut neutrals, path, lightness, chroma, law.hue);
        }

        Ok(Self { law, raw, neutrals })
    }

    #[must_use]
    pub fn neutral_law(&self) -> NeutralLaw {
        self.law
    }

    #[must_use]
    pub fn neutral_sample(&self, path: &str) -> Option<NeutralSample> {
        self.neutrals.get(path).copied()
    }

    /// Resolve a color token or alias.
    ///
    /// # Errors
    ///
    /// Returns a typed error for missing paths, cycles, or non-color values.
    pub fn color(&self, path: &str) -> Result<Rgba, TokenError> {
        self.color_inner(path, &mut BTreeSet::new())
    }

    fn color_inner(&self, path: &str, seen: &mut BTreeSet<String>) -> Result<Rgba, TokenError> {
        if let Some(sample) = self.neutrals.get(path) {
            return Ok(sample.color);
        }
        if !seen.insert(path.to_owned()) {
            return Err(TokenError::AliasCycle(path.to_owned()));
        }
        let value = self
            .raw
            .get(path)
            .ok_or_else(|| TokenError::MissingToken(path.to_owned()))?;
        let string = value
            .as_str()
            .ok_or_else(|| TokenError::InvalidToken(path.to_owned()))?;
        if let Some(alias) = alias_path(string) {
            self.color_inner(alias, seen)
        } else {
            Rgba::parse(string)
        }
    }

    /// Resolve a number token. CSS `px` suffixes are stripped at this typed
    /// boundary; the source representation is retained for CSS emission.
    pub fn number(&self, path: &str) -> Result<f32, TokenError> {
        let value = self.resolve_raw(path, &mut BTreeSet::new())?;
        if let Some(number) = value.as_f64() {
            return Ok(number as f32);
        }
        value
            .as_str()
            .and_then(|string| string.strip_suffix("px").unwrap_or(string).parse().ok())
            .ok_or_else(|| TokenError::InvalidToken(path.to_owned()))
    }

    pub fn string(&self, path: &str) -> Result<String, TokenError> {
        let value = self.resolve_raw(path, &mut BTreeSet::new())?;
        value
            .as_str()
            .map(ToOwned::to_owned)
            .ok_or_else(|| TokenError::InvalidToken(path.to_owned()))
    }

    pub(crate) fn token_paths(&self) -> impl Iterator<Item = &str> {
        self.raw.keys().map(String::as_str)
    }

    pub(crate) fn css_value(&self, path: &str) -> Result<String, TokenError> {
        if self.neutrals.contains_key(path)
            || path.starts_with("surface.")
            || path.starts_with("actor.") && path.ends_with("accent")
            || path.starts_with("color.") && !path.starts_with("color.neutral.")
            || path.ends_with("colorBack")
            || path.ends_with("colorFront")
            || path.ends_with(".color")
        {
            if let Ok(color) = self.color(path) {
                return Ok(color.hex());
            }
        }
        let value = self.resolve_raw(path, &mut BTreeSet::new())?;
        match value {
            Value::String(string) => Ok(string.clone()),
            Value::Number(number) => Ok(number.to_string()),
            Value::Array(values) => values
                .iter()
                .map(|value| self.resolve_css_array_value(value))
                .collect::<Result<Vec<_>, _>>()
                .map(|values| values.join(", ")),
            Value::Object(object) if object.contains_key("lightness") => {
                self.color(path).map(|color| color.hex())
            }
            _ => Err(TokenError::InvalidToken(path.to_owned())),
        }
    }

    fn resolve_css_array_value(&self, value: &Value) -> Result<String, TokenError> {
        if let Some(string) = value.as_str() {
            if let Some(alias) = alias_path(string) {
                return self
                    .color(alias)
                    .map(|color| color.hex())
                    .or_else(|_| self.string(alias));
            }
            return Ok(string.to_owned());
        }
        Ok(value.to_string())
    }

    fn resolve_raw<'a>(
        &'a self,
        path: &str,
        seen: &mut BTreeSet<String>,
    ) -> Result<&'a Value, TokenError> {
        if !seen.insert(path.to_owned()) {
            return Err(TokenError::AliasCycle(path.to_owned()));
        }
        let value = self
            .raw
            .get(path)
            .ok_or_else(|| TokenError::MissingToken(path.to_owned()))?;
        if let Some(alias) = value.as_str().and_then(alias_path) {
            self.resolve_raw(alias, seen)
        } else {
            Ok(value)
        }
    }
}

fn insert_neutral(
    neutrals: &mut BTreeMap<String, NeutralSample>,
    path: &str,
    lightness: f32,
    chroma: f32,
    hue: f32,
) {
    let color = oklch_to_rgba(Oklch {
        l: f64::from(lightness),
        c: f64::from(chroma),
        h: f64::from(hue),
    });
    neutrals.insert(
        path.to_owned(),
        NeutralSample {
            lightness,
            chroma,
            color,
        },
    );
}

fn neutral_lightness(raw: &BTreeMap<String, Value>, path: &str) -> Result<f32, TokenError> {
    let value = raw
        .get(path)
        .ok_or_else(|| TokenError::MissingToken(path.to_owned()))?;
    if value.as_str().is_some_and(|value| value.starts_with('#')) {
        return Err(TokenError::AuthoredNeutral(path.to_owned()));
    }
    value
        .get("lightness")
        .and_then(Value::as_f64)
        .map(|value| value as f32)
        .ok_or_else(|| TokenError::InvalidToken(path.to_owned()))
}

fn alias_path(value: &str) -> Option<&str> {
    value.strip_prefix('{')?.strip_suffix('}')
}

fn collect_tokens(value: &Value, prefix: &str, output: &mut BTreeMap<String, Value>) {
    let Some(object) = value.as_object() else {
        return;
    };
    if let Some(value) = object.get("$value") {
        output.insert(prefix.to_owned(), value.clone());
        return;
    }
    for (key, value) in object {
        if key.starts_with('$') {
            continue;
        }
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        collect_tokens(value, &path, output);
    }
}
