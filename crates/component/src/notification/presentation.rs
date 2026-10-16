//! A platform lease over a Notification; its list still owns every timer.
use super::*;
use gpui::{Bounds, Pixels};
use std::cell::Cell;

pub type NotificationPresenterFactory = Rc<dyn Fn() -> NotificationPresenter>;
#[derive(Clone)]
pub struct NotificationPresenter {
    pub(super) frame: Rc<dyn Fn(NotificationPresentation, &mut Window, &mut App) -> bool>,
    closed: Rc<dyn Fn(u64)>,
    pub(super) delegated: Rc<Cell<bool>>,
    identity: Rc<Cell<Option<u64>>>,
}
impl NotificationPresenter {
    pub fn new(
        frame: impl Fn(NotificationPresentation, &mut Window, &mut App) -> bool + 'static,
        closed: impl Fn(u64) + 'static,
    ) -> Self {
        Self {
            frame: Rc::new(frame),
            closed: Rc::new(closed),
            delegated: Rc::default(),
            identity: Rc::default(),
        }
    }
    pub fn is_platform_delegated(&self) -> bool {
        self.delegated.get()
    }
    fn close(&self) {
        self.delegated.set(false);
        if let Some(id) = self.identity.take() {
            (self.closed)(id);
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct NotificationActionPresentation {
    pub label: SharedString,
    pub description: Option<SharedString>,
    pub disabled: bool,
    pub bounds: Bounds<Pixels>,
}
#[derive(Clone)]
pub struct NotificationPresentation {
    pub entity: WeakEntity<Notification>,
    pub revision: u64,
    pub bounds: Bounds<Pixels>,
    pub dismiss_bounds: Bounds<Pixels>,
    pub copy_bounds: Bounds<Pixels>,
    pub title: Option<SharedString>,
    pub message: Option<SharedString>,
    pub icon_svg: Option<std::sync::Arc<[u8]>>,
    pub type_: Option<NotificationType>,
    pub phase: ToastTransitionStatus,
    pub opacity: f32,
    /// The same current animation helper used by the native Toast renderer.
    pub shadows: Vec<gpui::BoxShadow>,
    pub visible: bool,
    pub action: Option<NotificationActionPresentation>,
    pub clickable: bool,
    complete: bool,
    delegated: bool,
}
impl NotificationPresentation {
    pub fn is_complete(&self) -> bool {
        self.complete
    }
    pub fn is_platform_delegated(&self) -> bool {
        self.delegated
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationInput {
    Action,
    Click,
    Dismiss,
    Hover(bool),
    Focus(bool),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationInputRefusal {
    StaleRevision,
    NotPresented,
    TargetAbsent,
    Disabled,
}
#[derive(Default)]
pub(super) struct PresentationState {
    pub(super) presenter: Option<NotificationPresenter>,
    pub(super) bounds: Bounds<Pixels>,
    pub(super) action_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub(super) dismiss_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub(super) copy_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub(super) revision: u64,
    pub(super) icon: Option<std::sync::Arc<[u8]>>,
    pub(super) stack: gpui_base::ToastItemPresentation,
    pub(super) opacity: Rc<Cell<f32>>,
    fingerprint: Option<Fingerprint>,
    action: Option<NotificationActionPresentation>,
    action_handler: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    action_complete: bool,
    pub(super) hovered: bool,
    pub(super) focused: bool,
}
#[derive(PartialEq)]
struct Fingerprint {
    title: Option<SharedString>,
    message: Option<SharedString>,
    action: Option<(SharedString, Option<SharedString>, bool)>,
    clickable: bool,
    custom_content: bool,
    kind: Option<String>,
    icon: Option<std::sync::Arc<[u8]>>,
}
impl Notification {
    pub(super) fn capture_action(&mut self, action: Option<&Button>) {
        self.presentation.action_complete = action.is_none();
        self.presentation.action = None;
        self.presentation.action_handler = None;
        if let Some((label, description, disabled, handler)) =
            action.and_then(Button::notification_action)
        {
            self.presentation.action = Some(NotificationActionPresentation {
                label,
                description,
                disabled,
                bounds: self.presentation.action_bounds.get(),
            });
            self.presentation.action_handler = Some(handler);
            self.presentation.action_complete = true;
        }
    }
    pub(super) fn refresh_presentation_revision(&mut self) {
        let fingerprint = Fingerprint {
            title: self.title.clone(),
            message: self.message.clone(),
            action: self.presentation.action.as_ref().map(|action| {
                (
                    action.label.clone(),
                    action.description.clone(),
                    action.disabled,
                )
            }),
            clickable: self.on_click.is_some(),
            custom_content: self.content_builder.is_some(),
            kind: self.type_.map(|kind| format!("{kind:?}")),
            icon: self.presentation.icon.clone(),
        };
        if self.presentation.fingerprint.as_ref() != Some(&fingerprint) {
            self.presentation.revision = self.presentation.revision.saturating_add(1);
            self.presentation.fingerprint = Some(fingerprint);
        }
    }
    pub fn presentation(&self, entity: WeakEntity<Self>) -> NotificationPresentation {
        if let Some(presenter) = &self.presentation.presenter {
            presenter.identity.set(Some(entity.entity_id().as_u64()));
        }
        NotificationPresentation {
            entity,
            revision: self.presentation.revision,
            bounds: self.presentation.bounds,
            dismiss_bounds: self.presentation.dismiss_bounds.get(),
            copy_bounds: self.presentation.copy_bounds.get(),
            title: self.title.clone(),
            message: self.message.clone(),
            icon_svg: self.presentation.icon.clone(),
            type_: self.type_,
            phase: self.transition_status,
            opacity: self.presentation.opacity.get() * self.presentation.stack.opacity(),
            shadows: crate::styled::toast_shadow(self.presentation.opacity.get().powi(3)),
            visible: self.presentation.stack.is_visible(),
            action: self.presentation.action.clone().map(|mut action| {
                action.bounds = self.presentation.action_bounds.get();
                action
            }),
            clickable: self.on_click.is_some(),
            complete: self.content_builder.is_none() && self.presentation.action_complete,
            delegated: self
                .presentation
                .presenter
                .as_ref()
                .is_some_and(NotificationPresenter::is_platform_delegated),
        }
    }
    /// Deliver an existing native listener after releasing the notification lease.
    pub fn deliver_presented_input(
        owner: &Entity<Self>,
        revision: u64,
        input: NotificationInput,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<(), NotificationInputRefusal> {
        let handler = owner.update(cx, |note, cx| {
            note.prepare_presented_input(revision, input, window, cx)
        })?;
        if let Some(handler) = handler {
            handler(&ClickEvent::default(), window, cx);
        }
        Ok(())
    }
    fn prepare_presented_input(
        &mut self,
        revision: u64,
        input: NotificationInput,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>, NotificationInputRefusal>
    {
        if revision != self.presentation.revision {
            return Err(NotificationInputRefusal::StaleRevision);
        }
        if !self
            .presentation
            .presenter
            .as_ref()
            .is_some_and(NotificationPresenter::is_platform_delegated)
        {
            return Err(NotificationInputRefusal::NotPresented);
        }
        if self.transition_status == ToastTransitionStatus::Ending {
            return Err(NotificationInputRefusal::Disabled);
        }
        let handler = match input {
            NotificationInput::Action => {
                let action = self
                    .presentation
                    .action
                    .as_ref()
                    .ok_or(NotificationInputRefusal::TargetAbsent)?;
                if action.disabled {
                    return Err(NotificationInputRefusal::Disabled);
                }
                let handler = self
                    .presentation
                    .action_handler
                    .clone()
                    .ok_or(NotificationInputRefusal::TargetAbsent)?;
                Some(handler)
            }
            NotificationInput::Click => {
                let handler = self
                    .on_click
                    .clone()
                    .ok_or(NotificationInputRefusal::TargetAbsent)?;
                self.dismiss(window, cx);
                Some(handler)
            }
            NotificationInput::Dismiss => {
                self.dismiss(window, cx);
                None
            }
            NotificationInput::Hover(active) => {
                self.presentation.hovered = active;
                None
            }
            NotificationInput::Focus(active) => {
                self.presentation.focused = active;
                None
            }
        };
        cx.notify();
        Ok(handler)
    }
}
impl Drop for Notification {
    fn drop(&mut self) {
        if let Some(presenter) = &self.presentation.presenter {
            presenter.close();
        }
    }
}
impl NotificationList {
    pub fn visible_notifications(&self, cx: &App) -> Vec<Entity<Notification>> {
        self.grouped(cx)
            .into_iter()
            .flat_map(|(_, _, items)| {
                items
                    .into_iter()
                    .map(|(_, note)| note)
                    .filter(|note| note.read(cx).presentation.stack.is_visible())
            })
            .collect()
    }
    pub fn set_presenter(
        &mut self,
        presenter: Option<NotificationPresenterFactory>,
        cx: &mut Context<Self>,
    ) {
        for (_, note, _) in self.notifications.iter() {
            let next = presenter.as_ref().map(|factory| factory());
            note.update(cx, |note, cx| {
                if let Some(old) = note.presentation.presenter.take() {
                    old.close();
                }
                note.presentation.presenter = next;
                note.presentation.hovered = false;
                note.presentation.focused = false;
                cx.notify();
            });
        }
        self.presenter = presenter;
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Disableable as _;
    #[gpui::test]
    fn notification_receiver_rechecks_revision_and_the_actual_button_before_calling_it(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| cx.set_global(crate::Theme::default()));
        let calls = Rc::new(Cell::new(0));
        let (note, cx) = cx.add_window_view(|_, _| Notification::info("Saved"));
        let presenter = NotificationPresenter::new(|_, _, _| true, |_| {});
        let revision = note.update(cx, |note, _| {
            let button = Button::new("existing-action").label("View").on_click({
                let calls = calls.clone();
                move |_, _, _| calls.set(calls.get() + 1)
            });
            note.capture_action(Some(&button));
            note.refresh_presentation_revision();
            note.presentation.presenter = Some(presenter.clone());
            note.presentation.revision
        });
        note.update_in(cx, |note, window, cx| {
            assert_eq!(
                note.prepare_presented_input(revision, NotificationInput::Action, window, cx)
                    .map(|_| ()),
                Err(NotificationInputRefusal::NotPresented)
            );
            presenter.delegated.set(true);
            assert_eq!(
                note.prepare_presented_input(revision + 1, NotificationInput::Action, window, cx)
                    .map(|_| ()),
                Err(NotificationInputRefusal::StaleRevision)
            );
            assert_eq!(calls.get(), 0);
            let handler = note
                .prepare_presented_input(revision, NotificationInput::Action, window, cx)
                .unwrap()
                .unwrap();
            handler(&ClickEvent::default(), window, cx);
            assert_eq!(calls.get(), 1);
            let disabled = Button::new("existing-action")
                .label("View")
                .disabled(true)
                .on_click({
                    let calls = calls.clone();
                    move |_, _, _| calls.set(calls.get() + 1)
                });
            note.capture_action(Some(&disabled));
            note.refresh_presentation_revision();
            assert_eq!(
                note.prepare_presented_input(revision, NotificationInput::Action, window, cx)
                    .map(|_| ()),
                Err(NotificationInputRefusal::StaleRevision)
            );
            assert_eq!(
                note.prepare_presented_input(
                    note.presentation.revision,
                    NotificationInput::Action,
                    window,
                    cx
                )
                .map(|_| ()),
                Err(NotificationInputRefusal::Disabled)
            );
            assert_eq!(calls.get(), 1);
        });
    }
    #[gpui::test]
    fn platform_notification_action_releases_owner_lease_before_existing_context_listener(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| cx.set_global(crate::Theme::default()));
        let (note, cx) = cx.add_window_view(|_, _| Notification::info("Before owner action"));
        let presenter = NotificationPresenter::new(|_, _, _| true, |_| {});
        presenter.delegated.set(true);
        let revision = note.update(cx, |note, cx| {
            let action = Button::new("existing-owner-action")
                .label("Update owner")
                .on_click(cx.listener(|note, _, _, cx| {
                    note.message = Some("After owner action".into());
                    cx.notify();
                }));
            note.capture_action(Some(&action));
            note.refresh_presentation_revision();
            note.presentation.presenter = Some(presenter);
            note.presentation.revision
        });
        cx.update(|window, cx| {
            Notification::deliver_presented_input(
                &note,
                revision,
                NotificationInput::Action,
                window,
                cx,
            )
            .unwrap()
        });
        assert_eq!(
            note.read_with(cx, |note, _| note.message.clone())
                .as_deref(),
            Some("After owner action")
        );
    }
    struct ListHarness {
        list: Entity<NotificationList>,
    }
    impl Render for ListHarness {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(self.list.clone())
        }
    }
    #[gpui::test]
    fn platform_hover_pauses_and_resumes_the_existing_notification_clock(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| cx.set_global(crate::Theme::default()));
        let (root, cx) = cx.add_window_view(|window, cx| ListHarness {
            list: cx.new(|cx| NotificationList::new(window, cx)),
        });
        let list = root.read_with(cx, |root, _| root.list.clone());
        list.update_in(cx, |list, window, cx| {
            list.push(Notification::info("Pause this real toast"), window, cx)
        });
        cx.background_executor
            .advance_clock(NOTIFICATION_TRANSITION_DURATION + NOTIFICATION_ADVANCE_INTERVAL);
        cx.run_until_parked();
        let note = list.read_with(cx, |list, _| list.notifications()[0].clone());
        let presenter = NotificationPresenter::new(|_, _, _| true, |_| {});
        presenter.delegated.set(true);
        note.update_in(cx, |note, window, cx| {
            note.presentation.presenter = Some(presenter);
            note.prepare_presented_input(
                note.presentation.revision,
                NotificationInput::Hover(true),
                window,
                cx,
            )
            .unwrap();
        });
        cx.background_executor.advance_clock(Duration::from_secs(6));
        cx.run_until_parked();
        assert_eq!(
            note.read_with(cx, |note, _| note.transition_status),
            ToastTransitionStatus::Present
        );
        note.update_in(cx, |note, window, cx| {
            note.prepare_presented_input(
                note.presentation.revision,
                NotificationInput::Hover(false),
                window,
                cx,
            )
            .unwrap();
        });
        cx.background_executor.advance_clock(Duration::from_secs(6));
        cx.run_until_parked();
        assert_eq!(
            note.read_with(cx, |note, _| note.transition_status),
            ToastTransitionStatus::Ending
        );
    }
}
