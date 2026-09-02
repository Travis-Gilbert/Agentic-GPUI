use gpui::{
    App, ElementId, IntoElement, RenderOnce, SharedString, StyleRefinement, Styled, Window,
};

use crate::text::{TextView, TextViewStyle};

/// A run of text the reader can select and copy.
///
/// A label names a thing and is chrome; a value *is* the thing, and a reader
/// has a reason to take it somewhere else -- a title into a message, a date
/// into a calendar, an id into a query, a path into a terminal, an address
/// into a map. Text drawn with plain `div().child(..)` cannot be selected at
/// all, so those values are readable and unreachable at the same time. This
/// element is the difference.
///
/// It is a [`TextView`] in its plain format with selection on: a drag
/// selects, `cmd-c` (`ctrl-c` off macOS) copies through the platform
/// clipboard, and every character is taken literally rather than read as
/// markup.
///
/// The `id` must be unique among the selectable values on screen, because it
/// keys the retained selection. Two rows of a table that share one id share
/// one selection, so derive it from the record, not from the call site.
#[derive(IntoElement)]
pub struct SelectableText {
    id: ElementId,
    text: SharedString,
    style: StyleRefinement,
    text_view_style: TextViewStyle,
}

impl SelectableText {
    /// Create a selectable run of `text`, keyed by `id`.
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            style: StyleRefinement::default(),
            text_view_style: TextViewStyle::default(),
        }
    }

    /// Set the [`TextViewStyle`] used to draw the text.
    pub fn text_style(mut self, style: TextViewStyle) -> Self {
        self.text_view_style = style;
        self
    }
}

impl Styled for SelectableText {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for SelectableText {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let mut view = TextView::plain(self.id, self.text)
            .selectable(true)
            .style(self.text_view_style);
        // `TextView` has an inherent `style` taking a `TextViewStyle`, which
        // shadows the `Styled` accessor by name, so the trait method is
        // called explicitly.
        *Styled::style(&mut view) = self.style;
        view
    }
}

/// Create a [`SelectableText`] keyed by `id`.
pub fn selectable_text(id: impl Into<ElementId>, text: impl Into<SharedString>) -> SelectableText {
    SelectableText::new(id, text)
}
