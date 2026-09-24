# Tilde local-only maintenance

Tilde is a local terminal, not an ADE or an agent hub. **Cloud/AI removal is incomplete.**
Disabling a feature does not remove its source or dependencies, and this audit is not proof of
network silence. GitHub Releases updates and necessary local networking remain.

## Recovery and upstream policy

Recovery starts from [v0.1.4](https://github.com/nguyenphutrong/tilde/commit/42cab8f9), using the
[lost work transcript](https://ampcode.com/threads/T-01a090cf-5832-75de-bf88-03f6638f68e6).
The original upstream comparison used ancestor
[a45efa09](https://github.com/warpdotdev/warp/commit/a45efa093ef2bfcb42717b1d29f2966651a68545)
and tip [76761c22](https://github.com/warpdotdev/warp/commit/76761c22e3251307949c24e3ade134e575c4e426).

Never merge upstream wholesale. Review full diffs and tests before importing terminal grid/ANSI,
editor, WarpUI/rendering, PTY/shell, completion/history, or platform correctness fixes. Reject
AI/Oz/MCP/agents, teams, collaboration, accounts/auth, cloud/server, GraphQL and telemetry additions,
including additions hidden inside otherwise eligible commits. New product UI is not a platform fix.
Check ancestry and current behavior; patch conflicts do not prove a fix is already present. Port
only the relevant behavior, reproduce the boundary case, and commit each verified coherent change.
Linux compilation does not verify Windows RDP, Windows shells, or macOS runtime behavior.

## Recovered correctness fixes

| Upstream | Behavior and recovery evidence |
| --- | --- |
| [a7326f8f](https://github.com/warpdotdev/warp/commit/a7326f8fecd6a52050a735da1e9979822b46c71b) | Wide-character EOL promotion; five regressions failed before the fix, then 557 terminal tests passed with two existing skips. |
| [58d94a53](https://github.com/warpdotdev/warp/commit/58d94a53b308b8e7ab1541f194ce5ffef29742c6) | Reject reversed smart-select bounds before clamp; regression failed before, both tests passed after. |
| [83e270f1](https://github.com/warpdotdev/warp/commit/83e270f1d17a9505f7aa32fdcccf9067752faa40) | Use glyph advance when bounds are absent; three regressions failed before, five selected font tests passed after. Windows rendering remains unverified. |
| [ad28e590](https://github.com/warpdotdev/warp/commit/ad28e590f8781cac00c8b762425a19971a8549da) | Route Windows RDP validation failure through existing device-loss recovery. Linux lint and resource tests passed; Windows RDP remains unverified. |
| [17f43202](https://github.com/warpdotdev/warp/commit/17f4320276f9baaf272119f0c319db96a3aba183) | Avoid Bash PS1 double expansion. Syntax/direct branch checks passed: honor mode expands zero times, preview once. Integration build has nine pre-existing stale AI/launch-config errors. |
| [742ca57b](https://github.com/warpdotdev/warp/commit/742ca57b50a034d5f7ddcdd9e7791367ab7624c0) | Remove disposable editor layout caches, retaining frame/placeholder caches, BOM stripping and fallback requests. 500 editor tests passed before and after; platform-boundary BOM regression passed. Recovery includes the corrected harness directly. |
| [9589305a](https://github.com/warpdotdev/warp/commit/9589305a) | Omit unsupported PowerShell login flag on Windows, retaining Unix login flags. Five Linux executor tests passed; the Windows PowerShell 5.1 execution test remains unrun here. |

Shell widget handoff ([bf2364bc](https://github.com/warpdotdev/warp/commit/bf2364bc99c118f562e21b5529c1db492b0b939f))
is recovered across shell bootstrap, DCS, input, and workspace key routing. Matching selections restore
the draft without executing it; cancellation and stale sessions preserve the original input. Fish
uses whole-line replacement; Bash/Zsh splice at the captured cursor. Missing widgets leave Ctrl-T as a
raw PTY keystroke. The Fish helper checks execute shipped functions with a mocked picker; real
fzf/atuin GUI interaction remains unverified. Native completion generators, Bash shell-plugin payload
preservation, and the cosmic-text fallback pin remain unported candidates.

## Removed dependencies and guards

ORT, Candle/ONNX inference, model initialization/features, classifier evaluation binary, three BERT
models and tokenizer assets are removed. Model assets fell from 53,470,541 bytes to zero. These
optional-runtime deletions do not imply an equivalent reduction in shipped binary size. Cargo/LFS
caches and Git history are not erased.

The standalone `serve-wasm` shared-session/Drive server and `managed_secrets_wasm` credential wrapper,
their scripts/profiles, and the wrapper-only sealing API/test are removed. Workspace members now
number 77 (baseline 79); lock packages number 1,480 (baseline 1,544). Native managed-secrets behavior
remains: all 31 tests pass. All-target app Clippy passes with existing warnings.

`script/check_local_only.py` rejects `ort`, `ort-sys`, `candle-core`, `candle-nn`, `candle-onnx`, and
`tokenizers`, plus `serve-wasm` and `managed_secrets_wasm`, in manifests and lockfiles, including
renamed, optional, target, build and dev dependencies.
`script/local_only_residue.json` initially records 280 occurrences: 111 dependency declarations,
133 endpoint lines and 36 lock entries. This is a debt inventory, not approved product functionality.
Endpoint lines are hashed to avoid exposing credentials. Changes and duplicates fail the guard;
review removals and shrink the inventory, never bless new cloud entries. The guard is not a general
secret scanner and cannot detect every dynamically assembled host, new domain, or external client.

The retired palette A/B experiment no longer runs at startup. Local full-text search stays enabled
directly, avoiding the experiment's synchronous telemetry lookup of cloud workspace state.
Release notes use Tilde's GitHub release-tag endpoint directly, without Warp `ServerApi`; unpublished
local versions return without making a request. This preserves the GitHub Releases updater.

Cloud Rules panes, their manager/views, Drive rows, slash commands, and integration routes are removed.
Legacy Rules snapshots remain deserializable and restore a local terminal, including when the old
Rules flag is enabled. Historical Rules data codecs and migrations remain; the local project-rules
file command is preserved. This does not remove the remaining AI rule/cloud synchronization code.

The left panel contains only the local file tree and project search. Drive and conversation-list
views are no longer constructed or polled. Legacy saved cloud-tab names remain readable and restore
the local file tree; stored cloud preferences and historical data are not erased.

Agent Management views, the agent-type selector, and the Oz setup guide are removed. Saved agent
filters round-trip unchanged through workspace snapshots without applying filters or fetching tasks.
The setup guide's three endpoint entries are removed from the residue inventory.

Code review no longer submits comments or attaches selected text, files, or hunks to agents. Its
send/debug controls, keybindings, AI-credit subscription, and codebase-indexing zero state are removed.
Local diff editing/revert, Git operations, comment edit/copy/delete, the focused-terminal provider,
and Cmd/Ctrl-Enter for saving comments remain. Terminal-owned diff attachments remain for later removal.

LSP detection, installation, and startup use a dedicated HTTP client, not `ServerApiProvider` or its
auth hooks. Language-server downloads remain enabled with the existing HTTP defaults and interactive
PATH lookup. Code-review tests construct their views without the server provider.

Agent mailbox, toast implementations, notification model/items, unread tab indicators, and Oz
desktop-notification summaries are removed along with their actions and subscriptions. Ordinary
terminal/update toasts, CLI desktop notifications, local tab badges, and synchronized-input indicators
remain. Saved toolbar notification discriminators remain readable but unavailable.

Workspace no longer subscribes to Drive update/activity, staging-auth, shared-session, or bonus-credit
notifications. Cloud toast formatting and the bonus-credit notification model are deleted. Local
autoupdate/terminal toasts, database tables, and persisted bonus-credit settings remain unchanged.

The Drive import/upload modal, directory tree, parsing, upload queue, and personal/team menu actions
are removed. Local terminal-configuration import and native file/directory pickers remain; the local
Alacritty importer still uses `async-recursion`. The deleted import endpoint leaves the residue inventory.

Terminal secret rendering depends on local safe-mode settings, without account-policy lookups.
Terminal models/blocks no longer carry AI-UGC telemetry state or expand serialized output for telemetry;
the existing 50-line local output bound and forced secret-redaction path remain. Local and legacy
secret-display preferences are preserved, with no database-schema or persisted-record changes.

Local and SSH-wrapper InitShell events now reach ordinary PTY bootstrap directly, even with the old
remote-server flag enabled. The remote-server bootstrap controller and daemon bootstrap notification
are removed; ordinary PTY writing, resizing, interrupts, and shell initialization remain.

The unused SSH daemon transport, account-token adapter, Oz archive download/cache, and SCP fallback
installer are removed. Ordinary SSH/SCP support, existing caches, and remote installations are untouched.

SSH extension installation settings UI and daemon auth-token, crash-preference, and AI-limit forwarding
are removed. SSH shell integration and reuse of existing ControlMaster remain; legacy install-mode
records are retained. A typed Settings-view regression checks actions and observers without a server.

Terminal SSH-extension install/skip blocks, loading footers, failure banners, focus interception,
and manager setup/telemetry subscriptions are removed. Ordinary local/SSH shell bootstrap remains;
session-level daemon executor/setup-state plumbing and daemon-only integration fixtures are removed.
Local, ControlMaster SSH, and in-band SSH executor selection is tested without a remote-server model,
even with the legacy daemon flag enabled. Ordinary SSH integration tests remain.

Extension-only ExitShell forwarding, tmux deprecation banners, and installation preferences are
removed. The ANSI hook remains parseable through its default handler. Ordinary SSH opt-out migration
remains; a TOML regression verifies that retired extension values survive settings loading and writes.

Background passive-suggestion models and their terminal subscriptions are removed, including passive
code-diff creation and request cancellation on Clear Blocks. Their orphaned controller request builders,
query-suggestion endpoint/schema, and request-only tests are removed. Existing prompt UI, query
prediction, and request-tool override state remain; local PTY and completion paths are unchanged.
This removal does not change persistence schemas or establish a usable-GUI or network-silent result.

AI query ghost-text prediction is removed: input debounce/request state, endpoint/schema, setting,
feature flag, and setting telemetry. A registration test fences the retired setting; historical
values remain opaque. Local history/completion and prompt/banner UI remain, as does AI next-command
prediction pending its separate removal.

Local history lookup, directory-prioritized autosuggestions, and completion-based argument validation
now live under `terminal::autosuggestions`, with their existing tests. This is a behavior-preserving
move; the remaining AI model consumes the local helpers rather than owning them.

## Remaining scope and data safety

AI next-command prediction, its transport, cycling UI, settings and telemetry are removed. Local
autosuggestions still match history and following commands within a session; they no longer gather
preceding-command context for an LLM. The retained block-content width regression lives with the
terminal block tests. Stored settings and serialized billing policy data are not migrated or deleted.

A local-history regression replays real migrations into disposable in-memory SQLite and checks
session boundaries, empty-command skipping, directory/exit-status filtering and chronological order
with interleaved sessions. CI runs the autosuggestion module.

The inline AI conversation menu, its query ranking, entry points and telemetry are removed.
`/conversations` is absent from GUI and TUI command registries. The retired serialized menu-height
key remains so saved shell-history heights still load and round-trip without data loss. Local
history menus, completion, and the separate cloud prompt-history overlay remain.

HTTP OpenTelemetry span propagation and its dependencies are removed. The guard rejects the entire
`opentelemetry*` and `tracing-opentelemetry*` package families, including renamed and transitive
dependencies. Local diagnostics remain. Tests build a GitHub updater request and execute loopback
HTTP without cloud trace headers; they do not assert network silence or remove other Warp headers.

Shared error reporting, logging and PTY log forwarding no longer capture Sentry events or
breadcrumbs. Local Error/Warn classification, error chains, extra context, once-per-run suppression,
rotation and panic logging remain.

App-level Sentry forwarding and lifecycle callers are removed, including SQLite/plugin logs,
profiling attachments, user/experiment/settings/GPU metadata and Cocoa PTY hooks. SQLite retains its
FFI unwind boundary; explicit local profile/sample saving and crash recovery remain. The SDK and
its crash-recovery status getter, native stack, packaging and stored privacy settings remain until
their owning removal increment. No stored data or schema is deleted.

AI, Drive/cloud objects, authentication/server, GraphQL, MCP, shared sessions, Sentry,
and associated bootstrap/settings/UI contracts remain. Classifier heuristics and serialized decision
variants still exist. Remove leaf consumers first, then their owning contracts; do not replace removed
consumers with dummy singletons or stubs. Preserve local shell/PTY, history, completion, editor and Git
helpers. Retain historical migrations and unknown legacy DB rows opaquely; do not DROP old tables.
Migration replay tests use disposable SQLite data, not every historical production database.

## Verification

```sh
python3 script/test_local_only.py
python3 script/check_local_only.py
./script/format --check
cargo check --locked -p warp --bin tilde --features gui
cargo clippy --locked -p warp --all-targets
cargo nextest run --locked -p warp_terminal -p warp_editor -p warpui_core -p persistence
cargo test --locked -p warp_cli --lib
cargo test --locked -p warp_tui --lib
```

Recovery uses owner-authorized warning-allowing Clippy plus focused checks for each commit/push.
Strict presubmit `-D warnings` fails on pre-existing unused/dead-code warnings; these are not fixed or
suppressed as part of recovery. Ordinary Clippy passing does not mean warning-clean. The obsolete AI
assistant integration test is removed and seven launch-config constructors match their current API;
all-target integration checking and Clippy pass without removing local launch-config assertions.
Linux build and GUI smoke are separate verification requirements; unit checks do not substitute for
them. Integration compilation does not imply a successful real-display test run.

At the inline-menu recovery increment, 68 selected tests, all-target Clippy, TUI checking and the
Linux GUI build pass. Xvfb startup exits before creating a window: the login experiment snapshots
`PrivacySettings`, which still requests unregistered `UserWorkspaces`. No GUI success or rendered
terminal is established; remove the remaining consumer coupling rather than adding a dummy model.
