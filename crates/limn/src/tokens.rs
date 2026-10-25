//! Shape and motion shared by the whole library.
//!
//! Color travels with each component as a [`crate::ComponentPalette`], so two
//! palettes can sit side by side. Corner radii and durations are different:
//! the engine paints tooltips, menus, scrollbars and focus rings itself,
//! without ever seeing a palette, and an application has one shape and one
//! pace. They are therefore set once for the process with [`set_tokens`] and
//! read where they are used, like variables on a style sheet's root. So is
//! a host's one look for every tooltip, when it wants one.

use std::sync::{PoisonError, RwLock};

use crate::TooltipStyle;

/// Corner radii in logical pixels, by what is being rounded.
///
/// Pills, circles and the thumb of a switch are shapes rather than rounded
/// corners and stay round under every setting. Marks inside content, such as
/// a text selection or the bars of a chart, keep their own two or three pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Radius {
    /// Buttons, inputs, menu rows, keycaps and tooltips.
    pub control: f32,
    /// Surfaces that float above the page: popovers, menus, toasts.
    pub surface: f32,
    /// Groups set into the page: alerts, preference groups, activity cards.
    pub card: f32,
    /// The largest containers: dialogs, image previews, the composer.
    pub panel: f32,
}

impl Radius {
    /// The library's radii: 4 for controls, 6 for surfaces, 8 for cards and 12 for panels.
    pub const DEFAULT: Self = Self {
        control: 4.0,
        surface: 6.0,
        card: 8.0,
        panel: 12.0,
    };

    /// Controls as pills and everything else generously round: 16 for controls,
    /// which is half the height of a regular one, 16 for surfaces, 20 for cards and
    /// 28 for panels.
    pub const ROUND: Self = Self {
        control: 16.0,
        surface: 16.0,
        card: 20.0,
        panel: 28.0,
    };

    /// Right angles everywhere.
    pub const SQUARE: Self = Self {
        control: 0.0,
        surface: 0.0,
        card: 0.0,
        panel: 0.0,
    };

    /// Every radius multiplied by `factor`: below one is sharper, above one
    /// is rounder. Factors that are negative or not finite leave it unchanged.
    pub fn scaled(self, factor: f32) -> Self {
        if !factor.is_finite() || factor < 0.0 {
            return self;
        }
        Self {
            control: self.control * factor,
            surface: self.surface * factor,
            card: self.card * factor,
            panel: self.panel * factor,
        }
    }

    fn sanitized(self) -> Self {
        let valid = |value: f32, fallback: f32| {
            if value.is_finite() && value >= 0.0 {
                value
            } else {
                fallback
            }
        };
        Self {
            control: valid(self.control, Self::DEFAULT.control),
            surface: valid(self.surface, Self::DEFAULT.surface),
            card: valid(self.card, Self::DEFAULT.card),
            panel: valid(self.panel, Self::DEFAULT.panel),
        }
    }
}

impl Default for Radius {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Which of the four radii a component rounds with. A component that lets
/// its caller choose takes one of these, never a number of pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RadiusRole {
    /// `Radius::control`: 4 pixels by default.
    Control,
    /// `Radius::surface`: 6 pixels by default.
    Surface,
    /// `Radius::card`: 8 pixels by default.
    Card,
    /// `Radius::panel`: 12 pixels by default.
    Panel,
    /// No rounding, whatever the tokens: a cell that sits flush inside a row
    /// or a bar that paints the surface.
    Square,
}

impl RadiusRole {
    /// The radius in force for this role, read from [`tokens`].
    pub fn px(self) -> f32 {
        let radius = tokens().radius;
        match self {
            Self::Control => radius.control,
            Self::Surface => radius.surface,
            Self::Card => radius.card,
            Self::Panel => radius.panel,
            Self::Square => 0.0,
        }
    }
}

/// The four durations, in milliseconds, that every animation is built from.
/// A component takes one of them rather than a number of its own, so the same
/// kind of change always takes the same time. Zero makes that kind immediate.
///
/// These set the pace. To switch movement off altogether, including spinners
/// and loading pulses, use [`crate::UiTree::set_reduced_motion`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Motion {
    /// A control answering the pointer: hover and press fills.
    pub hover_ms: u16,
    /// A surface arriving: popover, menu, dialog, tooltip, toast.
    pub enter_ms: u16,
    /// A surface leaving. Shorter than its arrival, so closing never waits.
    pub exit_ms: u16,
    /// Something travelling or changing state in place: a tab indicator, a
    /// switch thumb, a step badge.
    pub move_ms: u16,
}

impl Motion {
    /// The library's durations in milliseconds: 120 to hover, 140 to enter, 110 to exit and
    /// 170 to move.
    pub const DEFAULT: Self = Self {
        hover_ms: 120,
        enter_ms: 140,
        exit_ms: 110,
        move_ms: 170,
    };

