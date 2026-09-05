# agpui-contract

Renderer-free semantic-tree contract for AGPUI: nodes, snapshots, hashes, diffs, actions, receipts

## What it is

`agpui-contract`: the renderer-free half of the AGPUI semantic tree.

This is the first of the four AGPUI crates on the rate-of-change axis
(`agpui-contract`, `agpui-runtime`, `agpui-registry`, `agpui`). It carries
the types, the laws over them, the canonical hash, the diff, and the action
and receipt wire contract. It names no renderer, and `tests/boundary.rs`
fails the build if one enters its dependency tree.

See `docs/AGPUI-CRATES.md` and `docs/plans/SPEC-AGPUI-SEMANTIC-TREE-1_0.md`.

## Build and test

```bash
cd rustyredcore_THG && cargo test -p agpui-contract
```

Part of the `rustyredcore_THG` Cargo workspace. See the crate table in [CLAUDE.md](../../../CLAUDE.md) for how this fits the substrate. This README is generated from the crate's `Cargo.toml` description and `//!` module docs; edit those and regenerate with `scripts/gen-crate-readmes.sh`.
