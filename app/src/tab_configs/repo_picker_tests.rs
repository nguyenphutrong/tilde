use warp_core::ui::appearance::Appearance;
use warpui::platform::WindowStyle;
use warpui::{App, View};

use super::RepoPicker;
use crate::settings_view::keybindings::KeybindingChangedNotifier;
use crate::test_util::settings::initialize_settings_for_tests;

#[test]
fn repo_picker_selects_local_paths_without_ai_workspace_registry() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        app.add_singleton_model(|_| Appearance::mock());
        app.add_singleton_model(|_| KeybindingChangedNotifier::mock());
        let (_, view) =
            app.add_window(WindowStyle::NotStealFocus, |ctx| RepoPicker::new(None, ctx));
        view.read(&app, |picker, ctx| {
            assert_eq!(picker.selected_value(ctx), None);
            assert_eq!(picker.dropdown.as_ref(ctx).selected_item_label(), None);
            picker.render(ctx);
        });

        for directory in ["first repo α", "different repo β"] {
            let path = std::env::temp_dir().join(directory);
            view.update(&mut app, |picker, ctx| {
                picker.refresh_and_select(path.clone(), ctx);
                picker.refresh_items(None, ctx);
            });
            view.read(&app, |picker, ctx| {
                assert_eq!(
                    picker.selected_value(ctx),
                    Some(path.to_string_lossy().into_owned())
                );
                let label = picker.dropdown.as_ref(ctx).selected_item_label().unwrap();
                assert!(label.ends_with(directory));
                picker.render(ctx);
            });
        }
    });
}
