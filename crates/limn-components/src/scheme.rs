//! Explicit color roles for a host that chooses every value itself.
//!
//! A [`Theme`](crate::Theme) derives its colors from three; a `Scheme` names
//! each one. Neither reaches a component: both only produce the
//! [`ComponentPalette`] a component takes.

use crate::components::{darker, lighter};
use crate::{Color, ComponentPalette};

/// How far [`Scheme::step`] moves each channel.
const STEP: u8 = 8;

/// The colors of a filled control that sits on a raised surface, such as a
/// button on a card.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ControlColors {
    /// The control's fill at rest.
    pub surface: Color,
    /// The fill under the pointer.
    pub hover: Color,
    /// The fill while pressed.
    pub pressed: Color,
    /// The control's outline.
    pub border: Color,
    /// The label and icon on the fill.
    pub text: Color,
}

/// The role of the color an action is painted on. The caller states it;
/// a scheme never guesses a role from a color's value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Surface {
    /// Level with the page: the canvas, a panel, or nothing at all.
    Flat(Color),
    /// Lifted above the page: a card, a sidebar, a chosen row.
    Raised(Color),
    /// A statement color, such as a primary or destructive fill, which holds
    /// under the pointer.
    Solid(Color),
}

/// One color per role. Build it as a `const` and derive every palette from
/// it, so two palettes cannot disagree about a shared color.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Scheme {
    /// The page behind everything.
    pub canvas: Color,
    /// Docks and side panels, level with the page.
    pub panel: Color,
    /// Cards, sidebars and inputs.
    pub raised: Color,
    /// Pointer feedback on a flat surface.
    pub hover: Color,
    /// A pressed control or a chosen item.
    pub selected: Color,
    /// Outlines and dividers, on every surface.
    pub border: Color,
    /// Primary ink.
    pub text: Color,
    /// Secondary ink on a flat surface.
    pub muted: Color,
    /// Secondary ink on a raised surface, where `muted` would read too faint.
    pub muted_on_raised: Color,
    /// Floating menus and popovers.
    pub menu: Color,
    /// The row under the pointer, and the chosen row, in a menu.
    pub menu_hover: Color,
    /// Secondary ink in a menu.
    pub menu_muted: Color,
    /// The shadow under anything that floats.
    pub shadow: Color,
    /// Filled controls that sit on `raised`.
    pub control: ControlColors,
}

impl Scheme {
    /// The palette of the page: components on the canvas.
    pub const fn palette(&self) -> ComponentPalette {
        ComponentPalette {
            surface: self.canvas,
            raised: self.raised,
            hover: self.hover,
            selected: self.selected,
            border: self.border,
            text: self.text,
            muted: self.muted,
            shadow: self.shadow,
        }
    }

    /// The palette of a floating menu. Its chosen row holds the hover fill.
    pub const fn menu(&self) -> ComponentPalette {
        ComponentPalette {
            surface: self.menu,
            raised: self.menu,
            hover: self.menu_hover,
            selected: self.menu_hover,
            border: self.border,
            text: self.text,
            muted: self.menu_muted,
            shadow: self.shadow,
        }
    }

    /// The palette of content laid directly on a raised surface.
    pub const fn card(&self) -> ComponentPalette {
        ComponentPalette {
            surface: self.raised,
            muted: self.muted_on_raised,
            ..self.palette()
        }
    }

    /// The palette of a filled control on a raised surface.
    pub const fn on_raised(&self) -> ComponentPalette {
        ComponentPalette {
            surface: self.control.surface,
            raised: self.control.surface,
            hover: self.control.hover,
            selected: self.control.pressed,
            border: self.control.border,
            text: self.control.text,
            muted: self.muted,
            shadow: self.shadow,
        }
    }

    /// The palette of an action painted on `surface` in `ink`. A flat surface
    /// takes the scheme's hover, a raised one moves a [`step`](Self::step)
    /// from its own color, and a solid one holds.
    pub const fn on(&self, surface: Surface, ink: Color) -> ComponentPalette {
        let (color, hover, selected) = match surface {
            Surface::Flat(color) => (color, self.hover, self.selected),
            Surface::Raised(color) => (color, self.step(color), self.selected),
            Surface::Solid(color) => (color, color, color),
        };
        ComponentPalette {
            surface: color,
            raised: color,
            hover,
            selected,
            border: self.border,
            text: ink,
            muted: self.muted,
            shadow: self.shadow,
        }
    }

