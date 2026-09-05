//! Linear-compatible generated theme law.
//!
//! Linear performs its adjustments in CSS CIE LCH, not OKLCH. The original
//! handoff called the public coordinate `Oklch`; [`Oklch`] remains as a type
//! alias for source compatibility, while the canonical name is [`CieLch`].

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Rgba, TokenError};

const APCA_SOFT_CLAMP: f64 = 0.022;
const APCA_SOFT_EXPONENT: f64 = 1.414;
const MIN_TEXT_APCA: f32 = 38.0;

/// A CSS `lch()` colour in the CIE L*C*h colour space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CieLch {
    /// Perceptual lightness, in the CSS range `0..=100`.
    pub l: f64,
    /// CIE chroma, in the Linear source range `0..=132`.
    pub c: f64,
    /// Hue angle in degrees.
    pub h: f64,
}

/// Compatibility spelling from HANDOFF-THEOREMWEB-SHELL-1.0.
///
/// The coordinates are CIE LCH. New code should prefer [`CieLch`].
pub type Oklch = CieLch;

impl CieLch {
    #[must_use]
    pub const fn new(l: f64, c: f64, h: f64) -> Self {
        Self { l, c, h }
    }

    /// Parse the three-input theme colour syntax, `lch(L% C H)`.
    pub fn parse_css(value: &str) -> Result<Self, TokenError> {
        let body = value
            .trim()
            .strip_prefix("lch(")
            .and_then(|value| value.strip_suffix(')'))
            .ok_or_else(|| TokenError::InvalidThemeColor(value.to_owned()))?;
        let mut parts = body.split_whitespace();
        let l = parts
            .next()
            .and_then(|value| value.strip_suffix('%'))
            .and_then(|value| value.parse::<f64>().ok());
        let c = parts.next().and_then(|value| value.parse::<f64>().ok());
        let h = parts.next().and_then(|value| value.parse::<f64>().ok());
        if parts.next().is_some() {
            return Err(TokenError::InvalidThemeColor(value.to_owned()));
        }
        match (l, c, h) {
            (Some(l), Some(c), Some(h))
                if (0.0..=100.0).contains(&l) && (0.0..=132.0).contains(&c) && h.is_finite() =>
            {
                Ok(Self::new(l, c, normalize_hue(h)))
            }
            _ => Err(TokenError::InvalidThemeColor(value.to_owned())),
        }
    }

    #[must_use]
    pub fn to_css(self) -> String {
        format!("lch({:.3}% {:.3} {:.3})", self.l, self.c, self.h)
    }

    /// Convert this CSS CIE LCH color to a gamut-fitted sRGB color.
    #[must_use]
    pub fn to_rgba(self) -> Rgba {
        to_rgba(self.into())
    }
}

/// The three user-authored theme inputs plus the renderer hairline policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThemeInput {
    pub base: CieLch,
    pub accent: CieLch,
    pub contrast: f32,
    pub hairlines: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ThemeFile {
    base: String,
    accent: String,
    contrast: f32,
}

impl ThemeInput {
    #[must_use]
    pub const fn theorem_default() -> Self {
        Self {
            base: CieLch::new(98.2, 0.6, 260.0),
            accent: CieLch::new(52.3, 52.7, 44.7),
            contrast: 30.0,
            hairlines: true,
        }
    }

    #[must_use]
    pub const fn theorem_dark() -> Self {
        Self {
            base: CieLch::new(7.5, 0.8, 265.0),
            ..Self::theorem_default()
        }
    }

    #[must_use]
    pub const fn linear_reference() -> Self {
        Self {
            base: CieLch::new(97.94, 0.5, 282.0),
            accent: CieLch::new(53.0, 52.26, 286.91),
            contrast: 30.0,
            hairlines: true,
        }
    }

    /// Derive the complete palette with the same adjustment law as Linear's
    /// `darkThemeRefresh` generator.
    #[must_use]
    pub fn derive(&self) -> DerivedTheme {
        ThemeLaw::new(*self).derive()
    }

    pub fn from_json_str(source: &str) -> Result<Self, TokenError> {
        let file: ThemeFile = serde_json::from_str(source)?;
        Self::from_file(file)
    }

    pub fn from_toml_str(source: &str) -> Result<Self, TokenError> {
        let file: ThemeFile = toml::from_str(source)
            .map_err(|error| TokenError::InvalidThemeFile(error.to_string()))?;
        Self::from_file(file)
    }

