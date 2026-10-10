//! One translucent fill for a continuous editor selection. Only the perimeter
//! receives antialiasing; joins between selected lines never accumulate alpha.
use eframe::egui::{self, Color32, Pos2, Vec2};
use limn::Point;

const MAX_POINTS: usize = 16_384;

#[derive(Clone, Copy)]
struct Edge {
    top: Pos2,
    bottom: Pos2,
}
impl Edge {
    fn x_at(self, y: f64) -> f32 {
        let fraction =
            (y - f64::from(self.top.y)) / (f64::from(self.bottom.y) - f64::from(self.top.y));
        (f64::from(self.top.x) + fraction * (f64::from(self.bottom.x) - f64::from(self.top.x)))
            as f32
    }
}

fn vertex(pos: Pos2, color: Color32) -> egui::epaint::Vertex {
    egui::epaint::Vertex {
        pos,
        uv: egui::epaint::WHITE_UV,
        color,
    }
}

pub(super) fn mesh(
    points: &[Point],
    origin: Pos2,
    color: Color32,
    pixels_per_point: f32,
) -> egui::Mesh {
    let mut result = egui::Mesh::default();
    if points.len() < 3 || points.len() > MAX_POINTS || color.a() == 0 || !origin.is_finite() {
        return result;
    }
    let mut points: Vec<_> = points
        .iter()
        .map(|point| origin + egui::vec2(point.x, point.y))
        .collect();
    if points.iter().any(|point| !point.is_finite()) {
        return result;
    }
    points.dedup();
    if points.first() == points.last() {
        points.pop();
    }
    if points.len() < 3 {
        return result;
    }
    let signed_area = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| f64::from(a.x) * f64::from(b.y) - f64::from(b.x) * f64::from(a.y))
        .sum::<f64>();
    if signed_area.abs() < 1e-8 {
        return result;
    }
    if signed_area < 0.0 {
        points.reverse();
    }
    let mut levels: Vec<_> = points.iter().map(|point| point.y).collect();
    levels.sort_by(f32::total_cmp);
    levels.dedup();
    let mut edges: Vec<_> = points
        .iter()
        .copied()
        .zip(points.iter().copied().cycle().skip(1))
        .take(points.len())
        .filter_map(|(a, b)| {
            if a.y == b.y {
                None
            } else if a.y < b.y {
                Some(Edge { top: a, bottom: b })
            } else {
                Some(Edge { top: b, bottom: a })
            }
        })
        .collect();
    edges.sort_by(|a, b| a.top.y.total_cmp(&b.top.y));
    let mut next = 0;
    let mut active = Vec::<Edge>::with_capacity(2);
    // A Y-monotone contour intersects every open horizontal band twice. All
    // internal triangles carry the exact same color, with no feathered seams.
    for band in levels.windows(2) {
        let (top, bottom) = (band[0], band[1]);
        active.retain(|edge| edge.bottom.y > top);
        while next < edges.len() && edges[next].top.y <= top {
            if edges[next].bottom.y > top {
                active.push(edges[next]);
            }
            next += 1;
        }
        if active.len() != 2 {
            return egui::Mesh::default();
        }
        let middle = f64::from(top) + (f64::from(bottom) - f64::from(top)) * 0.5;
        if active[0].x_at(middle) > active[1].x_at(middle) {
            active.swap(0, 1);
        }
        let (left, right) = (active[0], active[1]);
        let quad = [
            Pos2::new(left.x_at(f64::from(top)), top),
            Pos2::new(right.x_at(f64::from(top)), top),
            Pos2::new(right.x_at(f64::from(bottom)), bottom),
            Pos2::new(left.x_at(f64::from(bottom)), bottom),
        ];
        if quad.iter().any(|point| !point.is_finite())
            || quad[0].x > quad[1].x + 1e-4
            || quad[3].x > quad[2].x + 1e-4
        {
            return egui::Mesh::default();
        }
        let first = result.vertices.len() as u32;
        result
            .vertices
            .extend(quad.map(|point| vertex(point, color)));
        result.indices.extend_from_slice(&[
            first,
            first + 1,
            first + 2,
            first,
            first + 2,
            first + 3,
        ]);
    }
    let pixel = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
        pixels_per_point.recip()
    } else {
        1.0
    };
    let normals: Vec<_> = points
        .iter()
        .copied()
        .zip(points.iter().copied().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| {
            let edge = b - a;
            let length = f64::from(edge.x).hypot(f64::from(edge.y));
            if length == 0.0 {
                Vec2::ZERO
            } else {
                egui::vec2(
                    (f64::from(edge.y) / length) as f32,
                    (-f64::from(edge.x) / length) as f32,
                )
            }
        })
        .collect();
    let first = result.vertices.len() as u32;
    for (index, point) in points.iter().copied().enumerate() {
        let previous = normals[(index + normals.len() - 1) % normals.len()];
        let current = normals[index];
        let sum = previous + current;
        let denominator = sum.dot(current);
        let outward = if denominator > 1e-4 {
            sum * (pixel / denominator)
        } else {
            current * pixel
        };
        // Rounded selection geometry normally has gentle joins. Bound the
        // fallback for sharp/degenerate callers instead of emitting a spike.
        let outward = if outward.length_sq() > 4.0 * pixel * pixel {
            outward.normalized() * (2.0 * pixel)
        } else {
            outward
        };
        result.vertices.push(vertex(point, color));
        result
            .vertices
            .push(vertex(point + outward, Color32::TRANSPARENT));
    }
    for index in 0..points.len() as u32 {
        let a = first + index * 2;
        let b = first + ((index + 1) % points.len() as u32) * 2;
        result
            .indices
            .extend_from_slice(&[a, b, b + 1, a, b + 1, a + 1]);
    }
    result
}

#[cfg(test)]
#[path = "selection_contour_render_tests.rs"]
mod tests;
