use warpui::keymap::{BindingDescription, BindingId};

use super::is_excluded_binding;
use crate::util::bindings::{BindingGroup, CommandBinding};

fn binding(group: BindingGroup) -> CommandBinding {
    CommandBinding {
        name: "test".into(),
        description: BindingDescription::new("test"),
        trigger: None,
        action: None,
        group: Some(group),
        id: BindingId(0),
    }
}

#[test]
fn excludes_ai_and_cloud_action_groups() {
    for group in [
        BindingGroup::WarpAi,
        BindingGroup::Workflow,
        BindingGroup::Notebooks,
        BindingGroup::EnvVarCollection,
    ] {
        assert!(is_excluded_binding(&binding(group)));
    }

    assert!(!is_excluded_binding(&binding(BindingGroup::Settings)));
    assert!(!is_excluded_binding(&binding(BindingGroup::Terminal)));
}
