use std::path::PathBuf;

use ai::skills::SkillReference;
use settings::Setting;
use warp_core::channel::ChannelState;
use warp_util::local_or_remote_path::LocalOrRemotePath;
#[cfg(feature = "local_fs")]
use warp_util::path::LineAndColumnArg;
use warpui::{App, SingletonEntity};

#[cfg(feature = "local_fs")]
use super::{AIBlockEvent, open_code_action_event};
use super::{
    CollapsibleElementState, CollapsibleExpansionState, UserAvatarInfo,
    default_collapsible_state_for_orchestration_action,
    default_collapsible_state_for_orchestration_message, received_message_collapsible_id,
    recording_artifact_view_url, user_avatar_info_for_conversation_creator,
};
use crate::ai::agent::AIAgentActionType;
use crate::ai::ambient_agents::AmbientAgentTaskId;
use crate::auth::UserUid;
#[cfg(feature = "local_fs")]
use crate::code::editor_management::CodeSource;
use crate::settings::{AISettings, OrchestrationMessageDisplayMode};
use crate::test_util::settings::initialize_settings_for_tests;
use crate::workspaces::user_profiles::{UserProfileWithUID, UserProfiles};

#[test]
fn reasoning_auto_collapses_when_user_has_not_manually_toggled() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        let mut state = CollapsibleElementState::default();
        app.update(|ctx| {
            state.finish_reasoning(ctx);
        });

        assert!(matches!(
            state.expansion_state,
            CollapsibleExpansionState::Collapsed
        ));
    });
}

#[test]
fn collapsed_initializer_starts_collapsed() {
    let state = CollapsibleElementState::collapsed();

    assert!(matches!(
        state.expansion_state,
        CollapsibleExpansionState::Collapsed
    ));
}

#[test]
fn orchestration_show_and_collapse_collapses_after_finish() {
    let mut state = default_collapsible_state_for_orchestration_message(
        OrchestrationMessageDisplayMode::ShowAndCollapse,
    );

    state.finish_orchestration_message(OrchestrationMessageDisplayMode::ShowAndCollapse);

    assert!(matches!(
        state.expansion_state,
        CollapsibleExpansionState::Collapsed
    ));
}

#[test]
fn orchestration_always_show_stays_expanded_after_finish() {
    let mut state = default_collapsible_state_for_orchestration_message(
        OrchestrationMessageDisplayMode::AlwaysShow,
    );

    state.finish_orchestration_message(OrchestrationMessageDisplayMode::AlwaysShow);

    assert!(matches!(
        state.expansion_state,
        CollapsibleExpansionState::Expanded {
            is_finished: true,
            scroll_pinned_to_bottom: false
        }
    ));
}

#[test]
fn orchestration_send_message_starts_collapsed() {
    let state = default_collapsible_state_for_orchestration_action(
        &AIAgentActionType::SendMessageToAgent {
            addresses: vec!["child-agent".to_string()],
            subject: "Status".to_string(),
            message: "Body".to_string(),
        },
        OrchestrationMessageDisplayMode::AlwaysCollapse,
    )
    .expect("send-message actions should get a collapsible state");

    assert!(matches!(
        state.expansion_state,
        CollapsibleExpansionState::Collapsed
    ));
}

#[test]
fn non_orchestration_actions_do_not_get_collapsible_state_defaults() {
    assert!(
        default_collapsible_state_for_orchestration_action(
            &AIAgentActionType::OpenCodeReview,
            OrchestrationMessageDisplayMode::AlwaysCollapse,
        )
        .is_none()
    );
}

#[test]
fn recording_artifact_view_url_uses_configured_oz_origin() {
    let task_id: AmbientAgentTaskId = "00000000-0000-0000-0000-000000000123".parse().unwrap();

    assert_eq!(
        recording_artifact_view_url(Some(task_id), "recording-123"),
        Some(format!(
            "{}/runs/{task_id}?artifact=recording-123",
            ChannelState::oz_root_url()
        ))
    );
}

#[test]
fn recording_artifact_view_url_requires_task_id() {
    assert_eq!(recording_artifact_view_url(None, "recording-123"), None);
}

