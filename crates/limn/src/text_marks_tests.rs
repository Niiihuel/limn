use super::*;
use crate::{ComponentPalette, TextHighlight};

fn span(text: &str) -> TextSpan {
    TextSpan::new(text, Color::WHITE, 14.0, FontFamily::Sans)
}

/// A tree whose text is 8 px a character.
fn tree(element: Element, size: Size) -> (UiTree, Frame) {
    let mut tree = UiTree::new(element);
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
    let frame = tree.layout(size);
    (tree, frame)
}

fn fills(frame: &Frame, ink: Color) -> Vec<Rect> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, .. } if *color == ink => Some(*rect),
            _ => None,
        })
        .collect()
}

fn painted(frame: &Frame) -> Vec<&str> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn underlined_and_struck_runs_draw_their_lines_in_their_ink_on_every_line() {
    let red = Color::rgb(200, 40, 40);
    let blue = Color::rgb(40, 40, 200);
    let (_, frame) = tree(
        Element::formatted_text(vec![
            span("plain "),
            TextSpan::new("under line", red, 14.0, FontFamily::Sans).underline(),
            TextSpan::new("gone", blue, 14.0, FontFamily::Sans).strikethrough(),
        ])
        .size(Some(100.0), None),
        Size::new(100.0, 200.0),
    );
    // "under " ends the first line; "line" wraps and "gone" follows it.
    let under = fills(&frame, red);
    assert_eq!(under.len(), 2, "{under:?}");
    assert_eq!(
        (under[0].x, under[0].width, under[0].height),
        (48.0, 48.0, 1.0)
    );
    assert_eq!((under[1].x, under[1].width), (0.0, 32.0));
    assert!(under[1].y > under[0].y);
    let through = fills(&frame, blue);
    assert_eq!(through.len(), 1);
    assert_eq!((through[0].x, through[0].width), (32.0, 32.0));
    // Through the middle of the glyphs, above the underline of its line.
    assert!(through[0].y < under[1].y && through[0].y > under[0].y + 1.0);
    // Larger text, thicker lines.
    let (_, frame) = tree(
        Element::formatted_text(vec![
            TextSpan::new("big", red, 28.0, FontFamily::Sans).underline()
        ]),
        Size::new(200.0, 100.0),
    );
    assert_eq!(fills(&frame, red)[0].height, 2.0);
}

#[test]
fn a_run_with_a_hint_shows_it_once_the_pointer_rests_on_it() {
    let palette = ComponentPalette::neutral_dark();
    let size = Size::new(300.0, 200.0);
    let root = Element::column(vec![
        Element::spacer().size(None, Some(100.0)),
        Element::formatted_text(vec![
            span("hola "),
            span("(editado)").tooltip("Editado el martes", palette),
            span(" fin").tooltip(" ", palette),
        ]),
    ]);
    let (mut tree, frame) = tree(root, size);
    // Only its run is a target; the blank hint is none.
    let targets: Vec<&HitRegion> = frame
        .regions
        .iter()
        .filter(|region| region.kind == HitKind::Hover)
        .collect();
    assert_eq!(targets.len(), 1);
    assert_eq!((targets[0].rect.x, targets[0].rect.width), (40.0, 72.0));
    tree.handle_event(UiEvent::PointerMove(Point::new(60.0, 105.0)));
    let started = tree.tooltip_hover_since.as_ref().unwrap().1;
    let early = tree.layout_at(size, started + TOOLTIP_HOVER_DELAY / 2);
    assert!(!painted(&early).contains(&"Editado el martes"));
    assert!(tree.is_animating(), "it wakes for the hint");
    let late = tree.layout_at(size, started + TOOLTIP_HOVER_DELAY);
    assert!(painted(&late).contains(&"Editado el martes"));
    // A press on it still starts a selection of the text.
    tree.handle_event(UiEvent::PointerDown(Point::new(42.0, 105.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(66.0, 105.0)));
    assert_eq!(tree.selected_text(), Some("(ed"));
    tree.handle_event(UiEvent::PointerUp(Point::new(66.0, 105.0)));
    // Over the text before it, none.
    tree.handle_event(UiEvent::PointerMove(Point::new(8.0, 105.0)));
    // The first frame without it fades it out.
    tree.layout_at(size, started + TOOLTIP_HOVER_DELAY * 4);
    let away = tree.layout_at(size, started + TOOLTIP_HOVER_DELAY * 8);
    assert!(!painted(&away).contains(&"Editado el martes"));
}

#[test]
fn two_texts_with_the_same_hint_show_it_only_over_the_one_under_the_pointer() {
    let palette = ComponentPalette::neutral_dark();
    let size = Size::new(300.0, 200.0);
    let edited = || {
        Element::formatted_text(vec![
            span("hola "),
            span("(editado)").tooltip("Editado el martes", palette),
        ])
    };
    let root = Element::column(vec![
        edited(),
        Element::spacer().size(None, Some(60.0)),
        edited(),
    ]);
    let (mut tree, frame) = tree(root, size);
    let targets: Vec<&str> = frame
        .regions
        .iter()
        .filter(|region| region.kind == HitKind::Hover)
        .map(|region| region.id.as_str())
        .collect();
    assert_eq!(targets.len(), 2);
    assert_ne!(targets[0], targets[1]);
    tree.handle_event(UiEvent::PointerMove(Point::new(60.0, 5.0)));
    let started = tree.tooltip_hover_since.as_ref().unwrap().1;
    let late = tree.layout_at(size, started + TOOLTIP_HOVER_DELAY);
    let shown = painted(&late)
        .into_iter()
        .filter(|text| *text == "Editado el martes")
        .count();
    assert_eq!(shown, 1);
}

#[test]
fn one_action_with_a_hint_in_two_texts_shows_it_once() {
    let palette = ComponentPalette::neutral_dark();
    let size = Size::new(300.0, 200.0);
    let moment = || {
        Element::formatted_text(vec![span("en 2 días")
            .action("moment:1")
            .tooltip("martes 14", palette)])
    };
    let root = Element::column(vec![
        moment(),
        Element::spacer().size(None, Some(60.0)),
        moment(),
    ]);
    let (mut tree, _) = tree(root, size);
    tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 5.0)));
    let started = tree.tooltip_hover_since.as_ref().unwrap().1;
    let late = tree.layout_at(size, started + TOOLTIP_HOVER_DELAY);
    let shown = painted(&late)
        .into_iter()
        .filter(|text| *text == "martes 14")
        .count();
    assert_eq!(shown, 1);
}

