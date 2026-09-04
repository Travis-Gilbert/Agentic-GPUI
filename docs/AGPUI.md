# AGPUI: the crate map and the laws

## The three laws

### 1. One GPUI

Every crate in this repository, and every consumer of it, resolves the GPUI
family — `gpui`, `gpui_platform`, `gpui_web`, `gpui_macros`, `sum_tree` — to a
single revision of `Travis-Gilbert/zed`. That revision is declared once, in the
root `Cargo.toml`:

```toml
[workspace.metadata.agpui]
zed_rev = "0a3e29a42a1646cb9f50df076e2aa6c9fda85868"
```

Two GPUIs in one binary is not a version conflict, it is two incompatible
`Entity` and `Context` type families that fail to unify at the call site, so the
law is enforced mechanically rather than by convention. AGPUI is the authority:
Theorem's pin guard reads `zed_rev` from AGPUI's metadata and refuses when its
own manifests disagree. The upstream tag AGPUI is forked from
(`upstream_tag`) is likewise AGPUI's fact, not the consumer's.

The port lane exists because the ported sources came from projects that vendor
their own GPUI. GPUI Box's kit depends on `gpui = { package = "gpui-box", path
= "crates/gpui" }`; Butler `gpuikit` depends on `gpui-unofficial` from
crates.io. Neither dependency crosses into this repository. Their source was
read and rewritten against our GPUI, which is what makes it a port rather than
a vendor.

### 2. The dependency runs one way

AGPUI does not know Theorem exists. No crate here names a RustyRed record type,
a Theorem tool, a Theorem HTTP route, or `theorem_body_registry`. The boundary
is not a style preference; it is what lets the story app, the portfolio site,
and RustyRed's service images each build an AGPUI crate with no part of the
Theorem tree present.

Two consequences are load-bearing:

- **The closure decides what moves, not the topic.** A module that looks like a
  framework concern but reaches a Theorem type stays in Theorem. A module that
  two AGPUI crates both need moves down into `agpui-contract`. The
  classification is computed from `cargo metadata`, recorded, and then applied.
- **Theorem extends AGPUI, never the reverse.** Where a Theorem-only concept
  must ride a moved vocabulary, it travels through an open variant
  (`MessagePart::Unknown`) rather than being pulled into the contract.

`tests/boundary.rs` is the observable form of this law. The story app in
`crates/agpui-story` is the second: an element the story cannot host without
Theorem is not an AGPUI element.

### 3. Provenance travels with the code

Nothing here is anonymous.

- Every ported file keeps its attribution header naming the upstream project,
  file, and revision it was rewritten from.
- `PROVENANCE.toml` sits beside the ported module and carries the digest of the
  upstream file at that revision. `scripts/check-provenance.sh` recomputes and
  compares, so a silent upstream drift is a failing check rather than a
  discovery years later.
- Upstream test names are kept verbatim, so a later sync diffs behaviour rather
  than guessing at intent.
- `LICENSE_LEDGER.md` at the root carries one row per ported family.
- History is carried, not copied. Moved files arrive by `git filter-repo` and
  are merged with `--allow-unrelated-histories`, so `git blame` on a moved file
  still reaches the commit that wrote the line.

## The crate map

```text
agpui-contract        The vocabulary. Semantic nodes, roles, identity,
                      snapshots, the four UI documents. No renderer.
   |
   +-- agpui          The GPUI half: the semantic probe, the coordinator,
   |                  the dispatcher, interaction, shell regions.
   |     |
   |     +-- agpui-agent     Agent elements: the agent kit, elicitation.
   |     +-- agpui-canvas    Canvas elements: nodes and edges.
   |     +-- agpui-web       Browser mitigations: boot, overlay, drop,
   |     |                   and the leaf's JavaScript side.
   |     +-- agpui-theme     The theme law (CIE LCH, APCA), the token
   |     |                   schema, the GPUI emitter, the checker.
   |     +-- agpui-story     Hosts every element beside its snapshot.
   |
   +-- agpui-runtime   The renderer-neutral kernel: threads, messages,
                       parts, runs, sessions, replay, scope, SSE.
                       Carries the `theorem-chat/1` wire vocabulary.
```

Arrows point from the depended-upon to the dependant. `agpui-contract` names no
renderer at all, which is what lets RustyRed's service images build it alone.

### What each crate is for

| Crate | Lane | Its job |
|---|---|---|
| `agpui-contract` | own + port | The words. A semantic node's role, identity, state, and accepted gestures; the snapshot an external head reads; the four UI documents. Standalone manifest with explicit dependencies, deliberately not inheriting from the workspace, so it builds outside this tree. |
| `agpui` | own + port | The GPUI half of the contract. Walks the element tree, publishes snapshots, routes an incoming gesture to the element that declared it, and owns the shell's region geometry. |
| `agpui-agent` | port | The elements an agent conversation is made of, and elicitation. |
| `agpui-canvas` | port | Graph nodes and edges as first-class semantic elements. |
| `agpui-runtime` | own + port | The state machine underneath a conversation, with no renderer in it. The `theorem-chat/1` schema id is a frozen vocabulary and is not renamed. |
| `agpui-web` | own | What the browser needs and the desktop does not: boot sequencing, the overlay, drop handling, and the leaf's JavaScript half. |
| `agpui-theme` | own | Colour and metric law as code. Metrics are values (`ShellMetrics`), not a const table, so a consumer supplies its own without forking the law. |
| `agpui-story` | own | The boundary oracle. Every element, hosted, with its snapshot visible beside it. |

## Reading order

Start at `agpui-contract`: the vocabulary explains every crate above it. Then
`agpui`'s coordinator, which is where a rendered element becomes a published
node. Then the story, which is the shortest path to seeing both at once.
