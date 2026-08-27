//! Integration tests for terminal-only settings navigation and search.

use warp::integration_testing::settings::{
    assert_settings_nav_page_visible, assert_settings_section, clear_settings_search,
    click_settings_nav_page, open_settings_page, press_settings_nav_down, press_settings_nav_up,
    type_settings_search,
};
use warp::integration_testing::terminal::wait_until_bootstrapped_single_pane_for_tab;
use warp::settings_view::SettingsSection;

use super::{Builder, new_builder};

pub fn test_settings_mouse_navigation() -> Builder {
    new_builder()
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_settings_page(SettingsSection::Appearance))
        .with_step(click_settings_nav_page(SettingsSection::Keybindings))
        .with_step(assert_settings_section(SettingsSection::Keybindings))
        .with_step(click_settings_nav_page(SettingsSection::Warpify))
        .with_step(assert_settings_section(SettingsSection::Warpify))
}

pub fn test_settings_keyboard_navigation() -> Builder {
    new_builder()
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_settings_page(SettingsSection::Appearance))
        .with_step(press_settings_nav_down())
        .with_step(assert_settings_section(SettingsSection::Keybindings))
        .with_step(press_settings_nav_up())
        .with_step(assert_settings_section(SettingsSection::Appearance))
}

pub fn test_settings_search_filters_pages() -> Builder {
    new_builder()
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_settings_page(SettingsSection::Appearance))
        .with_step(type_settings_search("keyboard shortcut"))
        .with_step(assert_settings_nav_page_visible(
            SettingsSection::Keybindings,
            true,
        ))
        .with_step(assert_settings_nav_page_visible(
            SettingsSection::About,
            false,
        ))
        .with_step(assert_settings_section(SettingsSection::Keybindings))
}

pub fn test_settings_search_clear_restores_pages() -> Builder {
    new_builder()
        .with_step(wait_until_bootstrapped_single_pane_for_tab(0))
        .with_step(open_settings_page(SettingsSection::Appearance))
        .with_step(type_settings_search("keyboard shortcut"))
        .with_step(clear_settings_search())
        .with_step(assert_settings_nav_page_visible(
            SettingsSection::About,
            true,
        ))
        .with_step(assert_settings_nav_page_visible(
            SettingsSection::Warpify,
            true,
        ))
}
