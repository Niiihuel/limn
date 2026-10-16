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

fn scaled(factor: f32) -> Element {
    Element::column(vec![
        Element::button("lyric", "a line of a song").animate_scale("lyric", factor),
        Element::text("next"),
    ])
}

fn text_of(frame: &Frame, wanted: &str) -> (Point, f32) {
    frame.commands.iter().find_map(|command| match command {
        DrawCommand::Text { position, text, font_size, .. } if text == wanted => Some((*position, *font_size)),
        _ => None,
    }).unwrap()
}

#[test]
fn an_animated_scale_grows_the_paint_on_a_spring_and_leaves_layout_alone() {
    let size = Size::new(300.0, 80.0);
    let start = Instant::now();
    let mut tree = UiTree::new(scaled(1.0));
    let rest = tree.layout_at(size, start);
    let (line, font) = text_of(&rest, "a line of a song");
    let region = rest.regions[0].rect;
    tree.set_root(scaled(1.25));
    tree.layout_at(size, start);
    let midway = text_of(&tree.layout_at(size, start + Duration::from_millis(40)), "a line of a song").1;
    assert!(midway > font && midway < font * 1.25, "on its way: {midway}");
    let settled = tree.layout_at(size, start + Duration::from_secs(2));
    let (grown, grown_font) = text_of(&settled, "a line of a song");
    assert!((grown_font - font * 1.25).abs() < 1e-3);
    // From the middle of the left edge: the start stays, the text grows rightwards.
    assert!(grown.x >= line.x && (grown.x - line.x) < (line.x - region.x) * 0.3 + 0.01);
    // The neighbour, the hit target and the line break are those of factor one.
    assert_eq!(settled.regions[0].rect, region);
    assert_eq!(text_of(&settled, "next"), text_of(&rest, "next"));

    // A first sight rests on its factor; reduced motion changes at once.
    let mut first = UiTree::new(scaled(1.25));
    assert!((text_of(&first.layout_at(size, start), "a line of a song").1 - font * 1.25).abs() < 1e-3);
    first.set_reduced_motion(true);
    first.set_root(scaled(1.0));
    assert_eq!(text_of(&first.layout_at(size, start), "a line of a song").1, font);
    // Bad factors are ignored.
    for bad in [f32::NAN, 0.0, -1.0, f32::INFINITY] {
        assert!(Element::text("x").animate_scale("x", bad).kept.is_none(), "{bad}");
    }
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
