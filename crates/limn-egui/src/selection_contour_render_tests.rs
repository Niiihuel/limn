use super::*;
use limn::{Color, DrawCommand, Frame, Rect};

fn contour() -> std::sync::Arc<[Point]> {
    // Three connected rows with both convex and concave corners. The middle
    // row is deliberately wider, so a convex-only tessellator would be wrong.
    vec![
        Point::new(25.0, 10.0),
        Point::new(100.0, 10.0),
        Point::new(100.0, 30.0),
        Point::new(110.0, 30.0),
        Point::new(110.0, 50.0),
        Point::new(70.0, 50.0),
        Point::new(70.0, 70.0),
        Point::new(10.0, 70.0),
        Point::new(10.0, 30.0),
        Point::new(25.0, 30.0),
    ]
    .into()
}

fn native_meshes(scale: f32, alpha: u8, origin: Pos2, clip: Rect) -> Vec<egui::ClippedPrimitive> {
    let mut frame = Frame::default();
    frame.commands = vec![
        DrawCommand::PushClip(clip),
        DrawCommand::SelectionContour {
            points: contour(),
            color: Color::rgba(75, 130, 210, alpha),
        },
        DrawCommand::PopClip,
    ];
    native_frame_meshes(&frame, scale, origin)
}

fn native_frame_meshes(frame: &Frame, scale: f32, origin: Pos2) -> Vec<egui::ClippedPrimitive> {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(scale);
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::vec2(200.0, 160.0),
            )),
            ..Default::default()
        },
        |ui| super::super::paint_at(ui, frame, origin),
    );
    let primitives = ctx.tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
    output.textures_delta.clear();
    primitives
}

fn alpha_at(primitives: &[egui::ClippedPrimitive], point: Pos2) -> f32 {
    let mut result = 0.0;
    for primitive in primitives {
        if !primitive.clip_rect.contains(point) {
            continue;
        }
        let egui::epaint::Primitive::Mesh(mesh) = &primitive.primitive else {
            continue;
        };
        for indices in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [
                mesh.vertices[indices[0] as usize],
                mesh.vertices[indices[1] as usize],
                mesh.vertices[indices[2] as usize],
            ];
            let cross = |a: Vec2, b: Vec2| a.x * b.y - a.y * b.x;
            let denominator = cross(b.pos - a.pos, c.pos - a.pos);
            if denominator.abs() < 1e-7 {
                continue;
            }
            let wb = cross(point - a.pos, c.pos - a.pos) / denominator;
            let wc = cross(b.pos - a.pos, point - a.pos) / denominator;
            let wa = 1.0 - wb - wc;
            // All probes deliberately avoid shared triangle edges.
            if wa > 1e-6 && wb > 1e-6 && wc > 1e-6 {
                let alpha =
                    (wa * a.color.a() as f32 + wb * b.color.a() as f32 + wc * c.color.a() as f32)
                        / 255.0;
                result = alpha + result * (1.0 - alpha);
            }
        }
    }
    result * 255.0
}

#[test]
fn native_selection_contour_has_uniform_alpha_across_connected_rows_at_one_and_two_x() {
    for scale in [1.0, 2.0] {
        for alpha in [100, 255] {
            let primitives =
                native_meshes(scale, alpha, Pos2::ZERO, Rect::new(0.0, 0.0, 150.0, 100.0));
            for y in [
                10.37, 19.37, 29.37, 30.37, 39.37, 49.37, 50.37, 59.37, 69.37,
            ] {
                let actual = alpha_at(&primitives, Pos2::new(45.29, y));
                assert!(
                    (actual - alpha as f32).abs() < 0.02,
                    "{scale}× alpha={alpha} y={y}: {actual}"
                );
            }
            // Concave cutouts remain empty instead of becoming a convex hull.
            for point in [
                Pos2::new(15.29, 20.37),
                Pos2::new(90.29, 60.37),
                Pos2::new(105.29, 20.37),
            ] {
                assert_eq!(alpha_at(&primitives, point), 0.0);
            }
            let samples: Vec<_> = [0.1, 0.35, 0.7, 1.2]
                .into_iter()
                .map(|outside| alpha_at(&primitives, Pos2::new(45.29, 10.0 - outside / scale)))
                .collect();
            assert!(
                samples.windows(2).all(|pair| pair[0] > pair[1]),
                "only the perimeter fades: {samples:?}"
            );
            assert_eq!(samples[3], 0.0);
        }
    }
}

#[test]
fn native_selection_contour_preserves_host_origin_and_existing_clip() {
    let origin = Pos2::new(7.0, 13.0);
    let primitives = native_meshes(2.0, 100, origin, Rect::new(40.0, 20.0, 30.0, 30.0));
    assert!((alpha_at(&primitives, origin + egui::vec2(45.29, 30.37)) - 100.0).abs() < 0.02);
    assert_eq!(
        alpha_at(&primitives, origin + egui::vec2(35.29, 30.37)),
        0.0
    );
    assert_eq!(
        alpha_at(&primitives, origin + egui::vec2(45.29, 60.37)),
        0.0
    );
}

