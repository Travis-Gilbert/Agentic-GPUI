//! Placement for geometry supplied by an external layout owner.
//! This carries allocations; it does not recompute layout or own resize state.

use gpui::{
    AnyElement, App, Bounds, ElementId, InteractiveElement, IntoElement, ParentElement, Pixels,
    RenderOnce, Styled, Window, div,
};

/// A passive, clipped container with parent-local logical-pixel allocations.
/// Children are supplied back to front, with stable owner-derived identities.
#[derive(IntoElement)]
pub struct MeasuredLayout {
    id: ElementId,
    size: gpui::Size<Pixels>,
    children: Vec<AnyElement>,
}

impl MeasuredLayout {
    /// The external owner has already measured this container.
    pub fn new(id: impl Into<ElementId>, size: gpui::Size<Pixels>) -> Self {
        Self {
            id: id.into(),
            size,
            children: Vec::new(),
        }
    }

    /// Append an allocation in paint order. Zero-area children occupy no
    /// paint or pointer region; they remain in the owner's description.
    pub fn child(
        mut self,
        id: impl Into<ElementId>,
        bounds: Bounds<Pixels>,
        child: impl IntoElement,
    ) -> Self {
        if bounds.size.width > gpui::px(0.) && bounds.size.height > gpui::px(0.) {
            self.children.push(
                div()
                    .id(id)
                    .absolute()
                    .left(bounds.origin.x)
                    .top(bounds.origin.y)
                    .w(bounds.size.width)
                    .h(bounds.size.height)
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .child(child)
                    .into_any_element(),
            );
        }
        self
    }
}

impl RenderOnce for MeasuredLayout {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .relative()
            .w(self.size.width)
            .h(self.size.height)
            .flex_shrink_0()
            .overflow_hidden()
            .children(self.children)
    }
}
