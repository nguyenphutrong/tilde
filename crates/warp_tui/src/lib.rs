//! Local-shell-only headless TUI front-end for Warp.

mod alt_screen_view;
mod keybindings;
pub mod root_view;
pub mod session;
mod session_registry;
mod terminal_background;
mod terminal_block;
mod terminal_content_element;
mod terminal_session_view;

pub use root_view::RootTuiView;
pub use session::run;
