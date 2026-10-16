//! Optional presentation of the actual command owner. Matching, original
//! index paths, selection, scroll and callbacks remain in CommandState.
use super::{CommandRow, CommandState};
use crate::{
    IndexPath,
    actions::{Cancel, Confirm, SelectDown, SelectUp},
    input::InputState,
    kbd::Kbd,
};
use gpui::{App, Bounds, Context, Entity, Pixels, SharedString, WeakEntity, Window};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

pub type CommandPresenterFactory = Rc<dyn Fn(&str) -> CommandPresenter>;
#[derive(Clone)]
pub struct CommandPresenter {
    frame: Rc<dyn Fn(CommandPresentation, &mut Window, &mut App) -> bool>,
    closed: Rc<dyn Fn(u64)>,
    delegated: Rc<Cell<bool>>,
    identity: Rc<Cell<Option<u64>>>,
    retired: Rc<Cell<bool>>,
}
impl CommandPresenter {
    pub fn new(
        frame: impl Fn(CommandPresentation, &mut Window, &mut App) -> bool + 'static,
        closed: impl Fn(u64) + 'static,
    ) -> Self {
        Self {
            frame: Rc::new(frame),
            closed: Rc::new(closed),
            delegated: Rc::default(),
            identity: Rc::default(),
            retired: Rc::default(),
        }
    }
    pub fn is_platform_delegated(&self) -> bool {
        self.delegated.get() && !self.retired.get()
    }
    fn close(&self) {
        if !self.retired.replace(true) {
            self.delegated.set(false);
            if let Some(id) = self.identity.take() {
                (self.closed)(id);
            }
        }
    }
}
/// A real rendered-element lifetime guard. The query owner can persist while
/// its popup is closed, so its Entity's Drop is not a popup-close signal.
pub(crate) struct PresentationGuard {
    factory: CommandPresenterFactory,
    invoker: SharedString,
    presenter: CommandPresenter,
}
impl PresentationGuard {
    pub(crate) fn new(factory: CommandPresenterFactory, invoker: SharedString) -> Self {
        let presenter = factory(&invoker);
        Self {
            factory,
            invoker,
            presenter,
        }
    }
    pub(crate) fn install(
        &mut self,
        factory: CommandPresenterFactory,
        invoker: SharedString,
    ) -> CommandPresenter {
        if !Rc::ptr_eq(&self.factory, &factory) || self.invoker != invoker {
            self.presenter.close();
            self.presenter = factory(&invoker);
            self.factory = factory;
            self.invoker = invoker;
        }
        self.presenter.clone()
    }
}
impl Drop for PresentationGuard {
    fn drop(&mut self) {
        self.presenter.close();
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandRowKind {
    Item,
    Heading,
    Separator,
}
#[derive(Clone, Debug)]
pub struct CommandRowPresentation {
    pub id: SharedString,
    pub kind: CommandRowKind,
    pub index: Option<IndexPath>,
    pub label: SharedString,
    pub checked: bool,
    pub disabled: bool,
    pub shortcut: Option<SharedString>,
    pub icon_svg: Option<std::sync::Arc<[u8]>>,
    pub bounds: Bounds<Pixels>,
}
#[derive(Clone)]
pub struct CommandPresentation {
    pub entity: WeakEntity<CommandState>,
    pub query_input: Entity<InputState>,
    pub revision: u64,
    pub query: SharedString,
    pub searchable: bool,
    pub placeholder: SharedString,
    pub loading: bool,
    pub selected: Option<IndexPath>,
    pub bounds: Bounds<Pixels>,
    pub query_bounds: Bounds<Pixels>,
    pub list_bounds: Bounds<Pixels>,
    /// The actual native root customization, border gate, and shared row radius.
    pub bordered: bool,
    pub style: gpui::StyleRefinement,
    pub item_radius: Pixels,
    pub rows: Vec<CommandRowPresentation>,
    pub header_actions: Vec<crate::command::CommandHeaderAction>,
    complete: bool,
    delegated: bool,
}
impl CommandPresentation {
    /// Revalidate against the actual retained widget and release its read
    /// before invoking the exact Button listener, which may reenter it.
    pub fn deliver_header_action(
        &self,
        id: &str,
        event: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<(), CommandInputRefusal> {
        let owner = self
            .entity
            .upgrade()
            .ok_or(CommandInputRefusal::TargetAbsent)?;
        let action = {
            let state = owner.read(cx);
            if state.presentation.revision != self.revision {
                return Err(CommandInputRefusal::StaleRevision);
            }
            if !state.is_platform_delegated() {
                return Err(CommandInputRefusal::NotPresented);
            }
            let action = state
                .presentation
                .header_actions
                .iter()
                .find(|action| action.id.as_ref() == id)
                .ok_or(CommandInputRefusal::TargetAbsent)?;
            if action.disabled {
                return Err(CommandInputRefusal::Disabled);
            }
            action.clone()
        };
        (action.handler)(event, window, cx);
        Ok(())
    }
    pub fn is_complete(&self) -> bool {
        self.complete
    }
    pub fn is_platform_delegated(&self) -> bool {
        self.delegated
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandInput {
    Select(IndexPath),
    Activate(IndexPath),
    Previous,
    Next,
    Confirm,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandInputRefusal {
    StaleRevision,
    NotPresented,
    TargetAbsent,
    Disabled,
}
#[derive(Default)]
pub(super) struct PresentationState {
    pub(super) presenter: Option<CommandPresenter>,
    pub(super) revision: u64,
    pub(super) bounds: Bounds<Pixels>,
    pub(super) query_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub(super) list_bounds: Bounds<Pixels>,
    pub(super) rows: Rc<RefCell<BTreeMap<usize, Bounds<Pixels>>>>,
    pub(super) header_actions: Vec<crate::command::CommandHeaderAction>,
    pub(super) header_complete: bool,
}
impl CommandState {
    pub(crate) fn install_platform_presenter(
        &mut self,
        presenter: Option<CommandPresenter>,
        cx: &mut Context<Self>,
    ) {
        let before = self.is_platform_delegated();
        if let Some(previous) = &self.presentation.presenter {
            if presenter
                .as_ref()
                .is_none_or(|next| !Rc::ptr_eq(&previous.delegated, &next.delegated))
            {
                previous.close();
            }
        }
        self.presentation.presenter = presenter;
        if before != self.is_platform_delegated() {
            cx.notify();
        }
    }
    /// The same InputState that native search uses; a platform editing island
    /// attaches this owner, rather than keeping a second query text store.
    pub fn query_input(&self) -> Entity<InputState> {
        self.query_input.clone()
    }
    pub fn is_platform_delegated(&self) -> bool {
        self.presentation
            .presenter
            .as_ref()
            .is_some_and(CommandPresenter::is_platform_delegated)
    }
    pub(super) fn command_presentation(
        &self,
        entity: WeakEntity<Self>,
        window: &Window,
        cx: &App,
    ) -> CommandPresentation {
        let identity = entity.entity_id().as_u64();
        // An opaque header/footer/item is not silently reconstructed from text.
        // Structured source descriptors are required before it can delegate.
        let mut complete = self.options.header.is_none()
            && self.presentation.header_complete
            && self.options.footer.is_none()
            && self.options.empty.is_none();
        let measured = self.presentation.rows.borrow();
        let rows = measured
            .iter()
            .filter_map(|(row_ix, bounds)| {
                let (kind, index, label, checked, disabled, shortcut, icon) =
                    match self.rows.get(*row_ix)? {
                        CommandRow::Separator => (
                            CommandRowKind::Separator,
                            None,
                            "".into(),
                            false,
                            true,
                            None,
                            None,
                        ),
                        CommandRow::Heading(label) => (
                            CommandRowKind::Heading,
                            None,
                            label.clone(),
                            false,
                            true,
                            None,
                            None,
                        ),
                        CommandRow::Item(matched_ix) => {
                            let item = self.item_at(*matched_ix)?;
                            if item.content.is_some() || item.label_text().is_none() {
                                complete = false;
                            }
                            let shortcut = self
                                .item_binding(*matched_ix, window, cx)
                                .map(|key| Kbd::format(key.keystroke()).into());
                            let icon =
                                item.icon.as_ref().and_then(|icon| match icon.source_ref() {
                                    crate::icon::IconSource::Data(bytes) => Some(bytes.clone()),
                                    crate::icon::IconSource::Path(path) => cx
                                        .asset_source()
                                        .load(path.as_ref())
                                        .ok()
                                        .flatten()
                                        .map(|bytes| std::sync::Arc::from(bytes.as_ref())),
                                });
                            (
                                CommandRowKind::Item,
                                Some(self.matched[*matched_ix].index_path),
                                item.label_text().cloned().unwrap_or_default(),
                                item.checked,
                                item.is_disabled(),
                                shortcut,
                                icon,
                            )
                        }
                    };
                Some(CommandRowPresentation {
                    id: format!("command-{identity}-row-{row_ix}").into(),
                    kind,
                    index,
                    label,
                    checked,
                    disabled,
                    shortcut,
                    icon_svg: icon,
                    bounds: *bounds,
                })
            })
            .collect();
        CommandPresentation {
            entity,
            query_input: self.query_input(),
            revision: self.presentation.revision,
            query: self.query(cx),
            searchable: self.model.searchable,
            placeholder: self.applied_placeholder.clone(),
            loading: self.loading,
            selected: self.selected_index(),
            bounds: self.presentation.bounds,
            query_bounds: self.presentation.query_bounds.get(),
            list_bounds: self.presentation.list_bounds,
            bordered: self.options.bordered,
            style: self.options.style.clone(),
            item_radius: self.item_radius(window, cx),
            rows,
            header_actions: self
                .presentation
                .header_actions
                .iter()
                .map(|action| action.snapshot())
                .collect(),
            complete,
            delegated: self.is_platform_delegated(),
        }
    }
    pub fn presented_input(
        &mut self,
        revision: u64,
        input: CommandInput,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), CommandInputRefusal> {
        if revision != self.presentation.revision {
            return Err(CommandInputRefusal::StaleRevision);
        }
        if !self.presentation.presenter.as_ref().is_some_and(|p| {
            !p.retired.get() && (p.is_platform_delegated() || input == CommandInput::Cancel)
        }) {
            return Err(CommandInputRefusal::NotPresented);
        }
        match input {
            CommandInput::Select(index) | CommandInput::Activate(index) => {
                let matched = self
                    .matched
                    .iter()
                    .position(|matched| matched.index_path == index)
                    .ok_or(CommandInputRefusal::TargetAbsent)?;
                if self.matched[matched].disabled {
                    return Err(CommandInputRefusal::Disabled);
                }
                self.select(matched, window, cx);
                if matches!(input, CommandInput::Activate(_)) {
                    self.confirm(matched, window, cx);
                }
            }
            CommandInput::Previous => self.on_action_select_up(&SelectUp, window, cx),
            CommandInput::Next => self.on_action_select_down(&SelectDown, window, cx),
            CommandInput::Confirm => {
                self.on_action_confirm(&Confirm { secondary: false }, window, cx)
            }
            CommandInput::Cancel => self.on_action_cancel(&Cancel, window, cx),
        }
        Ok(())
    }
    pub(super) fn publish_platform(
        &mut self,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.presentation.bounds = bounds;
        let Some(presenter) = self.presentation.presenter.clone() else {
            return;
        };
        presenter.identity.set(Some(cx.entity_id().as_u64()));
        let frame = self.command_presentation(cx.entity().downgrade(), window, cx);
        let complete = frame.is_complete();
        let requested =
            (presenter.frame)(frame, window, cx) && complete && !presenter.retired.get();
        if presenter.delegated.replace(requested) != requested {
            let owner = cx.weak_entity();
            window.on_next_frame(move |_, cx| {
                let Some(owner) = owner.upgrade() else {
                    return;
                };
                let current =
                    owner
                        .read(cx)
                        .presentation
                        .presenter
                        .as_ref()
                        .is_some_and(|current| {
                            Rc::ptr_eq(&current.delegated, &presenter.delegated)
                                && !current.retired.get()
                                && current.delegated.get() == requested
                        });
                if current {
                    owner.update(cx, |_, cx| cx.notify());
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Disableable as _,
        command::{Command, CommandItem},
    };
    use gpui::{
        AppContext as _, ParentElement as _, Render, Styled as _, div, prelude::FluentBuilder as _,
        px,
    };
    struct Fixture {
        state: Entity<CommandState>,
        factory: CommandPresenterFactory,
        confirmed: Rc<Cell<u32>>,
        show: bool,
        changed: bool,
        opaque_header: bool,
    }
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl gpui::IntoElement {
            let count = self.confirmed.clone();
            let command = Command::new(&self.state)
                .item(CommandItem::new().label(if self.changed {
                    "Changed owner row"
                } else {
                    "Owner row"
                }))
                .item(CommandItem::new().label("Unavailable row").disabled(true))
                .on_confirm(move |_, _, _| count.set(count.get() + 1))
                .presenter("fixture-invoker", self.factory.clone());
            let command = if self.opaque_header {
                command.header(|_, _, _| div().child("Opaque header"))
            } else {
                command
            };
            div()
                .w(px(360.))
                .h(px(300.))
                .when(self.show, |this| this.child(command))
        }
    }
    #[gpui::test]
    fn platform_command_uses_actual_matching_refusals_and_retained_widget_lifetime(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::init);
        let frames: Rc<RefCell<Vec<CommandPresentation>>> = Rc::default();
        let closed = Rc::new(Cell::new(0));
        let confirmed = Rc::new(Cell::new(0));
        let factory: CommandPresenterFactory = Rc::new({
            let frames = frames.clone();
            let closed = closed.clone();
            move |_| {
                CommandPresenter::new(
                    {
                        let frames = frames.clone();
                        move |frame, _, _| {
                            frames.borrow_mut().push(frame);
                            true
                        }
                    },
                    {
                        let closed = closed.clone();
                        move |_| closed.set(closed.get() + 1)
                    },
                )
            }
        });
        let (view, cx) = cx.add_window_view(move |window, cx| Fixture {
            state: cx.new(|cx| CommandState::new(window, cx)),
            factory,
            confirmed,
            show: true,
            changed: false,
            opaque_header: false,
        });
        cx.run_until_parked();
        let frame = frames
            .borrow()
            .last()
            .cloned()
            .expect("actual command prepaint publishes");
        assert!(frame.is_complete());
        assert!(
            frame
                .rows
                .iter()
                .any(|row| row.label.as_ref() == "Owner row"
                    && row.bounds.size.height > gpui::px(0.))
        );
        let revision = frame.revision;
        let state = frame.entity.upgrade().unwrap();
        state.update_in(cx, |state, window, cx| {
            assert_eq!(
                state.presented_input(
                    revision,
                    CommandInput::Activate(IndexPath::new(1)),
                    window,
                    cx
                ),
                Err(CommandInputRefusal::Disabled)
            );
            state
                .presented_input(
                    revision,
                    CommandInput::Activate(IndexPath::new(0)),
                    window,
                    cx,
                )
                .unwrap();
        });
        cx.run_until_parked();
        assert_eq!(view.read_with(cx, |view, _| view.confirmed.get()), 1);
        view.update(cx, |view, cx| {
            view.changed = true;
            cx.notify();
        });
        cx.run_until_parked();
        state.update_in(cx, |state, window, cx| {
            assert_eq!(
                state.presented_input(
                    revision,
                    CommandInput::Activate(IndexPath::new(0)),
                    window,
                    cx
                ),
                Err(CommandInputRefusal::StaleRevision)
            );
        });
        view.update(cx, |view, cx| {
            view.opaque_header = true;
            cx.notify();
        });
        cx.run_until_parked();
        assert!(!frames.borrow().last().unwrap().is_complete());
        assert!(!state.read_with(cx, |state, _| state.is_platform_delegated()));
        view.update(cx, |view, cx| {
            view.show = false;
            cx.notify();
        });
        cx.run_until_parked();
        // Complete the next native frame: element states retire at frame
        // publication, rather than merely when the view reducer changes.
        cx.update(|window, cx| {
            window.simulate_next_frame(cx);
            window.refresh();
        });
        cx.run_until_parked();
        assert_eq!(
            closed.get(),
            1,
            "retiring the rendered widget closes while query owner still exists"
        );
        state.update_in(cx, |state, window, cx| {
            assert_eq!(
                state.presented_input(
                    state.presentation.revision,
                    CommandInput::Confirm,
                    window,
                    cx
                ),
                Err(CommandInputRefusal::NotPresented)
            );
        });
    }
}
