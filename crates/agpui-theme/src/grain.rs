use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{GrainParams, TokenError};

const TILE_SIZE: u32 = 256;
const PAPER_SHADERS_PACKAGE: &str = "@paper-design/shaders";
const PAPER_SHADERS_VERSION: &str = "0.0.77";
const PAPER_SHADERS_GIT_HEAD: &str = "f9f2a8b2edeb78ec59256c4dc571f5eaf943d798";
const PAPER_SHADERS_TARBALL_SHA256: &str =
    "6b77c990dc98d794011b1374bd183ef94464f280ee289e63554a2cc373dec481";
const PAPER_SHADER_SHA256: &str =
    "b2fa3e8281bf85f9505880056d0cec947454604f4c780e11257ffec416d7e8ef";
const PAPER_NOISE_SHA256: &str = "5116a06c428a75e2db9bd55062c560bb02600383ee54da007f1628e845b2b73a";
const BAKE_CONTRACT: &str = "paperTexture-static-opaque-page-v1";
const PAPER_SHADER_SOURCE: &str = include_str!("../vendor/paper-shaders-0.0.77/paper-texture.js");
const PAPER_NOISE_PNG: &[u8] = include_bytes!("../vendor/paper-shaders-0.0.77/noise.png");

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GrainBakeReceipt {
    pub width: u32,
    pub height: u32,
    pub parameter_hash: String,
    pub png_sha256: String,
    pub oracle: GrainOracleReceipt,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GrainOracleReceipt {
    pub package: &'static str,
    pub version: &'static str,
    pub git_head: &'static str,
    pub tarball_sha256: &'static str,
    pub shader_sha256: &'static str,
    pub noise_sha256: &'static str,
    pub bake_contract: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GrainHashInput {
    color_back: String,
    color_front: String,
    opacity_page: f32,
    opacity_sidebar: f32,
    scale: f32,
    speed: f32,
    contrast: f32,
    roughness: f32,
    fiber: f32,
    fiber_size: f32,
    crumples: f32,
    crumple_size: f32,
    folds: f32,
    fold_count: f32,
    fade: f32,
    drops: f32,
    seed: f32,
    oracle: GrainOracleReceipt,
}

struct NoiseTexture {
    width: usize,
    height: usize,
    green: Vec<f32>,
}

/// Bake the static product specialization of Paper Shaders' `paperTexture`.
///
/// This is a CPU translation of the roughness and lighting lane in the pinned
/// 0.0.77 fragment shader. It samples the package's exact noise PNG with the
/// same LINEAR + CLAMP_TO_EDGE rules as `ShaderMount`. The PNG stores the
/// shader's opaque `colorFront * res + colorBack * (1 - res)` output. `res`
/// has negative highlight lobes, so an alpha-only encoding would silently
/// clamp away real shader output. Per-surface page/sidebar strength remains a
/// renderer concern.
///
/// # Errors
///
/// Returns an error when parameters no longer select the supported static
/// specialization, or if either artifact cannot be written.
pub fn bake_grain_png(
    path: impl AsRef<Path>,
    params: GrainParams,
) -> Result<GrainBakeReceipt, TokenError> {
    validate_static_specialization(params)?;
    validate_vendored_oracle()?;

    let path = path.as_ref();
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, TILE_SIZE, TILE_SIZE);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        // Polars enables `flate2/zlib-rs` in the connected product graph.
        // Pin png's backend-independent encoder so Cargo feature unification
        // cannot change the byte identity of this generated artifact.
        encoder.set_deflate_compression(png::DeflateCompression::FdeflateUltraFast);
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
        oracle: oracle_receipt(),
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
        contrast: params.contrast,
        roughness: params.roughness,
        fiber: params.fiber,
        fiber_size: params.fiber_size,
        crumples: params.crumples,
        crumple_size: params.crumple_size,
        folds: params.folds,
        fold_count: params.fold_count,
        fade: params.fade,
        drops: params.drops,
        seed: params.seed,
        oracle: oracle_receipt(),
    };
    hex_digest(
        &serde_json::to_vec(&input).expect("grain parameters contain only serializable values"),
    )
}

