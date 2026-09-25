use ai::agent::action::RunAgentsExecutionMode;

use super::accept_disabled_reason_with_auth;
use crate::ai::orchestration::config_state::{AuthSecretSelection, OrchestrationConfigState};

fn state(
    harness: &str,
    mode: RunAgentsExecutionMode,
    auth: AuthSecretSelection,
) -> OrchestrationConfigState {
    let mut state =
        OrchestrationConfigState::from_run_agents_fields(Some("auto"), Some(harness), &mode);
    state.auth_secret_selection = auth;
    state
}

fn cloud() -> RunAgentsExecutionMode {
    RunAgentsExecutionMode::Remote {
        environment_id: "env-1".to_string(),
        worker_host: "warp".to_string(),
        computer_use_enabled: false,
        runner_id: String::new(),
    }
}

#[test]
fn accept_allowed_for_oz_local_and_cloud() {
    for mode in [RunAgentsExecutionMode::Local, cloud()] {
        let state = state("oz", mode, AuthSecretSelection::Unset);
        assert_eq!(accept_disabled_reason_with_auth(&state), None);
    }
}

#[test]
fn accept_blocked_for_product_disabled_local_codex() {
    let state = state(
        "codex",
        RunAgentsExecutionMode::Local,
        AuthSecretSelection::Unset,
    );
    assert_eq!(
        accept_disabled_reason_with_auth(&state),
        Some("Local Codex child agents are temporarily disabled.".to_string())
    );
}

#[test]
fn accept_blocked_for_opencode_cloud() {
    let state = state("opencode", cloud(), AuthSecretSelection::Unset);
    let reason =
        accept_disabled_reason_with_auth(&state).expect("OpenCode + Cloud should block Accept");
    assert!(reason.contains("OpenCode"));
}

#[test]
fn accept_blocked_for_cloud_harness_with_unset_auth_secret() {
    for harness in ["claude", "codex"] {
        let state = state(harness, cloud(), AuthSecretSelection::Unset);
        assert_eq!(
            accept_disabled_reason_with_auth(&state),
            Some("Select an API key for this harness to continue.".to_string()),
            "Cloud + {harness} without an API key choice should block Accept"
        );
    }
}

#[test]
fn accept_allowed_for_cloud_harness_with_named_or_inherited_auth() {
    for harness in ["claude", "codex"] {
        for auth in [
            AuthSecretSelection::Named("my-key".to_string()),
            AuthSecretSelection::Inherit,
        ] {
            let state = state(harness, cloud(), auth);
            assert_eq!(accept_disabled_reason_with_auth(&state), None);
        }
    }
}
