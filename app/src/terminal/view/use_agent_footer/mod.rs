//! Shell-integration footer placement and legacy CLI input controls.

use crate::terminal::cli_agent_sessions::{
    CLIAgentInputEntrypoint, CLIAgentInputState, CLIAgentSession, CLIAgentSessionContext,
    CLIAgentSessionStatus, CLIAgentSessionsModel,
};
mod warpify_footer;

use std::sync::Arc;
use std::time::Duration;

use async_io::Timer;
use parking_lot::FairMutex;
use pathfinder_color::ColorU;
use warp_core::features::FeatureFlag;
use warp_core::send_telemetry_from_ctx;
use warp_core::ui::appearance::Appearance;
use warp_core::ui::color::contrast::{
    MinimumAllowedContrast, high_enough_contrast, pick_best_foreground_color,
};
use warp_core::ui::theme::Fill as ThemeFill;
use warp_core::ui::theme::color::internal_colors;
pub(super) use warpify_footer::WarpifyFooterView;
use warpify_footer::WarpifyFooterViewEvent;
use warpui::{AppContext, SingletonEntity, TypedActionView, ViewContext};

use super::{RichContentInsertionPosition, TerminalAction, TerminalView};
use crate::server::telemetry::{CLIAgentType, TelemetryEvent};
use crate::settings::{AISettings, CompiledCommandsForCodingAgentToolbar, InputModeSettings};
use crate::terminal::cli_agent_sessions::CLIAgentRichInputCloseReason;
use crate::terminal::cli_agent_sessions::event::parse_event;
use crate::terminal::cli_agent_sessions::listener::{CLIAgentSessionListener, is_agent_supported};
use crate::terminal::model::escape_sequences::{BRACKETED_PASTE_END, BRACKETED_PASTE_START};
use crate::terminal::{CLIAgent, TerminalModel};
use crate::ui_components::blended_colors;
use crate::view_components::action_button::ActionButtonTheme;

impl TerminalView {
    pub(super) fn register_subscriptions_for_warpify_footer(
        &mut self,
        ctx: &mut ViewContext<Self>,
    ) {
        ctx.subscribe_to_view(&self.warpify_footer, |me, _, event, ctx| {
            me.hide_warpify_footer_in_blocklist(ctx);
            match event {
                WarpifyFooterViewEvent::Warpify => {
                    me.handle_action(&TerminalAction::TriggerSubshellBootstrap, ctx)
                }
                WarpifyFooterViewEvent::Dismiss => {}
            }
        });
        let input_mode_settings = InputModeSettings::handle(ctx);
        let mut was_pinned_to_top = input_mode_settings.as_ref(ctx).is_pinned_to_top();
        ctx.subscribe_to_model(&input_mode_settings, move |me, settings_handle, _, ctx| {
            let is_pinned_to_top = settings_handle.as_ref(ctx).is_pinned_to_top();
            if was_pinned_to_top != is_pinned_to_top {
                was_pinned_to_top = is_pinned_to_top;
                me.maybe_show_warpify_footer_in_blocklist(ctx);
            }
        });
    }

    pub(super) fn has_active_cli_agent_input_session(&self, app: &AppContext) -> bool {
        CLIAgentSessionsModel::as_ref(app).is_input_open(self.view_id)
    }

