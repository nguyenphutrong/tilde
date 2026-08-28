use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use lazy_static::lazy_static;
use parking_lot::Mutex;
use pathfinder_geometry::rect::RectF;
use pathfinder_geometry::vector::{Vector2F, vec2f};
use serde::{Deserialize, Serialize};
use settings::Setting as _;
use warp_core::context_flag::ContextFlag;
use warpui::elements::Border;
use warpui::keymap::{EditableBinding, FixedBinding};
use warpui::platform::{WindowBounds, WindowStyle};
use warpui::presenter::ChildView;
use warpui::rendering::OnGPUDeviceSelected;
use warpui::windowing::WindowManager;
use warpui::{
    AddWindowOptions, AppContext, DisplayId, Element, Entity, EntityId, FocusContext,
    NextNewWindowsHasThisWindowsBoundsUponClose, SingletonEntity, TypedActionView, View,
    ViewContext, ViewHandle, WindowId, id,
};

use crate::ai::blocklist::SerializedBlockListItem;
use crate::app_state::{AppState, PaneUuid, WindowSnapshot};
use crate::appearance::Appearance;
use crate::interval_timer::IntervalTimer;
use crate::launch_configs::launch_config;
use crate::pane_group::{NewTerminalOptions, PanesLayout};
use crate::settings::QuakeModeSettings;
use crate::settings_view::{SettingsSection, flags};
use crate::terminal::available_shells::AvailableShell;
use crate::terminal::general_settings::GeneralSettings;
use crate::terminal::keys_settings::KeysSettings;
use crate::terminal::shell::ShellType;
use crate::terminal::view::cell_size_and_padding;
use crate::themes::theme::{AnsiColorIdentifier, Blend, Fill};
use crate::uri::OpenSettingsArgs;
use crate::util::bindings::{self, is_binding_pty_compliant};
use crate::window_settings::WindowSettings;
use crate::workspace::{PaneViewLocator, Workspace, WorkspaceAction, WorkspaceRegistry};
use crate::{
    ChannelState, GlobalResourceHandles, GlobalResourceHandlesProvider, UpdateQuakeModeEventArg,
};

const WINDOW_TITLE: &str = "Warp";

lazy_static! {
    static ref FALLBACK_WINDOW_SIZE: Vector2F = vec2f(800.0, 600.0);
    static ref QUAKE_STATE: Arc<Mutex<Option<QuakeModeState>>> = Arc::new(Mutex::new(None));
}

pub(crate) fn unthemed_window_border() -> Border {
    if cfg!(all(not(target_os = "macos"), not(target_family = "wasm"))) {
        Border::all(1.).with_border_fill(Fill::black().blend(&Fill::white().with_opacity(15)))
    } else {
        Border::all(1.).with_border_fill(Fill::black().with_opacity(0))
    }
}

#[derive(Debug, Clone)]
enum WindowState {
    Open,
    PendingOpen,
    Hidden,
}

#[derive(Debug, Clone)]
pub struct QuakeModeState {
    window_state: WindowState,
    window_id: WindowId,
    active_display_id: DisplayId,
}

struct QuakeModeFrameConfig {
    display_id: DisplayId,
    window_bounds: RectF,
}

#[derive(Debug)]
enum QuakeModeMoveTrigger {
    ScreenConfigurationChange,
    ActiveScreenSetting,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Hash,
    Eq,
    PartialEq,
    Deserialize,
    Serialize,
    Default,
    schemars::JsonSchema,
    settings_value::SettingsValue,
)]
#[schemars(
    description = "Screen edge to pin the hotkey window to.",
    rename_all = "snake_case"
)]
pub enum QuakeModePinPosition {
    #[default]
    Top,
    Bottom,
    Left,
    Right,
}

pub struct OpenFromRestoredArg {
    pub app_state: Option<AppState>,
}

pub struct OpenLaunchConfigArg {
    pub launch_config: launch_config::LaunchConfig,
    pub open_in_active_window: bool,
}

pub struct OpenPath {
    pub path: PathBuf,
}

pub struct SubshellCommandArg {
    pub command: String,
    pub shell_type: Option<ShellType>,
}

