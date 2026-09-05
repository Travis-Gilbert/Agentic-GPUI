# agpui-theme

The theme law, and nothing anybody's brand is in.

A DTCG token document declares its neutral steps by lightness alone. This crate
derives their chroma from one curve, resolves aliases, and emits the result for
GPUI and for CSS. It embeds no token file: `TokenSet::from_dtcg_str` takes the
document as an argument, so the law can be true of more than one palette.

`ShellMetrics` is the same division applied to geometry. AGPUI owns the shape of
a shell's chrome, in rem; the product fills in its own numbers and holds them as
a const beside its token file.

## What is here and what is not

| Here | In the product |
| --- | --- |
| `NeutralLaw`, `NeutralSample`, `TokenSet` | the token document itself |
| alias resolution and the two parse refusals | the const that names the document |
| `emit_gpui`, `emit_css`, prose highlight styles | the grain PNG bake and its vendored shader |
| `ShellMetrics`, the shape | `METRICS`, the values |
| law tests over `fixtures/law.tokens.json` | every test that asserts a hex |

The fixture is deliberately not a palette. Its hue is blue and its ramp is
round, so a value that leaks from it into a product assertion is obvious.

## theme_check

```
cargo run -p agpui-theme --bin theme_check -- <tokens.json>
```

Parses the document, which is most of the check, then verifies that every
generated neutral is inside the declared relative chroma bound and that every
prose capture resolves.

## Provenance

Moved from Theorem's `theorem-design-core` by SPEC-AGPUI-HOME-1.0 H7, with
history. It is AGPUI's own code, not a port: it shares no lineage with the
Longbridge `gpui-kit` fork this repository is built on, with the GPUI Box
`gpui-kit` crates, or with Butler `gpuikit`.

Licence: MIT OR Apache-2.0.