fn oracle_receipt() -> GrainOracleReceipt {
    GrainOracleReceipt {
        package: PAPER_SHADERS_PACKAGE,
        version: PAPER_SHADERS_VERSION,
        git_head: PAPER_SHADERS_GIT_HEAD,
        tarball_sha256: PAPER_SHADERS_TARBALL_SHA256,
        shader_sha256: PAPER_SHADER_SHA256,
        noise_sha256: PAPER_NOISE_SHA256,
        bake_contract: BAKE_CONTRACT,
    }
}

fn validate_vendored_oracle() -> Result<(), TokenError> {
    for (name, actual, expected) in [
        (
            "paper-texture.js",
            hex_digest(PAPER_SHADER_SOURCE.as_bytes()),
            PAPER_SHADER_SHA256,
        ),
        ("noise.png", hex_digest(PAPER_NOISE_PNG), PAPER_NOISE_SHA256),
    ] {
        if actual != expected {
            return Err(TokenError::InvalidToken(format!(
                "vendored Paper Shaders oracle {name} has sha256 {actual}, expected {expected}"
            )));
        }
    }
    Ok(())
}

fn validate_static_specialization(params: GrainParams) -> Result<(), TokenError> {
    let inactive = [
        ("speed", params.speed),
        ("fiber", params.fiber),
        ("crumples", params.crumples),
        ("folds", params.folds),
        ("fade", params.fade),
        ("drops", params.drops),
    ];
    if let Some((name, value)) = inactive.into_iter().find(|(_, value)| *value != 0.0) {
        return Err(TokenError::InvalidToken(format!(
            "CPU paperTexture bake supports only the static roughness lane; {name} was {value}"
        )));
    }
    if params.color_back.a != 1.0 || params.color_front.a != 1.0 {
        return Err(TokenError::InvalidToken(
            "CPU paperTexture bake requires opaque colorBack and colorFront".into(),
        ));
    }
    Ok(())
}

fn tile_rgba(params: GrainParams) -> Vec<u8> {
    let noise = paper_noise();
    let mut pixels = Vec::with_capacity((TILE_SIZE * TILE_SIZE * 4) as usize);
    for output_y in 0..TILE_SIZE {
        for output_x in 0..TILE_SIZE {
            // PNG rows run top-to-bottom; WebGL gl_FragCoord.y runs
            // bottom-to-top. Pixel centers are at n + 0.5 in both systems.
            let fragment = [
                output_x as f32 + 0.5,
                TILE_SIZE as f32 - output_y as f32 - 0.5,
            ];
            let center = TILE_SIZE as f32 * 0.5;
            let roughness_uv = [1.5 * (fragment[0] - center), 1.5 * (fragment[1] - center)];
            let right = paper_roughness([roughness_uv[0] + 1.0, roughness_uv[1]], noise);
            let left = paper_roughness([roughness_uv[0] - 1.0, roughness_uv[1]], noise);
            let normal = params.roughness * 1.5 * (right - left);
            let normal_z = 9.5 - 9.0 * params.contrast.powf(0.1);
            let normal_length = (2.0 * normal * normal + normal_z * normal_z).sqrt();
            let res = (3.0 * normal + normal_z) / (normal_length * 6.0_f32.sqrt());
            let mix_channel = |front: u8, back: u8| {
                (f32::from(front) * res + f32::from(back) * (1.0 - res))
                    .clamp(0.0, 255.0)
                    .round() as u8
            };
            pixels.extend_from_slice(&[
                mix_channel(params.color_front.r, params.color_back.r),
                mix_channel(params.color_front.g, params.color_back.g),
                mix_channel(params.color_front.b, params.color_back.b),
                255,
            ]);
        }
    }
    pixels
}