pub fn init(app: &mut AppContext) {
    app.register_binding_validator::<RootView>(is_binding_pty_compliant);
    app.add_global_action("root_view:open_from_restored", open_from_restored);
    app.add_global_action("root_view:open_new", open_new);
    app.add_global_action("root_view:open_new_with_shell", open_new_with_shell);
    app.add_global_action("root_view:open_new_from_path", |arg, ctx| {
        open_new_from_path(arg, ctx);
    });
    app.add_global_action(
        "root_view:open_new_tab_insert_subshell_command_and_bootstrap_if_supported",
        open_new_tab_insert_subshell_command_and_bootstrap_if_supported,
    );
    app.add_global_action("root_view:open_launch_config", open_launch_config);
    app.add_global_action(
        "root_view:toggle_quake_mode_window",
        toggle_quake_mode_window,
    );
    app.add_global_action(
        "root_view:show_or_hide_non_quake_mode_windows",
        show_or_hide_non_quake_mode_windows,
    );
    app.add_global_action("root_view:update_quake_mode_state", update_quake_mode_state);
    app.add_global_action(
        "root_view:move_quake_mode_window_from_screen_change",
        move_quake_mode_window_from_screen_change,
    );
    app.add_action(
        "root_view:add_session_at_path",
        |view: &mut RootView, path: &PathBuf, ctx: &mut ViewContext<RootView>| {
            view.add_session_at_path(path, ctx)
        },
    );
    app.add_action(
        "root_view:handle_notification_click",
        RootView::handle_notification_click,
    );
    app.add_action(
        "root_view:handle_pane_navigation_event",
        RootView::focus_pane,
    );
    app.add_action(
        "root_view:activate_tab_by_pane_group_id",
        RootView::activate_tab_by_pane_group_id,
    );
    app.add_action("root_view:close_window", RootView::close_window);
    app.add_action("root_view:minimize_window", RootView::minimize_window);
    app.add_action(
        "root_view:toggle_maximize_window",
        RootView::toggle_maximize_window,
    );
    app.add_action("root_view:toggle_fullscreen", RootView::toggle_fullscreen);
    app.add_global_action(
        "root_view:open_settings_page_in_new_window",
        open_settings_page_in_new_window,
    );
    app.add_action(
        "root_view:open_settings_page_in_existing_window",
        RootView::open_settings_page_in_existing_window,
    );
    app.add_global_action(
        "root_view:open_settings_in_new_window",
        open_settings_in_new_window,
    );
    app.add_action(
        "root_view:open_settings_in_existing_window",
        RootView::open_settings_in_existing_window,
    );

    app.register_fixed_bindings([
        FixedBinding::empty(
            "Hide All Windows",
            RootViewAction::ShowOrHideNonQuakeModeWindows,
            id!("RootView") & id!(flags::ACTIVATION_HOTKEY_FLAG),
        ),
        FixedBinding::empty(
            "Show Dedicated Hotkey Window",
            RootViewAction::ToggleQuakeModeWindow,
            id!("RootView")
                & id!(flags::QUAKE_MODE_ENABLED_CONTEXT_FLAG)
                & !id!(flags::QUAKE_WINDOW_OPEN_FLAG),
        ),
        FixedBinding::empty(
            "Hide Dedicated Hotkey Window",
            RootViewAction::ToggleQuakeModeWindow,
            id!("RootView")
                & id!(flags::QUAKE_MODE_ENABLED_CONTEXT_FLAG)
                & id!(flags::QUAKE_WINDOW_OPEN_FLAG),
        ),
    ]);
    app.register_editable_bindings([EditableBinding::new(
        "root_view:toggle_fullscreen",
        "Toggle fullscreen",
        RootViewAction::ToggleFullscreen,
    )
    .with_group(bindings::BindingGroup::Navigation.as_str())
    .with_context_predicate(id!("RootView"))
    .with_linux_or_windows_key_binding("f11")]);
}

fn maybe_register_global_window_shortcuts(
    global_resource_handles: GlobalResourceHandles,
    ctx: &mut AppContext,
) {
    if let Some(key) = KeysSettings::as_ref(ctx)
        .quake_mode_settings
        .keybinding
        .clone()
        .filter(|_| *KeysSettings::as_ref(ctx).quake_mode_enabled)
    {
        ctx.register_global_shortcut(
            key,
            "root_view:toggle_quake_mode_window",
            global_resource_handles,
        );
    }

    if let Some(key) = KeysSettings::as_ref(ctx)
        .activation_hotkey_keybinding
        .clone()
        .filter(|_| *KeysSettings::as_ref(ctx).activation_hotkey_enabled)
    {
        ctx.register_global_shortcut(key, "root_view:show_or_hide_non_quake_mode_windows", ());
    }
}

fn active_workspace(ctx: &mut AppContext) -> Option<ViewHandle<Workspace>> {
    let window_id = ctx.windows().active_window()?;
    WorkspaceRegistry::as_ref(ctx).get(window_id, ctx)
}

