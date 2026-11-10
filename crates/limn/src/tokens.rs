//! Shape and motion shared by the whole library.
//!
//! Color travels with each component as a [`crate::ComponentPalette`], so two
//! palettes can sit side by side. Corner radii and durations are different:
//! the engine paints tooltips, menus, scrollbars and focus rings itself,
//! without ever seeing a palette, and an application has one shape and one
//! pace. They are therefore set once for the process with [`set_tokens`] and
//! read where they are used, like variables on a style sheet's root. So is
//! a host's one look for every tooltip, when it wants one. [`with_tokens`]
//! puts other tokens in force on one thread while a closure runs: a test, or
//! a window that builds and lays out its tree in it.

use std::cell::Cell;
use std::sync::{PoisonError, RwLock};

use crate::{TooltipOptions, TooltipStyle};

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
    /// Cards that stand on their own in a grid: a directory's entries, a feature's card.
    pub tile: f32,
    /// The largest containers: dialogs, image previews, the composer.
    pub panel: f32,
}

impl Radius {
    /// The library's radii: 4 for controls, 6 for surfaces, 8 for cards, 16 for tiles and
    /// 12 for panels.
    pub const DEFAULT: Self = Self {
        control: 4.0,
        surface: 6.0,
        card: 8.0,
        tile: 16.0,
        panel: 12.0,
    };

    /// Controls as pills and everything else generously round: 16 for controls,
    /// which is half the height of a regular one, 16 for surfaces, 20 for cards, 24 for
    /// tiles and 28 for panels.
    pub const ROUND: Self = Self {
        control: 16.0,
        surface: 16.0,
        card: 20.0,
        tile: 24.0,
        panel: 28.0,
    };

    /// Right angles everywhere.
    pub const SQUARE: Self = Self {
        control: 0.0,
        surface: 0.0,
        card: 0.0,
        tile: 0.0,
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
            tile: self.tile * factor,
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
            tile: valid(self.tile, Self::DEFAULT.tile),
            panel: valid(self.panel, Self::DEFAULT.panel),
        }
    }
}

impl Default for Radius {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Which of the five radii a component rounds with. A component that lets
/// its caller choose takes one of these, never a number of pixels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RadiusRole {
    /// `Radius::control`: 4 pixels by default.
    Control,
    /// `Radius::surface`: 6 pixels by default.
    Surface,
    /// `Radius::card`: 8 pixels by default.
    Card,
    /// `Radius::tile`: 16 pixels by default.
    Tile,
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
            Self::Tile => radius.tile,
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
    /// The options every tooltip starts from: its delay, its title's size
    /// and face, its pointer, its [`TooltipShape`](crate::TooltipShape) and
    /// its entrance. A [`Tooltip`](crate::Tooltip)'s builders change them
    /// for that one hint; the hints the engine draws itself, such as a menu
    /// row's, take them as they are.
    pub tooltip_options: TooltipOptions,
}

impl Tokens {
    /// The default radii and durations, each palette's own tooltips and the
    /// library's hints.
    pub const DEFAULT: Self = Self {
        radius: Radius::DEFAULT,
        motion: Motion::DEFAULT,
        tooltip: None,
        shortcut_style: ShortcutStyle::Keycaps,
        tooltip_options: TooltipOptions::DEFAULT,
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

thread_local! {
    /// What [`with_tokens`] put in force on this thread, over [`CURRENT`].
    static SCOPED: Cell<Option<Tokens>> = const { Cell::new(None) };
}

/// The tokens in force: those of the innermost [`with_tokens`] running on this
/// thread, else the process's. Cheap enough to read wherever one is needed.
pub fn tokens() -> Tokens {
    SCOPED
        .with(Cell::get)
        .unwrap_or_else(|| *CURRENT.read().unwrap_or_else(PoisonError::into_inner))
}

/// Run `f` with `tokens` in force on this thread alone, and put back what was
/// in force when it returns or unwinds. Other threads, and the process's
/// tokens, are untouched, so tests that change tokens run side by side. A
/// window with tokens of its own builds and lays out its tree inside it, since
/// both read them. A radius that is negative or not finite falls back to its
/// default, as with [`set_tokens`].
pub fn with_tokens<R>(tokens: Tokens, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<Tokens>);
    impl Drop for Restore {
        fn drop(&mut self) {
            SCOPED.with(|scoped| scoped.set(self.0));
        }
    }
    let sanitized = Tokens {
        radius: tokens.radius.sanitized(),
        ..tokens
    };
    let _restore = Restore(SCOPED.with(|scoped| scoped.replace(Some(sanitized))));
    f()
}

/// Replace the tokens for the whole process. Elements built from here on use
/// them, so an application that rebuilds its tree every frame changes shape
/// and pace on the next one. A radius that is negative or not finite falls
/// back to its default, and the tooltip options' numbers are made safe as
/// [`TooltipOptions`] documents them.
pub fn set_tokens(tokens: Tokens) {
    *CURRENT.write().unwrap_or_else(PoisonError::into_inner) = Tokens {
        radius: tokens.radius.sanitized(),
        tooltip_options: tokens.tooltip_options.sanitized(),
        ..tokens
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    // These never call `set_tokens`: the tokens are shared by every test in
    // this binary. Changing them is covered by `tests/tokens.rs`, which runs
    // in a process of its own, or done on one thread with `with_tokens`.

    #[test]
    fn with_tokens_holds_on_its_own_thread_until_it_returns_or_unwinds() {
        let round = Tokens { radius: Radius::ROUND, ..Tokens::DEFAULT };
        let square = Tokens { radius: Radius::SQUARE, ..Tokens::DEFAULT };
        let process = tokens();
        with_tokens(round, || {
            assert_eq!(tokens().radius, Radius::ROUND);
            let other = std::thread::spawn(tokens).join().unwrap();
            assert_eq!(other, process, "another thread keeps the process's tokens");
            with_tokens(square, || assert_eq!(tokens().radius, Radius::SQUARE));
            assert_eq!(tokens().radius, Radius::ROUND, "the outer scope is back");
            let unwound = std::panic::catch_unwind(|| with_tokens(square, || panic!("inside")));
            assert!(unwound.is_err());
            assert_eq!(tokens().radius, Radius::ROUND, "an unwind puts it back too");
        });
        assert_eq!(tokens(), process);
        let bad = Radius { control: f32::NAN, ..Radius::ROUND };
        with_tokens(Tokens { radius: bad, ..Tokens::DEFAULT }, || {
            assert_eq!(tokens().radius.control, Radius::DEFAULT.control);
        });
    }

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
        assert_eq!(RadiusRole::Tile.px(), radius.tile);
        assert_eq!(RadiusRole::Panel.px(), radius.panel);
        assert_eq!(RadiusRole::Square.px(), 0.0);
    }

    #[test]
    fn a_radius_that_cannot_be_drawn_falls_back_to_its_default() {
        let radius = Radius {
            control: -3.0,
            surface: f32::NAN,
            card: 20.0,
            tile: f32::NAN,
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
