//! Unit tests for [`BlocklistAIInputModel`] input handling.
//!
//! Covers the policy-driven mechanism where the initial config, the locked-AI gate, and
//! the reactive subscriptions all defer to the injected [`InputModePolicy`].

use std::rc::Rc;
use std::sync::Arc;

use parking_lot::FairMutex;
use warpui::r#async::executor::Background;
use warpui::{App, AppContext, ModelHandle};

use super::*;
use crate::ai::agent::conversation::AIConversationId;
use crate::ai::blocklist::agent_view::AgentViewEntryOrigin;
use crate::ai::blocklist::conversation_selection::{
    ConversationSelection, ConversationSelectionEvent, ConversationSelectionHandle,
    MockConversationSelection,
};
use crate::ai::blocklist::input_mode_policy::{InputModePolicy, PolicyConfigUpdate};
use crate::terminal::color::{self, Colors};
use crate::terminal::event_listener::ChannelEventListener;
use crate::terminal::model::TerminalModel;
use crate::terminal::model::test_utils::block_size;
use crate::test_util::settings::initialize_history_persistence_for_tests;

const AI_LOCKED: InputConfig = InputConfig {
    input_type: InputType::AI,
    is_locked: true,
};
const SHELL_LOCKED: InputConfig = InputConfig {
    input_type: InputType::Shell,
    is_locked: true,
};
const SHELL_UNLOCKED: InputConfig = InputConfig {
    input_type: InputType::Shell,
    is_locked: false,
};

/// Configurable [`InputModePolicy`] stub.
struct StubPolicy {
    initial: InputConfig,
    allows_locked_ai: bool,
    on_conversation_activated: Option<InputConfig>,
    on_conversation_deactivated: Option<InputConfig>,
}

impl StubPolicy {
    /// A policy with `initial` config that permits locked AI and never reacts.
    fn inert(initial: InputConfig) -> Self {
        Self {
            initial,
            allows_locked_ai: true,
            on_conversation_activated: None,
            on_conversation_deactivated: None,
        }
    }
}

impl InputModePolicy for StubPolicy {
    fn initial_config(&self) -> InputConfig {
        self.initial
    }

    fn allows_locked_ai_input(&self, _app: &AppContext) -> bool {
        self.allows_locked_ai
    }

    fn config_on_conversation_selection_changed(
        &self,
        event: &ConversationSelectionEvent,
        _current: InputConfig,
        _app: &AppContext,
    ) -> Option<PolicyConfigUpdate> {
        match event {
            ConversationSelectionEvent::Changed => None,
            ConversationSelectionEvent::Activated { .. } => {
                self.on_conversation_activated.map(PolicyConfigUpdate::new)
            }
            ConversationSelectionEvent::Deactivated { .. } => self
                .on_conversation_deactivated
                .map(PolicyConfigUpdate::new),
        }
    }
}

/// Builds an input model driven by `policy`, returning the conversation
/// selection handle so tests can emit selection events.
fn build_input_model(
    app: &mut App,
    policy: StubPolicy,
) -> (
    ModelHandle<BlocklistAIInputModel>,
    ConversationSelectionHandle,
) {
    initialize_history_persistence_for_tests(app);

    let terminal_model = Arc::new(FairMutex::new(TerminalModel::new_for_test(
        block_size(),
        color::List::from(&Colors::default()),
        ChannelEventListener::new_for_test(),
        Arc::new(Background::default()),
        false, /* should_show_bootstrap_block */
        None,  /* restored_blocks */
        false, /* honor_ps1 */
        false, /* is_inverted */
        None,  /* session_startup_path */
    )));
    let conversation_selection =
        app.add_model(|_| Box::new(MockConversationSelection) as Box<dyn ConversationSelection>);
    let input_model = app.add_model(|ctx| {
        BlocklistAIInputModel::new(
            terminal_model,
            conversation_selection.clone(),
            Rc::new(policy),
            ctx,
        )
    });
    (input_model, conversation_selection)
}

#[test]
fn initial_config_comes_from_policy() {
    App::test((), |mut app| async move {
        // A locked-AI initial config sticks — a TUI-style policy is not subject
        // to any GUI gating.
        let (input_model, _) = build_input_model(&mut app, StubPolicy::inert(AI_LOCKED));
        input_model.read(&app, |model, _| {
            assert_eq!(model.input_config(), AI_LOCKED);
        });
    });
}

#[test]
fn locked_ai_write_requires_policy_permission() {
    App::test((), |mut app| async move {
        let policy = StubPolicy {
            allows_locked_ai: false,
            ..StubPolicy::inert(SHELL_UNLOCKED)
        };
        let (input_model, _) = build_input_model(&mut app, policy);

        // Rejected: the policy forbids locking to AI.
        input_model.update(&mut app, |model, ctx| {
            model.set_input_config(AI_LOCKED, true, None, ctx);
        });
        input_model.read(&app, |model, _| {
            assert_eq!(model.input_config(), SHELL_UNLOCKED);
        });

        // Locked shell (and unlocked AI) writes are not gated.
        input_model.update(&mut app, |model, ctx| {
            model.set_input_config(SHELL_LOCKED, true, None, ctx);
        });
        input_model.read(&app, |model, _| {
            assert_eq!(model.input_config(), SHELL_LOCKED);
        });
    });
}

#[test]
fn conversation_events_with_inert_policy_leave_config_unchanged() {
    App::test((), |mut app| async move {
        let (input_model, conversation_selection) =
            build_input_model(&mut app, StubPolicy::inert(AI_LOCKED));

        conversation_selection.update(&mut app, |_, ctx| {
            ctx.emit(ConversationSelectionEvent::Activated {
                is_fullscreen: true,
                origin: AgentViewEntryOrigin::Cli,
            });
            ctx.emit(ConversationSelectionEvent::Deactivated {
                conversation_id: AIConversationId::new(),
                final_exchange_count: 0,
                is_exit_before_new_entrance: false,
            });
        });

        input_model.read(&app, |model, _| {
            assert_eq!(model.input_config(), AI_LOCKED);
        });
    });
}

#[test]
fn conversation_events_apply_policy_updates() {
    App::test((), |mut app| async move {
        let policy = StubPolicy {
            on_conversation_activated: Some(SHELL_LOCKED),
            on_conversation_deactivated: Some(AI_LOCKED),
            ..StubPolicy::inert(SHELL_UNLOCKED)
        };
        let (input_model, conversation_selection) = build_input_model(&mut app, policy);

        conversation_selection.update(&mut app, |_, ctx| {
            ctx.emit(ConversationSelectionEvent::Activated {
                is_fullscreen: false,
                origin: AgentViewEntryOrigin::Cli,
            });
        });
        input_model.read(&app, |model, _| {
            assert_eq!(model.input_config(), SHELL_LOCKED);
        });

        conversation_selection.update(&mut app, |_, ctx| {
            ctx.emit(ConversationSelectionEvent::Deactivated {
                conversation_id: AIConversationId::new(),
                final_exchange_count: 0,
                is_exit_before_new_entrance: false,
            });
        });
        input_model.read(&app, |model, _| {
            assert_eq!(model.input_config(), AI_LOCKED);
        });
    });
}

#[test]
fn submitting_shell_input_does_not_resume_autodetection() {
    App::test((), |mut app| async move {
        let (input_model, _) = build_input_model(&mut app, StubPolicy::inert(SHELL_UNLOCKED));
        input_model.update(&mut app, |model, ctx| {
            model.handle_input_buffer_submitted(ctx);
        });

        input_model.read(&app, |model, _| {
            assert_eq!(model.input_config(), SHELL_LOCKED);
        });
    });
}