fn open_launch_config(arg: &OpenLaunchConfigArg, ctx: &mut AppContext) {
    let active_workspace = active_workspace(ctx);
    if arg.launch_config.windows.is_empty() {
        open_new(&(), ctx);
    } else if arg.open_in_active_window
        && arg.launch_config.windows.len() == 1
        && let Some(workspace) = active_workspace
    {
        workspace.update(ctx, |workspace, ctx| {
            workspace.open_launch_config_window(arg.launch_config.windows[0].clone(), ctx);
        });
    } else {
        let active_index = arg.launch_config.active_window_index;
        for (index, window_template) in arg.launch_config.windows.iter().enumerate() {
            if Some(index) != active_index {
                open_new_with_workspace_source(
                    NewWorkspaceSource::FromTemplate {
                        window_template: window_template.clone(),
                    },
                    ctx,
                );
            }
        }
        if let Some(window_template) =
            active_index.and_then(|index| arg.launch_config.windows.get(index))
        {
            open_new_with_workspace_source(
                NewWorkspaceSource::FromTemplate {
                    window_template: window_template.clone(),
                },
                ctx,
            );
        }
    }
}

pub fn create_transferred_window(
    transferred_tab: crate::workspace::view::TransferredTab,
    source_window_id: WindowId,
    window_size: Vector2F,
    window_position: Vector2F,
    is_tab_drag_preview: bool,
    ctx: &mut AppContext,
) -> WindowId {
    let global_resource_handles = GlobalResourceHandlesProvider::handle(ctx)
        .as_ref(ctx)
        .get()
        .clone();
    let window_settings = WindowSettings::handle(ctx).as_ref(ctx);
    let window_bounds = WindowBounds::ExactPosition(RectF::new(window_position, window_size));
    let window_style = if is_tab_drag_preview {
        WindowStyle::PositionedNoFocus
    } else {
        WindowStyle::Normal
    };

    let (new_window_id, _) = ctx.add_window(
        AddWindowOptions {
            window_style,
            window_bounds,
            title: Some(WINDOW_TITLE.to_owned()),
            background_blur_radius_pixels: Some(*window_settings.background_blur_radius),
            background_blur_texture: *window_settings.background_blur_texture,
            on_gpu_driver_selected: on_gpu_driver_selected_callback(),
            ..Default::default()
        },
        |ctx| {
            let mut view = RootView::new(
                global_resource_handles.clone(),
                NewWorkspaceSource::TransferredTab {
                    source_window_id,
                    tab_color: transferred_tab.color,
                    custom_title: transferred_tab.custom_title.clone(),
                    left_panel_open: transferred_tab.left_panel_open,
                    vertical_tabs_panel_open: transferred_tab.vertical_tabs_panel_open,
                    right_panel_open: transferred_tab.right_panel_open,
                    is_right_panel_maximized: transferred_tab.is_right_panel_maximized,
                    is_tab_drag_preview,
                },
                ctx,
            );
            if !is_tab_drag_preview {
                view.focus(ctx);
            }
            view
        },
    );

    ctx.transfer_view_tree_to_window(
        transferred_tab.pane_group.id(),
        source_window_id,
        new_window_id,
    );
    if let Some(new_workspace) = WorkspaceRegistry::as_ref(ctx).get(new_window_id, ctx) {
        new_workspace.update(ctx, |workspace, ctx| {
            workspace.adopt_transferred_pane_group(transferred_tab.pane_group, ctx);
        });
    } else {
        log::warn!("Failed to find workspace in newly created window {new_window_id:?}");
    }
    new_window_id
}

#[cfg(feature = "crash_reporting")]
fn on_gpu_driver_selected_callback() -> Option<Box<OnGPUDeviceSelected>> {
    Some(Box::new(|gpu_device_info| {
        crate::crash_reporting::set_gpu_device_info(gpu_device_info)
    }))
}

#[cfg(not(feature = "crash_reporting"))]
fn on_gpu_driver_selected_callback() -> Option<Box<OnGPUDeviceSelected>> {
    None
}

