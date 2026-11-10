use super::*;

/// One paint instruction of a [`Frame`], in logical pixels. A backend
/// carries out `Frame::commands` in order.
#[derive(Clone, Debug, PartialEq)]
pub enum DrawCommand {
    /// A rounded backdrop-sampling surface, painted before its foreground.
    Glass {
        /// The surface's bounds; what lies behind them is what the material
        /// samples.
        rect: Rect,
        /// Corner radius, the same on all four corners.
        radius: f32,
        /// How the backdrop is blurred and tinted. A backend that cannot sample
        /// the backdrop can fill the bounds with `material.tint`, as the egui
        /// one does.
        material: GlassMaterial,
        /// Opacity of the whole surface from 0 to 1: 1 unless a transition is
        /// fading it.
        opacity: f32,
    },
    /// A selection lens sampling a local, feedback-free copy of its track and labels.
    GlassSelection {
        /// Lens bounds in logical pixels.
        rect: Rect,
        /// Lens corner radius in logical pixels.
        radius: f32,
        /// Optical properties of the lens.
        material: GlassMaterial,
        /// Opacity of the lens, from zero to one.
        opacity: f32,
        /// Local backdrop bounds, including the neighboring labels.
        backdrop_rect: Rect,
        /// Ordinary paint captured before inserting this lens.
        backdrop: Vec<DrawCommand>,
    },
    /// A box shadow, pushed before the fill of the box that casts it.
    Shadow {
        /// The box that casts the shadow, not the shadow's own extent.
        rect: Rect,
        /// Color, offset, blur and spread of the shadow.
        shadow: BoxShadow,
        /// Corner radius of the casting box.
        radius: f32,
    },
    /// A filled rectangle with the same radius on every corner.
    FillRect {
        /// Bounds of the fill, relative to the frame's top-left corner.
        rect: Rect,
        /// Fill color; its alpha is the fill's opacity.
        color: Color,
        /// Corner radius; 0 is a square corner.
        radius: f32,
    },
    /// A filled rectangle whose corners do not all have the same radius.
    FillCornerRect {
        /// Bounds of the fill, relative to the frame's top-left corner.
        rect: Rect,
        /// Fill color; its alpha is the fill's opacity.
        color: Color,
        /// Radius of each corner.
        radii: CornerRadii,
    },
    /// One implicitly closed, simple Y-monotone selection contour. Renderers
    /// tessellate its interior once and antialias only its exterior boundary.
    SelectionContour {
        /// The contour's vertices, relative to the frame's top-left corner; the
        /// last joins the first.
        points: Arc<[Point]>,
        /// Fill color of the interior.
        color: Color,
    },
    /// Paint the exterior of four rounded corners as a continuous surface.
    /// Renderers must not antialias internal tessellation edges independently.
    CornerMask {
        /// The rectangle whose corners are masked.
        rect: Rect,
        /// Color of the area between the rectangle's square corners and its
        /// rounded ones.
        color: Color,
        /// Radius of each rounded corner; the mask covers what lies outside it.
        radii: CornerRadii,
    },
    /// A rectangle filled with a linear gradient from its top edge to its
    /// bottom edge. The tree paints fades with it, one end transparent.
    FillVerticalGradient {
        /// Bounds of the gradient.
        rect: Rect,
        /// Color along the top edge.
        top: Color,
        /// Color along the bottom edge.
        bottom: Color,
    },
    /// A rectangle filled with a gradient from `left` to `right`, with square
    /// corners: the length of a bar that runs sideways.
    FillHorizontalGradient {
        /// Bounds of the fill, relative to the frame's top-left corner.
        rect: Rect,
        /// Color at the left edge.
        left: Color,
        /// Color at the right edge.
        right: Color,
    },
    /// A rectangle outline with the same radius on every corner. The egui
    /// backend strokes inside `rect`.
    StrokeRect {
        /// Outer edge of the outline.
        rect: Rect,
        /// Stroke color; its alpha is the outline's opacity.
        color: Color,
        /// Stroke thickness in logical pixels.
        width: f32,
        /// Corner radius of the outer edge.
        radius: f32,
    },
    /// A rectangle outline whose corners do not all have the same radius. The
    /// egui backend strokes inside `rect`.
    StrokeCornerRect {
        /// Outer edge of the outline.
        rect: Rect,
        /// Stroke color; its alpha is the outline's opacity.
        color: Color,
        /// Stroke thickness in logical pixels.
        width: f32,
        /// Radius of each corner of the outer edge.
        radii: CornerRadii,
    },
    /// A filled triangle: the caret of a tooltip or coach mark, or the arrow of
    /// a scrollbar button.
    FillTriangle {
        /// The three vertices, relative to the frame's top-left corner.
        points: [Point; 3],
        /// Fill color; its alpha is the fill's opacity.
        color: Color,
    },
    /// Triangles with a color at each corner, blended across each: the body of
    /// an area under a line, a slice of a pie, the shape of a radar's series.
    /// Its edges are not smoothed; what draws it strokes them.
    FillMesh {
        /// The corners, relative to the frame's top-left corner.
        points: Arc<[Point]>,
        /// The color at each corner, one for each point.
        colors: Arc<[Color]>,
        /// Three indices into `points` for each triangle.
        indices: Arc<[u32]>,
    },
    /// An open stroked path through `points`, centered on them. A dashed
    /// path alternates short marks and gaps along its length.
    Polyline {
        /// The vertices in order, relative to the frame's top-left corner.
        points: Arc<[Point]>,
        /// Stroke color.
        color: Color,
        /// Stroke thickness in logical pixels.
        width: f32,
        /// True for a dashed stroke; the egui backend draws 3 pixel marks and 3
        /// pixel gaps.
        dashed: bool,
    },
    /// Glyphs limn has shaped and placed; see [`crate::GlyphRun`]. A backend
    /// draws each glyph where the run says, and lays nothing out again.
    Glyphs(crate::GlyphRun),
    /// An outline or a shadow of text, painted before the text itself:
    /// `effect` of each of `runs`, a [`DrawCommand::Text`] or a
    /// [`DrawCommand::Glyphs`] placed where the text that follows is. The
    /// runs' own ink is not painted. A backend that paints no effects may
    /// skip it; the text still reads.
    TextEffect {
        /// What is painted of the runs.
        effect: crate::TextEffect,
        /// The text the effect is of, as it is painted after this.
        runs: Vec<DrawCommand>,
    },
    /// A run of text the tree has already measured and wrapped. A backend
    /// paints it from `position` without wrapping it again.
    Text {
        /// Top-left corner of the run.
        position: Point,
        /// The run's characters.
        text: String,
        /// Color of the glyphs.
        color: Color,
        /// Font size in logical pixels.
        font_size: f32,
        /// The face to paint in: monospace, or a weight or slant of the sans
        /// face.
        font_family: FontFamily,
    },
    /// One of the library's icons, drawn in one color.
    Icon {
        /// Where the icon is drawn; the egui backend aligns it to the pixel
        /// grid.
        rect: Rect,
        /// Which of the library's icons to draw.
        icon: Icon,
        /// Color the icon's shape is drawn in.
        tint: Color,
        /// How the icon is turned and stretched about the middle of `rect`:
        /// [`IconTransform::IDENTITY`](crate::IconTransform::IDENTITY) unless
        /// an [`Element::hover_motion`](crate::Element::hover_motion) is
        /// moving it.
        transform: crate::IconTransform,
    },
    /// A picture. A cover image fills `rect` and is cropped; any other is
    /// fitted inside it.
    RasterImage {
        /// The area given to the picture.
        rect: Rect,
        /// The picture, with the text alternative a backend gives to assistive
        /// technology.
        image: RasterImage,
        /// Opacity from 0 to 1: 1 unless a transition is fading the element.
        opacity: f32,
        /// Corner radius the picture is rounded to.
        radius: f32,
    },
    /// Pixels the host presents to the backend under `id`, outside the frame,
    /// placed in the rectangle as `fit` says and clipped to it. With
    /// `SurfaceFit::Native` they start at the top-left corner at their own
    /// density, never stretched to fill it.
    Surface {
        /// The area the surface shows in; its pixels are clipped to it.
        rect: Rect,
        /// The ID the host presents the surface's pixels under.
        id: String,
        /// The name a backend gives to assistive technology for the surface.
        label: String,
        /// Opacity from 0 to 1: 1 unless a transition is fading the element.
        opacity: f32,
        /// How the pixels fill the rectangle.
        fit: crate::SurfaceFit,
        /// Corner radius the pixels shown are rounded to.
        radius: f32,
        /// The pixels are painted flipped left to right, as a mirror shows
        /// them; see [`Element::surface_mirrored`](crate::Element::surface_mirrored).
        mirrored: bool,
    },
    /// An activity indicator that turns, sized to the shorter side of `rect`.
    Spinner {
        /// Bounds of the indicator.
        rect: Rect,
        /// Stroke color.
        color: Color,
        /// Rotation in radians, from 0 to a full turn every 0.9 seconds. Always
        /// 0 under reduced motion.
        angle: f32,
        /// Which shape turns: an open circle or eight spokes.
        variant: SpinnerVariant,
    },
    /// A record behind its sleeve. `rect` is the sleeve; the record's
    /// diameter is `RECORD_SHARE` of its side and at `out` 1 it shows
    /// `RECORD_REACH` of the side beyond the sleeve's end. `fill` paints a
    /// sleeve with no picture.
    Record {
        /// The sleeve. The egui backend draws it as a square of the shorter
        /// side, from the top-left corner.
        rect: Rect,
        /// The sleeve's picture, cropped to cover it. `None` paints `fill`.
        image: Option<RasterImage>,
        /// Sleeve color when there is no picture: the element's background, or
        /// transparent without one.
        fill: Color,
        /// How far the record is drawn out, from 0 (centered behind the sleeve)
        /// to 1.
        out: f32,
        /// The record's turn in radians, from 0 to a full turn.
        angle: f32,
        /// Corner radius of the sleeve.
        radius: f32,
        /// Opacity from 0 to 1: 1 unless a transition is fading the element.
        opacity: f32,
    },
    /// A circle of `track` with an arc of `fill` over it, starting at twelve
    /// o'clock and running clockwise. It is centered in `rect` and sized to the
    /// shorter side.
    ProgressRing {
        /// Bounds of the ring.
        rect: Rect,
        /// Share of the circle the arc covers, from 0 to 1. `None` draws the
        /// track alone.
        fraction: Option<f32>,
        /// Color of the whole circle underneath.
        track: Color,
        /// Color of the arc.
        fill: Color,
    },
    /// The animated effort slider: a pill-shaped rail filled up to a round
    /// knob, with a dot for each step and particles drifting in the fill.
    EffortSlider {
        /// Bounds of the control; the rail is centered vertically in them.
        rect: Rect,
        /// Where the knob is shown, 0 at the left end and 1 at the right. It
        /// follows the pointer during a drag and eases to the value after it.
        fraction: f32,
        /// Number of levels. The egui backend draws their dots when there are 2
        /// to 12.
        steps: usize,
        /// Clock of the idle animation in seconds, wrapping at 1000. It does
        /// not advance under reduced motion.
        phase: f32,
        /// True while the pointer holds the knob.
        dragging: bool,
        /// True when the tree was told to reduce motion: a backend leaves out
        /// the knob's pulse and the burst.
        reduced_motion: bool,
        /// How much of the top level's gradient covers the fill, from 0 to 1.
        /// It eases to 1 while the slider is on its last step and that level is
        /// available.
        ultra_mix: f32,
        /// Seconds since the slider settled on its top level, for the 0.28
        /// seconds the burst lasts. `None` at any other time.
        burst_age: Option<f32>,
        /// Opacity from 0 to 1: 1 unless a transition is fading the element.
        opacity: f32,
    },
    /// Clip every command up to the matching `PopClip` to this rectangle,
    /// intersected with the clips already in force.
    PushClip(Rect),
    /// Round the clip of the `PushClip` just before it: until that clip's
    /// `PopClip`, fills and images stay inside `rect` with these corners, so
    /// content flush with or past the edge of a rounded panel keeps the
    /// panel's corners. Text is not affected. A nested rounded clip replaces
    /// it; a nested plain one keeps it. A backend that cannot round a clip
    /// may ignore this and keep the rectangle.
    RoundClip {
        /// The rounded shape, inside the rectangle of the clip it rounds.
        rect: Rect,
        /// Its corners.
        radii: CornerRadii,
    },
    /// End the innermost `PushClip`.
    PopClip,
}

