//! Selection decoration only: source offsets, glyph advances and hit testing
//! remain in the editor. Adjacent visual rows share one rounded contour.
use super::*;

const RADIUS: f32 = 3.0;
const CAP_INSET: f32 = 0.0;
pub(super) const HORIZONTAL_PADDING: f32 = 1.5;
pub(super) const COLOR: Color = Color::rgba(75, 130, 210, 100);
const CURVE_STEPS: usize = 6;

/// Rectangles arrive in visual order for one normalized selection. Disjoint
/// spans stay disjoint; a folded or wrapped visual row is otherwise ordinary.
pub(super) fn contours(rows: &[Rect]) -> Vec<Arc<[Point]>> {
    // Decoration may breathe around a glyph without changing its advance,
    // selection offsets, caret position, or the source viewport's hit testing.
    let rows: Vec<_> = rows
        .iter()
        .map(|row| {
            if valid(*row) {
                Rect::new(
                    row.x - HORIZONTAL_PADDING,
                    row.y,
                    row.width + HORIZONTAL_PADDING * 2.0,
                    row.height,
                )
            } else {
                *row
            }
        })
        .collect();
    let mut result = Vec::new();
    let mut start = 0;
    while start < rows.len() {
        if !valid(rows[start]) {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end < rows.len() && valid(rows[end]) && connected(rows[end - 1], rows[end]) {
            end += 1;
        }
        let points = contour(&rows[start..end]);
        if points.len() >= 3 {
            result.push(points.into());
        }
        start = end;
    }
    result
}
fn valid(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width > 0.0
        && rect.height > CAP_INSET * 2.0
}
fn connected(a: Rect, b: Rect) -> bool {
    (a.y + a.height - b.y).abs() < 0.01 && a.x.max(b.x) < (a.x + a.width).min(b.x + b.width) - 0.001
}
fn contour(rows: &[Rect]) -> Vec<Point> {
    let first = rows[0];
    let last = rows[rows.len() - 1];
    let top = first.y + CAP_INSET;
    let bottom = last.y + last.height - CAP_INSET;
    let mut polygon = Vec::with_capacity(rows.len() * 4 + 4);
    polygon.push(Point::new(first.x, top));
    polygon.push(Point::new(first.x + first.width, top));
    for (index, row) in rows.iter().enumerate() {
        let edge = if index + 1 == rows.len() {
            bottom
        } else {
            row.y + row.height
        };
        polygon.push(Point::new(row.x + row.width, edge));
        if let Some(next) = rows.get(index + 1) {
            polygon.push(Point::new(next.x + next.width, edge));
        }
    }
    polygon.push(Point::new(last.x, bottom));
    for index in (0..rows.len()).rev() {
        let row = rows[index];
        let edge = if index == 0 { top } else { row.y };
        polygon.push(Point::new(row.x, edge));
        if index > 0 {
            polygon.push(Point::new(rows[index - 1].x, edge));
        }
    }
    polygon.dedup();
    if polygon.first() == polygon.last() {
        polygon.pop();
    }
    // Flat joins keep the complete shared edge, with no rounded seam per row.
    let corners: Vec<_> = (0..polygon.len())
        .filter_map(|index| {
            let a = polygon[(index + polygon.len() - 1) % polygon.len()];
            let b = polygon[index];
            let c = polygon[(index + 1) % polygon.len()];
            let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
            (cross.abs() > 0.001).then_some(b)
        })
        .collect();
    let mut rounded = Vec::with_capacity(corners.len() * (CURVE_STEPS + 1));
    for index in 0..corners.len() {
        let a = corners[(index + corners.len() - 1) % corners.len()];
        let b = corners[index];
        let c = corners[(index + 1) % corners.len()];
        let incoming = (b.x - a.x).hypot(b.y - a.y);
        let outgoing = (c.x - b.x).hypot(c.y - b.y);
        let radius = RADIUS.min(incoming * 0.5).min(outgoing * 0.5);
        let start = Point::new(
            b.x + (a.x - b.x) * radius / incoming,
            b.y + (a.y - b.y) * radius / incoming,
        );
        let end = Point::new(
            b.x + (c.x - b.x) * radius / outgoing,
            b.y + (c.y - b.y) * radius / outgoing,
        );
        rounded.push(start);
        for step in 1..=CURVE_STEPS {
            let t = step as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            rounded.push(if step == CURVE_STEPS {
                end
            } else {
                // Relative evaluation keeps joins exactly on their shared
                // scanline, even far from the origin at fractional UI scales.
                Point::new(
                    start.x + 2.0 * u * t * (b.x - start.x) + t * t * (end.x - start.x),
                    start.y + 2.0 * u * t * (b.y - start.y) + t * t * (end.y - start.y),
                )
            });
        }
    }
    rounded.dedup();
    rounded
}

#[cfg(test)]
mod tests {
    use super::*;
    fn intervals(points: &[Point], y: f32) -> Vec<(f32, f32)> {
        let mut xs = Vec::new();
        for (a, b) in points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .take(points.len())
        {
            if (a.y <= y && b.y > y) || (b.y <= y && a.y > y) {
                xs.push(a.x + (b.x - a.x) * (y - a.y) / (b.y - a.y));
            }
        }
        xs.sort_by(f32::total_cmp);
        xs.chunks_exact(2).map(|pair| (pair[0], pair[1])).collect()
    }
    #[test]
    fn connected_rows_have_one_contour_with_flat_joins_and_soft_inner_and_outer_steps() {
        let paths = contours(&[
            Rect::new(20.0, 0.0, 80.0, 20.0),
            Rect::new(0.0, 20.0, 120.0, 20.0),
            Rect::new(0.0, 40.0, 60.0, 20.0),
        ]);
        assert_eq!(paths.len(), 1);
        let points = &paths[0];
        for y in [1.0, 10.0, 19.5, 20.5, 39.5, 40.5, 59.0] {
            assert_eq!(
                intervals(points, y).len(),
                1,
                "single connected span at {y}"
            );
        }
        assert_eq!(
            intervals(points, 30.0),
            vec![(-HORIZONTAL_PADDING, 120.0 + HORIZONTAL_PADDING)]
        );
        assert!(
            intervals(points, 19.5)[0].0 < 20.0,
            "concave join curves towards the next row"
        );
        assert!(
            intervals(points, 20.5)[0].0 > -HORIZONTAL_PADDING,
            "convex step is softened"
        );
        let flat = contours(&[
            Rect::new(0.0, 0.0, 40.0, 20.0),
            Rect::new(0.0, 20.0, 40.0, 20.0),
        ]);
        assert_eq!(intervals(&flat[0], 19.99), intervals(&flat[0], 20.01));
        assert_eq!(
            intervals(&flat[0], 20.0),
            vec![(-HORIZONTAL_PADDING, 40.0 + HORIZONTAL_PADDING)]
        );
    }
    #[test]
    fn disconnected_ranges_remain_independent_and_narrow_newline_spans_stay_finite() {
        let paths = contours(&[
            Rect::new(0.0, 0.0, 1.5, 20.0),
            Rect::new(8.0, 20.0, 1.5, 20.0),
            Rect::new(8.0, 60.0, 20.0, 20.0),
        ]);
        assert_eq!(paths.len(), 3);
        assert!(paths
            .iter()
            .flat_map(|path| path.iter())
            .all(|point| point.x.is_finite() && point.y.is_finite()));
        assert_eq!(
            intervals(&paths[0], 10.0),
            vec![(-HORIZONTAL_PADDING, 1.5 + HORIZONTAL_PADDING)]
        );
        assert!(contours(&[Rect::new(0.0, 0.0, 0.0, 20.0)]).is_empty());
        assert!(contours(&[Rect::new(f32::NAN, 0.0, 10.0, 20.0)]).is_empty());
    }
    #[test]
    fn varied_staircases_keep_simple_y_monotone_contours_and_bounded_work() {
        let rows: Vec<_> = (0..80)
            .map(|index| {
                Rect::new(
                    (index % 5) as f32 * 3.0,
                    index as f32 * 20.0,
                    18.0 + (index % 7) as f32 * 9.0,
                    20.0,
                )
            })
            .collect();
        let paths = contours(&rows);
        assert_eq!(paths.len(), 1);
        assert!(paths[0].len() <= rows.len() * 4 * (CURVE_STEPS + 1));
        for step in 4..6396 {
            let spans = intervals(&paths[0], step as f32 * 0.25);
            assert_eq!(spans.len(), 1);
            assert!(spans[0].0 < spans[0].1);
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    fn measure(text: &str) -> f32 {
        unicode_width::UnicodeWidthStr::width(text) as f32 * 8.0
    }
    fn draw(view: &mut EditorView) -> Vec<DrawCommand> {
        let rect = Rect::new(0.0, 0.0, 360.0, 180.0);
        view.paint(rect, rect, true, &mut |_, _, _| true, &measure, 16.0)
    }
    #[test]
    fn native_selection_uses_exact_tabs_unicode_and_empty_line_geometry_without_changing_source() {
        let source = "\tlet café = \"🧠\";\r\n\r\n    東京();\r\nend();";
        let controller = EditorController::new(TextBuffer::new(source).unwrap());
        let before = controller.snapshot().unwrap();
        let end = before.line_range(2).unwrap().end;
        controller
            .execute(EditorCommand::SetSelection(Selection {
                anchor: 0,
                head: end,
            }))
            .unwrap();
        let mut view = EditorView::new(
            CodeEditor::new("selection", controller.clone()),
            Instant::now(),
        );
        let commands = draw(&mut view);
        let paths: Vec<_> = commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::SelectionContour { points, color } => {
                    assert_eq!(color.a, 100);
                    Some(points)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            paths.len(),
            1,
            "newline-only row remains connected to adjacent selected rows"
        );
        let left = paths[0]
            .iter()
            .map(|point| point.x)
            .fold(f32::INFINITY, f32::min);
        assert_eq!(left, view.viewport.body.x - HORIZONTAL_PADDING);
        let first = visible_display_line(&before, 0, 0.0, 1000.0, 4, &measure).0;
        let right = paths[0]
            .iter()
            .filter(|point| point.y < view.viewport.body.y + LINE_HEIGHT)
            .map(|point| point.x)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            right <= view.viewport.body.x + first.width + measure(" ") + HORIZONTAL_PADDING + 0.01
        );
        assert!(right >= view.viewport.body.x + first.width);
        let after = controller.snapshot().unwrap();
        assert_eq!(after.revision(), before.revision());
        assert_eq!(after.text_range(0..after.char_count()).unwrap(), source);
        assert_eq!(
            after.selection(),
            Selection {
                anchor: 0,
                head: end
            }
        );
    }
    #[test]
    fn selection_padding_does_not_move_unicode_hit_boundaries_or_caret() {
        let controller = EditorController::new(TextBuffer::new("a\t東京e\u{301}z").unwrap());
        controller
            .execute(EditorCommand::SetSelection(Selection {
                anchor: 2,
                head: 6,
            }))
            .unwrap();
        let snapshot = controller.snapshot().unwrap();
        let mut view = EditorView::new(
            CodeEditor::new("padded-selection", controller.clone()),
            Instant::now(),
        );
        let commands = draw(&mut view);
        let display = visible_display_line(&snapshot, 0, 0.0, 1000.0, 4, &measure).0;
        let start = view.viewport.body.x + display.x_at(2);
        let end = view.viewport.body.x + display.x_at(6);
        let points = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::SelectionContour { points, .. } => Some(points),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            points
                .iter()
                .map(|point| point.x)
                .fold(f32::INFINITY, f32::min),
            start - HORIZONTAL_PADDING
        );
        assert_eq!(
            points
                .iter()
                .map(|point| point.x)
                .fold(f32::NEG_INFINITY, f32::max),
            end + HORIZONTAL_PADDING
        );
        let (_, ime_caret) = view.ime_rects.expect("focused primary selection endpoint");
        assert_eq!(ime_caret.x, end);
        assert_eq!(ime_caret.width, 1.5);
        assert_eq!(
            view.offset_at_point(
                &snapshot,
                Point::new(end, view.viewport.body.y + 10.0),
                &measure
            ),
            6
        );
        assert_eq!(
            controller.snapshot().unwrap().selection(),
            snapshot.selection()
        );
        assert_eq!(
            controller.snapshot().unwrap().revision(),
            snapshot.revision()
        );
        // A nonempty selection hides the ordinary caret. Once collapsed, its
        // painted position still follows the original advance, not the padding.
        controller
            .execute(EditorCommand::SetSelection(Selection::caret(6)))
            .unwrap();
        let commands = draw(&mut view);
        assert!(commands.iter().any(|command| matches!(command,
            DrawCommand::FillRect { rect, color, .. }
                if rect.x == end && rect.width == 1.5 && *color == view.editor.palette.text
        )));
    }
    #[test]
    fn multiple_selections_keep_independent_contours_and_clipped_continuations_have_no_false_caps()
    {
        let source = (0..100)
            .map(|row| format!("line_{row:03}_content"))
            .collect::<Vec<_>>()
            .join("\n");
        let controller = EditorController::new(TextBuffer::new(&source).unwrap());
        let snapshot = controller.snapshot().unwrap();
        let selections = [
            Selection {
                anchor: 0,
                head: snapshot.line_range(30).unwrap().end,
            },
            Selection {
                anchor: snapshot.line_range(40).unwrap().start,
                head: snapshot.line_range(50).unwrap().end,
            },
        ];
        controller
            .shared_buffer()
            .lock()
            .unwrap()
            .set_selections(selections.to_vec())
            .unwrap();
        let mut view = EditorView::new(
            CodeEditor::new("selection", controller.clone()).soft_wrap(true),
            Instant::now(),
        );
        view.viewport.reveal_caret = false;
        draw(&mut view);
        view.viewport.scroll(0.0, -240.0);
        let commands = draw(&mut view);
        let points = commands
            .iter()
            .find_map(|command| match command {
                DrawCommand::SelectionContour { points, .. } => Some(points),
                _ => None,
            })
            .unwrap();
        assert!(
            points.iter().any(|point| point.y < view.viewport.body.y),
            "neighbor above viewport preserves an open continuation"
        );
        assert!(
            points
                .iter()
                .any(|point| point.y > view.viewport.body.y + view.viewport.body.height),
            "neighbor below viewport preserves an open continuation"
        );
        let mut depth = 0;
        for command in &commands {
            match command {
                DrawCommand::PushClip(_) => depth += 1,
                DrawCommand::PopClip => {
                    depth -= 1;
                    assert!(depth >= 0);
                }
                DrawCommand::SelectionContour { .. } => assert!(
                    depth >= 2,
                    "selection is clipped by editor and source viewport"
                ),
                _ => {}
            }
        }
        assert_eq!(depth, 0);
        assert_eq!(
            controller
                .snapshot()
                .unwrap()
                .selections()
                .collect::<Vec<_>>(),
            selections
        );
        controller
            .shared_buffer()
            .lock()
            .unwrap()
            .set_selections(vec![
                Selection { anchor: 0, head: 4 },
                Selection {
                    anchor: snapshot.line_range(2).unwrap().start,
                    head: snapshot.line_range(2).unwrap().start + 4,
                },
            ])
            .unwrap();
        view.viewport.scroll_y = 0.0;
        let commands = draw(&mut view);
        assert_eq!(
            commands
                .iter()
                .filter(|command| matches!(command, DrawCommand::SelectionContour { .. }))
                .count(),
            2
        );
    }
}