fn open_from_restored(arg: &OpenFromRestoredArg, ctx: &mut AppContext) {
    let global_resource_handles = GlobalResourceHandlesProvider::as_ref(ctx).get().clone();
    IntervalTimer::handle(ctx).update(ctx, |timer, _| {
        timer.mark_interval_end("HANDLING_OPEN_ACTION");
    });

    let Some(app_state) = &arg.app_state else {
        return;
    };
    maybe_register_global_window_shortcuts(global_resource_handles.clone(), ctx);
    if !*GeneralSettings::as_ref(ctx).restore_session {
        return;
    }

    let (background_blur_radius_pixels, background_blur_texture) = {
        let window_settings = WindowSettings::as_ref(ctx);
        (
            Some(*window_settings.background_blur_radius),
            *window_settings.background_blur_texture,
        )
    };
    let mut active_index = None;
    let mut normal_window_count = 0;
    for (index, window) in app_state.windows.iter().enumerate() {
        if window.quake_mode {
            if cfg!(windows) {
                continue;
            }
            let frame = quake_mode_config(
                &KeysSettings::as_ref(ctx)
                    .quake_mode_settings
                    .value()
                    .clone(),
                ctx,
            );
            let (window_id, _) = ctx.add_window(
                AddWindowOptions {
                    window_style: WindowStyle::Pin,
                    window_bounds: WindowBounds::ExactPosition(frame.window_bounds),
                    title: Some(WINDOW_TITLE.to_owned()),
                    fullscreen_state: window.fullscreen_state,
                    background_blur_radius_pixels,
                    background_blur_texture,
                    anchor_new_windows_from_closed_position:
                        NextNewWindowsHasThisWindowsBoundsUponClose::No,
                    on_gpu_driver_selected: on_gpu_driver_selected_callback(),
                    window_instance: Some(ChannelState::app_id().to_string() + "-hotkey"),
                },
                |ctx| {
                    let mut view = RootView::new(
                        global_resource_handles.clone(),
                        NewWorkspaceSource::Restored {
                            window_snapshot: window.clone(),
                            block_lists: app_state.block_lists.clone(),
                        },
                        ctx,
                    );
                    view.focus(ctx);
                    view
                },
            );
            ctx.windows().hide_window(window_id);
            set_quake_mode(Some(QuakeModeState {
                window_state: WindowState::Hidden,
                window_id,
                active_display_id: frame.display_id,
            }));
            continue;
        }

        normal_window_count += 1;
        if app_state.active_window_index == Some(index) {
            active_index = Some(index);
            continue;
        }
        add_restored_window(
            window,
            app_state.block_lists.clone(),
            global_resource_handles.clone(),
            background_blur_radius_pixels,
            background_blur_texture,
            ctx,
        );
    }

    if normal_window_count == 0 {
        let options = default_window_options(WindowSettings::as_ref(ctx), ctx);
        ctx.add_window(options, |ctx| {
            let mut view = RootView::new(
                global_resource_handles.clone(),
                NewWorkspaceSource::Empty {
                    previous_active_window: None,
                    shell: None,
                },
                ctx,
            );
            view.focus(ctx);
            view
        });
    }

    if let Some(window) = active_index.and_then(|index| app_state.windows.get(index)) {
        add_restored_window(
            window,
            app_state.block_lists.clone(),
            global_resource_handles,
            background_blur_radius_pixels,
            background_blur_texture,
            ctx,
        );
    }
}

fn add_restored_window(
    window: &WindowSnapshot,
    block_lists: Arc<HashMap<PaneUuid, Vec<SerializedBlockListItem>>>,
    global_resource_handles: GlobalResourceHandles,
    background_blur_radius_pixels: Option<u8>,
    background_blur_texture: bool,
    ctx: &mut AppContext,
) {
    ctx.add_window(
        AddWindowOptions {
            window_bounds: WindowBounds::new(window.bounds),
            title: Some(WINDOW_TITLE.to_owned()),
            fullscreen_state: window.fullscreen_state,
            background_blur_radius_pixels,
            background_blur_texture,
            on_gpu_driver_selected: on_gpu_driver_selected_callback(),
            ..Default::default()
        },
        |ctx| {
            let mut view = RootView::new(
                global_resource_handles,
                NewWorkspaceSource::Restored {
                    window_snapshot: window.clone(),
                    block_lists,
                },
                ctx,
            );
            view.focus(ctx);
            view
        },
    );
}

fn path_if_directory(path: &Path) -> Option<&Path> {
    path.is_dir().then_some(path)
}

pub(crate) fn open_new_with_workspace_source(
    source: NewWorkspaceSource,
    ctx: &mut AppContext,
) -> (WindowId, ViewHandle<RootView>) {
    let global_resource_handles = GlobalResourceHandlesProvider::as_ref(ctx).get().clone();
    let options = default_window_options(WindowSettings::as_ref(ctx), ctx);
    ctx.add_window(options, |ctx| {
        let mut view = RootView::new(global_resource_handles, source, ctx);
        view.focus(ctx);
        view
    })
}

pub(crate) fn open_new_from_path(
    arg: &OpenPath,
    ctx: &mut AppContext,
) -> (WindowId, ViewHandle<RootView>) {
    open_new_with_workspace_source(
        NewWorkspaceSource::Session {
            options: Box::new(
                NewTerminalOptions::default()
                    .with_initial_directory_opt(path_if_directory(&arg.path).map(Into::into)),
            ),
        },
        ctx,
    )
}

