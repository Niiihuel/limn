use super::*;
use crate::{Color, DrawCommand, Frame, HitKind, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

const WINDOW: Size = Size::new(480.0, 400.0);

fn palette() -> ComponentPalette {
    ComponentPalette::neutral_dark()
}

fn items(count: usize) -> Vec<SideRailItem> {
    (0..count)
        .map(|index| SideRailItem::new(format!("item-{index}"), format!("Playlist {index}")).subtitle(format!("Playlist · {index} songs")))
        .collect()
}

/// The rail at the window's left, as tall as the window.
fn tree(rail: SideRail) -> UiTree {
    let mut tree = UiTree::new(Element::row(vec![rail.render(), Element::spacer().flex(1.0)]));
    tree.set_reduced_motion(true);
    tree.set_text_measurer(|text, _size, _family| text.chars().count() as f32 * 7.0);
    tree
}

fn full(count: usize) -> SideRail {
    SideRail::new("rail", items(count), palette()).expand("Open Your Library").create("Create a playlist")
}

fn region(frame: &Frame, id: &str) -> Rect {
    frame
        .regions
        .iter()
        .find(|region| region.id == id && region.kind == HitKind::Button)
        .unwrap_or_else(|| panic!("{id} is a button"))
        .rect
}

fn middle(rect: Rect) -> Point {
    Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

fn texts(frame: &Frame) -> Vec<(&str, Point, Color)> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, position, color, .. } => Some((text.as_str(), *position, *color)),
            _ => None,
        })
        .collect()
}

fn key(key: Key) -> UiEvent {
    UiEvent::Key { key, shift: false, ctrl: false }
}

#[test]
fn the_rail_is_72_wide_with_48_pictures_inset_8_in_items_4_from_its_edges() {
    let frame = tree(full(3)).layout(WINDOW);
    assert_eq!(SideRail::WIDTH, 72.0);
    // The surface runs the window's height at the rail's width.
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { rect, color, .. } if *rect == Rect::new(0.0, 0.0, 72.0, WINDOW.height) && *color == palette().surface
    )));
    // The head: the unfolding button is 40 square and centered, its icon 24.
    let expand = region(&frame, "rail:expand");
    assert_eq!((expand.x, expand.width, expand.height), (16.0, 40.0, 40.0));
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::Icon { rect, icon: Icon::Library, .. } if rect.width == 24.0 && middle(*rect) == middle(expand)
    )));
    let create = region(&frame, "rail:create");
    assert!(create.y >= expand.y + expand.height && middle(create).x == 36.0);
    let first = region(&frame, "item-0");
    let second = region(&frame, "item-1");
    assert_eq!((first.x, first.width, first.height), (4.0, 64.0, 64.0));
    assert!(first.y > create.y + create.height);
    assert_eq!(second.y, first.y + 64.0);
    // Without a picture, the placeholder fills the 48 px square with the control radius.
    let placeholder = Rect::new(12.0, first.y + 8.0, 48.0, 48.0);
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { rect, color, radius }
            if *rect == placeholder && *color == palette().selected && *radius == crate::tokens().radius.control
    )));
    // No text is drawn, but every item is a button named by its title.
    assert!(texts(&frame).is_empty());
    let node = frame.access.iter().find(|node| node.id == "item-1").unwrap();
    assert_eq!((node.role, node.name.as_str(), node.description.as_deref()), (Role::Button, "Playlist 1", Some("Playlist · 1 songs")));
}

#[test]
fn a_picture_fills_its_square_and_a_person_is_round() {
    let picture = || {
        let image = crate::RasterImage::new("a".repeat(64), crate::RasterImageFormat::Png, 4, std::sync::Arc::from("AAAA"), "A cover").unwrap();
        Element::raster_image(image)
    };
    for (round, radius) in [(false, crate::tokens().radius.control), (true, 24.0)] {
        let item = SideRailItem::new("item", "Someone").picture(picture()).round(round);
        let frame = tree(SideRail::new("rail", vec![item], palette())).layout(WINDOW);
        let painted = frame.commands.iter().find_map(|command| match command {
            DrawCommand::RasterImage { rect, radius, .. } => Some((*rect, *radius)),
            _ => None,
        });
        // Without a head, the items start at the margin.
        assert_eq!(painted, Some((Rect::new(12.0, 12.0, 48.0, 48.0), radius)));
    }
}

