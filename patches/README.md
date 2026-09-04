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