    pub(super) fn detect_cli_agent(&mut self, ctx: &mut ViewContext<Self>) {
        let command = {
            let model = self.model.lock();
            let block = model.block_list().active_block();
            if !block.is_active_and_long_running() {
                return;
            }
            block.command_with_secrets_obfuscated(false)
        };
        let agent = self.active_block_session_id().and_then(|id| {
            let session = self.sessions.as_ref(ctx).get(id)?;
            CLIAgent::detect(
                &command,
                Some(session.shell_family().escape_char()),
                Some(session.aliases()),
            )
        });
        let custom = agent.is_none();
        let Some(agent) =
            agent.or_else(|| CompiledCommandsForCodingAgentToolbar::matched_agent(ctx, &command))
        else {
            return;
        };
        if !agent.supports_cli_agent_footer()
            || CLIAgentSessionsModel::as_ref(ctx)
                .session(self.view_id)
                .is_some()
        {
            return;
        }
        let should_auto_toggle_input =
            *AISettings::as_ref(ctx).auto_open_rich_input_on_cli_agent_start;
        CLIAgentSessionsModel::handle(ctx).update(ctx, |sessions, ctx| {
            sessions.set_session(
                self.view_id,
                CLIAgentSession {
                    agent,
                    status: CLIAgentSessionStatus::InProgress,
                    session_context: CLIAgentSessionContext::default(),
                    input_state: CLIAgentInputState::Closed,
                    should_auto_toggle_input,
                    listener: None,
                    plugin_version: None,
                    remote_host: None,
                    draft_text: None,
                    custom_command_prefix: custom
                        .then(|| command.split_whitespace().next().map(str::to_owned))
                        .flatten(),
                    received_rich_notification: false,
                },
                ctx,
            );
        });
        if agent == CLIAgent::Codex {
            let events = self.model_events_handle.clone();
            let view_id = self.view_id;
            let listener =
                ctx.add_model(|ctx| CLIAgentSessionListener::new(view_id, agent, &events, ctx));
            CLIAgentSessionsModel::handle(ctx).update(ctx, |sessions, ctx| {
                sessions.register_listener(
                    view_id,
                    agent,
                    None,
                    None,
                    None,
                    None,
                    None,
                    should_auto_toggle_input,
                    listener,
                    ctx,
                );
            });
        }
        if should_auto_toggle_input {
            self.open_cli_agent_rich_input(CLIAgentInputEntrypoint::AutoShow, ctx);
        }
    }

    pub(super) fn handle_cli_agent_notification(
        &mut self,
        title: Option<&str>,
        body: &str,
        ctx: &mut ViewContext<Self>,
    ) {
        let Some(notification) = parse_event(title, body) else {
            return;
        };
        let agent = notification.agent;
        if !agent.supports_cli_agent_footer()
            || !is_agent_supported(&agent)
            || CLIAgentSessionsModel::as_ref(ctx)
                .session(self.view_id)
                .is_some_and(|s| s.listener.is_some())
        {
            return;
        }
        let view_id = self.view_id;
        let events = self.model_events_handle.clone();
        let listener =
            ctx.add_model(|ctx| CLIAgentSessionListener::new(view_id, agent, &events, ctx));
        let should_auto_toggle_input =
            *AISettings::as_ref(ctx).auto_open_rich_input_on_cli_agent_start;
        CLIAgentSessionsModel::handle(ctx).update(ctx, |sessions, ctx| {
            sessions.register_listener(
                view_id,
                agent,
                notification.cwd.clone(),
                notification.project.clone(),
                notification.session_id.clone(),
                notification.payload.plugin_version.clone(),
                None,
                should_auto_toggle_input,
                listener,
                ctx,
            );
            sessions.update_from_event(view_id, &notification, ctx);
        });
        if should_auto_toggle_input {
            self.open_cli_agent_rich_input(CLIAgentInputEntrypoint::AutoShow, ctx);
        }
    }

    pub(super) fn submit_cli_agent_input(&mut self, text: String, ctx: &mut ViewContext<Self>) {
        let Some(session) = CLIAgentSessionsModel::as_ref(ctx).session(self.view_id) else {
            return;
        };
        if text.trim().is_empty() || !self.has_active_cli_agent_input_session(ctx) {
            return;
        }
        let agent = session.agent;
        let block_id = self.model.lock().block_list().active_block().id().clone();
        let bytes = text.into_bytes();
        self.input.update(ctx, |input, ctx| {
            input.clear_buffer_and_reset_undo_stack(ctx)
        });
        if bytes.len() > 1 && matches!(bytes[0], b'!' | b'&') {
            // Claude-style mode prefixes need a separate input event before the prompt body.
            self.write_user_bytes_to_pty(vec![bytes[0]], ctx);
            ctx.spawn(
                Timer::after(Duration::from_millis(50)),
                move |me, _, ctx| {
                    if me.has_active_cli_agent_input_session(ctx)
                        && me.model.lock().block_list().active_block().id() == &block_id
                    {
                        me.write_cli_agent_text_then_submit(bytes[1..].to_vec(), agent, ctx);
                    }
                },
            );
        } else {
            self.write_cli_agent_text_then_submit(bytes, agent, ctx);
        }
    }

