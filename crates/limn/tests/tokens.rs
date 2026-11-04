//! Changing the process-wide tokens. This lives outside the library's unit
//! tests because they share one process, and a test that reshapes every
//! component would break the others while it ran. One test function keeps
//! the changes in order.

use limn::{
    set_tokens, tokens, Button, Color, Component, ComponentPalette, DrawCommand, Element,
    ElementKind, Empty, FontFamily, Frame, Markdown, Modal, Motion, Point, Radius, Size, Toast,
    Tokens, Tooltip, TooltipOptions, TooltipShape, TooltipStyle, UiTree,
};
use web_time::{Duration, Instant};

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

/// A quote and a code block, which round with tokens; the tree is cached by the engine.
fn markdown(palette: ComponentPalette) -> Frame {
    let prose = Markdown::new("> cita\n\n```rust\nfn main() {}\n```", palette).render().element;
    let mut tree = UiTree::new(prose);
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

    let prose = radii(&markdown(palette));
    assert!(prose.contains(&6) && prose.contains(&8), "{prose:?}");

    // Square corners reach components and engine-painted surfaces alike.
    set_tokens(Tokens { radius: Radius::SQUARE, ..Tokens::DEFAULT });
    assert_eq!(radii(&settled(palette)), [0], "every corner is square");

    // A Markdown tree cached while corners were round is not reused square.
    assert_eq!(radii(&markdown(palette)), [0], "cached quote and code block are square");

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

    // One look for every tooltip, whichever palette its control was built with.
    set_tokens(Tokens::DEFAULT);
    let light = ComponentPalette { raised: Color::WHITE, border: Color::rgb(220, 220, 220), ..palette };
    let hint_of = |element: Element| match element.kind {
        ElementKind::Tooltip { colors, options, .. } => (colors, options),
        _ => panic!("a hint"),
    };
    let hint = |palette| hint_of(Button::new("b", "B", palette).tooltip("Guardar").render());
    assert_ne!(hint(palette).0, hint(light).0, "each palette derives its own");
    assert_eq!(hint(palette).1, TooltipOptions::DEFAULT);
    let own = TooltipStyle {
        background: Color::rgb(10, 10, 12),
        border: Color::rgba(149, 149, 162, 51),
        keycap: Color::rgb(36, 36, 38),
        text: Color::rgb(220, 220, 223),
        muted: Color::rgb(150, 151, 158),
        shadow: Color::rgba(0, 0, 0, 61),
    };
    // And the options every hint starts from, the engine's own included.
    let options = TooltipOptions {
        delay_ms: 70,
        title_font_size: 14.0,
        title_font_family: FontFamily::SansMedium,
        shape: Some(TooltipShape {
            radius: 8.0,
            padding_x: 12.0,
            padding_y: 8.0,
            shadow_offset: Point::new(0.0, 12.0),
            shadow_blur: 24.0,
            caret: Size::new(16.0, 10.0),
        }),
        open_scale: 0.95,
        enter_ms: Some(70),
        ..TooltipOptions::DEFAULT
    };
    set_tokens(Tokens { tooltip: Some(own), tooltip_options: options, ..Tokens::DEFAULT });
    for palette in [palette, light] {
        assert_eq!(hint(palette), (own, options));
        let (colors, kept) = hint_of(Tooltip::new(Element::spacer(), "a", "A", palette).delay_ms(0).render());
        assert_eq!((colors, kept), (own, TooltipOptions { delay_ms: 0, ..options }));
    }
    // Its body is painted in the host's fill and shape.
    let mut tree = UiTree::new(Button::new("b", "B", light).tooltip("Guardar").render());
    tree.set_reduced_motion(true);
    tree.layout(WINDOW);
    assert!(tree.focus("b"));
    let frame = tree.layout(WINDOW);
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::FillRect { color, radius, .. } if *color == own.background && *radius == 8.0
    )));
    assert!(frame.commands.iter().any(|command| matches!(command,
        DrawCommand::Text { text, font_size, font_family: FontFamily::SansMedium, .. } if text == "Guardar" && *font_size == 14.0
    )));
    // Numbers that cannot be painted are made safe.
    set_tokens(Tokens {
        tooltip_options: TooltipOptions {
            title_font_size: f32::NAN,
            open_scale: 0.1,
            shape: Some(TooltipShape { radius: -4.0, padding_x: f32::INFINITY, ..options.shape.unwrap() }),
            ..options
        },
        ..Tokens::DEFAULT
    });
    let safe = tokens().tooltip_options;
    assert_eq!((safe.title_font_size, safe.open_scale), (12.0, 0.5));
    assert_eq!(safe.shape.map(|shape| (shape.radius, shape.padding_x)), Some((0.0, 0.0)));

    set_tokens(Tokens::DEFAULT);
    assert_eq!(tokens(), Tokens::DEFAULT);
}