#[cfg(feature = "local_fs")]
#[test]
fn open_code_action_routes_links_to_configured_editor_and_non_links_to_warp() {
    let linked_source = CodeSource::Link {
        path: PathBuf::from("/workspace/project/src/main.rs"),
        range_start: Some(LineAndColumnArg {
            line_num: 42,
            column_num: Some(7),
        }),
        range_end: None,
    };

    assert!(matches!(
        open_code_action_event(
            &linked_source,
            crate::util::file::external_editor::settings::EditorLayout::SplitPane,
        ),
        AIBlockEvent::OpenDetectedFilePath {
            absolute_path,
            line_and_column_num: Some(LineAndColumnArg {
                line_num: 42,
                column_num: Some(7),
            }),
            target_override: None,
        } if absolute_path.as_path() == std::path::Path::new("/workspace/project/src/main.rs")
    ));

    let skill_source = CodeSource::Skill {
        reference: SkillReference::Path(LocalOrRemotePath::Local(PathBuf::from(
            "/workspace/project/.warp/skills/example/SKILL.md",
        ))),
        location: LocalOrRemotePath::Local(PathBuf::from(
            "/workspace/project/.warp/skills/example/SKILL.md",
        )),
        origin: crate::ai::skills::SkillOpenOrigin::ReadSkill,
    };

    assert!(matches!(
        open_code_action_event(
            &skill_source,
            crate::util::file::external_editor::settings::EditorLayout::NewTab,
        ),
        AIBlockEvent::OpenCodeInWarp {
            source,
            layout: crate::util::file::external_editor::settings::EditorLayout::NewTab,
        } if source == skill_source
    ));
}
#[test]
fn orchestration_show_and_collapse_starts_sent_messages_expanded() {
    let state = default_collapsible_state_for_orchestration_action(
        &AIAgentActionType::SendMessageToAgent {
            addresses: vec!["child-agent".to_string()],
            subject: "Status".to_string(),
            message: "Body".to_string(),
        },
        OrchestrationMessageDisplayMode::ShowAndCollapse,
    )
    .expect("send-message actions should get a collapsible state");

    assert!(matches!(
        state.expansion_state,
        CollapsibleExpansionState::Expanded {
            is_finished: false,
            scroll_pinned_to_bottom: true
        }
    ));
}

#[test]
fn orchestration_always_show_starts_sent_messages_expanded() {
    let state = default_collapsible_state_for_orchestration_action(
        &AIAgentActionType::SendMessageToAgent {
            addresses: vec!["child-agent".to_string()],
            subject: "Status".to_string(),
            message: "Body".to_string(),
        },
        OrchestrationMessageDisplayMode::AlwaysShow,
    )
    .expect("send-message actions should get a collapsible state");

    assert!(matches!(
        state.expansion_state,
        CollapsibleExpansionState::Expanded {
            is_finished: false,
            scroll_pinned_to_bottom: true
        }
    ));
}

#[test]
fn orchestration_received_messages_follow_initial_message_display_mode() {
    let show_and_collapse = default_collapsible_state_for_orchestration_message(
        OrchestrationMessageDisplayMode::ShowAndCollapse,
    );
    assert!(matches!(
        show_and_collapse.expansion_state,
        CollapsibleExpansionState::Expanded {
            is_finished: false,
            scroll_pinned_to_bottom: true
        }
    ));
    let collapsed = default_collapsible_state_for_orchestration_message(
        OrchestrationMessageDisplayMode::AlwaysCollapse,
    );
    assert!(matches!(
        collapsed.expansion_state,
        CollapsibleExpansionState::Collapsed
    ));
    let expanded = default_collapsible_state_for_orchestration_message(
        OrchestrationMessageDisplayMode::AlwaysShow,
    );

    assert!(matches!(
        expanded.expansion_state,
        CollapsibleExpansionState::Expanded {
            is_finished: false,
            scroll_pinned_to_bottom: true
        }
    ));
}

#[test]
fn always_show_thinking_stays_expanded_after_finish() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        AISettings::handle(&app).update(&mut app, |settings, ctx| {
            settings
                .thinking_display_mode
                .set_value(crate::settings::ThinkingDisplayMode::AlwaysShow, ctx)
                .unwrap();
        });

        let mut state = CollapsibleElementState::default();
        app.update(|ctx| {
            state.finish_reasoning(ctx);
        });

        assert!(matches!(
            state.expansion_state,
            CollapsibleExpansionState::Expanded {
                is_finished: true,
                scroll_pinned_to_bottom: false
            }
        ));
    });
}

