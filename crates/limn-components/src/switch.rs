//! An on/off control: one target, its thumb sliding inside its track.

use crate::{hover_ms, lighter, Access, Color, Component, ComponentPalette, ControlSize, ControlState, Element, Role};

/// How a [`Switch`] paints its track and its thumb.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SwitchVariant {
    /// On, a thumb of the palette's surface on a track of its text, or of
    /// `ink`; off, a thumb of the palette's text on its selected fill.
    #[default]
    Default,
    /// A thumb of the palette's lightest color at full strength, the text of
    /// a dark palette and the surface of a light one, on and off: on a track
    /// of the ink, which is the palette's `link` color unless `ink` says
    /// otherwise, and off on the palette's muted color. Either track is
    /// darkened where needed until the thumb stands at a contrast of 3 or
    /// more on it. This is how chat applications draw a switch.
    Accent,
}

/// How far a switch's thumb stands in from its track's edges, unless a host
/// says otherwise: 3 pixels, so a size's thumb is 6 less than its track.
const THUMB_INSET: f32 = 3.0;

/// A caller-controlled switch with a single focusable target. The thumb is
/// painted inside its track and moves only when the caller passes a new
/// `checked` value after receiving `UiAction::Clicked`.
pub struct Switch {
    action_id: String,
    checked: bool,
    state: ControlState,
    palette: ComponentPalette,
    accessibility_label: Option<String>,
    size: ControlSize,
    track: Option<(f32, f32)>,
    variant: SwitchVariant,
    ink: Option<Color>,
    off_ink: Option<Color>,
    thumb_inset: f32,
    thumb_width: Option<f32>,
    off_track: Option<(Color, Color)>,
    glass: Option<crate::GlassMaterial>,
}

impl Switch {
    /// An on/off control that shows `checked` and reports `Clicked` with this
    /// ID. The host flips its own value.
    pub fn new(action_id: impl Into<String>, checked: bool, palette: ComponentPalette) -> Self {
        Self {
            action_id: action_id.into(),
            checked,
            state: ControlState::Enabled,
            palette,
            accessibility_label: None,
            size: ControlSize::Regular,
            track: None,
            variant: SwitchVariant::Default,
            ink: None,
            off_ink: None,
            thumb_inset: THUMB_INSET,
            thumb_width: None,
            off_track: None,
            glass: None,
        }
    }

    /// Use an optical thumb with a spring lift while pressed. A tap toggles;
    /// a drag reports `Clicked` only when released on the opposite half of
    /// the track. Releasing on the current half or cancelling reports nothing.
    pub fn glass(mut self, material: crate::GlassMaterial) -> Self {
        self.glass = Some(material);
        self
    }

