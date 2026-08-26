//! [`RootTuiView`]: the local-terminal root view of the `warp-tui` front-end.

use warpui::SingletonEntity as _;
use warpui_core::elements::tui::{TuiChildView, TuiElement, TuiText};
use warpui_core::{AppContext, Entity, EntityId, TuiView, TypedActionView, ViewContext, keymap};

use crate::session_registry::{TuiSessionView, TuiSessions};

pub struct RootTuiView;

#[derive(Debug, Clone)]
pub enum RootTuiAction {}

impl RootTuiView {
    pub(crate) fn new(ctx: &mut ViewContext<Self>) -> Self {
        ctx.focus_self();
        Self
    }

    fn focused_session_view(ctx: &AppContext) -> Option<TuiSessionView> {
        ctx.has_singleton_model::<TuiSessions>()
            .then(|| TuiSessions::as_ref(ctx).focused_session())
            .flatten()
            .map(|session| session.view().clone())
    }
}

impl Entity for RootTuiView {
    type Event = ();
}

impl TuiView for RootTuiView {
    fn ui_name() -> &'static str {
        "RootTuiView"
    }

    fn child_view_ids(&self, ctx: &AppContext) -> Vec<EntityId> {
        Self::focused_session_view(ctx)
            .map(|view| vec![view.id()])
            .unwrap_or_default()
    }

    fn render(&self, ctx: &AppContext) -> Box<dyn TuiElement> {
        Self::focused_session_view(ctx)
            .map(|view| match view {
                TuiSessionView::Terminal(view) => TuiChildView::new(&view).finish(),
            })
            .unwrap_or_else(|| TuiText::new("Starting local terminal…").finish())
    }

    fn keymap_context(&self, _ctx: &AppContext) -> keymap::Context {
        let mut context = keymap::Context::default();
        context.set.insert("RootTuiView");
        context
    }
}

impl TypedActionView for RootTuiView {
    type Action = RootTuiAction;

    fn handle_action(&mut self, action: &RootTuiAction, _ctx: &mut ViewContext<Self>) {
        match *action {}
    }
}
