//! A measured list: rows of their own heights, built only while in view,
//! with the row at the top of the viewport held still while the rows above
//! it are measured, change or are added, and the end followed when asked.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use limn::{Component, Element, Point, ScrollArea, Size, UiEvent, UiTree};

const VIEW: Size = Size::new(300.0, 200.0);
const ESTIMATE: f32 = 40.0;

/// Row `key` is this tall: 20 to 100, so neighbours differ and none is the
/// estimate.
fn height(key: u64) -> f32 {
    20.0 + (key * 7 % 9) as f32 * 10.0
}

fn row(key: u64) -> Element {
    Element::clickable(
        format!("row:{key}"),
        Element::spacer().size(None, Some(height(key))),
    )
}

/// A list of the rows named by `keys`, in order. `built` counts the rows the
/// list asked for.
fn list(keys: &[u64], built: &Arc<AtomicUsize>, follow: bool) -> Element {
    let (keys, built) = (Arc::new(keys.to_vec()), built.clone());
    let key_of = keys.clone();
    ScrollArea::measured_list(
        "messages",
        keys.len(),
        ESTIMATE,
        move |index| key_of[index],
        move |index| {
            built.fetch_add(1, Ordering::Relaxed);
            row(keys[index])
        },
    )
    .follow_end(follow)
    .render()
    .flex(1.0)
}

fn tree(keys: &[u64], built: &Arc<AtomicUsize>, follow: bool) -> UiTree {
    let mut tree = UiTree::new(Element::column(vec![list(keys, built, follow)]));
    tree.set_reduced_motion(true);
    tree
}

/// How far below the top of the viewport row `key` starts at the last
/// layout, negative once scrolled past. `None` while the row is not built.
fn top(tree: &UiTree, key: u64) -> Option<f32> {
    tree.anchor_offset(&format!("row:{key}"))
}

/// The key of the row at the top of the viewport.
fn first_shown(tree: &UiTree, keys: &[u64]) -> u64 {
    keys[tree.visible_rows("messages").unwrap().start]
}

/// A wheel step: positive shows what is above, negative what is below.
fn scroll(tree: &mut UiTree, delta_y: f32) {
    tree.handle_event(UiEvent::ScrollDirect {
        position: Point::new(VIEW.width / 2.0, VIEW.height / 2.0),
        delta_y,
    });
}

#[test]
fn rows_take_their_own_heights_one_under_another() {
    let built = Arc::new(AtomicUsize::new(0));
    let keys: Vec<u64> = (0..6).collect();
    let mut tree = tree(&keys, &built, false);
    tree.layout(VIEW);
    let mut bottom = 0.0;
    for key in tree.visible_rows("messages").unwrap().map(|index| keys[index]) {
        assert_eq!(top(&tree, key), Some(bottom), "row {key} starts where the one above it ends");
        bottom += height(key);
    }
    assert!(bottom >= VIEW.height, "the rows fill the viewport");
}

#[test]
fn only_the_rows_in_view_are_built() {
    let built = Arc::new(AtomicUsize::new(0));
    let keys: Vec<u64> = (0..10_000).collect();
    let mut tree = tree(&keys, &built, false);
    tree.layout(VIEW);
    let shown = tree.visible_rows("messages").unwrap();
    assert_eq!(shown.start, 0);
    assert!(shown.len() < 12, "{shown:?} of ten thousand rows");
    assert!(built.load(Ordering::Relaxed) < 40, "a layout builds what it shows");
}

#[test]
fn the_end_is_the_last_row_at_its_real_height() {
    let built = Arc::new(AtomicUsize::new(0));
    let keys: Vec<u64> = (0..500).collect();
    let mut tree = tree(&keys, &built, false);
    tree.layout(VIEW);
    tree.scroll_to_end("messages");
    tree.layout(VIEW);
    let last = top(&tree, 499).expect("the last row is in view");
    assert_eq!(last + height(499), VIEW.height, "it ends at the bottom of the viewport");
    assert!(tree.is_scroll_at_end("messages", 0.5));
    assert_eq!(tree.visible_rows("messages").unwrap().end, 500);
}

#[test]
fn rows_added_above_leave_the_rows_in_view_where_they_were() {
    let built = Arc::new(AtomicUsize::new(0));
    let keys: Vec<u64> = (100..200).collect();
    let mut tree = tree(&keys, &built, false);
    tree.layout(VIEW);
    scroll(&mut tree, -1_000.0);
    tree.layout(VIEW);
    let offset = tree.scroll_offset("messages");
    assert!(offset > 900.0, "scrolled down to {offset}");
    let watched = first_shown(&tree, &keys);
    let was = top(&tree, watched).unwrap();

    // An older page arrives, of rows the tree has never measured.
    let older: Vec<u64> = (0..100).chain(keys.iter().copied()).collect();
    tree.set_root(Element::column(vec![list(&older, &built, false)]));
    tree.layout(VIEW);
    assert_eq!(top(&tree, watched), Some(was), "row {watched} did not move");
    assert!(tree.scroll_offset("messages") > offset + 100.0 * 20.0, "the offset took the new rows in");
}