    pub fn to_json(&self) -> Result<String, TokenError> {
        serde_json::to_string_pretty(&self.as_file()).map_err(TokenError::from)
    }

    pub fn to_toml(&self) -> Result<String, TokenError> {
        toml::to_string_pretty(&self.as_file())
            .map_err(|error| TokenError::InvalidThemeFile(error.to_string()))
    }

    fn from_file(file: ThemeFile) -> Result<Self, TokenError> {
        if !(0.0..=100.0).contains(&file.contrast) {
            return Err(TokenError::InvalidThemeFile(
                "contrast must be in 0..=100".to_owned(),
            ));
        }
        Ok(Self {
            base: CieLch::parse_css(&file.base)?,
            accent: CieLch::parse_css(&file.accent)?,
            contrast: file.contrast,
            hairlines: true,
        })
    }

    fn as_file(&self) -> ThemeFile {
        ThemeFile {
            base: self.base.to_css(),
            accent: self.accent.to_css(),
            contrast: self.contrast,
        }
    }
}

impl Default for ThemeInput {
    fn default() -> Self {
        Self::theorem_default()
    }
}

/// Named colours generated from one [`ThemeInput`].
///
/// The map uses Linear's camelCase role names because those names are the
/// stable bridge into the GPUI theme config and the DTCG alias layer.
#[derive(Clone, Debug)]
pub struct DerivedTheme {
    colors: BTreeMap<&'static str, Rgba>,
    lch: BTreeMap<&'static str, CieLch>,
    pub elevation: [String; 5],
    pub is_light: bool,
}

impl DerivedTheme {
    #[must_use]
    pub fn color(&self, name: &str) -> Option<Rgba> {
        self.colors.get(name).copied()
    }

    #[must_use]
    pub fn lch(&self, name: &str) -> Option<CieLch> {
        self.lch.get(name).copied()
    }

    pub fn colors(&self) -> impl Iterator<Item = (&'static str, Rgba)> + '_ {
        self.colors.iter().map(|(name, color)| (*name, *color))
    }
}

#[derive(Clone, Copy)]
struct Lcha {
    l: f64,
    c: f64,
    h: f64,
    a: f64,
}

impl From<CieLch> for Lcha {
    fn from(value: CieLch) -> Self {
        Self {
            l: value.l,
            c: value.c,
            h: value.h,
            a: 1.0,
        }
    }
}

impl From<Lcha> for CieLch {
    fn from(value: Lcha) -> Self {
        Self::new(value.l, value.c, value.h)
    }
}

#[derive(Clone, Copy, Default)]
struct Adjustment {
    l: Option<f64>,
    c: Option<f64>,
    h: Option<f64>,
    a: Option<f64>,
}

struct ThemeLaw {
    input: ThemeInput,
    base: Lcha,
    is_light: bool,
    surface_factor: f64,
    control_factor: f64,
    border_factor: f64,
    label_factor: f64,
    spread: f64,
}

impl ThemeLaw {
    fn new(input: ThemeInput) -> Self {
        let base = Lcha::from(input.base);
        let is_light = base.l > 50.0;
        let contrast = f64::from(input.contrast.clamp(0.0, 100.0));
        let ramp = contrast.min(30.0) + (contrast - 30.0).max(0.0) * 0.25;
        Self {
            input,
            base,
            is_light,
            surface_factor: if is_light { -ramp / 30.0 } else { ramp / 30.0 },
            control_factor: if is_light {
                -0.8 * ramp / 70.0
            } else {
                ramp / 70.0
            },
            border_factor: if is_light {
                -0.9 * (contrast + (contrast - 30.0).max(0.0) * 0.4) / 10.0
            } else {
                0.8 * (contrast + (contrast - 30.0).max(0.0) * 0.4) / 10.0
            },
            label_factor: (if is_light { -1.0 } else { 1.0 }) * (3.0 + (100.0 - contrast) / 70.0)
                / 4.0,
            spread: (1.0 + (base.l - 50.0).abs() / 50.0) / 2.0,
        }
    }

    fn derive(&self) -> DerivedTheme {
        let mut colors = BTreeMap::new();
        let mut lch = BTreeMap::new();
        let mut put = |name: &'static str, color: Lcha| {
            colors.insert(name, to_rgba(color));
            lch.insert(name, color.into());
        };