    /// The track's width and height and the thumb's side: 28, 16 and 10
    /// pixels for `Dense`, 32, 18 and 12 for `Compact`, 36, 20 and 14 for
    /// `Regular`, which is the switch without a size, and 44, 24 and 18 for
    /// `Spacious`. The thumb is 3 pixels inside the track unless
    /// [`thumb_inset`](Self::thumb_inset) says otherwise.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self.track = None;
        self
    }

    /// A track of the host's own size, in logical pixels, in place of a
    /// [`ControlSize`]'s, such as a settings page's 50 by 26: the thumb is
    /// as tall as the track less twice its inset, 3 pixels from each edge
    /// unless [`thumb_inset`](Self::thumb_inset) says otherwise. The height is
    /// at least 12 and the width leaves room for the thumb to travel, at least
    /// twice the height less twice the inset. Values that are not finite are ignored.
    pub fn track_size(mut self, width: f32, height: f32) -> Self {
        if width.is_finite() && height.is_finite() {
            self.track = Some((width, height.max(12.0)));
        }
        self
    }

    /// How far the thumb stands in from the track's edges, in logical
    /// pixels: 3 unless set, and a track of 24 with a thumb of 16 takes 4.
    /// The thumb is never larger than its track; a value that is not finite is
    /// ignored and a negative one is zero.
    pub fn thumb_inset(mut self, inset: f32) -> Self {
        if inset.is_finite() {
            self.thumb_inset = inset.max(0.0);
        }
        self
    }

    /// Set the thumb's width in logical pixels, independently of its height.
    /// Non-finite values are ignored; the width is clamped inside the track.
    pub fn thumb_width(mut self, width: f32) -> Self {
        if width.is_finite() { self.thumb_width = Some(width.max(0.0)); }
        self
    }

    /// The track while the switch is off, in `fill` with a 1 pixel `border`
    /// around it, as Discord draws it: a translucent fill that shows what
    /// lies under it. The thumb keeps its color. The track while it is on is
    /// unchanged.
    pub fn off_track(mut self, fill: Color, border: Color) -> Self {
        self.off_track = Some((fill, border));
        self
    }

    /// How the track and the thumb are painted; see [`SwitchVariant`].
    pub fn variant(mut self, variant: SwitchVariant) -> Self {
        self.variant = variant;
        self
    }

    /// The track's color while the switch is on, in place of the palette's
    /// text, or of its `link` color for `SwitchVariant::Accent`, which
    /// darkens it where needed for its light thumb to read on it.
    pub fn ink(mut self, color: Color) -> Self {
        self.ink = Some(color);
        self
    }

    /// The track's color while the switch is off, as given, in place of the
    /// palette's selected fill, or of its muted color darkened for
    /// `SwitchVariant::Accent`: a pale gray track under a white thumb, as a
    /// desktop settings page draws it. The host answers for the thumb
    /// reading on it; the state still shows in the thumb's place.
    pub fn off_ink(mut self, color: Color) -> Self {
        self.off_ink = Some(color);
        self
    }

    /// What the switch is called to the platform. It shows no text of its
    /// own, so without this, or a `FormField` around it, it has no name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Disable it: it keeps its place and its value and cannot be used.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled {
            ControlState::Enabled
        } else {
            ControlState::Disabled
        };
        self
    }
}

impl Component for Switch {
    fn render(self) -> Element {
        let (track_width, track_height) = match (self.track, self.size) {
            (Some((width, height)), _) => (width, height),
            (None, ControlSize::Dense) => (28.0, 16.0),
            (None, ControlSize::Compact) => (32.0, 18.0),
            (None, ControlSize::Regular) => (36.0, 20.0),
            (None, ControlSize::Spacious) => (44.0, 24.0),
        };
        // At most half the track, so the thumb is never negative.
        let inset = self.thumb_inset.min(track_height / 2.0);
        let thumb = track_height - inset * 2.0;
        // Room for the thumb to travel between its two slots, with a gap between them.
        let track_width = track_width.max(thumb * 2.0 + inset * 2.0);
        let palette = self.palette;
        let (thumb_color, track_color) = match (self.variant, self.checked) {
            (SwitchVariant::Default, true) => (palette.surface, self.ink.unwrap_or(palette.text)),
            (SwitchVariant::Default, false) => (palette.text, palette.selected),
            (SwitchVariant::Accent, checked) => accent_switch_colors(palette, checked, self.ink),
        };
        let thumb_color = if self.glass.is_some() { if palette.is_dark() { palette.text } else { palette.surface } } else { thumb_color };
        let track_color = match self.off_ink {
            Some(off) if !self.checked => off,
            _ => track_color,
        };
        // The thumb is the strip's selection indicator: it slides between the
        // two slots when `checked` changes and keeps its place across rebuilds.
        let slot = |name: &str| {
            Element::spacer()
                .size(Some(thumb), Some(thumb))
                .indicator_slot(name)
        };
        let mut contents = Element::tab_strip(
            vec![
                slot("off"),
                Element::spacer().size(Some(track_width - thumb * 2.0 - inset * 2.0), Some(thumb)),
                slot("on"),
            ],
            Vec::new(),
        )
        .tab_selection(
            format!("{}:thumb", self.action_id),
            if self.checked { "on" } else { "off" },
            thumb_color,
            thumb / 2.0,
            crate::tokens().motion.move_ms,
        );
        if let Some(width) = self.thumb_width {
            let width = width.clamp(thumb, track_width - inset * 2.0);
            let cell = (track_width - inset * 2.0) / 2.0;
            if let crate::ElementKind::TabStrip { children, .. } = &mut contents.kind {
                *children = vec![Element::spacer().size(Some(cell), Some(thumb)).indicator_slot("off"),
                    Element::spacer().size(Some(cell), Some(thumb)).indicator_slot("on")];
            }
            contents = contents.selection_width(width);
        }
        if let Some(material) = self.glass {
            contents = contents.glass_selection(material).selection_press(self.action_id.clone());
        }
        // Off, the host's own track when it gives one, with its border.
        let (track_color, border) = match (self.off_track, self.checked) {
            (Some((fill, border)), false) => (fill, Some(border)),
            _ => (track_color, None),
        };
        let enabled = self.state == ControlState::Enabled;
        // Drawn disabled it is passive, and known by the ID it would have had.
        let mut access = Access::new(Role::Switch)
            .id(self.action_id.clone())
            .toggled(self.checked)
            .disabled(!enabled);
        if let Some(name) = self.accessibility_label {
            access = access.name(name);
        }
        let track = if enabled {
            Element::clickable(self.action_id, contents)
                .hover_background(lighter(track_color, 10))
                .pressed_background(lighter(track_color, 16))
                .transition_ms(hover_ms())
        } else {
            Element::panel(contents)
        };
        let track = track
            .background(track_color)
            .padding(inset)
            .radius(track_height / 2.0)
            .size(Some(track_width), Some(track_height))
            .accessible(access);
        match border {
            Some(border) => track.border(border, 1.0),
            None => track,
        }
    }
}

