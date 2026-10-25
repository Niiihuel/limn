//! A terminal's screen, painted from the grid of cells its host keeps. limn
//! runs no shell and reads no escape sequences: the host's emulator does,
//! and hands the view what it would show.

use std::sync::Arc;

use crate::chip::readable_ink;
use crate::{
    Color, Component, ComponentPalette, DrawCommand, Element, FontFamily, Rect, ScrollbarAppearance,
    Size, TextSpan,
};

/// A cell's width in ems: the advance of Hack, the monospace face limn
/// bundles (1233 of its 2048 units).
const CELL_WIDTH: f32 = 1233.0 / 2048.0;
/// A cell's height in ems, above Hack's natural line (1.16) and the tree's
/// default metrics (1.29), so every row is this tall.
const CELL_HEIGHT: f32 = 1.3;
const FONT_SIZE: f32 = 13.0;
const BAR_WIDTH: f32 = 2.0;

/// One character cell. Its colors are already resolved by the host, from the
/// 16, 256 or true colors the program asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerminalCell {
    /// The character in the cell; a space for an empty one. A control
    /// character is drawn as a space.
    pub ch: char,
    /// Ink of the character.
    pub fg: Color,
    /// Fill behind the cell; `None` shows the view's background.
    pub bg: Option<Color>,
    /// Bold, as the program asked: the cell is drawn in
    /// [`FontFamily::MonospaceBold`]. A host that also brightens bold text
    /// does so in `fg`.
    pub bold: bool,
    /// Draw a line under the character, in its ink.
    pub underline: bool,
    /// Swap the ink and the fill: the character is drawn in the fill, or the
    /// background, on a fill of its ink.
    pub inverse: bool,
}

impl TerminalCell {
    /// `ch` in `fg` on the view's background, with no attributes.
    pub fn new(ch: char, fg: Color) -> Self {
        Self { ch, fg, bg: None, bold: false, underline: false, inverse: false }
    }
}

/// How the cursor is drawn.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TerminalCursorShape {
    /// A fill over the whole cell, the character in the cell's fill on it.
    #[default]
    Block,
    /// A bar 2 pixels wide at the cell's left edge.
    Bar,
}

/// Where the cursor is, in the screen's cells.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminalCursor {
    /// Column from the left, from 0. One past the last column hides it.
    pub col: usize,
    /// Row of the screen from the top, from 0, not counting the scrollback.
    pub row: usize,
    /// How it is drawn.
    pub shape: TerminalCursorShape,
}

/// What the host's emulator shows: the screen's cells, the lines scrolled off
/// above it, and the cursor.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TerminalScreen {
    /// Cells in a row.
    pub cols: usize,
    /// Rows on the screen.
    pub rows: usize,
    /// The screen's cells, row by row from the top: `cols` times `rows` of
    /// them. Rows that are missing are blank.
    pub cells: Vec<TerminalCell>,
    /// Lines scrolled off the top of the screen, oldest first. Each may be
    /// shorter than `cols`.
    pub scrollback: Vec<Vec<TerminalCell>>,
    /// The cursor; `None` hides it, as a program can ask.
    pub cursor: Option<TerminalCursor>,
}

/// A terminal's screen and its scrollback in one scrolling area, on a grid of
/// cells of the bundled monospace face.
///
/// The host owns the screen and passes it on every rebuild. The view takes
/// the keyboard focus under its ID on a press or Tab and reports nothing: the
/// host reads the keys while `UiTree::focused_id()` is the view's ID and
/// writes them to its terminal. Dragging over the text selects it, and
/// Ctrl+C reports `UiAction::CopyRequested` with it. The scrolling area's ID
/// is `{id}:scroll`. Bold cells are drawn in [`FontFamily::MonospaceBold`],
/// which must be as wide as the regular face, as Hack's is.
pub struct TerminalView {
    id: String,
    screen: Arc<TerminalScreen>,
    palette: ComponentPalette,
    font_size: f32,
    background: Option<Color>,
    min_contrast: Option<f32>,
    focus_ring: bool,
}

