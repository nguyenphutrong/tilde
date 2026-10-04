use super::slash_command_composition_filter;

#[test]
fn slash_composition_distinguishes_commands_from_paths() {
    assert_eq!(
        slash_command_composition_filter("/plan app/src/lib.rs"),
        Some("plan app/src/lib.rs")
    );
    assert_eq!(slash_command_composition_filter("/usr/bin/env"), None);
    assert_eq!(slash_command_composition_filter(" /plan"), None);
}
