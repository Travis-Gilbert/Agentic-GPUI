use std::{ops::Range, rc::Rc};

use crate::{
    IconName, Sizable, Size, StyledExt,
    group_box::GroupBoxVariant,
    h_resizable,
    input::{Input, InputState},
    resizable_panel,
    setting::SettingPage,
    sidebar::{Sidebar, SidebarGroup, SidebarMenu, SidebarMenuItem},
};
use gpui::{
    App, AppContext as _, Axis, ElementId, Entity, IntoElement, ParentElement as _, Pixels,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, container_query, div,
    prelude::FluentBuilder as _, px, relative,
};
use rust_i18n::t;

const STACKED_LAYOUT_MAX_WIDTH: Pixels = px(480.);

/// A labelled group of pages in the settings sidebar.
#[derive(Clone)]
pub struct SettingsSection {
    title: SharedString,
    pages: Vec<SettingPage>,
    show_when_empty: bool,
}

impl SettingsSection {
    /// Create a settings section with the given title.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            pages: Vec::new(),
            show_when_empty: false,
        }
    }

    /// Add a page to this section.
    pub fn page(mut self, page: SettingPage) -> Self {
        self.pages.push(page);
        self
    }

    /// Add pages to this section.
    pub fn pages(mut self, pages: impl IntoIterator<Item = SettingPage>) -> Self {
        self.pages.extend(pages);
        self
    }

    /// Keep this section's label visible when it has no pages.
    ///
    /// Empty sections remain hidden during search so the results list only
    /// contains matching navigation.
    pub fn show_when_empty(mut self, show_when_empty: bool) -> Self {
        self.show_when_empty = show_when_empty;
        self
    }
}

type SelectIndexChange = Rc<dyn Fn(SelectIndex, &mut Window, &mut App)>;

/// A section as the sidebar shows it: its title and the global indices of
/// its visible pages. Page indices count across every section in order, the
/// same numbering [`SelectIndex::page_ix`] uses.
type VisibleSection = (SharedString, Vec<usize>);

/// The settings structure containing multiple pages for app settings.
///
/// The hierarchy of settings is as follows:
///
/// ```ignore
/// Settings
///   SettingPage     <- The single active page displayed
///     SettingGroup
///       SettingItem
///         Label
///         SettingField (e.g., Switch, Dropdown, Input)
/// ```
#[derive(IntoElement)]
pub struct Settings {
    id: ElementId,
    pages: Vec<SettingPage>,
    sections: Vec<SettingsSection>,
    group_variant: GroupBoxVariant,
    size: Size,
    sidebar_width: Pixels,
    sidebar_size_range: Range<Pixels>,
    sidebar_style: StyleRefinement,
    default_selected_index: SelectIndex,
    selected_index: Option<SelectIndex>,
    on_selected_index_change: Option<SelectIndexChange>,
    header_style: StyleRefinement,
}

