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
