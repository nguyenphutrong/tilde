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
number 75 (baseline 79); lock packages number 1,424 (baseline 1,544). Native managed-secrets behavior
remains: all 31 tests pass. All-target app Clippy passes with existing warnings.

`script/check_local_only.py` rejects `ort`, `ort-sys`, `candle-core`, `candle-nn`, `candle-onnx`, and
`tokenizers`, plus `serve-wasm`, `managed_secrets_wasm`, `input_classifier`, and
`natural_language_detection`, in manifests and lockfiles, including
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
its crash-recovery status getter, native stack, worker command, auth-overlay toggle and packaging
are now removed. The guard rejects `sentry*`, `minidumper` and `crash-handler`, including renamed,
optional target dependencies and transitive lock entries. Stored privacy settings remain; no stored
data or schema is deleted.

The crash-reporting setting, command context, telemetry field and account/GraphQL sync are removed.
Legacy defaults and TOML values remain opaque, including on logout; the legacy sync-exclusion key
remains to prevent uploading stored values. The historical schema and remote protocol are unchanged.

Repository detection no longer calls the remote daemon. Local sessions use filesystem Git detection;
remote paths never fall through to local detection, even when the same path exists locally. Local
repository watcher events, nested repositories and worktrees remain supported.

The AI credit purchase banner, its billing callbacks, experiment, telemetry and exclusive display
state are removed. Remaining pricing and request-usage contracts are unchanged; no purchase or
account behavior is replaced with a stub. Local input focus and keybindings remain.

The passive AI prompt suggestion banner, its billing/account bridge, static plan suggestion and
editable acceptance binding are removed. Zero-state prompt presets, their auto-attach/submission
path, workspace actions and exclusive integration test are also removed. Code-diff and unit-test
suggestion handlers remain. Shell-integration and ordinary Enter/Ctrl-Enter handling remain covered
by focused tests. Retired entrypoint variants were outbound metadata, not SQLite/history payloads.

Retired prompt-suggestion Cmd/Ctrl-Enter routing and its editor predicate are removed. CLI-agent
Ctrl-Enter still submits and clears the buffer only when enabled; otherwise it preserves the buffer.
Remaining Cmd-Enter remote routing and passive diff/unit-test UI constants are unchanged.

Natural-language classifier execution, its history-similarity matching, asynchronous cancellation,
heuristic crates, dictionaries and AI-only Git `difflib` dependency are removed. Explicit input modes
retain their existing serialization; shell completion parsing, aliases, history and decorations
remain. The Unicode-decoration fixture declares its builtin instead of depending on host commands.
Input starts in locked Shell mode and edits, clearing, and submission do not resume classification.
Restored input configurations are locked. Autodetection settings, their one-time migration,
mode transitions, slash command and dedicated telemetry are removed; manual AI modes remain pending.
The classifier-only prompt cache, block override marker and autodetection timestamp are removed.
Persisted queries, conversation data and input-mode serialization are unchanged.

Orphaned TUI zero-state and transcript benchmark targets are removed: both imported the deleted
AI-front-end benchmark harness. Benchmark discovery, production TUI/PTY code and unit tests remain.
This removes stale build targets, not a performance measurement.

The terminal AI welcome block, autodetection footer control, classifier command denylist and
history-matching feature flag are removed. The three eager profile/model-selector controls and their
visibility setting are removed. Legacy toolbar arrays still decode retired model-selector and NLD
items without exposing them or discarding other saved items. Historical settings and database rows
are untouched; OSC52 clipboard controls remain registered. The cloud-V2 model selector and saved
harness-model preference are removed. GUI `/model` dispatch is retired; its TUI-only registration
remains. Inline selectors and the explicit OpenModelSelector action remain for later removal.
Prompt-restoration integration scenarios retain that action; only the retired chip-toggle-close
scenario is removed.

The obsolete developer-input button bar and its empty-buffer/hover event plumbing are removed.
The surviving context-menu predicate retains SSH/subshell restrictions, the shell-mode setting and
category availability checks. Middle-click paste, file drop, shell-widget replacement and the local
prompt/editor remain; voice, attachment and context-menu tests exercise their handlers directly.

The agent input footer, cloud environment selector and CLI-footer voice interception are removed.
Local prompt chips and Kitty modifier encoding remain; unhandled modifiers return to normal routing.
The `/environment` command remains absent even with the old cloud-input flag enabled. The shared
button theme and legacy toolbar-item decoding remain for their surviving consumers.

TerminalView now owns the shell-integration footer directly. Its Enable shell integration/Dismiss
controls, pinned-input placement, alt-screen background and bootstrap keybinding remain; the
UseAgentToolbar wrapper, remote-control event forwarding and user-command toolbar setting are
removed. Dismissal removes the content and a later explicit show reopens it. Legacy CLI rich-input
submit strategies remain as separate removal debt.

AI, Drive/cloud objects, authentication/server, GraphQL, MCP, shared sessions,
and associated bootstrap/settings/UI contracts remain. Remove leaf consumers first, then their
owning contracts; do not replace removed
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

Terminal test bootstrap preserves the supplied fixture shell. Zsh `histignorespace` passes without
changing production history filtering. The 178-test input/parser run passes 174 tests; four existing
AI shortcut/changelog failures remain (queued-prompt `?`, two new-conversation shortcuts, changelog).

At the inline-menu recovery increment, 68 selected tests, all-target Clippy, TUI checking and the
Linux GUI build pass. Xvfb startup exits before creating a window: the login experiment snapshots
`PrivacySettings`, which still requests unregistered `UserWorkspaces`. No GUI success or rendered
terminal is established; remove the remaining consumer coupling rather than adding a dummy model.