fn open_settings_page_in_new_window(section: &SettingsSection, ctx: &mut AppContext) {
    let root = open_new_window_get_handles(None, ctx).1;
    root.update(ctx, |root, ctx| {
        let window_id = ctx.window_id();
        ctx.dispatch_typed_action_for_view(
            window_id,
            root.workspace.id(),
            &WorkspaceAction::ShowSettingsPage(*section),
        );
    });
}

fn workspace_action_for_open_settings(args: &OpenSettingsArgs) -> WorkspaceAction {
    match args {
        OpenSettingsArgs::Default => WorkspaceAction::ShowSettings,
        OpenSettingsArgs::Search { query } => WorkspaceAction::ShowSettingsPageWithSearch {
            search_query: query.clone(),
            section: None,
        },
    }
}

fn open_settings_in_new_window(args: &OpenSettingsArgs, ctx: &mut AppContext) {
    let action = workspace_action_for_open_settings(args);
    let root = open_new_window_get_handles(None, ctx).1;
    root.update(ctx, |root, ctx| {
        let window_id = ctx.window_id();
        ctx.dispatch_typed_action_for_view(window_id, root.workspace.id(), &action);
    });
}

pub(crate) fn open_new_window_get_handles(
    shell: Option<AvailableShell>,
    ctx: &mut AppContext,
) -> (WindowId, ViewHandle<RootView>) {
    let previous_active_window = ctx.windows().active_window();
    open_new_with_workspace_source(
        NewWorkspaceSource::Empty {
            previous_active_window,
            shell,
        },
        ctx,
    )
}

fn open_new(_: &(), ctx: &mut AppContext) {
    open_new_window_get_handles(None, ctx);
}

fn open_new_with_shell(shell: &Option<AvailableShell>, ctx: &mut AppContext) {
    open_new_window_get_handles(shell.clone(), ctx);
}

fn open_new_tab_insert_subshell_command_and_bootstrap_if_supported(
    arg: &SubshellCommandArg,
    ctx: &mut AppContext,
) {
    let root: Option<ViewHandle<RootView>> = ctx
        .windows()
        .frontmost_window_id()
        .and_then(|window_id| ctx.root_view(window_id));
    let root = match root {
        Some(root) => {
            root.update(ctx, |root, ctx| {
                root.workspace.update(ctx, |workspace, ctx| {
                    workspace.add_terminal_tab(false, ctx);
                });
            });
            root
        }
        None => open_new_window_get_handles(None, ctx).1,
    };
    root.update(ctx, |root, ctx| {
        root.insert_subshell_command_and_bootstrap_if_supported(arg, ctx);
    });
}

fn default_window_options(window_settings: &WindowSettings, ctx: &AppContext) -> AddWindowOptions {
    let (inherited_bounds, window_style) = ctx.next_window_bounds_and_style();
    AddWindowOptions {
        window_style,
        window_bounds: bounds_for_opening_at_custom_window_size(
            inherited_bounds,
            window_settings,
            ctx,
        ),
        title: Some(WINDOW_TITLE.to_owned()),
        background_blur_radius_pixels: Some(*window_settings.background_blur_radius),
        background_blur_texture: *window_settings.background_blur_texture,
        on_gpu_driver_selected: on_gpu_driver_selected_callback(),
        ..Default::default()
    }
}

fn bounds_for_opening_at_custom_window_size(
    bounds: WindowBounds,
    window_settings: &WindowSettings,
    app: &AppContext,
) -> WindowBounds {
    if !*window_settings.open_windows_at_custom_size.value() {
        return bounds;
    }

    let appearance = Appearance::as_ref(app);
    let cell = cell_size_and_padding(
        app.font_cache(),
        appearance.monospace_font_family(),
        appearance.monospace_font_size(),
        appearance.ui_builder().line_height_ratio(),
    );
    let size = vec2f(
        *window_settings.new_windows_num_columns.value() as f32 * cell.cell_width_px.as_f32()
            + 2. * cell.padding_x_px.as_f32(),
        *window_settings.new_windows_num_rows.value() as f32 * cell.cell_height_px.as_f32()
            + 2. * cell.padding_y_px.as_f32(),
    );
    match bounds {
        WindowBounds::ExactPosition(rect) => {
            WindowBounds::ExactPosition(RectF::new(rect.origin(), size))
        }
        WindowBounds::ExactSize(_) | WindowBounds::Default => WindowBounds::ExactSize(size),
    }
}

pub fn quake_mode_window_is_open() -> bool {
    QUAKE_STATE.lock().as_ref().is_some_and(|state| {
        matches!(
            state.window_state,
            WindowState::Open | WindowState::PendingOpen
        )
    })
}

