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
    /// The line is the one the vocabulary already draws, and it is drawn around
    /// controls: a role that names something a person presses -- a button, a
    /// link, a tab, a menu item, a checkbox, a field -- is an activation
    /// target. A role that names a container, a heading, an output region, or
    /// the *structure* of a table or a tree is not, and the thing they meant to
    /// press is a descendant with a name of its own. The refusal carries the
    /// role, so an agent that aimed at the parent can see what it actually
    /// named.
    ///
    /// `Row`, `Cell`, `GridCell` and `TreeItem` sit on the structural side even
    /// though ARIA counts some of them as widgets, because a press is aimed at
    /// the node's centre and a row's centre is whatever the row holds. Every
    /// row this repository publishes is a label rather than a control -- a
    /// marked code line in `agent_kit/code_view.rs` and an activity lane in
    /// `thread.rs`, neither with a click handler or a tab stop -- so admitting
    /// the role bought nothing and cost the guarantee: the hitbox preflight
    /// passed, the click went out, and `Activate` reported `Applied` under the
    /// row's name for a press that did nothing. A surface with a genuinely
    /// pressable row should publish the control it holds; the day one cannot,
    /// this gate wants an explicit capability on the node rather than a wider
    /// guess from the role.
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
                | Self::Field
                | Self::Splitter
                | Self::Scrollbar
                | Self::Input
                | Self::MultilineInput
                | Self::PasswordInput
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Role;

    /// The activation gate admits controls and refuses structure.
    ///
    /// `Row` is the one that has already been wrong once: it reads like a
    /// widget, and every row this repository publishes is a label. Pinning the
    /// four structural roles here keeps the next reader from restoring them
    /// from the ARIA taxonomy alone.
    #[test]
    fn only_the_roles_that_name_a_control_accept_activation() {
        for role in [
            Role::Button,
            Role::Link,
            Role::Tab,
            Role::MenuItem,
            Role::Checkbox,
            Role::Switch,
            Role::Combobox,
            Role::Input,
        ] {
            assert!(role.accepts_activation(), "{role:?} names a control");
        }
        for role in [
            Role::Row,
            Role::Cell,
            Role::GridCell,
            Role::TreeItem,
            Role::Window,
            Role::Region,
            Role::Group,
            Role::Text,
            Role::Heading,
        ] {
            assert!(
                !role.accepts_activation(),
                "{role:?} names structure, and a press aimed at it lands on a child"
            );
        }
    }
}
