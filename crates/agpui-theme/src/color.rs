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