pub fn quake_mode_window_id() -> Option<WindowId> {
    QUAKE_STATE.lock().as_ref().map(|state| state.window_id)
}

pub fn set_quake_mode(new_state: Option<QuakeModeState>) {
    *QUAKE_STATE.lock() = new_state;
}

fn move_quake_mode_window_from_screen_change(settings: &QuakeModeSettings, ctx: &mut AppContext) {
    fit_quake_mode_window_within_active_screen(
        settings,
        QuakeModeMoveTrigger::ScreenConfigurationChange,
        ctx,
    );
}

pub fn update_quake_window_bounds(settings: &QuakeModeSettings, ctx: &mut AppContext) {
    let config = quake_mode_config(settings, ctx);
    let Some(state) = &*QUAKE_STATE.lock() else {
        return;
    };
    ctx.windows()
        .set_window_bounds(state.window_id, config.window_bounds);
}

fn fit_quake_mode_window_within_active_screen(
    settings: &QuakeModeSettings,
    trigger: QuakeModeMoveTrigger,
    ctx: &mut AppContext,
) {
    let mut state = QUAKE_STATE.lock();
    let Some(state) = state.as_mut() else {
        return;
    };
    let active_display_id = ctx.windows().active_display_id();
    if matches!(trigger, QuakeModeMoveTrigger::ActiveScreenSetting)
        && active_display_id == state.active_display_id
    {
        return;
    }
    let window_bounds = settings.resolve_quake_mode_bounds(ctx);
    ctx.windows()
        .set_window_bounds(state.window_id, window_bounds);
    state.active_display_id = active_display_id;
}

fn update_quake_mode_state(arg: &UpdateQuakeModeEventArg, ctx: &mut AppContext) {
    if !KeysSettings::as_ref(ctx)
        .quake_mode_settings
        .hide_window_when_unfocused
    {
        return;
    }
    let mut state = QUAKE_STATE.lock();
    let Some(state) = state.as_mut() else {
        return;
    };
    state.window_state = match state.window_state {
        WindowState::PendingOpen => WindowState::Open,
        WindowState::Open if arg.active_window_id == Some(state.window_id) => WindowState::Open,
        WindowState::Open => {
            ctx.windows().hide_window(state.window_id);
            WindowState::Hidden
        }
        WindowState::Hidden => WindowState::Hidden,
    };
}

fn quake_mode_config(settings: &QuakeModeSettings, ctx: &mut AppContext) -> QuakeModeFrameConfig {
    QuakeModeFrameConfig {
        display_id: ctx.windows().active_display_id(),
        window_bounds: settings.resolve_quake_mode_bounds(ctx),
    }
}

fn get_quake_mode_state(ctx: &mut AppContext) -> Option<QuakeModeState> {
    QUAKE_STATE
        .lock()
        .as_ref()
        .filter(|state| ctx.is_window_open(state.window_id))
        .cloned()
}

fn toggle_quake_mode_window(global_resource_handles: &GlobalResourceHandles, ctx: &mut AppContext) {
    match get_quake_mode_state(ctx) {
        None => {
            let config = quake_mode_config(
                &KeysSettings::as_ref(ctx)
                    .quake_mode_settings
                    .value()
                    .clone(),
                ctx,
            );
            let window_settings = WindowSettings::as_ref(ctx);
            let previous_active_window = ctx.windows().active_window();
            let (window_id, _) = ctx.add_window(
                AddWindowOptions {
                    window_style: WindowStyle::Pin,
                    window_bounds: WindowBounds::ExactPosition(config.window_bounds),
                    title: Some(WINDOW_TITLE.to_owned()),
                    background_blur_radius_pixels: Some(*window_settings.background_blur_radius),
                    background_blur_texture: *window_settings.background_blur_texture,
                    anchor_new_windows_from_closed_position:
                        NextNewWindowsHasThisWindowsBoundsUponClose::No,
                    on_gpu_driver_selected: on_gpu_driver_selected_callback(),
                    window_instance: Some(ChannelState::app_id().to_string() + "-hotkey"),
                    ..Default::default()
                },
                |ctx| {
                    let mut view = RootView::new(
                        global_resource_handles.clone(),
                        NewWorkspaceSource::Empty {
                            previous_active_window,
                            shell: None,
                        },
                        ctx,
                    );
                    view.focus(ctx);
                    view
                },
            );
            set_quake_mode(Some(QuakeModeState {
                window_state: WindowState::PendingOpen,
                window_id,
                active_display_id: config.display_id,
            }));
        }
        Some(state) if matches!(state.window_state, WindowState::Hidden) => {
            if KeysSettings::as_ref(ctx)
                .quake_mode_settings
                .pin_screen
                .is_none()
            {
                fit_quake_mode_window_within_active_screen(
                    &KeysSettings::as_ref(ctx)
                        .quake_mode_settings
                        .value()
                        .clone(),
                    QuakeModeMoveTrigger::ActiveScreenSetting,
                    ctx,
                );
            }
            ctx.windows().show_window_and_focus_app(state.window_id);
            if let Some(state) = QUAKE_STATE.lock().as_mut() {
                state.window_state = WindowState::PendingOpen;
            }
        }
        Some(state) => {
            ctx.windows().hide_window(state.window_id);
            if let Some(state) = QUAKE_STATE.lock().as_mut() {
                state.window_state = WindowState::Hidden;
            }
        }
    }
}