        let base = self.base;
        let bg_base_hover = self.surface(
            base,
            if self.is_light {
                adjustment(3.5, 0.0)
            } else {
                adjustment(4.25, 0.5)
            },
        );
        let bg_sub = self.surface(
            base,
            if self.is_light {
                adjustment(3.5, 0.0)
            } else {
                adjustment(-3.25, 0.0)
            },
        );
        let bg_sub_hover = self.surface(
            bg_sub,
            if self.is_light {
                adjustment(5.0, 0.0)
            } else {
                adjustment(2.5, 3.0)
            },
        );
        let bg_shade = self.surface(
            base,
            if self.is_light {
                adjustment(5.5, 0.0)
            } else {
                adjustment(2.0, 0.5)
            },
        );
        let bg_shade_hover = self.surface(
            bg_shade,
            if self.is_light {
                adjustment(1.5, 0.0)
            } else {
                adjustment(1.0, 0.5)
            },
        );
        let bg_selected = mix(
            base,
            self.input.accent.into(),
            (1.0 + base.c / 30.0) * if self.is_light { 0.05 } else { 0.18 },
        );
        let bg_selected_hover = self.surface(
            bg_selected,
            if self.is_light {
                adjustment(2.0, 0.0)
            } else {
                adjustment(2.5, 2.0)
            },
        );
        let bg_focus = self.surface(
            base,
            if self.is_light {
                adjustment(5.0, 0.0)
            } else {
                adjustment(9.0, 0.5)
            },
        );

        let border = |l, c| self.border(base, adjustment(l, c));
        let bg_border = border(
            if self.is_light { 3.5 } else { 4.0 },
            if self.is_light { 1.0 } else { 0.5 },
        );
        let bg_border_hover = border(
            if self.is_light { 4.5 } else { 5.0 },
            if self.is_light { 1.0 } else { 0.5 },
        );
        let bg_border_thin = if self.input.hairlines {
            border(
                if self.is_light { 3.0 } else { 6.0 },
                if self.is_light { 1.0 } else { 0.5 },
            )
        } else {
            bg_border
        };
        let bg_border_faint = border(
            if self.is_light { 1.0 } else { 2.0 },
            if self.is_light { 1.0 } else { 0.5 },
        );
        let bg_border_faint_hover = border(
            if self.is_light { 2.0 } else { 2.75 },
            if self.is_light { 1.0 } else { 0.5 },
        );
        let bg_border_faint_thin = if self.input.hairlines {
            border(
                if self.is_light { 3.0 } else { 3.5 },
                if self.is_light { 1.0 } else { 0.5 },
            )
        } else {
            bg_border_faint
        };
        let bg_border_solid = border(5.0, if self.is_light { 1.0 } else { 0.5 });
        let bg_border_solid_hover = border(
            if self.is_light { 9.0 } else { 7.0 },
            if self.is_light { 1.0 } else { 0.5 },
        );
        let bg_border_solid_thin = if self.input.hairlines {
            border(
                if self.is_light { 5.0 } else { 10.0 },
                if self.is_light { 1.0 } else { 0.5 },
            )
        } else {
            bg_border_solid
        };
        let bg_border_strong = border(
            if self.is_light { 17.0 } else { 20.0 },
            if self.is_light { 1.0 } else { 0.5 },
        );
        let bg_border_strong_hover = border(
            if self.is_light { 21.0 } else { 24.0 },
            if self.is_light { 1.0 } else { 0.5 },
        );
        let bg_border_strong_thin = border(
            if self.is_light { 17.0 } else { 26.0 },
            if self.is_light { 1.0 } else { 0.5 },
        );

        let label_title = self.label(
            base,
            Adjustment {
                l: Some(if self.is_light {
                    -10.0 * self.spread
                } else {
                    10.0
                }),
                ..Adjustment::default()
            },
            Some(Adjustment {
                c: Some(0.0),
                ..Adjustment::default()
            }),
        );
        let label_base = self.label(
            base,
            Adjustment {
                l: Some(if self.is_light {
                    -20.0 * self.spread
                } else {
                    -10.0 * self.spread
                }),
                c: Some(1.0),
                ..Adjustment::default()
            },
            None,
        );
        let label_muted = self.label(
            base,
            Adjustment {
                l: Some(-40.0 * self.spread),
                c: Some(1.0),
                ..Adjustment::default()
            },
            None,
        );
        let label_faint = self.label(
            base,
            Adjustment {
                l: Some(-66.0 * self.spread),
                c: Some(1.0),
                ..Adjustment::default()
            },
            None,
        );
        let label_link = self.label(
            base,
            Adjustment {
                l: Some(-45.0 * self.spread),
                ..Adjustment::default()
            },
            Some(Adjustment {
                h: Some(self.input.accent.h),
                c: Some(70.0),
                ..Adjustment::default()
            }),
        );
        let label_hover = |color: Lcha| {
            let distance = (color.l - base.l).abs() / 100.0;
            let range = if self.is_light { 30.0 } else { 15.0 }
                * (1.0 + (f64::from(self.input.contrast) - 30.0).max(0.0) / 70.0);
            let delta = distance * range;
            adjust(
                color,
                Adjustment {
                    l: Some(if color.l > 100.0 - delta {
                        -delta
                    } else {
                        delta
                    }),
                    ..Adjustment::default()
                },
            )
        };

