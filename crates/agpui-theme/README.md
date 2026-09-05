# theorem-design-core

Renderer-free design tokens for Theorem's Rust product surfaces.

The embedded DTCG source is `../../assets/design/theorem-tokens.json`. Cream
and ink tokens declare lightness only; this crate derives chroma from the
neutral law in `SPEC-THEOREM-STYLING-API-1.0` and rejects authored neutral
hexes. CSS, GPUI semantic and legacy JSON, prose capture styles, grain
parameters, and the static grain tile all project from the resulting typed
`TokenSet`.

Run the complete local gate from the repository root:

```sh
THEOREM_DESIGN_TARGET_DIR=/path/to/target scripts/check-theorem-design-core.sh
```

The grain baker specializes the active Paper Shaders product-chrome lane:
static roughness with fiber, crumples, folds, drops, fade, and animation
disabled by the token source. Its sidecar binds the baked PNG digest to a
hash of the renderer-neutral parameters. Rendered paper fidelity is verified
separately by the native/wasm visual oracle.
