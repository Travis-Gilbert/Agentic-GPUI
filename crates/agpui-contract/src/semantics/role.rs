//! The closed role vocabulary every semantic node is drawn from.
//!
//! Ported from `gpui-box`, `crates/gpui-kit-semantics/src/lib.rs`, at
//! `e993d0f4e2dbd4a9697db79c6428a623856444a4` (GPUI Box contributors,
//! MIT OR Apache-2.0). Split out of the monolithic module by
//! SPEC-AGPUI-SEMANTIC-TREE-1.0 D1; the variants are unchanged.
//!
//! The mapping onto platform accessibility roles lives in `agpui`, because it
//! names GPUI types. Nothing here knows a renderer exists.

use serde::{Deserialize, Serialize};

/// What a node is, in the vocabulary an agent, a test, and a screen reader
/// share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Window,
    #[default]
    Region,
    Group,
    List,
    Row,
    Button,
    Link,
    Tab,
    TabPanel,
    Input,
    MultilineInput,
    PasswordInput,
    Text,
    Heading,
    Dialog,
    Menu,
    MenuItem,
    Status,
    Checkbox,
    Radio,
    Switch,
    Slider,
    Table,
    TreeGrid,
    Cell,
    GridCell,
    Tree,
    TreeItem,
    Progress,
    Toast,
    Tooltip,
    Separator,
    Splitter,
    Toolbar,
    Scrollbar,
    Combobox,
    Option,
    Form,
    Field,
    Image,
    /// A drag in flight: what is being carried, and where it would land.
    Drag,
}

impl Role {
    /// True for the three roles that accept typed text.
    ///
    /// `SemanticGesture::SetValue` refuses every other role, so this is the
    /// one place the answer is written down.
    #[must_use]
    pub const fn accepts_typed_text(self) -> bool {
        matches!(self, Self::Input | Self::MultilineInput | Self::PasswordInput)
    }
}
