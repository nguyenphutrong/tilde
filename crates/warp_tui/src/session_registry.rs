use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;

use pathfinder_geometry::vector::Vector2F;
use warp::tui_export::{
    BannerState, BlockPadding, BlockSpacing, IsSharedSessionCreator, LocalTtyTerminalManager,
    PersistenceWriter, TerminalManagerTrait, TerminalSurfaceResult,
};
use warpui::SingletonEntity;
use warpui_core::runtime::TuiDriverHandle;
use warpui_core::{AppContext, Entity, ModelHandle, ViewHandle, WindowId};

use crate::terminal_session_view::TuiTerminalSessionView;

#[derive(Clone)]
pub(crate) enum TuiSessionView {
    Terminal(ViewHandle<TuiTerminalSessionView>),
}

impl TuiSessionView {
    pub(crate) fn id(&self) -> warpui_core::EntityId {
        match self {
            Self::Terminal(view) => view.id(),
        }
    }
}

pub(crate) struct TuiSession {
    view: TuiSessionView,
    _manager: ModelHandle<Box<dyn TerminalManagerTrait>>,
}

impl TuiSession {
    pub(crate) fn view(&self) -> &TuiSessionView {
        &self.view
    }
}

pub(crate) struct TuiSessions {
    _driver: TuiDriverHandle,
    sessions: Vec<TuiSession>,
}

impl Entity for TuiSessions {
    type Event = ();
}

impl SingletonEntity for TuiSessions {}

impl TuiSessions {
    pub(crate) fn new(driver: TuiDriverHandle) -> Self {
        Self {
            _driver: driver,
            sessions: Vec::new(),
        }
    }

    pub(crate) fn focused_session(&self) -> Option<&TuiSession> {
        self.sessions.last()
    }

    pub(crate) fn create_local_terminal_session(
        sessions: &ModelHandle<Self>,
        window_id: WindowId,
        startup_directory: Option<PathBuf>,
        ctx: &mut AppContext,
    ) -> ViewHandle<TuiTerminalSessionView> {
        let banner = ctx.add_model(|_| BannerState::default());
        let model_event_sender = PersistenceWriter::as_ref(ctx).sender();
        let manager = LocalTtyTerminalManager::<TuiTerminalSessionView>::create_tui_model(
            startup_directory,
            HashMap::<OsString, OsString>::from_iter(std::env::vars_os()),
            IsSharedSessionCreator::No,
            None,
            banner,
            Vector2F::new(120., 24.),
            model_event_sender,
            None,
            BlockSpacing {
                block_padding: BlockPadding::new(0., 0., 0., 0.),
                warp_prompt_height_lines: 0.,
                show_memory_stats: false,
            },
            ctx,
            move |surface_init, ctx| {
                let surface = ctx.add_typed_action_tui_view(window_id, |ctx| {
                    TuiTerminalSessionView::new(surface_init, ctx)
                });
                TerminalSurfaceResult {
                    surface,
                    post_wire: move |_: &mut LocalTtyTerminalManager<TuiTerminalSessionView>,
                                     _: &ViewHandle<TuiTerminalSessionView>,
                                     _: &mut AppContext| {},
                }
            },
        );
        let surface = manager.surface.clone();
        sessions.update(ctx, |sessions, _| {
            sessions.sessions.push(TuiSession {
                view: TuiSessionView::Terminal(manager.surface),
                _manager: manager.manager,
            });
        });
        surface
    }
}
