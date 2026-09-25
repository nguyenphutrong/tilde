pub mod editor;
pub mod toolbar_item;

use pathfinder_color::ColorU;
use warp_core::ui::color::blend::Blend;
use warp_core::ui::theme::Fill;
use warp_core::ui::theme::color::internal_colors;

use crate::appearance::Appearance;
use crate::view_components::action_button::ActionButtonTheme;

pub(crate) struct AgentInputButtonTheme;

impl ActionButtonTheme for AgentInputButtonTheme {
    fn background(&self, hovered: bool, appearance: &Appearance) -> Option<Fill> {
        // Solid surface fills keep the button readable even when its parent
        // isn't `theme.background()` (for example, over an alt-screen CLI agent).
        let theme = appearance.theme();
        Some(if hovered {
            theme.surface_2()
        } else {
            theme.surface_1()
        })
    }

    fn text_color(
        &self,
        _hovered: bool,
        background: Option<Fill>,
        appearance: &Appearance,
    ) -> ColorU {
        // If a caller overrides `background()` with a translucent fill, blend
        // it over `surface_1` so text contrast is computed against the actual
        // rendered color rather than the raw overlay.
        let base_bg = appearance.theme().surface_1();
        let effective_bg = background
            .map(|overlay| base_bg.blend(&overlay))
            .unwrap_or(base_bg);

        appearance.theme().sub_text_color(effective_bg).into_solid()
    }

    fn border(&self, appearance: &Appearance) -> Option<ColorU> {
        Some(internal_colors::neutral_3(appearance.theme()))
    }

    fn should_opt_out_of_contrast_adjustment(&self) -> bool {
        true
    }

    fn font_properties(&self) -> Option<warpui::fonts::Properties> {
        if crate::features::FeatureFlag::CloudModeInputV2.is_enabled() {
            Some(warpui::fonts::Properties {
                weight: warpui::fonts::Weight::Semibold,
                ..Default::default()
            })
        } else {
            None
        }
    }
}
