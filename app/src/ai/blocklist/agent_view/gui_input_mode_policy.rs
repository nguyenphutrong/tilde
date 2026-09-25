//! GUI implementation of [`InputModePolicy`].

use warp_core::features::FeatureFlag;
use warpui::{AppContext, EntityId, ModelHandle, SingletonEntity};

use super::super::conversation_selection::ConversationSelectionEvent;
use super::super::input_mode_policy::{InputModePolicy, PolicyConfigUpdate};
use super::super::input_model::{InputConfig, InputType, InputTypeAutoDetectionSource};
use super::super::{BlocklistAIContextModel, ConversationSelectionHandle};
use super::AgentViewEntryOrigin;
use crate::terminal::cli_agent_sessions::CLIAgentSessionsModel;

/// GUI input-mode policy. AI input may only be locked inside an agent view or an open
/// CLI-agent rich input session when `FeatureFlag::AgentView` is enabled.
pub(crate) struct GuiInputModePolicy {
    conversation_selection: ConversationSelectionHandle,
    ai_context_model: ModelHandle<BlocklistAIContextModel>,
    terminal_surface_id: EntityId,
}

impl GuiInputModePolicy {
    /// Creates the GUI policy for a terminal surface.
    pub(crate) fn new(
        conversation_selection: ConversationSelectionHandle,
        ai_context_model: ModelHandle<BlocklistAIContextModel>,
        terminal_surface_id: EntityId,
    ) -> Self {
        Self {
            conversation_selection,
            ai_context_model,
            terminal_surface_id,
        }
    }
}

impl InputModePolicy for GuiInputModePolicy {
    fn initial_config(&self) -> InputConfig {
        InputConfig::default()
    }

    fn allows_locked_ai_input(&self, app: &AppContext) -> bool {
        // CLI agent rich input uses AI mode to suppress shell decorations.
        !FeatureFlag::AgentView.is_enabled()
            || self
                .conversation_selection
                .as_ref(app)
                .is_conversation_active(app)
            || CLIAgentSessionsModel::as_ref(app).is_input_open(self.terminal_surface_id)
    }

    fn config_on_conversation_selection_changed(
        &self,
        event: &ConversationSelectionEvent,
        current: InputConfig,
        app: &AppContext,
    ) -> Option<PolicyConfigUpdate> {
        match event {
            ConversationSelectionEvent::Changed => None,
            ConversationSelectionEvent::Activated {
                is_fullscreen,
                origin,
            } => {
                if !is_fullscreen {
                    Some(PolicyConfigUpdate::with_source(
                        InputConfig {
                            input_type: InputType::AI,
                            is_locked: true,
                        },
                        InputTypeAutoDetectionSource::InlineAgentViewEntry,
                    ))
                } else if matches!(origin, AgentViewEntryOrigin::ClearBuffer) {
                    Some(PolicyConfigUpdate::new(InputConfig {
                        input_type: current.input_type,
                        is_locked: true,
                    }))
                } else if self.ai_context_model.as_ref(app).has_locking_attachment() {
                    Some(PolicyConfigUpdate::with_source(
                        InputConfig {
                            input_type: InputType::AI,
                            is_locked: true,
                        },
                        InputTypeAutoDetectionSource::AttachmentForcedAi,
                    ))
                } else {
                    Some(PolicyConfigUpdate {
                        config: InputConfig {
                            input_type: InputType::AI,
                            is_locked: true,
                        },
                        decision_source: None,
                    })
                }
            }
            ConversationSelectionEvent::Deactivated {
                is_exit_before_new_entrance,
                ..
            } => {
                if *is_exit_before_new_entrance {
                    return None;
                }
                Some(PolicyConfigUpdate::new(InputConfig::default()))
            }
        }
    }
}