    fn write_cli_agent_text_then_submit(
        &mut self,
        mut bytes: Vec<u8>,
        agent: CLIAgent,
        ctx: &mut ViewContext<Self>,
    ) {
        let block_id = self.model.lock().block_list().active_block().id().clone();
        if matches!(
            agent,
            CLIAgent::Codex | CLIAgent::OhMyPi | CLIAgent::Hermes | CLIAgent::Copilot
        ) {
            bytes = [BRACKETED_PASTE_START, &bytes, BRACKETED_PASTE_END].concat();
        }
        self.write_user_bytes_to_pty(bytes, ctx);
        let delay = if agent == CLIAgent::Copilot { 300 } else { 50 };
        ctx.spawn(
            Timer::after(Duration::from_millis(delay)),
            move |me, _, ctx| {
                // A delayed Enter must never execute a shell command after the agent exits.
                if CLIAgentSessionsModel::as_ref(ctx)
                    .session(me.view_id)
                    .is_some_and(|s| s.agent == agent)
                    && me.model.lock().block_list().active_block().id() == &block_id
                {
                    me.write_user_bytes_to_pty(b"\r".to_vec(), ctx);
                    let settings = AISettings::as_ref(ctx);
                    let auto_toggle = *settings.auto_toggle_rich_input
                        && CLIAgentSessionsModel::as_ref(ctx)
                            .session(me.view_id)
                            .is_some_and(|s| {
                                s.supports_rich_status() && s.should_auto_toggle_input
                            });
                    if !auto_toggle && *settings.auto_dismiss_rich_input_after_submit {
                        me.close_cli_agent_rich_input(CLIAgentRichInputCloseReason::Submit, ctx);
                    }
                }
            },
        );
    }

    pub(super) fn maybe_show_warpify_footer_in_blocklist(&mut self, ctx: &mut ViewContext<Self>) {
        self.hide_warpify_footer_in_blocklist(ctx);
        if self.model.lock().is_alt_screen_active() || !self.warpify_footer.as_ref(ctx).is_active()
        {
            return;
        }
        self.insert_rich_content(
            None,
            self.warpify_footer.clone(),
            None,
            RichContentInsertionPosition::Append {
                insert_below_long_running_block: !InputModeSettings::as_ref(ctx).is_pinned_to_top(),
            },
            ctx,
        );
    }

    pub(super) fn hide_warpify_footer_in_blocklist(&mut self, ctx: &mut ViewContext<Self>) {
        let mut model = self.model.lock();
        let block_list = model.block_list_mut();
        block_list.remove_rich_content(self.warpify_footer.id());
        ctx.notify();
    }

    /// Closes the CLI agent rich input session. Subscribers to
    /// `CLIAgentSessionsModelEvent::InputSessionChanged` update the editor and focus.
    pub(in crate::terminal) fn close_cli_agent_rich_input(
        &mut self,
        reason: CLIAgentRichInputCloseReason,
        ctx: &mut ViewContext<Self>,
    ) {
        self.close_cli_agent_rich_input_impl(true, reason, ctx);
    }

    pub(in crate::terminal) fn close_cli_agent_rich_input_and_disable_auto_toggle(
        &mut self,
        ctx: &mut ViewContext<Self>,
    ) {
        self.close_cli_agent_rich_input_impl(false, CLIAgentRichInputCloseReason::Manual, ctx);
    }

    fn close_cli_agent_rich_input_impl(
        &mut self,
        should_auto_toggle_input: bool,
        reason: CLIAgentRichInputCloseReason,
        ctx: &mut ViewContext<Self>,
    ) {
        if !self.has_active_cli_agent_input_session(ctx) {
            return;
        }

        // Save the current buffer text as a draft before closing, so it can
        // be restored if the user reopens the composer.
        let draft = self.input.as_ref(ctx).buffer_text(ctx);
        let view_id = self.view_id;
        CLIAgentSessionsModel::handle(ctx).update(ctx, |sessions_model, ctx| {
            sessions_model.set_draft(view_id, draft);
            sessions_model.close_input(view_id, should_auto_toggle_input, ctx);
        });

        let cli_agent_type: Option<CLIAgentType> = CLIAgentSessionsModel::as_ref(ctx)
            .session(self.view_id)
            .map(|s| s.agent.into());
        if let Some(cli_agent) = cli_agent_type {
            send_telemetry_from_ctx!(
                TelemetryEvent::CLIAgentRichInputClosed { cli_agent, reason },
                ctx
            );
        }

        ctx.notify();
    }

