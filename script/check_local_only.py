#!/usr/bin/env python3
"""Reject removed inference dependencies and ratchet remaining cloud source references."""

import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]
BASELINE = ROOT / "script/local_only_residue.json"
REMOVED = {
    "ort", "ort-sys", "candle-core", "candle-nn", "candle-onnx", "tokenizers",
    "serve-wasm", "managed_secrets_wasm",
    "minidumper", "crash-handler",
    "input_classifier", "natural_language_detection",
    "app-installation-detection", "voice_input",
}
RESIDUE = {
    "ai", "ai_types", "mcp",
    "cloud_objects", "cloud_object_client", "cloud_object_models",
    "cloud_object_persistence", "firebase", "warp_graphql", "warp_graphql_schema",
    "warp_server_auth", "warp_server_client", "warp_multi_agent_api",
    "warp_multi_agent_client", "remote_server", "computer_use",
    "oauth2", "cynic", "graphql-ws-client",
}
ENDPOINT = re.compile(
    r"(?:https?|wss?)://[^\s\"'<>]*"
    r"(?:\.warp\.dev|\.warp\.work|\.sentry\.io|\.firebaseio\.com|"
    r"identitytoolkit\.googleapis\.com|securetoken\.googleapis\.com)"
    r"|AIza[0-9A-Za-z_-]{35}",
    re.IGNORECASE,
)
SOURCE_SUFFIXES = {".rs", ".toml", ".sh", ".ps1", ".yml", ".yaml", ".json"}
REMOVED_SOURCES = (
    "crates/voice_input/",
    "app/src/voice/",
    "app/src/ai/voice/",
    "app/src/editor/view/voice.rs",
    "app/src/settings/tui_voice.rs",
    "app/src/settings/tui_voice_tests.rs",
    "crates/app-installation-detection/",
    "app/src/ai/get_relevant_files/",
    "app/src/ai/blocklist/action_model/execute/get_files.rs",
    "app/src/ai/blocklist/action_model/execute/search_codebase.rs",
    "app/src/ai/blocklist/inline_action/search_codebase.rs",
    "app/src/ai/blocklist/action_model/execute/start_agent.rs",
    "app/src/ai/blocklist/action_model/execute/run_agents.rs",
    "app/src/ai/blocklist/action_model/execute/run_agents_tests.rs",
    "app/src/ai/blocklist/inline_action/run_agents_card_view.rs",
    "app/src/ai/blocklist/inline_action/run_agents_card_view_tests.rs",
    "app/src/ai/document/plan_publication.rs",
    "app/src/ai/skills/resolve_skill_spec.rs",
    "app/src/ai/skills/resolve_skill_spec_tests.rs",
    "app/src/ai/conversation_details_panel.rs",
    "app/src/ai/conversation_details_panel_tests.rs",
    "app/src/ai/blocklist/block/status_bar.rs",
    "app/src/ai/blocklist/summarization_cancel_dialog.rs",
    "app/src/settings/cloud_preferences_syncer.rs",
    "app/src/settings/cloud_preferences_syncer_tests.rs",
    "app/src/server/cloud_objects/fake_object_client.rs",
    "app/src/terminal/view/init_project/",
    "app/src/terminal/view/inline_banner/agent_mode_setup.rs",
    "app/src/pane_group/pane/view/header/sharing.rs",
    "app/src/ai/blocklist/agent_view/zero_state_block.rs",
    "app/src/ai/blocklist/agent_view/zero_state_block_tests.rs",
    "app/src/terminal/input/plans/",
    "app/src/terminal/input/user_query/",
    "app/src/terminal/input/rewind/",
    "app/src/terminal/input/repos/",
    "app/src/terminal/input/skills/data_source.rs",
    "app/src/terminal/input/skills/view.rs",
    "app/src/terminal/input/slash_commands/view.rs",
    "app/src/terminal/input/slash_commands/cloud_mode_v2_view.rs",
    "app/src/terminal/shared_session/sharer/",
    "app/src/server/telemetry/context.rs",
    "app/src/server/telemetry/rudder_message.rs",
    "app/src/server/telemetry/mod_tests.rs",
    "app/src/server/telemetry_ext.rs",
    "app/src/server/telemetry_ext_tests.rs",
)
REMOVED_TELEMETRY_SYMBOLS = re.compile(
    r"\b(?:TelemetryApi|send_telemetry_sync_from_\w+|to_rudder_batch_message|"
    r"flush_telemetry_events|flush_persisted_events_to_rudder|persist_telemetry_events)\b"
    r"|\btelemetry_context\s*\("
)
REMOVED_SOURCE_SYMBOLS = {
    "app/src/lib.rs": ("app_installation_detection",),
    "app/src/tab.rs": ("BlocklistAIHistoryModel", "agent_indicator", "is_cloud_agent_session"),
    "app/src/terminal/input.rs": ("ActiveAIConversationHasHistory",),
    "app/src/workflows/categories.rs": (
        "CloudModel", "UserWorkspaces", "load_cloud_workflows", "PersonalCloud", "TelemetryEvent",
    ),
    "app/src/ai/mod.rs": ("get_relevant_files", "conversation_details_panel"),
    "app/src/server/server_api.rs": ("get_relevant_files", "GetRelevantFiles", "transcribe", "TranscribeError"),
    "app/src/editor/view/mod.rs": ("VoiceTranscriber", "VoiceTranscriptionOptions", "VoiceStateUpdated"),
    "app/src/editor/view/element.rs": ("voice_input_cursor_icon", "voice_input_toggle_key_code"),
    "app/src/ai/agent/api/impl.rs": ("SearchCodebase", "RunAgents"),
    "crates/ai/src/agent/action/mod.rs": (
        "SearchCodebase", "SearchCodebaseRequest", "RunAgentsRequest", "StartAgentExecutionMode",
    ),
    "crates/ai/src/agent/action/convert.rs": ("SearchCodebase",),
    "app/src/ai/blocklist/action_model.rs": (
        "GetRelevantFilesController", "SearchCodebaseExecutor", "StartAgentExecutor", "RunAgentsExecutor",
    ),
    "app/src/ai/blocklist/action_model/execute.rs": (
        "GetRelevantFilesController", "SearchCodebaseExecutor", "search_codebase",
        "StartAgentExecutor", "RunAgentsExecutor", "start_agent", "run_agents",
    ),
    "app/src/ai/blocklist/history_model.rs": ("NewConversationRequestComplete",),
    "app/src/ai/blocklist/block.rs": ("RunAgentsCardView", "run_agents_card_views"),
    "app/src/pane_group/pane/terminal_pane.rs": (
        "StartAgentConversation", "prepare_remote_child_launch", "prepare_local_oz_child_launch",
    ),
    "app/src/ai/blocklist/controller.rs": (
        "OrchestrationEventService", "OrchestrationEventStreamer",
        "conversation_ready_for_pending_events", "inject_pending_events_for_request",
        "handle_pending_events_ready", "drain_events_for_request", "requeue_awaiting_events",
    ),
    "app/src/ai/blocklist/block/cli_controller.rs": (
        "BlocklistAIHistoryEvent", "handle_history_model_event",
    ),
    "app/src/app_state.rs": ("ServerId",),
    "app/src/settings/mod.rs": ("cloud_preferences_syncer",),
    "app/src/settings/privacy.rs": ("CloudPreferencesSyncer", "maybe_sync_with_warp_drive_prefs"),
    "app/src/auth/auth_manager.rs": ("CloudPreferencesSyncer",),
    "app/src/ai/execution_profiles/profiles.rs": (
        "CloudPreferencesSyncer", "cloud_collection_awaiting_reconciliation",
        "sync_explicit_settings_collection",
    ),
    "crates/settings/src/lib.rs": ("should_sync_to_cloud",),
    "crates/warpui_extras/src/user_preferences/toml_backed.rs": ("file_content_hash",),
    "app/src/workspace/view.rs": (
        "team_uid_for_window", "notify_terminal_focus_change", "CloudPreferencesSettings",
        "TELEMETRY_FLAG", "SETTINGS_SYNC_FLAG", "AI_RULES_FLAG", "FILE_BASED_MCP_FLAG",
        "IS_CODEBASE_INDEXING_ENABLED", "IS_AUTOINDEXING_ENABLED",
        "transcript_details_panel", "transcript_info_button", "view_cloud_runs_button",
        "ToggleConversationTranscriptDetailsPanel",
    ),
    "app/src/workspace/view/wasm_view.rs": ("ConversationDetailsPanel", "AgentConversationsModel"),
    "app/src/terminal/view/init.rs": ("ToggleConversationDetailsPanel",),
    "app/src/terminal/local_tty/terminal_manager.rs": ("session_sharer",),
    "app/src/terminal/local_tty/terminal_view_adaptor.rs": (
        "session_sharer", "wire_up_terminal_view_session_sharing", "start_sharing_session",
        "BlocklistAIHistoryModel", "LLMPreferences", "CLIAgentSessionsModel", "NetworkStatus",
    ),
    "app/src/terminal/view.rs": (
        "update_focused_terminal_info", "mcp_execution_path", "PersistedWorkspace",
        "InitProjectModel", "maybe_set_pending_repo_init_path", "start_lsp_server_in_active_pwd",
        "needs_git_status_for_agent_context", "needs_pr_info_for_agent_context",
        "should_hide_cli_agent_cursor_cell", "with_hide_cursor_cell",
        "is_using_conversation_for_pane_header_title", "active_session_remote_host",
        "GetRelevantFilesController", "get_relevant_files_controller",
        "StartAgentConversation", "StartAgentExecutor", "StartAgentRequest",
        "conversation_details_panel", "ToggleConversationDetailsPanel",
        "handle_ai_history_model_event", "render_owner_for_ai_history_event",
        "last_observed_conversation_status", "read_image_files_from_paths",
        "apply_shared_session_agent_interaction_state", "tag_into_active_block",
    ),
    "app/src/terminal/view/action.rs": (
        "SetInputModeAgent", "SetInputModeTerminal",
    ),
    "app/src/pane_group/mod.rs": ("transitively_share_existing_local_children",),
    "app/src/terminal/view/pane_impl.rs": (
        "CLIAgentSessionsModel", "selected_cli_agent_title_for_chrome",
        "is_using_conversation_for_pane_header_title", "terminal_view_agent_icon_variant",
        "shared_session_indicator_color", "PANE_HEADER_AGENT_SIZE",
    ),
    "app/src/context_chips/display.rs": ("terminal_view_id",),
    "app/src/context_chips/display_chip.rs": (
        "terminal_view_id", "CLIAgentSessionsModel", "is_cli_agent_session_active",
    ),
    **{f"app/src/terminal/{path}.rs": ("hide_cursor_cell", "with_hide_cursor_cell") for path in (
        "grid_renderer", "blockgrid_renderer", "blockgrid_element", "block_list_element",
        "alt_screen/alt_screen_element", "share_block_modal",
    )},
}


