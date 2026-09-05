//! Renderer-neutral TheoremWeb shell metrics measured from Linear.

/// Every fixed shell dimension. Renderers convert these scalar values at their
/// own typed boundary (`gpui::px`, CSS variables, or test geometry).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub main_page_margin: f32,
    pub main_content_radius: f32,
    pub main_header_height: f32,
    pub subheader_height: f32,
    pub list_header_height: f32,
    pub row_height_record: f32,
    pub row_height_plan: f32,
    pub row_height_two_line: f32,
    pub row_height_goal: f32,
    pub desktop_tabs_height: f32,
    pub desktop_tab_height: f32,
    pub sidebar_width_default: f32,
    pub sidebar_padding: f32,
    pub sidebar_primary_button: f32,
    pub sidebar_link_padding_end: f32,
    pub agent_panel_width: f32,
    pub agent_measure_max: f32,
    pub agent_example_card_height: f32,
    pub agent_inner_radius: f32,
    pub composer_bottom_radius: f32,
    pub control_height: f32,
    pub menu_item_height: f32,
    pub menu_min_width: f32,
    pub tooltip_max_width: f32,
    pub scrollbar_width: f32,
    pub content_padding_inline: f32,
    pub content_padding_block: f32,
    pub status_bar_height: f32,
    pub window_width_default: f32,
    pub window_height_default: f32,
    pub window_width_min: f32,
    pub window_height_min: f32,
    pub transition_quick_ms: u64,
    pub transition_regular_ms: u64,
    pub transition_slow_ms: u64,
    pub highlight_fade_out_ms: u64,
}

pub const METRICS: Metrics = Metrics {
    main_page_margin: 8.0,
    main_content_radius: 12.0,
    main_header_height: 44.0,
    subheader_height: 44.0,
    list_header_height: 36.0,
    row_height_record: 44.0,
    row_height_plan: 48.0,
    row_height_two_line: 58.0,
    row_height_goal: 58.0,
    desktop_tabs_height: 40.0,
    desktop_tab_height: 28.0,
    sidebar_width_default: 240.0,
    sidebar_padding: 12.0,
    sidebar_primary_button: 28.0,
    sidebar_link_padding_end: 2.0,
    agent_panel_width: 400.0,
    agent_measure_max: 821.0,
    agent_example_card_height: 108.0,
    agent_inner_radius: 20.0,
    composer_bottom_radius: 13.0,
    control_height: 28.0,
    menu_item_height: 28.0,
    menu_min_width: 240.0,
    tooltip_max_width: 280.0,
    scrollbar_width: 12.0,
    content_padding_inline: 8.0,
    content_padding_block: 6.0,
    status_bar_height: 28.0,
    window_width_default: 1280.0,
    window_height_default: 800.0,
    window_width_min: 500.0,
    window_height_min: 520.0,
    transition_quick_ms: 100,
    transition_regular_ms: 250,
    transition_slow_ms: 350,
    highlight_fade_out_ms: 150,
};

impl Metrics {
    #[must_use]
    pub fn sidebar_collapse_threshold(self) -> f32 {
        self.sidebar_width_default / 4.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_shell_geometry_is_one_table() {
        assert_eq!(METRICS.main_page_margin, 8.0);
        assert_eq!(METRICS.main_content_radius, 12.0);
        assert_eq!(METRICS.main_header_height, 44.0);
        assert_eq!(METRICS.sidebar_width_default, 240.0);
        assert_eq!(METRICS.sidebar_collapse_threshold(), 60.0);
        assert_eq!(METRICS.agent_measure_max, 821.0);
        assert_eq!(METRICS.agent_example_card_height, 108.0);
    }
}
