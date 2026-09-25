//! View-supplied policy for input-mode decisions.
//!
//! [`BlocklistAIInputModel`](super::BlocklistAIInputModel) is shared between
//! frontends (the GUI terminal input and the TUI prompt input), but several of
//! its decisions depend on view concepts the model cannot know about — e.g.
//! whether the surface distinguishes "fullscreen agent view" from "top-level
//! terminal", or whether locking the input to AI is allowed outside a
//! conversation. Each frontend supplies those answers via [`InputModePolicy`],
//! mirroring how [`ConversationSelection`](super::conversation_selection::ConversationSelection)
//! injects per-view selection semantics.
//!
//! The GUI implementation lives in
//! `super::agent_view::GuiInputModePolicy`; the TUI implementation lives in
//! `crates/warp_tui/src/input_mode_policy.rs`.

use std::rc::Rc;

use warpui::AppContext;

use super::conversation_selection::ConversationSelectionEvent;
use super::input_model::{InputConfig, InputTypeAutoDetectionSource};

/// A config write produced by an [`InputModePolicy`] decision.
pub struct PolicyConfigUpdate {
    /// The config to apply.
    pub config: InputConfig,
    /// The decision source recorded alongside the config.
    pub decision_source: Option<InputTypeAutoDetectionSource>,
}

impl PolicyConfigUpdate {
    /// An update with no decision source.
    pub fn new(config: InputConfig) -> Self {
        Self {
            config,
            decision_source: None,
        }
    }

    /// An update recorded with `decision_source`.
    pub fn with_source(config: InputConfig, decision_source: InputTypeAutoDetectionSource) -> Self {
        Self {
            config,
            decision_source: Some(decision_source),
        }
    }
}

/// Per-view policy consulted by [`BlocklistAIInputModel`](super::BlocklistAIInputModel)
/// for decisions it cannot make view-agnostically: lock gating and reactive
/// config transitions driven by conversation-selection events.
///
/// The reactive hooks receive the raw event and decide the config to apply,
/// so view-specific event payloads (fullscreen vs. inline, entry origins)
/// stay a concern of the implementing view.
pub trait InputModePolicy: 'static {
    /// The config the surface starts with.
    fn initial_config(&self) -> InputConfig;

    /// Whether the input may currently be locked to AI. When this returns
    /// `false`, `{AI, locked}` config writes are rejected.
    fn allows_locked_ai_input(&self, app: &AppContext) -> bool;

    /// The config to apply in response to a conversation-selection event, or
    /// `None` to leave the config unchanged.
    fn config_on_conversation_selection_changed(
        &self,
        event: &ConversationSelectionEvent,
        current: InputConfig,
        app: &AppContext,
    ) -> Option<PolicyConfigUpdate>;
}

/// Shared handle to a view-supplied [`InputModePolicy`].
pub type InputModePolicyHandle = Rc<dyn InputModePolicy>;
