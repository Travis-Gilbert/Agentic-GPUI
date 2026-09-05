//! Shell chrome geometry, as a type.
//!
//! SPEC-AGPUI-HOME-1.0 move rule 4: metrics become values. AGPUI owns the
//! shape of a shell's chrome; the product owns its numbers. Theorem holds a
//! `ShellMetrics` const beside its token file, and any other consumer of this
//! crate holds its own.
//!
//! Every length is in rem. That is the unit the surfaces already agree in:
//! the Leptos shell writes `grid-cols-[3.2rem_16rem_minmax(0,1fr)_29rem]` and
//! GPUI carries its own rem, which is why Theorem's browser geometry oracle
//! multiplies by [`ShellMetrics::rem_px`] rather than trusting the browser's
//! default of 16. Storing px here would pin the type to one root size.

use serde::{Deserialize, Serialize};

/// The chrome geometry of one shell, in rem.
///
/// A zero is a measurement, not a placeholder. A shell with no status bar
/// records `status_bar_height: 0.0`, and its oracle should assert the bar is
/// absent rather than assert nothing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShellMetrics {
    /// CSS px per rem at the shell's root. The product decides this; Theorem
    /// sets 14 so the browser and GPUI measure the same box.
    pub rem_px: f32,
    /// The icon rail: the always-present column of surface tiles.
    pub rail_width: f32,
    /// The sidebar column when it is shown. Collapsing sets the column to 0,
    /// it does not change this number.
    pub sidebar_width_default: f32,
    /// The agent drawer when it is open. Closing sets the column to 0.
    pub agent_panel_width: f32,
    /// The widest a line of agent prose is allowed to set. A measure cap, not
    /// a column: the drawer can be wider than the text inside it.
    pub agent_measure_max: f32,
    /// The header row above the main region.
    pub main_header_height: f32,
    /// A second header row under the first, where a surface declares one.
    pub subheader_height: f32,
    /// A desktop tab strip, where the shell has one.
    pub desktop_tabs_height: f32,
    /// A persistent status bar along the bottom, where the shell has one.
    pub status_bar_height: f32,
    /// The inset between the main region and the surface raised inside it.
    pub main_page_margin: f32,
}

impl ShellMetrics {
    /// The viewport width below which the sidebar can no longer be shown
    /// without squeezing the measure.
    ///
    /// Derived rather than declared, which is why it is a method. Below this
    /// width the rail, the sidebar and a full-measure main region do not fit
    /// side by side, so something has to give and the sidebar is what gives.
    /// A shell that declared this as a twelfth field could let it drift out of
    /// agreement with the three widths it is a statement about.
    #[must_use]
    pub fn sidebar_collapse_threshold(&self) -> f32 {
        self.rail_width + self.sidebar_width_default + self.agent_measure_max
    }

    /// The same length in CSS px at this shell's root size.
    #[must_use]
    pub fn px(&self, rem: f32) -> f32 {
        rem * self.rem_px
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: ShellMetrics = ShellMetrics {
        rem_px: 14.0,
        rail_width: 3.2,
        sidebar_width_default: 16.0,
        agent_panel_width: 29.0,
        agent_measure_max: 48.0,
        main_header_height: 2.5,
        subheader_height: 0.0,
        desktop_tabs_height: 0.0,
        status_bar_height: 0.0,
        main_page_margin: 0.5,
    };

    #[test]
    fn the_threshold_is_derived_from_the_three_widths_it_is_about() {
        assert!((SAMPLE.sidebar_collapse_threshold() - 67.2).abs() < 1e-4);
    }

    #[test]
    fn widening_the_sidebar_moves_the_threshold_with_it() {
        let mut wider = SAMPLE;
        wider.sidebar_width_default += 4.0;
        assert!(
            (wider.sidebar_collapse_threshold() - SAMPLE.sidebar_collapse_threshold() - 4.0).abs()
                < 1e-4
        );
    }

    #[test]
    fn rem_converts_at_the_root_the_product_declared() {
        assert!((SAMPLE.px(3.2) - 44.8).abs() < 1e-4);
        assert!((SAMPLE.px(16.0) - 224.0).abs() < 1e-4);
    }

    #[test]
    fn metrics_round_trip_as_a_value() {
        let wire = serde_json::to_string(&SAMPLE).unwrap();
        let back: ShellMetrics = serde_json::from_str(&wire).unwrap();
        assert_eq!(back, SAMPLE);
    }
}
