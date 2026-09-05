use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{CieLch, DerivedTheme, Rgba, ThemeInput, TokenError};

/// Fully resolved renderer-neutral token set.
///
/// Theme authors provide only `theme.base`, `theme.accent`, and
/// `theme.contrast`. The Linear-compatible law expands those inputs into the
/// `theme.*` role registry before aliases are resolved.
#[derive(Clone, Debug)]
pub struct TokenSet {
    input: ThemeInput,
    derived: DerivedTheme,
    raw: BTreeMap<String, Value>,
}

impl TokenSet {
    /// Parse a DTCG source, generate the complete theme, and eagerly resolve
    /// every alias.
    ///
    /// # Errors
    ///
    /// Returns a typed error for malformed JSON, missing theme inputs,
    /// invalid colors, missing aliases, or cyclic aliases.
    pub fn from_dtcg_str(source: &str) -> Result<Self, TokenError> {
        let root: Value = serde_json::from_str(source)?;
        let mut raw = BTreeMap::new();
        collect_tokens(&root, "", &mut raw);

        let theme_string = |path: &str| -> Result<&str, TokenError> {
            raw.get(path)
                .and_then(Value::as_str)
                .ok_or_else(|| TokenError::MissingToken(path.to_owned()))
        };
        let contrast = raw
            .get("theme.contrast")
            .and_then(Value::as_f64)
            .ok_or_else(|| TokenError::MissingToken("theme.contrast".to_owned()))?
            as f32;
        if !contrast.is_finite() || !(0.0..=100.0).contains(&contrast) {
            return Err(TokenError::InvalidThemeFile(
                "contrast must be finite and in 0..=100".to_owned(),
            ));
        }
        let input = ThemeInput {
            base: CieLch::parse_css(theme_string("theme.base")?)?,
            accent: CieLch::parse_css(theme_string("theme.accent")?)?,
            contrast,
            hairlines: true,
        };
        let derived = input.derive();
        for (name, color) in derived.colors() {
            raw.insert(format!("theme.{name}"), Value::String(color.hex()));
        }

        let raw = raw
            .keys()
            .map(|path| {
                resolve_alias_value(path, &raw, &mut BTreeSet::new())
                    .map(|value| (path.clone(), value))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        Ok(Self {
            input,
            derived,
            raw,
        })
    }

    #[must_use]
    pub const fn theme_input(&self) -> ThemeInput {
        self.input
    }

    #[must_use]
    pub const fn derived_theme(&self) -> &DerivedTheme {
        &self.derived
    }

    /// Re-derive this token registry for another three-field theme input.
    ///
    /// Authored typography, density, and texture metadata is retained. GPUI
    /// mappings consume generated `theme.*` roles directly, so no renderer
    /// needs a parallel dark palette.
    #[must_use]
    pub fn with_theme_input(&self, input: ThemeInput) -> Self {
        let derived = input.derive();
        let mut raw = self.raw.clone();
        raw.insert("theme.base".to_owned(), Value::String(input.base.to_css()));
        raw.insert(
            "theme.accent".to_owned(),
            Value::String(input.accent.to_css()),
        );
        raw.insert(
            "theme.contrast".to_owned(),
            Value::from(f64::from(input.contrast)),
        );
        for (name, color) in derived.colors() {
            raw.insert(format!("theme.{name}"), Value::String(color.hex()));
        }
        Self {
            input,
            derived,
            raw,
        }
    }

    /// Resolve a color token or generated role.
    ///
    /// # Errors
    ///
    /// Returns a typed error for missing paths or non-color values.
    pub fn color(&self, path: &str) -> Result<Rgba, TokenError> {
        let value = self
            .raw
            .get(path)
            .ok_or_else(|| TokenError::MissingToken(path.to_owned()))?;
        let string = value
            .as_str()
            .ok_or_else(|| TokenError::InvalidToken(path.to_owned()))?;
        parse_color(string)
    }

    /// Resolve a number token. CSS `px` suffixes are stripped at this typed
    /// boundary; the source representation is retained for CSS emission.
    pub fn number(&self, path: &str) -> Result<f32, TokenError> {
        let value = self
            .raw
            .get(path)
            .ok_or_else(|| TokenError::MissingToken(path.to_owned()))?;
        if let Some(number) = value.as_f64() {
            return Ok(number as f32);
        }
        value
            .as_str()
            .and_then(|string| string.strip_suffix("px").unwrap_or(string).parse().ok())
            .ok_or_else(|| TokenError::InvalidToken(path.to_owned()))
    }

    pub fn string(&self, path: &str) -> Result<String, TokenError> {
        self.raw
            .get(path)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| TokenError::InvalidToken(path.to_owned()))
    }

    pub(crate) fn token_paths(&self) -> impl Iterator<Item = &str> {
        self.raw.keys().map(String::as_str)
    }

    pub(crate) fn css_value(&self, path: &str) -> Result<String, TokenError> {
        let value = self
            .raw
            .get(path)
            .ok_or_else(|| TokenError::MissingToken(path.to_owned()))?;
        if let Some(string) = value.as_str() {
            if let Ok(color) = parse_color(string) {
                return Ok(color.hex());
            }
            return Ok(string.to_owned());
        }
        match value {
            Value::Number(number) => Ok(number.to_string()),
            Value::Array(values) => values
                .iter()
                .map(resolve_css_array_value)
                .collect::<Result<Vec<_>, _>>()
                .map(|values| values.join(", ")),
            _ => Err(TokenError::InvalidToken(path.to_owned())),
        }
    }
}

fn parse_color(value: &str) -> Result<Rgba, TokenError> {
    Rgba::parse(value).or_else(|_| CieLch::parse_css(value).map(CieLch::to_rgba))
}

fn resolve_css_array_value(value: &Value) -> Result<String, TokenError> {
    if let Some(string) = value.as_str() {
        return parse_color(string)
            .map(|color| color.hex())
            .or_else(|_| Ok(string.to_owned()));
    }
    Ok(value.to_string())
}

fn alias_path(value: &str) -> Option<&str> {
    value.strip_prefix('{')?.strip_suffix('}')
}

fn resolve_alias_value(
    path: &str,
    raw: &BTreeMap<String, Value>,
    seen: &mut BTreeSet<String>,
) -> Result<Value, TokenError> {
    if !seen.insert(path.to_owned()) {
        return Err(TokenError::AliasCycle(path.to_owned()));
    }
    let value = raw
        .get(path)
        .ok_or_else(|| TokenError::MissingToken(path.to_owned()))?;
    resolve_nested_aliases(value, raw, seen)
}

fn resolve_nested_aliases(
    value: &Value,
    raw: &BTreeMap<String, Value>,
    seen: &mut BTreeSet<String>,
) -> Result<Value, TokenError> {
    match value {
        Value::String(string) => alias_path(string).map_or_else(
            || Ok(value.clone()),
            |alias| resolve_alias_value(alias, raw, seen),
        ),
        Value::Array(values) => values
            .iter()
            .map(|value| resolve_nested_aliases(value, raw, &mut seen.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(values) => values
            .iter()
            .map(|(key, value)| {
                resolve_nested_aliases(value, raw, &mut seen.clone())
                    .map(|value| (key.clone(), value))
            })
            .collect::<Result<serde_json::Map<_, _>, _>>()
            .map(Value::Object),
        _ => Ok(value.clone()),
    }
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
