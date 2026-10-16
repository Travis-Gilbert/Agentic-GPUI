//! A lease over the actual Root ActiveDialog and its native dispatch node.
use super::dialog::Dialog;
use crate::ActiveTheme as _;
use crate::root::WindowState;
use gpui::{
    App, Bounds, FocusHandle, Focusable as _, Pixels, Refineable as _, SharedString,
    StyleRefinement, Styled as _, TextStyle, Window,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
    sync::atomic::{AtomicU64, Ordering},
};

pub type DialogPresenterFactory = Rc<dyn Fn(&str) -> DialogPresenter>;
type RestoreFocus = Rc<dyn Fn(u64, &str, &mut Window, &mut App) -> bool>;
#[derive(Clone)]
pub struct DialogPresenter {
    frame: Rc<dyn Fn(DialogPresentation, &mut Window, &mut App) -> bool>,
    closed: Rc<dyn Fn(u64)>,
    delegated: Rc<Cell<bool>>,
    restore_focus: Option<RestoreFocus>,
}
impl DialogPresenter {
    pub fn new(
        frame: impl Fn(DialogPresentation, &mut Window, &mut App) -> bool + 'static,
        closed: impl Fn(u64) + 'static,
    ) -> Self {
        Self {
            frame: Rc::new(frame),
            closed: Rc::new(closed),
            delegated: Rc::default(),
            restore_focus: None,
        }
    }
    /// Called by the same Root close/focus-restore path, outside its entity
    /// lease. Returning true handles the platform focus decision.
    pub fn on_restore_focus(
        mut self,
        restore: impl Fn(u64, &str, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.restore_focus = Some(Rc::new(restore));
        self
    }
    pub fn is_platform_delegated(&self) -> bool {
        self.delegated.get()
    }
}
pub(crate) struct DialogFocusRestore {
    pub(crate) previous: Option<FocusHandle>,
    platform: Option<(u64, SharedString, RestoreFocus)>,
}
impl DialogFocusRestore {
    pub(crate) fn new(
        owner: &Rc<RefCell<DialogPresentationOwner>>,
        previous: Option<gpui::WeakFocusHandle>,
    ) -> Self {
        let state = owner.borrow();
        let platform = state
            .presenter
            .as_ref()
            .filter(|presenter| presenter.delegated.get())
            .and_then(|presenter| presenter.restore_focus.clone())
            .map(|restore| (state.id, state.invoker.clone(), restore));
        Self {
            previous: previous.and_then(|handle| handle.upgrade()),
            platform,
        }
    }
    pub(crate) fn has_target(&self) -> bool {
        self.previous.is_some() || self.platform.is_some()
    }
    pub(crate) fn apply(self, window: &mut Window, cx: &mut App) {
        if let Some((id, invoker, restore)) = self.platform {
            if restore(id, &invoker, window, cx) {
                return;
            }
        }
        if let Some(previous) = self.previous {
            window.focus(&previous, cx);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DialogButtonPresentation {
    pub label: SharedString,
    pub danger: bool,
    pub variant: crate::button::ButtonVariant,
    pub bounds: Bounds<Pixels>,
}
#[derive(Clone)]
pub struct DialogIconPresentation {
    pub svg: std::sync::Arc<[u8]>,
    pub style: StyleRefinement,
}
/// A real single-line input inside a Root-owned Dialog. The frame and
/// material are observed from that same native Input renderer.
#[derive(Clone)]
pub struct DialogFieldPresentation {
    pub id: SharedString,
    pub label: SharedString,
    pub input: gpui::Entity<crate::input::InputState>,
    pub bounds: Bounds<Pixels>,
    pub material: StyleRefinement,
}
#[derive(Clone)]
pub(crate) struct DialogField {
    pub id: SharedString,
    pub label: SharedString,
    pub input: gpui::Entity<crate::input::InputState>,
    pub native: Rc<RefCell<Option<crate::input::InputFramePresentation>>>,
}
#[derive(Clone)]
pub struct DialogPresentation {
    owner: Weak<RefCell<DialogPresentationOwner>>,
    pub id: u64,
    pub revision: u64,
    pub invoker: SharedString,
    pub previous_focus: Option<gpui::WeakFocusHandle>,
    pub layer: usize,
    pub title: SharedString,
    pub description: SharedString,
    pub bounds: Bounds<Pixels>,
    pub title_bounds: Bounds<Pixels>,
    pub description_bounds: Bounds<Pixels>,
    pub confirm: DialogButtonPresentation,
    pub cancel: Option<DialogButtonPresentation>,
    pub opacity: f32,
    pub shadows: Vec<gpui::BoxShadow>,
    pub material: StyleRefinement,
    pub title_style: TextStyle,
    pub description_style: TextStyle,
    pub overlay_color: gpui::Hsla,
    pub keyboard: bool,
    pub overlay_closable: bool,
    pub close_bounds: Option<Bounds<Pixels>>,
    pub close_icon: Option<DialogIconPresentation>,
    pub fields: Vec<DialogFieldPresentation>,
    pub alert: bool,
    focus: FocusHandle,
    complete: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogInput {
    Confirm,
    Cancel,
    Escape,
    Outside,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogInputRefusal {
    TargetAbsent,
    StaleRevision,
    NotPresented,
    NotAllowed,
}
impl DialogPresentation {
    pub fn is_complete(&self) -> bool {
        self.complete
    }
    pub fn is_platform_delegated(&self) -> bool {
        self.owner.upgrade().is_some_and(|owner| {
            owner
                .borrow()
                .presenter
                .as_ref()
                .is_some_and(DialogPresenter::is_platform_delegated)
        })
    }
    fn validate_current(&self, window: &Window, cx: &App) -> Result<(), DialogInputRefusal> {
        let owner = self
            .owner
            .upgrade()
            .ok_or(DialogInputRefusal::TargetAbsent)?;
        if window.has_active_prompt()
            || !WindowState::try_read(window, cx).is_some_and(|root| {
                root.active_dialogs
                    .last()
                    .is_some_and(|dialog| Rc::ptr_eq(&dialog.presentation, &owner))
            })
        {
            return Err(DialogInputRefusal::NotPresented);
        }
        {
            let current = owner.borrow();
            if self.revision != current.revision {
                return Err(DialogInputRefusal::StaleRevision);
            }
            if !current
                .presenter
                .as_ref()
                .is_some_and(DialogPresenter::is_platform_delegated)
                || !self.focus.contains(&self.focus, window)
            {
                return Err(DialogInputRefusal::NotPresented);
            }
        }
        Ok(())
    }
    /// Focus only a field belonging to this actual topmost, current dialog.
    /// The Root and presentation leases are released before changing focus.
    pub fn focus_field(
        &self,
        id: &str,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<(), DialogInputRefusal> {
        self.validate_current(window, cx)?;
        let field = self
            .fields
            .iter()
            .find(|field| field.id.as_ref() == id)
            .ok_or(DialogInputRefusal::TargetAbsent)?;
        let focus = field.input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        Ok(())
    }
    pub fn deliver(
        &self,
        input: DialogInput,
        window: &mut Window,
        cx: &mut App,
    ) -> Result<(), DialogInputRefusal> {
        self.validate_current(window, cx)?;
        {
            if matches!(input, DialogInput::Outside) && !self.overlay_closable
                || matches!(input, DialogInput::Escape) && !self.keyboard
            {
                return Err(DialogInputRefusal::NotAllowed);
            }
            if input == DialogInput::Cancel && self.cancel.is_none() && self.close_bounds.is_none()
            {
                return Err(DialogInputRefusal::TargetAbsent);
            }
        }
        // This public FocusHandle path dispatches synchronously on the exact
        // rendered native dialog. Unlike Window::dispatch_action, it does not
        // defer beyond the caller's invocation provenance scope.
        match input {
            DialogInput::Confirm => self.focus.dispatch_action(
                &gpui_base::actions::Confirm { secondary: false },
                window,
                cx,
            ),
            DialogInput::Cancel | DialogInput::Escape | DialogInput::Outside => self
                .focus
                .dispatch_action(&gpui_base::actions::Cancel, window, cx),
        }
        Ok(())
    }
}
#[derive(Clone, Default, PartialEq)]
pub(crate) struct DialogText {
    pub title: Option<SharedString>,
    pub description: Option<SharedString>,
    pub alert: bool,
    pub complete: bool,
}
#[derive(Clone, PartialEq)]
struct Fingerprint {
    content: DialogText,
    confirm: SharedString,
    cancel: Option<SharedString>,
    danger: bool,
    keyboard: bool,
    overlay_closable: bool,
    field: Option<(SharedString, SharedString, u64)>,
    standard_footer: bool,
}
pub(crate) struct DialogPresentationOwner {
    id: u64,
    revision: u64,
    fingerprint: Option<Fingerprint>,
    factory: Option<DialogPresenterFactory>,
    invoker: SharedString,
    presenter: Option<DialogPresenter>,
    pub opacity: Cell<f32>,
    pub bounds: Cell<Bounds<Pixels>>,
    pub title_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub description_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub confirm_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub cancel_bounds: Rc<Cell<Bounds<Pixels>>>,
    pub close_bounds: Rc<Cell<Bounds<Pixels>>>,
    field: Option<DialogField>,
}
impl Default for DialogPresentationOwner {
    fn default() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT.fetch_add(1, Ordering::Relaxed),
            revision: 0,
            fingerprint: None,
            factory: None,
            invoker: SharedString::default(),
            presenter: None,
            opacity: Cell::new(1.),
            bounds: Cell::default(),
            title_bounds: Rc::default(),
            description_bounds: Rc::default(),
            confirm_bounds: Rc::default(),
            cancel_bounds: Rc::default(),
            close_bounds: Rc::default(),
            field: None,
        }
    }
}
impl Drop for DialogPresentationOwner {
    fn drop(&mut self) {
        if let Some(presenter) = self.presenter.take() {
            presenter.delegated.set(false);
            (presenter.closed)(self.id);
        }
    }
}
impl DialogPresentationOwner {
    pub(crate) fn install(&mut self, requested: Option<(SharedString, DialogPresenterFactory)>) {
        let same = requested.as_ref().is_some_and(|(invoker, factory)| {
            self.invoker == *invoker
                && self
                    .factory
                    .as_ref()
                    .is_some_and(|old| Rc::ptr_eq(old, factory))
        });
        if same {
            return;
        }
        if let Some(previous) = self.presenter.take() {
            previous.delegated.set(false);
            (previous.closed)(self.id);
        }
        self.factory = None;
        if let Some((invoker, factory)) = requested {
            self.presenter = Some(factory(&invoker));
            self.invoker = invoker;
            self.factory = Some(factory);
        }
    }
    pub(crate) fn delegated(&self) -> bool {
        self.presenter
            .as_ref()
            .is_some_and(DialogPresenter::is_platform_delegated)
    }
    pub(crate) fn prepare(
        owner: &Rc<RefCell<Self>>,
        dialog: &Dialog,
        window: &Window,
        cx: &mut App,
    ) -> Option<DialogPresentation> {
        let mut state = owner.borrow_mut();
        state.install(dialog.platform_presenter.clone());
        let confirm = dialog.button_props.ok_label();
        let cancel = dialog
            .button_props
            .is_cancel_shown()
            .then(|| dialog.button_props.cancel_label());
        let danger = matches!(
            dialog.button_props.ok_variant,
            Some(crate::button::ButtonVariant::Danger)
        );
        let fingerprint = Fingerprint {
            content: dialog.platform_text.clone(),
            confirm: confirm.clone(),
            cancel: cancel.clone(),
            danger,
            keyboard: dialog.props.keyboard,
            overlay_closable: dialog.props.overlay_closable,
            field: dialog.platform_field.as_ref().map(|field| {
                (
                    field.id.clone(),
                    field.label.clone(),
                    field.input.entity_id().as_u64(),
                )
            }),
            standard_footer: dialog.standard_footer,
        };
        if state.fingerprint.as_ref() != Some(&fingerprint) {
            state.revision = state.revision.saturating_add(1);
            state.fingerprint = Some(fingerprint);
        }
        state.presenter.as_ref()?;
        state.field = dialog.platform_field.clone();
        let field = dialog.platform_field.as_ref().and_then(|field| {
            field
                .native
                .borrow()
                .clone()
                .map(|native| DialogFieldPresentation {
                    id: field.id.clone(),
                    label: field.label.clone(),
                    input: field.input.clone(),
                    bounds: native.bounds,
                    material: native.material,
                })
        });
        let form_complete = dialog.platform_field.is_some()
            && dialog.standard_footer
            && dialog.platform_text.title.is_some()
            && dialog.children.len() == 1
            && dialog.content_builder.is_none()
            && dialog.header.is_none();
        let complete = (dialog.platform_text.complete
            && dialog.platform_text.alert
            && !dialog.props.overlay_closable)
            || form_complete;
        let mut native_material = gpui::div()
            .bg(cx.theme().tokens.background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius_lg);
        native_material.style().refine(&dialog.style);
        let material = native_material.style().clone();
        let mut inherited = window.text_style();
        inherited.refine(&material.text);
        let mut title_style = inherited.clone();
        title_style.refine(&super::DialogTitle::presentation_style().text);
        let mut description_style = inherited;
        description_style.refine(&super::DialogDescription::presentation_style(cx).text);
        let close_icon = if dialog.props.close_button {
            use crate::{IconNamed as _, Sizable as _};
            let color = crate::button::ButtonVariant::Ghost
                .presentation_styles(false, cx)
                .normal
                .foreground;
            let mut svg = crate::Icon::new(crate::IconName::Close).small().into_svg(
                window.text_style().font_size.to_pixels(window.rem_size()),
                color,
            );
            cx.asset_source()
                .load(crate::IconName::Close.path().as_ref())
                .ok()
                .flatten()
                .map(|source| DialogIconPresentation {
                    svg: std::sync::Arc::from(source.as_ref()),
                    style: svg.style().clone(),
                })
        } else {
            None
        };
        Some(DialogPresentation {
            owner: Rc::downgrade(owner),
            id: state.id,
            revision: state.revision,
            invoker: state.invoker.clone(),
            previous_focus: dialog.platform_previous_focus.clone(),
            layer: dialog.layer_ix,
            title: dialog.platform_text.title.clone().unwrap_or_default(),
            description: dialog.platform_text.description.clone().unwrap_or_default(),
            bounds: state.bounds.get(),
            title_bounds: state.title_bounds.get(),
            description_bounds: state.description_bounds.get(),
            confirm: DialogButtonPresentation {
                label: confirm,
                danger,
                variant: dialog
                    .button_props
                    .ok_variant
                    .unwrap_or(crate::button::ButtonVariant::Primary),
                bounds: state.confirm_bounds.get(),
            },
            cancel: cancel.map(|label| DialogButtonPresentation {
                label,
                danger: false,
                variant: dialog.button_props.cancel_variant.unwrap_or_default(),
                bounds: state.cancel_bounds.get(),
            }),
            opacity: state.opacity.get(),
            shadows: super::dialog::dialog_shadow(state.opacity.get()),
            material,
            title_style,
            description_style,
            overlay_color: super::dialog::overlay_color(dialog.props.overlay_visible, cx),
            keyboard: dialog.props.keyboard,
            overlay_closable: dialog.props.overlay_closable,
            close_bounds: dialog.props.close_button.then(|| state.close_bounds.get()),
            close_icon,
            fields: field.into_iter().collect(),
            alert: dialog.platform_text.alert,
            focus: dialog.focus_handle.clone(),
            complete,
        })
    }
    pub(crate) fn publish(
        owner: &Rc<RefCell<Self>>,
        mut frame: DialogPresentation,
        window: &mut Window,
        cx: &mut App,
    ) {
        let presenter = {
            let state = owner.borrow();
            frame.bounds = state.bounds.get();
            frame.opacity = state.opacity.get();
            frame.shadows = super::dialog::dialog_shadow(frame.opacity);
            frame.title_bounds = state.title_bounds.get();
            frame.description_bounds = state.description_bounds.get();
            if frame.close_bounds.is_some() {
                frame.close_bounds = Some(state.close_bounds.get());
            }
            if let Some(field) = &state.field {
                frame.fields = field
                    .native
                    .borrow()
                    .clone()
                    .map(|native| DialogFieldPresentation {
                        id: field.id.clone(),
                        label: field.label.clone(),
                        input: field.input.clone(),
                        bounds: native.bounds,
                        material: native.material,
                    })
                    .into_iter()
                    .collect();
                frame.complete &= frame.fields.iter().any(|field| {
                    field.bounds.size.width > gpui::px(0.)
                        && field.bounds.size.height > gpui::px(0.)
                });
            }

            frame.confirm.bounds = state.confirm_bounds.get();
            if let Some(cancel) = &mut frame.cancel {
                cancel.bounds = state.cancel_bounds.get();
            }
            state.presenter.clone()
        };
        if let Some(presenter) = presenter {
            let complete = frame.complete
                && frame.bounds.size.width > gpui::px(0.)
                && frame.confirm.bounds.size.width > gpui::px(0.);
            let requested = (presenter.frame)(frame, window, cx) && complete;
            if presenter.delegated.replace(requested) != requested {
                // The native Root must repaint its changed policy outside prepaint.
                let owner = Rc::downgrade(owner);
                window.on_next_frame(move |window, cx| {
                    let Some(owner) = owner.upgrade() else {
                        return;
                    };
                    let current = owner.borrow().presenter.as_ref().is_some_and(|current| {
                        Rc::ptr_eq(&current.delegated, &presenter.delegated)
                            && current.delegated.get() == requested
                    });
                    let mounted = WindowState::try_read(window, cx).is_some_and(|root| {
                        root.active_dialogs
                            .iter()
                            .any(|dialog| Rc::ptr_eq(&dialog.presentation, &owner))
                    });
                    if current && mounted {
                        let _ = WindowState::try_update(window, cx, |_, _, cx| cx.notify());
                    }
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WindowExt as _, dialog::DialogButtonProps};
    use gpui::{AppContext as _, TestAppContext, px, size};
    fn draw(cx: &mut gpui::VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    #[gpui::test]
    fn platform_alert_dispatches_existing_refusals_and_rechecks_actual_top_owner(
        cx: &mut TestAppContext,
    ) {
        let cx = super::super::dialog::tests::window(cx, size(px(600.), px(500.)));
        let frames: Rc<RefCell<Vec<DialogPresentation>>> = Rc::default();
        let admit = Rc::new(Cell::new(false));
        let attempts = Rc::new(Cell::new(0));
        let allow = Rc::new(Cell::new(false));
        let closed = Rc::new(Cell::new(0));
        let factory: DialogPresenterFactory = Rc::new({
            let frames = frames.clone();
            let admit = admit.clone();
            move |_| {
                DialogPresenter::new(
                    {
                        let frames = frames.clone();
                        let admit = admit.clone();
                        move |frame, _, _| {
                            frames.borrow_mut().push(frame);
                            admit.get()
                        }
                    },
                    |_| {},
                )
            }
        });
        cx.update(|window, cx| {
            window.open_alert_dialog(cx, {
                let attempts = attempts.clone();
                let allow = allow.clone();
                let closed = closed.clone();
                move |alert, _, _| {
                    alert
                        .title_text("Existing confirmation")
                        .description_text("The retained owner decides.")
                        .button_props(
                            DialogButtonProps::default()
                                .show_cancel(true)
                                .ok_text("Delete"),
                        )
                        .presenter("chat.more", factory.clone())
                        .on_cancel({
                            let attempts = attempts.clone();
                            let allow = allow.clone();
                            move |_, _, _| {
                                attempts.set(attempts.get() + 1);
                                allow.get()
                            }
                        })
                        .on_close({
                            let closed = closed.clone();
                            move |_, _, _| closed.set(closed.get() + 1)
                        })
                }
            })
        });
        draw(cx);
        draw(cx);
        let frame = frames
            .borrow()
            .last()
            .cloned()
            .expect("actual Root dialog must publish");
        assert!(frame.is_complete());
        assert!(frame.bounds.size.width > px(0.));
        assert!(frame.title_bounds.size.height > px(0.));
        assert!(frame.description_bounds.size.height > px(0.));
        assert!(frame.confirm.bounds.size.width > px(0.));
        assert!(frame.cancel.as_ref().unwrap().bounds.size.width > px(0.));
        assert_eq!(
            cx.update(|window, cx| frame.deliver(DialogInput::Cancel, window, cx)),
            Err(DialogInputRefusal::NotPresented)
        );
        admit.set(true);
        cx.update(|window, _| window.refresh());
        draw(cx);
        draw(cx);
        let frame = frames.borrow().last().unwrap().clone();
        assert!(frame.is_platform_delegated());
        assert_eq!(
            cx.update(|window, cx| frame.deliver(DialogInput::Outside, window, cx)),
            Err(DialogInputRefusal::NotAllowed)
        );
        assert_eq!(attempts.get(), 0);
        cx.update(|window, cx| frame.deliver(DialogInput::Cancel, window, cx).unwrap());
        assert_eq!(attempts.get(), 1);
        assert_eq!(closed.get(), 0);
        assert!(
            cx.update(|window, cx| window.has_active_dialog(cx)),
            "native refusal keeps its actual owner"
        );
        cx.update(|window, cx| {
            window.open_alert_dialog(cx, |alert, _, _| {
                alert
                    .title("Native second dialog")
                    .description("This unpresented owner is topmost.")
            })
        });
        draw(cx);
        assert_eq!(
            cx.update(|window, cx| frame.deliver(DialogInput::Confirm, window, cx)),
            Err(DialogInputRefusal::NotPresented)
        );
        cx.update(|window, cx| window.close_dialog(cx));
        draw(cx);
        allow.set(true);
        let current = frames.borrow().last().unwrap().clone();
        cx.update(|window, cx| current.deliver(DialogInput::Cancel, window, cx).unwrap());
        assert_eq!(attempts.get(), 2);
        assert_eq!(closed.get(), 1);
        assert!(!cx.update(|window, cx| window.has_active_dialog(cx)));
        assert_eq!(
            cx.update(|window, cx| current.deliver(DialogInput::Confirm, window, cx)),
            Err(DialogInputRefusal::TargetAbsent)
        );
    }
    #[gpui::test]
    fn platform_form_retains_the_actual_input_and_native_submit_refusals(cx: &mut TestAppContext) {
        use gpui::IntoElement as _;
        let cx = super::super::dialog::tests::window(cx, size(px(600.), px(500.)));
        let frames: Rc<RefCell<Vec<DialogPresentation>>> = Rc::default();
        let submitted: Rc<RefCell<Vec<String>>> = Rc::default();
        let allow = Rc::new(Cell::new(false));
        let factory: DialogPresenterFactory = Rc::new({
            let frames = frames.clone();
            move |_| {
                DialogPresenter::new(
                    {
                        let frames = frames.clone();
                        move |frame, _, _| {
                            frames.borrow_mut().push(frame);
                            true
                        }
                    },
                    |_| {},
                )
            }
        });
        let input = cx.update(|window, cx| {
            cx.new(|cx| crate::input::InputState::new(window, cx).default_value("Original"))
        });
        cx.update(|window, cx| {
            let input = input.clone();
            let submitted = submitted.clone();
            let allow = allow.clone();
            window.open_dialog(cx, move |dialog, _, _| {
                let confirmed = input.clone();
                let submitted = submitted.clone();
                let allow = allow.clone();
                dialog
                    .title_text("Rename conversation")
                    .text_field("chat.rename.title", "Title", input.clone(), |native| {
                        native.into_any_element()
                    })
                    .standard_footer()
                    .button_props(
                        DialogButtonProps::default()
                            .ok_text("Rename")
                            .show_cancel(true),
                    )
                    .presenter("chat.more", factory.clone())
                    .on_ok(move |_, _, cx| {
                        submitted
                            .borrow_mut()
                            .push(confirmed.read(cx).value().to_string());
                        allow.get()
                    })
            });
        });
        draw(cx);
        draw(cx);
        let frame = frames.borrow().last().unwrap().clone();
        assert!(frame.is_complete());
        assert!(frame.is_platform_delegated());
        assert!(!frame.alert);
        assert_eq!(frame.fields.len(), 1);
        let field = &frame.fields[0];
        assert_eq!(field.id.as_ref(), "chat.rename.title");
        assert_eq!(field.input.entity_id(), input.entity_id());
        assert!(field.bounds.size.width > px(0.));
        assert!(field.bounds.size.height > px(0.));
        assert!(
            field.material.background.is_some(),
            "material comes from the real Input frame"
        );
        assert!(frame.confirm.bounds.size.width > px(0.));
        assert!(frame.cancel.as_ref().unwrap().bounds.size.width > px(0.));
        cx.update(|window, cx| frame.focus_field("chat.rename.title", window, cx).unwrap());
        cx.simulate_keystrokes("cmd-a");
        cx.simulate_input("Revised");
        assert_eq!(
            cx.update(|_, cx| input.read(cx).value().to_string()),
            "Revised"
        );
        cx.simulate_keystrokes("cmd-z");
        assert_ne!(
            cx.update(|_, cx| input.read(cx).value().to_string()),
            "Revised",
            "the actual input's undo changes its canonical value"
        );
        cx.simulate_keystrokes("cmd-shift-z");
        assert_eq!(
            cx.update(|_, cx| input.read(cx).value().to_string()),
            "Revised"
        );
        draw(cx);
        let current = frames.borrow().last().unwrap().clone();
        cx.update(|window, cx| current.deliver(DialogInput::Confirm, window, cx).unwrap());
        assert_eq!(submitted.borrow().as_slice(), &["Revised"]);
        assert!(
            cx.update(|window, cx| window.has_active_dialog(cx)),
            "original callback refuses close"
        );
        allow.set(true);
        cx.update(|window, cx| current.deliver(DialogInput::Confirm, window, cx).unwrap());
        assert_eq!(submitted.borrow().as_slice(), &["Revised", "Revised"]);
        assert!(!cx.update(|window, cx| window.has_active_dialog(cx)));
        assert_eq!(
            cx.update(|window, cx| current.deliver(DialogInput::Confirm, window, cx)),
            Err(DialogInputRefusal::TargetAbsent)
        );
        assert_eq!(
            cx.update(|window, cx| current.focus_field("chat.rename.title", window, cx)),
            Err(DialogInputRefusal::TargetAbsent)
        );
    }

    #[gpui::test]
    fn platform_focus_restore_runs_after_the_actual_root_releases_its_lease(
        cx: &mut TestAppContext,
    ) {
        let cx = super::super::dialog::tests::window(cx, size(px(600.), px(500.)));
        let restores = Rc::new(Cell::new(0));
        let frames: Rc<RefCell<Vec<DialogPresentation>>> = Rc::default();
        let factory: DialogPresenterFactory = Rc::new({
            let restores = restores.clone();
            let frames = frames.clone();
            move |_| {
                DialogPresenter::new(
                    {
                        let frames = frames.clone();
                        move |frame, _, _| {
                            frames.borrow_mut().push(frame);
                            true
                        }
                    },
                    |_| {},
                )
                .on_restore_focus({
                    let restores = restores.clone();
                    move |_, invoker, window, cx| {
                        assert_eq!(invoker, "actual-invoker");
                        restores.set(restores.get() + 1);
                        // Reentering the same Root from its platform focus callback
                        // must be safe; a borrowed Root lease would panic here.
                        window.open_alert_dialog(cx, |alert, _, _| {
                            alert.title("Replacement native owner")
                        });
                        true
                    }
                })
            }
        });
        cx.update(|window, cx| {
            window.open_alert_dialog(cx, move |alert, _, _| {
                alert
                    .title_text("Current owner")
                    .description_text("Actual close path")
                    .presenter("actual-invoker", factory.clone())
            })
        });
        draw(cx);
        draw(cx);
        assert!(frames.borrow().last().unwrap().is_platform_delegated());
        cx.update(|window, cx| window.close_dialog(cx));
        assert_eq!(restores.get(), 1);
        assert!(
            cx.update(|window, cx| window.has_active_dialog(cx)),
            "the actual callback reopened the actual Root"
        );
        cx.update(|window, cx| window.close_dialog(cx));
        assert_eq!(
            restores.get(),
            1,
            "closing an unrelated native owner cannot reuse the retired platform focus callback"
        );
    }

    #[gpui::test]
    fn opaque_alert_content_never_yields_native_modal_authority(cx: &mut TestAppContext) {
        let cx = super::super::dialog::tests::window(cx, size(px(600.), px(500.)));
        let frames: Rc<RefCell<Vec<DialogPresentation>>> = Rc::default();
        let factory: DialogPresenterFactory = Rc::new({
            let frames = frames.clone();
            move |_| {
                DialogPresenter::new(
                    {
                        let frames = frames.clone();
                        move |frame, _, _| {
                            frames.borrow_mut().push(frame);
                            true
                        }
                    },
                    |_| {},
                )
            }
        });
        cx.update(|window, cx| {
            window.open_alert_dialog(cx, move |alert, _, _| {
                alert
                    .title("Opaque title element")
                    .description_text("No inferred descriptor is permitted.")
                    .presenter("chat.more", factory.clone())
            })
        });
        draw(cx);
        draw(cx);
        let frame = frames.borrow().last().unwrap().clone();
        assert!(!frame.is_complete());
        assert!(!frame.is_platform_delegated());
        assert!(cx.update(|window, cx| window.has_active_dialog(cx)));
    }
}
