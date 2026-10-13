//! Geometry uses logical pixels. The host handles device scaling.

/// A position in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Horizontal coordinate, growing to the right.
    pub x: f32,
    /// Vertical coordinate, growing downwards.
    pub y: f32,
}

impl Point {
    /// A point at `x`, `y`.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A width and a height in logical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    /// Horizontal extent.
    pub width: f32,
    /// Vertical extent.
    pub height: f32,
}

impl Size {
    /// A size of `width` by `height`.
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// A rectangle in logical pixels. `x` and `y` are its top left corner.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// The left edge.
    pub x: f32,
    /// The top edge; `y + height` is the bottom.
    pub y: f32,
    /// From the left edge to the right one, which is itself outside the rectangle.
    pub width: f32,
    /// From the top edge to the bottom one, which is itself outside the rectangle.
    pub height: f32,
}

impl Rect {
    /// A rectangle from its top left corner and its size.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Whether the point is inside. The left and top edges are inside; the right
    /// and bottom edges are not.
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x < self.x + self.width
            && point.y < self.y + self.height
    }

    /// The part two rectangles share, or `None` when they do not overlap.
    /// Rectangles that only touch along an edge do not overlap.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        (right > x && bottom > y).then(|| Self::new(x, y, right - x, bottom - y))
    }

    /// Move every edge inwards by `amount`. The width and height stop at zero,
    /// and a negative amount is read as zero.
    pub fn inset(self, amount: f32) -> Self {
        let inset = amount.max(0.0);
        Self::new(
            self.x + inset,
            self.y + inset,
            (self.width - inset * 2.0).max(0.0),
            (self.height - inset * 2.0).max(0.0),
        )
    }
}

/// An RGBA color with 8 bits per channel. An `a` of 0 is transparent and 255
/// is opaque.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Color {
    /// Red, from 0 to 255.
    pub r: u8,
    /// Green, from 0 to 255.
    pub g: u8,
    /// Blue, from 0 to 255.
    pub b: u8,
    /// Alpha: 0 is transparent and 255 is opaque.
    pub a: u8,
}

impl Color {
    /// Opaque white.
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Black with an alpha of zero: nothing is painted.
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);

    /// An opaque color.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// A color with its alpha, from 0 (transparent) to 255 (opaque).
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangles_have_half_open_edges() {
        let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
        assert!(rect.contains(Point::new(10.0, 20.0)));
        assert!(!rect.contains(Point::new(40.0, 20.0)));
        assert!(!rect.contains(Point::new(10.0, 60.0)));
    }

    #[test]
    fn intersection_and_inset_never_make_negative_size() {
        let a = Rect::new(0.0, 0.0, 20.0, 20.0);
        let b = Rect::new(15.0, 5.0, 10.0, 10.0);
        assert_eq!(a.intersection(b), Some(Rect::new(15.0, 5.0, 5.0, 10.0)));
        assert_eq!(a.inset(15.0), Rect::new(15.0, 15.0, 0.0, 0.0));
        assert_eq!(a.intersection(Rect::new(20.0, 0.0, 1.0, 1.0)), None);
    }
}
