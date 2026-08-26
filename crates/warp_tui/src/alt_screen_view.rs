//! Full-screen alt-screen rendering for the TUI.
//!
//! When a PTY app switches to the alternate screen (vim, htop, less, …), the
//! terminal model flips [`TerminalModel::is_alt_screen_active`] and populates a
//! dedicated alt-screen grid. [`TuiTerminalSessionView`] then renders this
//! element full-area instead of the block/transcript UI — mirroring the GUI's
//! `AltScreenElement` (`app/src/terminal/alt_screen/alt_screen_element.rs`).
//!
//! Covers rendering and the cursor. PTY sizing plus keyboard, paste, and mouse
//! forwarding are handled by the session view's `TuiTerminalContentElement`
//! wrapper.
//!
//! [`TuiTerminalSessionView`]: crate::terminal_session_view::TuiTerminalSessionView
//! [`TerminalModel::is_alt_screen_active`]: warp::tui_export::TerminalModel

use std::sync::Arc;

use parking_lot::FairMutex;
use warp::tui_export::{BlockGrid, GridHandler, TermMode, TerminalColorList, TerminalModel};
use warp_terminal::model::grid::Dimensions as _;
use warpui_core::AppContext;
use warpui_core::elements::tui::{
    TuiConstraint, TuiElement, TuiLayoutContext, TuiPaintContext, TuiPaintSurface, TuiScreenPoint,
    TuiScreenPosition, TuiSize,
};

use crate::terminal_block::{render_block_grid_rows, render_grid_handler};

/// Renders the terminal's alt-screen grid full-area while a full-screen app is
/// active.
pub(crate) struct AltScreenElement {
    model: Arc<FairMutex<TerminalModel>>,
    size: Option<TuiSize>,
    origin: Option<TuiScreenPoint>,
}

impl AltScreenElement {
    pub(crate) fn new(model: Arc<FairMutex<TerminalModel>>) -> Self {
        Self {
            model,
            size: None,
            origin: None,
        }
    }
}

impl TuiElement for AltScreenElement {
    fn layout(
        &mut self,
        constraint: TuiConstraint,
        _ctx: &mut TuiLayoutContext,
        _app: &AppContext,
    ) -> TuiSize {
        // The alt-screen app owns the whole pane.
        let size = constraint.max;
        self.size = Some(size);
        size
    }

    fn render(
        &mut self,
        origin: TuiScreenPosition,
        surface: &mut TuiPaintSurface<'_>,
        ctx: &mut TuiPaintContext,
    ) {
        self.origin = Some(ctx.scene_point(origin));
        let Some(size) = self.size else {
            return;
        };
        let model = self.model.lock();
        let colors = model.colors();
        let cursor = if model.is_alt_screen_active() {
            let grid = model.alt_screen().grid_handler();
            render_grid_handler(grid, origin, size, surface, &colors);
            visible_grid_cursor(grid, size)
        } else {
            render_block_list(&model, origin, size, surface, &colors)
        };

        let cursor = model
            .is_term_mode_set(TermMode::SHOW_CURSOR)
            .then_some(cursor)
            .flatten();
        drop(model);
        if let Some((col, row)) = cursor {
            let cursor_point = ctx.scene_point(origin.offset(i32::from(col), i32::from(row)));
            ctx.set_terminal_cursor(cursor_point);
        }
    }

    fn size(&self) -> Option<TuiSize> {
        self.size
    }

    fn origin(&self) -> Option<TuiScreenPoint> {
        self.origin
    }
}

fn visible_grid_cursor(grid: &GridHandler, size: TuiSize) -> Option<(u16, u16)> {
    let point = grid.cursor_render_point();
    let row = point.row.checked_sub(grid.history_size())?;
    let col = u16::try_from(point.col).ok()?;
    let row = u16::try_from(row).ok()?;
    (col < size.width && row < size.height).then_some((col, row))
}

fn render_block_list(
    model: &TerminalModel,
    origin: TuiScreenPosition,
    size: TuiSize,
    surface: &mut TuiPaintSurface<'_>,
    colors: &TerminalColorList,
) -> Option<(u16, u16)> {
    let block_list = model.block_list();
    let active_block_id = block_list.active_block_id();
    let grids = block_list
        .blocks()
        .iter()
        .filter(|block| {
            block.id() == active_block_id
                || (block.is_visible(block_list.transcript_scope())
                    && (block.started() || block.finished()))
        })
        .flat_map(|block| {
            [
                (!block.should_hide_command_grid()).then_some(block.prompt_and_command_grid()),
                (!block.should_hide_output_grid()).then_some(block.output_grid()),
            ]
            .into_iter()
            .flatten()
        })
        .collect::<Vec<&BlockGrid>>();
    let total_rows = grids.iter().map(|grid| grid.len_displayed()).sum::<usize>();
    let first_row = total_rows.saturating_sub(usize::from(size.height));
    let mut grid_start = 0;
    let mut target_row = 0;
    for grid in grids {
        let grid_end = grid_start + grid.len_displayed();
        let visible_start = grid_start.max(first_row);
        if visible_start < grid_end {
            let local_start = visible_start - grid_start;
            let visible_rows = grid_end - visible_start;
            render_block_grid_rows(
                grid,
                local_start..local_start + visible_rows,
                origin.offset(0, target_row as i32),
                TuiSize::new(size.width, size.height.saturating_sub(target_row as u16)),
                surface,
                colors,
            );
            target_row += visible_rows;
        }
        grid_start = grid_end;
    }

    let point = block_list
        .active_block()
        .grid_of_type(block_list.active_block().active_grid_type())?
        .grid_handler()
        .cursor_render_point();
    let col = u16::try_from(point.col).ok()?;
    let row = u16::try_from(target_row.min(usize::from(size.height.saturating_sub(1)))).ok()?;
    (col < size.width).then_some((col, row))
}
