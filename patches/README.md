# Theorem GPUI Kit patch series

This hard fork tracks `longbridge/gpui-kit` tag `v0.6.0`. The files listed in
`series` are applied in order and are never proposed upstream. The fork pins
the entire GPUI family to one immutable revision of `Travis-Gilbert/zed`.

The replay workflow applies this series to `v0.6.0` for branch validation and
to upstream `main` on its monthly schedule. A failed scheduled run is the
signal to rebase the series. If upstream independently implements a patch, the
patch is removed during that rebase.

The v0.6.0 rebase deliberately drops three historical patches:

- Longbridge change `#2823` is already present in v0.6.0.
- The earlier selectable-text patch is superseded by v0.6.0's
  `gpui_base::SelectableText` implementation.
- The old text-input-hints chain is superseded by Zed's
  `TextInputConfiguration` API at the pinned Theorem revision.

The replayable text-change-delta commit from the old fork was not consumed by
the named Theorem PR stack and is not part of this series.

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
