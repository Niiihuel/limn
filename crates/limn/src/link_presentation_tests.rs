use crate::{
    Color, DrawCommand, Element, FontFamily, Icon, Key, Point, Size, TextHighlight, TextSpan,
    UiAction, UiEvent, UiTree,
};
use unicode_segmentation::UnicodeSegmentation;

#[test]
fn decorative_link_icon_reserves_width_but_never_enters_selection_or_copy() {
    let root = Element::formatted_text(vec![
        TextSpan::new("á ", Color::WHITE, 14.0, FontFamily::Sans),
        TextSpan::new("GitHub", Color::WHITE, 14.0, FontFamily::Sans)
            .icon(Icon::Github)
            .action("repository"),
        TextSpan::new(" fin", Color::WHITE, 14.0, FontFamily::Sans),
    ]);
    let mut tree = UiTree::new(root);
    tree.set_text_measurer(|text, _, _| text.graphemes(true).count() as f32 * 5.0);
    let frame = tree.layout(Size::new(200.0, 50.0));
    let icon = frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Icon {
                rect,
                icon: Icon::Github,
                ..
            } => Some(*rect),
            _ => None,
        })
        .unwrap();
    assert_eq!(icon.x, 10.0);
    assert_eq!(
        frame.hit_test(Point::new(icon.x + 1.0, icon.y + 1.0)),
        Some("repository")
    );
    let layouts = frame.text_layouts();
    let label = layouts[0]
        .runs
        .iter()
        .find(|run| run.text == "GitHub")
        .unwrap();
    assert_eq!(label.rect.x, 30.0);
    assert_eq!(frame.text_layouts()[0].source, "á GitHub fin");
    tree.handle_event(UiEvent::PointerDown(Point::new(0.0, 4.0)));
    tree.handle_event(UiEvent::PointerMove(Point::new(150.0, 4.0)));
    tree.handle_event(UiEvent::PointerUp(Point::new(150.0, 4.0)));
    assert_eq!(tree.selected_text(), Some("á GitHub fin"));
    assert_eq!(
        tree.handle_event(UiEvent::Key {
            key: Key::C,
            shift: false,
            ctrl: true
        }),
        vec![UiAction::CopyRequested {
            text: "á GitHub fin".into()
        }]
    );
}

#[test]
fn linked_icon_wraps_with_its_label_and_invalidates_cached_layout() {
    let root = |icon| {
        Element::formatted_text(vec![
            TextSpan::new("prefix ", Color::WHITE, 14.0, FontFamily::Sans),
            TextSpan::new("repository", Color::WHITE, 14.0, FontFamily::Sans)
                .icon(icon)
                .action("link"),
        ])
    };
    let mut tree = UiTree::new(root(Icon::Globe));
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 5.0);
    let viewport = Size::new(50.0, 100.0);
    tree.layout(viewport);
    tree.set_root(root(Icon::Github));
    let frame = tree.layout(viewport);
    let icon = frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Icon {
                rect,
                icon: Icon::Github,
                ..
            } => Some(*rect),
            _ => None,
        })
        .unwrap();
    let layouts = frame.text_layouts();
    let first = layouts[0]
        .runs
        .iter()
        .find(|run| run.text.starts_with("repos"))
        .unwrap();
    assert_eq!(first.rect.x, 20.0);
    assert!(first.rect.y > 0.0);
    assert!(icon.y >= first.rect.y && icon.y < first.rect.y + first.rect.height);
    assert_eq!(frame.text_layouts()[0].source, "prefix repository");
    assert!(frame.text_layouts()[0]
        .runs
        .iter()
        .all(|run| run.rect.x + run.rect.width <= 50.0));
}