    pub(in crate::terminal) fn open_cli_agent_rich_input(
        &mut self,
        entrypoint: CLIAgentInputEntrypoint,
        ctx: &mut ViewContext<Self>,
    ) {
        if !FeatureFlag::CLIAgentRichInput.is_enabled()
            || self.has_active_cli_agent_input_session(ctx)
        {
            return;
        }

        // The Ctrl-G binding and footer button are both gated on an active CLI
        // agent session, so the session should always exist here.
        let Some(cli_agent) = CLIAgentSessionsModel::as_ref(ctx)
            .session(self.view_id)
            .filter(|session| session.agent.supports_cli_agent_footer())
            .map(|session| session.agent)
        else {
            return;
        };

        let ai_input_model = self.ai_input_model.as_ref(ctx);
        let previous_input_config = ai_input_model.input_config();
        let previous_was_lock_set_with_empty_buffer =
            ai_input_model.was_lock_set_with_empty_buffer();

        let view_id = self.view_id;
        CLIAgentSessionsModel::handle(ctx).update(ctx, |sessions_model, ctx| {
            sessions_model.open_input(
                view_id,
                entrypoint,
                previous_input_config,
                previous_was_lock_set_with_empty_buffer,
                true,
                ctx,
            );
        });

        send_telemetry_from_ctx!(
            TelemetryEvent::CLIAgentRichInputOpened {
                cli_agent: cli_agent.into(),
                entrypoint,
            },
            ctx
        );

        ctx.notify();
    }
}

#[derive(Clone)]
pub(super) struct AgentFooterButtonTheme {
    /// When set, enables alt-screen contrast adjustment for text and border.
    terminal_model: Option<Arc<FairMutex<TerminalModel>>>,
}

impl AgentFooterButtonTheme {
    pub fn new(terminal_model: Option<Arc<FairMutex<TerminalModel>>>) -> Self {
        Self { terminal_model }
    }

    /// Returns the inferred background colour of the alt screen, if active.
    fn inferred_alt_screen_bg(&self) -> Option<ColorU> {
        let terminal_model = self.terminal_model.as_ref()?;
        let terminal_model = terminal_model.lock();
        terminal_model
            .is_alt_screen_active()
            .then(|| terminal_model.alt_screen().inferred_bg_color())
            .flatten()
    }

    /// Picks a colour that contrasts well against `bg`, choosing between two
    /// neutral candidates.
    fn contrast_adjusted_color(
        bg: ColorU,
        default: ColorU,
        contrast: MinimumAllowedContrast,
        appearance: &Appearance,
    ) -> ColorU {
        if high_enough_contrast(default, bg, contrast) {
            default
        } else {
            pick_best_foreground_color(
                bg,
                blended_colors::neutral_2(appearance.theme()),
                blended_colors::neutral_6(appearance.theme()),
                contrast,
            )
        }
    }
}

impl ActionButtonTheme for AgentFooterButtonTheme {
    fn background(&self, hovered: bool, appearance: &Appearance) -> Option<ThemeFill> {
        if hovered {
            Some(internal_colors::fg_overlay_2(appearance.theme()))
        } else {
            None
        }
    }

    fn border(&self, appearance: &Appearance) -> Option<ColorU> {
        let color = appearance.theme().outline().into_solid();
        if let Some(bg) = self.inferred_alt_screen_bg() {
            return Some(Self::contrast_adjusted_color(
                bg,
                color,
                MinimumAllowedContrast::NonText,
                appearance,
            ));
        }
        Some(color)
    }

    fn text_color(
        &self,
        _hovered: bool,
        _background: Option<ThemeFill>,
        appearance: &Appearance,
    ) -> ColorU {
        let color = appearance
            .theme()
            .sub_text_color(appearance.theme().surface_1())
            .into_solid();

        // If rendered in the alt screen, the footer is rendered with the inferred background color
        // of the alt screen output grid (if there is one). In such cases, we have to ensure that
        // the text within the footer is high-contrast enough to be legible, since the background
        // color can essentially be anything.
        if let Some(bg) = self.inferred_alt_screen_bg() {
            return Self::contrast_adjusted_color(
                bg,
                color,
                MinimumAllowedContrast::Text,
                appearance,
            );
        }
        color
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
