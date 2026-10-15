//! Typed facts for a custom header built from the same ordinary Buttons.
//! The caller still builds the actual native header and semantic wrappers.
use crate::ElementExt as _;
use crate::button::Button;
use gpui::{
    AnyElement, App, Bounds, ClickEvent, ElementId, IntoElement as _, ParentElement as _, Pixels,
    SharedString, Styled as _, Window, div,
};
use std::{cell::Cell, rc::Rc};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandHeaderActionKind {
    Button,
    Radio,
}
#[derive(Clone)]
pub struct CommandHeaderAction {
    pub id: SharedString,
    pub label: SharedString,
    pub description: Option<SharedString>,
    pub disabled: bool,
    pub selected: bool,
    pub(crate) appearance: crate::button::ButtonHeaderAppearance,
    pub kind: CommandHeaderActionKind,
    pub(crate) bounds: Rc<Cell<Bounds<Pixels>>>,
    pub(crate) handler: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>,
}
impl CommandHeaderAction {
    pub fn styles(&self, cx: &mut App) -> crate::button::ButtonPresentationStyles {
        self.appearance.styles(cx)
    }
    pub fn instance_style(&self) -> &gpui::StyleRefinement {
        self.appearance.instance()
    }
    pub fn content_style(&self) -> &gpui::StyleRefinement {
        self.appearance.content()
    }

    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds.get()
    }
    pub(crate) fn snapshot(&self) -> Self {
        let mut value = self.clone();
        value.bounds = Rc::new(Cell::new(self.bounds.get()));
        value
    }
    pub(crate) fn facts(
        &self,
    ) -> (
        SharedString,
        SharedString,
        Option<SharedString>,
        bool,
        bool,
        CommandHeaderActionKind,
    ) {
        (
            self.id.clone(),
            self.label.clone(),
            self.description.clone(),
            self.disabled,
            self.selected,
            self.kind,
        )
    }
}
pub struct CommandHeader {
    pub(crate) element: AnyElement,
    pub(crate) actions: Vec<CommandHeaderAction>,
    pub(crate) complete: bool,
}
/// A construction resource, consumed into the actual Command render. It owns
/// no visibility, selection, query, callback routing or matching state.
pub struct CommandHeaderBuilder {
    actions: Vec<CommandHeaderAction>,
    complete: bool,
}
impl Default for CommandHeaderBuilder {
    fn default() -> Self {
        Self {
            actions: Vec::new(),
            complete: true,
        }
    }
}
impl CommandHeaderBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    /// Capture this exact ordinary Button's handler before its one native
    /// render. Opaque/custom children make the whole header native-only.
    pub fn button(&mut self, id: impl Into<SharedString>, button: Button) -> AnyElement {
        self.button_kind(id, button, CommandHeaderActionKind::Button)
    }
    pub fn radio(&mut self, id: impl Into<SharedString>, button: Button) -> AnyElement {
        self.button_kind(id, button, CommandHeaderActionKind::Radio)
    }
    fn button_kind(
        &mut self,
        id: impl Into<SharedString>,
        mut button: Button,
        kind: CommandHeaderActionKind,
    ) -> AnyElement {
        let id = id.into();
        let facts = button.command_header_action();
        match facts {
            Some(source) if source.id == ElementId::Name(id.clone()) => {
                let bounds = Rc::new(Cell::new(Bounds::default()));
                self.actions.push(CommandHeaderAction {
                    id,
                    label: source.label,
                    description: source.description,
                    disabled: source.disabled,
                    selected: source.selected,
                    appearance: source.appearance,
                    kind,
                    bounds: bounds.clone(),
                    handler: source.handler,
                });
                div()
                    .flex_none()
                    .on_prepaint(move |actual, _, _| bounds.set(actual))
                    .child(button)
                    .into_any_element()
            }
            _ => {
                self.complete = false;
                button.into_any_element()
            }
        }
    }
    pub fn finish(self, element: impl gpui::IntoElement) -> CommandHeader {
        CommandHeader {
            element: element.into_any_element(),
            actions: self.actions,
            complete: self.complete,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Disableable as _,Selectable as _,
        command::{
            Command, CommandInputRefusal, CommandItem, CommandPresentation, CommandPresenter,
            CommandPresenterFactory, CommandState,
        },
    };
    use gpui::{
        AppContext as _, Context, Entity, ParentElement as _, Render, TestAppContext,
        prelude::FluentBuilder as _, px,
    };
    use std::cell::RefCell;
    struct Fixture {
        state: Entity<CommandState>,
        factory: CommandPresenterFactory,
        count: Rc<Cell<u32>>,
        show: bool,
        disabled: bool,
    }
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui::IntoElement {
            let target = self.state.downgrade();
            let count = self.count.clone();
            let disabled = self.disabled;
            let command = Command::new(&self.state)
                .item(CommandItem::new().label("Native owner row"))
                .structured_header(move |_, _, _| {
                    let target = target.clone();
                    let count = count.clone();
                    let mut header = CommandHeaderBuilder::new();
                    let button = header.radio(
                        "header.place",
                        Button::new("header.place")
                            .label("Add beside")
                            .selected(true)
                            .disabled(disabled)
                            .on_click(move |_, _, cx| {
                                // This is the actual Button listener, intentionally reentering the
                                // SAME retained CommandState that provides its presentation lease.
                                target.upgrade().unwrap().update(cx, |state, cx| {
                                    let _ = state.query(cx);
                                    count.set(count.get() + 1);
                                });
                            }),
                    );
                    header.finish(crate::h_flex().p_1().child(button))
                })
                .presenter("header-invoker", self.factory.clone());
            div()
                .w(px(360.))
                .h(px(280.))
                .when(self.show, |this| this.child(command))
        }
    }
    #[gpui::test]
    fn structured_command_header_invokes_current_button_outside_command_lease(
        cx: &mut TestAppContext,
    ) {
        cx.update(crate::init);
        let frames: Rc<RefCell<Vec<CommandPresentation>>> = Rc::default();
        let captured = frames.clone();
        let factory: CommandPresenterFactory = Rc::new(move |_| {
            let frames = captured.clone();
            CommandPresenter::new(
                move |frame, _, _| {
                    frames.borrow_mut().push(frame);
                    true
                },
                |_| {},
            )
        });
        let (view, cx) = cx.add_window_view(move |window, cx| Fixture {
            state: cx.new(|cx| CommandState::new(window, cx)),
            factory,
            count: Rc::new(Cell::new(0)),
            show: true,
            disabled: false,
        });
        cx.run_until_parked();
        let first = frames.borrow().last().cloned().unwrap();
        assert!(first.is_complete());
        assert_eq!(first.header_actions.len(), 1);
        assert_eq!(first.header_actions[0].label.as_ref(), "Add beside");
        assert!(first.header_actions[0].selected);
        assert_eq!(first.header_actions[0].kind, CommandHeaderActionKind::Radio);
        assert!(first.header_actions[0].bounds().size.width > px(0.));
        cx.update(|window, cx| {
            first
                .deliver_header_action("header.place", &ClickEvent::default(), window, cx)
                .unwrap()
        });
        assert_eq!(view.read_with(cx, |view, _| view.count.get()), 1);
        view.update(cx, |view, cx| {
            view.disabled = true;
            cx.notify();
        });
        cx.run_until_parked();
        let disabled = frames.borrow().last().cloned().unwrap();
        cx.update(|window, cx| {
            assert_eq!(
                first.deliver_header_action("header.place", &ClickEvent::default(), window, cx),
                Err(CommandInputRefusal::StaleRevision)
            );
            assert_eq!(
                disabled.deliver_header_action("header.place", &ClickEvent::default(), window, cx),
                Err(CommandInputRefusal::Disabled)
            );
        });
        assert_eq!(view.read_with(cx, |view, _| view.count.get()), 1);
        view.update(cx, |view, cx| {
            view.show = false;
            cx.notify();
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
            window.refresh();
        });
        cx.run_until_parked();
        cx.update(|window, cx| {
            assert_eq!(
                disabled.deliver_header_action("header.place", &ClickEvent::default(), window, cx),
                Err(CommandInputRefusal::NotPresented)
            )
        });
        assert_eq!(view.read_with(cx, |view, _| view.count.get()), 1);
    }
}
