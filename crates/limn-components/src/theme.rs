//! A whole interface palette derived from three colors.
//!
//! The scale follows Warp's theme model: surfaces are the background with a
//! little foreground mixed in, text is the foreground at a fixed strength, and
//! the accent only ever appears diluted. Changing three inputs re-tints
//! everything, and no token can drift out of step with the others.

use crate::{Color, ComponentPalette, ControlColors, Element, Scheme};

/// The three colors a whole palette is derived from.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Theme {
    /// What the surfaces are built on. Each surface level mixes a share of the
    /// foreground into it.
    pub background: Color,
    /// The ink. Text is this color at 90, 60, 40 or 20 % opacity, and the hover and
    /// pressed veils are it at 10 and 20 %.
    pub foreground: Color,
    /// The color of selection and emphasis, mixed into the background for accent
    /// fills and into the foreground for accent text.
    pub accent: Color,
}

/// Foreground share, in percent, of each surface level.
const LEVELS: [u32; 7] = [5, 10, 15, 20, 40, 60, 90];

pub(crate) const fn mix(base: Color, top: Color, percent: u32) -> Color {
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

    /// Primary text: the foreground at 90% opacity.
    pub const fn text(&self) -> Color {
        alpha(self.foreground, 90)
    }

    /// Secondary labels and metadata.
    pub const fn text_sub(&self) -> Color {
        alpha(self.foreground, 60)
    }

    /// Text a step quieter than [`text_sub`](Self::text_sub): the foreground at 40%
    /// opacity.
    pub const fn text_hint(&self) -> Color {
        alpha(self.foreground, 40)
    }

    /// Text of disabled content: the foreground at 20% opacity.
    pub const fn text_disabled(&self) -> Color {
        alpha(self.foreground, 20)
    }

    /// Translucent feedback that works over any surface.
    pub const fn hover(&self) -> Color {
        alpha(self.foreground, 10)
    }

    /// The veil of a pressed control: the foreground at 20% opacity. It is also the
    /// palette's `selected` fill.
    pub const fn pressed(&self) -> Color {
        alpha(self.foreground, 20)
    }

    /// Selected or active fill: the accent, diluted into the background.
    pub const fn accent_surface(&self) -> Color {
        mix(self.background, self.accent, 40)
    }

    /// A stronger accent fill: 60% of the accent in the background, where
    /// [`accent_surface`](Self::accent_surface) has 40%.
    pub const fn accent_surface_strong(&self) -> Color {
        mix(self.background, self.accent, 60)
    }

    /// Accent-tinted text and icons, readable on the background.
    pub const fn accent_text(&self) -> Color {
        mix(self.foreground, self.accent, 60)
    }

    /// The one pointer-feedback treatment: hover veil, pressed veil, control
    /// radius and transition. Every button, icon button and clickable goes
    /// through here; nothing sets a hover color of its own.
    pub fn interactive(&self, element: Element) -> Element {
        self.palette().interactive_surface(element)
    }

    /// An interactive element in its chosen state, such as the active tab or
    /// a selected row. It holds `fill` under the pointer instead of flashing
    /// back to the hover veil.
    pub fn active(&self, element: Element, fill: Color) -> Element {
        self.interactive(element)
            .background(fill)
            .hover_background(fill)
    }

    /// The component palette of this theme: surface levels 1 and 2, the hover and pressed
    /// veils, the outline as border, and the primary and secondary text.
    pub const fn palette(&self) -> ComponentPalette {
        ComponentPalette {
            surface: self.surface(1),
            raised: self.surface(2),
            // Veils, not fills: the same feedback reads on every surface.
            hover: self.hover(),
            selected: self.pressed(),
            border: self.outline(),
            text: self.text(),
            muted: self.text_sub(),
            shadow: Color::rgba(0, 0, 0, 120),
        }
    }

    /// The same colors by role, for a host that derives its palettes from a
    /// [`Scheme`]. `scheme().palette()` equals [`palette`](Self::palette).
    pub const fn scheme(&self) -> Scheme {
        let palette = self.palette();
        Scheme {
            canvas: palette.surface,
            panel: self.background,
            raised: palette.raised,
            hover: palette.hover,
            selected: palette.selected,
            border: palette.border,
            text: palette.text,
            muted: palette.muted,
            // Translucent ink reads the same on every surface.
            muted_on_raised: palette.muted,
            menu: palette.raised,
            menu_hover: palette.hover,
            menu_muted: palette.muted,
            shadow: palette.shadow,
            // Opaque steps: a veil in place of a fill would erase the control.
            control: ControlColors {
                surface: self.surface(3),
                hover: self.surface(4),
                pressed: mix(self.background, self.foreground, 25),
                border: self.surface(4),
                text: palette.text,
            },
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

    #[test]
    fn every_interactive_element_gets_the_same_feedback() {
        let theme = Theme::DARK;
        let styles = [
            theme.interactive(Element::button("a", "a")),
            theme.interactive(Element::icon_button("b", crate::Icon::Add, "")),
            theme.interactive(Element::clickable("c", Element::text("c"))),
        ]
        .map(|element| {
            let style = element.style;
            (style.hover_background, style.pressed_background, style.radius)
        });
        assert_eq!(styles[0], (Some(theme.hover()), Some(theme.pressed()), 4.0));
        assert!(styles.iter().all(|style| *style == styles[0]));

        let fill = theme.surface(4);
        let active = theme.active(Element::clickable("d", Element::text("d")), fill).style;
        assert_eq!((active.background, active.hover_background), (Some(fill), Some(fill)));
        // Passive content is left alone.
        assert_eq!(theme.interactive(Element::text("e")).style.hover_background, None);
    }
}
