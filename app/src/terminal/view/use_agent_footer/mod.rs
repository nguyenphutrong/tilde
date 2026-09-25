//! Shell-integration footer placement and legacy CLI input controls.

use warpui::clipboard::{ClipboardContent, ImageData};

use crate::terminal::cli_agent_sessions::{CLIAgentInputEntrypoint, CLIAgentSessionsModel};
use crate::util::image::{MAX_IMAGE_SIZE_BYTES_FOR_CLI_AGENT, MIME_SNIFF_BYTES, infer_mime_type};
mod warpify_footer;

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

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
use warp_errors::report_error;
pub(super) use warpify_footer::WarpifyFooterView;
use warpify_footer::WarpifyFooterViewEvent;
use warpui::r#async::Timer;
use warpui::{AppContext, SingletonEntity, TypedActionView, ViewContext};

use super::{RichContentInsertionPosition, TerminalAction, TerminalView};
use crate::server::telemetry::{CLIAgentType, CLISubagentControlState, TelemetryEvent};
use crate::settings::InputModeSettings;
pub use crate::terminal::CLIAgent;
use crate::terminal::TerminalModel;
use crate::terminal::cli_agent_sessions::CLIAgentRichInputCloseReason;
use crate::ui_components::blended_colors;
use crate::view_components::action_button::ActionButtonTheme;

/// Longer delay between clipboard image pastes (Ctrl+V) to CLI agents.
/// The CLI agent needs time to read from the system clipboard before
/// we overwrite it with the next image.
const CLI_AGENT_IMAGE_PASTE_DELAY: Duration = Duration::from_millis(300);

