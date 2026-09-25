use super::*;

#[test]
fn retired_input_controls_decode_without_becoming_available() {
    let items: Vec<AgentToolbarItemKind> =
        serde_json::from_str(r#"["ModelSelector","NLDToggle","FileAttach"]"#).unwrap();
    assert_eq!(
        items,
        vec![
            AgentToolbarItemKind::ModelSelector,
            AgentToolbarItemKind::NLDToggle,
            AgentToolbarItemKind::FileAttach
        ]
    );
    for item in &items[..2] {
        assert!(!AgentToolbarItemKind::default_left().contains(item));
        assert!(!AgentToolbarItemKind::default_right().contains(item));
        assert!(!AgentToolbarItemKind::all_available().contains(item));
    }
    warpui::App::test((), |app| async move {
        app.read(|ctx| {
            assert!(!items[0].is_available(ctx));
            assert!(!items[1].is_available(ctx));
            assert!(items[2].is_available(ctx));
        });
    });
}

/// The File explorer chip is offered to the agent view toolbelt editor.
#[test]
fn file_explorer_is_offered_in_the_agent_view() {
    assert!(
        AgentToolbarItemKind::FileExplorer
            .available_in()
            .is_available_for_agent_view()
    );
    assert!(AgentToolbarItemKind::all_available().contains(&AgentToolbarItemKind::FileExplorer));
}

/// ...but it is opt-in, so it must stay out of the default agent view layout.
#[test]
fn file_explorer_is_not_an_agent_view_default() {
    assert!(!AgentToolbarItemKind::default_left().contains(&AgentToolbarItemKind::FileExplorer));
    assert!(!AgentToolbarItemKind::default_right().contains(&AgentToolbarItemKind::FileExplorer));
}
