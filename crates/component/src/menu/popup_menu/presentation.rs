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
    pub(super) delegated: Rc<Cell<bool>>,
}
impl MenuItemPresentation {
    pub fn new(id: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: None,
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
    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
}

#[derive(Clone)]
pub struct PopupMenuPresentation {
    entity: WeakEntity<PopupMenu>,
    revision: u64,
    bounds: Bounds<Pixels>,
    priority: usize,
    selected_index: Option<usize>,
    rows: Vec<MenuRowPresentation>,
    complete: bool,
    delegated: bool,
}
impl PopupMenuPresentation {
    pub fn entity(&self) -> WeakEntity<PopupMenu> {
        self.entity.clone()
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
    closed: Rc<dyn Fn()>,
    pub(super) delegated: Rc<Cell<bool>>,
    is_closed: Rc<Cell<bool>>,
}
impl PopupMenuPresenter {
    pub fn new(
        frame: impl Fn(PopupMenuPresentation, &mut Window, &mut App) -> bool + 'static,
        closed: impl Fn() + 'static,
    ) -> Self {
        Self {
            frame: Rc::new(frame),
            closed: Rc::new(closed),
            delegated: Rc::default(),
            is_closed: Rc::default(),
        }
    }
    pub(super) fn close(&self) {
        if !self.is_closed.replace(true) {
            self.delegated.set(false);
            (self.closed)();
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
    pub(super) presenter: Option<PopupMenuPresenter>,
    pub(super) revision: u64,
}

impl PopupMenu {
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
                    bounds: bounds.get(&index).copied().unwrap_or_default(),
                })
            })
            .collect();
        PopupMenuPresentation {
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
        if !self
            .presentation
            .presenter
            .as_ref()
            .is_some_and(|p| p.delegated.get() && !p.is_closed.get())
        {
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
    fn presented_menu_rejects_stale_and_disabled_rows_before_the_existing_handler(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| cx.set_global(crate::Theme::default()));
        let invoked = Rc::new(Cell::new(0));
        let closed = Rc::new(Cell::new(0));
        let presenter = PopupMenuPresenter::new(|_, _, _| true, {
            let closed = closed.clone();
            move || closed.set(closed.get() + 1)
        });
        // This owner-only unit test admits the presentation lease directly;
        // browser oracles separately prove the inert/acknowledged handoff.
        presenter.delegated.set(true);
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
