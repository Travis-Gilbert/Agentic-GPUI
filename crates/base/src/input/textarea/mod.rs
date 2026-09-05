use gpui::{App, Context, Entity, IntoElement, RenderOnce, Window};

use super::{InputBaseState, InputExtras, InputModeKind, TextDecoration, TextareaMode};

/// State for editing ordinary multi-line text.
///
/// This is the shared editing engine in its multi-line kind. Code-editor
/// facilities such as languages, diagnostics, folding, and LSP do not exist on
/// this type — those methods live on [`super::EditorState`].
pub type TextareaState = InputBaseState<TextareaMode>;

/// An unstyled ordinary multi-line text input.
#[derive(IntoElement)]
pub struct Textarea {
    state: Entity<TextareaState>,
}

impl Textarea {
    pub fn new(state: &Entity<TextareaState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for Textarea {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        self.state
    }
}

/// Presentation-only ranges for ordinary multiline prose. Empty by default;
/// this does not add syntax parsing, diagnostics, folding, or an LSP.
#[derive(Default)]
pub struct TextareaExtras {
    decorations: Vec<TextDecoration>,
}

impl InputExtras for TextareaExtras {
    fn decoration_layers(&self) -> Vec<&[TextDecoration]> {
        vec![&self.decorations]
    }
}

impl InputModeKind for TextareaMode {
    const MULTI_LINE: bool = true;
    type Extras = TextareaExtras;

    fn reset_annotations(state: &mut InputBaseState<Self>) {
        state.extras.decorations.clear();
    }

    fn reset_document_presentation(state: &mut InputBaseState<Self>) {
        state.extras.decorations.clear();
    }

    fn adjust_annotations(
        state: &mut InputBaseState<Self>,
        range: &std::ops::Range<usize>,
        new_len: usize,
    ) {
        state.extras.decorations.retain_mut(|decoration| {
            decoration.range =
                super::decorations::adjust_range_for_edit(&decoration.range, range, new_len);
            !decoration.range.is_empty()
        });
    }
}

impl TextareaState {
    /// Replace presentation styles without replacing text, selection, or IME
    /// marked text. UTF-8 ranges use the same clipping and edit adjustment as
    /// editor decorations. Applications retain authority over the styles.
    pub fn set_text_decorations(
        &mut self,
        decorations: Vec<TextDecoration>,
        cx: &mut Context<Self>,
    ) {
        let decorations = super::decorations::normalize(&self.text, decorations);
        if self.extras.decorations != decorations {
            self.extras.decorations = decorations;
            cx.notify();
        }
    }

    /// The normalized presentation styles currently painted by this textarea.
    pub fn text_decorations(&self) -> &[TextDecoration] {
        &self.extras.decorations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{EntityInputHandler as _, HighlightStyle, TestAppContext};

    #[gpui::test]
    fn textarea_decorations_preserve_text_selection_and_composition(cx: &mut TestAppContext) {
        cx.update(crate::init);
        let window = cx.add_window(|window, cx| TextareaState::new(window, cx));
        window
            .update(cx, |state, window, cx| {
                assert!(state.text_decorations().is_empty());
                assert!(!state.is_code_editor());
                state.set_value("héllo world", window, cx);
                state.set_selected_range(1..4, cx);
                let value = state.value();
                let selection = state.selected_range();
                let bold = HighlightStyle {
                    font_weight: Some(gpui::FontWeight::BOLD),
                    ..Default::default()
                };
                state.set_text_decorations(vec![TextDecoration::new(2..4, bold)], cx);
                assert_eq!(state.value(), value);
                assert_eq!(state.selected_range(), selection);
                assert_eq!(state.text_decorations(), &[TextDecoration::new(1..4, bold)]);
                state.set_selected_range(0..0, cx);
                state.replace_and_mark_text_in_range(None, "界", Some(1..1), window, cx);
                let marked = state.marked_text_range(window, cx);
                let value = state.value();
                let selection = state.selected_range();
                assert!(marked.is_some());
                assert_eq!(state.text_decorations()[0].range, 4..7);
                state.set_text_decorations(vec![TextDecoration::new(4..7, bold)], cx);
                assert_eq!(state.value(), value);
                assert_eq!(state.selected_range(), selection);
                assert_eq!(state.marked_text_range(window, cx), marked);
                state.set_value("replacement", window, cx);
                assert!(state.text_decorations().is_empty());
            })
            .expect("textarea remains mounted");
    }
}
