//! Context menus drawn by a host's view: the tree paints what the view builds and still
//! drives the menu by its row and scroll IDs.

use super::*;
use crate::{Component, ContextMenu, ContextMenuEntry, ContextMenuItem, ContextMenuView, ContextMenuViewFn};

const FILL: Color = Color::rgb(250, 240, 200);
const LIT: Color = Color::rgb(200, 30, 30);
const HELD: Color = Color::rgb(30, 200, 30);
const ROW: f32 = 40.0;
const RADIUS: f32 = 2.0;
const SIZE: Size = Size::new(400.0, 300.0);

/// A look of a host's own, from plain elements: taller rows, square corners, its own colors.
fn plain(fill: Color) -> ContextMenuViewFn {
    ContextMenuViewFn::new(move |view| {
        // A touch screen's menu of text is drawn narrower, to tell the two apart.
        let width = if view.touch.is_some() { 120.0 } else { view.width };
        let rows: Vec<_> = view.entries.iter().enumerate().map(|(index, entry)| match entry {
            ContextMenuEntry::Item(item) if item.enabled => Element::clickable(ContextMenuView::row_id(index), Element::text(&item.label))
                .action_menu_item()
                .background(fill)
                .hover_background(LIT)
                .pressed_background(HELD)
                .radius(RADIUS)
                .size(Some(width), Some(ROW)),
            ContextMenuEntry::Item(item) => Element::text(&item.label).size(Some(width), Some(ROW)),
            ContextMenuEntry::Separator => Element::spacer().size(Some(width), Some(4.0)),
        }).collect();
        let natural: f32 = view.entries.iter().map(|entry| if matches!(entry, ContextMenuEntry::Separator) { 4.0 } else { ROW }).sum();
        let height = natural.min(view.max_height);
        (Element::scroll(ContextMenuView::scroll_id(), Element::column(rows)).size(Some(width), Some(height)), Size::new(width, height))
    })
}

fn item(id: &str) -> ContextMenuEntry {
    ContextMenuEntry::Item(ContextMenuItem::new(id, id.to_uppercase()))
}

/// Enabled, separator, disabled, enabled.
fn mixed() -> Vec<ContextMenuEntry> {
    vec![item("a"), ContextMenuEntry::Separator, ContextMenuEntry::Item(ContextMenuItem::new("b", "B").disabled()), item("c")]
}

fn menu(entries: Vec<ContextMenuEntry>, view: Option<ContextMenuViewFn>) -> Element {
    let menu = ContextMenu::new("owner", Element::button("owner", "Owner").size(Some(180.0), Some(32.0)), entries, ComponentPalette::neutral_dark());
    match view {
        Some(view) => menu.view(view).render(),
        None => menu.render(),
    }
}

fn tree_of(root: Element) -> UiTree {
    let mut tree = UiTree::new(root);
    tree.set_reduced_motion(true);
    tree.layout(SIZE);
    tree
}

fn by_keyboard(tree: &mut UiTree) -> Frame {
    tree.focus("owner");
    tree.handle_event(UiEvent::ContextMenuKeyboard);
    tree.layout(SIZE)
}

fn rect_of(frame: &Frame, id: &str) -> Option<Rect> {
    frame.regions.iter().find(|region| region.id == id).map(|region| region.rect)
}

fn row(frame: &Frame, index: usize) -> Rect {
    rect_of(frame, &ContextMenuView::row_id(index)).unwrap_or_else(|| panic!("no row {index}"))
}

fn middle(rect: Rect) -> Point {
    Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
}

fn filled(frame: &Frame, color: Color) -> Vec<(Rect, f32)> {
    frame.commands.iter().filter_map(|command| match command {
        DrawCommand::FillRect { rect, color: fill, radius, .. } if *fill == color => Some((*rect, *radius)),
        _ => None,
    }).collect()
}

fn key(key: Key) -> UiEvent {
    UiEvent::Key { key, ctrl: false, shift: false, alt: false }
}

