use warpui::{App, TypedActionView};

use super::warpify_footer::WarpifyFooterViewAction;
use super::{CLIAgent, RichInputSubmitStrategy, rich_input_submit_strategy};
use crate::terminal::model::ansi::{BootstrappedValue, Handler as _, InitShellValue};
use crate::test_util::add_window_with_terminal;
use crate::test_util::terminal::initialize_app_for_terminal_view;

#[test]
fn shell_integration_footer_dismissal_removes_content_and_allows_reopening() {
    App::test((), |mut app| async move {
        initialize_app_for_terminal_view(&mut app);
        let terminal = add_window_with_terminal(&mut app, None);
        let footer = terminal.update(&mut app, |view, _| {
            let mut model = view.model.lock();
            model.init_shell(InitShellValue {
                session_id: 0.into(),
                shell: "zsh".to_owned(),
                ..Default::default()
            });
            model.bootstrapped(BootstrappedValue {
                shell: "zsh".to_owned(),
                ..Default::default()
            });
            model.simulate_long_running_block("ssh localhost", "Password:");
            view.warpify_footer.clone()
        });

        for _ in 0..2 {
            terminal.update(&mut app, |view, ctx| {
                view.warpify_footer
                    .update(ctx, |footer, ctx| footer.show(ctx));
                view.maybe_show_warpify_footer_in_blocklist(ctx);
                assert!(view.warpify_footer.as_ref(ctx).is_active());
                let model = view.model.lock();
                let index = model.block_list().active_block_index();
                assert_eq!(
                    model
                        .block_list()
                        .last_non_hidden_rich_content_block_after_block(Some(index))
                        .map(|(_, item)| item.view_id),
                    Some(view.warpify_footer.id())
                );
            });
            footer.update(&mut app, |footer, ctx| {
                footer.handle_action(&WarpifyFooterViewAction::Dismiss, ctx);
            });
            terminal.read(&app, |view, ctx| {
                assert!(!view.warpify_footer.as_ref(ctx).is_active());
                let model = view.model.lock();
                let index = model.block_list().active_block_index();
                assert!(
                    model
                        .block_list()
                        .last_non_hidden_rich_content_block_after_block(Some(index))
                        .is_none()
                );
            });
        }
    })
}

#[test]
fn test_rich_input_submit_strategy_for_oh_my_pi() {
    assert_eq!(
        rich_input_submit_strategy(CLIAgent::OhMyPi),
        RichInputSubmitStrategy::BracketedPaste
    );
}

/// Hermes interprets embedded newlines as submit actions when text is written
/// directly. Bracketed paste preserves them as part of one input payload.
#[test]
fn test_rich_input_submit_strategy_for_hermes_uses_bracketed_paste() {
    assert_eq!(
        rich_input_submit_strategy(CLIAgent::Hermes),
        RichInputSubmitStrategy::BracketedPaste
    );
}