        let control_primary = Lcha::from(self.input.accent);
        let control_primary_hover = self.surface(
            control_primary,
            adjustment(if self.is_light { 6.0 } else { 5.0 }, 2.0),
        );
        let control_primary_label = self.label(
            control_primary,
            Adjustment::default(),
            Some(Adjustment {
                c: Some(control_primary.c.min(5.0)),
                ..Adjustment::default()
            }),
        );
        let control_secondary = self.control(
            base,
            if self.is_light {
                adjustment(-6.0, 0.0)
            } else {
                adjustment(12.0, 0.75)
            },
        );
        let control_secondary_hover = self.control(control_secondary, adjustment(12.0, 1.0));
        let control_secondary_selected = self.control(
            control_secondary,
            if self.is_light {
                adjustment(15.0, 1.0)
            } else {
                adjustment(22.0, 1.0)
            },
        );
        let control_tertiary = self.control(
            base,
            if self.is_light {
                adjustment(-6.0, 0.0)
            } else {
                adjustment(12.0, 0.5)
            },
        );
        let control_tertiary_hover = self.control(
            base,
            if self.is_light {
                adjustment(9.0, 0.0)
            } else {
                adjustment(22.0, 0.5)
            },
        );
        let control_tertiary_selected = self.control(
            base,
            if self.is_light {
                adjustment(13.0, 0.0)
            } else {
                adjustment(29.0, 1.5)
            },
        );

        let focus_is_visible = if self.is_light {
            control_primary.l < 90.0
        } else {
            control_primary.l > 30.0
        };
        let focus = if control_primary.c > 50.0 && focus_is_visible {
            control_primary
        } else {
            let hue = if control_primary.c < 20.0 {
                288.43
            } else {
                control_primary.h
            };
            adjust_to(
                control_primary,
                if self.is_light {
                    Adjustment {
                        l: Some(70.0),
                        c: Some(90.0),
                        h: Some(hue),
                        ..Adjustment::default()
                    }
                } else {
                    Adjustment {
                        l: Some(50.0),
                        c: Some(120.0),
                        h: Some(hue),
                        ..Adjustment::default()
                    }
                },
            )
        };

