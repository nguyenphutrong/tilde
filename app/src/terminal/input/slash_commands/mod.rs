mod data_source;
mod mixer;
mod search_item;

#[cfg(feature = "local_fs")]
use std::path::PathBuf;

use ai::skills::SkillReference;
pub use data_source::*;
pub use mixer::{SlashCommandMixer, build_slash_command_mixer, slash_command_query};
#[cfg(not(target_family = "wasm"))]
use warp_cli::agent::Harness;
use warp_core::send_telemetry_from_ctx;
#[cfg(feature = "local_fs")]
use warp_util::path::{CleanPathResult, LineAndColumnArg};
use warpui::{AppContext, SingletonEntity, ViewContext};

use crate::TelemetryEvent;
use crate::ai::agent::conversation::AIConversationId;
#[cfg(not(target_family = "wasm"))]
use crate::ai::agent_conversations_model::AgentConversationsModel;
use crate::ai::blocklist::BlocklistAIHistoryModel;
use crate::cloud_object::model::persistence::CloudModel;
use crate::search::slash_command_menu::static_commands::SlashCommandKind;
#[cfg(not(target_family = "wasm"))]
use crate::search::slash_command_menu::static_commands::commands;
use crate::search::slash_command_menu::{SlashCommandId, StaticCommand};
use crate::server::ids::SyncId;
use crate::server::telemetry::SlashCommandAcceptedDetails;
use crate::terminal::input::inline_menu::{InlineMenuAction, InlineMenuType};
use crate::terminal::input::{Event, Input};
#[cfg(feature = "local_fs")]
use crate::terminal::model::session::Session;
use crate::terminal::view::TerminalAction;
use crate::view_components::DismissibleToast;
use crate::workflows::command_parser::compute_workflow_display_data;
use crate::workspace::ToastStack;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcceptSlashCommandOrSavedPrompt {
    SlashCommand {
        id: SlashCommandId,
    },
    SavedPrompt {
        id: SyncId,
    },
    /// A skill selected from browse or search. Contains name (for display/insertion) and path/bundled_skill_id (for execution).
    Skill {
        reference: SkillReference,
        name: String,
    },
}
impl InlineMenuAction for AcceptSlashCommandOrSavedPrompt {
    const MENU_TYPE: InlineMenuType = InlineMenuType::SlashCommands;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCommandSelectionBehavior {
    InsertCommandText(String),
    Execute,
}

/// Menu-selection policy for static slash commands in the TUI.
pub fn slash_command_selection_behavior(command: &StaticCommand) -> SlashCommandSelectionBehavior {
    if command
        .argument
        .as_ref()
        .is_some_and(|argument| !argument.should_execute_on_selection)
    {
        SlashCommandSelectionBehavior::InsertCommandText(format!("{} ", command.name))
    } else {
        SlashCommandSelectionBehavior::Execute
    }
}

/// Whether an already-open slash command menu should close after the input becomes an exact
/// static-command or skill match.
///
/// This preserves the GUI's existing behavior: exact input stays visible while multiple prior
/// results remain, but a unique match or the start of argument entry closes the menu.
pub fn should_close_slash_command_menu_for_exact_match(
    result_count: usize,
    argument_started: bool,
) -> bool {
    result_count < 2 || argument_started
}

/// Records a static slash command accepted from either the GUI or TUI surface.
pub fn record_static_slash_command_accepted(
    command_name: &str,
    is_in_agent_view: bool,
    ctx: &mut AppContext,
) {
    send_telemetry_from_ctx!(
        TelemetryEvent::SlashCommandAccepted {
            command_details: SlashCommandAcceptedDetails::StaticCommand {
                command_name: command_name.to_owned(),
            },
            is_in_agent_view,
        },
        ctx
    );
}

/// Records a saved prompt accepted from either the GUI or TUI slash menu.
pub fn record_saved_prompt_accepted(is_in_agent_view: bool, ctx: &mut AppContext) {
    send_telemetry_from_ctx!(
        TelemetryEvent::SlashCommandAccepted {
            command_details: SlashCommandAcceptedDetails::SavedPrompt,
            is_in_agent_view,
        },
        ctx
    );
}

pub fn saved_prompt_text_for_id(id: &SyncId, ctx: &AppContext) -> Option<String> {
    let workflow = CloudModel::as_ref(ctx).get_workflow(id)?;
    workflow.model().data.is_agent_mode_workflow().then(|| {
        compute_workflow_display_data(&workflow.model().data).command_with_replaced_arguments
    })
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SlashCommandTrigger {
    Input { cmd_or_ctrl_enter: bool },
    Keybinding,
}

impl SlashCommandTrigger {
    fn cmd_or_ctrl_enter() -> Self {
        Self::Input {
            cmd_or_ctrl_enter: true,
        }
    }

    pub fn input() -> Self {
        Self::Input {
            cmd_or_ctrl_enter: false,
        }
    }

    pub(super) fn keybinding() -> Self {
        Self::Keybinding
    }

    pub fn is_keybinding(&self) -> bool {
        matches!(self, Self::Keybinding)
    }

    fn is_cmd_or_ctrl_enter(&self) -> bool {
        matches!(
            self,
            Self::Input {
                cmd_or_ctrl_enter: true
            }
        )
    }
}

#[cfg(feature = "local_fs")]
fn open_file_command_path(
    session: &Session,
    current_dir: &str,
    raw_arg: &str,
) -> (PathBuf, Option<LineAndColumnArg>) {
    let parsed_path = CleanPathResult::with_line_and_column_number(raw_arg.trim());
    // The argument may contain shell-escaped characters (e.g. `\ ` for spaces) from auto-suggest.
    // Unescape them so the path matches the actual filesystem entry.
    let unescaped_path = session.shell_family().unescape(&parsed_path.path);
    // Expand `~` to the user's home directory.
    let expanded_path = shellexpand::tilde(&unescaped_path);

    let shell_path = session
        .convert_directory_to_typed_path_buf(current_dir.to_owned())
        .join(session.convert_directory_to_typed_path_buf(expanded_path.into_owned()))
        .normalize();
    let file_path = session
        .maybe_convert_to_native_path(&shell_path.to_path())
        .unwrap_or_else(|err| {
            log::warn!("unable to convert /open-file path to native path: {err:?}");
            PathBuf::from(shell_path.to_string_lossy().into_owned())
        });

    (file_path, parsed_path.line_and_column_num)
}

impl Input {
    pub(super) fn maybe_open_local_file(&mut self, ctx: &mut ViewContext<Self>) -> bool {
        let text = self.buffer_text(ctx);
        let text = text.trim();
        let (command, argument) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
        if command != "/open" && command != "/open-file" {
            return false;
        }
        #[cfg(feature = "local_fs")]
        {
            let argument = argument.trim();
            if argument.is_empty() {
                ctx.emit(Event::OpenFilesPalette {
                    source: crate::server::telemetry::PaletteSource::Keybinding,
                });
            } else {
                let Some(session_id) = self.active_block_session_id() else {
                    return false;
                };
                let Some(session) = self.sessions.as_ref(ctx).get(session_id) else {
                    return false;
                };
                if !session.is_local() {
                    return false;
                }
                let Some(current_dir) = self
                    .active_block_metadata
                    .as_ref()
                    .and_then(|metadata| metadata.current_working_directory())
                else {
                    return false;
                };
                let (path, line_col) = open_file_command_path(&session, current_dir, argument);
                match std::fs::metadata(&path) {
                    Ok(metadata) if metadata.is_file() => {
                        ctx.dispatch_typed_action(&TerminalAction::OpenCodeInWarp {
                            path,
                            layout: crate::util::file::external_editor::settings::EditorLayout::SplitPane,
                            line_col,
                        });
                    }
                    result => {
                        let message = if result.is_ok() {
                            "The /open-file command only works for files, not directories"
                                .to_owned()
                        } else {
                            format!("File not found: {}", path.display())
                        };
                        let window_id = ctx.window_id();
                        ToastStack::handle(ctx).update(ctx, |stack, ctx| {
                            stack.add_ephemeral_toast(
                                DismissibleToast::error(message),
                                window_id,
                                ctx,
                            );
                        });
                        return true;
                    }
                }
            }
            self.clear_buffer_and_reset_undo_stack(ctx);
            true
        }
        #[cfg(not(feature = "local_fs"))]
        {
            false
        }
    }
}

/// Whether executing the static slash `command` submits its text to the conversation as an AI
/// prompt (handled downstream like a normal user query) rather than performing an immediate
/// local action.
///
/// Only `/compact`, `/plan`, and `/orchestrate` are sent as prompts.
/// Every other slash command emits an immediate action (forking, switching model, opening a
/// menu, etc.), so callers gating prompt queuing or shared-session forwarding should treat those
/// as "run now".
pub fn slash_command_is_submitted_as_prompt(command: &StaticCommand) -> bool {
    matches!(
        command.kind,
        SlashCommandKind::Compact | SlashCommandKind::Plan | SlashCommandKind::Orchestrate
    )
}

/// Returns true when the conversation with `conversation_id` is associated with an Oz
/// `AmbientAgentTask`. Callers deciding between `/fork` and `/continue-locally` should also
/// check the same `CLOUD_AGENT` context that gates `/continue-locally`.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn conversation_is_cloud_oz_for_slash_command(
    conversation_id: AIConversationId,
    ctx: &AppContext,
) -> bool {
    let history = BlocklistAIHistoryModel::as_ref(ctx);
    let Some(conversation) = history.conversation(&conversation_id) else {
        return false;
    };
    let Some(task_id) = conversation.task_id() else {
        return false;
    };

    let Some(task) = AgentConversationsModel::as_ref(ctx).get_task_data(&task_id) else {
        // Permissive: not yet fetched. Matches the data-source default so the command isn't
        // wrongly blocked while the task fetch is in flight.
        return true;
    };

    match task
        .agent_config_snapshot
        .as_ref()
        .and_then(|s| s.harness.as_ref())
    {
        Some(config) => config.harness_type == Harness::Oz,
        None => true,
    }
}

/// Tooltip and slash command name for the fork button, returned as a unit so
/// callers rendering the button and callers inserting the command always agree.
#[cfg(not(target_family = "wasm"))]
pub(crate) struct ForkButtonAction {
    pub tooltip: &'static str,
    pub command_name: &'static str,
}

/// Returns the tooltip and slash command for the fork button given an optional
/// conversation ID. Uses `/continue-locally` for Oz conversations when `/fork`
/// is unavailable in the current cloud-agent context, and `/fork` otherwise.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn fork_button_action(
    conversation_id: Option<AIConversationId>,
    is_cloud_agent_context: bool,
    ctx: &AppContext,
) -> ForkButtonAction {
    if is_cloud_agent_context
        && conversation_id.is_some_and(|id| conversation_is_cloud_oz_for_slash_command(id, ctx))
    {
        ForkButtonAction {
            tooltip: "Continue locally",
            command_name: commands::CONTINUE_LOCALLY.name,
        }
    } else {
        ForkButtonAction {
            tooltip: "Fork conversation",
            command_name: commands::FORK.name,
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
