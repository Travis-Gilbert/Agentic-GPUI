pub use gpui_base::input::{
    Diagnostic, DiagnosticEntry, DiagnosticRelatedInformation, DiagnosticSet, DiagnosticSeverity,
    DiagnosticSummary, DiagnosticTag, RelatedInformation,
};

mod diagnostic_styles;
pub(crate) use diagnostic_styles::*;

// Every `tree-sitter` gate below also asks the target, not just the feature.
//
// The grammars are native C, so the optional `tree-sitter` dependencies sit
// under `[target.'cfg(not(target_family = "wasm"))'.dependencies]` and never
// activate for a wasm build. Cargo resolves features without consulting the
// target, so a consumer that turns the feature on for its native build turns
// it on for its wasm build too, and the gate would then admit code whose
// `tree_sitter` crate is not there. The feature says what the consumer wants;
// the target says what is possible. Both have to agree, and on wasm the
// `wasm_stub` path below is the answer.

#[cfg(all(feature = "tree-sitter", not(target_family = "wasm")))]
mod input_adapter;
#[cfg(all(feature = "tree-sitter", not(target_family = "wasm")))]
pub(crate) use input_adapter::input_highlighter_factory;

#[cfg(not(all(feature = "tree-sitter", not(target_family = "wasm"))))]
pub(crate) fn input_highlighter_factory() -> gpui_base::input::InputHighlighterFactory {
    std::rc::Rc::new(|_| None)
}

// Native implementation with full tree-sitter support
#[cfg(all(feature = "tree-sitter", not(target_family = "wasm")))]
mod highlighter;
#[cfg(all(feature = "tree-sitter", not(target_family = "wasm")))]
mod languages;
#[cfg(all(feature = "tree-sitter", not(target_family = "wasm")))]
mod registry;

#[cfg(all(feature = "tree-sitter", not(target_family = "wasm")))]
pub use highlighter::*;
#[cfg(all(feature = "tree-sitter", not(target_family = "wasm")))]
pub use languages::*;
#[cfg(all(feature = "tree-sitter", not(target_family = "wasm")))]
pub use registry::*;

// WASM stub implementation (no tree-sitter support or disabled)
#[cfg(not(all(feature = "tree-sitter", not(target_family = "wasm"))))]
mod wasm_stub;
#[cfg(not(all(feature = "tree-sitter", not(target_family = "wasm"))))]
pub use wasm_stub::*;
