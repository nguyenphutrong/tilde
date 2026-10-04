use warpui::elements::{Container, Element, Empty};
use warpui::keymap::Keystroke;
use warpui::{AppContext, Entity, ModelHandle, View, ViewContext};

use super::inline_history::AcceptHistoryItem;
use super::inline_menu::{InlineMenuModel, InlineMenuModelEvent};
use super::message_bar::common::render_terminal_message;
use super::message_bar::{Message, MessageItem};
use super::suggestions_mode_model::InputSuggestionsModeModel;

pub struct TerminalInputMessageBar {
    suggestions_mode_model: ModelHandle<InputSuggestionsModeModel>,
    inline_history_model: ModelHandle<InlineMenuModel<AcceptHistoryItem>>,
}

impl Entity for TerminalInputMessageBar {
    type Event = ();
}

impl TerminalInputMessageBar {
    pub fn new(
        suggestions_mode_model: ModelHandle<InputSuggestionsModeModel>,
        inline_history_model: ModelHandle<InlineMenuModel<AcceptHistoryItem>>,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        ctx.subscribe_to_model(&suggestions_mode_model, |_, _, _, ctx| ctx.notify());
        ctx.subscribe_to_model(&inline_history_model, |_, _, event, ctx| {
            if let InlineMenuModelEvent::UpdatedSelectedItem = event {
                ctx.notify();
            }
        });
        Self {
            suggestions_mode_model,
            inline_history_model,
        }
    }
}

impl View for TerminalInputMessageBar {
    fn ui_name() -> &'static str {
        "TerminalInputMessageBar"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        if !self
            .suggestions_mode_model
            .as_ref(app)
            .is_inline_history_menu()
            || self
                .inline_history_model
                .as_ref(app)
                .selected_item()
                .is_none()
        {
            return Empty::new().finish();
        }
        let message = Message::new(vec![
            MessageItem::keystroke(Keystroke {
                key: "enter".to_owned(),
                ..Default::default()
            }),
            MessageItem::text(" to execute"),
        ]);
        Container::new(render_terminal_message(message, app))
            .with_padding_bottom(8.)
            .with_padding_right(8.)
            .finish()
    }
}