/// Opened by the pointer, which then leaves it: no row is lit.
fn open_at(tree: &mut UiTree, position: Point) -> Frame {
    tree.handle_event(UiEvent::ContextMenu { position });
    tree.handle_event(UiEvent::PointerMove(Point::new(SIZE.width - 4.0, SIZE.height - 4.0)));
    assert!(tree.context_menu_open());
    tree.layout(SIZE)
}

fn click(tree: &mut UiTree, point: Point) -> Vec<UiAction> {
    let mut actions = tree.handle_event(UiEvent::PointerMove(point));
    actions.extend(tree.handle_event(UiEvent::PointerDown(point)));
    actions.extend(tree.handle_event(UiEvent::PointerUp(point)));
    actions
}

fn clicked(id: &str) -> Vec<UiAction> {
    vec![UiAction::Clicked { id: id.into() }]
}

#[test]
fn a_menu_drawn_by_its_own_view_is_what_is_painted() {
    let mut tree = tree_of(menu(mixed(), Some(plain(FILL))));
    let frame = open_at(&mut tree, Point::new(20.0, 16.0));
    let menu = rect_of(&frame, "__context_menu:surface").unwrap();
    // Placed at the press, at the width the tree gave and the height the view said.
    assert_eq!(menu, Rect::new(20.0, 16.0, crate::context_menu::WIDTH, ROW * 3.0 + 4.0));
    let (a, c) = (row(&frame, 0), row(&frame, 3));
    assert_eq!((a.height, c.height), (ROW, ROW));
    assert_eq!(filled(&frame, FILL), vec![(a, RADIUS), (c, RADIUS)], "the view's rows, nothing of limn's");
    assert!(rect_of(&frame, &ContextMenuView::row_id(2)).is_none(), "a disabled row is no target");
    let palette = ComponentPalette::neutral_dark();
    assert!(filled(&frame, palette.surface).is_empty(), "limn's surface is not drawn");
}

#[test]
fn the_keyboard_moves_a_viewed_menus_row_past_what_cannot_be_chosen_and_enter_reports_it() {
    let mut tree = tree_of(menu(mixed(), Some(plain(FILL))));
    let frame = by_keyboard(&mut tree);
    assert_eq!(filled(&frame, LIT), vec![(row(&frame, 0), RADIUS)], "the first row is the keyboard's");
    // Down skips the separator and the disabled row.
    tree.handle_event(key(Key::ArrowDown));
    let frame = tree.layout(SIZE);
    assert_eq!(filled(&frame, LIT), vec![(row(&frame, 3), RADIUS)]);
    // And wraps round to the first.
    tree.handle_event(key(Key::ArrowDown));
    let frame = tree.layout(SIZE);
    assert_eq!(filled(&frame, LIT), vec![(row(&frame, 0), RADIUS)]);
    tree.handle_event(key(Key::ArrowUp));
    tree.layout(SIZE);
    assert_eq!(tree.handle_event(key(Key::Enter)), clicked("c"));
    assert!(!tree.context_menu_open());
}

#[test]
fn a_click_on_a_viewed_row_reports_it_and_one_on_a_disabled_row_or_a_separator_nothing() {
    let mut tree = tree_of(menu(mixed(), Some(plain(FILL))));
    let frame = open_at(&mut tree, Point::new(20.0, 16.0));
    let (a, c) = (row(&frame, 0), row(&frame, 3));
    let separator = Point::new(a.x + 10.0, a.y + a.height + 2.0);
    let disabled = Point::new(a.x + 10.0, a.y + a.height + 4.0 + ROW / 2.0);
    for point in [separator, disabled] {
        assert!(click(&mut tree, point).is_empty(), "nothing is chosen at {point:?}");
        // As in limn's own look, the press lands on the rows' scroll area, which closes it.
        assert!(!tree.context_menu_open());
        tree.layout(SIZE);
        open_at(&mut tree, Point::new(20.0, 16.0));
    }
    // Held, the row shows the view's pressed color.
    tree.handle_event(UiEvent::PointerMove(middle(c)));
    tree.handle_event(UiEvent::PointerDown(middle(c)));
    assert_eq!(filled(&tree.layout(SIZE), HELD), vec![(c, RADIUS)]);
    assert_eq!(tree.handle_event(UiEvent::PointerUp(middle(c))), clicked("c"));
    assert!(!tree.context_menu_open());
}

