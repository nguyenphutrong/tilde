//! Bootstrap for the local-shell-only headless TUI.

use std::path::PathBuf;

use anyhow::Result;
use clap::error::ErrorKind;
use clap::{Parser, Subcommand};
use warp::settings::{TuiThemeSettings, TuiZeroStateSettings};
use warp::tui_export::Appearance;
use warp_core::settings::Setting as _;
use warpui::SingletonEntity as _;
use warpui_core::platform::{TerminationMode, WindowStyle};
use warpui_core::runtime::{TuiDriverStartupError, TuiFocusPolicy, spawn_tui_driver};
use warpui_core::{AddWindowOptions, AppContext};

use crate::root_view::RootTuiView;
use crate::session_registry::TuiSessions;
use crate::terminal_background::probe_and_select_theme;

const CLI_VERSION: &str = match option_env!("GIT_RELEASE_TAG") {
    Some(version) => version,
    None => "v0.0.0.0.0.0",
};

#[derive(Debug, Parser)]
#[command(name = "warp", version = CLI_VERSION)]
struct TuiArgs {
    #[command(subcommand)]
    command: Option<TuiCommand>,
}

#[derive(Debug, Subcommand)]
enum TuiCommand {
    /// Print the settings schema and exit.
    DumpSettingsSchema { output_path: Option<PathBuf> },
}

pub fn run() -> Result<()> {
    if let Some(result) = warp::run_tui_worker_if_requested() {
        return result;
    }
    let args = match TuiArgs::try_parse() {
        Ok(args) => args,
        Err(error) if error.kind() == ErrorKind::DisplayVersion => {
            println!("{CLI_VERSION}");
            return Ok(());
        }
        Err(error) if error.kind() == ErrorKind::DisplayHelp => {
            error.print()?;
            return Ok(());
        }
        Err(error) => return Err(anyhow::Error::new(error)),
    };
    if let Some(TuiCommand::DumpSettingsSchema { output_path }) = args.command {
        warp::features::init_feature_flags();
        return warp::settings::dump_settings_schema(output_path.as_deref());
    }

    warp::run_tui(None, Box::new(init))
}

fn init(ctx: &mut AppContext) {
    crate::keybindings::init(ctx);

    let selected_theme = TuiThemeSettings::as_ref(ctx).selected_theme();
    let theme = probe_and_select_theme(selected_theme);
    Appearance::handle(ctx).update(ctx, |appearance, ctx| appearance.set_theme(theme, ctx));

    let (window_id, root) = ctx.add_tui_window(
        AddWindowOptions {
            window_style: WindowStyle::NotStealFocus,
            ..Default::default()
        },
        RootTuiView::new,
    );
    let freeze = *TuiZeroStateSettings::as_ref(ctx)
        .freeze_animation_when_unfocused
        .value();
    match spawn_tui_driver(
        ctx,
        window_id,
        root,
        TuiFocusPolicy::PresentedTree,
        false,
        freeze,
    ) {
        Ok(driver) => {
            let sessions = ctx.add_singleton_model(|_| TuiSessions::new(driver));
            let surface = TuiSessions::create_local_terminal_session(
                &sessions,
                window_id,
                std::env::current_dir().ok(),
                ctx,
            );
            surface.update(ctx, |view, ctx| view.activate(ctx));
        }
        Err(TuiDriverStartupError::TerminalDisconnected(error)) => {
            log::error!("failed to start the TUI driver: {error}");
            ctx.terminate_app(TerminationMode::ForceTerminate, None);
        }
        Err(TuiDriverStartupError::Unexpected(error)) => {
            ctx.terminate_app(TerminationMode::ForceTerminate, Some(Err(error.into())));
        }
    }
}