#[test]
fn manual_collapse_while_streaming_stays_collapsed_after_finish() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        let mut state = CollapsibleElementState::default();

        state.toggle_expansion();
        app.update(|ctx| {
            state.finish_reasoning(ctx);
        });

        assert!(matches!(
            state.expansion_state,
            CollapsibleExpansionState::Collapsed
        ));
    });
}

#[test]
fn manual_reexpand_while_streaming_stays_expanded_after_finish() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        let mut state = CollapsibleElementState::default();

        state.toggle_expansion();
        state.toggle_expansion();
        app.update(|ctx| {
            state.finish_reasoning(ctx);
        });

        assert!(matches!(
            state.expansion_state,
            CollapsibleExpansionState::Expanded {
                is_finished: true,
                scroll_pinned_to_bottom: false
            }
        ));
    });
}

#[test]
fn received_message_collapsible_id_prefixes_row_ids() {
    let first = received_message_collapsible_id("message-1");
    let second = received_message_collapsible_id("message-2");

    assert_eq!(&*first, "received-message:message-1");
    assert_eq!(&*second, "received-message:message-2");
    assert_ne!(first, second);
}

#[test]
fn user_avatar_info_prefers_conversation_creator_profile() {
    App::test((), |app| async move {
        let creator = UserProfileWithUID {
            firebase_uid: UserUid::new("creator-uid"),
            display_name: Some("Creator Name".to_string()),
            email: "creator@example.com".to_string(),
            photo_url: "https://example.com/creator.png".to_string(),
        };
        let fallback = UserAvatarInfo {
            display_name: "Current User".to_string(),
            profile_image_path: Some("https://example.com/current.png".to_string()),
        };

        app.read(|ctx| {
            let avatar_info = user_avatar_info_for_conversation_creator(
                Some(&creator),
                Some("fallback-uid"),
                fallback,
                ctx,
            );

            assert_eq!(avatar_info.display_name, "Creator Name");
            assert_eq!(
                avatar_info.profile_image_path.as_deref(),
                Some("https://example.com/creator.png")
            );
        });
    });
}

#[test]
fn user_avatar_info_uses_cached_profile_for_creator_uid() {
    App::test((), |app| async move {
        app.add_singleton_model(|_| {
            UserProfiles::new(vec![UserProfileWithUID {
                firebase_uid: UserUid::new("creator-uid"),
                display_name: Some("Cached Creator".to_string()),
                email: "cached@example.com".to_string(),
                photo_url: "https://example.com/cached.png".to_string(),
            }])
        });
        let fallback = UserAvatarInfo {
            display_name: "Current User".to_string(),
            profile_image_path: Some("https://example.com/current.png".to_string()),
        };

        app.read(|ctx| {
            let avatar_info =
                user_avatar_info_for_conversation_creator(None, Some("creator-uid"), fallback, ctx);

            assert_eq!(avatar_info.display_name, "Cached Creator");
            assert_eq!(
                avatar_info.profile_image_path.as_deref(),
                Some("https://example.com/cached.png")
            );
        });
    });
}

#[test]
fn should_show_agent_mode_ask_user_question_speedbump_defaults_to_true() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        AISettings::handle(&app).read(&app, |settings, _ctx| {
            assert!(*settings.should_show_agent_mode_ask_user_question_speedbump);
        });
    });
}

#[test]
fn should_show_agent_mode_ask_user_question_speedbump_round_trips_to_false() {
    App::test((), |mut app| async move {
        initialize_settings_for_tests(&mut app);
        AISettings::handle(&app).update(&mut app, |settings, ctx| {
            settings
                .should_show_agent_mode_ask_user_question_speedbump
                .set_value(false, ctx)
                .unwrap();
        });
        AISettings::handle(&app).read(&app, |settings, _ctx| {
            assert!(!*settings.should_show_agent_mode_ask_user_question_speedbump);
        });
    });
}
