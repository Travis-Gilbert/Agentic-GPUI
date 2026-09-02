use std::sync::Arc;

use gpui::SharedString;

use crate::text::{
    document::ParsedDocument,
    node::{BlockNode, NodeContext, Paragraph},
};

/// Parse plain text, taking every character literally.
///
/// The other two formats are markup, and a value is not authored markup. An
/// id like `task_a_1` loses its underscores to emphasis, a Windows path
/// drops its backslashes to escapes, and `<null>` vanishes into an unknown
/// tag. Nothing here may be read as markup, so nothing is parsed: the text
/// becomes text runs and stops.
///
/// Each line is its own paragraph, which is what lets a copied selection
/// round-trip. [`BlockNode::text`] writes one newline after each non-empty
/// block, so the rendered text of a multi-line value is the value again. A
/// blank line has no text to write and so does not survive that round trip,
/// exactly as in Markdown, where a blank line is a block separator rather
/// than content.
pub(crate) fn parse(source: &str, _: &mut NodeContext) -> Result<ParsedDocument, SharedString> {
    let blocks = source
        .split('\n')
        .map(|line| BlockNode::Paragraph(Paragraph::new(line.to_owned())))
        .collect::<Vec<_>>();

    Ok(ParsedDocument {
        source: source.to_owned().into(),
        blocks: Arc::new(blocks),
    })
}
