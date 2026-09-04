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

    /// True for the roles a pointer press is addressed to.
    ///
    /// `SemanticGesture::Activate` promises no ARIA state, so unlike `Toggle`,
    /// `Expand` and `Select` it has no postcondition to fail: a press that
    /// lands anywhere at all comes back `Applied`. That is only harmless while
    /// the press lands on the node that was named, and a press aims at the
    /// node's centre -- which for a container is generally occupied by one of
    /// its children. `Activate` on a paragraph of streamed prose, on a turn's
    /// group, or on the window root pressed whatever was painted in the middle
    /// of it, opened whatever that was, and reported success under the
    /// container's name.
    ///
    /// The line is the one the vocabulary already draws. A role that names a
    /// control, a row, a cell or a field is something a person clicks; a role
    /// that names a container, a heading, or an output region is not, and the
    /// thing they meant to click is a descendant with a name of its own. The
    /// refusal carries the role, so an agent that aimed at the parent can see
    /// what it actually named.
    #[must_use]
    pub const fn accepts_activation(self) -> bool {
        matches!(
            self,
            Self::Button
                | Self::Link
                | Self::Tab
                | Self::MenuItem
                | Self::Checkbox
                | Self::Radio
                | Self::Switch
                | Self::Slider
                | Self::Combobox
                | Self::Option
                | Self::Row
                | Self::Cell
                | Self::GridCell
                | Self::TreeItem
                | Self::Field
                | Self::Splitter
                | Self::Scrollbar
                | Self::Input
                | Self::MultilineInput
                | Self::PasswordInput
        )
    }
}
