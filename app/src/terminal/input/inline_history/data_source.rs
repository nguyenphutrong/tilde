//! Shell history ordered oldest first, with current-session commands closest to the input.

use chrono::Local;
use ordered_float::OrderedFloat;
use warpui::{AppContext, Entity, EntityId, ModelHandle, SingletonEntity};

use crate::input_suggestions::HistoryInputSuggestion;
use crate::search::SyncDataSource;
use crate::search::data_source::{Query, QueryFilter, QueryResult};
use crate::search::mixer::DataSourceRunErrorWrapper;
use crate::terminal::history::{History, LinkedWorkflowData, UpArrowHistoryConfig};
use crate::terminal::input::inline_history::search_item::InlineHistoryItem;
use crate::terminal::input::inline_menu::{
    InlineMenuAction, InlineMenuClickBehavior, InlineMenuType,
};
use crate::terminal::model::session::active_session::ActiveSession;

#[derive(Clone, Debug)]
pub struct AcceptHistoryItem {
    pub command: String,
    pub linked_workflow_data: Option<LinkedWorkflowData>,
}

impl InlineMenuAction for AcceptHistoryItem {
    const MENU_TYPE: InlineMenuType = InlineMenuType::InlineHistoryMenu;

    fn click_behavior(&self) -> InlineMenuClickBehavior {
        InlineMenuClickBehavior::SelectOnClick
    }
}

pub struct InlineHistoryMenuDataSource {
    terminal_view_id: EntityId,
    active_session: ModelHandle<ActiveSession>,
}

impl InlineHistoryMenuDataSource {
    pub fn new(terminal_view_id: EntityId, active_session: ModelHandle<ActiveSession>) -> Self {
        Self {
            terminal_view_id,
            active_session,
        }
    }
}

impl SyncDataSource for InlineHistoryMenuDataSource {
    type Action = AcceptHistoryItem;

    fn run_query(
        &self,
        query: &Query,
        app: &AppContext,
    ) -> Result<Vec<QueryResult<Self::Action>>, DataSourceRunErrorWrapper> {
        if !query.filters.is_empty() && !query.filters.contains(&QueryFilter::Commands) {
            return Ok(Vec::new());
        }
        let trimmed_query = query.text.trim();
        let session_id = self
            .active_session
            .as_ref(app)
            .session(app)
            .map(|session| session.id());
        let suggestions = History::as_ref(app).up_arrow_suggestions_for_terminal_surface(
            self.terminal_view_id,
            session_id,
            UpArrowHistoryConfig {
                include_commands: true,
                include_prompts: false,
            },
            app,
        );
        let mut results = Vec::new();
        for suggestion in suggestions {
            let command = suggestion.normalized_text();
            if !command.starts_with(trimmed_query) {
                continue;
            }
            let HistoryInputSuggestion::Command { entry } = &suggestion else {
                continue;
            };
            let item = InlineHistoryItem::command(
                command.to_owned(),
                entry.linked_workflow_data(),
                entry.start_ts.unwrap_or_else(Local::now),
            )
            .with_prefix_match_len(trimmed_query.len())
            .with_score(OrderedFloat(results.len() as f64));
            results.push(QueryResult::from(item));
        }
        Ok(results)
    }
}

impl Entity for InlineHistoryMenuDataSource {
    type Event = ();
}

#[cfg(test)]
#[path = "data_source_tests.rs"]
mod tests;
