//! Observe the actual root allocation without inserting a layout or hit region.

use gpui::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, Pixels, Window,
};

/// A transparent element-phase adapter. Unlike an absolute child probe, its
/// bounds include the observed root's border and padding. It delegates the
/// same layout ID and preserves the child's focus, accessibility and painting.
pub struct BoundsObserver {
    child: AnyElement,
    observe: Option<Box<dyn FnOnce(Bounds<Pixels>, &mut Window, &mut App)>>,
}

impl BoundsObserver {
    pub fn new(
        child: impl IntoElement,
        observe: impl FnOnce(Bounds<Pixels>, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            child: child.into_any_element(),
            observe: Some(Box::new(observe)),
        }
    }
}

impl IntoElement for BoundsObserver {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for BoundsObserver {
    type RequestLayoutState = ();
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        (self.child.request_layout(window, cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
        if let Some(observe) = self.observe.take() {
            observe(bounds, window, cx);
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}
