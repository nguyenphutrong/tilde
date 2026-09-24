use std::sync::LazyLock;

use warpui::keymap::Keystroke;

pub const ACCEPT_PROMPT_SUGGESTION_KEYBINDING: &str = "terminal:accept_prompt_suggestions";

pub static REJECT_PROMPT_SUGGESTION_KEYSTROKE: LazyLock<Keystroke> = LazyLock::new(|| Keystroke {
    ctrl: true,
    key: "c".to_owned(),
    ..Default::default()
});