        for (name, color) in [
            ("bgBase", base),
            ("bgBaseHover", bg_base_hover),
            ("bgSub", bg_sub),
            ("bgSubHover", bg_sub_hover),
            ("bgShade", bg_shade),
            ("bgShadeHover", bg_shade_hover),
            ("bgSelected", bg_selected),
            ("bgSelectedHover", bg_selected_hover),
            ("bgFocus", bg_focus),
            ("bgBorder", bg_border),
            ("bgBorderHover", bg_border_hover),
            ("bgBorderThin", bg_border_thin),
            ("bgBorderFaint", bg_border_faint),
            ("bgBorderFaintHover", bg_border_faint_hover),
            ("bgBorderFaintThin", bg_border_faint_thin),
            ("bgBorderSolid", bg_border_solid),
            ("bgBorderSolidHover", bg_border_solid_hover),
            ("bgBorderSolidThin", bg_border_solid_thin),
            ("bgBorderStrong", bg_border_strong),
            ("bgBorderStrongHover", bg_border_strong_hover),
            ("bgBorderStrongThin", bg_border_strong_thin),
            (
                "bgSelectedBorder",
                self.border(bg_selected, adjustment(3.5, 1.0)),
            ),
            (
                "bgSelectedBorderHover",
                self.border(bg_selected, adjustment(4.5, 1.0)),
            ),
            ("labelBase", label_base),
            ("labelBaseHover", label_hover(label_base)),
            ("labelFaint", label_faint),
            ("labelLink", label_link),
            ("labelMuted", label_muted),
            ("labelMutedHover", label_hover(label_muted)),
            ("labelTitle", label_title),
            ("labelTitleHover", label_hover(label_title)),
            ("controlPrimary", control_primary),
            ("controlPrimaryHover", control_primary_hover),
            ("controlPrimaryLabel", control_primary_label),
            ("controlSecondary", control_secondary),
            ("controlSecondaryHover", control_secondary_hover),
            ("controlSecondarySelected", control_secondary_selected),
            ("controlSecondaryLabel", label_base),
            ("controlTertiary", control_tertiary),
            ("controlTertiaryHover", control_tertiary_hover),
            ("controlTertiarySelected", control_tertiary_selected),
            ("controlTertiaryLabel", label_base),
            ("scrollbarBg", adjust(label_faint, adjustment(0.3, 0.0))),
            (
                "scrollbarBgHover",
                adjust(
                    label_faint,
                    adjustment(if self.is_light { 0.8 } else { 0.4 }, 0.0),
                ),
            ),
            ("scrollbarBgActive", label_faint),
            (
                "chromeTabBg",
                self.surface(
                    base,
                    if self.is_light {
                        adjustment(-2.5, 0.0)
                    } else {
                        adjustment(5.0, 2.0)
                    },
                ),
            ),
            (
                "chromeTabBgHover",
                self.surface(
                    base,
                    if self.is_light {
                        adjustment(-5.0, 0.0)
                    } else {
                        adjustment(7.0, 2.0)
                    },
                ),
            ),
            (
                "chromeTabBgActive",
                self.surface(
                    base,
                    if self.is_light {
                        adjustment(-8.0, 0.0)
                    } else {
                        adjustment(10.0, 2.0)
                    },
                ),
            ),
            ("focus", focus),
        ] {
            put(name, color);
        }

        for (names, source) in [
            (
                (
                    "successBase",
                    "successHover",
                    "successBg",
                    "successText",
                    "successForeground",
                    "successTint",
                ),
                CieLch::new(60.0, 64.37, 141.95),
            ),
            (
                (
                    "warningBase",
                    "warningHover",
                    "warningBg",
                    "warningText",
                    "warningForeground",
                    "warningTint",
                ),
                CieLch::new(66.0, 80.0, 48.0),
            ),
            (
                (
                    "errorBase",
                    "errorHover",
                    "errorBg",
                    "errorText",
                    "errorForeground",
                    "errorTint",
                ),
                CieLch::new(38.0, 70.0, 15.0),
            ),
            (
                (
                    "agentBase",
                    "agentHover",
                    "agentBg",
                    "agentText",
                    "agentForeground",
                    "agentTint",
                ),
                CieLch::new(50.2, 29.2, 181.9),
            ),
        ] {
            let source = Lcha::from(source);
            let background =
                readable_lightness(source.into(), label_title.into()).map_or(source, Lcha::from);
            let foreground = get_text_color(background);
            put(names.0, source);
            put(names.1, self.surface(source, adjustment(5.0, 0.0)));
            put(names.2, background);
            put(
                names.3,
                self.label(
                    source,
                    Adjustment::default(),
                    Some(Adjustment {
                        l: Some(if self.is_light { 80.0 } else { 50.0 }),
                        c: Some(80.0),
                        ..Adjustment::default()
                    }),
                ),
            );
            put(names.4, foreground);
            put(
                names.5,
                mix(base, source, if self.is_light { 0.03 } else { 0.2 }),
            );
        }

        let modal_alpha = ((if self.is_light { 0.4 } else { 0.25 })
            * (1.0
                + (f64::from(self.input.contrast) - 30.0).max(0.0)
                    / if self.is_light { 10.0 } else { 50.0 }))
        .clamp(0.0, 0.8);
        put(
            "bgModalOverlay",
            Lcha {
                l: 0.0,
                c: 0.0,
                h: 0.0,
                a: modal_alpha,
            },
        );

        let shadow = if self.is_light { "#000000" } else { "#FFFFFF" };
        let elevation = if self.is_light {
            [
                "none".to_owned(),
                format!("0 0.5px 1px 1px {shadow}4D"),
                format!("0 3px 8px {shadow}20, 0 1px 1px {shadow}20"),
                format!("0 4px 40px {shadow}1A, 0 2px 8px {shadow}20"),
                format!("0 1px 3px inset {shadow}12, 0 2px 5px inset {shadow}1A"),
            ]
        } else {
            [
                "none".to_owned(),
                format!("0 3px 6px -2px {shadow}05, 0 1px 1px {shadow}0A"),
                format!("0 6px 18px {shadow}05, 0 3px 9px {shadow}0A"),
                format!("0 9px 48px {shadow}14, 0 6px 24px {shadow}1A"),
                format!("0 1px 3px inset {shadow}12, 0 2px 5px inset {shadow}1A"),
            ]
        };