Orchestration pill controls, breadcrumbs, and their pin/scroll singleton are removed. Static avatar
and label helpers remain for legacy AI surfaces; persisted pin fields and decoding remain untouched.
The capped drag-preview and restored pane-topology regressions remain runnable.

Terminal pane headers no longer expose sharing, participants, cloud cancellation, conversation
details toggles, or parent-conversation cards. Their overflow menu contains only local maximize/
restore when split; legacy viewer state cannot restore sharing actions. Titles, error/shell
indicators, close controls, and dragging remain. Conversation titles/back buttons and shared-object
configuration remain removal debt.

At the pane-header recovery increment, all eight focused tests and the Linux GUI build pass.
Xvfb startup exits 101 before creating a window: `AgentViewConversationSelection::new` requests the
unregistered `BlocklistAIHistoryModel`. Live terminal rendering remains unverified.

Agent/cloud promotional tips, cooldown timers, tip settings/flags, and tip analytics transport are
removed. Stored settings rows remain untouched. Ambient progress, errors, authentication, and the
status bar's model-fallback explanation remain legacy AI/cloud debt. The reviewed cloud-residue inventory
shrinks from 237 to 170 entries, with no additions.

Cloud composer host/harness/credential selectors, credential creation/deletion dialogs, their focus
and pane/workspace event wiring, `/host` and `/harness`, and the TUI credential revision bridge are
removed. Orchestration cards no longer create or auto-adopt credentials; they retain existing-name
catalog reads and explicit named/inherit/unset choices. An unset required cloud credential still
blocks acceptance. Stored settings rows and credential tables remain untouched. Credential catalog,
create/delete transport, and cloud orchestration remain removal debt.

Harness credential mutation APIs, credential-form metadata, and mutation events are removed.
The remaining catalog holds names only; Claude/Codex eligibility and persisted choices are preserved.
Existing-key catalog fetching/pickers and cloud execution remain removal debt. Stored rows are untouched.

Credential pickers, catalog fetching/cache/retries, and their GraphQL client operation are removed.
Model-catalog revalidation preserves explicit legacy names and inheritance; only unset credentials
are restored from persisted settings. Legacy request/settings fields and execution consumers remain
removal debt. No stored rows or tables are deleted.

Shell prompt construction no longer receives AI input/context/controller handles or subscribes to
AI history. Plan/todo chip rendering is removed; its saved enum remains decodable without a runtime
generator. Same-line layout uses PS1 and saved prompt settings directly. Local chips are preserved;
other context-chip AI actions and terminal AI constructors remain removal debt.

The unrendered agent todo popup, its terminal state, and toggle/close actions are removed.
Todo data models and history-readable stored data remain intact.

Context chips no longer construct the AI-credit-reset popup or relay queries to an agent.
The Node menu explains missing nvm without an AI installation action; local `nvm use` and
`nvm install node` commands remain unchanged.

Prompt generators use only the local prompt configuration and PS1/input settings. Saved agent and
CLI-agent footer selections remain persisted but cannot start generators. Shell bootstrap tracks
pending session IDs without timing telemetry; legacy shell timing payloads still decode. Local
bootstrap and PS1 generator suppression are tested without AI/auth/server/telemetry models.

The prompt-chip renderer no longer owns ambient-agent state or unreachable agent-only styling.
Local chip colors, fonts, margins, hover behavior, and CLI-agent interaction guards are preserved.

Bootstrap success/slow telemetry and bootstrap-content collection are removed. The real timeout
handler still warns locally, unhides SSH output, and opens the auto-dismiss banner; its regression
test now exercises that handler. Legacy shell timing payloads and local session behavior remain.

Terminal teardown no longer sends abandonment telemetry or retains a server, background executor,
privacy snapshot, or bootstrap timestamp for it. Terminal/input constructors and pane resources no
longer carry that server handle. Nonblocking local teardown logging remains unchanged.

Background block creation and SSH bootstrap no longer emit telemetry-only events. Background output
insertion, block-height updates, completion events, and the supported-shell SSH bootstrap timer remain.

Passive AI suggestion dispatch and keybindings are removed. Down retains local completion/history
navigation and editor movement. Editor Ctrl-C no longer traverses workspace or AI history: it retains
Vim handling, Windows selected-text copying, and undoable input clearing. Other terminal-level agent
Ctrl-C branches remain removal debt.

Editor/input attachment ingestion, file-picker actions, image-processing futures and limits, and
Figma-PNG detection are removed. Paste uses clipboard text/path conversion; all dropped paths,
including images, are transformed and shell-escaped. SSH uploads and the separate long-running CLI
image relay remain. Backend attachments and persisted attachment chips remain removal debt. This
increment also removes the unreachable image-picker helper/test missed in the earlier billing-alert
recovery; the original transcript deleted them before attachment ingestion.

Typing `@` no longer automatically opens the AI context menu; its package-installer classifier is
removed. Alias expansion, highlighting, and shell completions remain. The literal-input regression
enables the old menu flags/preferences explicitly. Explicit menu construction/dispatch remains for
the next removal increment.

The explicit AI context menu is detached from editor/input construction, events, navigation,
rendering, and telemetry. Its diff-attachment loader and ambient/shared/CLI menu updates are removed.
Editor input-mode bookkeeping remains for existing keymap behavior, without constructing a menu.
The unused search adapters are the next deletion target; local completions and history remain.
