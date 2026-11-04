use super::*;
use crate::{IconKeyframe, IconMotion, IconTransform};
use std::time::Duration;

const SIZE: Size = Size::new(300.0, 40.0);
const FILL: Color = Color::rgb(200, 30, 30);

fn icon(frame: &Frame) -> (Rect, IconTransform) {
    frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Icon { rect, transform, .. } => Some((*rect, *transform)),
            _ => None,
        })
        .expect("an icon is painted")
}

/// Everything painted but the icons.
fn the_rest(frame: &Frame) -> Vec<DrawCommand> {
    frame.commands.iter().filter(|command| !matches!(command, DrawCommand::Icon { .. })).cloned().collect()
}

fn gear() -> IconMotion {
    IconMotion::new().keyframe(IconKeyframe::at(120)).keyframe(IconKeyframe::at(1070).turn(180.0)).return_ms(200)
}

fn settings(motion: IconMotion) -> Element {
    Element::row(vec![
        Element::clickable("settings", Element::row(vec![Element::icon(Icon::Settings), Element::text("Settings")]))
            .background(FILL)
            .size(Some(120.0), Some(40.0))
            .hover_motion("settings-gear", motion),
        Element::spacer().flex(1.0),
    ])
}

#[test]
fn under_the_pointer_only_the_icons_play_and_hold_and_they_ease_home_after() {
    let mut tree = UiTree::new(settings(gear()));
    let start = Instant::now();
    let at = |ms: u64| start + Duration::from_millis(ms);
    let rest = tree.layout_at(SIZE, at(0));
    let (place, transform) = icon(&rest);
    assert_eq!(transform, IconTransform::IDENTITY);

    tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 20.0)));
    tree.layout_at(SIZE, at(1000));
    let playing = tree.layout_at(SIZE, at(1595));
    let (rect, transform) = icon(&playing);
    assert!((transform.angle - std::f32::consts::FRAC_PI_2).abs() < 1e-3, "half way: {transform:?}");
    assert_eq!(rect, place, "turned where it is");
    assert_eq!(the_rest(&playing), the_rest(&rest), "the fill and the text stay put");
    assert_eq!(playing.regions, rest.regions, "and so do the targets");
    let held = tree.layout_at(SIZE, at(5000));
    assert!((icon(&held).1.angle - std::f32::consts::PI).abs() < 1e-6, "the last keyframe is held");

    tree.handle_event(UiEvent::PointerMove(Point::new(250.0, 20.0)));
    let left = icon(&tree.layout_at(SIZE, at(5000))).1.angle;
    assert!((left - std::f32::consts::PI).abs() < 1e-6, "no jump on leaving");
    let returning = icon(&tree.layout_at(SIZE, at(5100))).1.angle;
    assert!(returning > 0.0 && returning < std::f32::consts::PI, "{returning}");

    tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 20.0)));
    let back = icon(&tree.layout_at(SIZE, at(5100))).1.angle;
    assert!((back - returning).abs() < 1e-6, "coming back plays again from where it is");
    tree.handle_event(UiEvent::PointerMove(Point::new(250.0, 20.0)));
    tree.layout_at(SIZE, at(5200));
    assert_eq!(icon(&tree.layout_at(SIZE, at(5400))).1, IconTransform::IDENTITY, "home");
}

#[test]
fn an_icon_is_stretched_and_moved_and_nothing_moves_under_reduced_motion() {
    let squash = IconMotion::new().keyframe(IconKeyframe::at(100).scale(1.06, 0.88).offset(Point::new(0.0, 2.0)));
    let mut tree = UiTree::new(settings(squash));
    let start = Instant::now();
    let place = icon(&tree.layout_at(SIZE, start)).0;
    tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 20.0)));
    tree.layout_at(SIZE, start);
    let (rect, transform) = icon(&tree.layout_at(SIZE, start + Duration::from_millis(400)));
    assert_eq!(rect, Rect::new(place.x, place.y + 2.0, place.width, place.height));
    assert_eq!(transform, IconTransform { angle: 0.0, scale_x: 1.06, scale_y: 0.88 });

    let mut still = UiTree::new(settings(gear()));
    still.set_reduced_motion(true);
    still.handle_event(UiEvent::PointerMove(Point::new(20.0, 20.0)));
    still.layout(SIZE);
    assert_eq!(icon(&still.layout(SIZE)).1, IconTransform::IDENTITY);
    assert!(!still.is_animating());
}

#[test]
fn a_playing_icon_keeps_the_window_painting() {
    let mut tree = UiTree::new(settings(gear()));
    tree.layout(SIZE);
    assert!(!tree.is_animating());
    tree.handle_event(UiEvent::PointerMove(Point::new(20.0, 20.0)));
    tree.layout(SIZE);
    assert_eq!(tree.animation_repaint_after(), Some(Duration::ZERO));
}

