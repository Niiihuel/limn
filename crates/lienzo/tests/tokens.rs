//! Changing the process-wide tokens. This lives outside the library's unit
//! tests because they share one process, and a test that reshapes every
//! component would break the others while it ran. One test function keeps
//! the changes in order.

use lienzo::{
    set_tokens, tokens, Button, Component, ComponentPalette, DrawCommand, Element, Empty, Frame,
    Modal, Motion, Radius, Size, Toast, Tokens, UiTree,
};
use std::time::{Duration, Instant};

const WINDOW: Size = Size::new(800.0, 600.0);

fn scene(palette: ComponentPalette) -> Element {
    let page = Element::column(vec![
        Element::row(vec![Button::new("save", "Guardar", palette).render(), Element::spacer().flex(1.0)]),
        Empty::new("Sin resultados", "Prueba otro término.", palette).width(360.0).render(),
    ]);
    let page = Modal::new(page, true, "Título", "Descripción", "close", palette).render();
    Toast::layer(page, vec![Toast::new("Guardado", palette).render()], 0.0)
}

/// Every corner radius painted in a frame, rounded for comparison.
fn radii(frame: &Frame) -> Vec<i32> {
    let mut radii: Vec<i32> = frame
        .commands
        .iter()
        .filter_map(|command| match command {
            DrawCommand::FillRect { radius, .. } | DrawCommand::StrokeRect { radius, .. } => Some(*radius),
            _ => None,
        })
        .map(|radius| radius.round() as i32)
        .collect();
    radii.sort_unstable();
    radii.dedup();
    radii
}

fn settled(palette: ComponentPalette) -> Frame {
    let mut tree = UiTree::new(scene(palette));
    tree.set_reduced_motion(true);
    tree.layout(WINDOW)
}

/// Opacity of the toast's surface on the very first frame it is painted.
fn toast_opacity_on_arrival(palette: ComponentPalette) -> u8 {
    let surface = tokens().radius.surface;
    let toast = Toast::layer(Element::spacer(), vec![Toast::new("Guardado", palette).render()], 0.0);
    UiTree::new(toast)
        .layout_at(WINDOW, Instant::now())
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::FillRect { color, radius, .. } if *radius == surface => Some(color.a),
            _ => None,
        })
        .expect("toast surface")
}

#[test]
fn tokens_reshape_and_repace_every_component_built_after_they_change() {
    let palette = ComponentPalette::neutral_dark();

    // Out of the box: the documented defaults, and nothing square.
    assert_eq!(tokens(), Tokens::DEFAULT);
    let defaults = radii(&settled(palette));
    for expected in [4, 6, 12] {
        assert!(defaults.contains(&expected), "{expected} missing from {defaults:?}");
    }
    assert_eq!(Button::new("b", "B", palette).render().style.transition_ms, 120);
    assert_eq!(toast_opacity_on_arrival(palette), 0, "a toast fades in by default");

    // Square corners reach components and engine-painted surfaces alike.
    set_tokens(Tokens { radius: Radius::SQUARE, ..Tokens::DEFAULT });
    assert_eq!(radii(&settled(palette)), [0], "every corner is square");

    // Rounder: each role scales, and the roles stay distinct.
    set_tokens(Tokens { radius: Radius::DEFAULT.scaled(2.0), ..Tokens::DEFAULT });
    let round = radii(&settled(palette));
    for expected in [8, 12, 24] {
        assert!(round.contains(&expected), "{expected} missing from {round:?}");
    }
    assert!(!round.contains(&4), "no control kept the old radius: {round:?}");

    // A radius that cannot be drawn falls back instead of reaching a renderer.
    set_tokens(Tokens {
        radius: Radius { control: f32::NAN, ..Radius::DEFAULT },
        ..Tokens::DEFAULT
    });
    assert_eq!(tokens().radius, Radius::DEFAULT);

    // Pace: controls read the hover token, surfaces the entrance token.
    set_tokens(Tokens { motion: Motion::DEFAULT.scaled(2.0), ..Tokens::DEFAULT });
    assert_eq!(Button::new("b", "B", palette).render().style.transition_ms, 240);
    assert_eq!(toast_opacity_on_arrival(palette), 0);

    // A zero entrance shows a surface whole on its first frame.
    set_tokens(Tokens {
        motion: Motion { enter_ms: 0, ..Motion::DEFAULT },
        ..Tokens::DEFAULT
    });
    assert_eq!(toast_opacity_on_arrival(palette), 255);

    // And a slow entrance is still partway through after the default time.
    set_tokens(Tokens {
        motion: Motion { enter_ms: 1_000, ..Motion::DEFAULT },
        ..Tokens::DEFAULT
    });
    let toast = Toast::layer(Element::spacer(), vec![Toast::new("Guardado", palette).render()], 0.0);
    let mut tree = UiTree::new(toast);
    let start = Instant::now();
    tree.layout_at(WINDOW, start);
    let later = tree.layout_at(WINDOW, start + Duration::from_millis(140));
    let opacity = later
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::FillRect { color, radius, .. } if *radius == tokens().radius.surface => Some(color.a),
            _ => None,
        })
        .expect("toast surface");
    assert!(opacity > 0 && opacity < 255, "{opacity}");

    set_tokens(Tokens::DEFAULT);
    assert_eq!(tokens(), Tokens::DEFAULT);
}
