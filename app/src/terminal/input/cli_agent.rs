use warpui::elements::{
    Border, Clipped, ConstrainedBox, Container, DispatchEventResult, DropTarget, Element,
    EventHandler, Hoverable, SavePosition,
};
use warpui::presenter::ChildView;
use warpui::{AppContext, SingletonEntity, ViewContext};

use super::common::wrap_input_with_terminal_padding_and_focus_handler;
use super::{Input, InputDropTargetData};
use crate::appearance::Appearance;
use crate::editor::{EnterAction, EnterSettings};
use crate::settings::AISettings;
use crate::terminal::cli_agent_sessions::CLIAgentSessionsModel;
use crate::terminal::should_right_click_paste;
use crate::terminal::view::TerminalAction;

impl Input {
    pub(super) fn render_cli_agent_input(&self, app: &AppContext) -> Box<dyn Element> {
        let appearance = Appearance::as_ref(app);
        let editor_position = self.editor_save_position_id();
        let editor = SavePosition::new(
            EventHandler::new(
                ConstrainedBox::new(Clipped::new(ChildView::new(&self.editor).finish()).finish())
                    .with_max_height(236.)
                    .finish(),
            )
            .on_right_mouse_down(move |ctx, app, position, modifiers| {
                if should_right_click_paste(modifiers.shift, app) {
                    ctx.dispatch_typed_action(TerminalAction::Paste);
                } else if let Some(rect) = ctx.element_position_by_id(editor_position.clone()) {
                    ctx.dispatch_typed_action(TerminalAction::OpenInputContextMenu {
                        position: position - rect.origin(),
                    });
                }
                DispatchEventResult::StopPropagation
            })
            .finish(),
            &self.editor_save_position_id(),
        )
        .finish();
        let input = Container::new(wrap_input_with_terminal_padding_and_focus_handler(
            self.is_active_session(app),
            editor,
            false,
        ))
        .with_padding_top(10.)
        .with_padding_bottom(8.)
        .with_border(Border::top(1.).with_border_color(appearance.theme().outline().into_solid()))
        .with_background(appearance.theme().surface_1())
        .finish();
        let input = DropTarget::new(
            input,
            InputDropTargetData::new(self.weak_view_handle.clone()),
        )
        .finish();
        let input = Hoverable::new(self.hoverable_handle.clone(), |_| input)
            .on_middle_click(|ctx, _, _| {
                ctx.dispatch_typed_action(TerminalAction::MiddleClickOnInput)
            })
            .finish();
        SavePosition::new(
            SavePosition::new(input, &self.status_free_input_save_position_id()).finish(),
            &self.save_position_id(),
        )
        .finish()
    }

    pub(super) fn update_cli_agent_enter_settings(&mut self, ctx: &mut ViewContext<Self>) {
        let settings = if CLIAgentSessionsModel::as_ref(ctx).is_input_open(self.terminal_view_id) {
            EnterSettings {
                enter: EnterAction::Emit,
                ctrl_enter: if *AISettings::as_ref(ctx).submit_on_ctrl_enter {
                    EnterAction::Emit
                } else {
                    EnterAction::InsertNewLineIfMultiLine
                },
                ..Default::default()
            }
        } else {
            EnterSettings::default()
        };
        self.editor
            .update(ctx, |editor, _| editor.set_enter_settings(settings));
    }
}
