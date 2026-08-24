# Paper Shaders 0.0.77 oracle assets

These files are unmodified oracle inputs from the published
`@paper-design/shaders@0.0.77` package:

- package git head: `f9f2a8b2edeb78ec59256c4dc571f5eaf943d798`
- npm tarball SHA-256: `6b77c990dc98d794011b1374bd183ef94464f280ee289e63554a2cc373dec481`
- `paper-texture.js` SHA-256: `b2fa3e8281bf85f9505880056d0cec947454604f4c780e11257ffec416d7e8ef`
- `noise.png` SHA-256: `5116a06c428a75e2db9bd55062c560bb02600383ee54da007f1628e845b2b73a`

`theorem-design-core` specializes the shader's static roughness lane on the
CPU. The vendored source and noise hashes are checked before every bake and
are included in the generated sidecar receipt. The Rust translation is a
modified derivative; the original Apache-2.0 `LICENSE` and `NOTICE` are
retained here.
