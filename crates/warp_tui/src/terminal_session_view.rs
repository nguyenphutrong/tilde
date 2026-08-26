use std::borrow::Cow;
use std::sync::Arc;

use async_channel::Sender;
use parking_lot::FairMutex;
use warp::tui_export::{
    PtyIntent, PtyIntentEvent, SizeInfo, SizeUpdate, TerminalModel, TerminalSurface,
    TerminalSurfaceInit,
};
use warpui_core::elements::tui::{TuiElement, TuiSize};
use warpui_core::{AppContext, Entity, TuiView, TypedActionView, ViewContext, keymap};

use crate::alt_screen_view::AltScreenElement;
use crate::terminal_content_element::TuiTerminalContentElement;

pub(crate) enum TuiTerminalSessionEvent {
    WriteUserInput(Cow<'static, [u8]>),
    Resize(SizeUpdate),
}

impl PtyIntentEvent for TuiTerminalSessionEvent {
    fn pty_intent(&self) -> Option<PtyIntent> {
        match self {
            Self::WriteUserInput(bytes) => Some(PtyIntent::WriteBytes(bytes.clone())),
            Self::Resize(size) => Some(PtyIntent::Resize(*size)),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum TuiTerminalSessionAction {
    ForwardUserPtyBytes(Vec<u8>),
}

pub(crate) struct TuiTerminalSessionView {
    terminal_model: Arc<FairMutex<TerminalModel>>,
    size_info: SizeInfo,
    terminal_resize_tx: Sender<TuiSize>,
}

pub(crate) fn init(_app: &mut AppContext) {}

impl TuiTerminalSessionView {
    pub(crate) fn new(surface_init: TerminalSurfaceInit, ctx: &mut ViewContext<Self>) -> Self {
        let TerminalSurfaceInit {
            model,
            wakeups_rx,
            size_info,
            ..
        } = surface_init;
        let (terminal_resize_tx, terminal_resize_rx) = async_channel::unbounded();
        ctx.spawn_stream_local(wakeups_rx, |_, _, ctx| ctx.notify(), |_, _| {});
        ctx.spawn_stream_local(
            terminal_resize_rx,
            |view, size, ctx| view.handle_terminal_resize(size, ctx),
            |_, _| {},
        );
        ctx.focus_self();
        Self {
            terminal_model: model,
            size_info,
            terminal_resize_tx,
        }
    }

    pub(crate) fn activate(&mut self, ctx: &mut ViewContext<Self>) {
        ctx.focus_self();
    }

    fn handle_terminal_resize(&mut self, size: TuiSize, ctx: &mut ViewContext<Self>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        let update = SizeUpdate::from_cell_dimensions(
            self.size_info,
            usize::from(size.height),
            usize::from(size.width),
        );
        if update.rows_or_columns_changed() {
            self.terminal_model.lock().resize(update);
            self.size_info = update.new_size();
            ctx.emit(TuiTerminalSessionEvent::Resize(update));
        }
    }
}

impl Entity for TuiTerminalSessionView {
    type Event = TuiTerminalSessionEvent;
}

impl TuiView for TuiTerminalSessionView {
    fn ui_name() -> &'static str {
        "TuiTerminalSessionView"
    }

    fn render(&self, _ctx: &AppContext) -> Box<dyn TuiElement> {
        TuiTerminalContentElement::new(
            self.terminal_resize_tx.clone(),
            AltScreenElement::new(self.terminal_model.clone()).finish(),
        )
        .with_pty_input(self.terminal_model.clone())
        .finish()
    }

    fn keymap_context(&self, _ctx: &AppContext) -> keymap::Context {
        let mut context = keymap::Context::default();
        context.set.insert(Self::ui_name());
        context
    }
}

impl TypedActionView for TuiTerminalSessionView {
    type Action = TuiTerminalSessionAction;

    fn handle_action(&mut self, action: &Self::Action, ctx: &mut ViewContext<Self>) {
        match action {
            TuiTerminalSessionAction::ForwardUserPtyBytes(bytes) => ctx.emit(
                TuiTerminalSessionEvent::WriteUserInput(Cow::Owned(bytes.clone())),
            ),
        }
    }
}

impl TerminalSurface for TuiTerminalSessionView {
    fn on_pty_spawn_failed(&mut self, error: anyhow::Error, ctx: &mut ViewContext<Self>) {
        log::error!("TUI PTY spawn failed: {error:#}");
        ctx.notify();
    }
}
