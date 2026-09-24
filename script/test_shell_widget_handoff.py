#!/usr/bin/env python3
"""Exercise the shipped Fish handoff helpers without a GUI or external picker."""

import json
from pathlib import Path
import subprocess
import unittest


SOURCE = (Path(__file__).resolve().parents[1] / "app/assets/bundled/bootstrap/fish.sh").read_text()


def function(name):
    start = SOURCE.index(f"function {name}\n")
    return SOURCE[start:SOURCE.index("\nend", start) + len("\nend")]


class ShellWidgetHandoffTests(unittest.TestCase):
    def test_fish_hex_draft_preserves_bytes(self):
        for draft in ("", "héllo\nsecond line\n", "x; $(echo should-not-run)"):
            with self.subTest(draft=draft):
                result = subprocess.run(
                    ["fish", "-c", function("warp_hex_decode_string") +
                     "\nwarp_hex_decode_string $argv[1]", "--", draft.encode().hex()],
                    check=True, capture_output=True, stdin=subprocess.DEVNULL, timeout=10,
                )
                self.assertEqual(result.stderr, b"")
                self.assertEqual(result.stdout, draft.encode())

    def test_fish_ctrl_t_selection_and_cancel(self):
        helpers = "\n".join(function(name) for name in (
            "warp_hex_decode_string", "warp_escape_json", "warp_ctrl_t_widget_result",
            "warp_run_external_ctrl_t_widget",
        ))
        script = helpers + """
function commandline
    switch "$argv[1]"
        case '-r'
            set -g line "$argv[-1]"
        case '-C'
            set -g cursor "$argv[-1]"
        case '*'
            printf '%s\\n' "$line"
    end
end
function fzf-file-widget
    test "$line" = "$expected_draft"; or echo 'wrong draft' >&2
    test "$cursor" = 4; or echo 'wrong cursor' >&2
    if test "$selection" != cancel
        set -g line (warp_hex_decode_string "$selection" | string collect --no-trim-newlines)
    end
end
function warp_send_json_message
    printf '%s' "$argv[1]"
end
set -g WARP_SESSION_ID 123
set -g _WARP_EXTERNAL_CTRL_T_WIDGET fzf-file-widget
set -g selection "$argv[2]"
set -g expected_draft (warp_hex_decode_string "$argv[1]" | string collect --no-trim-newlines)
warp_run_external_ctrl_t_widget "4:$argv[1]"
"""
        draft = "vim héllo\nsecond line"
        for selection in (None, "vim chosen\nother path"):
            with self.subTest(selection=selection):
                result = subprocess.run(
                    ["fish", "-c", script, "--", draft.encode().hex(),
                     "cancel" if selection is None else selection.encode().hex()],
                    check=True, capture_output=True, stdin=subprocess.DEVNULL, timeout=10,
                )
                self.assertEqual(result.stderr, b"")
                self.assertEqual(json.loads(result.stdout), {
                    "hook": "ExternalShellWidgetSelection",
                    "value": {"buffer": selection or "", "session_id": 123},
                })

    def test_fish_ctrl_r_detects_only_available_allowlisted_widgets(self):
        helpers = "\n".join(function(name) for name in (
            "warp_escape_json", "warp_bootstrapped", "warp_run_external_ctrl_r_widget",
        ))
        script = helpers + """
function warp_external_ctrl_r_widget
    printf '%s' "$widget"
end
function warp_external_ctrl_t_widget
    return 1
end
function warp_send_json_message
    printf '%s\\n' "$argv[1]"
end
function commandline
    if test "$argv[1]" = -r
        set -g line "$argv[-1]"
    else
        printf '%s\\n' "$line"
    end
end
set -g widget "$argv[1]"
if test "$argv[2]" = present
    function $widget
        set -g line 'echo selected; false'
    end
end
set -g WARP_SESSION_ID 123
set -g fish_private_mode 1
warp_bootstrapped
if test -n "$_WARP_EXTERNAL_CTRL_R_WIDGET"
    warp_run_external_ctrl_r_widget
end
"""
        for widget in ("fzf-history-widget", "_fzf_search_history", "untrusted_widget"):
            for present in (False, True):
                with self.subTest(widget=widget, present=present):
                    result = subprocess.run(
                        ["fish", "--no-config", "-c", script, "--", widget,
                         "present" if present else "missing"],
                        check=True, capture_output=True, stdin=subprocess.DEVNULL, timeout=10,
                    )
                    self.assertEqual(result.stderr, b"")
                    messages = [json.loads(line) for line in result.stdout.splitlines()]
                    allowed = present and widget != "untrusted_widget"
                    self.assertEqual(messages[0]["value"]["shell_plugins"],
                                     "external_ctrl_r_history" if allowed else "")
                    self.assertEqual(len(messages), 2 if allowed else 1)
                    if allowed:
                        self.assertEqual(messages[1], {
                            "hook": "ExternalShellWidgetSelection",
                            "value": {"buffer": "echo selected; false", "session_id": 123},
                        })


if __name__ == "__main__":
    unittest.main()