#[test]
fn editable_url_color_preserves_unicode_geometry_selection_copy_and_stale_edits() {
    let source = "ñ🦀 e\u{301} https://github.com/a/b\nfin";
    let start = source[..source.find("https:").unwrap()].chars().count();
    let blue = Color::rgb(120, 169, 255);
    let root = |styled| {
        Element::text_area("prompt", source, "")
            .text_area_highlights(if styled {
                vec![TextHighlight {
                    range: start..start + "https://github.com/a/b".len(),
                    color: blue,
                    italic: false,
                }]
            } else {
                vec![]
            })
            .font_family(FontFamily::Sans)
            .foreground(Color::WHITE)
            .padding(0.0)
            .size(Some(85.0), Some(160.0))
    };
    let mut plain = UiTree::new(root(false));
    let mut styled = UiTree::new(root(true));
    for tree in [&mut plain, &mut styled] {
        tree.set_text_measurer(|text, _, _| text.graphemes(true).count() as f32 * 5.0);
        tree.layout(Size::new(85.0, 160.0));
        assert!(tree.focus("prompt"));
        tree.handle_event(UiEvent::Key {
            key: Key::Home,
            shift: false,
            ctrl: true,
        });
        tree.handle_event(UiEvent::Key {
            key: Key::ArrowRight,
            shift: true,
            ctrl: false,
        });
        assert_eq!(tree.input_value("prompt"), Some(source));
    }
    assert_eq!(
        plain.input_selection("prompt"),
        styled.input_selection("prompt")
    );
    let plain_frame = plain.layout(Size::new(85.0, 160.0));
    let styled_frame = styled.layout(Size::new(85.0, 160.0));
    let white_text = |frame: &crate::Frame| {
        frame
            .commands
            .iter()
            .filter_map(|cmd| match cmd {
                DrawCommand::Text {
                    position,
                    text,
                    color,
                    ..
                } if *color == Color::WHITE => Some((*position, text.clone())),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(white_text(&plain_frame), white_text(&styled_frame));
    assert!(styled_frame
        .commands
        .iter()
        .any(|cmd| matches!(cmd, DrawCommand::Text { color, .. } if *color == blue)));
    styled.handle_event(UiEvent::Key {
        key: Key::A,
        shift: false,
        ctrl: true,
    });
    assert_eq!(
        styled.handle_event(UiEvent::Key {
            key: Key::C,
            shift: false,
            ctrl: true
        }),
        vec![UiAction::CopyRequested {
            text: source.into()
        }]
    );
    styled.handle_event(UiEvent::Text("nuevo".into()));
    let stale = styled.layout(Size::new(85.0, 160.0));
    assert_eq!(styled.input_value("prompt"), Some("nuevo"));
    assert!(!stale
        .commands
        .iter()
        .any(|cmd| matches!(cmd, DrawCommand::Text { color, .. } if *color == blue)));
}

#[test]
fn a_slanted_highlight_replaces_the_upright_ink_under_it() {
    let dim = Color::rgb(120, 120, 120);
    let mut tree = UiTree::new(
        Element::text_area("prompt", "hola dictado fin", "")
            .text_area_highlights(vec![TextHighlight {
                range: 5..12,
                color: dim,
                italic: true,
            }])
            .font_family(FontFamily::Sans)
            .foreground(Color::WHITE)
            .padding(0.0)
            .size(Some(200.0), Some(40.0)),
    );
    tree.set_text_measurer(|text, _, _| text.graphemes(true).count() as f32 * 5.0);
    let frame = tree.layout(Size::new(200.0, 40.0));
    // Each text is painted through the clip pushed just before it.
    let painted = |family: FontFamily| {
        frame
            .commands
            .windows(2)
            .filter_map(|pair| match (&pair[0], &pair[1]) {
                (DrawCommand::PushClip(clip), DrawCommand::Text { font_family, .. })
                    if *font_family == family =>
                {
                    Some((clip.x, clip.x + clip.width))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(painted(FontFamily::SansItalic), vec![(25.0, 63.0)]);
    assert_eq!(painted(FontFamily::Sans), vec![(0.0, 25.0), (60.0, 200.0)]);
}

#[test]
fn a_text_area_shows_its_scrollbar_while_it_scrolls_and_then_hides_it() {
    use std::time::{Duration, Instant};
    let size = Size::new(200.0, 60.0);
    let mut tree = UiTree::new(
        Element::text_area("prompt", "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12", "")
            .foreground(Color::WHITE)
            .size(Some(200.0), Some(60.0)),
    );
    let start = Instant::now();
    let bar = |frame: &crate::Frame| {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillRect { rect, .. } if rect.width == 4.0 => Some(*rect),
            _ => None,
        })
    };
    // The first frame scrolls to the caret at the end.
    let bottom = bar(&tree.layout_at(size, start)).expect("scrolling shows the bar");
    assert!(tree.animation_repaint_after().is_some());
    let later = start + Duration::from_secs(2);
    assert_eq!(bar(&tree.layout_at(size, later)), None);
    tree.handle_event(UiEvent::Scroll {
        position: Point::new(20.0, 20.0),
        delta_y: 400.0,
    });
    let top = bar(&tree.layout_at(size, later)).expect("the wheel shows it again");
    assert!(top.y < bottom.y);
    // Resting the pointer on the text keeps it shown.
    tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 20.0)));
    assert!(bar(&tree.layout_at(size, later + Duration::from_secs(5))).is_some());
}
