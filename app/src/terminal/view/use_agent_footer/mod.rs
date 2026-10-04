//! Shell-integration footer placement and legacy CLI input controls.

use crate::terminal::cli_agent_sessions::{CLIAgentInputEntrypoint, CLIAgentSessionsModel};
mod warpify_footer;

use std::sync::Arc;

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
use crate::settings::InputModeSettings;
use crate::terminal::TerminalModel;
use crate::terminal::cli_agent_sessions::CLIAgentRichInputCloseReason;
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

    /// Closes the CLI agent rich input session. Side effects (input config restore,
    /// buffer clear, hint text) are handled reactively by subscribers to
    /// `CLIAgentSessionsModelEvent::InputSessionChanged`.
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

        self.redetermine_terminal_focus(ctx);
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

        // Input mode switch, buffer clear, draft restoration, and hint text
        // are handled reactively by Input's subscription to InputSessionChanged.
        self.redetermine_terminal_focus(ctx);
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