impl TerminalView {
    /// A view of `screen`. Its cursor is drawn in the palette's `text`, and its
    /// background is the palette's `surface` unless set.
    pub fn new(id: impl Into<String>, screen: Arc<TerminalScreen>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            screen,
            palette,
            font_size: FONT_SIZE,
            background: None,
            min_contrast: None,
            focus_ring: true,
        }
    }

    /// Whether the keyboard focus ring is drawn around the view while it has
    /// the focus. It is by default; without it the blinking cursor is the
    /// mark of the focus, as in xterm.js.
    pub fn focus_ring(mut self, visible: bool) -> Self {
        self.focus_ring = visible;
        self
    }

    /// Text size in logical pixels, 13 by default. Values that are not finite
    /// or not above zero are ignored; others are kept within 1..=96.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = font_size(size);
        self
    }

    /// Fill behind cells that have none of their own.
    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Move each character's ink toward white or black until it stands at
    /// this contrast ratio on its fill, as a program's dark blue on black is
    /// made readable. Kept within 1..=21; a value that is not finite is
    /// ignored. Unset, the inks are drawn as given.
    pub fn min_contrast(mut self, ratio: f32) -> Self {
        if ratio.is_finite() {
            self.min_contrast = Some(ratio.clamp(1.0, 21.0));
        }
        self
    }

    /// The columns and rows of cells that fit in `size` at `font_size`, at
    /// least one of each, for the host to size its terminal to. It assumes the
    /// bundled monospace face, as the view does.
    pub fn grid_size(size: Size, font_size: f32) -> (usize, usize) {
        let size_px = self::font_size(font_size);
        let fit = |length: f32, cell: f32| {
            if length.is_finite() {
                ((length / cell).floor() as usize).max(1)
            } else {
                1
            }
        };
        (fit(size.width, size_px * CELL_WIDTH), fit(size.height, size_px * CELL_HEIGHT))
    }
}

fn font_size(size: f32) -> f32 {
    if size.is_finite() && size > 0.0 {
        size.clamp(1.0, 96.0)
    } else {
        FONT_SIZE
    }
}

fn family(bold: bool) -> FontFamily {
    if bold { FontFamily::MonospaceBold } else { FontFamily::Monospace }
}

/// Add `ch` to the last run when it has the same ink, line and weight, else
/// start one.
fn push(spans: &mut Vec<TextSpan>, ch: char, ink: Color, underline: bool, bold: bool, size: f32) {
    match spans.last_mut() {
        Some(last) if last.color == ink && last.underline == underline && last.font_family == family(bold) => {
            last.text.push(ch)
        }
        _ => {
            let span = TextSpan::new(ch, ink, size, family(bold));
            spans.push(if underline { span.underline() } else { span });
        }
    }
}

fn blank(cell: &TerminalCell) -> bool {
    (cell.ch == ' ' || cell.ch.is_control()) && cell.bg.is_none() && !cell.underline && !cell.inverse
}

