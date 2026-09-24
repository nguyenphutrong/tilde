use std::collections::HashMap;

use warpui::App;

use super::InlineMenuModel;
use crate::terminal::input::inline_menu::{InlineMenuAction, InlineMenuType};

#[test]
fn retired_conversation_menu_preserves_saved_history_height() {
    let saved = serde_json::json!({"ConversationMenu": 125.5, "InlineHistoryMenu": 210.25});
    let heights: HashMap<InlineMenuType, f32> = serde_json::from_value(saved.clone()).unwrap();
    assert_eq!(heights[&InlineMenuType::InlineHistoryMenu], 210.25);
    assert_eq!(serde_json::to_value(heights).unwrap(), saved);
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TestAction(&'static str);

impl InlineMenuAction for TestAction {
    const MENU_TYPE: InlineMenuType = InlineMenuType::SlashCommands;
}

#[test]
fn refreshed_selection_clears_stale_selected_item() {
    App::test((), |mut app| async move {
        let model = app.add_model(|_| InlineMenuModel::<TestAction>::new());

        model.update(&mut app, |model, ctx| {
            model.update_selected_item(Some(TestAction("enabled")), ctx);
        });
        model.read(&app, |model, _| {
            assert_eq!(model.selected_item(), Some(&TestAction("enabled")));
        });

        model.update(&mut app, |model, ctx| {
            model.update_selected_item(None, ctx);
        });
        model.read(&app, |model, _| {
            assert_eq!(model.selected_item(), None);
        });
    });
}
