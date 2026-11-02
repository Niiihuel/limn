use crate::{
    Color, ComponentPalette, DrawCommand, Element, FontFamily, Key, Markdown, Point, Size,
    TextSpan, UiAction, UiEvent, UiTree,
};

const WINDOW: Size = Size::new(400.0, 600.0);

const ANSWER: &str = "First paragraph with `code` here.\n\n\
## Heading\n\n\
- one\n\
- two [link](https://example.com)\n\n\
```rust\nlet a = 1;\nlet b = 2;\n```\n\n\
Last words.";

fn answer() -> Element {
    Markdown::new(ANSWER, ComponentPalette::neutral_dark())
        .width(WINDOW.width)
        .code_label(false)
        .link_action_prefix("link:")
        .render()
        .element
}

fn tree(root: Element) -> UiTree {
    let mut tree = UiTree::new(root);
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 5.0);
    tree.layout(WINDOW);
    tree
}

/// Just inside the start of the block whose text starts with `text`, and
/// just before the end of its last run.
fn spot(tree: &mut UiTree, text: &str) -> (Point, Point) {
    let frame = tree.layout(WINDOW);
    let block = frame
        .text_layouts()
        .into_iter()
        .find(|block| block.source.starts_with(text))
        .unwrap_or_else(|| panic!("no text starts with {text:?}"));
    let (first, last) = (block.runs[0].rect, block.runs[block.runs.len() - 1].rect);
    (
        Point::new(first.x + 1.0, first.y + first.height / 2.0),
        Point::new(last.x + last.width - 0.5, last.y + last.height / 2.0),
    )
}

fn drag(tree: &mut UiTree, from: Point, to: Point) {
    tree.handle_event(UiEvent::PointerDown(from));
    tree.handle_event(UiEvent::PointerMove(to));
    tree.handle_event(UiEvent::PointerUp(to));
    tree.layout(WINDOW);
}

fn ctrl(key: Key) -> UiEvent {
    UiEvent::Key {
        key,
        shift: false,
        ctrl: true,
        alt: false,
    }
}

const WHOLE: &str = "First paragraph with code here.\n\n\
Heading\n\n\
• one\n\
• two link\n\n\
let a = 1;\nlet b = 2;\n\n\
Last words.";

#[test]
fn a_drag_runs_from_one_paragraph_into_the_next_ones() {
    let mut tree = tree(answer().selection_scope());
    let (start, _) = spot(&mut tree, "First");
    let (_, end) = spot(&mut tree, "Last");
    drag(&mut tree, start, end);
    assert_eq!(tree.selected_text().as_deref(), Some(WHOLE));
    // Upwards, from the end of the code to the middle of the first line.
    let (_, code_end) = spot(&mut tree, "let a");
    let (first, _) = spot(&mut tree, "First");
    drag(
        &mut tree,
        code_end,
        Point::new(first.x + 5.0 * 6.0, first.y),
    );
    assert_eq!(
        tree.selected_text().as_deref(),
        Some("paragraph with code here.\n\nHeading\n\n• one\n• two link\n\nlet a = 1;\nlet b = 2;")
    );
}

#[test]
fn every_text_between_the_ends_is_painted_selected() {
    let mut tree = tree(answer().selection_scope());
    let (start, _) = spot(&mut tree, "First");
    let (_, end) = spot(&mut tree, "Last");
    drag(&mut tree, start, end);
    let frame = tree.layout(WINDOW);
    let highlights = frame
        .commands
        .iter()
        .filter(|command| {
            matches!(command, DrawCommand::FillRect { color, .. } if *color == Color::rgba(180, 180, 180, 84))
        })
        .count();
    // A paragraph, the heading, two markers, two items, two code lines and the last paragraph.
    assert!(highlights >= 9, "{highlights} highlights");
}

#[test]
fn a_drag_into_the_gap_between_texts_stops_at_the_nearest() {
    let mut tree = tree(answer().selection_scope());
    let (start, end) = spot(&mut tree, "First");
    let (heading, _) = spot(&mut tree, "Heading");
    // Below the first paragraph, above the heading, nearer the paragraph.
    let gap = Point::new(start.x, (end.y + heading.y) / 2.0 - 4.0);
    drag(&mut tree, start, gap);
    assert_eq!(
        tree.selected_text().as_deref(),
        Some("First paragraph with code here.")
    );
}

