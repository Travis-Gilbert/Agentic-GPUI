use crate::{Rgba, TokenSet};

/// Renderer-neutral Paper Shaders parameters shared by DOM and GPUI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrainParams {
    pub color_back: Rgba,
    pub color_front: Rgba,
    pub opacity_page: f32,
    pub opacity_sidebar: f32,
    pub scale: f32,
    pub speed: f32,
    pub contrast: f32,
    pub roughness: f32,
    pub fiber: f32,
    pub fiber_size: f32,
    pub crumples: f32,
    pub crumple_size: f32,
    pub folds: f32,
    pub fold_count: f32,
    pub fade: f32,
    pub drops: f32,
    pub seed: f32,
}

/// Renderer-neutral canvas dot-grid parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DotGridParams {
    pub color: Rgba,
    pub spacing_px: f32,
    pub size_px: f32,
}

impl TokenSet {
    #[must_use]
    pub fn grain(&self) -> GrainParams {
        GrainParams {
            color_back: self.required_color("texture.grain.colorBack"),
            color_front: self.required_color("texture.grain.colorFront"),
            opacity_page: self.required_number("texture.grain.opacity"),
            opacity_sidebar: self.required_number("texture.grain.opacity.sidebar"),
            scale: self.required_number("texture.grain.scale"),
            speed: self.required_number("texture.grain.speed"),
            contrast: self.required_number("texture.grain.contrast"),
            roughness: self.required_number("texture.grain.roughness"),
            fiber: self.required_number("texture.grain.fiber"),
            fiber_size: self.required_number("texture.grain.fiberSize"),
            crumples: self.required_number("texture.grain.crumples"),
            crumple_size: self.required_number("texture.grain.crumpleSize"),
            folds: self.required_number("texture.grain.folds"),
            fold_count: self.required_number("texture.grain.foldCount"),
            fade: self.required_number("texture.grain.fade"),
            drops: self.required_number("texture.grain.drops"),
            seed: self.required_number("texture.grain.seed"),
        }
    }

    #[must_use]
    pub fn dot_grid(&self) -> DotGridParams {
        DotGridParams {
            color: self.required_color("texture.canvas.dotGrid.color"),
            spacing_px: self.required_number("texture.canvas.dotGrid.spacing"),
            size_px: self.required_number("texture.canvas.dotGrid.size"),
        }
    }

    fn required_color(&self, path: &str) -> Rgba {
        self.color(path)
            .unwrap_or_else(|error| panic!("embedded token {path} must be a color: {error}"))
    }

    fn required_number(&self, path: &str) -> f32 {
        self.number(path)
            .unwrap_or_else(|error| panic!("embedded token {path} must be numeric: {error}"))
    }
}