fn paper_roughness(mut point: [f32; 2], noise: &NoiseTexture) -> f32 {
    point[0] *= 0.1;
    point[1] *= 0.1;
    let mut output = 0.0;

    // The GLSL loop is `for (float i = 0.; ++i < 4.; ...)`, so the body
    // executes three times (i = 1, 2, 3), not four.
    for _ in 0..3 {
        let floor = [point[0].floor(), point[1].floor()];
        let ceil = [point[0].ceil(), point[1].ceil()];
        let fraction = [glsl_fract(point[0]), glsl_fract(point[1])];
        let floor_x = mix(
            random_green([floor[0], floor[1]], noise),
            random_green([floor[0], ceil[1]], noise),
            fraction[1],
        );
        let ceil_x = mix(
            random_green([ceil[0], floor[1]], noise),
            random_green([ceil[0], ceil[1]], noise),
            fraction[1],
        );
        output += mix(floor_x, ceil_x, fraction[0]);
        output += 0.2 / (2.0 * (0.2 * point[0] + 0.5 * point[1]).sin().abs()).exp();
        point[0] *= 2.1;
        point[1] *= 2.1;
    }
    output / 3.0
}

fn random_green(point: [f32; 2], noise: &NoiseTexture) -> f32 {
    let uv = [
        glsl_fract(point[0].floor() / 50.0 + 0.5),
        glsl_fract(point[1].floor() / 50.0 + 0.5),
    ];
    noise.sample_linear_clamp(uv)
}

impl NoiseTexture {
    fn sample_linear_clamp(&self, uv: [f32; 2]) -> f32 {
        let (x0, x1, x_mix) = linear_axis(uv[0], self.width);
        let (y0, y1, y_mix) = linear_axis(uv[1], self.height);
        let top = mix(
            self.green[y0 * self.width + x0],
            self.green[y0 * self.width + x1],
            x_mix,
        );
        let bottom = mix(
            self.green[y1 * self.width + x0],
            self.green[y1 * self.width + x1],
            x_mix,
        );
        mix(top, bottom, y_mix)
    }
}

fn linear_axis(coordinate: f32, size: usize) -> (usize, usize, f32) {
    let position = coordinate * size as f32 - 0.5;
    let base = position.floor();
    let fraction = position - base;
    let maximum = size as isize - 1;
    let first = (base as isize).clamp(0, maximum) as usize;
    let second = (base as isize + 1).clamp(0, maximum) as usize;
    (first, second, fraction)
}

fn paper_noise() -> &'static NoiseTexture {
    static NOISE: OnceLock<NoiseTexture> = OnceLock::new();
    NOISE.get_or_init(|| {
        let mut decoder = png::Decoder::new(Cursor::new(PAPER_NOISE_PNG));
        decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut reader = decoder
            .read_info()
            .expect("vendored Paper Shaders noise PNG must decode");
        let mut pixels = vec![0; reader.output_buffer_size().expect("noise buffer must fit")];
        let info = reader
            .next_frame(&mut pixels)
            .expect("vendored Paper Shaders noise PNG must contain one frame");
        pixels.truncate(info.buffer_size());

        let green = match info.color_type {
            png::ColorType::Rgb => pixels
                .chunks_exact(3)
                .map(|pixel| f32::from(pixel[1]) / 255.0)
                .collect(),
            png::ColorType::Rgba => pixels
                .chunks_exact(4)
                .map(|pixel| f32::from(pixel[1]) / 255.0)
                .collect(),
            color_type => panic!("expanded Paper Shaders noise PNG was {color_type:?}, not RGB(A)"),
        };

        NoiseTexture {
            width: info.width as usize,
            height: info.height as usize,
            green,
        }
    })
}

fn mix(first: f32, second: f32, amount: f32) -> f32 {
    first + (second - first) * amount
}

fn glsl_fract(value: f32) -> f32 {
    value - value.floor()
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
