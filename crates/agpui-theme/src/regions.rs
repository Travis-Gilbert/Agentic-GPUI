//! The shell region map, computed once per frame from the shared metric table.
//!
//! Every rectangle the environment draws comes from here, so the geometry
//! check and the render path read the same numbers. Nothing in this module
//! touches GPUI: it is arithmetic over `theorem_design_core::METRICS`, which
//! is what makes the check a unit test rather than a screenshot.

use theorem_design_core::METRICS;

/// One region, in content-area coordinates with the origin top-left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[must_use]
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    #[must_use]
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
}

/// Every region of the shell for one content size.
///
/// The right dock is an overlay over the canvas, so it is not subtracted from
/// the canvas rectangle: the canvas keeps its full width and the dock is drawn
/// on top of its trailing edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellRegions {
    pub tab_strip: Rect,
    pub sidebar: Rect,
    pub card: Rect,
    pub header: Rect,
    pub filter_band: Rect,
    pub canvas: Rect,
    pub status_band: Rect,
    pub right_dock: Option<Rect>,
}

impl ShellRegions {
    /// Lay the shell out inside a window content area.
    ///
    /// `sidebar_width` is zero when the sidebar is collapsed away entirely;
    /// `right_dock` is `None` when the domain pins no instance to it.
    #[must_use]
    pub fn for_content(
        width: f32,
        height: f32,
        sidebar_width: f32,
        right_dock_width: Option<f32>,
    ) -> Self {
        let tab_height = METRICS.desktop_tabs_height;
        let status_height = METRICS.status_bar_height;
        let margin = METRICS.main_page_margin;

        let band_top = tab_height;
        let band_height = (height - tab_height - status_height).max(0.0);

        let sidebar = Rect::new(0.0, band_top, sidebar_width, band_height);
        let card = Rect::new(
            sidebar_width + margin,
            band_top + margin,
            (width - sidebar_width - margin * 2.0).max(0.0),
            (band_height - margin * 2.0).max(0.0),
        );
        let header = Rect::new(card.x, card.y, card.width, METRICS.main_header_height);
        let filter_band = Rect::new(
            card.x,
            header.bottom(),
            card.width,
            METRICS.subheader_height,
        );
        let canvas = Rect::new(
            card.x,
            filter_band.bottom(),
            card.width,
            (card.bottom() - filter_band.bottom()).max(0.0),
        );
        let right_dock = right_dock_width.map(|dock_width| {
            let dock_width = dock_width.min(canvas.width);
            Rect::new(
                canvas.right() - dock_width,
                canvas.y,
                dock_width,
                canvas.height,
            )
        });

        Self {
            tab_strip: Rect::new(0.0, 0.0, width, tab_height),
            sidebar,
            card,
            header,
            filter_band,
            canvas,
            status_band: Rect::new(0.0, height - status_height, width, status_height),
            right_dock,
        }
    }

    /// The width a module's measure may use inside the canvas.
    ///
    /// Linear caps its reading measure well below the canvas width; the cap is
    /// the same number whether or not the right dock is open, because the dock
    /// overlays rather than reflows.
    #[must_use]
    pub fn measure(&self) -> f32 {
        self.canvas.width.min(METRICS.agent_measure_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default window content area named by the 1.0 region map.
    const CONTENT_WIDTH: f32 = 1274.0;
    const CONTENT_HEIGHT: f32 = 796.0;

    fn default_regions() -> ShellRegions {
        ShellRegions::for_content(
            CONTENT_WIDTH,
            CONTENT_HEIGHT,
            METRICS.sidebar_width_default,
            Some(METRICS.agent_panel_width),
        )
    }

    #[test]
    fn the_region_map_lays_out_at_the_measured_window_size() {
        let regions = default_regions();

        assert_eq!(regions.tab_strip, Rect::new(0.0, 0.0, 1274.0, 40.0));
        assert_eq!(regions.sidebar, Rect::new(0.0, 40.0, 240.0, 728.0));
        assert_eq!(regions.card, Rect::new(248.0, 48.0, 1018.0, 712.0));
        assert_eq!(regions.header, Rect::new(248.0, 48.0, 1018.0, 44.0));
        assert_eq!(regions.filter_band, Rect::new(248.0, 92.0, 1018.0, 44.0));
        assert_eq!(regions.canvas, Rect::new(248.0, 136.0, 1018.0, 624.0));
        assert_eq!(regions.status_band, Rect::new(0.0, 768.0, 1274.0, 28.0));
        assert_eq!(
            regions.right_dock,
            Some(Rect::new(866.0, 136.0, 400.0, 624.0))
        );
    }

    #[test]
    fn the_bands_tile_the_window_height_exactly() {
        let regions = default_regions();

        assert_eq!(regions.tab_strip.bottom(), regions.sidebar.y);
        assert_eq!(regions.sidebar.bottom(), regions.status_band.y);
        assert_eq!(regions.status_band.bottom(), CONTENT_HEIGHT);
        assert_eq!(regions.tab_strip.width, CONTENT_WIDTH);
        assert_eq!(regions.status_band.width, CONTENT_WIDTH);
    }

    #[test]
    fn the_card_is_inset_by_the_page_margin_on_every_side() {
        let regions = default_regions();

        assert_eq!(
            regions.card.x - regions.sidebar.right(),
            METRICS.main_page_margin
        );
        assert_eq!(
            regions.card.y - regions.tab_strip.bottom(),
            METRICS.main_page_margin
        );
        assert_eq!(
            CONTENT_WIDTH - regions.card.right(),
            METRICS.main_page_margin
        );
        assert_eq!(
            regions.status_band.y - regions.card.bottom(),
            METRICS.main_page_margin
        );
    }

    #[test]
    fn the_right_dock_overlays_the_canvas_rather_than_narrowing_it() {
        let regions = default_regions();
        let dock = regions.right_dock.expect("the fixture pins a right dock");

        assert_eq!(dock.right(), regions.canvas.right());
        assert_eq!(dock.y, regions.canvas.y);
        assert_eq!(dock.height, regions.canvas.height);
        assert_eq!(
            regions.canvas.width,
            ShellRegions::for_content(
                CONTENT_WIDTH,
                CONTENT_HEIGHT,
                METRICS.sidebar_width_default,
                None,
            )
            .canvas
            .width
        );
    }

    #[test]
    fn a_collapsed_sidebar_gives_its_width_to_the_card() {
        let collapsed = ShellRegions::for_content(CONTENT_WIDTH, CONTENT_HEIGHT, 0.0, None);

        assert_eq!(collapsed.sidebar.width, 0.0);
        assert_eq!(
            collapsed.card.width,
            default_regions().card.width + METRICS.sidebar_width_default
        );
    }

    #[test]
    fn a_window_narrower_than_its_chrome_never_produces_a_negative_region() {
        let tiny =
            ShellRegions::for_content(120.0, 40.0, METRICS.sidebar_width_default, Some(400.0));

        assert!(tiny.card.width >= 0.0);
        assert!(tiny.card.height >= 0.0);
        assert!(tiny.canvas.height >= 0.0);
        assert!(tiny.right_dock.expect("a pinned dock still resolves").width >= 0.0);
    }
}
