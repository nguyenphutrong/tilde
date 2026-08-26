use std::ops::Range;

use warp::tui_export::{BlockGrid, GridHandler, TerminalColorList};
use warp_terminal::model::ansi::{Color, NamedColor};
use warp_terminal::model::grid::Dimensions as _;
use warp_terminal::model::grid::cell::{Cell, Flags};
use warpui_core::elements::tui::{
    Color as TuiColor, Modifier, TuiPaintSurface, TuiScreenPosition, TuiSize, TuiStyle,
};

pub(crate) fn render_grid_handler(
    grid: &GridHandler,
    origin: TuiScreenPosition,
    size: TuiSize,
    surface: &mut TuiPaintSurface<'_>,
    colors: &TerminalColorList,
) {
    let history = grid.history_size();
    let rows = grid.visible_rows().min(usize::from(size.height));
    let columns = grid.columns().min(usize::from(size.width));
    for screen_row in 0..rows {
        let Some(row) = grid.row(history + screen_row) else {
            continue;
        };
        for column in 0..columns {
            let cell = &row[column];
            let Some(buffer_cell) = surface.cell_mut(
                origin.offset(i32::try_from(column).unwrap_or(i32::MAX), screen_row as i32),
            ) else {
                continue;
            };
            buffer_cell
                .set_symbol(&sanitized_symbol(cell))
                .set_style(cell_to_style(cell, colors));
        }
    }
}

pub(crate) fn render_block_grid_rows(
    block_grid: &BlockGrid,
    displayed_rows: Range<usize>,
    origin: TuiScreenPosition,
    size: TuiSize,
    surface: &mut TuiPaintSurface<'_>,
    colors: &TerminalColorList,
) {
    let grid = block_grid.grid_handler();
    let end = displayed_rows.end.min(block_grid.len_displayed());
    let columns = grid.columns().min(usize::from(size.width));
    for (target_row, displayed_row) in (displayed_rows.start.min(end)..end).enumerate() {
        if target_row >= usize::from(size.height) {
            break;
        }
        let row = grid.maybe_translate_row_from_displayed_to_original(displayed_row);
        let Some(row) = grid.row(row) else {
            continue;
        };
        for column in 0..columns {
            let cell = &row[column];
            let Some(buffer_cell) = surface.cell_mut(
                origin.offset(i32::try_from(column).unwrap_or(i32::MAX), target_row as i32),
            ) else {
                continue;
            };
            buffer_cell
                .set_symbol(&sanitized_symbol(cell))
                .set_style(cell_to_style(cell, colors));
        }
    }
}

fn cell_to_color(color: &Color, colors: &TerminalColorList) -> TuiColor {
    match color {
        Color::Named(named) => {
            let color = &colors[named.into_color_index()];
            TuiColor::Rgb(color.r, color.g, color.b)
        }
        Color::Spec(color) => TuiColor::Rgb(color.r, color.g, color.b),
        Color::Indexed(index) => {
            let color = &colors[*index as usize];
            TuiColor::Rgb(color.r, color.g, color.b)
        }
    }
}

fn cell_to_style(cell: &Cell, colors: &TerminalColorList) -> TuiStyle {
    let mut style = TuiStyle::default().fg(cell_to_color(&cell.fg, colors));
    if cell.bg != Color::Named(NamedColor::Background) {
        style = style.bg(cell_to_color(&cell.bg, colors));
    }
    for (flag, modifier) in [
        (Flags::BOLD, Modifier::BOLD),
        (Flags::ITALIC, Modifier::ITALIC),
        (Flags::UNDERLINE, Modifier::UNDERLINED),
        (Flags::DOUBLE_UNDERLINE, Modifier::UNDERLINED),
        (Flags::INVERSE, Modifier::REVERSED),
        (Flags::DIM, Modifier::DIM),
        (Flags::HIDDEN, Modifier::HIDDEN),
        (Flags::STRIKEOUT, Modifier::CROSSED_OUT),
    ] {
        if cell.flags.contains(flag) {
            style = style.add_modifier(modifier);
        }
    }
    style
}

fn sanitized_symbol(cell: &Cell) -> String {
    let content = cell.content_for_display().to_string();
    if content.is_empty() || content.chars().any(char::is_control) {
        " ".to_owned()
    } else {
        content
    }
}