    /// Every duration multiplied by `factor`: two is half speed, a half is
    /// double speed. Factors that are negative or not finite leave it unchanged.
    pub fn scaled(self, factor: f32) -> Self {
        if !factor.is_finite() || factor < 0.0 {
            return self;
        }
        let scale = |ms: u16| (f32::from(ms) * factor).round().min(f32::from(u16::MAX)) as u16;
        Self {
            hover_ms: scale(self.hover_ms),
            enter_ms: scale(self.enter_ms),
            exit_ms: scale(self.exit_ms),
            move_ms: scale(self.move_ms),
        }
    }
}

impl Default for Motion {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Everything about the library's look that is not a palette's color.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tokens {
    /// The corner radii, by what is being rounded.
    pub radius: Radius,
    /// The four durations every animation is built from.
    pub motion: Motion,
    /// The colors every tooltip is painted with, whichever palette its
    /// control was built with, such as a body of #282828 with white text and
    /// a transparent `border`, which draws no outline. Unless set, each
    /// palette derives its own. A tooltip given its own `background` keeps it.
    pub tooltip: Option<TooltipStyle>,
    /// How shortcuts are drawn: keycaps unless set. `Text` draws every
    /// [`crate::Kbd`], and so every menu's shortcuts, as muted text, and a
    /// tooltip's in its style's `muted`. A field's shortcut hint keeps its
    /// keycaps.
    pub shortcut_style: ShortcutStyle,
}

impl Tokens {
    /// The default radii and durations, and each palette's own tooltips.
    pub const DEFAULT: Self = Self {
        radius: Radius::DEFAULT,
        motion: Motion::DEFAULT,
        tooltip: None,
        shortcut_style: ShortcutStyle::Keycaps,
    };
}

/// How keyboard shortcuts are drawn in menus, tooltips and other hints.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ShortcutStyle {
    /// Each key in an outlined cap.
    #[default]
    Keycaps,
    /// One line of muted text, such as "Ctrl+Shift+I", as a menu's
    /// accelerator is drawn.
    Text,
}

static CURRENT: RwLock<Tokens> = RwLock::new(Tokens::DEFAULT);

/// The tokens in force. Cheap enough to read wherever one is needed.
pub fn tokens() -> Tokens {
    *CURRENT.read().unwrap_or_else(PoisonError::into_inner)
}

/// Replace the tokens for the whole process. Elements built from here on use
/// them, so an application that rebuilds its tree every frame changes shape
/// and pace on the next one. A radius that is negative or not finite falls
/// back to its default.
pub fn set_tokens(tokens: Tokens) {
    *CURRENT.write().unwrap_or_else(PoisonError::into_inner) = Tokens {
        radius: tokens.radius.sanitized(),
        ..tokens
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // These never call `set_tokens`: the tokens are shared by every test in
    // this binary. Changing them is covered by `tests/tokens.rs`, which runs
    // in a process of its own.

    #[test]
    fn scaling_multiplies_every_token_and_ignores_bad_factors() {
        assert_eq!(Radius::DEFAULT.scaled(0.0), Radius::SQUARE);
        assert_eq!(Radius::DEFAULT.scaled(1.5).panel, 18.0);
        assert_eq!(Radius::DEFAULT.scaled(f32::NAN), Radius::DEFAULT);
        assert_eq!(Radius::DEFAULT.scaled(-1.0), Radius::DEFAULT);
        let slow = Motion::DEFAULT.scaled(2.0);
        assert_eq!((slow.hover_ms, slow.enter_ms, slow.exit_ms, slow.move_ms), (240, 280, 220, 340));
        assert_eq!(Motion::DEFAULT.scaled(0.0).enter_ms, 0);
        assert_eq!(Motion::DEFAULT.scaled(1e9).enter_ms, u16::MAX);
        assert_eq!(Motion::DEFAULT.scaled(f32::INFINITY), Motion::DEFAULT);
    }

    #[test]
    fn each_radius_role_reads_its_token() {
        let radius = tokens().radius;
        assert_eq!(RadiusRole::Control.px(), radius.control);
        assert_eq!(RadiusRole::Surface.px(), radius.surface);
        assert_eq!(RadiusRole::Card.px(), radius.card);
        assert_eq!(RadiusRole::Panel.px(), radius.panel);
        assert_eq!(RadiusRole::Square.px(), 0.0);
    }

    #[test]
    fn a_radius_that_cannot_be_drawn_falls_back_to_its_default() {
        let radius = Radius {
            control: -3.0,
            surface: f32::NAN,
            card: 20.0,
            panel: f32::INFINITY,
        }
        .sanitized();
        assert_eq!(
            radius,
            Radius {
                card: 20.0,
                ..Radius::DEFAULT
            }
        );
    }
}
