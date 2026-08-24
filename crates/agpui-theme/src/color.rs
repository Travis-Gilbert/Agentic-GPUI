use serde::{Deserialize, Serialize};

use crate::TokenError;

/// Renderer-neutral sRGB color.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

impl Rgba {
    /// Render as `#RRGGBB`, or `#RRGGBBAA` when alpha is not opaque.
    pub fn hex(&self) -> String {
        if (self.a - 1.0).abs() < f32::EPSILON {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            let alpha = (self.a.clamp(0.0, 1.0) * 255.0).round() as u8;
            format!("#{:02X}{:02X}{:02X}{alpha:02X}", self.r, self.g, self.b)
        }
    }

    /// Parse six- or eight-digit sRGB hex.
    ///
    /// # Errors
    ///
    /// Returns [`TokenError::InvalidColor`] for any other representation.
    pub fn parse(value: &str) -> Result<Self, TokenError> {
        let hex = value
            .strip_prefix('#')
            .ok_or_else(|| TokenError::InvalidColor(value.to_owned()))?;
        if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(TokenError::InvalidColor(value.to_owned()));
        }
        let channel = |offset| {
            u8::from_str_radix(&hex[offset..offset + 2], 16)
                .map_err(|_| TokenError::InvalidColor(value.to_owned()))
        };
        Ok(Self {
            r: channel(0)?,
            g: channel(2)?,
            b: channel(4)?,
            a: if hex.len() == 8 {
                f32::from(channel(6)?) / 255.0
            } else {
                1.0
            },
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Oklch {
    pub l: f64,
    pub c: f64,
    pub h: f64,
}

fn linear_srgb(color: Oklch) -> [f64; 3] {
    // Björn Ottosson's OKLab conversion. These are the same coefficients used
    // by the already-verified theorem-ui-core donor.
    let hue = color.h.to_radians();
    let a = color.c * hue.cos();
    let b = color.c * hue.sin();
    let l = (color.l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m = (color.l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s = (color.l - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    [
        4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
        -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
        -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701 * s,
    ]
}

fn in_gamut(color: Oklch) -> bool {
    linear_srgb(color)
        .into_iter()
        .all(|channel| (-0.000_1..=1.000_1).contains(&channel))
}

pub(crate) fn fit_chroma(color: Oklch) -> Oklch {
    if in_gamut(color) {
        return color;
    }
    let mut low = 0.0;
    let mut high = color.c;
    for _ in 0..24 {
        let middle = (low + high) / 2.0;
        if in_gamut(Oklch { c: middle, ..color }) {
            low = middle;
        } else {
            high = middle;
        }
    }
    Oklch { c: low, ..color }
}

pub(crate) fn oklch_to_rgba(color: Oklch) -> Rgba {
    let [r, g, b] = linear_srgb(fit_chroma(color));
    let encode = |channel: f64| {
        let channel = channel.clamp(0.0, 1.0);
        let encoded = if channel <= 0.003_130_8 {
            12.92 * channel
        } else {
            1.055 * channel.powf(1.0 / 2.4) - 0.055
        };
        (encoded * 255.0).round() as u8
    };
    Rgba {
        r: encode(r),
        g: encode(g),
        b: encode(b),
        a: 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_hex_round_trip() {
        for value in ["#FBFAF7", "#C0603F", "#25242180"] {
            assert_eq!(Rgba::parse(value).unwrap().hex(), value);
        }
    }
}
