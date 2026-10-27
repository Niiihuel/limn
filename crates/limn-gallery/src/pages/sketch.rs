use super::{caption, section, Cx, Section};
use limn::{AlertColors, AlertTone, Component, Element, Point, RadiusRole, Sketch, SketchMark, SketchTool};

/// One mark of each kind, drawn over the same area.
fn marks(ink: limn::Color) -> Vec<SketchMark> {
    vec![
        SketchMark::Stroke {
            points: vec![Point::new(0.05, 0.9), Point::new(0.15, 0.55), Point::new(0.25, 0.9)],
            color: ink,
            width: 4.0,
        },
        SketchMark::Rectangle { from: Point::new(0.3, 0.1), to: Point::new(0.5, 0.5), color: ink, width: 2.0 },
        SketchMark::Ellipse { from: Point::new(0.55, 0.1), to: Point::new(0.75, 0.5), color: ink, width: 2.0 },
        SketchMark::Line { from: Point::new(0.3, 0.7), to: Point::new(0.7, 0.7), color: ink, width: 8.0 },
        SketchMark::Arrow { from: Point::new(0.8, 0.9), to: Point::new(0.95, 0.15), color: ink, width: 4.0 },
    ]
}

pub fn view(cx: &Cx) -> Vec<Section> {
    let palette = cx.palette;
    let width = cx.width.min(480.0);
    let ink = AlertColors::for_tone(AlertTone::Error, palette).accent;
    let area = |sketch: Element| {
        Element::panel(sketch)
            .background(palette.raised)
            .radius(RadiusRole::Surface.px())
    };
    let shown = area(Sketch::new("sketch-marks", &marks(ink), SketchTool::Pen, ink, 2.0).size(width, 160.0).render());
    let live = area(Sketch::new("sketch-live", &[], SketchTool::Arrow, ink, 4.0).size(width, 160.0).render());
    vec![section(cx, "Sketch", vec![
        shown,
        caption(cx, "Sketch::new(id, marks, tool, color, width): the marks a host keeps, drawn over the area in its own shares, so they keep their place when the size changes."),
        live,
        caption(cx, "Drag over it with the arrow tool: the arrow is drawn as the pointer moves and reported as one mark on release. The gallery keeps no marks, so the next frame shows only the ones it was given."),
    ])]
}
