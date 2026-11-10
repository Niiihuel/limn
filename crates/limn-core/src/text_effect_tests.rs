use super::*;

const INK: Color = Color::rgb(255, 255, 255);
const EDGE: Color = Color::rgb(200, 40, 120);

/// Text limn lays out itself needs its faces: without the bundled ones only
/// measured text is checked.
const LAYOUTS: &[bool] = if cfg!(feature = "bundled-fonts") { &[false, true] } else { &[false] };

fn tree(element: Element, laid_out: bool) -> Frame {
    let mut tree = UiTree::new(Element::column(vec![element]));
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
    tree.set_text_layout(laid_out);
    tree.layout(Size::new(400.0, 100.0))
}

fn effects(frame: &Frame) -> Vec<(TextEffect, Vec<DrawCommand>)> {
    frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::TextEffect { effect, runs } => Some((*effect, runs.clone())),
            _ => None,
        })
        .collect()
}

fn without_effects(frame: &Frame) -> Vec<DrawCommand> {
    frame
        .commands
        .iter()
        .filter(|command| !matches!(command, DrawCommand::TextEffect { .. }))
        .cloned()
        .collect()
}

fn styled(element: Element) -> Element {
    element
        .foreground(INK)
        .text_shadow(Point::new(0.0, 0.0), 6.0, EDGE)
        .text_stroke(1.6, EDGE)
}

#[test]
fn a_stroke_and_a_shadow_paint_under_the_text_and_change_nothing_else() {
    for &laid_out in LAYOUTS {
        for element in [Element::truncated_text("Neón"), Element::text("Neón")] {
            let plain = tree(element.clone().foreground(INK), laid_out);
            let frame = tree(styled(element), laid_out);
            assert_eq!(
                without_effects(&frame),
                plain.commands,
                "laid out: {laid_out}"
            );
            assert_eq!(frame.regions, plain.regions);
            let found = effects(&frame);
            assert_eq!(found.len(), 2);
            // The shadow first, under the stroke, both before the text.
            assert_eq!(
                found[0].0,
                TextEffect::Shadow {
                    offset: Point::new(0.0, 0.0),
                    blur: 6.0,
                    color: EDGE
                }
            );
            assert_eq!(
                found[1].0,
                TextEffect::Stroke {
                    width: 1.6,
                    color: EDGE
                }
            );
            let first_text = frame
                .commands
                .iter()
                .position(|command| {
                    matches!(command, DrawCommand::Text { .. } | DrawCommand::Glyphs(_))
                })
                .unwrap();
            let last_effect = frame
                .commands
                .iter()
                .rposition(|command| matches!(command, DrawCommand::TextEffect { .. }))
                .unwrap();
            assert!(last_effect < first_text);
            // The effect is of the text as it is painted.
            let painted: Vec<DrawCommand> = plain
                .commands
                .iter()
                .filter(|command| {
                    matches!(command, DrawCommand::Text { .. } | DrawCommand::Glyphs(_))
                })
                .cloned()
                .collect();
            assert_eq!(found[1].1, painted);
        }
    }
}

#[test]
fn effects_go_once_under_a_gradient_and_take_the_foregrounds_alpha() {
    let stops = vec![Color::rgb(255, 0, 0), Color::rgb(0, 0, 255)];
    for &laid_out in LAYOUTS {
        for element in [
            Element::truncated_text("Degradado"),
            Element::text("Degradado"),
        ] {
            let frame = tree(
                styled(element)
                    .foreground(Color::rgba(255, 255, 255, 128))
                    .foreground_gradient(stops.clone()),
                laid_out,
            );
            let found = effects(&frame);
            assert_eq!(found.len(), 2, "laid out: {laid_out}");
            // Not clipped to a band: before the first of them.
            let first_clip = frame
                .commands
                .iter()
                .position(|command| matches!(command, DrawCommand::PushClip(_)))
                .unwrap();
            let last_effect = frame
                .commands
                .iter()
                .rposition(|command| matches!(command, DrawCommand::TextEffect { .. }))
                .unwrap();
            assert!(last_effect < first_clip);
            for (effect, _) in found {
                let (TextEffect::Stroke { color, .. } | TextEffect::Shadow { color, .. }) = effect;
                assert_eq!(color, Color::rgba(EDGE.r, EDGE.g, EDGE.b, 128));
            }
            // The gradient paints its bands, laid out or not.
            let bands = frame
                .commands
                .iter()
                .filter(|command| matches!(command, DrawCommand::PushClip(_)))
                .count();
            assert_eq!(bands, 24, "laid out: {laid_out}");
        }
    }
}

#[test]
fn a_stroke_of_no_width_paints_none_and_numbers_are_kept_in_range() {
    let frame = tree(
        Element::text("x")
            .text_stroke(0.0, EDGE)
            .text_stroke(f32::NAN, EDGE),
        false,
    );
    assert!(effects(&frame).is_empty());
    let kept = Element::text("x").text_stroke(40.0, EDGE).text_shadow(
        Point::new(f32::INFINITY, 2.0),
        100.0,
        EDGE,
    );
    let found = effects(&tree(kept, false));
    assert_eq!(
        found[0].0,
        TextEffect::Shadow {
            offset: Point::new(0.0, 2.0),
            blur: 32.0,
            color: EDGE
        }
    );
    assert_eq!(
        found[1].0,
        TextEffect::Stroke {
            width: 16.0,
            color: EDGE
        }
    );
}

#[test]
fn an_effect_moves_scales_and_fades_with_its_text() {
    let run = DrawCommand::Text {
        position: Point::new(10.0, 20.0),
        text: "a".into(),
        color: INK,
        font_size: 14.0,
        font_family: FontFamily::Sans,
    };
    let effect = DrawCommand::TextEffect {
        effect: TextEffect::Shadow {
            offset: Point::new(2.0, 2.0),
            blur: 4.0,
            color: EDGE,
        },
        runs: vec![run.clone()],
    };
    let DrawCommand::TextEffect {
        effect: shadow,
        runs,
    } = effect.moved(5.0, 0.0, 0.5)
    else {
        unreachable!()
    };
    assert_eq!(runs, vec![run.moved(5.0, 0.0, 0.5)]);
    assert!(matches!(shadow, TextEffect::Shadow { color, .. } if color.a == 128));
    let mut scaled = effect.clone();
    animate_popover_command(&mut scaled, 1.0, 0.0, 2.0, Point::new(0.0, 0.0));
    let DrawCommand::TextEffect {
        effect: TextEffect::Shadow { offset, blur, .. },
        runs,
    } = scaled
    else {
        unreachable!()
    };
    assert_eq!((offset, blur), (Point::new(4.0, 4.0), 8.0));
    assert!(matches!(runs[0], DrawCommand::Text { font_size, .. } if font_size == 28.0));
    // Its bounds take in where the shadow reaches.
    let bounds = effect.bounds().unwrap();
    let text = run.bounds().unwrap();
    assert!(
        bounds.x <= text.x + 2.0 - 4.0
            && bounds.y + bounds.height >= text.y + text.height + 2.0 + 4.0
    );
}
