//! A continuous corner cutout: only the circular boundary is antialiased.
//!
//! Painting thin horizontal rectangles here makes egui antialias every strip,
//! exposing the child surface between strips. A single mesh avoids those seams.

use eframe::egui::{self, Color32, Pos2, Rect};
use lienzo::CornerRadii;

pub(super) fn mesh(
    area: Rect,
    color: Color32,
    radii: CornerRadii,
    pixels_per_point: f32,
) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    if !area.is_finite() || !area.is_positive() || color.a() == 0 {
        return mesh;
    }
    let scale = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
        pixels_per_point
    } else {
        1.0
    };
    let maximum = area.width().min(area.height()) * 0.5;
    for (outer, direction, radius) in [
        (area.left_top(), egui::vec2(1.0, 1.0), radii.top_left),
        (area.right_top(), egui::vec2(-1.0, 1.0), radii.top_right),
        (
            area.right_bottom(),
            egui::vec2(-1.0, -1.0),
            radii.bottom_right,
        ),
        (area.left_bottom(), egui::vec2(1.0, -1.0), radii.bottom_left),
    ] {
        if !radius.is_finite() || radius <= 0.0 {
            continue;
        }
        let radius = radius.min(maximum);
        // Keep the circular approximation below a tenth of a physical pixel
        // for normal UI radii, with a hard bound for malformed/huge surfaces.
        let segments = ((radius * scale).sqrt() * 2.0).ceil().clamp(4.0, 128.0) as u32;
        let inner_radius = (radius - scale.recip()).max(0.0);
        let start = mesh.vertices.len() as u32;
        let vertex = |pos, color| egui::epaint::Vertex {
            pos,
            uv: egui::epaint::WHITE_UV,
            color,
        };
        mesh.vertices.push(vertex(outer, color));
        for index in 0..=segments {
            let angle = index as f32 / segments as f32 * std::f32::consts::FRAC_PI_2;
            let (sin, cos) = angle.sin_cos();
            // Exact endpoints avoid float slivers at the outer surface edges.
            let radial = if index == 0 {
                egui::vec2(0.0, -1.0)
            } else if index == segments {
                egui::vec2(-1.0, 0.0)
            } else {
                egui::vec2(-sin, -cos)
            };
            let point = |r: f32| {
                let local = egui::vec2(radius, radius) + radial * r;
                Pos2::new(
                    outer.x + direction.x * local.x,
                    outer.y + direction.y * local.y,
                )
            };
            mesh.vertices.push(vertex(point(radius), color));
            mesh.vertices
                .push(vertex(point(inner_radius), Color32::TRANSPARENT));
        }
        for index in 0..segments {
            let arc = start + 1 + index * 2;
            // Opaque fan plus one physical pixel of fringe toward the rounded
            // content. Internal fan/strip edges have no additional feathering.
            mesh.indices.extend_from_slice(&[
                start,
                arc,
                arc + 2,
                arc,
                arc + 1,
                arc + 2,
                arc + 2,
                arc + 1,
                arc + 3,
            ]);
        }
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use lienzo::{Color, DrawCommand, Frame};

    // Sample the actual tessellated native triangles. This catches the old
    // half-pixel rect implementation, whose interior was only 75% opaque at 1×.
    fn native_meshes(scale: f32, alpha: u8) -> Vec<egui::ClippedPrimitive> {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(scale);
        let mut frame = Frame::default();
        frame.commands = vec![
            DrawCommand::PushClip(lienzo::Rect::new(8.0, 8.0, 80.0, 60.0)),
            DrawCommand::CornerMask {
                rect: lienzo::Rect::new(8.0, 8.0, 80.0, 60.0),
                color: Color {
                    r: 60,
                    g: 90,
                    b: 120,
                    a: alpha,
                },
                radii: CornerRadii::same(12.0),
            },
            DrawCommand::PopClip,
        ];
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 80.0))),
                ..Default::default()
            },
            |ui| super::super::paint_at(ui, &frame, Pos2::ZERO),
        );
        let primitives =
            ctx.tessellate(std::mem::take(&mut output.shapes), output.pixels_per_point);
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
                let cross = |a: egui::Vec2, b: egui::Vec2| a.x * b.y - a.y * b.x;
                let denominator = cross(b.pos - a.pos, c.pos - a.pos);
                if denominator.abs() < 1e-6 {
                    continue;
                }
                let wb = cross(point - a.pos, c.pos - a.pos) / denominator;
                let wc = cross(b.pos - a.pos, point - a.pos) / denominator;
                let wa = 1.0 - wb - wc;
                // Probe strictly inside a triangle, away from shared edges.
                if wa > 1e-6 && wb > 1e-6 && wc > 1e-6 {
                    let source = (wa * a.color.a() as f32
                        + wb * b.color.a() as f32
                        + wc * c.color.a() as f32)
                        / 255.0;
                    result = source + result * (1.0 - source);
                }
            }
        }
        result * 255.0
    }

    #[test]
    fn native_corner_mask_has_no_internal_antialias_seams_at_one_and_two_x() {
        for scale in [1.0, 2.0] {
            for alpha in [128, 255] {
                let primitives = native_meshes(scale, alpha);
                for (x, y, sx, sy) in [
                    (8.0, 8.0, 1.0, 1.0),
                    (88.0, 8.0, -1.0, 1.0),
                    (88.0, 68.0, -1.0, -1.0),
                    (8.0, 68.0, 1.0, -1.0),
                ] {
                    for (px, py) in [(0.5, 1.5), (1.5, 2.5), (2.5, 0.5)] {
                        let point = Pos2::new(x + sx * px / scale, y + sy * py / scale);
                        let actual = alpha_at(&primitives, point);
                        assert!(
                            (actual - alpha as f32).abs() < 0.01,
                            "scale={scale} alpha={alpha} point={point:?}: {actual}"
                        );
                    }
                }
                assert_eq!(alpha_at(&primitives, Pos2::new(40.0, 40.0)), 0.0);
                assert_eq!(alpha_at(&primitives, Pos2::new(7.5, 9.5)), 0.0);
                let direction = egui::vec2(-0.6, -0.8);
                let center = Pos2::new(20.0, 20.0);
                let samples: Vec<_> = [0.1, 0.4, 0.7, 1.1]
                    .into_iter()
                    .map(|inset| alpha_at(&primitives, center + direction * (12.0 - inset / scale)))
                    .collect();
                assert!(
                    samples.windows(2).all(|pair| pair[0] > pair[1]),
                    "arc fringe must fade monotonically: {samples:?}"
                );
                assert_eq!(samples[3], 0.0);
            }
        }
    }

    #[test]
    fn native_corner_mask_mesh_is_bounded_and_honors_independent_radii() {
        let area = Rect::from_min_size(Pos2::new(0.25, 0.5), egui::vec2(100.0, 80.0));
        let radii = CornerRadii {
            top_left: 0.0,
            top_right: 8.0,
            bottom_right: f32::INFINITY,
            bottom_left: 10_000.0,
        };
        let mesh = mesh(area, Color32::from_white_alpha(128), radii, 2.0);
        assert!(mesh.is_valid());
        assert!(mesh.vertices.len() <= 4 * (1 + 2 * 129));
        assert!(mesh.vertices.iter().all(|vertex| area.contains(vertex.pos)));
        assert!(mesh
            .vertices
            .iter()
            .all(|vertex| vertex.color.a() == 0 || vertex.color.a() == 128));
        assert!(mesh.vertices.iter().all(
            |vertex| vertex.pos.x >= area.right() - 8.0 || vertex.pos.y >= area.bottom() - 40.0
        ));
    }
}