#[test]
fn a_hinted_link_wrapped_over_two_lines_shows_one_hint_for_the_keyboard() {
    let palette = ComponentPalette::neutral_dark();
    let size = Size::new(300.0, 200.0);
    let root = Element::column(vec![Element::formatted_text(vec![
        span("ver "),
        span("una fecha larga")
            .action("fecha")
            .tooltip("martes 14", palette),
    ])
    .size(Some(100.0), None)]);
    let (mut tree, frame) = tree(root, size);
    let parts = frame
        .regions
        .iter()
        .filter(|region| region.id == "fecha")
        .count();
    assert!(parts > 1, "the link wraps");
    tree.handle_event(UiEvent::Key {
        key: Key::Tab,
        shift: false,
        ctrl: false,
    });
    let frame = tree.layout(size);
    let hints = painted(&frame)
        .into_iter()
        .filter(|text| *text == "martes 14")
        .count();
    assert_eq!(hints, 1);
}

#[test]
fn a_text_cut_to_its_lines_ends_in_an_ellipsis_that_is_not_copied() {
    let ink = Color::rgb(1, 2, 3);
    let text = |width: f32, lines: usize| {
        Element::column(vec![
            Element::formatted_text(vec![TextSpan::new(
                "uno dos tres cuatro cinco",
                ink,
                14.0,
                FontFamily::Sans,
            )])
            .max_lines(lines)
            .size(Some(width), None),
            Element::text("after"),
        ])
    };
    // "uno dos tres" fills 96 of 100; "tre" gives up its "s" for the «…».
    let (_, frame) = tree(text(100.0, 1), Size::new(100.0, 200.0));
    let shown = painted(&frame);
    assert_eq!(shown[..2], ["uno dos tre", "…"]);
    let ellipsis = frame.commands.iter().find_map(|command| match command {
        DrawCommand::Text {
            text,
            color,
            position,
            ..
        } if text == "…" => Some((*color, *position)),
        _ => None,
    });
    assert_eq!(ellipsis.map(|(color, at)| (color, at.x)), Some((ink, 88.0)));
    // One line tall: what follows starts under it.
    let after = frame.commands.iter().find_map(|command| match command {
        DrawCommand::Text { text, position, .. } if text == "after" => Some(position.y),
        _ => None,
    });
    let layouts = frame.text_layouts();
    assert_eq!(after, Some(layouts[0].runs[0].rect.height));
    let copied: Vec<&str> = layouts[0]
        .runs
        .iter()
        .map(|run| run.text.as_str())
        .collect();
    assert_eq!(copied, ["uno dos tre"]);
    // A line ending in a space gives up the space, not a letter.
    let (_, frame) = tree(text(104.0, 1), Size::new(104.0, 200.0));
    assert_eq!(painted(&frame)[..2], ["uno dos tres", "…"]);
    // What fits its lines shows whole.
    let (_, frame) = tree(text(400.0, 2), Size::new(400.0, 200.0));
    assert_eq!(painted(&frame)[0], "uno dos tres cuatro cinco");
    assert!(!painted(&frame).contains(&"…"));
}

#[test]
fn a_highlight_with_a_fill_paints_it_under_its_characters_and_the_text() {
    let pill = Color::rgb(30, 60, 200);
    let (_, frame) = tree(
        Element::text_area("draft", "hola @ada", "")
            .text_area_highlights(vec![TextHighlight {
                range: 5..9,
                color: Color::WHITE,
                italic: false,
                background: Some(pill),
            }])
            .font_family(FontFamily::Sans)
            .padding(0.0)
            .size(Some(200.0), Some(40.0)),
        Size::new(200.0, 40.0),
    );
    let fill = fills(&frame, pill);
    assert_eq!(fill.len(), 1);
    assert_eq!((fill[0].x, fill[0].width), (40.0, 32.0));
    let fill_at = frame.commands.iter().position(
        |command| matches!(command, DrawCommand::FillRect { color, .. } if *color == pill),
    );
    let text_at = frame
        .commands
        .iter()
        .position(|command| matches!(command, DrawCommand::Text { .. }));
    assert!(fill_at < text_at, "the fill is under the text");
}