fn fills(frame: &Frame) -> Vec<(Rect, CornerRadii, u8)> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillCornerRect { rect, radii, color } if (color.r, color.g) == (FILL.r, FILL.g) => Some((*rect, *radii, color.a)),
            _ => None,
        })
        .collect()
}

/// The rail's pill: 4 px wide, rounded on its right end, its middle at y 50.
fn pill(height: f32) -> Element {
    let round = CornerRadii { top_left: 0.0, top_right: 4.0, bottom_right: 4.0, bottom_left: 0.0 };
    let bar = Element::panel(Element::spacer()).background(FILL).corner_radii(round).size(Some(4.0), Some(height));
    bar.centered_y().size(Some(4.0), Some(100.0))
}

#[test]
fn a_revealed_pill_grows_about_its_middle_and_keeps_round_ends() {
    let rail = |reduced: bool| {
        let row = Element::row(vec![pill(20.0).reveal_on_hover("pill").grow_from(4.0), Element::spacer().flex(1.0)]).hover_region();
        let mut tree = UiTree::new(Element::column(vec![row]));
        tree.set_reduced_motion(reduced);
        tree
    };
    let mut tree = rail(false);
    let start = Instant::now();
    tree.layout_at(Size::new(200.0, 100.0), start);
    tree.handle_event(UiEvent::PointerMove(Point::new(100.0, 50.0)));
    let [(from, ends, _)] = fills(&tree.layout_at(Size::new(200.0, 100.0), start))[..] else {
        panic!("one pill");
    };
    assert_eq!((from, ends.top_right, ends.bottom_right), (Rect::new(0.0, 48.0, 4.0, 4.0), 2.0, 2.0), "4 px tall, its ends half that");
    let [(rect, radii, alpha)] = fills(&tree.layout_at(Size::new(200.0, 100.0), start + Duration::from_millis(20)))[..] else {
        panic!("one pill");
    };
    assert!(rect.height > 4.0 && rect.height < 20.0, "on its way: {rect:?}");
    assert!((rect.y + rect.height / 2.0 - 50.0).abs() < 1e-3, "about its middle");
    assert_eq!(rect.width, 4.0, "as wide");
    assert!(radii.top_right <= rect.height / 2.0 && radii.top_left == 0.0, "{radii:?}");
    assert!(alpha > 0 && alpha < 255, "and fading in");
    let [(settled, ..)] = fills(&tree.layout_at(Size::new(200.0, 100.0), start + Duration::from_secs(1)))[..] else {
        panic!("one pill");
    };
    assert_eq!(settled, Rect::new(0.0, 40.0, 4.0, 20.0));

    let mut still = rail(true);
    still.handle_event(UiEvent::PointerMove(Point::new(100.0, 50.0)));
    assert_eq!(fills(&still.layout(Size::new(200.0, 100.0)))[0].0, settled, "no growth under reduced motion");
}

#[test]
fn an_entering_pill_grows_from_another_height_and_travels_in_without_the_settle() {
    let size = Size::new(200.0, 100.0);
    let mut tree = UiTree::new(Element::row(vec![pill(20.0)]));
    let start = Instant::now();
    tree.layout_at(size, start);
    tree.set_root(Element::row(vec![pill(40.0).animate_enter("chosen").grow_from(20.0).enter_from(Point::new(-4.0, 0.0))]));
    let [(first, ..)] = fills(&tree.layout_at(size, start))[..] else { panic!("one pill") };
    assert_eq!(first, Rect::new(-4.0, 40.0, 4.0, 20.0), "from 20 px tall, 4 px to the left, and not 4 px up");
    let [(midway, ..)] = fills(&tree.layout_at(size, start + Duration::from_millis(40)))[..] else { panic!("one pill") };
    assert!(midway.x > -4.0 && midway.x < 0.0 && midway.height > 20.0 && midway.height < 40.0, "{midway:?}");
    assert!((midway.y + midway.height / 2.0 - 50.0).abs() < 1e-3);
    let [(settled, ..)] = fills(&tree.layout_at(size, start + Duration::from_secs(1)))[..] else { panic!("one pill") };
    assert_eq!(settled, Rect::new(0.0, 30.0, 4.0, 40.0));

    for bad in [Element::spacer().grow_from(f32::NAN).grow_from(-1.0), Element::spacer().enter_from(Point::new(f32::INFINITY, 0.0))] {
        assert!(bad.kept.is_none_or(|kept| kept.grow_from.is_none() && kept.enter_from.is_none()));
    }
}