#[test]
fn rows_measured_above_while_scrolling_up_do_not_move_the_view() {
    let built = Arc::new(AtomicUsize::new(0));
    let keys: Vec<u64> = (0..400).collect();
    let mut tree = tree(&keys, &built, false);
    tree.layout(VIEW);
    tree.scroll_to_end("messages");
    tree.layout(VIEW);
    // Every step up reaches rows only estimated so far. The row at the top
    // of the view moves down by the step and by nothing else.
    for _ in 0..30 {
        let watched = first_shown(&tree, &keys);
        let was = top(&tree, watched).unwrap();
        scroll(&mut tree, 50.0);
        tree.layout(VIEW);
        let now = top(&tree, watched).unwrap();
        assert!((now - (was + 50.0)).abs() < 0.01, "row {watched}: {was} then {now}");
    }
}

#[test]
fn following_the_end_shows_each_new_row() {
    let built = Arc::new(AtomicUsize::new(0));
    let mut keys: Vec<u64> = (0..50).collect();
    let mut tree = tree(&keys, &built, true);
    tree.scroll_to_end("messages");
    tree.layout(VIEW);
    for key in 50..60 {
        keys.push(key);
        tree.set_root(Element::column(vec![list(&keys, &built, true)]));
        tree.layout(VIEW);
        let last = top(&tree, key).expect("the new row is in view");
        assert_eq!(last + height(key), VIEW.height);
    }
}

#[test]
fn a_short_list_that_follows_its_end_stays_at_the_top_until_it_overflows() {
    let built = Arc::new(AtomicUsize::new(0));
    let mut keys: Vec<u64> = vec![0];
    let mut tree = tree(&keys, &built, true);
    tree.layout(VIEW);
    assert_eq!(top(&tree, 0), Some(0.0));
    for key in 1..20 {
        keys.push(key);
        tree.set_root(Element::column(vec![list(&keys, &built, true)]));
        tree.layout(VIEW);
    }
    assert!(tree.is_scroll_at_end("messages", 0.5), "it followed once it overflowed");
    assert_eq!(top(&tree, 19).map(|top| top + height(19)), Some(VIEW.height));
}

#[test]
fn scrolled_away_from_the_end_a_new_row_does_not_pull_the_view() {
    let built = Arc::new(AtomicUsize::new(0));
    let mut keys: Vec<u64> = (0..50).collect();
    let mut tree = tree(&keys, &built, true);
    tree.scroll_to_end("messages");
    tree.layout(VIEW);
    scroll(&mut tree, 300.0);
    tree.layout(VIEW);
    let watched = first_shown(&tree, &keys);
    let was = top(&tree, watched);
    keys.push(50);
    tree.set_root(Element::column(vec![list(&keys, &built, true)]));
    tree.layout(VIEW);
    assert_eq!(top(&tree, watched), was);
    assert!(!tree.is_scroll_at_end("messages", 0.5));
}

#[test]
fn a_row_far_down_scrolls_into_view_by_its_index() {
    let built = Arc::new(AtomicUsize::new(0));
    let keys: Vec<u64> = (0..1_000).collect();
    let mut tree = tree(&keys, &built, false);
    tree.layout(VIEW);
    tree.scroll_to_index("messages", 700);
    tree.layout(VIEW);
    let row = top(&tree, 700).expect("row 700 is built");
    assert!(row >= 0.0 && row + height(700) <= VIEW.height, "row 700 at {row} is whole");
}

#[test]
fn a_row_that_grows_keeps_the_rows_above_it_in_place() {
    let built = Arc::new(AtomicUsize::new(0));
    let keys: Vec<u64> = (0..100).collect();
    let grown = Arc::new(AtomicUsize::new(0));
    let make = |grown: &Arc<AtomicUsize>| {
        let (keys, grown) = (keys.clone(), grown.clone());
        let key_of = keys.clone();
        Element::column(vec![Element::measured_list(
            "messages",
            keys.len(),
            ESTIMATE,
            move |index| key_of[index],
            move |index| {
                let extra = if index == 40 { grown.load(Ordering::Relaxed) as f32 } else { 0.0 };
                Element::clickable(
                    format!("row:{}", keys[index]),
                    Element::spacer().size(None, Some(height(keys[index]) + extra)),
                )
            },
        )
        .flex(1.0)])
    };
    let mut tree = UiTree::new(make(&grown));
    tree.set_reduced_motion(true);
    tree.layout(VIEW);
    tree.scroll_to_index("messages", 40);
    tree.layout(VIEW);
    let watched = first_shown(&tree, &keys);
    let was = top(&tree, watched);
    // A reaction lands on the row: it grows by a line.
    grown.store(24, Ordering::Relaxed);
    tree.set_root(make(&grown));
    tree.layout(VIEW);
    assert_eq!(top(&tree, watched), was);
    let _ = built;
}

