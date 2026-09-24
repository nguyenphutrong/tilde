use warpui::App;

use super::*;
use crate::test_util::settings::initialize_settings_for_tests;

#[test]
fn local_settings_control_obfuscation_without_an_account() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        app.update(|ctx| {
            assert!(matches!(
                get_secret_obfuscation_mode(ctx),
                ObfuscateSecrets::No
            ));
            SafeModeSettings::handle(ctx).update(ctx, |settings, ctx| {
                settings.safe_mode_enabled.set_value(true, ctx).unwrap();
            });
            assert!(matches!(
                get_secret_obfuscation_mode(ctx),
                ObfuscateSecrets::Strikethrough
            ));
            SafeModeSettings::handle(ctx).update(ctx, |settings, ctx| {
                settings
                    .hide_secrets_in_block_list
                    .set_value(true, ctx)
                    .unwrap();
            });
            assert!(matches!(
                get_secret_obfuscation_mode(ctx),
                ObfuscateSecrets::Yes
            ));
            SafeModeSettings::handle(ctx).update(ctx, |settings, ctx| {
                settings
                    .secret_display_mode
                    .set_value(SecretDisplayMode::AlwaysShow, ctx)
                    .unwrap();
            });
            assert!(matches!(
                get_secret_obfuscation_mode(ctx),
                ObfuscateSecrets::AlwaysShow
            ));
            SafeModeSettings::handle(ctx).update(ctx, |settings, ctx| {
                settings
                    .secret_display_mode
                    .set_value(SecretDisplayMode::Asterisks, ctx)
                    .unwrap();
                settings
                    .hide_secrets_in_block_list
                    .set_value(false, ctx)
                    .unwrap();
            });
            assert!(matches!(
                get_secret_obfuscation_mode(ctx),
                ObfuscateSecrets::Yes
            ));
            SafeModeSettings::handle(ctx).update(ctx, |settings, ctx| {
                settings.safe_mode_enabled.set_value(false, ctx).unwrap();
            });
            assert!(matches!(
                get_secret_obfuscation_mode(ctx),
                ObfuscateSecrets::No
            ));
        });
    });
}