        DerivedTheme {
            colors,
            lch,
            elevation,
            is_light: self.is_light,
        }
    }

    fn surface(&self, color: Lcha, delta: Adjustment) -> Lcha {
        adjust(
            color,
            Adjustment {
                l: delta.l.map(|value| value * self.surface_factor),
                c: delta.c.map(|value| value * self.surface_factor),
                h: delta.h,
                a: delta.a,
            },
        )
    }

    fn control(&self, color: Lcha, delta: Adjustment) -> Lcha {
        adjust(
            color,
            Adjustment {
                l: delta.l.map(|value| value * self.control_factor),
                c: delta.c.map(|value| value * self.control_factor),
                h: delta.h,
                a: delta.a,
            },
        )
    }

    fn border(&self, color: Lcha, delta: Adjustment) -> Lcha {
        adjust(
            color,
            Adjustment {
                l: delta.l.map(|value| value * self.border_factor),
                c: delta.c.map(|value| value * self.border_factor),
                h: delta.h,
                a: delta.a,
            },
        )
    }

    fn label(&self, color: Lcha, delta: Adjustment, target: Option<Adjustment>) -> Lcha {
        let text = adjust(
            get_text_color(color),
            Adjustment {
                l: delta.l.map(|value| value * self.label_factor),
                ..delta
            },
        );
        target.map_or(text, |target| adjust_to(text, target))
    }
}

fn adjustment(l: f64, c: f64) -> Adjustment {
    Adjustment {
        l: Some(l),
        c: Some(c),
        ..Adjustment::default()
    }
}

fn adjust(color: Lcha, delta: Adjustment) -> Lcha {
    Lcha {
        l: (color.l + delta.l.unwrap_or(0.0)).clamp(0.0, 100.0),
        c: (color.c + delta.c.unwrap_or(0.0)).clamp(0.0, 132.0),
        h: normalize_hue(color.h + delta.h.unwrap_or(0.0)),
        a: (color.a + delta.a.unwrap_or(0.0)).clamp(0.0, 1.0),
    }
}

fn adjust_to(color: Lcha, target: Adjustment) -> Lcha {
    Lcha {
        l: target.l.unwrap_or(color.l).clamp(0.0, 100.0),
        c: target.c.unwrap_or(color.c).clamp(0.0, 132.0),
        h: normalize_hue(target.h.unwrap_or(color.h)),
        a: target.a.unwrap_or(color.a).clamp(0.0, 1.0),
    }
}

fn get_text_color(color: Lcha) -> Lcha {
    Lcha {
        l: if color.l - color.c * 0.075 > 65.0 {
            0.0
        } else {
            100.0
        },
        c: (color.c / 2.0).min(color.c),
        h: color.h,
        a: 1.0,
    }
}

/// Linear's polarity-aware APCA approximation.
#[must_use]
pub fn apca_contrast(text: CieLch, background: CieLch) -> f32 {
    let soft = |lightness: f64| {
        let y = lightness / 100.0;
        if y >= APCA_SOFT_CLAMP {
            y
        } else {
            y + (APCA_SOFT_CLAMP - y).powf(APCA_SOFT_EXPONENT)
        }
    };
    let text_y = soft(text.l);
    let background_y = soft(background.l);
    if (text_y - background_y).abs() < 0.000_5 {
        return 0.0;
    }
    let raw = if background_y > text_y {
        (background_y.powf(0.56) - text_y.powf(0.57)) * 1.14
    } else {
        (background_y.powf(0.65) - text_y.powf(0.62)) * 1.14
    };
    let output = if raw.abs() < 0.1 {
        0.0
    } else if raw > 0.0 {
        raw - 0.027
    } else {
        raw + 0.027
    };
    (output.abs() * 100.0) as f32
}

