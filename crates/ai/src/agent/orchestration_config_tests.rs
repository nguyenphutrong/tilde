use super::*;

fn make_config(model: &str, harness: &str, remote: bool) -> OrchestrationConfig {
    OrchestrationConfig {
        model_id: model.to_string(),
        harness_type: harness.to_string(),
        execution_mode: if remote {
            OrchestrationExecutionMode::Remote {
                environment_id: "env-1".to_string(),
                worker_host: "warp".to_string(),
                runner_id: String::new(),
            }
        } else {
            OrchestrationExecutionMode::Local
        },
    }
}

#[test]
fn proto_round_trip_config_remote_with_runner() {
    let mut config = make_config("auto", "claude", true);
    if let OrchestrationExecutionMode::Remote { runner_id, .. } = &mut config.execution_mode {
        *runner_id = "runner-xyz".to_string();
    }
    let proto = config.to_proto();
    let round_tripped = OrchestrationConfig::from_proto(&proto);
    assert_eq!(config, round_tripped);
}

#[test]
fn status_default_is_none() {
    assert_eq!(
        OrchestrationConfigStatus::default(),
        OrchestrationConfigStatus::None
    );
}

#[test]
fn status_predicates() {
    assert!(OrchestrationConfigStatus::Approved.is_approved());
    assert!(!OrchestrationConfigStatus::Approved.is_disapproved());
    assert!(OrchestrationConfigStatus::Disapproved.is_disapproved());
    assert!(!OrchestrationConfigStatus::None.is_approved());
}

#[test]
fn proto_round_trip_config_local() {
    let config = make_config("auto", "oz", false);
    let proto = config.to_proto();
    let round_tripped = OrchestrationConfig::from_proto(&proto);
    assert_eq!(config, round_tripped);
}

#[test]
fn proto_round_trip_config_remote() {
    let config = make_config("auto", "claude", true);
    let proto = config.to_proto();
    let round_tripped = OrchestrationConfig::from_proto(&proto);
    assert_eq!(config, round_tripped);
}

#[test]
fn proto_round_trip_status() {
    for status in [
        OrchestrationConfigStatus::None,
        OrchestrationConfigStatus::Approved,
        OrchestrationConfigStatus::Disapproved,
    ] {
        let proto = status.to_proto();
        let round_tripped = OrchestrationConfigStatus::from_proto(proto.as_ref());
        assert_eq!(status, round_tripped);
    }
}