impl Component for TerminalView {
    fn render(self) -> Element {
        let screen = &*self.screen;
        let (cell_width, cell_height) = (self.font_size * CELL_WIDTH, self.font_size * CELL_HEIGHT);
        let background = self.background.unwrap_or(self.palette.surface);
        let shown = if screen.cols == 0 { 0 } else { screen.rows };
        let mut lines: Vec<&[TerminalCell]> = screen.scrollback.iter().map(Vec::as_slice).collect();
        lines.extend(screen.cells.chunks(screen.cols.max(1)).take(shown));
        let count = screen.scrollback.len() + shown;

        // One run for each stretch of cells with the same ink, line and weight, and
        // one fill for each stretch with the same fill: (line, from, to, color).
        let mut spans = Vec::new();
        let mut fills: Vec<(usize, usize, usize, Color)> = Vec::new();
        for (line, cells) in lines.iter().enumerate() {
            if line > 0 {
                let (ink, bold) = spans.last().map_or((self.palette.text, false), |span: &TextSpan| {
                    (span.color, span.font_family == FontFamily::MonospaceBold)
                });
                push(&mut spans, '\n', ink, false, bold, self.font_size);
            }
            let end = cells.iter().rposition(|cell| !blank(cell)).map_or(0, |last| last + 1);
            for (col, cell) in cells[..end].iter().enumerate() {
                let (ink, fill) = if cell.inverse {
                    (cell.bg.unwrap_or(background), Some(cell.fg))
                } else {
                    (cell.fg, cell.bg)
                };
                let ink = match self.min_contrast {
                    Some(ratio) => readable_ink(ink, fill.unwrap_or(background), ratio),
                    None => ink,
                };
                if let Some(fill) = fill {
                    match fills.last_mut() {
                        Some(last) if last.0 == line && last.2 == col && last.3 == fill => last.2 += 1,
                        _ => fills.push((line, col, col + 1, fill)),
                    }
                }
                let ch = if cell.ch.is_control() { ' ' } else { cell.ch };
                push(&mut spans, ch, ink, cell.underline, cell.bold, self.font_size);
            }
        }

        let fills = Element::drawing(move |_| {
            fills
                .iter()
                .map(|&(line, from, to, color)| DrawCommand::FillRect {
                    rect: Rect::new(
                        from as f32 * cell_width,
                        line as f32 * cell_height,
                        (to - from) as f32 * cell_width,
                        cell_height,
                    ),
                    color,
                    radius: 0.0,
                })
                .collect()
        });
        let text = Element::formatted_text(spans).font_size(self.font_size).line_height(cell_height);
        let mut layers = vec![fills, text];
        if let Some(cursor) = screen.cursor.filter(|cursor| cursor.row < shown && cursor.col < screen.cols) {
            let line = screen.scrollback.len() + cursor.row;
            let cell = screen.cells.get(cursor.row * screen.cols + cursor.col);
            let mark = match cursor.shape {
                TerminalCursorShape::Bar => Element::spacer().size(Some(BAR_WIDTH), Some(cell_height)),
                TerminalCursorShape::Block => {
                    let ch = cell.map(|cell| cell.ch).filter(|ch| !ch.is_whitespace() && !ch.is_control());
                    let ink = cell
                        .and_then(|cell| if cell.inverse { Some(cell.fg) } else { cell.bg })
                        .unwrap_or(background);
                    match ch {
                        Some(ch) => Element::truncated_text(ch)
                            .font_size(self.font_size)
                            .font_family(family(cell.is_some_and(|cell| cell.bold)))
                            .foreground(ink),
                        None => Element::spacer(),
                    }
                    .size(Some(cell_width), Some(cell_height))
                }
            };
            layers.push(Element::column(vec![
                Element::spacer().size(None, Some(line as f32 * cell_height)),
                Element::row(vec![
                    Element::spacer().size(Some(cursor.col as f32 * cell_width), None),
                    mark.background(self.palette.text).caret(self.id.clone(), line * screen.cols + cursor.col),
                ]),
            ]));
        }
        let grid = Element::stack(layers).size(None, Some(count as f32 * cell_height));
        Element::scroll(format!("{}:scroll", self.id), Element::focus_area(self.id, grid).focus_ring(self.focus_ring))
            .scrollbar_appearance(ScrollbarAppearance::overlay().tint(self.palette.muted))
            .background(background)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Frame, HitKind, Key, Point, UiAction, UiEvent, UiTree};
    use web_time::{Duration, Instant};

    const RED: Color = Color::rgb(220, 50, 47);
    const GREEN: Color = Color::rgb(133, 153, 0);
    const BLUE: Color = Color::rgb(38, 139, 210);

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn row(text: &str, fg: Color) -> Vec<TerminalCell> {
        text.chars().map(|ch| TerminalCell::new(ch, fg)).collect()
    }

