use warpui::App;
use warpui::platform::WindowStyle;

use super::*;
use crate::auth::AuthStateProvider;
use crate::auth::auth_manager::AuthManager;
use crate::cloud_object::model::persistence::CloudModel;
use crate::network::NetworkStatus;
use crate::server::cloud_objects::listener::Listener;
use crate::server::cloud_objects::update_manager::UpdateManager;
use crate::server::server_api::ServerApiProvider;
use crate::server::sync_queue::SyncQueue;
use crate::server::telemetry::context_provider::AppTelemetryContextProvider;
use crate::settings::AISettings;
use crate::settings_view::keybindings::KeybindingChangedNotifier;
use crate::system::SystemStats;
use crate::test_util::settings::initialize_settings_for_tests;
use crate::workflows::local_workflows::LocalWorkflows;
use crate::workspaces::team_tester::TeamTesterStatus;
use crate::workspaces::update_manager::TeamUpdateManager;
use crate::workspaces::user_workspaces::UserWorkspaces;

fn initialize_app(app: &mut App) {
    initialize_settings_for_tests(app);

    app.add_singleton_model(|_| ServerApiProvider::new_for_test());
    app.add_singleton_model(|_| AuthStateProvider::new_for_test());
    app.add_singleton_model(AppTelemetryContextProvider::new_context_provider);
    app.add_singleton_model(AuthManager::new_for_test);
    app.add_singleton_model(|_| NetworkStatus::new());
    app.add_singleton_model(|_| SystemStats::new());
    app.add_singleton_model(SyncQueue::mock);
    app.add_singleton_model(CloudModel::mock);
    app.add_singleton_model(UserWorkspaces::default_mock);
    app.add_singleton_model(TeamTesterStatus::mock);
    app.add_singleton_model(TeamUpdateManager::mock);
    app.add_singleton_model(Listener::mock);
    app.add_singleton_model(UpdateManager::mock);
    app.add_singleton_model(|_| Appearance::mock());
    app.add_singleton_model(|_| ResizableData::default());
    app.add_singleton_model(|_| KeybindingChangedNotifier::mock());
    #[cfg(feature = "voice_input")]
    app.add_singleton_model(voice_input::VoiceInput::new);
}

#[test]
fn test_render_view() {
    App::test((), |mut app| async move {
        initialize_app(&mut app);

        let (_window_id, _view) =
            app.add_window(WindowStyle::NotStealFocus, CommandSearchView::new);

        app.update(|_| {
            // This will force a redraw of the window, which lays out the
            // window, including the command search view.
        });
    });
}

#[test]
fn command_search_excludes_ai_sources_with_ai_enabled() {
    let _agent_mode = crate::features::FeatureFlag::AgentMode.override_enabled(true);
    App::test((), |mut app| async move {
        initialize_app(&mut app);
        app.add_singleton_model(LocalWorkflows::new);
        app.add_singleton_model(|_| History::new(vec![]));
        let (_, view) = app.add_window(WindowStyle::NotStealFocus, CommandSearchView::new);
        view.update(&mut app, |view, ctx| {
            assert!(AISettings::as_ref(ctx).is_any_ai_enabled(ctx));
            view.reset_command_search_mixer(SessionId::from(123), None, ctx);
            assert_eq!(
                view.mixer
                    .as_ref(ctx)
                    .registered_filters()
                    .collect::<HashSet<_>>(),
                HashSet::from([QueryFilter::Workflows]),
                "Only local workflows register before shell history is initialized"
            );
        });
    });
}