fn show_or_hide_non_quake_mode_windows(_: &(), ctx: &mut AppContext) {
    let quake_window_id = get_quake_mode_state(ctx).map(|state| state.window_id);
    if ctx
        .window_ids()
        .filter(|window_id| Some(*window_id) != quake_window_id)
        .count()
        == 0
    {
        open_new(&(), ctx);
    }
    if ctx.windows().active_window().is_some() {
        ctx.windows().hide_app();
    } else {
        ctx.windows().activate_app();
    }
}

#[derive(Clone)]
pub enum NewWorkspaceSource {
    Empty {
        previous_active_window: Option<WindowId>,
        shell: Option<AvailableShell>,
    },
    FromTemplate {
        window_template: launch_config::WindowTemplate,
    },
    Restored {
        window_snapshot: WindowSnapshot,
        block_lists: Arc<HashMap<PaneUuid, Vec<SerializedBlockListItem>>>,
    },
    Session {
        options: Box<NewTerminalOptions>,
    },
    TransferredTab {
        source_window_id: WindowId,
        tab_color: Option<AnsiColorIdentifier>,
        custom_title: Option<String>,
        left_panel_open: bool,
        vertical_tabs_panel_open: bool,
        right_panel_open: bool,
        is_right_panel_maximized: bool,
        is_tab_drag_preview: bool,
    },
}

impl NewWorkspaceSource {
    pub fn has_horizontal_split(&self) -> bool {
        match self {
            NewWorkspaceSource::Restored {
                window_snapshot, ..
            } => {
                let Some(active_tab) = window_snapshot
                    .tabs
                    .get(window_snapshot.active_tab_index)
                    .or_else(|| window_snapshot.tabs.first())
                else {
                    return false;
                };
                active_tab.root.has_horizontal_split()
            }
            NewWorkspaceSource::Empty { .. }
            | NewWorkspaceSource::FromTemplate { .. }
            | NewWorkspaceSource::Session { .. }
            | NewWorkspaceSource::TransferredTab { .. } => false,
        }
    }
}

pub struct RootView {
    workspace: ViewHandle<Workspace>,
}

impl RootView {
    pub fn new(
        global_resource_handles: GlobalResourceHandles,
        workspace_source: NewWorkspaceSource,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let workspace = ctx.add_typed_action_view(|ctx| {
            Workspace::new(global_resource_handles, None, workspace_source, ctx)
        });
        Self { workspace }
    }

    pub fn workspace_view(&self) -> Option<&ViewHandle<Workspace>> {
        Some(&self.workspace)
    }

    fn close_window(&mut self, _: &(), ctx: &mut ViewContext<Self>) -> bool {
        if ContextFlag::CloseWindow.is_enabled() {
            ctx.close_window();
        }
        true
    }

    fn minimize_window(&mut self, _: &(), ctx: &mut ViewContext<Self>) -> bool {
        ctx.minimize_window();
        true
    }

    fn toggle_maximize_window(&mut self, _: &(), ctx: &mut ViewContext<Self>) -> bool {
        ctx.toggle_maximized_window();
        true
    }

    fn toggle_fullscreen(&mut self, _: &(), ctx: &mut ViewContext<Self>) -> bool {
        let window_id = ctx.window_id();
        WindowManager::handle(ctx).update(ctx, |state, ctx| {
            state.toggle_fullscreen(window_id, ctx);
        });
        true
    }

    fn focus_pane(
        &mut self,
        pane_view_locator: &PaneViewLocator,
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        let window_id = ctx.window_id();
        if let Some(state) = QUAKE_STATE.lock().as_mut()
            && state.window_id == window_id
        {
            state.window_state = WindowState::Open;
        }
        ctx.windows().show_window_and_focus_app(window_id);
        self.workspace.update(ctx, |workspace, ctx| {
            workspace.focus_pane(*pane_view_locator, ctx);
        });
        true
    }