#[test]
fn a_hint_opens_at_once_to_the_right_with_the_title_and_a_muted_second_line() {
    let mut tree = tree(full(3));
    let item = region(&tree.layout(WINDOW), "item-1");
    tree.handle_event(UiEvent::PointerMove(middle(item)));
    // The same instant: no wait.
    let frame = tree.layout(WINDOW);
    let lines = texts(&frame);
    let style = palette().tooltip_style();
    let title = lines.iter().find(|line| line.0 == "Playlist 1").expect("the title shows at once");
    let second = lines.iter().find(|line| line.0 == "Playlist · 1 songs").expect("so does the second line");
    assert!(title.1.x >= item.x + item.width && second.1.x >= item.x + item.width, "{lines:?}");
    assert!(second.1.y > title.1.y);
    assert_eq!((title.2, second.2), (style.text, style.muted));
    // The fill under the pointer is the palette's hover.
    assert!(frame.commands.iter().any(|command| matches!(
        command,
        DrawCommand::FillRect { rect, color, .. } if *rect == item && *color == palette().hover
    )));
    // Moving on to the next item moves the hint with it, again at once.
    tree.handle_event(UiEvent::PointerMove(middle(region(&frame, "item-2"))));
    let frame = tree.layout(WINDOW);
    let lines = texts(&frame).into_iter().map(|line| line.0).collect::<Vec<_>>();
    assert!(lines.contains(&"Playlist 2") && !lines.contains(&"Playlist 1"), "{lines:?}");
}

#[test]
fn items_and_head_buttons_report_their_ids_by_pointer_and_by_keyboard() {
    for id in ["item-0", "item-2", "rail:expand", "rail:create"] {
        let mut tree = tree(full(3));
        let point = middle(region(&tree.layout(WINDOW), id));
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), vec![UiAction::Clicked { id: id.into() }]);
        assert!(tree.focus(id));
        for pressed in [Key::Enter, Key::Space] {
            assert_eq!(tree.handle_event(key(pressed)), vec![UiAction::Clicked { id: id.into() }], "{id} {pressed:?}");
        }
    }
}

#[test]
fn the_current_item_keeps_the_selected_fill_and_the_playing_one_a_loudspeaker() {
    let rail = full(3).current("item-1").playing("item-2");
    let frame = tree(rail).layout(WINDOW);
    let current = region(&frame, "item-1");
    let fills = |rect: Rect| {
        frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect: fill, color, .. } if *fill == rect && *color == palette().selected
        ))
    };
    assert!(fills(current) && !fills(region(&frame, "item-0")));
    let selected = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap().selected;
    assert_eq!((selected("item-0"), selected("item-1")), (Some(false), Some(true)));
    // The mark sits in the bottom right corner of the playing item's picture, and only there.
    let playing = region(&frame, "item-2");
    let marks: Vec<Rect> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Icon { rect, icon: Icon::Volume, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    let [mark] = marks.as_slice() else { panic!("one loudspeaker, got {marks:?}") };
    let corner = Point::new(playing.x + 8.0 + 48.0, playing.y + 8.0 + 48.0);
    assert_eq!(middle(*mark), Point::new(corner.x - 10.0, corner.y - 10.0));
    // Without a current page no item is said to be selected or not.
    let frame = tree(full(2)).layout(WINDOW);
    assert_eq!(frame.access.iter().find(|node| node.id == "item-0").unwrap().selected, None);
}

#[test]
fn a_long_rail_scrolls_under_its_head() {
    let mut tree = tree(full(30));
    let frame = tree.layout(WINDOW);
    let head = region(&frame, "rail:expand");
    let before = region(&frame, "item-0");
    tree.handle_event(UiEvent::ScrollDirect { position: middle(before), delta_y: -200.0 });
    let frame = tree.layout(WINDOW);
    assert_eq!(tree.scroll_offset("rail:items"), 200.0);
    // The items moved up; the head stayed.
    assert_eq!(region(&frame, "rail:expand"), head);
    let moved = frame.regions.iter().find(|region| region.id == "item-0").map(|region| region.rect);
    assert!(moved.is_none_or(|rect| rect.y < before.y), "{moved:?}");
    let after = region(&frame, "item-5");
    assert_eq!(after.y, before.y + 5.0 * 64.0 - 200.0);
    // The last one can be reached.
    tree.handle_event(UiEvent::ScrollDirect { position: middle(after), delta_y: -10_000.0 });
    let frame = tree.layout(WINDOW);
    let last = region(&frame, "item-29");
    assert!(last.y + last.height <= WINDOW.height);
}
