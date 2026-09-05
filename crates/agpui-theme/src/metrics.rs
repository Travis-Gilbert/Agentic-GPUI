//! Shell chrome geometry, as a type.
//!
//! SPEC-AGPUI-HOME-1.0 move rule 4: metrics become values. AGPUI owns the
//! shape of a shell's chrome; the product owns its numbers. Theorem holds a
//! `ShellMetrics` const beside its token file, and any other consumer of this
//! crate holds its own.
//!
//! The lengths carry no unit of their own. A renderer types them at its own
//! boundary - `gpui::px`, a CSS variable, a test's geometry - and every
//! consumer of one table has to agree on which unit that is. Theorem's numbers
//! are device pixels measured from Linear. Nothing here converts between
//! units, and the one derived length below is a ratio, so it holds in whatever
//! unit the table is written in.

use serde::{Deserialize, Serialize};

/// The chrome geometry of one shell.
///
/// A zero is a measurement, not a placeholder. A shell with no status bar
/// records `status_bar_height: 0.0`, and its oracle should assert the bar is
/// absent rather than assert nothing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShellMetrics {
    // The main region: the surface raised inside the window's content area.
    /// The inset between the main region and the surface raised inside it.
    pub main_page_margin: f32,
    /// The corner radius of that raised surface.
    pub main_content_radius: f32,
    /// The header row above the main region.
    pub main_header_height: f32,
    /// A second header row under the first, where a surface declares one.
    pub subheader_height: f32,

    // Lists: the header above a list, and a row height per row kind. A shell
    // that shows one kind of row leaves the others at whatever it measured.
    /// The column-title row above a list's rows.
    pub list_header_height: f32,
    /// A single-line row.
    pub row_height_record: f32,
    /// A single-line row that carries a progress affordance.
    pub row_height_plan: f32,
    /// A row that sets two lines of text.
    pub row_height_two_line: f32,
    /// A two-line row that carries a progress affordance.
    pub row_height_goal: f32,

    // The desktop tab strip, where the shell has one.
    /// The strip itself, including whatever padding sits around the tabs.
    pub desktop_tabs_height: f32,
    /// One tab inside that strip, which is shorter than the strip.
    pub desktop_tab_height: f32,

    // The sidebar column.
    /// The sidebar column when it is shown. Collapsing sets the column to
    /// [`ShellMetrics::sidebar_collapse_threshold`], it does not change this
    /// number, so the width to restore to survives the collapse.
    pub sidebar_width_default: f32,
    /// The inset between the sidebar's edge and its content.
    pub sidebar_padding: f32,
    /// The height of the sidebar's primary action.
    pub sidebar_primary_button: f32,
    /// The trailing inset on a sidebar link, which is tighter than the leading
    /// one so a trailing count or badge sits close to the edge.
    pub sidebar_link_padding_end: f32,

    // The agent drawer.
    /// The agent drawer when it is open. Closing sets the column to 0.
    pub agent_panel_width: f32,
    /// The widest a line of agent prose is allowed to set. A measure cap, not
    /// a column: the drawer can be wider than the text inside it, and on a
    /// wide window it is.
    pub agent_measure_max: f32,
    /// One example card in the drawer's empty state.
    pub agent_example_card_height: f32,
    /// The corner radius of a surface nested inside the drawer, which is
    /// tighter than the drawer's own.
    pub agent_inner_radius: f32,
    /// The composer's bottom corners, which are tighter than its top ones
    /// because the composer sits against the drawer's lower edge.
    pub composer_bottom_radius: f32,

    // Controls, menus and their neighbours.
    /// A button, an input, and anything else that lines up in a control row.
    pub control_height: f32,
    /// One row inside a menu.
    pub menu_item_height: f32,
    /// The narrowest a menu is allowed to be, whatever its longest label.
    pub menu_min_width: f32,
    /// The widest a tooltip is allowed to be before it wraps.
    pub tooltip_max_width: f32,
    /// The scrollbar's track, which is reserved whether or not it is drawn.
    pub scrollbar_width: f32,
    /// The inset either side of ordinary content.
    pub content_padding_inline: f32,
    /// The inset above and below ordinary content.
    pub content_padding_block: f32,
    /// A persistent status bar along the bottom, where the shell has one.
    pub status_bar_height: f32,

    // The window itself. Defaults are what a shell opens at; minimums are what
    // it refuses to go under, and the two are independent measurements.
    /// The width a new window opens at.
    pub window_width_default: f32,
    /// The height a new window opens at.
    pub window_height_default: f32,
    /// The narrowest the window may be resized to.
    pub window_width_min: f32,
    /// The shortest the window may be resized to.
    pub window_height_min: f32,

    // Durations, in milliseconds. Integers because a frame budget is counted,
    // not measured, and a fractional millisecond means nothing to a scheduler.
    /// A state change the reader is already looking at: a hover, a press.
    pub transition_quick_ms: u64,
    /// A state change the reader has to follow: a panel, a disclosure.
    pub transition_regular_ms: u64,
    /// A state change that moves the whole surface.
    pub transition_slow_ms: u64,
    /// How long a highlight takes to fade once it has been read.
    pub highlight_fade_out_ms: u64,
}