    /// `color` moved one step toward the ink: lighter in a dark scheme,
    /// darker in a light one. Alpha is kept, and a channel stops at its limit.
    pub const fn step(&self, color: Color) -> Color {
        const fn sum(color: Color) -> u16 {
            color.r as u16 + color.g as u16 + color.b as u16
        }
        if sum(self.text) > sum(self.canvas) {
            lighter(color, STEP)
        } else {
            darker(color, STEP)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Theme;

    const LIGHT: Theme = Theme {
        background: Color::rgb(255, 255, 255),
        foreground: Color::rgb(24, 26, 31),
        accent: Color::rgb(9, 105, 218),
    };

    #[test]
    fn a_theme_scheme_yields_the_theme_palette() {
        for theme in [Theme::DARK, LIGHT] {
            let scheme = theme.scheme();
            assert_eq!(scheme.palette(), theme.palette());
            assert_eq!(scheme.card().surface, scheme.raised);
            assert_eq!(scheme.card().muted, scheme.muted_on_raised);
            let menu = scheme.menu();
            assert_eq!((menu.surface, menu.raised), (scheme.menu, scheme.menu));
            assert_eq!((menu.hover, menu.selected), (scheme.menu_hover, scheme.menu_hover));
            assert_eq!(menu.muted, scheme.menu_muted);
            let control = scheme.on_raised();
            assert_eq!(control.surface, scheme.control.surface);
            assert_eq!(control.hover, scheme.control.hover);
            assert_eq!(control.selected, scheme.control.pressed);
            assert_eq!(control.border, scheme.control.border);
            assert_eq!(control.text, scheme.control.text);
            // A control must stand out from the card it sits on, in each state.
            assert_ne!(control.surface, scheme.raised);
            assert_ne!(control.hover, control.surface);
            assert_ne!(control.selected, control.hover);
        }
    }

    #[test]
    fn the_surface_role_decides_hover_and_pressed() {
        let scheme = Theme::DARK.scheme();
        let fill = Color::rgb(60, 70, 80);
        let ink = Color::rgb(1, 2, 3);

        let flat = scheme.on(Surface::Flat(fill), ink);
        assert_eq!((flat.hover, flat.selected), (scheme.hover, scheme.selected));

        let raised = scheme.on(Surface::Raised(fill), ink);
        assert_eq!((raised.hover, raised.selected), (Color::rgb(68, 78, 88), scheme.selected));

        let solid = scheme.on(Surface::Solid(fill), ink);
        assert_eq!((solid.hover, solid.selected), (fill, fill));

        for palette in [flat, raised, solid] {
            assert_eq!((palette.surface, palette.raised, palette.text), (fill, fill, ink));
            assert_eq!(palette.border, scheme.border);
            assert_eq!(palette.muted, scheme.muted);
            assert_eq!(palette.shadow, scheme.shadow);
        }
    }

    #[test]
    fn a_step_moves_toward_the_ink_and_keeps_alpha() {
        let dark = Theme::DARK.scheme();
        let light = LIGHT.scheme();
        let veil = Color::rgba(100, 110, 120, 40);
        assert_eq!(dark.step(veil), Color::rgba(108, 118, 128, 40));
        assert_eq!(light.step(veil), Color::rgba(92, 102, 112, 40));
        for scheme in [dark, light] {
            for surface in [scheme.canvas, scheme.panel, scheme.raised, scheme.menu] {
                assert_ne!(scheme.step(surface), surface);
            }
        }
        // A channel at its limit stays there.
        assert_eq!(dark.step(Color::rgb(255, 250, 0)), Color::rgb(255, 255, 8));
        assert_eq!(light.step(Color::rgb(0, 5, 255)), Color::rgb(0, 0, 247));
    }
}
