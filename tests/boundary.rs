//! AGPUI does not know Theorem exists, and one GPUI is resolved. This is the proof.
//!
//! SPEC-AGPUI-HOME-1.0 H2. The two laws this file guards are the ones that
//! cannot be checked by reading a manifest, because both are properties of the
//! *resolved* graph rather than of any one declaration:
//!
//! - The dependency runs one way. A single transitive edge into a `theorem-`,
//!   `theoremweb-`, or `rustyred` package would end the claim that the story
//!   app, the portfolio site, and RustyRed's service images can each build an
//!   AGPUI crate with no part of the Theorem tree present.
//! - One GPUI. Two revisions of the GPUI family in one binary is not a version
//!   conflict; it is two incompatible `Entity` and `Context` type families that
//!   fail to unify at the call site. Cargo will happily resolve both when two
//!   manifests name different revs, so the rev is asserted, not assumed.
//!
//! `crates/agpui-contract/tests/boundary.rs` is the crate-scope sibling: it
//! asks whether a renderer reached the contract. This one asks whether Theorem
//! reached the workspace, and whether the renderer that did reach it is one.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use serde_json::Value;

/// The resolved graph for the whole workspace, with every feature on.
///
/// `--all-features` for the reason the crate-scope sibling documents: an
/// optional dependency behind a feature nothing enables by default is absent
/// from `resolve.nodes` entirely, so a default-feature walk would let a
/// renderer or a Theorem package enter behind a feature flag and report clean.
fn metadata() -> Value {
    let manifest = format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(option_env!("CARGO").unwrap_or("cargo"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--all-features",
            "--manifest-path",
        ])
        .arg(&manifest)
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("metadata is JSON")
}

/// Whether this package name means Theorem entered AGPUI.
///
/// Prefixes rather than a roster, for the reason the sibling file argues at
/// length: a roster has to be edited by whoever adds the eleventh crate, and
/// they have no reason to look here. Every package in the Theorem monorepo is
/// named `theorem-*`, `theoremweb-*`, or `rustyred*`.
fn is_theorem(package: &str) -> bool {
    package.starts_with("theorem-")
        || package.starts_with("theoremweb-")
        || package.starts_with("rustyred")
        || package == "theorem"
        || package == "commonplace"
}

/// Whether this package is part of the GPUI family.
fn is_renderer(package: &str) -> bool {
    package == "gpui"
        || package.starts_with("gpui-")
        || package.starts_with("gpui_")
        || package.starts_with("declarative-gpui")
}

/// Every package in the resolved graph as `(name, source)`.
///
/// `source` is `null` for a path dependency and otherwise carries the registry
/// or the git URL together with the exact revision, which is the whole reason
/// this test reads it rather than reading the manifests.
fn resolved_packages(metadata: &Value) -> Vec<(String, String)> {
    metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .map(|package| {
            let name = package["name"].as_str().expect("package name").to_owned();
            let source = package["source"].as_str().unwrap_or("path").to_owned();
            (name, source)
        })
        .collect()
}

#[test]
fn no_theorem_package_is_in_the_agpui_resolve_graph() {
    let metadata = metadata();
    let offenders: BTreeSet<String> = resolved_packages(&metadata)
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| is_theorem(name))
        .collect();
    assert!(
        offenders.is_empty(),
        "AGPUI must not know Theorem exists, and resolved: {offenders:?}"
    );
}

#[test]
fn every_gpui_crate_resolves_to_one_source_and_one_rev() {
    let metadata = metadata();

    let mut sources_by_name: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (name, source) in resolved_packages(&metadata) {
        if is_renderer(&name) {
            sources_by_name.entry(name).or_default().insert(source);
        }
    }

    assert!(
        !sources_by_name.is_empty(),
        "the walk found no GPUI package at all, so it is not checking anything"
    );

    let split: Vec<(&String, &BTreeSet<String>)> = sources_by_name
        .iter()
        .filter(|(_, sources)| sources.len() > 1)
        .collect();
    assert!(
        split.is_empty(),
        "one GPUI: these packages resolved to more than one source: {split:?}"
    );

    // The families that come from git must agree on the revision. Two revs of
    // the same package under different names is exactly the failure the
    // per-name check above cannot see.
    let revs: BTreeSet<&str> = sources_by_name
        .values()
        .flatten()
        .filter(|source| source.starts_with("git+"))
        .filter_map(|source| source.split_once('#').map(|(_, rev)| rev))
        .collect();
    assert!(
        revs.len() <= 1,
        "one GPUI: the git-sourced renderer crates disagree on the revision: {revs:?}"
    );
}

#[test]
fn the_walk_actually_sees_the_workspace() {
    // The guard both boundary files need: if the metadata shape changes and
    // the package list comes back empty, every assertion above passes on
    // nothing and the boundary silently stops being checked.
    let metadata = metadata();
    let names: BTreeSet<String> = resolved_packages(&metadata)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    for expected in ["agpui-contract", "gpui-kit", "serde"] {
        assert!(names.contains(expected), "{expected} is in this workspace");
    }
}

#[test]
fn the_predicates_separate_theorem_from_agpui_from_the_renderer() {
    for theorem in [
        "theorem-surface-contracts",
        "theoremweb-app",
        "rustyred-thg-core",
        "rustyred-web",
    ] {
        assert!(is_theorem(theorem), "{theorem} must read as Theorem");
    }
    // AGPUI's own crates share no prefix with Theorem's, which is what lets
    // the check above be a prefix test rather than a roster.
    for agpui in [
        "agpui-contract",
        "agpui",
        "agpui-agent",
        "agpui-canvas",
        "agpui-runtime",
        "agpui-web",
        "agpui-theme",
        "agpui-story",
    ] {
        assert!(!is_theorem(agpui), "{agpui} is AGPUI's own, not Theorem's");
        assert!(!is_renderer(agpui), "{agpui} is not a GPUI package");
    }
    for renderer in ["gpui", "gpui_web", "gpui-component", "declarative-gpui"] {
        assert!(is_renderer(renderer), "{renderer} must read as a renderer");
        assert!(!is_theorem(renderer), "{renderer} is not Theorem's");
    }
}