impl DrawCommand {
    /// About where the command paints: its rectangle, the box around its
    /// points, or the line box of its text. `None` for clips and for what is
    /// fully transparent.
    pub fn bounds(&self) -> Option<Rect> {
        let around = |points: &[Point], pad: f32| {
            let first = points.first()?;
            let (mut left, mut top, mut right, mut bottom) = (first.x, first.y, first.x, first.y);
            for point in points {
                left = left.min(point.x);
                top = top.min(point.y);
                right = right.max(point.x);
                bottom = bottom.max(point.y);
            }
            Some(Rect::new(left - pad, top - pad, right - left + pad * 2.0, bottom - top + pad * 2.0))
        };
        let seen = |color: &Color| color.a > 0;
        match self {
            Self::Shadow { rect, shadow, .. } => seen(&shadow.color).then(|| {
                let grow = shadow.blur + shadow.spread;
                Rect::new(rect.x + shadow.offset.x - grow, rect.y + shadow.offset.y - grow, rect.width + grow * 2.0, rect.height + grow * 2.0)
            }),
            Self::FillRect { rect, color, .. }
            | Self::FillCornerRect { rect, color, .. }
            | Self::CornerMask { rect, color, .. }
            | Self::StrokeRect { rect, color, .. }
            | Self::StrokeCornerRect { rect, color, .. }
            | Self::Spinner { rect, color, .. }
            | Self::Icon { rect, tint: color, .. } => seen(color).then_some(*rect),
            Self::FillVerticalGradient { rect, top, bottom } => (seen(top) || seen(bottom)).then_some(*rect),
            Self::FillHorizontalGradient { rect, left, right } => (seen(left) || seen(right)).then_some(*rect),
            Self::Glass { rect, opacity, .. }
            | Self::GlassSelection { rect, opacity, .. }
            | Self::RasterImage { rect, opacity, .. }
            | Self::Record { rect, opacity, .. }
            | Self::Surface { rect, opacity, .. }
            | Self::EffortSlider { rect, opacity, .. } => (*opacity > 0.0).then_some(*rect),
            Self::ProgressRing { rect, .. } => Some(*rect),
            Self::SelectionContour { points, color } => seen(color).then(|| around(points, 0.0)).flatten(),
            Self::FillTriangle { points, color } => seen(color).then(|| around(points, 0.0)).flatten(),
            Self::Polyline { points, color, width, .. } => seen(color).then(|| around(points, width / 2.0)).flatten(),
            Self::FillMesh { points, .. } => around(points, 0.0),
            Self::Glyphs(run) => {
                let glyphs: Vec<Point> = run.glyphs.iter().map(|glyph| Point::new(run.origin.x + glyph.x, run.origin.y + glyph.y)).collect();
                // The glyphs' baselines, grown by about an ascent up, a descent down and the
                // last glyph's width right.
                let line = seen(&run.color).then(|| around(&glyphs, 0.0)).flatten()?;
                Some(Rect::new(line.x, line.y - run.size * 0.8, line.width + run.size, line.height + run.size * 1.05))
            }
            Self::TextEffect { effect, runs } => {
                let (reach, offset, color) = match *effect {
                    crate::TextEffect::Stroke { width, color } => (width / 2.0, Point::new(0.0, 0.0), color),
                    crate::TextEffect::Shadow { offset, blur, color } => (blur, offset, color),
                };
                let corners: Vec<Point> = runs
                    .iter()
                    .filter_map(DrawCommand::bounds)
                    .flat_map(|rect| [Point::new(rect.x, rect.y), Point::new(rect.x + rect.width, rect.y + rect.height)])
                    .map(|corner| Point::new(corner.x + offset.x, corner.y + offset.y))
                    .collect();
                seen(&color).then(|| around(&corners, reach)).flatten()
            }
            // `position` is the line's top-left corner.
            Self::Text { position, text, color, font_size, .. } => seen(color).then(|| {
                let width = text.chars().count() as f32 * font_size * 0.6;
                Rect::new(position.x, position.y, width, font_size * 1.3)
            }),
            Self::PushClip(_) | Self::RoundClip { .. } | Self::PopClip => None,
        }
    }