impl Settings {
    /// Create a new settings with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            pages: vec![],
            sections: vec![],
            group_variant: GroupBoxVariant::default(),
            size: Size::default(),
            sidebar_width: px(250.0),
            sidebar_size_range: px(160.0)..px(360.0),
            sidebar_style: StyleRefinement::default(),
            default_selected_index: SelectIndex::default(),
            selected_index: None,
            on_selected_index_change: None,
            header_style: StyleRefinement::default(),
        }
    }

    /// Set the width of the sidebar, default is `250px`.
    pub fn sidebar_width(mut self, width: impl Into<Pixels>) -> Self {
        self.sidebar_width = width.into();
        self
    }

    /// Set the resize range of the sidebar, default is `160px..360px`.
    pub fn sidebar_size_range(mut self, range: impl Into<Range<Pixels>>) -> Self {
        self.sidebar_size_range = range.into();
        self
    }

    /// Add a page to the settings.
    pub fn page(mut self, page: SettingPage) -> Self {
        self.pages.push(page);
        self
    }

    /// Add pages to the settings.
    pub fn pages(mut self, pages: impl IntoIterator<Item = SettingPage>) -> Self {
        self.pages.extend(pages);
        self
    }

    /// Add a labelled section of pages to the settings sidebar.
    ///
    /// When at least one section is present, the sidebar renders sectioned
    /// navigation instead of the flat pages added with [`Self::page`] or
    /// [`Self::pages`].
    pub fn section(mut self, section: SettingsSection) -> Self {
        self.sections.push(section);
        self
    }

    /// Add labelled sections of pages to the settings sidebar.
    pub fn sections(mut self, sections: impl IntoIterator<Item = SettingsSection>) -> Self {
        self.sections.extend(sections);
        self
    }

    /// Set the default variant for all setting groups.
    ///
    /// All setting groups will use this variant unless overridden individually.
    pub fn with_group_variant(mut self, variant: GroupBoxVariant) -> Self {
        self.group_variant = variant;
        self
    }

    /// Set the style refinement for the sidebar.
    pub fn sidebar_style(mut self, style: &StyleRefinement) -> Self {
        self.sidebar_style = style.clone();
        self
    }

    /// Set the default index of the page to be selected.
    pub fn default_selected_index(mut self, index: SelectIndex) -> Self {
        self.default_selected_index = index;
        self
    }

    /// Control the selected page from caller-owned state.
    ///
    /// Unlike [`Self::default_selected_index`], this value is reconciled on
    /// every render and is therefore suitable for deep links and external
    /// navigation state.
    pub fn selected_index(mut self, index: SelectIndex) -> Self {
        self.selected_index = Some(index);
        self
    }

    /// Observe page and group selection initiated in the Settings sidebar.
    pub fn on_selected_index_change(
        mut self,
        listener: impl Fn(SelectIndex, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_selected_index_change = Some(Rc::new(listener));
        self
    }

    /// Set the style refinement for the header.
    pub fn header_style(mut self, style: &StyleRefinement) -> Self {
        self.header_style = style.clone();
        self
    }

    /// Every page in display order: the sections' pages when any section is
    /// present, the flat pages otherwise.
    fn take_pages(&mut self) -> Vec<SettingPage> {
        if self.sections.is_empty() {
            std::mem::take(&mut self.pages)
        } else {
            self.sections
                .iter()
                .flat_map(|section| section.pages.iter().cloned())
                .collect()
        }
    }

    /// The sections the sidebar shows for this filter, each with the global
    /// indices of its visible pages.
    fn visible_sections(&self, filter: &SettingsFilter, query: &str) -> Vec<VisibleSection> {
        let mut start = 0;
        self.sections
            .iter()
            .filter_map(|section| {
                let range = start..start + section.pages.len();
                start = range.end;
                let pages = filter
                    .visible_pages()
                    .filter(|ix| range.contains(ix))
                    .collect::<Vec<_>>();
                (!pages.is_empty() || (section.show_when_empty && query.is_empty()))
                    .then(|| (section.title.clone(), pages))
            })
            .collect()
    }

    fn render_active_page(
        pages: &[SettingPage],
        state: &Entity<SettingsState>,
        filter: &SettingsFilter,
        options: &RenderOptions,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::AnyElement {
        let page_ix = state.read(cx).selected_index.page_ix;
        if let Some(page) = pages.get(page_ix)
            && !filter.groups[page_ix].is_empty()
        {
            return page
                .render(page_ix, &filter.groups[page_ix], state, options, window, cx)
                .into_any_element();
        }

        div().into_any_element()
    }

    fn sidebar_menu_item(
        state: &Entity<SettingsState>,
        selected_index: SelectIndex,
        page_ix: usize,
        page: &SettingPage,
        groups: &[usize],
        on_selected_index_change: Option<&SelectIndexChange>,
    ) -> SidebarMenuItem {
        let is_page_active = selected_index.page_ix == page_ix
            && (selected_index.group_ix.is_none() || groups.len() == 1);
        SidebarMenuItem::new(page.title.clone())
            .click_to_open(true)
            .when_some(page.icon.clone(), |this, icon| this.icon(icon))
            .default_open(page.default_open)
            .active(is_page_active)
            .on_click({
                let state = state.clone();
                let on_selected_index_change = on_selected_index_change.cloned();
                move |_, window, cx| {
                    let selected_index = SelectIndex {
                        page_ix,
                        ..Default::default()
                    };
                    state.update(cx, |state, cx| {
                        state.selected_index = selected_index;
                        state.deferred_scroll_group_ix = None;
                        cx.notify();
                    });
                    if let Some(listener) = &on_selected_index_change {
                        listener(selected_index, window, cx);
                    }
                }
            })
            .when(groups.len() > 1, |this| {
                this.children(
                    groups
                        .iter()
                        .copied()
                        .filter(|&ix| page.groups[ix].title.is_some())
                        .map(|group_ix| {
                            let group = &page.groups[group_ix];
                            let is_active = selected_index.page_ix == page_ix
                                && selected_index.group_ix == Some(group_ix);
                            let title = group.title.clone().unwrap_or_default();

                            SidebarMenuItem::new(title).active(is_active).on_click({
                                let state = state.clone();
                                let on_selected_index_change = on_selected_index_change.cloned();
                                move |_, window, cx| {
                                    let selected_index = SelectIndex {
                                        page_ix,
                                        group_ix: Some(group_ix),
                                    };
                                    state.update(cx, |state, cx| {
                                        state.selected_index = selected_index;
                                        state.deferred_scroll_group_ix = Some(group_ix);
                                        cx.notify();
                                    });
                                    if let Some(listener) = &on_selected_index_change {
                                        listener(selected_index, window, cx);
                                    }
                                }
                            })
                        }),
                )
            })
    }

    fn render_sidebar(
        &self,
        pages: &[SettingPage],
        sections: &[VisibleSection],
        state: &Entity<SettingsState>,
        filter: &SettingsFilter,
        _: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        let selected_index = state.read(cx).selected_index;
        let search_input = state.read(cx).search_input.clone();
        let on_change = self.on_selected_index_change.as_ref();
        let item = |page_ix: usize| {
            Self::sidebar_menu_item(
                state,
                selected_index,
                page_ix,
                &pages[page_ix],
                &filter.groups[page_ix],
                on_change,
            )
        };

        Sidebar::new("settings-sidebar")
            .w(relative(1.))
            .border_0()
            .refine_style(&self.sidebar_style)
            .collapsible(false)
            .collapsed(false)
            .header(
                div()
                    .w_full()
                    .refine_style(&self.header_style)
                    .child(Input::new(&search_input).prefix(IconName::Search)),
            )
            .when(self.sections.is_empty(), |this| {
                this.child(SidebarMenu::new().children(filter.visible_pages().map(&item)))
            })
            .children(sections.iter().map(|(title, page_ixs)| {
                SidebarGroup::new(title.clone())
                    .child(SidebarMenu::new().children(page_ixs.iter().copied().map(&item)))
            }))
    }
}

impl Sizable for Settings {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

/// Visible groups in each original page. Filtering never renumbers source data.
struct SettingsFilter {
    groups: Vec<Vec<usize>>,
}

impl SettingsFilter {
    fn new(pages: &[SettingPage], query: &str, cx: &App) -> Self {
        Self {
            groups: pages
                .iter()
                .map(|page| {
                    page.groups
                        .iter()
                        .enumerate()
                        .filter_map(|(ix, group)| group.is_match(query, cx).then_some(ix))
                        .collect()
                })
                .collect(),
        }
    }

    fn visible_pages(&self) -> impl Iterator<Item = usize> + '_ {
        self.groups
            .iter()
            .enumerate()
            .filter_map(|(ix, groups)| (!groups.is_empty()).then_some(ix))
    }

    fn selected_index(&self, selected: SelectIndex) -> SelectIndex {
        let page_ix = self
            .visible_pages()
            .find(|&ix| ix == selected.page_ix)
            .or_else(|| self.visible_pages().next());
        let Some(page_ix) = page_ix else {
            // Keep the selection while there are no results so clearing the query
            // can restore it. The empty filter prevents rendering a stale page.
            return selected;
        };

        SelectIndex {
            page_ix,
            group_ix: selected
                .group_ix
                .filter(|ix| page_ix == selected.page_ix && self.groups[page_ix].contains(ix)),
        }
    }
}

pub(super) struct SettingsState {
    pub(super) selected_index: SelectIndex,
    /// If set, defer scrolling to this group index after rendering.
    pub(super) deferred_scroll_group_ix: Option<usize>,
    pub(super) search_input: Entity<InputState>,
}

/// Options for rendering setting item.
///
/// The fields are private and reached through the methods below, so that a new
/// one can be added without breaking the item renderers. The setters take
/// `self` by value, so a nested renderer narrows a copy of its parent options:
///
/// ```ignore
/// item.render_item(&options.with_item_ix(item_ix), window, cx)
/// ```
#[derive(Clone, Copy)]
pub struct RenderOptions {
    page_ix: usize,
    group_ix: usize,
    item_ix: usize,
    size: Size,
    group_variant: GroupBoxVariant,
    layout: Axis,
    disabled: bool,
}

impl RenderOptions {
    pub fn new() -> Self {
        Self {
            page_ix: 0,
            group_ix: 0,
            item_ix: 0,
            size: Size::default(),
            group_variant: GroupBoxVariant::default(),
            layout: Axis::Horizontal,
            disabled: false,
        }
    }

    pub fn with_page_ix(mut self, page_ix: usize) -> Self {
        self.page_ix = page_ix;
        self
    }

    pub fn with_group_ix(mut self, group_ix: usize) -> Self {
        self.group_ix = group_ix;
        self
    }

    pub fn with_item_ix(mut self, item_ix: usize) -> Self {
        self.item_ix = item_ix;
        self
    }

    pub fn with_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn with_group_variant(mut self, group_variant: GroupBoxVariant) -> Self {
        self.group_variant = group_variant;
        self
    }

    pub fn with_layout(mut self, layout: Axis) -> Self {
        self.layout = layout;
        self
    }

    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn page_ix(&self) -> usize {
        self.page_ix
    }

    pub fn group_ix(&self) -> usize {
        self.group_ix
    }

    pub fn item_ix(&self) -> usize {
        self.item_ix
    }

    pub fn size(&self) -> Size {
        self.size
    }

    pub fn group_variant(&self) -> GroupBoxVariant {
        self.group_variant
    }

    pub fn layout(&self) -> Axis {
        self.layout
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectIndex {
    pub page_ix: usize,
    pub group_ix: Option<usize>,
}

impl RenderOnce for Settings {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |window, cx| {
            let search_input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder(t!("Settings.search_placeholder"))
                    .default_value("")
            });

            SettingsState {
                search_input,
                selected_index: self.default_selected_index,
                deferred_scroll_group_ix: None,
            }
        });

        if let Some(selected_index) = self.selected_index
            && state.read(cx).selected_index != selected_index
        {
            state.update(cx, |state, _| {
                state.selected_index = selected_index;
                state.deferred_scroll_group_ix = selected_index.group_ix;
            });
        }

        let pages = self.take_pages();
        let query = state.read(cx).search_input.read(cx).value();
        let filter = SettingsFilter::new(&pages, &query, cx);
        let sections = self.visible_sections(&filter, &query);
        let previous = state.read(cx).selected_index;
        let selected = filter.selected_index(previous);
        if selected != previous {
            state.update(cx, |state, _| {
                state.selected_index = selected;
                state.deferred_scroll_group_ix = None;
            });
        }
        let options = RenderOptions::new()
            .with_size(self.size)
            .with_group_variant(self.group_variant);
        let sidebar_size_range = self.sidebar_size_range.clone();
        let sidebar = self
            .render_sidebar(&pages, &sections, &state, &filter, window, cx)
            .into_any_element();

        h_resizable(self.id.clone())
            .child(
                resizable_panel()
                    .size(self.sidebar_width)
                    .size_range(sidebar_size_range)
                    .child(sidebar),
            )
            .child(
                resizable_panel().child(container_query(move |size, window, cx| {
                    let options = options.with_layout(if size.width <= STACKED_LAYOUT_MAX_WIDTH {
                        Axis::Vertical
                    } else {
                        Axis::Horizontal
                    });
                    Self::render_active_page(&pages, &state, &filter, &options, window, cx)
                })),
            )
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
mod section_tests {
    use super::*;
    use crate::setting::{SettingGroup, SettingItem};
    use gpui::TestAppContext;

    fn page(title: &'static str, keyword: &'static str) -> SettingPage {
        SettingPage::new(title).group(
            SettingGroup::new().item(SettingItem::render(|_, _, _| div()).keywords([keyword])),
        )
    }

    fn sections_for(settings: &mut Settings, query: &str, cx: &App) -> Vec<VisibleSection> {
        let pages = settings.take_pages();
        let filter = SettingsFilter::new(&pages, query, cx);
        settings.visible_sections(&filter, query)
    }

    #[gpui::test]
    fn section_search_preserves_global_page_indices(cx: &mut TestAppContext) {
        let mut settings = Settings::new("settings")
            .section(
                SettingsSection::new("User")
                    .page(page("Profile", "identity"))
                    .page(page("Experience", "appearance")),
            )
            .section(
                SettingsSection::new("Workspace")
                    .page(page("General", "workspace"))
                    .page(page("Models", "keychain")),
            )
            .section(SettingsSection::new("Other").page(page("Data", "keychain")));

        let sections = cx.update(|cx| sections_for(&mut settings, "keychain", cx));

        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].0, "Workspace");
        assert_eq!(sections[1].0, "Other");
        assert_eq!(sections[0].1, vec![3]);
        assert_eq!(sections[1].1, vec![4]);
    }

    #[test]
    fn section_builder_keeps_pages_in_insertion_order() {
        let section = SettingsSection::new("Workspace")
            .page(page("General", "workspace"))
            .pages([page("Models", "keychain"), page("API keys", "credentials")]);

        assert_eq!(section.title, "Workspace");
        assert_eq!(section.pages.len(), 3);
        assert_eq!(section.pages[0].title, "General");
        assert_eq!(section.pages[1].title, "Models");
        assert_eq!(section.pages[2].title, "API keys");
    }

    #[gpui::test]
    fn opted_in_empty_section_is_visible_until_search_starts(cx: &mut TestAppContext) {
        let build = || {
            Settings::new("settings")
                .section(SettingsSection::new("User").page(page("Profile", "identity")))
                .section(SettingsSection::new("Other").show_when_empty(true))
        };

        let unfiltered = cx.update(|cx| sections_for(&mut build(), "", cx));
        assert_eq!(unfiltered.len(), 2);
        assert_eq!(unfiltered[1].0, "Other");
        assert!(unfiltered[1].1.is_empty());

        let searched = cx.update(|cx| sections_for(&mut build(), "identity", cx));
        assert_eq!(searched.len(), 1);
        assert_eq!(searched[0].0, "User");
    }

    #[test]
    fn sections_replace_flat_pages() {
        let mut settings = Settings::new("settings")
            .page(page("Flat", "flat"))
            .section(SettingsSection::new("User").page(page("Profile", "identity")));

        let pages = settings.take_pages();
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].title, "Profile");
    }

    #[test]
    fn controlled_selection_is_opt_in_and_distinct_from_the_default() {
        let controlled = SelectIndex {
            page_ix: 4,
            group_ix: Some(1),
        };
        let settings = Settings::new("settings")
            .default_selected_index(SelectIndex {
                page_ix: 1,
                group_ix: None,
            })
            .selected_index(controlled);

        assert_eq!(settings.selected_index, Some(controlled));
        assert_eq!(settings.default_selected_index.page_ix, 1);
        assert_eq!(Settings::new("legacy").selected_index, None);
    }
}
