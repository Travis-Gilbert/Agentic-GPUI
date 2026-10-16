//! A presentation lease over the retained menu, never a second menu controller.
use super::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

/// Explicit semantics for a custom menu element. Clones share only the
/// read-only canvas-delegation fact written by the actual menu renderer.
#[derive(Clone)]
pub struct MenuItemPresentation {
    id: SharedString,
    label: SharedString,
    description: Option<SharedString>,
    checkable: bool,
    pub(super) delegated: Rc<Cell<bool>>,
}
impl MenuItemPresentation {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
            checkable: false,
            delegated: Rc::default(),
        }
    }
    pub fn with_description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
    pub fn id(&self) -> &SharedString {
        &self.id
    }
    pub fn checkable(mut self, checkable: bool) -> Self {
        self.checkable = checkable;
        self
    }
    pub fn label(&self) -> &SharedString {
        &self.label
    }
    pub fn description(&self) -> Option<&SharedString> {
        self.description.as_ref()
    }
    pub fn is_platform_delegated(&self) -> bool {
        self.delegated.get()
    }
}

/// An ordinary owner item with explicit presenter metadata. The public
/// `PopupMenuItem` variants and the existing `item` builder stay compatible.
pub struct PresentedMenuItem {
    item: PopupMenuItem,
    presentation: MenuItemPresentation,
}
impl PopupMenuItem {
    pub fn with_presentation(self, presentation: MenuItemPresentation) -> PresentedMenuItem {
        PresentedMenuItem {
            item: self,
            presentation,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuRowKind {
    Item,
    Label,
    Separator,
    Submenu,
}

#[derive(Clone)]
pub struct MenuRowPresentation {
    index: usize,
    id: SharedString,
    label: SharedString,
    description: Option<SharedString>,
    kind: MenuRowKind,
    disabled: bool,
    checked: bool,
    checkable: bool,
    icon_svg: Option<std::sync::Arc<[u8]>>,
    shortcut: Option<SharedString>,
    bounds: Bounds<Pixels>,
}
impl MenuRowPresentation {
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn id(&self) -> &SharedString {
        &self.id
    }
    pub fn label(&self) -> &SharedString {
        &self.label
    }
    pub fn description(&self) -> Option<&SharedString> {
        self.description.as_ref()
    }
    pub fn kind(&self) -> MenuRowKind {
        self.kind
    }
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }
    pub fn is_checked(&self) -> bool {
        self.checked
    }
    pub fn is_checkable(&self) -> bool {
        self.checkable
    }
    pub fn icon_svg(&self) -> Option<&[u8]> {
        self.icon_svg.as_deref()
    }
    pub fn shortcut(&self) -> Option<&SharedString> {
        self.shortcut.as_ref()
    }
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
}

#[derive(Clone)]
pub struct PopupMenuPresentation {
    focus: gpui::WeakFocusHandle,
    entity: WeakEntity<PopupMenu>,
    revision: u64,
    bounds: Bounds<Pixels>,
    priority: usize,
    selected_index: Option<usize>,
    rows: Vec<MenuRowPresentation>,
    complete: bool,
    delegated: bool,
    parent: Option<WeakEntity<PopupMenu>>,
    active_submenu: Option<WeakEntity<PopupMenu>>,
}
impl PopupMenuPresentation {
    /// The actual menu dispatch node, including after its owner is retired.
    pub fn focus(&self) -> gpui::WeakFocusHandle {
        self.focus.clone()
    }
    pub fn entity(&self) -> WeakEntity<PopupMenu> {
        self.entity.clone()
    }
    pub fn parent(&self) -> Option<WeakEntity<PopupMenu>> {
        self.parent.clone()
    }
    pub fn active_submenu(&self) -> Option<WeakEntity<PopupMenu>> {
        self.active_submenu.clone()
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
    pub fn priority(&self) -> usize {
        self.priority
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }
    pub fn rows(&self) -> &[MenuRowPresentation] {
        &self.rows
    }
    pub fn is_complete(&self) -> bool {
        self.complete
    }
    pub fn is_platform_delegated(&self) -> bool {
        self.delegated
    }
}

/// A host requests delegation by returning true after preparing its region.
/// The next owner frame acknowledges it before the host activates native input.
#[derive(Clone)]
pub struct PopupMenuPresenter {
    pub(super) frame: Rc<dyn Fn(PopupMenuPresentation, &mut Window, &mut App) -> bool>,
    closed: Rc<dyn Fn(u64)>,
    focus_closed: Option<Rc<dyn Fn(u64, bool)>>,
    pub(super) entity_id: Rc<Cell<Option<u64>>>,
    pub(super) delegated: Rc<Cell<bool>>,
    is_closed: Rc<Cell<bool>>,
}
impl PopupMenuPresenter {
    pub fn new(
        frame: impl Fn(PopupMenuPresentation, &mut Window, &mut App) -> bool + 'static,
        closed: impl Fn(u64) + 'static,
    ) -> Self {
        Self {
            frame: Rc::new(frame),
            closed: Rc::new(closed),
            focus_closed: None,
            entity_id: Rc::default(),
            delegated: Rc::default(),
            is_closed: Rc::default(),
        }
    }
    /// Observe the native owner's existing focus decision before its close
    /// notification. True means an item already moved native focus elsewhere.
    pub fn on_focus_close(mut self, observer: impl Fn(u64, bool) + 'static) -> Self {
        self.focus_closed = Some(Rc::new(observer));
        self
    }
    pub(super) fn close_with_focus(&self, moved: bool) {
        if !self.is_closed.get() {
            if let (Some(id), Some(observer)) = (self.entity_id.get(), &self.focus_closed) {
                observer(id, moved);
            }
        }
        self.close();
    }
    /// Whether this actual owner has acknowledged a prepared native region.
    pub fn is_platform_delegated(&self) -> bool {
        self.delegated.get() && !self.is_closed.get()
    }
    pub(super) fn close(&self) {
        if !self.is_closed.replace(true) {
            self.delegated.set(false);
            if let Some(id) = self.entity_id.get() {
                (self.closed)(id);
            }
        }
    }
    pub(super) fn for_submenu(&self) -> Self {
        Self {
            frame: self.frame.clone(),
            closed: self.closed.clone(),
            focus_closed: self.focus_closed.clone(),
            entity_id: Rc::default(),
            delegated: Rc::default(),
            is_closed: Rc::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PopupMenuInput {
    Select(usize),
    Activate(usize),
    Previous,
    Next,
    OpenSubmenu,
    Parent,
    Confirm,
    Dismiss,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuInputRefusal {
    StaleRevision,
    TargetAbsent,
    Disabled,
    NotPresented,
}

#[derive(Default)]
pub(super) struct PresentationState {
    pub(super) metadata: BTreeMap<usize, MenuItemPresentation>,
    pub(super) bounds: Rc<RefCell<BTreeMap<usize, Bounds<Pixels>>>>,
    pub(super) icons: RefCell<BTreeMap<usize, std::sync::Arc<[u8]>>>,
    pub(super) shortcuts: RefCell<BTreeMap<usize, SharedString>>,
    pub(super) presenter: Option<PopupMenuPresenter>,
    pub(super) revision: u64,
}

impl PopupMenu {
    /// The existing menu's acknowledged platform presentation fact.
    pub fn is_platform_delegated(&self) -> bool {
        self.presentation
            .presenter
            .as_ref()
            .is_some_and(PopupMenuPresenter::is_platform_delegated)
    }
    pub fn presented_item(mut self, item: PresentedMenuItem) -> Self {
        self.presentation
            .metadata
            .insert(self.menu_items.len(), item.presentation);
        self.item(item.item)
    }
    pub fn presenter(mut self, presenter: PopupMenuPresenter) -> Self {
        self.presentation.presenter = Some(presenter);
        self
    }
    pub fn presentation(&self, entity: WeakEntity<Self>) -> PopupMenuPresentation {
        let mut complete = true;
        let identity = entity.entity_id().as_u64();
        let bounds = self.presentation.bounds.borrow();
        let rows = self
            .menu_items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                if index + 1 == self.menu_items.len() && item.is_separator() {
                    return None;
                }
                let metadata = self.presentation.metadata.get(&index);
                let kind = match item {
                    PopupMenuItem::Separator => MenuRowKind::Separator,
                    PopupMenuItem::Label(_) => MenuRowKind::Label,
                    PopupMenuItem::Submenu { .. } => MenuRowKind::Submenu,
                    _ => MenuRowKind::Item,
                };
                if matches!(item, PopupMenuItem::ElementItem { .. }) && metadata.is_none() {
                    complete = false;
                }
                Some(MenuRowPresentation {
                    index,
                    id: metadata.map(|m| m.id.clone()).unwrap_or_else(|| {
                        format!("menu-{identity}-{}-{index}", self.presentation.revision).into()
                    }),
                    label: metadata
                        .map(|m| m.label.clone())
                        .or_else(|| item.a11y_label())
                        .unwrap_or_default(),
                    description: metadata.and_then(|m| m.description.clone()),
                    kind,
                    disabled: !item.is_clickable(),
                    checked: item.is_checked(),
                    checkable: item.is_checked() || metadata.is_some_and(|m| m.checkable),
                    icon_svg: self.presentation.icons.borrow().get(&index).cloned(),
                    shortcut: self.presentation.shortcuts.borrow().get(&index).cloned(),
                    bounds: bounds.get(&index).copied().unwrap_or_default(),
                })
            })
            .collect();
        PopupMenuPresentation {
            focus: self.focus_handle.downgrade(),
            entity,
            revision: self.presentation.revision,
            bounds: self.bounds,
            priority: self.priority,
            selected_index: self.selected_index,
            rows,
            complete,
            delegated: self
                .presentation
                .presenter
                .as_ref()
                .is_some_and(|p| p.delegated.get()),
            parent: self.parent_menu.clone(),
            active_submenu: self.active_submenu().map(|menu| menu.downgrade()),
        }
    }
    pub fn presented_input(
        &mut self,
        revision: u64,
        input: PopupMenuInput,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<(), MenuInputRefusal> {
        if revision != self.presentation.revision {
            return Err(MenuInputRefusal::StaleRevision);
        }
        if !self.presentation.presenter.as_ref().is_some_and(|p| {
            !p.is_closed.get() && (p.delegated.get() || input == PopupMenuInput::Dismiss)
        }) {
            return Err(MenuInputRefusal::NotPresented);
        }
        match input {
            PopupMenuInput::Select(index) | PopupMenuInput::Activate(index) => {
                let item = self
                    .menu_items
                    .get(index)
                    .ok_or(MenuInputRefusal::TargetAbsent)?;
                if !item.is_clickable() {
                    return Err(MenuInputRefusal::Disabled);
                }
                self.set_selected_index(index, cx);
                if matches!(input, PopupMenuInput::Activate(_)) {
                    if matches!(self.menu_items[index], PopupMenuItem::Submenu { .. }) {
                        self._select_submenu(window, cx);
                    } else {
                        self.on_click(index, window, cx);
                    }
                }
            }
            PopupMenuInput::Previous => self.select_up(&SelectUp, window, cx),
            PopupMenuInput::Next => self.select_down(&SelectDown, window, cx),
            PopupMenuInput::OpenSubmenu => self.select_right(&SelectRight, window, cx),
            PopupMenuInput::Parent => self.select_left(&SelectLeft, window, cx),
            PopupMenuInput::Confirm => {
                let index = self.selected_index.ok_or(MenuInputRefusal::TargetAbsent)?;
                if !self
                    .menu_items
                    .get(index)
                    .is_some_and(PopupMenuItem::is_clickable)
                {
                    return Err(MenuInputRefusal::Disabled);
                }
                self.confirm(&Confirm { secondary: false }, window, cx);
            }
            PopupMenuInput::Dismiss => self.dismiss(&Cancel, window, cx),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[gpui::test]
    fn external_ack_needs_a_settling_native_frame_to_publish_delegation(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(crate::init);
        let observed = Rc::new(Cell::new(false));
        let frames: Rc<RefCell<Vec<PopupMenuPresentation>>> = Rc::default();
        let (menu, cx) = cx.add_window_view({
            let observed = observed.clone();
            let frames = frames.clone();
            move |_, cx| {
                PopupMenu::new(cx)
                    .item(PopupMenuItem::new("Original action"))
                    .presenter(PopupMenuPresenter::new(
                        move |frame, _, _| {
                            frames.borrow_mut().push(frame);
                            observed.get()
                        },
                        |_| {},
                    ))
            }
        });
        cx.run_until_parked();
        assert!(frames.borrow().last().unwrap().is_complete());
        assert!(!frames.borrow().last().unwrap().is_platform_delegated());
        observed.set(true);
        let owner = menu.downgrade();
        cx.update(|window, _| {
            window.on_next_frame(move |window, cx| {
                let Some(owner) = owner.upgrade() else {
                    return;
                };
                owner.update(cx, |_, cx| cx.notify());
                window.refresh();
            });
        });
        cx.update(|window, cx| {
            assert_eq!(window.simulate_next_frame(cx), 1);
            window.draw(cx).clear(cx);
        });
        assert!(menu.read_with(cx, |menu, _| menu.is_platform_delegated()));
        assert!(
            !frames.borrow().last().unwrap().is_platform_delegated(),
            "the first acknowledgement callback precedes the renderer's delegation write"
        );
        cx.update(|window, cx| {
            assert_eq!(window.simulate_next_frame(cx), 1);
            window.draw(cx).clear(cx);
        });
        assert!(
            frames.borrow().last().unwrap().is_platform_delegated(),
            "the bounded settling frame publishes the actual renderer's accepted state"
        );
        cx.update(|window, cx| {
            assert_eq!(
                window.simulate_next_frame(cx),
                0,
                "acknowledgement does not create a perpetual frame source"
            );
        });
    }

    #[gpui::test]
    fn presented_menu_rejects_stale_and_disabled_rows_before_the_existing_handler(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| cx.set_global(crate::Theme::default()));
        let invoked = Rc::new(Cell::new(0));
        let closed = Rc::new(Cell::new(0));
        let presenter = PopupMenuPresenter::new(|_, _, _| true, {
            let closed = closed.clone();
            move |_| closed.set(closed.get() + 1)
        });
        // This owner-only unit test admits the presentation lease directly;
        // browser oracles separately prove the inert/acknowledged handoff.
        presenter.delegated.set(true);
        presenter.entity_id.set(Some(1));
        let metadata =
            MenuItemPresentation::new("existing-row", "Run").with_description("Existing command");
        let (menu, cx) = cx.add_window_view({
            let invoked = invoked.clone();
            let metadata = metadata.clone();
            move |_, cx| {
                PopupMenu::new(cx)
                    .label("Commands")
                    .item(PopupMenuItem::new("Unavailable").disabled(true))
                    .presented_item(
                        PopupMenuItem::element(|_, _| div().child("Run"))
                            .on_click(move |_, _, _| invoked.set(invoked.get() + 1))
                            .with_presentation(metadata),
                    )
                    .presenter(presenter)
            }
        });
        cx.update(|window, cx| {
            menu.update(cx, |menu, cx| {
                let snapshot = menu.presentation(cx.entity().downgrade());
                assert!(snapshot.is_complete());
                assert_eq!(snapshot.rows()[2].id().as_ref(), "existing-row");
                assert_eq!(
                    menu.presented_input(0, PopupMenuInput::Activate(1), window, cx),
                    Err(MenuInputRefusal::Disabled)
                );
                assert_eq!(
                    menu.presented_input(1, PopupMenuInput::Activate(2), window, cx),
                    Err(MenuInputRefusal::StaleRevision)
                );
                assert_eq!(invoked.get(), 0);
                menu.presented_input(0, PopupMenuInput::Next, window, cx)
                    .unwrap();
                assert_eq!(
                    menu.selected_index,
                    Some(2),
                    "navigation skips owner labels and disabled rows"
                );
                menu.rebuild(window, cx, {
                    let invoked = invoked.clone();
                    move |menu, _, _| {
                        menu.label("Updated")
                            .item(PopupMenuItem::new("Unavailable").disabled(true))
                            .item(
                                PopupMenuItem::new("Replacement")
                                    .on_click(move |_, _, _| invoked.set(invoked.get() + 1)),
                            )
                    }
                });
                assert_eq!(
                    menu.presented_input(0, PopupMenuInput::Activate(2), window, cx),
                    Err(MenuInputRefusal::StaleRevision)
                );
                assert_eq!(
                    invoked.get(),
                    0,
                    "old row index cannot execute a replacement command"
                );
                menu.presented_input(1, PopupMenuInput::Activate(2), window, cx)
                    .unwrap();
                assert_eq!(invoked.get(), 1, "the ordinary menu handler executes once");
                assert_eq!(closed.get(), 1);
                assert_eq!(
                    menu.presented_input(1, PopupMenuInput::Activate(2), window, cx),
                    Err(MenuInputRefusal::NotPresented)
                );
            })
        });
    }

    #[gpui::test]
    fn prepared_menu_is_inert_and_can_be_withdrawn(cx: &mut gpui::TestAppContext) {
        cx.update(|cx| cx.set_global(crate::Theme::default()));
        let invoked = Rc::new(Cell::new(0));
        let closed = Rc::new(Cell::new(0));
        let presenter = PopupMenuPresenter::new(|_, _, _| false, {
            let closed = closed.clone();
            move |_| closed.set(closed.get() + 1)
        });
        let (menu, cx) = cx.add_window_view({
            let invoked = invoked.clone();
            move |_, cx| {
                PopupMenu::new(cx)
                    .item(
                        PopupMenuItem::new("Run")
                            .on_click(move |_, _, _| invoked.set(invoked.get() + 1)),
                    )
                    .presenter(presenter)
            }
        });
        cx.update(|window, cx| {
            menu.update(cx, |menu, cx| {
                // This owner-only test has no paint callback: record the real
                // entity identity as render does before its first allocation.
                menu.presentation
                    .presenter
                    .as_ref()
                    .unwrap()
                    .entity_id
                    .set(Some(cx.entity_id().as_u64()));
                assert!(!menu.is_platform_delegated());
                assert_eq!(
                    menu.presented_input(0, PopupMenuInput::Activate(0), window, cx),
                    Err(MenuInputRefusal::NotPresented)
                );
                assert_eq!(invoked.get(), 0);
                menu.presented_input(0, PopupMenuInput::Dismiss, window, cx)
                    .unwrap();
                assert_eq!(
                    closed.get(),
                    1,
                    "preparing a region does not prevent owner withdrawal"
                );
                assert_eq!(
                    menu.presented_input(0, PopupMenuInput::Dismiss, window, cx),
                    Err(MenuInputRefusal::NotPresented)
                );
                assert_eq!(
                    menu.presented_input(0, PopupMenuInput::Activate(0), window, cx),
                    Err(MenuInputRefusal::NotPresented)
                );
                assert_eq!(invoked.get(), 0);
                assert_eq!(
                    closed.get(),
                    1,
                    "a closed lease cannot execute or close twice"
                );
            })
        });
    }

    #[gpui::test]
    fn custom_rows_require_explicit_owner_metadata_before_delegation(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            let menu = cx.new(|cx| {
                PopupMenu::new(cx).item(PopupMenuItem::element(|_, _| {
                    div().child("Opaque custom content")
                }))
            });
            assert!(!menu.read(cx).presentation(menu.downgrade()).is_complete());
            let metadata = MenuItemPresentation::new("custom-id", "Actual owner label");
            let described = cx.new(|cx| {
                PopupMenu::new(cx).presented_item(
                    PopupMenuItem::element(|_, _| div()).with_presentation(metadata),
                )
            });
            assert!(
                described
                    .read(cx)
                    .presentation(described.downgrade())
                    .is_complete()
            );
        });
    }
}

#[cfg(test)]
mod focus_restore_tests {
    use super::*;
    fn verify_native_close_decision(cx: &mut gpui::TestAppContext, moved: bool) {
        cx.update(|cx| cx.set_global(crate::Theme::default()));
        let reported = Rc::new(Cell::new(None));
        let presenter = PopupMenuPresenter::new(|_, _, _| true, |_| {}).on_focus_close({
            let reported = reported.clone();
            move |id, moved| reported.set(Some((id, moved)))
        });
        let menu_presenter = presenter.clone();
        let (menu, cx) =
            cx.add_window_view(move |_, cx| PopupMenu::new(cx).presenter(menu_presenter));
        let identity = menu.entity_id().as_u64();
        presenter.delegated.set(true);
        let other_focus = cx.update(|window, cx| {
            let focus = menu.read(cx).focus_handle.clone();
            focus.focus(window, cx);
            if moved {
                let other = cx.focus_handle();
                other.focus(window, cx);
                Some(other)
            } else {
                None
            }
        });
        menu.update_in(cx, |menu, window, cx| {
            menu.presented_input(
                menu.presentation.revision,
                PopupMenuInput::Dismiss,
                window,
                cx,
            )
            .unwrap();
        });
        assert_eq!(reported.get(), Some((identity, moved)));
        drop(other_focus);
    }
    #[gpui::test]
    fn escape_reports_the_existing_owner_focus_restore_decision(cx: &mut gpui::TestAppContext) {
        verify_native_close_decision(cx, false);
    }
    #[gpui::test]
    fn action_moved_focus_reports_the_existing_native_refusal_to_restore(
        cx: &mut gpui::TestAppContext,
    ) {
        verify_native_close_decision(cx, true);
    }
}