/// Bytes that simulate a "paste image from clipboard" keystroke for the
/// foreground CLI agent. `0x16` is `Ctrl+V` (SYN); on Windows Claude Code
/// listens for `Alt+V` (`ESC` + `'v'`) instead. Mirrored from the equivalent
/// branch in `TerminalView::paste`.
fn cli_agent_paste_keystroke_bytes() -> Vec<u8> {
    if cfg!(windows) {
        vec![0x1b, b'v']
    } else {
        vec![0x16]
    }
}

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

    /// Returns whether the active long-running command in this terminal is
    /// Warp's own headless TUI (`warp_tui`).
    pub(super) fn is_running_warp_tui(&self, model: &TerminalModel, ctx: &AppContext) -> bool {
        let active_block = model.block_list().active_block();
        if !active_block.is_active_and_long_running() {
            return false;
        }

        let command = active_block.command_with_secrets_obfuscated(false);
        let escape_char = self.active_block_session_id().and_then(|session_id| {
            self.sessions.read(ctx, |sessions, _| {
                sessions
                    .get(session_id)
                    .map(|session| session.shell_family().escape_char())
            })
        });
        CLIAgent::WarpTui.matches_command(&command, escape_char)
    }

    /// Updates the UI during a long running command to agent "tagged-in state".
    ///
    /// An agent may be "tagged in" during a _user-executed_ long running command, where being
    /// 'tagged in' means the input is visible and locked in agent mode, presumably awaiting user
    /// submission of a prompt for the agent to interact with the command.
    pub(super) fn tag_in_agent_for_user_long_running_command(
        &mut self,
        ctx: &mut ViewContext<Self>,
    ) {
        if self
            .model
            .lock()
            .block_list()
            .active_block()
            .is_agent_tagged_in()
            || !self
                .model
                .lock()
                .block_list()
                .active_block()
                .is_eligible_to_tag_in_agent()
        {
            return;
        }

        self.model
            .lock()
            .block_list_mut()
            .active_block_mut()
            .set_is_agent_tagged_in(true);

        if !self.model.lock().is_alt_screen_active() {
            self.warpify_footer.update(ctx, |footer, ctx| {
                footer.clear(ctx);
            });
            self.hide_warpify_footer_in_blocklist(ctx);
        }

        self.input.update(ctx, |input, ctx| {
            input.set_input_mode_agent(true, ctx);
            input.clear_buffer_and_reset_undo_stack(ctx);
        });
        ctx.notify();

        let model = self.model.lock();
        let active_block = model.block_list().active_block();
        let conversation_id = active_block.ai_conversation_id();
        let block_id = active_block.id().clone();
        send_telemetry_from_ctx!(
            TelemetryEvent::CLISubagentControlStateChanged {
                conversation_id,
                block_id,
                control_state: CLISubagentControlState::AgentTaggedIn,
            },
            ctx
        );
    }

    /// Tags the agent "out". See docs on `tag_in_agent_for_user_long_running_command` for
    /// 'tagged-in' semantics.
    ///
    /// Hides the agent input.
    pub(super) fn tag_out_agent_for_user_long_running_command(
        &mut self,
        ctx: &mut ViewContext<Self>,
    ) {
        if !self
            .model
            .lock()
            .block_list()
            .active_block()
            .is_agent_tagged_in()
        {
            return;
        }

        self.model
            .lock()
            .block_list_mut()
            .active_block_mut()
            .set_is_agent_tagged_in(false);

        if !self.model.lock().is_alt_screen_active() {
            self.maybe_show_warpify_footer_in_blocklist(ctx);
        }

        self.input.update(ctx, |input, ctx| {
            input.set_input_mode_terminal(false, ctx);
        });
        self.redetermine_terminal_focus(ctx);

        ctx.notify();

        let model = self.model.lock();
        let active_block = model.block_list().active_block();
        let conversation_id = active_block.ai_conversation_id();
        let block_id = active_block.id().clone();
        send_telemetry_from_ctx!(
            TelemetryEvent::CLISubagentControlStateChanged {
                conversation_id,
                block_id,
                control_state: CLISubagentControlState::AgentTaggedOut,
            },
            ctx
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

    /// Mirrors the CLI-agent Cmd+V image-paste path in `TerminalView::paste`
    /// for dropped image files: reads each file, writes its bytes to the
    /// system clipboard as image data, and sends the agent's paste keystroke
    /// to the PTY so the agent reads the image directly. This produces the
    /// same outcome as if the user had copied the image to their clipboard
    /// and pressed Cmd+V over the agent's TUI.
    pub(super) fn paste_dropped_images_to_cli_agent(
        &mut self,
        image_filepaths: Vec<String>,
        ctx: &mut ViewContext<Self>,
    ) {
        if image_filepaths.is_empty() {
            return;
        }
        let spawner = ctx.spawner();
        ctx.spawn(
            async move {
                for path_str in image_filepaths {
                    // Stat first so a multi-GB drop doesn't load into memory
                    // before we reject it. CLI agents handle their own
                    // compression, so the cap only exists to bound memory use.
                    match async_fs::metadata(&path_str).await {
                        Ok(meta) if (meta.len() as usize) > MAX_IMAGE_SIZE_BYTES_FOR_CLI_AGENT => {
                            let filename = Path::new(&path_str)
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| path_str.clone());
                            let limit_mb = MAX_IMAGE_SIZE_BYTES_FOR_CLI_AGENT / 1_000_000;
                            let msg = format!(
                                "{filename} is too large to send to the agent (limit {limit_mb}MB)."
                            );
                            let _ = spawner
                                .spawn(move |me, ctx| {
                                    me.show_error_toast(msg, ctx);
                                })
                                .await;
                            continue;
                        }
                        Ok(_) => {}
                        Err(e) => {
                            report_error!(
                                anyhow::Error::new(e).context("Failed to stat dropped image"),
                                extra: { "path" => %path_str }
                            );
                            continue;
                        }
                    }

                    let bytes = match async_fs::read(&path_str).await {
                        Ok(b) => b,
                        Err(e) => {
                            report_error!(
                                anyhow::Error::new(e).context("Failed to read dropped image"),
                                extra: { "path" => %path_str }
                            );
                            continue;
                        }
                    };
                    let path = Path::new(&path_str);
                    let filename = path.file_name().map(|n| n.to_string_lossy().into_owned());
                    let sniff_len = bytes.len().min(MIME_SNIFF_BYTES);
                    let mime_type = infer_mime_type(path, &bytes[..sniff_len]);

                    // Hop back to the view to write the clipboard + paste
                    // keystroke. Bail if the CLI agent session disappeared,
                    // OR if the agent's long-running block exited while we
                    // were reading off-thread — without that second check
                    // the paste byte would leak into the shell after the
                    // agent quit, since the session entry can outlive its
                    // foreground block.
                    let should_continue = spawner
                        .spawn(move |me, ctx| {
                            if !me.has_active_cli_agent_session(ctx) {
                                return false;
                            }
                            let still_long_running = me
                                .model
                                .lock()
                                .block_list()
                                .active_block()
                                .is_active_and_long_running();
                            if !still_long_running {
                                return false;
                            }
                            ctx.clipboard().write(ClipboardContent {
                                images: Some(vec![ImageData {
                                    data: bytes,
                                    mime_type,
                                    filename,
                                }]),
                                ..Default::default()
                            });
                            me.write_user_bytes_to_pty(cli_agent_paste_keystroke_bytes(), ctx);
                            true
                        })
                        .await;

                    if !matches!(should_continue, Ok(true)) {
                        return;
                    }

                    // Give the CLI agent time to read from the clipboard
                    // before we overwrite it with the next image.
                    Timer::after(CLI_AGENT_IMAGE_PASTE_DELAY).await;
                }
            },
            |_, _, _| {},
        );
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
