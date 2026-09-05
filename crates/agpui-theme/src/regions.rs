//! The shell region map, computed once per frame from a shell's metric table.
//!
//! Every rectangle a shell draws comes from here, so the geometry check and
//! the render path read the same numbers. Nothing in this module touches GPUI:
//! it is arithmetic over a [`ShellMetrics`], which is what makes the check a
//! unit test rather than a screenshot.
//!
//! The table arrives as an argument rather than a constant, because the shape
//! of a shell is AGPUI's and its numbers are the product's. The laws proven
//! below - that the bands tile the height, that the card is inset on every
//! side, that the dock overlays rather than reflows - hold for any table.

use crate::metrics::ShellMetrics;

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
        metrics: &ShellMetrics,
        width: f32,
        height: f32,
        sidebar_width: f32,
        right_dock_width: Option<f32>,
    ) -> Self {
        let tab_height = metrics.desktop_tabs_height;
        let status_height = metrics.status_bar_height;
        let margin = metrics.main_page_margin;

        let band_top = tab_height;
        let band_height = (height - tab_height - status_height).max(0.0);

        let sidebar = Rect::new(0.0, band_top, sidebar_width, band_height);
        let card = Rect::new(
            sidebar_width + margin,
            band_top + margin,
            (width - sidebar_width - margin * 2.0).max(0.0),
            (band_height - margin * 2.0).max(0.0),
        );
        let header = Rect::new(card.x, card.y, card.width, metrics.main_header_height);
        let filter_band = Rect::new(card.x, header.bottom(), card.width, metrics.subheader_height);
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
    /// A shell caps its reading measure well below the canvas width; the cap
    /// is the same number whether or not the right dock is open, because the
    /// dock overlays rather than reflows.
    #[must_use]
    pub fn measure(&self, metrics: &ShellMetrics) -> f32 {
        self.canvas.width.min(metrics.agent_measure_max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::SAMPLE_METRICS;

    const CONTENT_WIDTH: f32 = 1274.0;
    const CONTENT_HEIGHT: f32 = 796.0;

    fn default_regions() -> ShellRegions {
        ShellRegions::for_content(
            &SAMPLE_METRICS,
            CONTENT_WIDTH,
            CONTENT_HEIGHT,
            SAMPLE_METRICS.sidebar_width_default,
            Some(SAMPLE_METRICS.agent_panel_width),
        )
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
            SAMPLE_METRICS.main_page_margin
        );
        assert_eq!(
            regions.card.y - regions.tab_strip.bottom(),
            SAMPLE_METRICS.main_page_margin
        );
        assert_eq!(
            CONTENT_WIDTH - regions.card.right(),
            SAMPLE_METRICS.main_page_margin
        );
        assert_eq!(
            regions.status_band.y - regions.card.bottom(),
            SAMPLE_METRICS.main_page_margin
        );
    }

    #[test]
    fn the_header_and_filter_band_stack_on_the_cards_leading_edge() {
        let regions = default_regions();

        assert_eq!(regions.header.x, regions.card.x);
        assert_eq!(regions.header.y, regions.card.y);
        assert_eq!(regions.header.height, SAMPLE_METRICS.main_header_height);
        assert_eq!(regions.filter_band.y, regions.header.bottom());
        assert_eq!(regions.filter_band.height, SAMPLE_METRICS.subheader_height);
        assert_eq!(regions.canvas.y, regions.filter_band.bottom());
        assert_eq!(regions.canvas.bottom(), regions.card.bottom());
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
                &SAMPLE_METRICS,
                CONTENT_WIDTH,
                CONTENT_HEIGHT,
                SAMPLE_METRICS.sidebar_width_default,
                None,
            )
            .canvas
            .width
        );
    }

    /// The measure cap is a property of the table, not of what is drawn over
    /// the canvas, which is the same statement the overlay law makes from the
    /// other side.
    #[test]
    fn the_measure_caps_at_the_tables_maximum_however_wide_the_canvas_is() {
        let regions = default_regions();
        assert_eq!(
            regions.measure(&SAMPLE_METRICS),
            SAMPLE_METRICS.agent_measure_max
        );

        let narrow =
            ShellRegions::for_content(&SAMPLE_METRICS, 600.0, CONTENT_HEIGHT, 0.0, None);
        assert_eq!(narrow.measure(&SAMPLE_METRICS), narrow.canvas.width);
    }

    #[test]
    fn a_collapsed_sidebar_gives_its_width_to_the_card() {
        let collapsed = ShellRegions::for_content(
            &SAMPLE_METRICS,
            CONTENT_WIDTH,
            CONTENT_HEIGHT,
            0.0,
            None,
        );

        assert_eq!(collapsed.sidebar.width, 0.0);
        assert_eq!(
            collapsed.card.width,
            default_regions().card.width + SAMPLE_METRICS.sidebar_width_default
        );
    }

    #[test]
    fn a_window_narrower_than_its_chrome_never_produces_a_negative_region() {
        let tiny = ShellRegions::for_content(
            &SAMPLE_METRICS,
            120.0,
            40.0,
            SAMPLE_METRICS.sidebar_width_default,
            Some(400.0),
        );

        assert!(tiny.card.width >= 0.0);
        assert!(tiny.card.height >= 0.0);
        assert!(tiny.canvas.height >= 0.0);
        assert!(tiny.right_dock.expect("a pinned dock still resolves").width >= 0.0);
    }
}
