use super::*;
use std::time::Duration;

fn runs(frame: &Frame) -> Vec<(Point, u8)> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { position, color, .. } => Some((*position, color.a)),
            _ => None,
        })
        .collect()
}

fn line(radius: f32) -> Element {
    Element::column(vec![Element::text("a line of a song").soft_focus("line", radius)])
}

#[test]
fn a_soft_focus_spreads_a_run_round_its_place_and_none_leaves_it_as_it_was() {
    let size = Size::new(300.0, 60.0);
    let mut tree = UiTree::new(line(0.0));
    tree.set_reduced_motion(true);
    let sharp = runs(&tree.layout(size));
    assert_eq!(sharp.len(), 1);

    tree.set_root(line(2.0));
    let soft = runs(&tree.layout(size));
    assert_eq!(soft.len(), 13, "thirteen copies stand for the one run");
    let (at, ink) = sharp[0];
    let apart: Vec<f32> = soft.iter().map(|(position, _)| ((position.x - at.x).powi(2) + (position.y - at.y).powi(2)).sqrt()).collect();
    assert!(apart.iter().all(|apart| *apart < 2.01), "none past the radius: {apart:?}");
    assert_eq!(apart.iter().filter(|apart| (**apart - 2.0).abs() < 0.01).count(), 8, "eight on the rim");
    assert!(soft.iter().all(|(_, alpha)| *alpha < ink / 2), "each faint, so their overlap weighs what the run did");

    for bad in [f32::NAN, f32::INFINITY, -3.0] {
        tree.set_root(line(bad));
        assert_eq!(runs(&tree.layout(size)).len(), 1, "{bad}");
    }
}

#[test]
fn a_soft_focus_springs_to_its_radius() {
    let size = Size::new(300.0, 60.0);
    let mut tree = UiTree::new(line(0.0));
    let start = Instant::now();
    let at = runs(&tree.layout_at(size, start))[0].0;
    tree.set_root(line(4.0));
    tree.layout_at(size, start);
    let reach = |frame: &Frame| runs(frame).iter().map(|(position, _)| position.x - at.x).fold(0.0f32, f32::max);
    let midway = reach(&tree.layout_at(size, start + Duration::from_millis(60)));
    assert!(midway > 0.2 && midway < 3.8, "on its way: {midway}");
    let settled = reach(&tree.layout_at(size, start + Duration::from_secs(2)));
    assert!((settled - 4.0).abs() < 0.01, "{settled}");
}

#[test]
fn center_brings_a_control_to_the_middle_of_its_scroll_area() {
    let rows = |count: usize| {
        Element::scroll(
            "rows",
            Element::column((0..count).map(|index| Element::button(format!("row:{index}"), "Row").size(None, Some(30.0))).collect()),
        )
        .size(None, Some(100.0))
    };
    let size = Size::new(200.0, 100.0);
    let mut tree = UiTree::new(rows(20));
    tree.set_reduced_motion(true);
    tree.layout(size);
    tree.center("row:10");
    tree.layout(size);
    // Row 10 starts 300 down and is 30 tall in a viewport of 100.
    assert_eq!(tree.scroll_offset("rows"), 265.0);
    tree.center("row:0");
    tree.layout(size);
    assert_eq!(tree.scroll_offset("rows"), 0.0, "no further than the content scrolls");
    tree.center("nowhere");
    tree.layout(size);
    assert_eq!(tree.scroll_offset("rows"), 0.0);

    let mut moving = UiTree::new(rows(20));
    let start = Instant::now();
    moving.layout_at(size, start);
    moving.center("row:10");
    moving.layout_at(size, start);
    moving.layout_at(size, start + Duration::from_millis(40));
    let midway = moving.scroll_offset("rows");
    assert!(midway > 0.0 && midway < 265.0, "it travels there: {midway}");
    moving.layout_at(size, start + Duration::from_secs(1));
    assert_eq!(moving.scroll_offset("rows"), 265.0);
}
