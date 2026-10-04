use std::sync::Arc;

use warpui::App;
use warpui::platform::WindowStyle;

use crate::appearance::Appearance;
use crate::user_config::WarpConfig;
use crate::voltron::VoltronFeatureViewMeta;
use crate::workflows::categories::{CategoriesView, WorkflowMatchType, WorkflowViewType};
use crate::workflows::workflow::Workflow;
use crate::workflows::{WorkflowSource, WorkflowType};

#[test]
fn local_workflows_browse_without_cloud_models() {
    App::test((), |mut app| async move {
        app.add_singleton_model(|_| Appearance::mock());
        app.add_singleton_model(WarpConfig::mock);
        let (_, view) = app.add_window(WindowStyle::NotStealFocus, |ctx| {
            CategoriesView::new(
                [Workflow::Command {
                    name: "Local command".into(),
                    command: "printf local".into(),
                    tags: vec![],
                    description: None,
                    arguments: vec![],
                    source_url: None,
                    author: None,
                    author_url: None,
                    shells: vec![],
                    environment_variables: None,
                }],
                [],
                ctx,
            )
        });

        view.update(&mut app, |view, ctx| {
            view.on_load(Default::default(), ctx);
            view.increment_focused_workflow_type(ctx);
            assert_eq!(view.selected_workflow_type, WorkflowViewType::LocalPersonal);
            assert_eq!(view.active_workflows.len(), 1);
            assert_eq!(view.active_workflows[0].1, WorkflowSource::Local);
            assert_eq!(
                view.active_workflows[0].0.as_workflow().command(),
                Some("printf local")
            );

            view.increment_focused_workflow_type(ctx);
            assert_eq!(view.selected_workflow_type, WorkflowViewType::Project);
            assert!(view.active_workflows.is_empty());
            view.decrement_focused_workflow_type(ctx);
            assert_eq!(view.selected_workflow_type, WorkflowViewType::LocalPersonal);

            view.search_term = "local".into();
            assert_eq!(view.filtered_workflows().count(), 1);
            view.search_term = "unmatched-query".into();
            assert_eq!(view.filtered_workflows().count(), 0);
            view.on_load(Default::default(), ctx);
            assert_eq!(view.filtered_workflows().count(), 1);
            view.reset(ctx);
            assert_eq!(view.selected_workflow_type, WorkflowViewType::All);
        });
    });
}

#[test]
fn test_workflow_matches() {
    let workflow = Arc::new(WorkflowType::Local(Workflow::Command {
        name: "g workflow_name it ".into(),
        command: "command_name git".to_string(),
        tags: vec!["foo".into(), "bar".into()],
        description: None,
        arguments: vec![],
        source_url: None,
        author: None,
        author_url: None,
        shells: vec![],
        environment_variables: None,
    }));

    assert_eq!(
        CategoriesView::matches_workflow(&workflow, "foo"),
        WorkflowMatchType::Tag
    );
    assert_eq!(
        CategoriesView::matches_workflow(&workflow, "bar"),
        WorkflowMatchType::Tag
    );

    // The Workflow name has higher precedence than the command.
    assert!(matches!(
        CategoriesView::matches_workflow(&workflow, "name"),
        WorkflowMatchType::Name { .. }
    ));

    // Git matches both the name and the command, but fuzzy matches command with a higher score.
    assert!(matches!(
        CategoriesView::matches_workflow(&workflow, "git"),
        WorkflowMatchType::Command { .. }
    ));

    assert!(matches!(
        CategoriesView::matches_workflow(&workflow, "command"),
        WorkflowMatchType::Command { .. }
    ));

    assert!(matches!(
        CategoriesView::matches_workflow(&workflow, "command"),
        WorkflowMatchType::Command { .. }
    ));

    assert_eq!(
        CategoriesView::matches_workflow(&workflow, "gibberish"),
        WorkflowMatchType::Unmatched
    );
}