/// The thumb and the track of a `SwitchVariant::Accent` switch. The thumb is
/// light at full strength; the track is `on_ink` or the link color while on,
/// the muted color while off, darkened until it reads at 3:1 against the thumb.
pub(crate) fn accent_switch_colors(
    palette: ComponentPalette,
    checked: bool,
    on_ink: Option<Color>,
) -> (Color, Color) {
    let light = if palette.is_dark() { palette.text } else { palette.surface };
    let light = Color::rgb(light.r, light.g, light.b);
    let track = if checked { on_ink.unwrap_or_else(|| palette.link()) } else { palette.muted };
    (light, crate::readable_ink(track, light, 3.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessNode, DrawCommand, Point, Size, Tooltip, UiAction, UiEvent, UiTree};

    fn access_node(element: Element, id: &str) -> AccessNode {
        let nodes = UiTree::new(Element::column(vec![element, Element::spacer().flex(1.0)]))
            .layout(Size::new(480.0, 320.0))
            .access;
        nodes.iter().find(|node| node.id == id).cloned().unwrap_or_else(|| panic!("no node {id} among {nodes:?}"))
    }

    #[test]
    fn a_switch_says_whether_it_is_on_and_takes_its_name_from_the_host() {
        let palette = ComponentPalette::neutral_dark();
        for (checked, enabled) in [(true, true), (false, true), (true, false), (false, false)] {
            let switch = Switch::new("wifi", checked, palette).enabled(enabled).accessibility_label("Wi-Fi");
            let said = access_node(switch.render(), "wifi");
            assert_eq!((said.role, said.name.as_str(), said.toggled), (Role::Switch, "Wi-Fi", Some(checked)));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
        }
        assert_eq!(access_node(Switch::new("wifi", true, palette).render(), "wifi").name, "");
    }


    #[test]
    fn tooltip_keeps_switch_track_and_thumb_geometry_in_a_tall_row() {
        let palette = ComponentPalette::neutral_dark();
        for checked in [false, true] {
            let toggle = Tooltip::new(
                Switch::new("toggle", checked, palette).render(),
                "toggle",
                "Cambiar habilitación",
                palette,
            )
            .render();
            let mut tree = UiTree::new(
                Element::row(vec![toggle, Element::spacer().flex(1.0)]).size(None, Some(50.0)),
            );
            tree.set_reduced_motion(true);
            let viewport = Size::new(300.0, 160.0);
            let frame = tree.layout(viewport);
            let track = frame
                .regions
                .iter()
                .find(|region| region.id == "toggle")
                .expect("switch target")
                .rect;
            assert_eq!(track.width, 36.0);
            assert_eq!(track.height, 20.0);
            assert_eq!(
                frame
                    .regions
                    .iter()
                    .filter(|region| region.id == "toggle")
                    .count(),
                1
            );
            let thumb = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::FillRect { rect, radius, .. }
                        if rect.width == 14.0 && rect.height == 14.0 && *radius == 7.0 =>
                    {
                        Some(*rect)
                    }
                    _ => None,
                })
                .expect("switch thumb");
            assert_eq!(thumb.x - track.x, if checked { 19.0 } else { 3.0 });
            assert_eq!(thumb.y - track.y, 3.0);
            let point = Point::new(track.x + 8.0, track.y + 8.0);
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked {
                    id: "toggle".into()
                }]
            );
            assert!(tree.focus("toggle"));
            let focused = tree.layout(viewport);
            assert!(focused.commands.iter().any(|command| matches!(command,
                DrawCommand::Text { text, .. } if text == "Cambiar habilitación"
            )));
            assert_eq!(
                focused
                    .regions
                    .iter()
                    .find(|region| region.id == "toggle")
                    .unwrap()
                    .rect,
                track
            );
        }
    }


    #[test]
    fn switch_is_one_keyboard_and_pointer_target() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(Switch::new("keep_awake", false, palette).render());
        let frame = tree.layout(Size::new(36.0, 20.0));
        assert_eq!(frame.regions.len(), 1);
        assert_eq!(frame.regions[0].id, "keep_awake");
        assert_eq!(frame.regions[0].rect.width, 36.0);
        assert_eq!(frame.regions[0].rect.height, 20.0);
        assert!(tree.focus("keep_awake"));
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: crate::Key::Enter,
                shift: false,
                ctrl: false,
                alt: false,
            }),
            vec![UiAction::Clicked {
                id: "keep_awake".into()
            }]
        );
        tree.set_root(
            Switch::new("keep_awake", false, palette)
                .disabled()
                .render(),
        );
        let disabled = tree.layout(Size::new(36.0, 20.0));
        assert!(disabled.regions.is_empty());
    }


    /// A switch at its own size, settled: its track's rectangle and color,
    /// then its thumb's.
    fn switch_paint(switch: Switch) -> ((crate::Rect, Color), (crate::Rect, Color)) {
        let mut tree = UiTree::new(Element::row(vec![switch.render(), Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(200.0, 40.0));
        let fills: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, radius } => Some((*rect, *color, *radius)),
                _ => None,
            })
            .collect();
        // The track is the pill the others stand in; the thumb is the circle inside it.
        let [(track, track_color, _), (thumb, thumb_color, thumb_radius)] = fills.as_slice() else {
            panic!("a track and a thumb, got {fills:?}");
        };
        assert_eq!(*thumb_radius, thumb.width / 2.0);
        ((*track, *track_color), (*thumb, *thumb_color))
    }


    #[test]
    fn a_switch_size_sets_its_track_and_its_thumb_three_pixels_inside_it() {
        let palette = ComponentPalette::neutral_dark();
        for (size, width, height, side) in [
            (ControlSize::Dense, 28.0, 16.0, 10.0),
            (ControlSize::Compact, 32.0, 18.0, 12.0),
            (ControlSize::Regular, 36.0, 20.0, 14.0),
            (ControlSize::Spacious, 44.0, 24.0, 18.0),
        ] {
            for checked in [false, true] {
                let ((track, _), (thumb, _)) = switch_paint(Switch::new("s", checked, palette).size(size));
                assert_eq!((track.width, track.height), (width, height), "{size:?}");
                assert_eq!((thumb.width, thumb.height), (side, side), "{size:?}");
                let x = if checked { width - 3.0 - side } else { 3.0 };
                assert_eq!((thumb.x - track.x, thumb.y - track.y), (x, 3.0), "{size:?}, checked {checked}");
            }
        }
        // Without a size it is the regular one.
        assert_eq!(switch_paint(Switch::new("s", true, palette)).0 .0.width, 36.0);
    }


    #[test]
    fn a_switch_thumb_inset_and_off_track_are_opt_in() {
        let palette = ComponentPalette::neutral_dark();
        // Discord's: a 40 by 24 track with a 16-pixel thumb 4 pixels in from each edge.
        let discord = |checked| Switch::new("s", checked, palette).track_size(40.0, 24.0).thumb_inset(4.0);
        let ((track, _), (thumb, _)) = switch_paint(discord(false));
        assert_eq!((thumb.width, thumb.height), (16.0, 16.0));
        assert_eq!((thumb.x - track.x, thumb.y - track.y), (4.0, 4.0));
        let ((track, _), (thumb, _)) = switch_paint(discord(true));
        assert_eq!(thumb.x - track.x, 40.0 - 4.0 - 16.0, "on, the thumb is at the far inset");
        // A value that is not finite changes nothing.
        let ((_, _), (thumb, _)) = switch_paint(Switch::new("s", false, palette).size(ControlSize::Dense).thumb_inset(f32::NAN));
        assert_eq!((thumb.width, thumb.x), (10.0, 3.0));
        // An inset past half the track leaves no negative thumb.
        let frame = UiTree::new(Element::row(vec![Switch::new("s", false, palette).size(ControlSize::Dense).thumb_inset(20.0).render()]))
            .layout(Size::new(200.0, 40.0));
        for command in &frame.commands {
            if let DrawCommand::FillRect { rect, .. } = command {
                assert!(rect.width >= 0.0 && rect.height >= 0.0, "{rect:?}");
            }
        }
        // Off, the track is the host's fill with a 1-pixel border; on, the palette's.
        let fill = Color::rgba(255, 255, 255, 40);
        let border = Color::rgb(90, 90, 90);
        let ((track, color), (_, thumb_color)) = switch_paint(Switch::new("s", false, palette).off_track(fill, border));
        assert_eq!((color, thumb_color), (fill, palette.text));
        let mut tree = UiTree::new(Element::row(vec![Switch::new("s", false, palette).off_track(fill, border).render()]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(200.0, 40.0));
        let outlines: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::StrokeRect { rect, color, width, .. } if *color == border => Some((*rect, *width)),
                _ => None,
            })
            .collect();
        assert_eq!(outlines, vec![(track, 1.0)]);
        let ((_, on), _) = switch_paint(Switch::new("s", true, palette).off_track(fill, border));
        assert_eq!(on, palette.text);
    }


    #[test]
    fn a_track_of_its_own_size_keeps_the_thumb_three_pixels_inside_it() {
        let palette = ComponentPalette::neutral_dark();
        for (asked, (width, height)) in [((50.0, 26.0), (50.0, 26.0)), ((10.0, 26.0), (46.0, 26.0)), ((30.0, 4.0), (30.0, 12.0))] {
            for checked in [false, true] {
                let switch = Switch::new("s", checked, palette)
                    .size(ControlSize::Dense)
                    .track_size(asked.0, asked.1)
                    .track_size(f32::NAN, 40.0);
                let ((track, _), (thumb, _)) = switch_paint(switch);
                let side = height - 6.0;
                assert_eq!((track.width, track.height), (width, height), "{asked:?}");
                assert_eq!((thumb.width, thumb.height), (side, side), "{asked:?}");
                let x = if checked { width - 3.0 - side } else { 3.0 };
                assert_eq!((thumb.x - track.x, thumb.y - track.y), (x, 3.0), "{asked:?}, checked {checked}");
            }
        }
        // A size set after it takes its place again.
        let ((track, _), _) = switch_paint(Switch::new("s", true, palette).track_size(50.0, 26.0).size(ControlSize::Compact));
        assert_eq!((track.width, track.height), (32.0, 18.0));
    }
}
