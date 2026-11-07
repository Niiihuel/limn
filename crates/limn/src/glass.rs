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
    /// Saturation multiplier, clamped to 0–3.
    pub saturation: f32,
    /// Darkening of the sampled scene, clamped to 0–1.
    pub dim: f32,
    /// Corner superellipse exponent, clamped to 2–6. Capsules stay circular.
    pub power: f32,
    /// Strength of refraction, dispersion and edge lighting, clamped to 0–1.
    pub intensity: f32,
    /// Bezel width in points; zero uses the element's corner radius.
    pub bezel: f32,
    /// Magnification of the center, clamped to 1–2.
    pub magnify: f32,
    /// Material presence, clamped to 0–1, scales the veil during a morph.
    pub presence: f32,
    /// Milky bloom during a morph, clamped to 0–1.
    pub bloom: f32,
    /// Light under a press, clamped to 0–1: a wide soft lift around `glow_at`, as aurora's
    /// interactive glass brightens. Set by [`Element::glass_press`](crate::Element::glass_press).
    pub glow: f32,
    /// Where the press is, as shares of the surface's width and height.
    pub glow_at: (f32, f32),
    /// Share of the pane's height, clamped to 0–1, over which it fades out toward its foot:
    /// with no tint, rim or bend, the soft scroll edge iOS blurs a page into under its bars.
    pub fade: f32,
}

impl GlassMaterial {
    /// A lightly blurred surface whose rim refracts as far as its shape allows.
    pub const fn new(tint: Color) -> Self {
        Self {
            tint,
            blur: 16.0,
            refraction: 32.0,
            dispersion: 0.5,
            saturation: 2.2,
            dim: 0.0,
            power: 4.0,
            intensity: 1.0,
            bezel: 0.0,
            magnify: 1.0,
            presence: 1.0,
            bloom: 0.0,
            glow: 0.0,
            glow_at: (0.5, 0.5),
            fade: 0.0,
        }
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

    /// Set saturation; ignore non-finite input and clamp to 0–3.
    pub fn saturation(mut self, value: f32) -> Self {
        if value.is_finite() { self.saturation = value.clamp(0.0, 3.0); }
        self
    }

    /// Set scene darkening; ignore non-finite input and clamp to 0–1.
    pub fn dim(mut self, value: f32) -> Self {
        if value.is_finite() { self.dim = value.clamp(0.0, 1.0); }
        self
    }

    /// Set corner power; ignore non-finite input and clamp to 2–6.
    pub fn power(mut self, value: f32) -> Self {
        if value.is_finite() { self.power = value.clamp(2.0, 6.0); }
        self
    }

    /// Set optical intensity; ignore non-finite input and clamp to 0–1.
    pub fn intensity(mut self, value: f32) -> Self {
        if value.is_finite() { self.intensity = value.clamp(0.0, 1.0); }
        self
    }

    /// Set bezel width in points; zero follows the radius, clamped to 0–64.
    pub fn bezel(mut self, value: f32) -> Self {
        if value.is_finite() { self.bezel = value.clamp(0.0, 64.0); }
        self
    }

    /// Set center magnification; ignore non-finite input and clamp to 1–2.
    pub fn magnify(mut self, value: f32) -> Self {
        if value.is_finite() { self.magnify = value.clamp(1.0, 2.0); }
        self
    }

    /// Set veil presence during morphing; ignore non-finite input and clamp to 0–1.
    pub fn presence(mut self, value: f32) -> Self {
        if value.is_finite() { self.presence = value.clamp(0.0, 1.0); }
        self
    }

    /// Fade the pane out over this share of its height toward its foot; ignore non-finite
    /// input and clamp to 0–1.
    pub fn fade(mut self, value: f32) -> Self {
        if value.is_finite() { self.fade = value.clamp(0.0, 1.0); }
        self
    }

    /// Set material bloom; ignore non-finite input and clamp to 0–1.
    pub fn bloom(mut self, value: f32) -> Self {
        if value.is_finite() { self.bloom = value.clamp(0.0, 1.0); }
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
            saturation: bound(self.saturation, 3.0),
            dim: bound(self.dim, 1.0),
            power: bound(self.power, 6.0).max(2.0),
            intensity: bound(self.intensity, 1.0),
            bezel: bound(self.bezel, 64.0),
            magnify: bound(self.magnify, 2.0).max(1.0),
            presence: bound(self.presence, 1.0),
            bloom: bound(self.bloom, 1.0),
            glow: bound(self.glow, 1.0),
            glow_at: (bound(self.glow_at.0, 1.0), bound(self.glow_at.1, 1.0)),
            fade: bound(self.fade, 1.0),
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
