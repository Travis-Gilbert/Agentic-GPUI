//! A command palette: a search field over a filtered list of commands, with
//! groups, Action keybinding hints and keyboard navigation.
//!
//! [`Command`] owns the entries and rendering policy; [`CommandState`] holds
//! interaction state such as the query, focus, selection, and scrolling.
#[allow(clippy::module_inception)]
mod command;
mod item;
mod state;

pub use command::Command;
pub use item::{CommandEntry, CommandGroup, CommandItem};
pub use state::CommandState;

pub(crate) use state::init;

pub use state::presentation::{CommandPresenter,CommandPresenterFactory,CommandPresentation,CommandRowPresentation,CommandRowKind,CommandInput,CommandInputRefusal};

mod structured_header;
pub use structured_header::{CommandHeader,CommandHeaderAction,CommandHeaderActionKind,CommandHeaderBuilder};