#[test]
fn native_selection_keeps_a_one_and_a_half_pixel_newline_connection_and_nearly_flat_joins() {
    for scale in [1.0, 2.0] {
        for origin in [Pos2::ZERO, Pos2::new(7.25, 13.5)] {
            let points: std::sync::Arc<[Point]> = vec![
                Point::new(10.0, 10.0),
                Point::new(100.0, 10.0),
                Point::new(100.0, 30.0),
                Point::new(11.5, 30.000_008),
                Point::new(11.5, 50.0),
                Point::new(100.0, 50.000_008),
                Point::new(100.0, 70.0),
                Point::new(10.0, 70.0),
            ]
            .into();
            let mut frame = Frame::default();
            frame.commands = vec![DrawCommand::SelectionContour {
                points,
                color: Color::rgba(75, 130, 210, 100),
            }];
            let primitives = native_frame_meshes(&frame, scale, origin);
            for y in [11.37, 29.37, 30.37, 39.37, 49.37, 50.37, 69.37] {
                let point = origin + egui::vec2(10.63, y);
                let actual = alpha_at(&primitives, point);
                assert!(
                    (actual - 100.0).abs() < 0.02,
                    "{scale}× origin={origin:?} {point:?}: {actual}"
                );
            }
            assert_eq!(
                alpha_at(&primitives, origin + egui::vec2(15.29, 40.37)),
                0.0
            );
        }
    }
}

#[test]
fn selection_mesh_rejects_invalid_contours_and_is_bounded_for_many_rows() {
    for invalid in [
        vec![],
        vec![Point::new(0.0, 0.0); 4],
        vec![
            Point::new(f32::NAN, 0.0),
            Point::new(20.0, 0.0),
            Point::new(10.0, 10.0),
        ],
        // A U shape is not Y-monotone and would otherwise fill its hole.
        vec![
            Point::new(0.0, 0.0),
            Point::new(30.0, 0.0),
            Point::new(30.0, 30.0),
            Point::new(20.0, 30.0),
            Point::new(20.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 30.0),
            Point::new(0.0, 30.0),
        ],
    ] {
        assert!(mesh(&invalid, Pos2::ZERO, Color32::from_white_alpha(100), 1.0).is_empty());
    }
    let points = contour();
    let normal = mesh(&points, Pos2::ZERO, Color32::from_white_alpha(100), 2.0);
    let reversed: Vec<_> = points.iter().copied().rev().collect();
    let reversed = mesh(&reversed, Pos2::ZERO, Color32::from_white_alpha(100), 2.0);
    assert!(normal.is_valid() && reversed.is_valid());
    assert_eq!(normal.vertices.len(), reversed.vertices.len());
    assert!(normal.vertices.len() <= points.len() * 6);
    assert!(normal.indices.len() <= points.len() * 12);
    assert!(mesh(
        &vec![Point::new(1.0, 1.0); MAX_POINTS + 1],
        Pos2::ZERO,
        Color32::WHITE,
        1.0
    )
    .is_empty());
}

#[test]
fn real_editor_rounded_selection_reaches_native_mesh_without_gaps_or_overdraw() {
    use limn_editor::{Selection, TextBuffer};
    use limn::{CodeEditor, Component, EditorController, Size, UiTree};
    let mut buffer =
        TextBuffer::new("prefix abcdefghijklmnop\nabcdefghijklmnopqrstuvwxy\nabcdefghij suffix\n")
            .unwrap();
    let end = buffer.snapshot().line_range(2).unwrap().start + 10;
    buffer
        .set_selection(Selection {
            anchor: 7,
            head: end,
        })
        .unwrap();
    let controller = EditorController::new(buffer);
    let mut tree = UiTree::new(CodeEditor::new("selection-editor", controller).render());
    tree.set_reduced_motion(true);
    let mut frame = tree.layout(Size::new(350.0, 150.0));
    frame.commands.retain(|command| {
        matches!(
            command,
            DrawCommand::SelectionContour { .. } | DrawCommand::PushClip(_) | DrawCommand::PopClip
        )
    });
    let points = frame
        .commands
        .iter()
        .find_map(|command| match command {
            DrawCommand::SelectionContour { points, .. } => Some(points),
            _ => None,
        })
        .expect("native editor emits a connected rounded selection");
    assert!(
        points.len() > 12,
        "actual curved contour, not the square test fixture"
    );
    let inside = |point: Pos2| {
        let mut xs = Vec::new();
        for (a, b) in points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .take(points.len())
        {
            if (a.y <= point.y && b.y > point.y) || (b.y <= point.y && a.y > point.y) {
                xs.push(a.x + (b.x - a.x) * (point.y - a.y) / (b.y - a.y));
            }
        }
        xs.sort_by(f32::total_cmp);
        xs.len() == 2 && point.x > xs[0] + 2.0 && point.x < xs[1] - 2.0
    };
    for scale in [1.0, 2.0] {
        let primitives = native_frame_meshes(&frame, scale, Pos2::ZERO);
        let mut sampled = 0;
        for y in 10..52 {
            for x in (70..200).step_by(11) {
                let point = Pos2::new(x as f32 + 0.29, y as f32 + 0.37);
                if inside(point) {
                    let actual = alpha_at(&primitives, point);
                    assert!(
                        (actual - 100.0).abs() < 0.03,
                        "{scale}× {point:?}: {actual}"
                    );
                    sampled += 1;
                }
            }
        }
        assert!(sampled > 40);
    }
}

