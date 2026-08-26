use warpui_core::AppContext;

pub(crate) fn init(app: &mut AppContext) {
    crate::terminal_session_view::init(app);
}
