# Theorem GPUI Kit patch series

This hard fork tracks `longbridge/gpui-kit` tag `v0.7.3`. The files listed in
`series` are applied in order and are never proposed upstream. The fork pins
the entire GPUI family to one immutable revision of `Travis-Gilbert/zed`
(`67eb6eab969e3ba41c7ddb047d9fcf16e83d8c48`) in place of upstream's
`gpui-pre =0.3.9` crates.

The replay workflow applies this series to `v0.7.3` for branch validation and
to upstream `main` on its monthly schedule. A failed scheduled run is the
signal to rebase the series. If upstream independently implements a patch, the
patch is removed during that rebase.

## The v0.7.3 rebase

The 44 branch commits were rebased from `v0.7.0` onto `v0.7.3` with
`git rebase --onto v0.7.3 v0.7.0`, and the series was regenerated with
`git format-patch v0.7.3..HEAD -- . ':(exclude)patches'
':(exclude).github/workflows/gpui-kit-patch-series.yml'` (the workflow is the
fork's own file, and upstream has none to patch). The 22 entries replayed onto
a clean `v0.7.3` export reproduce the branch tree byte for byte. No Theorem
patch was dropped: upstream contains none of them. Upstream cut v0.7.2 and v0.7.3 from
its `stable` branch and left out every change that breaks existing consumers
(#3432 motion easing, #3415 `TabVariant::Folder`, #3402 pointer helpers, #3384
`TestWindowExt`, #3302 `ResizeHandle::on_drag`, and the `gpui-wry` to
`gpui-webview` rename), so the rebase met additions, not API moves.

Conflicts the replay resolved, each keeping the Theorem intent on upstream's
new code:

- Accessible navigation labels (patch 0002) met the tooltip delay (#3376) and
  tab scrolling (#3419): a button keeps both the configurable tooltip delay
  and its own accessible name, and the tab bar keeps one import list.
- The Zed source patch (0005) met #3375's GPUI Fast workspace entries and the
  `IntoPlot` crate-path lookup, which now accepts `gpui-kit`, `gpui` or
  `gpui-fast`.
- Measured link fragments (0015) met #3371, #3373 and #3421: link targets keep
  the text view's selection authority through the same stack the clamped
  descendants use, and #3371's `link_underline` style default coexists with the
  fragment underline override (solid keeps the parsed underline; dotted and
  none replace it).
- The v0.7.0 build patch met #3306, which points the hidden `gpui_kit::gpui`
  at the Kit itself.

What v0.7.3 needs that the Theorem Zed source does not have, and how the
v0.7.3 build patch meets it:

- GPUI Fast (#3375) is not carried. `gpui-fast-linux` requires `oo7 ^0.6`,
  which cannot resolve beside the Zed fork's `oo7 =0.6.0-alpha`, and a second
  engine would be a second GPUI type identity. Its dependencies and features
  leave every manifest; its `cfg(feature = "gpui-fast")` sources stay
  byte-identical and inert, declared through check-cfg.
- `gpui::IndentAdjustment` does not exist, so `LineWrapper::wrap_line` takes
  two arguments again; the Zed default is the same-indent wrap upstream asks
  for.
- `TextLayout` has no separate background and glyph passes. `Inline` keeps
  #3439's order (backgrounds, then the selection, then the glyphs) by painting
  the run backgrounds itself and handing the shaped text its runs without
  them, so selected text stays readable over an opaque selection.
- `FrameTiming` has no refresh interval, signal time or phases, so the fps
  sampler's test frames omit them; criterion stays at 0.5, the version the
  Zed source's `bench-support` links.
- `#[derive(Action)]`, `actions!` and `#[gpui::test]` expand to bare `gpui::`
  paths. Since #3306 a crate that both depends on `gpui` and glob-imports
  `gpui_kit::*` is ambiguous (E0659), so the Kit-only consumers (the story,
  tig, the recipe tests and the exports test) use the one line #3306
  documents, `extern crate gpui_kit as gpui;`. The story crate no longer
  depends on `gpui` directly, which supersedes the v0.7.0 note below.

The Kit integration tests are held to the fork's contracts (patch 0022): the
fork moves, selects and deletes by grapheme cluster (patch 0017), so the
textarea and editing tests that #3256 wrote for code-point steps expect one
step per cluster; and the Zed test window starts its pointer at the origin,
so the table sort test hovers the header before it looks for the hover-only
sort icon.

Verification on the rebased branch (rustc 1.97.1 stable and 1.96.1, the
Theorem repository pin): `cargo check --locked --workspace --all-targets`;
`cargo check --locked -p gpui-base -p gpui-kit-assets -p gpui-component
--target wasm32-unknown-unknown`; `cargo test --locked -p gpui-base -p
gpui-component --lib` 1328 and 666 passed; `cargo test --locked -p gpui-kit
--features "test-support component assets"` 350 integration tests and 2
doctests passed, including the 16 Diff tests, and its Metal suite (`--test
rendering`) passed 11 of 11, among them `selected_text_stays_over_an_opaque_selection`;
`cargo test --locked -p gpui-kit-recipes` 6 passed (the tests on stable).
`--all-features` stops only
at `quickjs-jit-core`, the Shell's JIT engine, which needs nightly here as it
did on v0.7.0. The workflow's Rust 1.90 replay has not run: GitHub Actions is
disabled on this account.

## The v0.7.0 rebase

The series was regenerated from the branch with
`git format-patch v0.7.0..HEAD -- . ':(exclude)patches'`. Replaying it onto a
clean `v0.7.0` export reproduces the branch tree byte for byte. The v0.6.0
series had 29 entries, most of them manifest and lock pin moves; on v0.7.0
those fold into patch 0005 (the Zed source) and patch 0018 (the regenerated
lock), leaving 18.

What v0.7.0 changed underneath the series:

- `Root` moved into `gpui_base::Root` with presentation plugins. The fork's
  runtime keymap now lands on Base's root (`secondary-c`), and the component
  crate only registers its `WindowState` plugin.
- Settings filtering became index-based (`SettingsFilter`). Labelled sections
  and controlled navigation (patch 0004) are ported onto it: sections hold
  their pages, `take_pages` flattens them, and each section's sidebar group
  lists global page indices, so filtering never renumbers source data.
- Text `Inline` retains its shaped layout across frames and lost its element
  id. Measured link fragments (patch 0015) give an `Inline` an id only when a
  fragment decorator is set, because the fragments' focus handles live in its
  element state. A fragment id is `link-{source offset}-{link}-{part}`.
  v0.7.0 also added a text-only paragraph fast path that builds one cached
  `Inline`; patch 0019 carries the decorator and underline onto it, which the
  rebase could not see because the path did not exist on v0.6.0.
- Upstream added an asynchronous web paste with a stale-target guard. Patch
  0017 keeps upstream's paste and carries only the grapheme-cluster
  boundaries, which still pass through upstream's atomic-token
  `cursor_boundary`.
- Upstream added Linux and Windows multi-cursor and word-selection chords.
  Patch 0016 places them inside its runtime split: the Command keymap gains
  `cmd-alt-up`/`cmd-alt-down`; the other keymap gains the Control
  home/end chords and keeps the Linux and Windows distinctions as `cfg`, which
  the web never reaches.

What the Theorem Zed source does not have, and how patch 0018 meets it:

- `register_inspector_element` takes the renderer closure, not a per-window
  factory, so the inspector entity is created on first use.
- `TestAppWindow::simulate_scale_factor_change` does not exist and the test
  window's scale factor is fixed at 2.0. The upstream line-height parity test
  runs at 2.0 with both preview zooms; its 1.6 case needs that hook added to
  the Zed fork first.
- `App::fetch_asset` answers with the shared load and whether the call
  started it, not an `Option` of a finished result. The shell's document
  images drop an image on release only when an existing load has finished.
- `#[derive(Action)]` expands to bare `gpui::` paths, so the story crate
  depends on `gpui` directly instead of reaching it only through `gpui-kit`.
- A replayed cached view does not re-record debug bounds. The cached-view
  selection test reads its probe from the first painted frame; its eight
  replayed-frame assertions run unchanged.

Verification on the rebased branch (rustc stable and 1.96.1 toolchains, the
Theorem repository pin): `cargo check -p gpui-base -p gpui-component --lib`
native and `wasm32-unknown-unknown`; `cargo nextest run --locked -p gpui-base
-p gpui-component --lib` 1804/1804; `cargo check -p gpui-component-story`,
the workflow's second check, with no warnings. The workflow's Rust 1.90 replay has not run:
GitHub Actions is disabled on this account.

## History from the v0.6.0 series

The final pin patches advance the Theorem Zed revision through its Rust 1.90
compatibility commits. The first selects the API-equivalent `oo7 0.6.0-alpha`
release (MSRV 1.86) because stable oo7 0.6.0 requires Rust 1.92. The second
replaces the newer `slice::as_array` helper with the stable array conversion
used by Rust 1.90. The third replaces post-1.90 standard-library profiling and
UTF-8 boundary helpers with equivalent local implementations. The final patch
uses the stable atomic update primitive for text-selection scope allocation,
then advances the Zed pin to include action-profiler compatibility as well.

The web integration pin selects the published textarea IME/accessibility repair
and its corrected workspace lock edge. Every GPUI family dependency continues
to resolve from the same immutable Theorem Zed revision.

The Linux portal follow-up advances the Zed source to its compatible ASHPD
patch. This consumer retains its existing ASHPD 0.13.10 lock entry, which
already declares a Rust 1.87 minimum; no consumer registry dependency changes.

The desktop keyboard-focus follow-up moves only the seven GPUI manifest pins
and 25 lock source revisions to Zed `2f7f1da474f6a8ab0f1f61ce35a1a2278ee31db4`.
It retains the registry dependency graph, including ASHPD 0.13.10. The pinned
web window keeps desktop shortcuts on its read-only IME event target after
an editor closes while preserving coarse-pointer keyboard dismissal.

The platform input configuration patch completes the v0.6.0 API migration:
`InputBaseState::input_configuration` and `set_input_configuration` forward
GPUI's actual `TextInputConfiguration` through `EntityInputHandler`. Defaults
remain unchanged. Search fields can request the existing `Search` action key
without restoring the retired text-input-hints API. Changing configuration
retains text and selection. This is a preceding input prerequisite; the
Theorem DATA conversion does not add a private input fork.

Validation of this addition: locked Wasm `gpui-base --lib` check passes. The
native `platform_input_configuration_preserves_default_and_forwards_changes`
regression covers default, Search builder, and changed Send/assistance settings.
It passed in the Linux workspace test job at source head `1636a905`; the
subsequent spelling configuration leaves that source unchanged. This branch
remains a draft requiring review before merge.

The spelling follow-up recognizes only complete Git `index` hash lines and
the exact `zed-scap` package identifier inside the ordered patches. It retains
source and prose spelling checks; no application or runtime code changes.

The final spelling follow-up recognizes the exact capture package name even
inside its escaped regex spelling in patch 0016. The exact CI typos-cli 1.50.1
reproduces that failure before this change and scans the entire final checkout
without errors afterward. Runtime and dependency source remain unchanged.

The textarea presentation patch adds `set_text_decorations` and
`text_decorations` to the ordinary multiline input. It reuses the existing
normalized UTF-8 range and edit adjustment rules, paints through the existing
decoration renderer, and leaves syntax parsing/LSP confined to the code editor.
Its default presentation remains empty. A native regression retains text,
selection, and active IME composition while styles change; native execution
is pending CI for this source. The locked Wasm library check passes.

Two existing runnable component doctests now import their actual owning crates
(`gpui_component` and `gpui`) instead of the undeclared facade `gpui_kit`.
The original examples and assertions remain executable. This draft still
requires review before merge.

The textarea regression uses the real Kit initializer before mounting the input.
Its test-only unused trait import, rejected by CI's warnings-as-errors gate, is
removed. No runtime behavior or assertions change in this follow-up.

The native textarea test reached all text/selection/IME assertions and exposed
stale styles after `set_value`. The replacement hook clears only the new
textarea presentation ranges; its default is a no-op, preserving existing
editor behavior. The same regression and all its assertions are retained.

The selected-replacement pin follows Zed's actual browser IME repair. It changes
only the seven GPUI manifest revisions and 25 lock source revisions; registry
packages remain unchanged. The replacement diff retains matching text inside
the selected range and fresh editor anchors, with UTF-16 boundary coverage.
The Wikia browser oracle remains the consumer's actual DOM/document gate.

The lock follow-up synchronizes the 25 source URLs with that manifest pin.
The initial guard counted URL lines rather than both revision occurrences in
each URL and refused the lock write. Both exact revision occurrences now match;
no package version, checksum, or dependency edge changes.

The sidebar decoration patch exposes the existing clickable menu row through
`SidebarMenuItem::item_with` and the retained toggle button through
`SidebarToggleButton::button_with`. The row hook excludes expanded submenus;
the button hook runs after the component's icon, label and click handler are
bound. Both default to no decoration, preserving existing mechanics. They
allow the downstream semantic adapter to publish the actual activation
geometry without a second role or interactive wrapper. The ordered source
series replay and locked Wasm component library check pass; native measured
pointer/semantic parity is a consumer
gate. This draft requires review.

Row decorations compose in call order. A native tab-stop decoration therefore
survives the subsequent semantic adapter: pinned GPUI delivers Enter and Space
through the row's existing click closure, including its submenu behavior.
No second key handler or interactive wrapper is installed. The actual consumer
AGPUI library, copied by hash into a bounded temporary workspace and bound to
this staged Kit source, passes all 159 native tests. Its new measured test
covers the parent row, expanded submenu exclusion, toggle button, pointer and
semantic activation, both keyboard keys, and disabled/read-only inertness.
The original activation, naming and Store source assertions also pass without
new exemptions. Locked consumer verification follows adoption of this exact
source revision; staged-source proof is not a browser deployment result.

The text-link decoration patch exposes each real parsed and measured line
fragment through `TextView::link_with`, preserving the original click handler
and source parser. `link_underline` selects solid, dotted, or absent underline
without reparsing text in the consumer. The default remains solid. Fragment
focus survives the selection layer's deferred mouse-down callback without
overriding focus chosen by navigation. The actual AGPUI consumer tests cover
wrapped pointer targets, semantic activation, Enter and Space, and drag
selection with the real TextSelectionLayer. Native interaction passed; browser
link and deployed application verification remain consumer gates.

The shipping-surface bump moves the zed pin to
207177c51070288bd172fea977ffb2d4cb835367, which corrects HTML-in-Canvas
detection to the members a browser defines. Both manifests must move together:
`[patch.crates-io]` rewrites crates.io sources and not git ones, so a consumer
that bumps only its own pin builds two `gpui` versions and fails where the two
dependency chains meet.

## Alert role and name, Button description (patch 0023)

`Alert` gains `role(impl Into<RoleOverride>)` and `aria_label(impl Into<SharedString>)`,
the same caller-controlled surface `Editor` and `Input` already have, through the
same `RoleOverride::resolve` the other components use. The default is unchanged
(`Role::Alert`); `RoleOverride::Presentational` publishes no node for the alert
container while its children keep theirs, for a host that publishes its own named
node for the same row. The name is published only while a role is. The alert's root
registers with `test_support()` so the role and name are observable in headless tests.

`Button` gains `accessibility_description`, written to the node as the AccessKit
description, so a control can say why it is unavailable. `ElementSnapshot` gains
`description()` to read it back.

At this patch, `Button::disabled(true)` did not yet publish AccessKit `disabled`,
because GPUI at the then-pinned Zed revision (`6d4d907`) had no way to set it.
Patch 0024 closes that.

## Disabled Button state (patch 0024)

The Zed pin moves from `6d4d907` to `45cde9a` on
`Travis-Gilbert/chat-workspace-native-motion`. That revision adds
`StatefulInteractiveElement::aria_disabled(bool)` and has `gpui_web` project a
node's description, disabled, required and read-only states as `aria-*`
attributes. The base `Button` now calls `aria_disabled` from `render`, using the
same flag that blocks its pointer input, so a disabled button reads as disabled
to assistive technology rather than only looking dim. The assertion in
`crates/kit/tests/components.rs` is now `window.find("disabled").disabled() ==
Some(true)`. TheoremWeb moves its Kit and Zed pins together with this patch, so
one `gpui` copy stays in its graph.

The platform selection patch implements the input handler's selection setters
on `InputBaseState`: `set_text_selection` takes a UTF-16 range and its
direction from the platform (a browser field's backward selection stays
backward, a caret is never reversed), `set_selected_text_range` is its forward
form, and `text_length_utf16` reports the document's length for a conduit that
mirrors the whole document. It moves the Zed pin to `1c7e4989`, the editing
island patch that adds the `set_text_selection` hook. Tested by
`test_platform_selection_keeps_direction_in_utf16`.

The paired GPUI pin patch (0026) moves Zed to `3f09c4f7`, whose public typed
`App::start_drag` lets a platform-owned source use the existing GPUI drop
coordinator. Browser Tab traversal leaves the canvas at either end rather than
wrapping, with native desktop traversal preserved. No Kit input or component
behavior changes in this pin move. TheoremWeb consumes the same Zed revision.

The paired GPUI pin patch (0027) moves Zed to `dc9632d4`, refusing an editing
island replacement before changing the active conduit while composition is
open. It changes no Kit behavior. TheoremWeb consumes the same revision.

## Retained platform owners (patches 0036–0039)

The dock panel menu anchor and native drag preview constructor reuse the actual
TabGroup/DragPanel owners. Notification presentation delegates guarded input to
the retained Notification and preserves its list clock. Menu focus-close reports
the existing owner decision so a DOM invoker restores focus only when the native
action did not move it. The paired GPUI pin exposes a read-only typed drag payload
so a retired scope can refuse delayed events for a successor session.
