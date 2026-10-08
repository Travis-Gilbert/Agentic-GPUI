# World retained-control continuation, 2026-10-07

The World candidate needs the retained IntelliJ presentation's native controls
on the same GPUI renderer that exports submitted paint through IOSurface.
Replay source `fd20e5c5466d41abcc36a42ddfc0dd9d7fd94f31` and patch record
`2a10431b522fef9e2ed44b2d56b4a269800b297b` onto the coherent Kit pin-only
base `a81356a6885efe59a8c278e612d4b45cb272da47`. The resulting source and
patch commits are `7a87d97c75661a2084f5a561517c6b4e915758fd` and
`dfa1ddf599c65e91da40106e72df15e5c3c9e456`.

The 17-file source closure includes `MeasuredLayout`, Base `BoundsObserver`,
button/checkbox bounds observation, checkbox accessible state, owner image
icons, native menu image support and keyed tab focus/navigation/root children.
The application remains the layout and document owner. No new editor engine,
provider authority, custom titlebar or document state store is supplied here.

Cargo manifests, lockfile, registry versions/checksums and existing Wasm feature
declarations are byte-identical to the pin-only base. All GPUI family entries
remain `fbed33d116a8f50b76535b7d73164fed11465bec`. Patches 0021–0023 record
the existing capture/export pin commits so replaying 0001–0023 reaches the
current source and dependencies rather than the old renderer.

## Current checks

Executed on macOS arm64 with Rust 1.97.1, normal Cargo home, one job, sccache,
debug info disabled and incremental compilation disabled:

```sh
cargo +1.97.1 check --locked --offline -p gpui-base -p gpui-component --lib
```

**Passed**, finished in 7m44s. The inherited `block 0.1.6` future-incompatibility
warning remains; it was not suppressed. Target was the task-owned SSD alias
`/Users/travisgilbert/.codex/ssd-theorem-world-gpui-20261007`. This is an actual
owning library check of the retained source APIs with the current GPUI fork.
No native test suite, current Wasm check or mounted IntelliJ acceptance is
claimed by compilation. The historical 1804-test result in README predates this
continuation and is retained solely as historical evidence.

All 23 ordered patches were applied to a disposable exact `v0.7.0` archive.
Comparing 3612 tracked files outside `patches/` found every Rust source, Cargo
manifest/lock and website source byte-identical. The sole inherited difference
is `.github/workflows/gpui-kit-patch-series.yml`, which the earlier patch-record
commit added without including its final workflow contents in the source
series. That governance-file mismatch remains explicit; the full tree replay
is not claimed exact and this continuation does not refresh that baseline.

Actual consumer compile, durable singleton document integration, native input
and live provider/mounted visual journeys remain World integration obligations.
Publication supplies an immutable dependency candidate for those checks.
