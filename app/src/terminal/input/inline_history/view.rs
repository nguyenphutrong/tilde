//! Inline shell history menu.
use std::collections::HashSet;

use warpui::elements::ChildView;
use warpui::{AppContext, Element, Entity, EntityId, ModelHandle, View, ViewContext, ViewHandle};

use crate::search::data_source::{Query, QueryFilter};
use crate::search::mixer::SearchMixer;
use crate::terminal::history::LinkedWorkflowData;
use crate::terminal::input::buffer_model::InputBufferModel;
use crate::terminal::input::inline_history::data_source::{
    AcceptHistoryItem, InlineHistoryMenuDataSource,
};
use crate::terminal::input::inline_menu::{
    InlineMenuEvent, InlineMenuModel, InlineMenuPositioner, InlineMenuView,
};
use crate::terminal::input::suggestions_mode_model::{
    InputSuggestionsModeEvent, InputSuggestionsModeModel,
};
use crate::terminal::model::session::active_session::ActiveSession;

#[derive(Debug, Clone)]
pub enum InlineHistoryMenuEvent {
    AcceptCommand {
        command: String,
        linked_workflow_data: Option<LinkedWorkflowData>,
    },
    SelectCommand {
        command: String,
        linked_workflow_data: Option<LinkedWorkflowData>,
    },
    NoResults,
    /// Close the menu and restore the original input buffer contents.
    Close,
}

pub struct InlineHistoryMenuView {
    menu_view: ViewHandle<InlineMenuView<AcceptHistoryItem>>,
    mixer: ModelHandle<SearchMixer<AcceptHistoryItem>>,
    model: ModelHandle<InlineMenuModel<AcceptHistoryItem>>,
    buffer_model: ModelHandle<InputBufferModel>,
}

impl InlineHistoryMenuView {
    pub fn new(
        terminal_view_id: EntityId,
        active_session: ModelHandle<ActiveSession>,
        input_suggestions_model: &ModelHandle<InputSuggestionsModeModel>,
        positioner: &ModelHandle<InlineMenuPositioner>,
        buffer_model: ModelHandle<InputBufferModel>,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let data_source =
            ctx.add_model(|_| InlineHistoryMenuDataSource::new(terminal_view_id, active_session));
        let mixer = ctx.add_model(|ctx| {
            let mut mixer = SearchMixer::<AcceptHistoryItem>::new();
            mixer.add_sync_source(data_source, [QueryFilter::Commands]);
            mixer.run_query(
                Query {
                    text: String::new(),
                    filters: HashSet::from([QueryFilter::Commands]),
                },
                ctx,
            );
            mixer
        });
        let menu_view = ctx.add_typed_action_view(|ctx| {
            InlineMenuView::new(
                mixer.clone(),
                positioner.clone(),
                input_suggestions_model,
                ctx,
            )
        });
        let model = menu_view.as_ref(ctx).model().clone();

        ctx.subscribe_to_model(input_suggestions_model, |me, model, event, ctx| {
            let InputSuggestionsModeEvent::ModeChanged { .. } = event;
            if model.as_ref(ctx).is_inline_history_menu() {
                me.open_with_current_buffer(ctx);
            }
        });
        ctx.subscribe_to_view(&menu_view, |_, _, event, ctx| match event {
            InlineMenuEvent::AcceptedItem { item, .. } => {
                ctx.emit(InlineHistoryMenuEvent::AcceptCommand {
                    command: item.command.clone(),
                    linked_workflow_data: item.linked_workflow_data.clone(),
                });
            }
            InlineMenuEvent::SelectedItem { item } => {
                ctx.emit(InlineHistoryMenuEvent::SelectCommand {
                    command: item.command.clone(),
                    linked_workflow_data: item.linked_workflow_data.clone(),
                });
            }
            InlineMenuEvent::Dismissed => ctx.emit(InlineHistoryMenuEvent::Close),
            InlineMenuEvent::NoResults => ctx.emit(InlineHistoryMenuEvent::NoResults),
            InlineMenuEvent::TabChanged => {}
        });
        Self {
            menu_view,
            mixer,
            model,
            buffer_model,
        }
    }

    pub fn model(&self) -> &ModelHandle<InlineMenuModel<AcceptHistoryItem>> {
        &self.model
    }

    pub fn render_results_only(&self, app: &AppContext) -> Box<dyn Element> {
        self.menu_view
            .as_ref(app)
            .render_results_only(false, 0., app)
    }

    pub fn result_count(&self, app: &AppContext) -> usize {
        self.menu_view.as_ref(app).result_count()
    }

    pub fn select_up(&self, ctx: &mut ViewContext<Self>) {
        self.menu_view.update(ctx, |v, ctx| v.select_up(ctx));
    }

    pub fn select_down(&self, ctx: &mut ViewContext<Self>) {
        let should_close = self.menu_view.read(ctx, |v, _| {
            let result_count = v.result_count();
            let is_last_item_selected =
                result_count > 0 && v.selected_idx().is_some_and(|idx| idx == result_count - 1);
            is_last_item_selected || result_count == 0
        });
        if should_close {
            ctx.emit(InlineHistoryMenuEvent::Close);
        } else {
            self.menu_view.update(ctx, |v, ctx| v.select_down(ctx));
        }
    }

    pub fn accept_selected_item(&self, ctx: &mut ViewContext<Self>) {
        self.menu_view
            .update(ctx, |v, ctx| v.accept_selected_item(false, ctx));
    }

    fn open_with_current_buffer(&mut self, ctx: &mut ViewContext<Self>) {
        let text = self.buffer_model.as_ref(ctx).current_value().to_owned();
        self.mixer.update(ctx, |mixer, ctx| {
            mixer.run_query(
                Query {
                    text,
                    filters: HashSet::from([QueryFilter::Commands]),
                },
                ctx,
            );
        });
    }
}

impl View for InlineHistoryMenuView {
    fn ui_name() -> &'static str {
        "InlineHistoryMenuView"
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        ChildView::new(&self.menu_view).finish()
    }
}

impl Entity for InlineHistoryMenuView {
    type Event = InlineHistoryMenuEvent;
}
