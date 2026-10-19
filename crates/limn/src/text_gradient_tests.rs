use super::*;

/// The ink of each band of `text` painted, left to right, with its clip.
fn bands(frame: &Frame, text: &str) -> Vec<(Rect, Color)> {
    let mut clip = None;
    let mut found = Vec::new();
    for command in &frame.commands {
        match command {
            DrawCommand::PushClip(rect) => clip = Some(*rect),
            DrawCommand::PopClip => clip = None,
            DrawCommand::Text {
                text: painted,
                color,
                ..
            } if painted == text => found.push((clip.unwrap_or_default(), *color)),
            _ => {}
        }
    }
    found
}

fn tree(element: Element) -> (UiTree, Frame) {
    let mut tree = UiTree::new(element);
    tree.set_text_measurer(|text, _, _| text.chars().count() as f32 * 8.0);
    let frame = tree.layout(Size::new(400.0, 100.0));
    (tree, frame)
}

#[test]
fn a_gradient_inks_a_line_in_bands_from_its_first_stop_to_its_last() {
    let (red, blue) = (Color::rgb(255, 0, 0), Color::rgb(0, 0, 255));
    for element in [
        Element::truncated_text("Gradiente"),
        Element::text("Gradiente"),
    ] {
        let (_, frame) = tree(element.foreground_gradient(vec![red, blue]));
        let painted = bands(&frame, "Gradiente");
        assert_eq!(painted.len(), 24, "one run per band");
        // The bands cover the painted line side by side: 9 characters of 8 px.
        assert_eq!(painted[0].0.x, 0.0);
        let end = painted[23].0;
        assert!((end.x + end.width - 72.0).abs() < 0.01);
        // Red at the left, blue at the right, blending between.
        assert!(painted[0].1.r > 240 && painted[0].1.b < 15);
        assert!(painted[23].1.b > 240 && painted[23].1.r < 15);
        assert!(painted.windows(2).all(|pair| pair[0].1.r >= pair[1].1.r));
    }
}

#[test]
fn a_single_stop_or_none_paints_the_plain_foreground_and_layout_is_unchanged() {
    let ink = Color::rgb(10, 200, 30);
    let plain = Element::truncated_text("Nombre largo que se corta").foreground(ink);
    let narrow = |text: Element| Element::row(vec![text.size(Some(80.0), None)]);
    let (_, before) = tree(narrow(plain.clone()));
    for stops in [vec![], vec![Color::rgb(1, 2, 3)]] {
        let (_, after) = tree(narrow(plain.clone().foreground_gradient(stops)));
        assert_eq!(after.commands, before.commands);
    }
    // Cut short, the gradient spans what is painted.
    let (_, frame) = tree(narrow(
        plain.foreground_gradient(vec![ink, Color::rgb(200, 10, 30)]),
    ));
    let painted: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(painted.len(), 24);
    let shown = before
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .unwrap();
    assert!(shown.chars().count() < "Nombre largo que se corta".chars().count());
    assert!(
        painted.iter().all(|text| *text == shown),
        "the same cut text"
    );
    assert_eq!(frame.regions, before.regions);
}

#[test]
fn a_gradient_keeps_the_foregrounds_alpha_and_inks_each_line() {
    let stops = vec![Color::rgb(255, 0, 0), Color::rgb(0, 0, 255)];
    let (_, frame) = tree(
        Element::text("uno\ndos")
            .foreground(Color::rgba(0, 0, 0, 128))
            .foreground_gradient(stops),
    );
    for line in ["uno", "dos"] {
        let painted = bands(&frame, line);
        assert_eq!(painted.len(), 24, "{line}");
        assert!(painted.iter().all(|(_, color)| color.a == 128));
    }
}