#[test]
fn without_a_scope_a_selection_stays_in_its_text() {
    let mut tree = tree(answer());
    let (start, _) = spot(&mut tree, "First");
    let (_, end) = spot(&mut tree, "Last");
    drag(&mut tree, start, end);
    let selected = tree.selected_text().unwrap_or_default();
    assert!(!selected.contains('\n'), "{selected:?}");
    assert!("First paragraph with code here.".starts_with(&selected));
}

#[test]
fn shift_and_a_press_extend_the_selection_to_another_text() {
    let mut tree = tree(answer().selection_scope());
    let (start, _) = spot(&mut tree, "First");
    tree.handle_event(UiEvent::PointerDown(start));
    tree.handle_event(UiEvent::PointerUp(start));
    tree.layout(WINDOW);
    let (_, one_end) = spot(&mut tree, "one");
    tree.handle_event(UiEvent::PointerDownModified {
        position: one_end,
        shift: true,
    });
    tree.handle_event(UiEvent::PointerUp(one_end));
    tree.layout(WINDOW);
    assert_eq!(
        tree.selected_text().as_deref(),
        Some("First paragraph with code here.\n\nHeading\n\n• one")
    );
    // A plain press starts over.
    tree.handle_event(UiEvent::PointerDown(one_end));
    tree.handle_event(UiEvent::PointerUp(one_end));
    tree.layout(WINDOW);
    assert_eq!(tree.selected_text(), None);
}

#[test]
fn ctrl_a_selects_the_scope_and_ctrl_c_copies_it() {
    let root = Element::column(vec![
        Element::text("Outside"),
        answer().selection_scope(),
        Element::text_input("field", "typed", ""),
    ]);
    let mut tree = tree(root);
    // Nothing pressed in the scope: Ctrl+A has nothing to select.
    assert!(tree.handle_event(ctrl(Key::A)).is_empty());
    assert_eq!(tree.selected_text(), None);
    let (point, _) = spot(&mut tree, "Heading");
    tree.handle_event(UiEvent::PointerDown(point));
    tree.handle_event(UiEvent::PointerUp(point));
    tree.layout(WINDOW);
    tree.handle_event(ctrl(Key::A));
    tree.layout(WINDOW);
    assert_eq!(tree.selected_text().as_deref(), Some(WHOLE));
    assert_eq!(
        tree.handle_event(ctrl(Key::C)),
        vec![UiAction::CopyRequested { text: WHOLE.into() }]
    );
    // A focused field keeps Ctrl+A for its own text.
    assert!(tree.focus("field"));
    tree.handle_event(ctrl(Key::A));
    assert_eq!(
        tree.handle_event(ctrl(Key::C)),
        vec![UiAction::CopyRequested {
            text: "typed".into()
        }]
    );
}

#[test]
fn a_link_in_a_scope_is_still_clicked() {
    let mut tree = tree(answer().selection_scope());
    let frame = tree.layout(WINDOW);
    let link = frame
        .regions
        .iter()
        .find(|region| region.id.starts_with("link:"))
        .expect("the link's region")
        .rect;
    let at = Point::new(link.x + 2.0, link.y + link.height / 2.0);
    tree.handle_event(UiEvent::PointerDown(at));
    let actions = tree.handle_event(UiEvent::PointerUp(at));
    assert!(
        matches!(actions.as_slice(), [UiAction::Clicked { id }] if id.starts_with("link:")),
        "{actions:?}"
    );
    assert_eq!(tree.selected_text(), None);
}

#[test]
fn texts_side_by_side_join_with_a_space_and_tight_lines_with_a_break() {
    let span = |text: &str| TextSpan::new(text, Color::WHITE, 14.0, FontFamily::Sans);
    let root = Element::column(vec![
        Element::row(vec![
            Element::formatted_text(vec![span("1.")]).size(Some(20.0), None),
            Element::formatted_text(vec![span("first")]),
        ]),
        Element::text("second"),
    ])
    .selection_scope();
    let mut tree = tree(root);
    let (start, _) = spot(&mut tree, "1.");
    let (_, end) = spot(&mut tree, "second");
    drag(&mut tree, start, end);
    assert_eq!(tree.selected_text().as_deref(), Some("1. first\nsecond"));
}

// Laid out in limn's own faces.
#[cfg(feature = "bundled-fonts")]
#[test]
fn laid_out_text_selects_across_paragraphs_too() {
    let mut tree = UiTree::new(answer().selection_scope());
    tree.set_text_layout(true);
    tree.layout(WINDOW);
    let (start, _) = spot(&mut tree, "First");
    let (_, end) = spot(&mut tree, "Last");
    drag(&mut tree, start, end);
    assert_eq!(tree.selected_text().as_deref(), Some(WHOLE));
}