#[test]
fn an_estimate_that_is_not_a_size_is_one_pixel() {
    for estimate in [f32::NAN, f32::INFINITY, -5.0, 0.0] {
        let keys: Vec<u64> = (0..20).collect();
        let key_of = keys.clone();
        let element = Element::measured_list(
            "messages",
            keys.len(),
            estimate,
            move |index| key_of[index],
            move |index| row(keys[index]),
        );
        let mut tree = UiTree::new(Element::column(vec![element.flex(1.0)]));
        tree.layout(VIEW);
        tree.scroll_to_end("messages");
        tree.layout(VIEW);
        assert_eq!(
            top(&tree, 19).map(|top| top + height(19)),
            Some(VIEW.height),
            "estimate {estimate}: the end is reached"
        );
    }
}

#[test]
fn an_empty_list_lays_out_with_nothing_to_show() {
    let built = Arc::new(AtomicUsize::new(0));
    let mut tree = tree(&[], &built, true);
    tree.scroll_to_end("messages");
    tree.layout(VIEW);
    assert_eq!(tree.visible_rows("messages"), Some(0..0));
    assert_eq!(built.load(Ordering::Relaxed), 0);
}

#[test]
fn tab_walks_past_the_rows_in_view() {
    let built = Arc::new(AtomicUsize::new(0));
    let keys: Vec<u64> = (0..200).collect();
    let mut tree = tree(&keys, &built, false);
    tree.layout(VIEW);
    let shown = tree.visible_rows("messages").unwrap().len();
    for _ in 0..shown + 3 {
        tree.handle_event(UiEvent::Key { key: limn::Key::Tab, shift: false, ctrl: false });
        tree.layout(VIEW);
    }
    let focused = tree.focused_id().expect("a row has focus").to_owned();
    let key: u64 = focused.trim_start_matches("row:").parse().unwrap();
    assert!(key as usize >= shown, "focus reached row {key}, past the {shown} first shown");
    let offset = top(&tree, key).expect("the focused row is built");
    assert!(offset >= 0.0 && offset + height(key) <= VIEW.height, "and is in view at {offset}");
}

#[test]
fn a_smooth_scroll_through_rows_not_yet_measured_keeps_its_pace() {
    use std::time::{Duration, Instant};
    let built = Arc::new(AtomicUsize::new(0));
    let keys: Vec<u64> = (0..400).collect();
    let mut tree = UiTree::new(Element::column(vec![list(&keys, &built, false)]));
    let start = Instant::now();
    tree.layout_at(VIEW, start);
    tree.scroll_to_end("messages");
    tree.layout_at(VIEW, start);
    let watched = first_shown(&tree, &keys);
    let was = top(&tree, watched).unwrap();
    tree.handle_event(UiEvent::Scroll {
        position: Point::new(VIEW.width / 2.0, VIEW.height / 2.0),
        delta_y: 200.0,
    });
    // Rows above come into view and are measured on the way; the row being
    // watched only ever moves down, and lands where the wheel sent it.
    let mut previous = was;
    for step in 1..=40 {
        tree.layout_at(VIEW, start + Duration::from_millis(step * 16));
        let now = top(&tree, watched).expect("the watched row stays built");
        assert!(now >= previous - 0.01, "step {step}: {previous} then {now}");
        previous = now;
    }
    assert!((previous - (was + 200.0)).abs() < 0.01, "{was} + 200 is not {previous}");
}

/// Rows of wrapped text, whose height depends on the width they are given.
fn text_list(count: usize) -> Element {
    ScrollArea::measured_list(
        "messages",
        count,
        ESTIMATE,
        |index| index as u64,
        |index| {
            let words = 3 + index * 37 % 41;
            Element::clickable(
                format!("row:{index}"),
                Element::column(vec![
                    Element::text(format!("Message {index}")),
                    Element::text(vec!["word"; words].join(" ")),
                ])
                .padding(6.0),
            )
        },
    )
    .follow_end(true)
    .render()
    .flex(1.0)
}

#[test]
fn wrapped_rows_that_follow_the_end_end_at_the_bottom() {
    let mut tree = UiTree::new(Element::column(vec![text_list(2_000)]));
    tree.set_reduced_motion(true);
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 7.0);
    tree.layout(VIEW);
    for count in [2_000, 2_001, 2_002] {
        tree.set_root(Element::column(vec![text_list(count)]));
        let frame = tree.layout(VIEW);
        assert!(tree.is_scroll_at_end("messages", 0.5), "{count} rows: at the end");
        let last = format!("row:{}", count - 1);
        let rect = frame.regions.iter().find(|region| region.id == last).unwrap().rect;
        let top = tree.anchor_offset(&last).unwrap();
        assert!((top + rect.height - VIEW.height).abs() < 0.5, "{count} rows: the last ends at {}", top + rect.height);
    }
}
