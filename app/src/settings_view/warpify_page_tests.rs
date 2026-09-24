use warp_core::features::FeatureFlag;
use warpui::App;
use warpui::platform::WindowStyle;

use super::*;
use crate::settings_view::keybindings::KeybindingChangedNotifier;
use crate::test_util::settings::initialize_settings_for_tests;

#[test]
fn shell_settings_actions_and_observers_work_without_a_remote_server() {
    App::test((), |mut app| async move {
        let _flag = FeatureFlag::SshRemoteServer.override_enabled(true);
        initialize_settings_for_tests(&mut app);
        app.add_singleton_model(|_| Appearance::mock());
        app.add_singleton_model(|_| KeybindingChangedNotifier::new());
        let (_, view) = app.add_window(WindowStyle::NotStealFocus, WarpifyPageView::new);

        WarpifySettings::handle(&app).update(&mut app, |settings, ctx| {
            settings.add_subshell_command("^custom-shell$", ctx);
            settings.add_subshell_command("^other-shell$", ctx);
            settings.denylist_subshell_command("blocked-shell", ctx);
        });
        view.read(&app, |view, _| {
            assert_eq!(view.remove_added_command_button_states.len(), 2);
            assert_eq!(view.remove_denylisted_command_button_states.len(), 1);
        });
        view.update(&mut app, |view, ctx| {
            view.handle_action(&WarpifyPageAction::RemoveAddedCommand(0), ctx);
            view.handle_action(&WarpifyPageAction::ToggleSshWarpification, ctx);
        });
        view.read(&app, |view, ctx| {
            assert_eq!(view.remove_added_command_button_states.len(), 1);
            assert_eq!(view.remove_denylisted_command_button_states.len(), 1);
            let settings = WarpifySettings::as_ref(ctx);
            assert_eq!(
                settings.added_subshell_commands.as_slice(),
                ["^other-shell$"]
            );
            assert!(!*settings.enable_ssh_warpification.value());
        });
        view.update(&mut app, |view, ctx| {
            view.handle_action(&WarpifyPageAction::ToggleSshWarpification, ctx);
        });
        app.read(|ctx| {
            assert!(
                *WarpifySettings::as_ref(ctx)
                    .enable_ssh_warpification
                    .value()
            );
        });
    });
}
