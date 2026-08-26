use clap::{CommandFactory, Parser};

use super::*;

#[test]
fn identifies_local_worker_subcommands() {
    assert!(is_worker_invocation("minidump-server"));
    #[cfg(unix)]
    assert!(is_worker_invocation(&terminal_server_subcommand()));
    #[cfg(feature = "plugin_host")]
    assert!(is_worker_invocation("--plugin-host"));
    assert!(!is_worker_invocation("agent"));
}

#[test]
fn help_only_lists_local_terminal_commands() {
    let help = Args::clap_command().render_long_help().to_string();

    assert!(help.contains("A fast local terminal"));
    let command = <Args as CommandFactory>::command();
    for removed in [
        "agent",
        "environment",
        "login",
        "mcp",
        "provider",
        "run",
        "schedule",
        "secret",
    ] {
        assert!(
            command.find_subcommand(removed).is_none(),
            "removed command {removed:?} remains registered"
        );
    }
}

#[test]
fn rejects_removed_cloud_commands() {
    for removed in ["agent", "environment", "login", "run", "schedule"] {
        assert!(
            Args::try_parse_from(["warp", removed]).is_err(),
            "removed command {removed:?} should not parse"
        );
    }
}

#[test]
fn parses_local_terminal_urls() {
    let args = Args::try_parse_from(["warp", "warp://action/new_tab"]).unwrap();

    assert_eq!(args.app_args().urls[0].as_str(), "warp://action/new_tab");
}

#[test]
fn command_factory_does_not_include_remote_workers() {
    let command = <Args as CommandFactory>::command();

    assert!(command.find_subcommand("remote-server-proxy").is_none());
    assert!(command.find_subcommand("remote-server-daemon").is_none());
}