def removed_dependency(name):
    return name in REMOVED or name.startswith(("opentelemetry", "tracing-opentelemetry", "sentry"))


def restricted(name):
    return name in RESIDUE


def dependencies(value):
    for key, item in value.items():
        if key in {"dependencies", "dev-dependencies", "build-dependencies"}:
            for alias, spec in item.items():
                yield spec.get("package", alias) if isinstance(spec, dict) else alias
        elif isinstance(item, dict):
            yield from dependencies(item)


def inventory(root, paths):
    found = Counter()
    forbidden = set()
    for relative in paths:
        path = root / relative
        if not path.is_file():
            continue
        if relative.startswith(REMOVED_SOURCES):
            forbidden.add(f"{relative}: removed source")
        if path.name == "Cargo.toml":
            for name in dependencies(tomllib.loads(path.read_text())):
                if removed_dependency(name):
                    forbidden.add(f"{relative}: {name}")
                elif restricted(name):
                    found[f"dependency:{relative}:{name}"] += 1
        if (path.suffix in SOURCE_SUFFIXES and relative.split("/")[0] in {
            "app", "crates", "script", ".github", ".cargo", "resources",
        }) or (relative.startswith("script/") and not path.suffix) or relative in {
            ".agents/setup", ".agents/resume", ".agents/Procfile", ".amp/services.yaml",
        }:
            content = path.read_text(errors="replace")
            for symbol in REMOVED_SOURCE_SYMBOLS.get(relative, ()):
                if re.search(rf"\b{symbol}\b", content):
                    forbidden.add(f"{relative}: removed {symbol}")
            if relative.startswith("app/src/") and REMOVED_TELEMETRY_SYMBOLS.search(content):
                forbidden.add(f"{relative}: removed telemetry transport")
            for line in content.splitlines():
                if ENDPOINT.search(line):
                    digest = hashlib.sha256(line.strip().encode()).hexdigest()
                    found[f"endpoint:{relative}:{digest}"] += 1
    for package in tomllib.loads((root / "Cargo.lock").read_text())["package"]:
        name = package["name"]
        if removed_dependency(name):
            forbidden.add(f"Cargo.lock: {name}")
        elif restricted(name):
            found[f"lock:{name}:{package['version']}"] += 1
    return dict(sorted(found.items())), sorted(forbidden)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inventory", action="store_true", help="Print hashed residue for review")
    args = parser.parse_args()
    paths = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=ROOT
    ).decode().split("\0")
    actual, forbidden = inventory(ROOT, sorted(set(filter(None, paths))))
    if forbidden:
        print("Removed components returned:\n" + "\n".join(forbidden), file=sys.stderr)
        return 1
    if args.inventory:
        print(json.dumps(actual, indent=2, sort_keys=True))
        return 0
    expected = json.loads(BASELINE.read_text())
    if actual != expected:
        added = Counter(actual) - Counter(expected)
        removed = Counter(expected) - Counter(actual)
        print(f"Cloud residue changed: {sum(added.values())} added, {sum(removed.values())} removed.")
        for key in added:
            print(f"Forbidden addition: {key}")
        print("Review removals and shrink the residue inventory; never accept new cloud entries.")
        return 1
    print(f"Local-only guard passed; {sum(actual.values())} explicitly tracked residue entries remain.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