#[test]
fn a_viewed_menu_taller_than_the_window_scrolls_to_its_last_row() {
    let entries = (0..30).map(|index| item(&format!("a{index}"))).collect();
    let mut tree = tree_of(menu(entries, Some(plain(FILL))));
    let frame = by_keyboard(&mut tree);
    assert_eq!(rect_of(&frame, "__context_menu:surface").unwrap().height, SIZE.height - 16.0);
    tree.handle_event(key(Key::End));
    let frame = tree.layout(SIZE);
    let menu = rect_of(&frame, "__context_menu:surface").unwrap();
    let last = row(&frame, 29);
    assert!(last.y >= menu.y && last.y + last.height <= menu.y + menu.height + 0.01, "End brings the last row into view: {last:?} in {menu:?}");
    assert_eq!(tree.handle_event(key(Key::Enter)), clicked("a29"));
    tree.layout(SIZE);
    // Opened again it starts at its top, and the wheel reaches the end.
    let frame = by_keyboard(&mut tree);
    assert_eq!(tree.scroll_offset(ContextMenuView::scroll_id()), 0.0);
    let menu = rect_of(&frame, "__context_menu:surface").unwrap();
    tree.handle_event(UiEvent::ScrollDirect { position: Point::new(menu.x + 20.0, menu.y + 20.0), delta_y: -100_000.0 });
    let frame = tree.layout(SIZE);
    assert!(tree.scroll_offset(ContextMenuView::scroll_id()) > 0.0);
    assert_eq!(click(&mut tree, middle(row(&frame, 29))), clicked("a29"));
}

#[test]
fn escape_and_a_press_outside_dismiss_a_viewed_menu() {
    let mut tree = tree_of(menu(mixed(), Some(plain(FILL))));
    by_keyboard(&mut tree);
    assert!(tree.context_menu_open());
    assert!(tree.handle_event(key(Key::Escape)).is_empty());
    assert!(!tree.context_menu_open());
    tree.layout(SIZE);
    tree.handle_event(UiEvent::ContextMenu { position: Point::new(20.0, 16.0) });
    tree.layout(SIZE);
    let outside = Point::new(SIZE.width - 4.0, SIZE.height - 4.0);
    assert!(tree.handle_event(UiEvent::PointerDown(outside)).is_empty());
    assert!(!tree.context_menu_open());
    assert!(tree.handle_event(UiEvent::PointerUp(outside)).is_empty());
}

#[test]
fn a_fields_menu_is_drawn_by_the_trees_view_on_a_pointer_and_on_a_touch_screen() {
    let page = Element::column(vec![
        Element::spacer().size(None, Some(150.0)),
        Element::text_input("draft", "hola mundo", "").size(Some(260.0), Some(32.0)),
    ]);
    let mut tree = tree_of(page);
    tree.set_context_menu_view(Some(plain(FILL)));
    tree.focus("draft");
    tree.handle_event(UiEvent::Key { key: Key::A, ctrl: true, shift: false, alt: false });
    let frame = open_at(&mut tree, Point::new(40.0, 165.0));
    // Cut, Copy, Paste, a separator, Select All: the tree's entries in the view's rows.
    let copy = row(&frame, 1);
    assert_eq!(copy.height, ROW);
    assert_eq!(filled(&frame, FILL).len(), 4);
    assert_eq!(click(&mut tree, middle(copy)), vec![UiAction::CopyRequested { text: "hola mundo".into() }]);
    // On a touch screen the view is told so, and the tree stands its menu over the field.
    tree.set_touch_text_menu(Some(crate::GlassMaterial::new(Color::rgba(40, 40, 42, 200))));
    tree.layout(SIZE);
    let frame = open_at(&mut tree, Point::new(40.0, 165.0));
    let menu = rect_of(&frame, "__context_menu:surface").unwrap();
    assert_eq!(menu.width, 120.0);
    assert!(menu.y + menu.height <= 150.0 - EDIT_BAR_GAP + 0.01, "over the field: {menu:?}");
    assert!(!frame.commands.iter().any(|command| matches!(command, DrawCommand::Glass { .. })), "no edit bar of limn's");
}

