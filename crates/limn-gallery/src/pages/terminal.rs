use std::sync::Arc;

use super::{caption, section, Cx, Section};
use limn::{
    Color, Component, Element, RadiusRole, Size, TerminalCell, TerminalCursor, TerminalCursorShape,
    TerminalScreen, TerminalView,
};

/// The colors a host's emulator resolved from the program's ANSI codes.
const GREEN: Color = Color::rgb(78, 186, 101);
const BLUE: Color = Color::rgb(76, 139, 245);
const YELLOW: Color = Color::rgb(229, 192, 76);
const RED: Color = Color::rgb(232, 89, 79);
/// A dark blue as `ls` paints directories on some themes: hard to read on black.
const NAVY: Color = Color::rgb(30, 40, 120);
const ROWS: usize = 9;
const FONT_SIZE: f32 = 13.0;

/// One run of a line: its text, ink and whether it is bold or inverse.
type Run<'a> = (&'a str, Option<Color>, bool);

/// A line of `cols` cells from runs; `None` is the default ink.
fn line(runs: &[Run], ink: Color, cols: usize) -> Vec<TerminalCell> {
    let mut cells: Vec<TerminalCell> = runs
        .iter()
        .flat_map(|&(text, color, inverse)| {
            text.chars().map(move |ch| TerminalCell { inverse, ..TerminalCell::new(ch, color.unwrap_or(ink)) })
        })
        .take(cols)
        .collect();
    cells.resize(cols, TerminalCell::new(' ', ink));
    cells
}

fn screen(ink: Color, cols: usize, directory: Color, shape: TerminalCursorShape) -> Arc<TerminalScreen> {
    let prompt = |command: &'static str| -> Vec<Run<'static>> {
        vec![("nihuel@quill", Some(GREEN), false), (":", None, false), ("~/limn", Some(BLUE), false), ("$ ", None, false), (command, None, false)]
    };
    let lines: Vec<Vec<Run>> = vec![
        prompt("ls"),
        vec![("Cargo.toml  ", None, false), ("crates", Some(directory), false), ("  ", None, false), ("docs", Some(directory), false), ("  README.md", None, false)],
        prompt("git status --short"),
        vec![(" M ", Some(RED), false), ("crates/limn/src/terminal_view.rs", None, false)],
        vec![("?? ", Some(RED), false), ("docs/components/terminal-view.md", None, false)],
        prompt("cargo test -p limn"),
        vec![("test result: ", None, false), ("ok", Some(GREEN), false), (". 1392 passed; ", None, false), ("2 ignored", Some(YELLOW), false)],
        vec![(" NORMAL ", None, true), (" terminal_view.rs", None, false)],
        prompt(""),
    ];
    let cursor = TerminalCursor { col: 22, row: lines.len() - 1, shape };
    Arc::new(TerminalScreen {
        cols,
        rows: ROWS,
        cells: lines.iter().flat_map(|runs| line(runs, ink, cols)).collect(),
        scrollback: Vec::new(),
        cursor: Some(cursor),
    })
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width.min(620.0);
    let (cols, _) = TerminalView::grid_size(Size::new(width, 0.0), FONT_SIZE);
    // A row is 1.3 times the font size.
    let height = ROWS as f32 * FONT_SIZE * 1.3;
    let framed = |view: TerminalView| {
        Element::panel(view.font_size(FONT_SIZE).render().size(None, Some(height)))
            .radius(RadiusRole::Surface.px())
            .clip_children()
            .size(Some(width), None)
    };
    let black = Color::rgb(0, 0, 0);
    let white = Color::rgb(229, 229, 229);
    vec![section(cx, "Terminal", vec![
        caption(cx, "A screen the host's emulator resolved: a prompt, output in ANSI colors, an inverse status line and a block cursor. Click it, or Tab to it, and the cursor blinks; drag over the text and Ctrl+C copies it."),
        framed(TerminalView::new("terminal", screen(palette.text, cols, BLUE, TerminalCursorShape::Block), palette)),
        caption(cx, "The same on black, where `ls` paints its directories navy, with a bar cursor. min_contrast(4.5) lifts the navy until it reads."),
        framed(
            TerminalView::new("terminal-contrast", screen(white, cols, NAVY, TerminalCursorShape::Bar), palette)
                .background(black)
                .min_contrast(4.5),
        ),
    ])]
}
