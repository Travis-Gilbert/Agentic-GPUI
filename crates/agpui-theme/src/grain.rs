use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{GrainParams, TokenError};

const TILE_SIZE: u32 = 256;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GrainBakeReceipt {
    pub width: u32,
    pub height: u32,
    pub parameter_hash: String,
    pub png_sha256: String,
}

#[derive(Serialize)]
struct GrainHashInput {
    color_back: String,
    color_front: String,
    opacity_page: f32,
    opacity_sidebar: f32,
    scale: f32,
    speed: f32,
    fixed_seed: u32,
}

/// Bake the static specialization of the Paper Shaders roughness lane.
///
/// Product parameters disable fibers, crumples, folds, drops, fade, and
/// animation. The remaining fragment lane is a central difference over a
/// deterministic, tileable noise field. The PNG alpha carries that field;
/// page/sidebar opacity remains a renderer concern.
///
/// # Errors
///
/// Returns an I/O or PNG encoding error if either artifact cannot be written.
pub fn bake_grain_png(
    path: impl AsRef<Path>,
    params: GrainParams,
) -> Result<GrainBakeReceipt, TokenError> {
    let path = path.as_ref();
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, TILE_SIZE, TILE_SIZE);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&tile_rgba(params))?;
    }
    let parameter_hash = grain_parameter_hash(params);
    let png_sha256 = hex_digest(&png_bytes);
    fs::write(path, &png_bytes)?;

    let receipt = GrainBakeReceipt {
        width: TILE_SIZE,
        height: TILE_SIZE,
        parameter_hash,
        png_sha256,
    };
    let mut sidecar = serde_json::to_vec_pretty(&receipt)
        .expect("grain receipt contains only serializable values");
    sidecar.push(b'\n');
    fs::write(sidecar_path(path), sidecar)?;
    Ok(receipt)
}

#[must_use]
pub fn grain_parameter_hash(params: GrainParams) -> String {
    let input = GrainHashInput {
        color_back: params.color_back.hex(),
        color_front: params.color_front.hex(),
        opacity_page: params.opacity_page,
        opacity_sidebar: params.opacity_sidebar,
        scale: params.scale,
        speed: params.speed,
        fixed_seed: 0,
    };
    hex_digest(
        &serde_json::to_vec(&input).expect("grain parameters contain only serializable values"),
    )
}

fn tile_rgba(params: GrainParams) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((TILE_SIZE * TILE_SIZE * 4) as usize);
    for y in 0..TILE_SIZE {
        for x in 0..TILE_SIZE {
            let left = roughness((x + TILE_SIZE - 1) % TILE_SIZE, y);
            let right = roughness((x + 1) % TILE_SIZE, y);
            let normal = 1.5 * (right - left);
            let light = normal / (normal.mul_add(normal, 0.25).sqrt()) * 0.105_409_26 + 0.948_683_3;
            let alpha = ((light - 0.84) * 255.0).clamp(0.0, 255.0).round() as u8;
            pixels.extend_from_slice(&[
                params.color_front.r,
                params.color_front.g,
                params.color_front.b,
                alpha,
            ]);
        }
    }
    pixels
}

fn roughness(x: u32, y: u32) -> f32 {
    let mut point_x = x as f32 * 0.15;
    let mut point_y = y as f32 * 0.15;
    let mut output = 0.0;
    for octave in 0..4 {
        output += bilinear_noise(point_x, point_y, octave);
        output += 0.2 / (2.0 * (0.2 * point_x + 0.5 * point_y).sin().abs()).exp();
        point_x *= 2.1;
        point_y *= 2.1;
    }
    output / 3.0
}

fn bilinear_noise(x: f32, y: f32, octave: u32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let tx = x.fract();
    let ty = y.fract();
    let sample = |dx, dy| hash_noise(x0 + dx, y0 + dy, octave);
    let top = sample(0, 0) + (sample(1, 0) - sample(0, 0)) * tx;
    let bottom = sample(0, 1) + (sample(1, 1) - sample(0, 1)) * tx;
    top + (bottom - top) * ty
}

fn hash_noise(x: i32, y: i32, octave: u32) -> f32 {
    let mut value = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ octave.wrapping_mul(0xC2B2_AE3D);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846C_A68B);
    value ^= value >> 16;
    value as f32 / u32::MAX as f32
}

fn sidecar_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_stem()
        .map(|stem| stem.to_os_string())
        .unwrap_or_default();
    name.push(".params.json");
    path.with_file_name(name)
}

fn hex_digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