#[test]
fn a_menus_own_view_wins_over_the_trees() {
    const TREES: Color = Color::rgb(10, 20, 250);
    let owner = |id: &str, view: Option<ContextMenuViewFn>| {
        let menu = ContextMenu::new(id, Element::button(id, id).size(Some(180.0), Some(32.0)), vec![item("a")], ComponentPalette::neutral_dark());
        match view {
            Some(view) => menu.view(view).render(),
            None => menu.render(),
        }
    };
    let page = Element::column(vec![
        owner("own", Some(plain(FILL))),
        owner("bare", None),
        Element::text_input("draft", "hola", "").size(Some(260.0), Some(32.0)),
    ]);
    let mut tree = tree_of(page);
    tree.set_context_menu_view(Some(plain(TREES)));
    // Its own view; one without takes the tree's, as a field's does.
    for (position, fill) in [(Point::new(20.0, 16.0), FILL), (Point::new(20.0, 48.0), TREES), (Point::new(20.0, 80.0), TREES)] {
        let frame = open_at(&mut tree, position);
        assert!(!filled(&frame, fill).is_empty(), "at {position:?}");
        assert!(filled(&frame, if fill == FILL { TREES } else { FILL }).is_empty());
        tree.handle_event(key(Key::Escape));
        tree.layout(SIZE);
    }
}

#[test]
fn default_element_draws_what_the_tree_draws_with_no_view() {
    let wrapped = ContextMenuViewFn::new(|view| view.default_element());
    let entries = || {
        let mut entries = mixed();
        entries.push(ContextMenuEntry::Item(ContextMenuItem::new("d", "Duplicate").shortcut("Ctrl+D").icon(crate::Icon::File)));
        entries
    };
    let field = || Element::column(vec![
        Element::spacer().size(None, Some(100.0)),
        Element::text_input("draft", "hola mundo", "").size(Some(260.0), Some(32.0)),
    ]);
    let now = Instant::now();
    // A host's menu, by the keyboard (no press); a field's, by the pointer and by a finger.
    let at = Some(Point::new(40.0, 115.0));
    for (root, touch, press) in [(menu(entries(), None), false, None), (field(), false, at), (field(), true, at)] {
        let frame = |view: Option<ContextMenuViewFn>| {
            let mut tree = UiTree::new(root.clone());
            tree.set_reduced_motion(true);
            tree.set_context_menu_view(view);
            if touch {
                tree.set_touch_text_menu(Some(crate::GlassMaterial::new(Color::rgba(40, 40, 42, 200))));
            }
            tree.layout_at(SIZE, now);
            match press {
                Some(position) => {
                    tree.focus("draft");
                    tree.handle_event_at(UiEvent::ContextMenu { position }, now);
                }
                None => {
                    tree.focus("owner");
                    tree.handle_event_at(UiEvent::ContextMenuKeyboard, now);
                }
            }
            assert!(tree.context_menu_open());
            tree.layout_at(SIZE, now)
        };
        let (limns, wrapping) = (frame(None), frame(Some(wrapped.clone())));
        assert!(rect_of(&limns, "__context_menu:surface").is_some());
        assert_eq!(limns.commands, wrapping.commands);
        assert_eq!(limns.regions, wrapping.regions);
        assert_eq!(limns.access, wrapping.access);
    }
}
