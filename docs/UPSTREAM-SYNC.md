# Upstream sync

AGPUI is a hard fork of Longbridge `gpui-kit` at tag `v0.6.0`
(`94a313a72a2513aee2780240cd322d552b2395f0`). Upstream crate names and paths
are deliberately untouched so a merge stays mechanical. Nothing here is ever
proposed back upstream.

```toml
[workspace.metadata.agpui]
upstream = "https://github.com/longbridge/gpui-kit"   # Longbridge
upstream_tag = "v0.6.0"
zed_rev = "0a3e29a42a1646cb9f50df076e2aa6c9fda85868"
```

## The runbook

A Longbridge release lands. In this repository:

```bash
git remote add upstream https://github.com/longbridge/gpui-kit   # Longbridge, once
git fetch upstream --tags
git checkout -b upstream-sync/v0.7.0
git merge v0.7.0
```

1. **Resolve the merge.** Conflicts should be confined to the fork lane crates
   under `crates/{kit,base,component,...}`. A conflict inside `crates/agpui*` means
   an AGPUI crate has grown a dependency on an upstream internal — fix that, not
   the conflict.
2. **Decide the GPUI revision.** Either bump `zed_rev` to what the release needs,
   or carry patches to keep the current one. Whichever you choose, `zed_rev` in
   `[workspace.metadata.agpui]` is the single declaration; consumers read it.
3. **Replay the fork patches.** Walk the table below in order. A patch upstream
   has independently implemented is dropped during the rebase, not reapplied —
   record the drop in `patches/README.md` the way the `v0.6.0` rebase recorded
   its three.
4. **Run the ladder.** `cargo check --locked --workspace` and
   `cargo nextest run --workspace` on Rust 1.90; the wasm lane; `tests/boundary.rs`;
   the hash-pin tests; `scripts/check-provenance.sh`; `scripts/check-agpui-naming.sh`.
5. **Bump `upstream_tag`** and update this file's replay table with any patch
   that changed shape.
6. **One line in Theorem.** Consumers see a single `agpui_rev` bump. Theorem's
   pin guard plus both workspaces' `cargo check --locked` are the whole review.

A failed scheduled replay is the signal to rebase the series, not to skip it.

## The replay list

Eighteen fork-only commits sit between Longbridge `v0.6.0` and the fork lane
tip `c8f28357`. Every one is replayable from `patches/`, applied in the order
given by `patches/series`. Commits marked *record* are the companion commits
that write the patch file for the change above them; they carry no code.

| # | Commit | Subject | Patch |
|---|---|---|---|
| 1 | `cd900db6` | `fix(test): preserve downstream GPUI fixture support` | `0001-downstream-test-support.patch` |
| 2 | `bcaefa92` | `fix(a11y): replay accessible navigation labels and roles` | `0002-accessible-navigation.patch` |
| 3 | `d6e6f242` | `feat(table): replay record-grid interaction seams` | `0003-record-grid-seams.patch` |
| 4 | `f185160e` | `feat(settings): add labelled settings sections` | `0004-settings-sections.patch` |
| 5 | `4376c1ff` | `build(gpui): use the Theorem Zed source` | `0005-theorem-zed-source.patch` |
| 6 | `82827038` | `chore(fork): add replayable GPUI Kit patch series` — the Longbridge fork series itself | *(the series)* |
| 7 | `928d84f3` | `fix(gpui): pin the Rust 1.90 Zed revision` | `0006-rust-1.90-zed-pin.patch` |
| 8 | `c3eacd4d` | `chore(fork): record the Rust 1.90 source pin` | *record* |
| 9 | `3a99e3d9` | `fix(gpui): pin the Rust 1.90 utility revision` | `0007-rust-1.90-utility-pin.patch` |
| 10 | `b1f60bf5` | `chore(fork): record the Rust 1.90 utility pin` | *record* |
| 11 | `995cfa79` | `fix(gpui): pin the Rust 1.90 stdlib revision` | `0008-rust-1.90-stdlib-pin.patch` |
| 12 | `b5ca1591` | `chore(fork): record the Rust 1.90 stdlib pin` | *record* |
| 13 | `0c6127d7` | `fix(base): support Rust 1.90 atomics` | `0009-rust-1.90-atomics.patch` |
| 14 | `4db11f02` | `chore(fork): record the Rust 1.90 atomics patch` | *record* |
| 15 | `42b38627` | `fix(gpui): pin the complete Rust 1.90 revision` | `0010-rust-1.90-final-zed-pin.patch` |
| 16 | `d05ea1ab` | `chore(fork): record the complete Rust 1.90 pin` | *record* |
| 17 | `1a2a02a7` | `feat(settings): add controlled navigation state` | `0011-feat-settings-add-controlled-navigation-state.patch` |
| 18 | `c8f28357` | `chore(fork): record controlled settings navigation` | *record* |

`42b38627` (row 15) is the revision Theorem pinned before AGPUI became the
home; rows 17 and 18 postdate it. After the move, Theorem pins the AGPUI tag,
not a fork-lane commit.

## What the patches do

Rows 1 through 4 are downstream seams: fixture support the upstream test suite
does not need, accessible navigation labels and roles, record-grid interaction
seams on the table, and labelled settings sections. Row 17 adds controlled
navigation state to settings.

Rows 5, 7, 9, 11, 13 and 15 exist for one reason: the whole GPUI family must
resolve to a single revision of `Travis-Gilbert/zed`, and that revision must
build on Rust 1.90. They redirect the GPUI source and then advance the pin
through its Rust 1.90 compatibility commits — selecting the API-equivalent
`oo7 0.6.0-alpha` release because stable `oo7 0.6.0` requires Rust 1.92,
replacing the newer `slice::as_array` helper with the stable array conversion,
replacing post-1.90 standard-library profiling and UTF-8 boundary helpers with
local equivalents, and using the stable atomic update primitive for
text-selection scope allocation.

`patches/README.md` records the three patches the `v0.6.0` rebase deliberately
dropped: Longbridge change `#2823`, already present in `v0.6.0`; the earlier
selectable-text patch, superseded by `gpui_base::SelectableText`; and the old
text-input-hints chain, superseded by Zed's `TextInputConfiguration` at the
pinned revision.

## Not this

- Nothing is upstreamed to Longbridge, to GPUI Box, to Zed, or to Butler.
- Upstream crates are not renamed. `agpui::kit` re-exports `gpui_component` so a
  consumer names one path without any upstream path changing.
- The `theorem-chat/1` schema id is not renamed. It is a frozen vocabulary with
  contracts and oracles bound to it, carried here as the runtime's default.
