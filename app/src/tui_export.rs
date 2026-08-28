//! Terminal APIs used by the standalone TUI frontend.

pub use crate::appearance::Appearance;
pub use crate::banner::BannerState;
pub use crate::persistence::PersistenceWriter;
pub use crate::terminal::alt_screen::{should_intercept_mouse, should_intercept_scroll};
pub use crate::terminal::color::List as TerminalColorList;
pub use crate::terminal::local_tty::{
    TerminalManager as LocalTtyTerminalManager, TerminalSurfaceInit, TerminalSurfaceResult,
};
pub use crate::terminal::model::blockgrid::BlockGrid;
pub use crate::terminal::model::escape_sequences::{KeystrokeWithDetails, ToEscapeSequence};
pub use crate::terminal::model::grid::grid_handler::{GridHandler, TermMode};
pub use crate::terminal::shared_session::IsSharedSessionCreator;
pub use crate::terminal::terminal_manager::BlockSpacing;
pub use crate::terminal::{
    BlockPadding, PtyIntent, PtyIntentEvent, SizeInfo, SizeUpdate,
    TerminalManager as TerminalManagerTrait, TerminalModel, TerminalSurface,
};
