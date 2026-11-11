//! The accent switch keeps a thumb that reads on its track, on a dark, a
//! light and a theme's palette. This test exercises `Switch` with the
//! palette of `Theme`.
use crate::*;

/// A switch at its own size, settled: its track's rectangle and color,
/// then its thumb's.
fn switch_paint(switch: Switch) -> ((crate::Rect, Color), (crate::Rect, Color)) {
    let mut tree = UiTree::new(Element::row(vec![switch.render(), Element::spacer().flex(1.0)]));
    tree.set_reduced_motion(true);
    let frame = tree.layout(Size::new(200.0, 40.0));
    let fills: Vec<_> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { rect, color, radius } => Some((*rect, *color, *radius)),
            _ => None,
        })
        .collect();
    // The track is the pill the others stand in; the thumb is the circle inside it.
    let [(track, track_color, _), (thumb, thumb_color, thumb_radius)] = fills.as_slice() else {
        panic!("a track and a thumb, got {fills:?}");
    };
    assert_eq!(*thumb_radius, thumb.width / 2.0);
    ((*track, *track_color), (*thumb, *thumb_color))
}

#[test]
fn the_accent_variant_keeps_a_light_thumb_on_the_link_color_or_the_ink_and_on_the_muted_color_off() {
    let dark = ComponentPalette::neutral_dark();
    let light = ComponentPalette {
        surface: Color::rgb(255, 255, 255),
        raised: Color::rgb(250, 250, 250),
        hover: Color::rgb(235, 235, 235),
        selected: Color::rgb(220, 220, 220),
        border: Color::rgb(205, 205, 205),
        text: Color::rgb(32, 32, 32),
        muted: Color::rgb(100, 100, 100),
        shadow: Color::rgba(0, 0, 0, 40),
    };
    let ink = Color::rgb(88, 101, 242);
    // A theme's text and muted color are translucent; its thumb is the foreground at full strength.
    let theme = crate::Theme::DARK.palette();
    // The thumb is the palette's lightest color: its text on a dark one, its surface on a light one.
    for (palette, thumb) in [(dark, dark.text), (light, light.surface), (theme, Color::rgb(255, 255, 255))] {
        let accent = |checked| Switch::new("s", checked, palette).variant(SwitchVariant::Accent);
        let paint = |switch: Switch| {
            let ((_, track), (_, thumb)) = switch_paint(switch);
            (track, thumb)
        };
        // A pale ink, such as the gallery theme's accent, is darkened like the link color.
        let pale = Color::rgb(25, 170, 216);
        for switch in [accent(true), accent(false), accent(true).ink(pale)] {
            let (track, drawn) = paint(switch);
            assert_eq!(drawn, thumb);
            let contrast = crate::chip::contrast(thumb, track);
            assert!(contrast >= 3.0, "{palette:?}: {track:?} at {contrast}");
        }
        for checked in [true, false] {
            assert_eq!(paint(accent(checked).disabled()), paint(accent(checked)), "disabled, it keeps its paint");
        }
        assert_eq!(paint(accent(true).ink(ink)), (ink, thumb), "an ink that reads is kept");
        // An off ink is the host's, kept as given, and leaves the switch on as it was.
        let pale_gray = Color::rgb(206, 206, 206);
        assert_eq!(paint(accent(false).off_ink(pale_gray)), (pale_gray, thumb));
        assert_eq!(paint(accent(true).off_ink(pale_gray)), paint(accent(true)));
        assert_eq!(paint(accent(false).ink(ink)), paint(accent(false)));
    }
    // Where the link and muted colors already read, they are the track.
    let paint = |checked| switch_paint(Switch::new("s", checked, light).variant(SwitchVariant::Accent)).0 .1;
    assert_eq!((paint(true), paint(false)), (light.link(), light.muted));
    // The default variant takes the ink for its track and keeps its own thumb.
    let paint = |checked| {
        let ((_, track), (_, thumb)) = switch_paint(Switch::new("s", checked, dark).ink(ink));
        (track, thumb)
    };
    assert_eq!((paint(true), paint(false)), ((ink, dark.surface), (dark.selected, dark.text)));
}
