# AGPUI

AGPUI is the framework half of Theorem's interface: a GPUI application kit whose
elements are legible to an agent as well as to a person. Every element publishes
a semantic node — role, identity, state, and the gestures it accepts — so a head
driving the application over the wire sees the same structure a reader sees on
screen, and a gesture it sends is receipted against the node it named. The
repository is a fork of Longbridge `gpui-kit`, extended with AGPUI-owned crates
for the contract, the semantic coordinator, the agent and canvas elements, the
renderer-neutral runtime kernel, the browser mitigations, the theme law, and a
story that hosts every element beside its snapshot. Theorem and RustyRed consume
it as one pinned git dependency, and the dependency runs one way: AGPUI never
names a Theorem record, tool, or route.

- [`docs/AGPUI.md`](docs/AGPUI.md) — the crate map and the three laws.
- [`docs/UPSTREAM-SYNC.md`](docs/UPSTREAM-SYNC.md) — the upstream merge runbook and the replay list.

## Three things are called kit

The word is overloaded across three unrelated projects. AGPUI always spells the
qualifier, and CI refuses a bare token.

| Kit | What it is | GPUI it builds against | Relationship to AGPUI |
|---|---|---|---|
| Longbridge `gpui-kit` | Components and paint: `gpui-component`, `gpui-base`, the dock, the table, the editor | Longbridge's Zed fork, redirected here to `Travis-Gilbert/zed@0a3e29a` | **Fork lane.** This repository's base. |
| GPUI Box `gpui-kit`, `gpui-kit-semantics` (`fran0220/gpui-box`) | Semantic tree vocabulary, agent kit, canvas; vendors its own GPUI | `gpui-box` by path | **Port lane.** Source read and rewritten against our GPUI. |
| Butler `gpuikit` (`iamnbutler/gpuikit`) | Single-crate toolkit, web showcase, `stitch`, editor | `gpui-unofficial` 1.18 from crates.io | **Borrow lane.** Read for ideas at [`Travis-Gilbert/gpuikit-reference`](https://github.com/Travis-Gilbert/gpuikit-reference); nothing is forked from it. |

## Crates by lane

Upstream crate names and paths are untouched, so an upstream merge stays
mechanical. AGPUI-owned crates sit beside them under `crates/agpui*`.

**Fork** — vendored from Longbridge, patched, never contributed back:

`crates/kit`, `crates/base`, `crates/component`, `crates/component-macros`,
`crates/assets`, `crates/shell`, `crates/component-shell`, `crates/webview`,
`crates/fps`, `crates/story`, `crates/story-web`.

**Own and port** — AGPUI's own crates. Each lands with the deliverable named:

| Crate | Lane | Lands in |
|---|---|---|
| `agpui-contract` | own + port | H2 |
| `agpui` | own + port | H3 |
| `agpui-agent` | port | H4 |
| `agpui-canvas` | port | H4 |
| `agpui-runtime` | own + port | H5 |
| `agpui-web` | own | H6 |
| `agpui-theme` | own | H6 |
| `agpui-story` | own | H8 |

`crates/agpui-scene` is a reserved name, not a crate. It is created the day a
`SceneView` composes; an empty crate calling nothing is a refusal.

## Pins

One GPUI. The whole family resolves to a single revision of
`Travis-Gilbert/zed`, declared here and read by every consumer:

```toml
[workspace.metadata.agpui]
upstream = "https://github.com/longbridge/gpui-kit"   # Longbridge
upstream_tag = "v0.6.0"
zed_rev = "0a3e29a42a1646cb9f50df076e2aa6c9fda85868"
```

AGPUI is the authority for `zed_rev`. Theorem's pin guard reads it and refuses
on mismatch rather than declaring the same fact twice.

## Licence

AGPUI-owned crates are `MIT OR Apache-2.0`. Crates forked from Longbridge keep
Apache-2.0. `LICENSE_LEDGER.md` records every ported file with its upstream
revision and licence; `PROVENANCE.toml` beside a ported module carries the
digest of the upstream file it was rewritten from.

Upstream documentation for the fork lane: <https://gpui-kit.com> (Longbridge).
