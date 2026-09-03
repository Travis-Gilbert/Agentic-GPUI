//! `agpui-contract` names no renderer, and this is the proof.
//!
//! SPEC-AGPUI-SEMANTIC-TREE-1.0 D1. A crate can claim renderer independence in
//! a doc comment and lose it to one transitive edge nobody reads. This test
//! asks Cargo for the resolved graph rooted at this package and fails the
//! build when a GPUI package appears anywhere in it.

use std::collections::{HashMap, HashSet};
use std::process::Command;

use serde_json::Value;

/// Whether this package name means a renderer entered the contract.
///
/// A roster of names was the first version of this and it was already wrong:
/// it listed six and the workspace pins ten, so `gpui_macros`,
/// `gpui-component-assets`, `gpui-component-macros`, `declarative-gpui` and
/// `declarative-gpui-core` could all have entered without failing anything.
/// A roster has to be edited every time `scripts/check-gpui-editor-pin.sh`
/// pins one more, by someone who has no reason to look at this file. The
/// predicate needs no edit: every renderer package in this workspace is
/// `gpui`, is prefixed `gpui-` or `gpui_`, or is `declarative-gpui`.
fn is_renderer(package: &str) -> bool {
    package == "gpui"
        || package.starts_with("gpui-")
        || package.starts_with("gpui_")
        || package.starts_with("declarative-gpui")
}

#[test]
fn no_renderer_package_is_reachable_from_agpui_contract() {
    let manifest = format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(option_env!("CARGO").unwrap_or("cargo"))
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest)
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("metadata is JSON");

    let mut name_of: HashMap<&str, &str> = HashMap::new();
    let mut root = None;
    for package in metadata["packages"].as_array().expect("packages") {
        let id = package["id"].as_str().expect("package id");
        let name = package["name"].as_str().expect("package name");
        name_of.insert(id, name);
        if name == "agpui-contract" {
            root = Some(id);
        }
    }
    let root = root.expect("agpui-contract is in the metadata it asked for");

    let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in metadata["resolve"]["nodes"].as_array().expect("resolve nodes") {
        let id = node["id"].as_str().expect("node id");
        let dependencies = node["dependencies"]
            .as_array()
            .expect("node dependencies")
            .iter()
            .map(|dependency| dependency.as_str().expect("dependency id"))
            .collect();
        edges.insert(id, dependencies);
    }

    let mut reached: HashSet<&str> = HashSet::new();
    let mut frontier = vec![root];
    while let Some(id) = frontier.pop() {
        for dependency in edges.get(id).into_iter().flatten() {
            if reached.insert(dependency) {
                frontier.push(dependency);
            }
        }
    }

    let renderers: Vec<&str> = reached
        .iter()
        .filter_map(|id| name_of.get(id).copied())
        .filter(|name| is_renderer(name))
        .collect();
    assert!(
        renderers.is_empty(),
        "agpui-contract must name no renderer, and reached: {renderers:?}"
    );
}

#[test]
fn the_resolve_walk_actually_sees_the_declared_dependencies() {
    // Guards the test above against silently passing on an empty walk: if the
    // metadata shape changes and nothing is reached, `renderers` is trivially
    // empty and the boundary stops being checked.
    let manifest = format!("{}/Cargo.toml", env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(option_env!("CARGO").unwrap_or("cargo"))
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest)
        .output()
        .expect("cargo metadata runs");
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("metadata is JSON");
    let names: HashSet<&str> = metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .filter_map(|package| package["name"].as_str())
        .collect();
    for expected in ["serde", "serde_json", "sha2"] {
        assert!(names.contains(expected), "{expected} is a declared dependency");
    }
}

#[test]
fn the_predicate_catches_every_renderer_the_workspace_pins() {
    // The names `scripts/check-gpui-editor-pin.sh` asserts a single source
    // for, plus `gpui-box`, which this crate was ported from and which no
    // longer has a pin row. If that script grows a name shaped unlike these,
    // this test is where the mismatch should surface.
    for pinned in [
        "gpui",
        "gpui_macros",
        "gpui_platform",
        "gpui_web",
        "gpui-base",
        "gpui-component",
        "gpui-component-assets",
        "gpui-component-macros",
        "declarative-gpui",
        "declarative-gpui-core",
        "gpui-box",
    ] {
        assert!(is_renderer(pinned), "{pinned} must read as a renderer");
    }
    for unrelated in ["serde", "sha2", "agpui-contract", "theorem-surface-contracts"] {
        assert!(!is_renderer(unrelated), "{unrelated} is not a renderer");
    }
}