    fn activate_tab_by_pane_group_id(
        &mut self,
        pane_group_id: &EntityId,
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        ctx.windows().show_window_and_focus_app(ctx.window_id());
        self.workspace.update(ctx, |workspace, ctx| {
            workspace.activate_tab_by_pane_group_id(*pane_group_id, ctx);
        });
        true
    }

    fn handle_notification_click(
        &mut self,
        pane_view_locator: &PaneViewLocator,
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        self.focus_pane(pane_view_locator, ctx)
    }

    fn add_session_at_path(&mut self, path: &Path, ctx: &mut ViewContext<Self>) -> bool {
        let window_id = ctx.window_id();
        self.workspace.update(ctx, |workspace, ctx| {
            workspace.add_tab_with_pane_layout(
                PanesLayout::SingleTerminal(Box::new(
                    NewTerminalOptions::default()
                        .with_initial_directory_opt(path_if_directory(path).map(Into::into)),
                )),
                Arc::new(HashMap::new()),
                None,
                ctx,
            );
            ctx.windows().show_window_and_focus_app(window_id);
            ctx.notify();
        });
        true
    }

    pub fn insert_subshell_command_and_bootstrap_if_supported(
        &mut self,
        arg: &SubshellCommandArg,
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        let window_id = ctx.window_id();
        self.workspace.update(ctx, |workspace, ctx| {
            workspace.insert_subshell_command_and_bootstrap_if_supported(
                &arg.command,
                arg.shell_type,
                ctx,
            );
            ctx.windows().show_window_and_focus_app(window_id);
        });
        true
    }

    pub fn open_settings_page_in_existing_window(
        &mut self,
        section: &SettingsSection,
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        let window_id = ctx.window_id();
        ctx.dispatch_typed_action_for_view(
            window_id,
            self.workspace.id(),
            &WorkspaceAction::ShowSettingsPage(*section),
        );
        ctx.windows().show_window_and_focus_app(window_id);
        true
    }

    pub fn open_settings_in_existing_window(
        &mut self,
        args: &OpenSettingsArgs,
        ctx: &mut ViewContext<Self>,
    ) -> bool {
        let window_id = ctx.window_id();
        ctx.dispatch_typed_action_for_view(
            window_id,
            self.workspace.id(),
            &workspace_action_for_open_settings(args),
        );
        ctx.windows().show_window_and_focus_app(window_id);
        true
    }

    pub fn focus(&mut self, ctx: &mut ViewContext<Self>) -> bool {
        ctx.focus(&self.workspace);
        ctx.notify();
        true
    }
}

impl Entity for RootView {
    type Event = ();
}

impl View for RootView {
    fn ui_name() -> &'static str {
        "RootView"
    }

    fn on_focus(&mut self, focus_ctx: &FocusContext, ctx: &mut ViewContext<Self>) {
        if focus_ctx.is_self_focused() {
            self.focus(ctx);
        }
    }

    fn render(&self, _app: &AppContext) -> Box<dyn Element> {
        ChildView::new(&self.workspace).finish()
    }

    fn keymap_context(&self, app: &AppContext) -> warpui::keymap::Context {
        let mut context = Self::default_keymap_context();
        if quake_mode_window_is_open() {
            context.set.insert(flags::QUAKE_WINDOW_OPEN_FLAG);
        }
        if *KeysSettings::as_ref(app).quake_mode_enabled {
            context.set.insert(flags::QUAKE_MODE_ENABLED_CONTEXT_FLAG);
        }
        if *KeysSettings::as_ref(app).activation_hotkey_enabled.value() {
            context.set.insert(flags::ACTIVATION_HOTKEY_FLAG);
        }
        context
    }
}

#[derive(Clone, Debug)]
pub enum RootViewAction {
    ToggleQuakeModeWindow,
    ShowOrHideNonQuakeModeWindows,
    ToggleFullscreen,
}

impl TypedActionView for RootView {
    type Action = RootViewAction;

    fn handle_action(&mut self, action: &RootViewAction, ctx: &mut ViewContext<Self>) {
        match action {
            RootViewAction::ToggleQuakeModeWindow => {
                let resources = GlobalResourceHandlesProvider::as_ref(ctx).get().clone();
                toggle_quake_mode_window(&resources, ctx);
            }
            RootViewAction::ShowOrHideNonQuakeModeWindows => {
                show_or_hide_non_quake_mode_windows(&(), ctx);
            }
            RootViewAction::ToggleFullscreen => {
                let window_id = ctx.window_id();
                WindowManager::handle(ctx).update(ctx, |state, ctx| {
                    state.toggle_fullscreen(window_id, ctx);
                });
            }
        }
    }
}