/// Find a same-hue chip background that clears APCA 38 against `against`.
#[must_use]
pub fn readable_lightness(color: CieLch, against: CieLch) -> Option<CieLch> {
    let mut low = 0.0;
    let mut high = 100.0;
    let mut selected = high;
    let mut found = false;
    while high - low > 1.0 {
        let middle = (low + high) / 2.0;
        let candidate = CieLch::new(middle, color.c, color.h);
        if apca_contrast(against, candidate) > MIN_TEXT_APCA {
            found = true;
            selected = middle;
            if against.l > middle {
                low = middle;
            } else {
                high = middle;
            }
        } else if against.l > middle {
            high = middle;
        } else {
            low = middle;
        }
    }
    if found {
        return Some(CieLch::new(selected, color.c, color.h));
    }
    for chroma in [color.c * 0.75, color.c * 0.5, color.c * 0.25, 0.0] {
        low = color.l;
        high = 100.0;
        selected = high;
        found = false;
        while high - low > 0.1 {
            let middle = (low + high) / 2.0;
            let candidate = CieLch::new(middle, chroma, color.h);
            if apca_contrast(candidate, against) > MIN_TEXT_APCA {
                found = true;
                selected = middle;
                high = middle;
            } else {
                low = middle;
            }
        }
        if found {
            return Some(CieLch::new(selected, chroma, color.h));
        }
    }
    None
}

fn mix(first: Lcha, second: Lcha, amount: f64) -> Lcha {
    let first_xyz = lab_to_xyz(lch_to_lab(first));
    let second_xyz = lab_to_xyz(lch_to_lab(second));
    let mixed = [
        first_xyz[0] * (1.0 - amount) + second_xyz[0] * amount,
        first_xyz[1] * (1.0 - amount) + second_xyz[1] * amount,
        first_xyz[2] * (1.0 - amount) + second_xyz[2] * amount,
    ];
    let [l, a, b] = xyz_to_lab(mixed);
    lab_to_lch([l, a, b], first.a * (1.0 - amount) + second.a * amount)
}

fn to_rgba(color: Lcha) -> Rgba {
    let lab = lch_to_lab(color);
    let xyz_d50 = lab_to_xyz(lab);
    let xyz_d65 = multiply_matrix(
        [
            [
                0.955_473_452_704_218_2,
                -0.023_098_536_874_261_423,
                0.063_259_308_661_021_7,
            ],
            [
                -0.028_369_706_963_208_136,
                1.009_995_458_005_822_6,
                0.021_041_398_966_943_008,
            ],
            [
                0.012_314_001_688_319_899,
                -0.020_507_696_433_477_912,
                1.330_365_936_608_075_3,
            ],
        ],
        xyz_d50,
    );
    let linear = multiply_matrix(
        [
            [
                3.240_969_941_904_522_6,
                -1.537_383_177_570_094,
                -0.498_610_760_293_003_4,
            ],
            [
                -0.969_243_636_280_879_6,
                1.875_967_501_507_720_2,
                0.041_555_057_407_175_59,
            ],
            [
                0.055_630_079_696_993_66,
                -0.203_976_958_888_976_52,
                1.056_971_514_242_878_6,
            ],
        ],
        xyz_d65,
    );
    let encode = |channel: f64| {
        let sign = channel.signum();
        let value = channel.abs();
        let encoded = if value > 0.003_130_8 {
            sign * (1.055 * value.powf(1.0 / 2.4) - 0.055)
        } else {
            12.92 * channel
        };
        (encoded.clamp(0.0, 1.0) * 255.0).round() as u8
    };
    Rgba {
        r: encode(linear[0]),
        g: encode(linear[1]),
        b: encode(linear[2]),
        a: color.a as f32,
    }
}

fn lch_to_lab(color: Lcha) -> [f64; 3] {
    let radians = color.h.to_radians();
    [color.l, color.c * radians.cos(), color.c * radians.sin()]
}

fn lab_to_lch(lab: [f64; 3], alpha: f64) -> Lcha {
    let mut hue = lab[2].atan2(lab[1]).to_degrees();
    if hue < 0.0 {
        hue += 360.0;
    }
    Lcha {
        l: lab[0],
        c: (lab[1] * lab[1] + lab[2] * lab[2]).sqrt(),
        h: hue,
        a: alpha,
    }
}

