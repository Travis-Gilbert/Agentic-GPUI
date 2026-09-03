//! `agpui-contract` names no renderer, and this is the proof.
//!
//! SPEC-AGPUI-SEMANTIC-TREE-1.0 D1. A crate can claim renderer independence in
//! a doc comment and lose it to one transitive edge nobody reads. This test
//! asks Cargo for the resolved graph rooted at this package and fails the
//! build when a GPUI package appears anywhere in it.

use std::collections::{HashMap, HashSet};
use std::process::Command;

use serde_json::Value;

/// The package names that would mean a renderer entered the contract. Both
/// spellings of each are listed because Cargo reports the package name, and
/// the workspace pins some of these under a `package = ` rename.
const RENDERER_PACKAGES: [&str; 6] = [
    "gpui",
    "gpui_platform",
    "gpui_web",
    "gpui-component",
    "gpui-base",
    "gpui-box",
];

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
        .filter(|name| RENDERER_PACKAGES.contains(name))
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
