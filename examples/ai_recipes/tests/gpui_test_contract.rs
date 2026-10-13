// The Theorem Zed GPUI macros expand to bare `gpui::` paths; this one line per
// crate root resolves them through the Kit (gpui-kit #3306).
extern crate gpui_kit as gpui;

// Preserve the published #[gpui_kit::test] API.
// This package depends only on gpui-kit, without a direct GPUI dependency.
// Running the same contract here prevents Kit's own GPUI dependency from hiding
// a broken re-export or macro expansion in consumers.
#[path = "../../../crates/kit/tests/test_macro.rs"]
mod test_macro;

// The complete example included by docs/test must work with the same imports.
#[path = "../../../crates/kit/tests/ui.rs"]
mod ui;