    /// A translated copy at `opacity` of its own: a drag preview, or a
    /// surface on its way to where it is laid out.
    pub(super) fn moved(&self, dx: f32, dy: f32, opacity: f32) -> Self {
        let fade = |color: &mut Color| color.a = (f32::from(color.a) * opacity).round() as u8;
        let shift = |rect: &mut Rect| {
            rect.x += dx;
            rect.y += dy;
        };
        let opacity_factor = opacity;
        let mut ghost = self.clone();
        match &mut ghost {
            Self::Shadow { rect, shadow, .. } => {
                shift(rect);
                fade(&mut shadow.color);
            }
            Self::FillRect { rect, color, .. }
            | Self::FillCornerRect { rect, color, .. }
            | Self::CornerMask { rect, color, .. }
            | Self::StrokeRect { rect, color, .. }
            | Self::StrokeCornerRect { rect, color, .. }
            | Self::Spinner { rect, color, .. }
            | Self::Icon {
                rect, tint: color, ..
            } => {
                shift(rect);
                fade(color);
            }
            Self::FillVerticalGradient { rect, top, bottom } => {
                shift(rect);
                fade(top);
                fade(bottom);
            }
            Self::FillHorizontalGradient { rect, left, right } => {
                shift(rect);
                fade(left);
                fade(right);
            }
            Self::ProgressRing {
                rect, track, fill, ..
            } => {
                shift(rect);
                fade(track);
                fade(fill);
            }
            Self::GlassSelection { rect, opacity, backdrop_rect, backdrop, .. } => {
                shift(rect);
                shift(backdrop_rect);
                *opacity *= opacity_factor;
                *backdrop = backdrop.iter().map(|command| command.moved(dx, dy, 1.0)).collect();
            }
            Self::RasterImage { rect, opacity, .. }
            | Self::Glass { rect, opacity, .. }
            | Self::Record { rect, opacity, .. }
            | Self::Surface { rect, opacity, .. }
            | Self::EffortSlider { rect, opacity, .. } => {
                shift(rect);
                *opacity *= opacity_factor;
            }
            Self::SelectionContour { points, color } | Self::Polyline { points, color, .. } => {
                *points = points
                    .iter()
                    .map(|point| Point::new(point.x + dx, point.y + dy))
                    .collect();
                fade(color);
            }
            Self::FillMesh { points, colors, .. } => {
                *points = points.iter().map(|point| Point::new(point.x + dx, point.y + dy)).collect();
                *colors = colors
                    .iter()
                    .map(|color| {
                        let mut color = *color;
                        fade(&mut color);
                        color
                    })
                    .collect();
            }
            Self::FillTriangle { points, color } => {
                for point in points {
                    *point = Point::new(point.x + dx, point.y + dy);
                }
                fade(color);
            }
            Self::Text {
                position, color, ..
            } => {
                *position = Point::new(position.x + dx, position.y + dy);
                fade(color);
            }
            Self::Glyphs(run) => {
                run.origin = Point::new(run.origin.x + dx, run.origin.y + dy);
                fade(&mut run.color);
            }
            Self::TextEffect { effect, runs } => {
                let (crate::TextEffect::Stroke { color, .. } | crate::TextEffect::Shadow { color, .. }) = effect;
                fade(color);
                for run in runs {
                    *run = run.moved(dx, dy, opacity);
                }
            }
            Self::PushClip(rect) | Self::RoundClip { rect, .. } => shift(rect),
            Self::PopClip => {}
        }
        ghost
    }
}
