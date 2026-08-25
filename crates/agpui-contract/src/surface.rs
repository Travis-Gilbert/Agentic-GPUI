//! The drawer leaf's document shapes.
//!
//! These live here, not in the GPUI crate that renders them, for the same
//! reason [`crate::intent`] does: the shell composes a drawer document out of
//! a transcript part and the leaf parses it back, and a second declaration of
//! the same wire shape on the other side of that seam is a drift a compiler
//! cannot see. The renderer keeps its own validation and projection - what is
//! shared is the shape, not the behaviour.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The largest document the drawer leaf will parse.
///
/// Declared beside the shape rather than only at the parse site so a producer
/// can refuse before it serializes, instead of learning the limit from a
/// rejection it has already paid to encode.
pub const MAX_SURFACE_BYTES: usize = 512 * 1024;

/// The node and edge ceilings a graph document is held to.
pub const MAX_GRAPH_NODES: usize = 128;
/// See [`MAX_GRAPH_NODES`].
pub const MAX_GRAPH_EDGES: usize = 256;

/// What the right drawer is showing.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SurfaceDocument {
    /// Nothing has been lifted into the drawer yet.
    #[default]
    Preview,
    /// One artifact, expanded out of the thread.
    Artifact(ArtifactDocument),
    /// A programmable-graph projection.
    Graph(GraphDocument),
}

/// One artifact, carrying the payload that already crossed the wire once.
///
/// The payload travels with the document rather than being re-fetched by id:
/// the transcript already holds it, and a second read could answer with a
/// different revision than the one the user clicked.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactDocument {
    pub artifact_id: String,
    pub artifact_revision: String,
    pub artifact_kind: String,
    pub contract_version: String,
    pub renderer_id: String,
    pub fallback_text: String,
    pub payload_ref: String,
    pub payload: Value,
    #[serde(default)]
    pub patches: Vec<Value>,
}

/// The one graph-projection schema this drawer speaks.
///
/// Named here rather than spelled out at each site: the producer
/// (`rustyred-thg-programmable-graph::materialize`), the resolver, and the
/// renderer's validator all had their own copy of the string, and a schema
/// bump that missed one of them would show as an empty drawer rather than a
/// refusal.
pub const GRAPH_CANVAS_SCHEMA: &str = "programmable-graph-canvas/1";

/// The widest graph field that still draws as a sheet rather than as wires.
///
/// A contract rather than a renderer detail: the renderer picks its projection
/// from the field's measured width, and the story declarations pick their
/// viewports to land on either side of the same number. Two copies of it would
/// let a story claim a projection the renderer would not choose.
pub const SHEET_PROJECTION_MAX_WIDTH: f64 = 640.0;

/// A programmable graph, as far as the drawer draws one.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphDocument {
    pub schema: String,
    pub program_id: String,
    pub graph_revision: String,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

/// One block in a graph document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub node_id: String,
    pub block_id: String,
    pub contract_id: String,
    pub process_kind: String,
    #[serde(default)]
    pub input_shapes: Vec<String>,
    #[serde(default)]
    pub output_shapes: Vec<String>,
    pub liveness: String,
}

/// One typed connection in a graph document.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub edge_id: String,
    pub from_node: String,
    pub from_port: String,
    pub to_node: String,
    pub to_port: String,
    pub shape_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_is_the_default_and_tags_itself() {
        let json = serde_json::to_string(&SurfaceDocument::default()).unwrap();
        assert_eq!(json, r#"{"kind":"preview"}"#);
    }

    #[test]
    fn an_artifact_document_is_camel_case_on_the_wire() {
        let document = SurfaceDocument::Artifact(ArtifactDocument {
            artifact_id: "artifact-1".to_owned(),
            artifact_revision: "rev-1".to_owned(),
            artifact_kind: "scene".to_owned(),
            contract_version: "scene-package-v2".to_owned(),
            renderer_id: "scene-os".to_owned(),
            fallback_text: "a scene".to_owned(),
            payload_ref: "sha256:abc".to_owned(),
            payload: serde_json::json!({}),
            patches: Vec::new(),
        });
        let json = serde_json::to_string(&document).unwrap();
        assert!(json.contains(r#""kind":"artifact""#));
        assert!(json.contains(r#""artifactRevision":"rev-1""#));
        assert!(!json.contains("artifact_revision"));
        assert_eq!(
            serde_json::from_str::<SurfaceDocument>(&json).unwrap(),
            document
        );
    }
}