fn lab_to_xyz(lab: [f64; 3]) -> [f64; 3] {
    const EPSILON: f64 = 216.0 / 24_389.0;
    const KAPPA: f64 = 24_389.0 / 27.0;
    const WHITE: [f64; 3] = [0.3457 / 0.3585, 1.0, 0.2958 / 0.3585];
    let fy = (lab[0] + 16.0) / 116.0;
    let fx = lab[1] / 500.0 + fy;
    let fz = fy - lab[2] / 200.0;
    let xyz = [
        if fx.powi(3) > EPSILON {
            fx.powi(3)
        } else {
            (116.0 * fx - 16.0) / KAPPA
        },
        if lab[0] > KAPPA * EPSILON {
            ((lab[0] + 16.0) / 116.0).powi(3)
        } else {
            lab[0] / KAPPA
        },
        if fz.powi(3) > EPSILON {
            fz.powi(3)
        } else {
            (116.0 * fz - 16.0) / KAPPA
        },
    ];
    [xyz[0] * WHITE[0], xyz[1] * WHITE[1], xyz[2] * WHITE[2]]
}

fn xyz_to_lab(xyz: [f64; 3]) -> [f64; 3] {
    const WHITE: [f64; 3] = [0.3457 / 0.3585, 1.0, 0.2958 / 0.3585];
    let f = |value: f64| {
        if value > 0.008_856_451_679_035_631 {
            value.cbrt()
        } else {
            (903.296_296_296_296_3 * value + 16.0) / 116.0
        }
    };
    let x = f(xyz[0] / WHITE[0]);
    let y = f(xyz[1] / WHITE[1]);
    let z = f(xyz[2] / WHITE[2]);
    [116.0 * y - 16.0, 500.0 * (x - y), 200.0 * (y - z)]
}

fn multiply_matrix(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2],
        matrix[2][0] * vector[0] + matrix[2][1] * vector[1] + matrix[2][2] * vector[2],
    ]
}

fn normalize_hue(value: f64) -> f64 {
    value.rem_euclid(360.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_reference_matches_live_painted_values() {
        let theme = ThemeInput::linear_reference().derive();
        assert_eq!(theme.color("bgBase").unwrap().hex(), "#F9F9FA");
        assert_eq!(theme.color("bgSub").unwrap().hex(), "#EFEFF0");
        assert_eq!(theme.color("bgBorder").unwrap().hex(), "#DEDEDE");
        assert_eq!(theme.color("focus").unwrap().hex(), "#6D78D5");
    }

    #[test]
    fn dark_theme_uses_the_same_derivation_path() {
        let dark = ThemeInput::theorem_dark().derive();
        assert!(!dark.is_light);
        assert_ne!(
            dark.color("bgBase"),
            ThemeInput::theorem_default().derive().color("bgBase")
        );
        assert!(dark.color("labelTitle").is_some());
    }

    #[test]
    fn theme_file_round_trips_with_only_three_authored_fields() {
        let input = ThemeInput::theorem_default();
        let json = input.to_json().unwrap();
        assert_eq!(json.matches(':').count(), 3);
        assert_eq!(ThemeInput::from_json_str(&json).unwrap(), input);
        let toml = input.to_toml().unwrap();
        assert_eq!(ThemeInput::from_toml_str(&toml).unwrap(), input);
    }

    #[test]
    fn contrast_moves_the_reference_contrast_registry() {
        let low = ThemeInput::theorem_default().derive();
        let high = ThemeInput {
            contrast: 70.0,
            ..ThemeInput::theorem_default()
        }
        .derive();
        for name in [
            "bgBaseHover",
            "bgSub",
            "bgShade",
            "bgBorder",
            "labelTitle",
            "labelBase",
            "labelMuted",
            "controlSecondaryHover",
            "controlTertiaryHover",
        ] {
            assert_ne!(low.color(name), high.color(name), "derived colour {name}");
        }
    }

    #[test]
    fn light_highlights_carry_no_beige_chroma() {
        let theme = ThemeInput::theorem_default().derive();
        for (name, _) in theme.colors() {
            let sample = theme.lch(name).unwrap();
            if sample.l > 80.0 && name.starts_with("bg") {
                assert!(sample.c <= 0.006 * 132.0, "{name} chroma was {}", sample.c);
            }
        }
    }

    #[test]
    fn apca_is_polarity_aware_and_status_search_clears_the_floor() {
        let dark = CieLch::new(20.0, 0.0, 0.0);
        let light = CieLch::new(95.0, 0.0, 0.0);
        assert_ne!(apca_contrast(dark, light), apca_contrast(light, dark));
        let chip = readable_lightness(CieLch::new(38.0, 70.0, 15.0), dark).unwrap();
        assert!(apca_contrast(chip, dark) > MIN_TEXT_APCA);
    }
}
