//! A whole interface palette derived from three colors.
//!
//! The scale follows Warp's theme model: surfaces are the background with a
//! little foreground mixed in, text is the foreground at a fixed strength, and
//! the accent only ever appears diluted. Changing three inputs re-tints
//! everything, and no token can drift out of step with the others.

use crate::{Color, ComponentPalette};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Theme {
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
}

/// Foreground share, in percent, of each surface level.
const LEVELS: [u32; 7] = [5, 10, 15, 20, 40, 60, 90];

const fn mix(base: Color, top: Color, percent: u32) -> Color {
    const fn channel(base: u8, top: u8, percent: u32) -> u8 {
        ((base as u32 * (100 - percent) + top as u32 * percent + 50) / 100) as u8
    }
    Color::rgb(
        channel(base.r, top.r, percent),
        channel(base.g, top.g, percent),
        channel(base.b, top.b, percent),
    )
}

const fn alpha(color: Color, percent: u32) -> Color {
    Color::rgba(color.r, color.g, color.b, ((255 * percent + 50) / 100) as u8)
}

impl Theme {
    /// Neutral dark grey with a calm blue accent.
    pub const DARK: Self = Self {
        background: Color::rgb(30, 31, 34),
        foreground: Color::rgb(255, 255, 255),
        accent: Color::rgb(25, 170, 216),
    };

    /// Opaque surface that stands out from the background; `level` runs from
    /// 1 (barely) to 7 (almost foreground) and is clamped to that range.
    pub const fn surface(&self, level: usize) -> Color {
        let index = if level < 1 { 0 } else if level > 7 { 6 } else { level - 1 };
        mix(self.background, self.foreground, LEVELS[index])
    }

    /// Hairlines between and around surfaces.
    pub const fn outline(&self) -> Color {
        self.surface(2)
    }

    pub const fn text(&self) -> Color {
        alpha(self.foreground, 90)
    }

    /// Secondary labels and metadata.
    pub const fn text_sub(&self) -> Color {
        alpha(self.foreground, 60)
    }

    pub const fn text_hint(&self) -> Color {
        alpha(self.foreground, 40)
    }

    pub const fn text_disabled(&self) -> Color {
        alpha(self.foreground, 20)
    }

    /// Translucent feedback that works over any surface.
    pub const fn hover(&self) -> Color {
        alpha(self.foreground, 10)
    }

    pub const fn pressed(&self) -> Color {
        alpha(self.foreground, 20)
    }

    /// Selected or active fill: the accent, diluted into the background.
    pub const fn accent_surface(&self) -> Color {
        mix(self.background, self.accent, 40)
    }

    pub const fn accent_surface_strong(&self) -> Color {
        mix(self.background, self.accent, 60)
    }

    /// Accent-tinted text and icons, readable on the background.
    pub const fn accent_text(&self) -> Color {
        mix(self.foreground, self.accent, 60)
    }

    pub const fn palette(&self) -> ComponentPalette {
        ComponentPalette {
            surface: self.surface(1),
            raised: self.surface(2),
            hover: self.surface(3),
            selected: self.surface(3),
            border: self.outline(),
            text: self.text(),
            muted: self.text_sub(),
            shadow: Color::rgba(0, 0, 0, 120),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surfaces_step_evenly_from_background_towards_foreground() {
        let theme = Theme {
            background: Color::rgb(0, 0, 0),
            foreground: Color::rgb(200, 100, 255),
            accent: Color::rgb(0, 200, 100),
        };
        assert_eq!(theme.surface(1), Color::rgb(10, 5, 13));
        assert_eq!(theme.surface(2), Color::rgb(20, 10, 26));
        assert_eq!(theme.surface(7), Color::rgb(180, 90, 230));
        assert_eq!(theme.surface(0), theme.surface(1));
        assert_eq!(theme.surface(99), theme.surface(7));
        assert_eq!(theme.outline(), theme.surface(2));
        assert_eq!(theme.accent_surface(), Color::rgb(0, 80, 40));
    }

    #[test]
    fn text_keeps_the_foreground_hue_at_falling_strength() {
        let theme = Theme::DARK;
        let alphas = [theme.text(), theme.text_sub(), theme.text_hint(), theme.text_disabled()]
            .map(|color| color.a);
        assert_eq!(alphas, [230, 153, 102, 51]);
        assert_eq!(theme.hover(), Color::rgba(255, 255, 255, 26));
        assert_eq!(theme.palette().border, theme.outline());
    }
}