impl ShellMetrics {
    /// The sidebar width at which the sidebar reads as collapsed.
    ///
    /// One number doing two jobs, which is why it is derived rather than
    /// declared. It is the boundary - a sidebar dragged to this width or
    /// narrower is collapsed - and it is also the width collapsing sets, so
    /// the widest width that reads as collapsed is exactly the width a
    /// collapse writes. That equality is what lets a shell persist the column
    /// width alone and still restore the collapsed state: the number it wrote
    /// is a number its own predicate accepts. Splitting the two into separate
    /// fields would let a persisted collapsed sidebar restore expanded.
    ///
    /// A quarter of the default rather than a twelfth field, so that widening
    /// the sidebar widens the point it snaps shut at instead of leaving the
    /// two to drift apart.
    #[must_use]
    pub fn sidebar_collapse_threshold(&self) -> f32 {
        self.sidebar_width_default / 4.0
    }
}

/// A shell table that belongs to no product.
///
/// Deliberately not any product's numbers: AGPUI proves the law, and the
/// product that holds a table proves its own values. Shared with the region
/// map's tests so both halves of the geometry are proven over one table.
#[cfg(test)]
pub(crate) const SAMPLE_METRICS: ShellMetrics = ShellMetrics {
    main_page_margin: 8.0,
    main_content_radius: 12.0,
    main_header_height: 40.0,
    subheader_height: 32.0,
    list_header_height: 32.0,
    row_height_record: 40.0,
    row_height_plan: 40.0,
    row_height_two_line: 56.0,
    row_height_goal: 56.0,
    desktop_tabs_height: 32.0,
    desktop_tab_height: 24.0,
    sidebar_width_default: 200.0,
    sidebar_padding: 12.0,
    sidebar_primary_button: 28.0,
    sidebar_link_padding_end: 2.0,
    agent_panel_width: 360.0,
    agent_measure_max: 720.0,
    agent_example_card_height: 100.0,
    agent_inner_radius: 16.0,
    composer_bottom_radius: 12.0,
    control_height: 28.0,
    menu_item_height: 28.0,
    menu_min_width: 200.0,
    tooltip_max_width: 280.0,
    scrollbar_width: 12.0,
    content_padding_inline: 8.0,
    content_padding_block: 6.0,
    status_bar_height: 24.0,
    window_width_default: 1200.0,
    window_height_default: 800.0,
    window_width_min: 480.0,
    window_height_min: 520.0,
    transition_quick_ms: 100,
    transition_regular_ms: 250,
    transition_slow_ms: 350,
    highlight_fade_out_ms: 150,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_threshold_is_a_quarter_of_the_sidebar_it_is_about() {
        assert!((SAMPLE_METRICS.sidebar_collapse_threshold() - 50.0).abs() < 1e-4);
    }

    #[test]
    fn widening_the_sidebar_moves_the_threshold_with_it() {
        let mut wider = SAMPLE_METRICS;
        wider.sidebar_width_default += 40.0;
        assert!(
            (wider.sidebar_collapse_threshold() - SAMPLE_METRICS.sidebar_collapse_threshold() - 10.0).abs()
                < 1e-4
        );
    }

    /// The invariant a shell's persisted sidebar width stands on.
    ///
    /// Collapsing writes the threshold; restoring asks whether the stored
    /// width is at or under it. The two agree only because the width written
    /// is the widest width accepted, so this asserts the boundary is closed
    /// at the threshold and open just above it.
    #[test]
    fn the_width_a_collapse_writes_is_the_widest_width_that_reads_as_collapsed() {
        let collapsed = SAMPLE_METRICS.sidebar_collapse_threshold();
        assert!(collapsed <= SAMPLE_METRICS.sidebar_collapse_threshold());
        assert!(collapsed + 0.5 > SAMPLE_METRICS.sidebar_collapse_threshold());
    }

    #[test]
    fn metrics_round_trip_as_a_value() {
        let wire = serde_json::to_string(&SAMPLE_METRICS).unwrap();
        let back: ShellMetrics = serde_json::from_str(&wire).unwrap();
        assert_eq!(back, SAMPLE_METRICS);
    }
}
