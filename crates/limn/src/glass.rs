//! A backend-neutral material that samples the scene behind a surface.

use crate::Color;

/// Optical properties of a rounded translucent surface. Distances are in
/// logical pixels. A renderer without backdrop sampling can use `tint` as
/// its fallback; a host must supply the scene it wants to refract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassMaterial {
    /// Veil blended over the sampled scene; alpha controls its strength.
    pub tint: Color,
    /// Backdrop blur radius, from 0 to 64 logical pixels. The radius is twice
    /// the Gaussian deviation, so CSS `blur(16px)` is 32 here. Apple's web
    /// glass uses that; its native glass, which refracts, blurs far less.
    pub blur: f32,
    /// Displacement at the very edge, from 0 to 32 logical pixels. A renderer
    /// limits it to the corner radius, where the rim mirrors the middle of
    /// the shape as Apple's native glass does.
    pub refraction: f32,
    /// Chromatic separation, from 0 to 4 logical pixels.
    pub dispersion: f32,
}

impl GlassMaterial {
    /// A lightly blurred surface whose rim refracts as far as its shape allows.
    pub const fn new(tint: Color) -> Self {
        Self {
            tint,
            blur: 8.0,
            refraction: 32.0,
            dispersion: 0.5,
        }
    }

    /// The veil Apple's web apps use in a dark theme: `#262628` at 60%.
    pub const fn dark() -> Self {
        Self::new(Color::rgba(38, 38, 40, 153))
    }

    /// The veil Apple's web apps use in a light theme: `#f5f5f7` at 55%.
    pub const fn light() -> Self {
        Self::new(Color::rgba(245, 245, 247, 140))
    }

    /// Set backdrop blur; ignore non-finite input and clamp to 0–64.
    pub fn blur(mut self, blur: f32) -> Self {
        if blur.is_finite() {
            self.blur = blur.clamp(0.0, 64.0);
        }
        self
    }

    /// Set edge displacement; ignore non-finite input and clamp to 0–32.
    pub fn refraction(mut self, refraction: f32) -> Self {
        if refraction.is_finite() {
            self.refraction = refraction.clamp(0.0, 32.0);
        }
        self
    }

    /// Set color separation; ignore non-finite input and clamp to 0–4.
    pub fn dispersion(mut self, dispersion: f32) -> Self {
        if dispersion.is_finite() {
            self.dispersion = dispersion.clamp(0.0, 4.0);
        }
        self
    }

    /// Validate public fields before painting. Non-finite values become zero.
    pub fn normalized(self) -> Self {
        let bound = |value: f32, max: f32| {
            if value.is_finite() {
                value.clamp(0.0, max)
            } else {
                0.0
            }
        };
        Self {
            blur: bound(self.blur, 64.0),
            refraction: bound(self.refraction, 32.0),
            dispersion: bound(self.dispersion, 4.0),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Element, Point, Size, UiTree};

    #[test]
    fn material_bounds_non_finite_and_extreme_input() {
        let glass = GlassMaterial::new(Color::WHITE)
            .blur(f32::NAN)
            .refraction(f32::INFINITY)
            .dispersion(-2.0);
        assert_eq!(glass.blur, GlassMaterial::new(Color::WHITE).blur);
        assert_eq!(
            glass.refraction,
            GlassMaterial::new(Color::WHITE).refraction
        );
        assert_eq!(glass.dispersion, 0.0);
        let glass = GlassMaterial {
            blur: 900.0,
            refraction: -2.0,
            dispersion: f32::NAN,
            ..glass
        }
        .normalized();
        assert_eq!(
            (glass.blur, glass.refraction, glass.dispersion),
            (64.0, 0.0, 0.0)
        );
    }

    #[test]
    fn glass_paints_after_shadow_before_content() {
        let material = GlassMaterial::new(Color::rgba(120, 130, 140, 60));
        let element = Element::panel(Element::text("foreground"))
            .glass(material)
            .radius(16.0)
            .shadow(Color::WHITE, Point::new(0.0, 2.0), 8.0, 0.0);
        let frame = UiTree::new(element).layout(Size::new(200.0, 60.0));
        assert!(matches!(frame.commands[0], DrawCommand::Shadow { .. }));
        assert!(
            matches!(frame.commands[1], DrawCommand::Glass { radius: 16.0, material: m, opacity: 1.0, .. } if m == material)
        );
        assert!(matches!(frame.commands[2], DrawCommand::Text { .. }));
    }
}