    /// Two rows of four: `$ ls` in green and red, then `ab` on a blue fill.
    fn screen(cursor: Option<TerminalCursor>) -> Arc<TerminalScreen> {
        let mut cells = row("$ ", GREEN);
        cells.extend(row("ls", RED));
        cells.extend(row("ab", RED).into_iter().map(|cell| TerminalCell { bg: Some(BLUE), ..cell }));
        cells.extend(row("  ", RED));
        Arc::new(TerminalScreen { cols: 4, rows: 2, cells, scrollback: vec![row("old", RED)], cursor })
    }

    fn tree(screen: Arc<TerminalScreen>) -> UiTree {
        let mut tree = UiTree::new(TerminalView::new("term", screen, palette()).font_size(10.0).render());
        tree.set_text_measurer(|text, size, _| text.chars().count() as f32 * size * CELL_WIDTH);
        tree
    }

    fn texts(frame: &Frame) -> Vec<(String, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, color, .. } => Some((text.clone(), *color)),
                _ => None,
            })
            .collect()
    }

    fn cursor(frame: &Frame) -> Option<Rect> {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == palette().text => Some(*rect),
            _ => None,
        })
    }

    #[test]
    fn a_screen_is_drawn_in_runs_of_one_ink_on_fills_of_its_cells() {
        let mut tree = tree(screen(None));
        let frame = tree.layout(Size::new(200.0, 100.0));
        let (width, height) = (10.0 * CELL_WIDTH, 13.0);
        let texts = texts(&frame);
        assert!(texts.contains(&("$ ".into(), GREEN)), "{texts:?}");
        assert!(texts.contains(&("ls".into(), RED)), "{texts:?}");
        assert!(texts.iter().any(|(text, _)| text == "old"), "{texts:?}");
        let fills: Vec<Rect> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if *color == BLUE => Some(*rect),
                _ => None,
            })
            .collect();
        // The scrollback's line, then the screen's first: `ab` starts the third line.
        assert_eq!(fills, vec![Rect::new(0.0, 2.0 * height, 2.0 * width, height)]);
        let region = frame.regions.iter().find(|region| region.id == "term").unwrap();
        assert_eq!(region.kind, HitKind::Focus);
        assert_eq!(frame.text_layouts()[0].source, "old\n$ ls\nab");
    }

    #[test]
    #[cfg(feature = "bundled-fonts")]
    fn laid_out_in_the_bundled_face_the_text_sits_on_the_grid_the_fills_assume() {
        let mut tree = UiTree::new(TerminalView::new("term", screen(None), palette()).font_size(10.0).render());
        tree.set_text_layout(true);
        let frame = tree.layout(Size::new(200.0, 100.0));
        let glyphs = frame.text_layouts()[0].runs.clone();
        let (width, height) = (10.0 * CELL_WIDTH, 13.0);
        // A line is one run: its width is its cells', and lines are a cell apart.
        let lines: Vec<(&str, Rect)> = glyphs.iter().map(|run| (run.text.as_str(), run.rect)).collect();
        assert_eq!(lines.iter().map(|(text, _)| *text).collect::<Vec<_>>(), ["old", "$ ls", "ab"]);
        for (line, (text, rect)) in lines.iter().enumerate() {
            assert!(rect.x.abs() < 0.01 && (rect.y - line as f32 * height).abs() < 0.01, "{lines:?}");
            assert!((rect.width - text.chars().count() as f32 * width).abs() < 0.01, "{lines:?}");
        }
    }

    /// One row: `$ ` bold, then `ls`, in one ink.
    fn bold_prompt(cursor: Option<TerminalCursor>) -> Arc<TerminalScreen> {
        let mut cells: Vec<TerminalCell> = row("$ ", GREEN).into_iter().map(|cell| TerminalCell { bold: true, ..cell }).collect();
        cells.extend(row("ls", GREEN));
        Arc::new(TerminalScreen { cols: 4, rows: 1, cells, cursor, ..TerminalScreen::default() })
    }

    #[test]
    fn bold_cells_are_a_run_of_their_own_in_the_bold_face_and_so_is_the_cursor_on_one() {
        let families = |frame: &Frame| -> Vec<(String, FontFamily)> {
            frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, font_family, .. } => Some((text.clone(), *font_family)),
                    _ => None,
                })
                .collect()
        };
        let frame = tree(bold_prompt(None)).layout(Size::new(200.0, 40.0));
        assert_eq!(
            families(&frame),
            [("$ ".into(), FontFamily::MonospaceBold), ("ls".into(), FontFamily::Monospace)]
        );
        let at = TerminalCursor { col: 0, row: 0, shape: TerminalCursorShape::Block };
        let mut tree = tree(bold_prompt(Some(at)));
        tree.layout(Size::new(200.0, 40.0));
        assert!(tree.focus("term"));
        let frame = tree.layout(Size::new(200.0, 40.0));
        assert!(families(&frame).contains(&("$".into(), FontFamily::MonospaceBold)), "{:?}", families(&frame));
    }

    #[test]
    #[cfg(feature = "bundled-fonts")]
    fn the_bold_face_keeps_the_grid() {
        let mut tree = UiTree::new(TerminalView::new("term", bold_prompt(None), palette()).font_size(10.0).render());
        tree.set_text_layout(true);
        let frame = tree.layout(Size::new(200.0, 40.0));
        let width = 10.0 * CELL_WIDTH;
        // Two bold cells and two regular ones: four cells wide.
        let line = &frame.text_layouts()[0].runs[0];
        assert!((line.rect.width - 4.0 * width).abs() < 0.01, "{line:?}");
        // Every printable ASCII character, not just these: Hack Bold advances as Hack does.
        let ascii: String = (' '..='~').collect();
        let measure = |family| crate::TextLayout::new(ascii.as_str(), crate::TextStyle::new(10.0, family)).size().width;
        assert!((measure(FontFamily::MonospaceBold) - ascii.len() as f32 * width).abs() < 0.01);
        assert!((measure(FontFamily::Monospace) - ascii.len() as f32 * width).abs() < 0.01);
    }

    #[test]
    fn without_its_focus_ring_the_focused_view_draws_none() {
        let ring = |frame: &Frame| {
            frame.commands.iter().any(|command| matches!(command,
                DrawCommand::StrokeRect { color, .. } if *color == Color::rgb(170, 170, 170)))
        };
        for (shown, expected) in [(true, true), (false, false)] {
            let mut tree = UiTree::new(TerminalView::new("term", screen(None), palette()).focus_ring(shown).render());
            tree.layout(Size::new(200.0, 100.0));
            assert!(tree.focus("term"));
            assert_eq!(ring(&tree.layout(Size::new(200.0, 100.0))), expected);
        }
        // A press still focuses it, for the host to read the keys.
        let mut tree = UiTree::new(TerminalView::new("term", screen(None), palette()).focus_ring(false).render());
        tree.layout(Size::new(200.0, 100.0));
        tree.handle_event(UiEvent::PointerDown(Point::new(5.0, 5.0)));
        assert_eq!(tree.focused_id(), Some("term"));
    }

    #[test]
    fn inverse_cells_swap_ink_and_fill_and_min_contrast_lifts_dim_ink() {
        let background = Color::rgb(0, 0, 0);
        let dim = Color::rgb(10, 10, 40);
        let cells = vec![
            TerminalCell { inverse: true, ..TerminalCell::new('x', RED) },
            TerminalCell::new('y', dim),
        ];
        let screen = Arc::new(TerminalScreen { cols: 2, rows: 1, cells, ..TerminalScreen::default() });
        let view = |ratio: Option<f32>| {
            let view = TerminalView::new("term", screen.clone(), palette()).background(background);
            let mut tree = UiTree::new(match ratio {
                Some(ratio) => view.min_contrast(ratio),
                None => view,
            }
            .render());
            tree.layout(Size::new(200.0, 40.0))
        };
        let plain = view(None);
        assert!(plain.commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { color, .. } if *color == RED)));
        assert!(texts(&plain).contains(&("x".into(), background)));
        assert!(texts(&plain).contains(&("y".into(), dim)));
        let lifted = texts(&view(Some(4.5)));
        let (_, ink) = lifted.iter().find(|(text, _)| text == "y").unwrap();
        assert!(crate::chip::contrast(*ink, background) >= 4.5, "{ink:?}");
    }

    #[test]
    fn grid_size_fits_whole_cells_and_survives_bad_input() {
        let (width, height) = (10.0 * CELL_WIDTH, 13.0);
        assert_eq!(TerminalView::grid_size(Size::new(80.0 * width + 1.0, 24.0 * height + 1.0), 10.0), (80, 24));
        assert_eq!(TerminalView::grid_size(Size::new(80.0 * width - 1.0, 24.0 * height - 1.0), 10.0), (79, 23));
        assert_eq!(TerminalView::grid_size(Size::new(0.0, f32::NAN), 10.0), (1, 1));
        assert_eq!(
            TerminalView::grid_size(Size::new(400.0, 300.0), f32::NAN),
            TerminalView::grid_size(Size::new(400.0, 300.0), FONT_SIZE)
        );
    }

    #[test]
    fn a_press_focuses_the_view_and_a_drag_selects_text_to_copy() {
        let mut tree = tree(screen(None));
        let frame = tree.layout(Size::new(200.0, 100.0));
        let start = frame.text_layouts()[0].runs[0].rect;
        tree.handle_event(UiEvent::PointerDown(Point::new(start.x + 1.0, start.y + 4.0)));
        assert_eq!(tree.focused_id(), Some("term"));
        tree.handle_event(UiEvent::PointerMove(Point::new(150.0, 3.0 * 13.0 - 4.0)));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(Point::new(150.0, 3.0 * 13.0 - 4.0))), vec![]);
        assert_eq!(
            tree.handle_event(UiEvent::Key { key: Key::C, ctrl: true, shift: true }),
            vec![UiAction::CopyRequested { text: "old\n$ ls\nab".into() }]
        );
    }

    #[test]
    fn the_cursor_blinks_while_focused_and_holds_still_under_reduced_motion() {
        let at = TerminalCursor { col: 2, row: 1, shape: TerminalCursorShape::Bar };
        let size = Size::new(200.0, 100.0);
        let start = Instant::now();
        let mut tree = tree(screen(Some(at)));
        assert_eq!(cursor(&tree.layout_at(size, start)), None, "hidden without the focus");
        assert!(tree.focus("term"));
        let shown = cursor(&tree.layout_at(size, start)).unwrap();
        assert_eq!(shown, Rect::new(2.0 * 10.0 * CELL_WIDTH, 2.0 * 13.0, BAR_WIDTH, 13.0));
        assert_eq!(cursor(&tree.layout_at(size, start + Duration::from_millis(500))), None);
        assert!(cursor(&tree.layout_at(size, start + Duration::from_millis(1000))).is_some());
        tree.set_reduced_motion(true);
        for later in [1500, 2000, 2500] {
            assert!(cursor(&tree.layout_at(size, start + Duration::from_millis(later))).is_some());
        }
        assert_eq!(tree.caret_repaint_after(), None);
        tree.set_root(TerminalView::new("term", screen(None), palette()).font_size(10.0).render());
        assert_eq!(cursor(&tree.layout_at(size, start)), None, "a hidden cursor");
    }

    #[test]
    fn a_block_cursor_draws_its_character_in_the_cells_fill() {
        let at = TerminalCursor { col: 0, row: 1, shape: TerminalCursorShape::Block };
        let mut tree = tree(screen(Some(at)));
        tree.layout(Size::new(200.0, 100.0));
        assert!(tree.focus("term"));
        let frame = tree.layout(Size::new(200.0, 100.0));
        assert_eq!(cursor(&frame), Some(Rect::new(0.0, 2.0 * 13.0, 10.0 * CELL_WIDTH, 13.0)));
        assert!(texts(&frame).contains(&("a".into(), BLUE)));
    }
}
