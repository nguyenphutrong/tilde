use super::*;

const ALL_SECTIONS: &[SettingsSection] = &[
    SettingsSection::About,
    SettingsSection::Appearance,
    SettingsSection::Keybindings,
    SettingsSection::Scripting,
    SettingsSection::Warpify,
];

#[test]
fn section_list_is_exhaustive() {
    fn assert_listed(section: SettingsSection) {
        let section = match section {
            SettingsSection::About
            | SettingsSection::Appearance
            | SettingsSection::Keybindings
            | SettingsSection::Scripting
            | SettingsSection::Warpify => section,
        };
        assert!(ALL_SECTIONS.contains(&section));
    }

    for section in ALL_SECTIONS {
        assert_listed(*section);
    }
}

#[test]
fn sections_round_trip_through_unique_slugs() {
    let mut slugs = Vec::new();
    for section in ALL_SECTIONS {
        assert_eq!(SettingsSection::from_slug(section.slug()), Some(*section));
        slugs.push(section.slug());
    }

    slugs.sort_unstable();
    slugs.dedup();
    assert_eq!(slugs.len(), ALL_SECTIONS.len());
}

#[test]
fn section_labels_are_terminal_only() {
    assert_eq!(SettingsSection::About.to_string(), "About");
    assert_eq!(SettingsSection::Appearance.to_string(), "Appearance");
    assert_eq!(
        SettingsSection::Keybindings.to_string(),
        "Keyboard shortcuts"
    );
    assert_eq!(SettingsSection::Scripting.to_string(), "Scripting");
    assert_eq!(SettingsSection::Warpify.to_string(), "Shell integration");
}

#[test]
fn removed_and_unknown_slugs_are_rejected() {
    for slug in ["Account", "Billing and usage", "Warp Agent", "Teams", ""] {
        assert_eq!(SettingsSection::from_slug(slug), None);
    }
}
