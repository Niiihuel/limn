//! A color chosen by hand: a field of saturation and brightness, a bar of
//! hues, and a field for the color's hex digits.

use crate::components::TextField;
use crate::{Access, Color, Component, ComponentPalette, Element, Role, UiAction};

/// The diameter of a thumb. The field and the bar are inset by half of it,
/// so a thumb's center reaches their edges and the thumb overhangs them.
const THUMB: f32 = 16.0;
/// The thickness of the hue bar, inside the thumb's height.
const BAR: f32 = 10.0;
/// The thumb's ring, in the text's ink, and the edge around it, in the surface's.
const RING: f32 = 2.0;
const EDGE: f32 = 1.0;
/// Space between the field, the bar and the hex field.
const GAP: f32 = 12.0;
const DEFAULT_WIDTH: f32 = 220.0;
const MIN_WIDTH: f32 = 120.0;
/// The field is this share of its width high.
const FIELD_RATIO: f32 = 0.75;
/// The hue at each sixth of the bar and red again at its end: the definition
/// of hue, not colors of a theme.
const HUES: [Color; 7] = [
    Color::rgb(255, 0, 0),
    Color::rgb(255, 255, 0),
    Color::rgb(0, 255, 0),
    Color::rgb(0, 255, 255),
    Color::rgb(0, 0, 255),
    Color::rgb(255, 0, 255),
    Color::rgb(255, 0, 0),
];

/// A color as hue, saturation and value: where a [`ColorPicker`]'s thumbs
/// stand. Keep one as the picker's state rather than a [`Color`]: a gray
/// has no hue of its own and black no saturation, and the picker keeps the
/// ones it had.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv {
    /// Degrees round the color wheel, from 0, red, through 120, green, and
    /// 240, blue, to 360, red again.
    pub hue: f32,
    /// From 0, a gray, to 1, the full hue.
    pub saturation: f32,
    /// From 0, black, to 1, the brightest.
    pub value: f32,
}

impl Hsv {
    /// A color from its three coordinates, each clamped to its range; one
    /// that is not a number is 0.
    pub fn new(hue: f32, saturation: f32, value: f32) -> Self {
        let clamp = |number: f32, max: f32| {
            if number.is_nan() {
                0.0
            } else {
                number.clamp(0.0, max)
            }
        };
        Self {
            hue: clamp(hue, 360.0),
            saturation: clamp(saturation, 1.0),
            value: clamp(value, 1.0),
        }
    }
}

impl From<Color> for Hsv {
    /// The color's coordinates, its alpha left out. A gray's hue is 0.
    fn from(color: Color) -> Self {
        let [r, g, b] = [color.r, color.g, color.b].map(|channel| f32::from(channel) / 255.0);
        let max = r.max(g).max(b);
        let delta = max - r.min(g).min(b);
        let hue = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };
        Self::new(hue, if max == 0.0 { 0.0 } else { delta / max }, max)
    }
}

impl From<Hsv> for Color {
    /// The opaque color at these coordinates, each brought into its range first.
    fn from(hsv: Hsv) -> Self {
        let Hsv {
            hue,
            saturation,
            value,
        } = Hsv::new(hsv.hue, hsv.saturation, hsv.value);
        let channel = |n: f32| {
            let k = (n + hue / 60.0) % 6.0;
            let level = value - value * saturation * k.min(4.0 - k).clamp(0.0, 1.0);
            (level * 255.0).round() as u8
        };
        Color::rgb(channel(5.0), channel(3.0), channel(1.0))
    }
}

/// A color chosen by hand: a field whose thumb sets the saturation, across
/// it, and the value, up it; a bar whose thumb sets the hue; and a field
/// for the color's hex digits.
///
/// The host owns the color, as an [`Hsv`]. The field reports
/// `UiAction::AreaSliderChanged` with `{id}:area`, the bar
/// `UiAction::SliderChanged` with `{id}:hue`, and the hex field
/// `UiAction::InputChanged` with `{id}:hex`; [`ColorPicker::color_after`]
/// turns any of them into the next color.
pub struct ColorPicker {
    id: String,
    color: Hsv,
    width: f32,
    enabled: bool,
    labels: [String; 3],
    palette: ComponentPalette,
}

impl ColorPicker {
    /// A picker standing on `color`, 220 pixels wide.
    pub fn new(id: impl Into<String>, color: Hsv, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            color: Hsv::new(color.hue, color.saturation, color.value),
            width: DEFAULT_WIDTH,
            enabled: true,
            labels: [
                "Saturation and brightness".into(),
                "Hue".into(),
                "Hex".into(),
            ],
            palette,
        }
    }

    /// The whole picker's width in logical pixels, never below 120; the
    /// field is three quarters as high as it is wide. A width that is not
    /// finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(MIN_WIDTH);
        }
        self
    }

    /// A disabled picker shows its color and is passive: the thumbs are
    /// ringed in the muted ink, and nothing takes the pointer or focus.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// What the platform calls the field, the bar and the hex field. They
    /// are the English "Saturation and brightness", "Hue" and "Hex" unless
    /// set: the library's labels have no words for them.
    pub fn part_labels(
        mut self,
        area: impl Into<String>,
        hue: impl Into<String>,
        hex: impl Into<String>,
    ) -> Self {
        self.labels = [area.into(), hue.into(), hex.into()];
        self
    }

    /// The color as `#rrggbb`, in lowercase; its alpha is left out.
    pub fn hex(color: Color) -> String {
        format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b)
    }

    /// The opaque color `text` writes: six hex digits or three, which stand
    /// for six as `#abc` does for `#aabbcc`, with or without a `#` and with
    /// spaces around them ignored. `None` for anything else.
    pub fn parse_hex(text: &str) -> Option<Color> {
        let text = text.trim();
        let digits = text.strip_prefix('#').unwrap_or(text);
        let number = u32::from_str_radix(digits, 16)
            .ok()
            .filter(|_| digits.bytes().all(|byte| byte.is_ascii_hexdigit()))?;
        let byte = |shift: u32| (number >> shift) as u8;
        match digits.len() {
            6 => Some(Color::rgb(byte(16), byte(8), byte(0))),
            3 => Some(Color::rgb(
                (byte(8) & 15) * 17,
                (byte(4) & 15) * 17,
                (byte(0) & 15) * 17,
            )),
            _ => None,
        }
    }

    /// The color an action of the picker `id` leads to from `current`, or
    /// `None` for an action that is not one of its parts, so it can be tried
    /// on every action, and for hex text that [`parse_hex`](Self::parse_hex)
    /// does not read. A typed gray keeps the current hue, and a typed black
    /// the current hue and saturation, so the thumbs do not jump.
    pub fn color_after(id: &str, current: Hsv, action: &UiAction) -> Option<Hsv> {
        let part = |action_id: &str, name: &str| {
            action_id
                .strip_prefix(id)
                .and_then(|rest| rest.strip_prefix(':'))
                == Some(name)
        };
        match action {
            UiAction::AreaSliderChanged { id, x, y, .. } if part(id, "area") => {
                Some(Hsv::new(current.hue, *x, 1.0 - y))
            }
            UiAction::SliderChanged { id, fraction, .. } if part(id, "hue") => Some(Hsv::new(
                fraction * 360.0,
                current.saturation,
                current.value,
            )),
            UiAction::InputChanged { id, value } if part(id, "hex") => {
                let color = Self::parse_hex(value)?;
                if color == Color::from(current) {
                    return Some(current);
                }
                let typed = Hsv::from(color);
                Some(Hsv::new(
                    if typed.saturation > 0.0 {
                        typed.hue
                    } else {
                        current.hue
                    },
                    if typed.value > 0.0 {
                        typed.saturation
                    } else {
                        current.saturation
                    },
                    typed.value,
                ))
            }
            _ => None,
        }
    }
}

/// `child` at `share` of the way along a row or down a column, the rest
/// shared out on either side.
fn at(share: f32, child: Element, down: bool) -> Element {
    let space = |share: f32| (share > 0.0).then(|| Element::spacer().flex(share));
    let parts = space(share)
        .into_iter()
        .chain([child])
        .chain(space(1.0 - share))
        .collect();
    if down {
        Element::column(parts)
    } else {
        Element::row(parts)
    }
}

impl Component for ColorPicker {
    fn render(self) -> Element {
        let Self {
            id,
            color,
            width,
            enabled,
            labels: [area_label, hue_label, hex_label],
            palette,
        } = self;
        let travel = width - THUMB;
        let field_height = (travel * FIELD_RATIO).round();
        let ink = if enabled { palette.text } else { palette.muted };
        // The color a thumb stands for, ringed in the text's ink and edged in
        // the surface's, so that one of the two shows on any color.
        let thumb = |fill: Color| {
            let inner = THUMB - 2.0 * EDGE;
            Element::panel(
                Element::spacer()
                    .size(Some(inner), Some(inner))
                    .background(fill)
                    .border(ink, RING)
                    .radius(inner / 2.0),
            )
            .padding(EDGE)
            .background(palette.surface)
            .radius(THUMB / 2.0)
            .size(Some(THUMB), Some(THUMB))
        };
        // A slider when enabled, a passive panel of the same size when not.
        let part = |part_id: String,
                    content: Element,
                    height: f32,
                    slide: &dyn Fn(Element) -> Element,
                    access: Access| {
            let access = access.id(part_id.clone()).disabled(!enabled);
            if enabled {
                slide(Element::clickable(part_id, content))
            } else {
                Element::panel(content)
            }
            .size(Some(width), Some(height))
            .accessible(access)
        };

        // White to the full hue across, then clear to black down over it: the
        // two blend into every saturation and value of the hue.
        let pure = Color::from(Hsv::new(color.hue, 1.0, 1.0));
        let radius = crate::tokens().radius.control;
        let shades = Element::panel(
            Element::spacer()
                .vertical_gradient([Color::TRANSPARENT, Color::rgb(0, 0, 0)])
                .radius(radius),
        )
        .horizontal_gradient([Color::WHITE, pure])
        .radius(radius);
        let (x, y) = (color.saturation, 1.0 - color.value);
        let area = part(
            format!("{id}:area"),
            Element::stack(vec![
                Element::panel(shades).padding(THUMB / 2.0),
                at(
                    y,
                    at(x, thumb(Color::from(color)), false).size(None, Some(THUMB)),
                    true,
                ),
            ]),
            field_height + THUMB,
            &|element| element.area_slider(x, y, THUMB / 2.0),
            Access::new(Role::Slider)
                .name(area_label)
                .value(color.saturation)
                .value_text(format!(
                    "{}%, {}%",
                    (color.saturation * 100.0).round(),
                    (color.value * 100.0).round()
                )),
        );

        let share = color.hue / 360.0;
        let inset = || Element::spacer().size(Some(THUMB / 2.0), None);
        let bar = Element::spacer()
            .horizontal_gradient(HUES)
            .radius(BAR / 2.0)
            .size(None, Some(BAR))
            .flex(1.0)
            .centered_y();
        let hue = part(
            format!("{id}:hue"),
            Element::stack(vec![
                Element::row(vec![inset(), bar, inset()]),
                at(share, thumb(pure), false),
            ]),
            THUMB,
            &|element| element.slider(share, 0, THUMB / 2.0),
            Access::new(Role::Slider)
                .name(hue_label)
                .value(share)
                .value_text(format!("{}°", color.hue.round())),
        );

        let hex = TextField::new(
            format!("{id}:hex"),
            Self::hex(Color::from(color)),
            "#rrggbb",
            palette,
        )
        .monospace(true)
        .accessibility_label(hex_label)
        .enabled(enabled)
        .render()
        .flex(1.0);
        Element::column(vec![area, hue, Element::row(vec![inset(), hex, inset()])])
            .gap(GAP)
            .size(Some(width), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, HitKind, Key, Point, Rect, Size, UiEvent, UiTree};

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn window() -> Size {
        Size::new(500.0, 400.0)
    }

    /// A blue at half saturation and four fifths of the brightness.
    fn blue() -> Hsv {
        Hsv::new(240.0, 0.5, 0.8)
    }

    fn tree_of(picker: ColorPicker) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![picker.render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    fn region(frame: &Frame, id: &str) -> Rect {
        frame
            .regions
            .iter()
            .find(|region| region.id == id)
            .unwrap_or_else(|| panic!("no region {id}"))
            .rect
    }

    /// The center of each thumb: the 14 pixel disk inside its 16 pixel edge.
    fn thumbs(frame: &Frame) -> Vec<(Point, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, .. } if rect.width == THUMB - 2.0 * EDGE => {
                    Some((
                        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0),
                        *color,
                    ))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn colors_go_to_hue_saturation_and_value_and_back() {
        for (color, hsv) in [
            (Color::rgb(255, 0, 0), (0.0, 1.0, 1.0)),
            (Color::rgb(0, 255, 0), (120.0, 1.0, 1.0)),
            (Color::rgb(0, 0, 255), (240.0, 1.0, 1.0)),
            (Color::rgb(255, 0, 255), (300.0, 1.0, 1.0)),
            (Color::rgb(102, 102, 204), (240.0, 0.5, 0.8)),
            (Color::rgb(128, 128, 128), (0.0, 0.0, 128.0 / 255.0)),
            (Color::rgb(0, 0, 0), (0.0, 0.0, 0.0)),
        ] {
            let found = Hsv::from(color);
            assert!(
                (found.hue - hsv.0).abs() < 0.01
                    && (found.saturation - hsv.1).abs() < 0.01
                    && (found.value - hsv.2).abs() < 0.01,
                "{color:?}: {found:?}"
            );
            assert_eq!(Color::from(found), color);
        }
        // 360 is red, as 0 is.
        assert_eq!(
            Color::from(Hsv::new(360.0, 1.0, 1.0)),
            Color::rgb(255, 0, 0)
        );
        // Every color survives the trip.
        for rgb in (0..=0xff_ff_ffu32).step_by(0x01_03_07) {
            let color = Color::rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8);
            assert_eq!(Color::from(Hsv::from(color)), color);
        }
    }

    #[test]
    fn coordinates_out_of_range_or_not_numbers_are_brought_into_range() {
        assert_eq!(
            Hsv::new(400.0, 2.0, -1.0),
            Hsv {
                hue: 360.0,
                saturation: 1.0,
                value: 0.0
            }
        );
        assert_eq!(
            Hsv::new(f32::NAN, f32::NAN, f32::INFINITY),
            Hsv {
                hue: 0.0,
                saturation: 0.0,
                value: 1.0
            }
        );
        let raw = Hsv {
            hue: -30.0,
            saturation: f32::NAN,
            value: 9.0,
        };
        assert_eq!(Color::from(raw), Color::rgb(255, 255, 255));
        let frame = tree_of(ColorPicker::new("c", raw, palette())).layout(window());
        assert!(thumbs(&frame)
            .iter()
            .all(|(point, _)| point.x.is_finite() && point.y.is_finite()));
    }

    #[test]
    fn hex_is_written_with_six_digits_and_read_with_six_or_three() {
        assert_eq!(ColorPicker::hex(Color::rgba(26, 188, 156, 10)), "#1abc9c");
        for (text, color) in [
            ("#1abc9c", Some(Color::rgb(26, 188, 156))),
            (" 1ABC9C ", Some(Color::rgb(26, 188, 156))),
            ("#fa0", Some(Color::rgb(255, 170, 0))),
            ("", None),
            ("#", None),
            ("#12345", None),
            ("#1234567", None),
            ("+12345", None),
            ("#gggggg", None),
            ("##123456", None),
        ] {
            assert_eq!(ColorPicker::parse_hex(text), color, "{text:?}");
        }
    }

    #[test]
    fn the_field_runs_from_white_to_the_full_hue_with_black_over_it_and_the_bar_through_every_hue()
    {
        let frame = tree_of(ColorPicker::new("c", blue(), palette())).layout(window());
        let (area, hue) = (region(&frame, "c:area"), region(&frame, "c:hue"));
        // The field is 204 across and 153 high, 8 in from the region's edges.
        assert_eq!((area.width, area.height), (220.0, 153.0 + THUMB));
        let field = Rect::new(area.x + 8.0, area.y + 8.0, 204.0, 153.0);
        let radius = crate::tokens().radius.control;
        let inside = |rect: Rect| {
            rect.x >= field.x
                && rect.y >= field.y
                && rect.x + rect.width <= field.x + field.width
                && rect.y + rect.height <= field.y + field.height
        };
        let across: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillHorizontalGradient { rect, left, right } => {
                    Some((*rect, *left, *right))
                }
                _ => None,
            })
            .collect();
        let down: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillVerticalGradient { rect, top, bottom } => {
                    Some((*rect, *top, *bottom))
                }
                _ => None,
            })
            .collect();
        // Across: one span from (nearly) white to (nearly) the full hue, short of the rounded ends.
        let (span, left, right) = across[0];
        assert_eq!(
            span,
            Rect::new(
                field.x + radius,
                field.y,
                field.width - 2.0 * radius,
                field.height
            )
        );
        assert!(
            left.r == left.g && left.b == 255 && left.r > 240,
            "{left:?}"
        );
        assert!(
            right.r == right.g && right.b == 255 && right.r < 15,
            "{right:?}"
        );
        // Down: one span from clear to black over it.
        let [(span, top, bottom)] = down[..] else {
            panic!("one fade: {down:?}")
        };
        assert_eq!(
            span,
            Rect::new(
                field.x,
                field.y + radius,
                field.width,
                field.height - 2.0 * radius
            )
        );
        // Both ends stop short of the rounded rows: nearly clear, nearly black.
        assert_eq!(
            (top.r, top.g, top.b, bottom.r, bottom.g, bottom.b),
            (0, 0, 0, 0, 0, 0)
        );
        assert!(top.a < 10 && bottom.a > 245 && inside(span));
        // The bar: six spans between the seven hues, 10 high in the middle of the thumb's 16.
        let bar = &across[1..];
        assert_eq!(bar.len(), 6, "{bar:?}");
        assert!(bar
            .iter()
            .all(|g| g.0.height == BAR && g.0.y == hue.y + (THUMB - BAR) / 2.0));
        assert_eq!(
            (bar[1].1, bar[2].1, bar[3].1, bar[4].1),
            (HUES[1], HUES[2], HUES[3], HUES[4])
        );
        assert_eq!(
            (bar[0].0.x, bar[5].0.x + bar[5].0.width),
            (hue.x + 8.0 + BAR / 2.0, hue.x + 212.0 - BAR / 2.0)
        );
    }

    #[test]
    fn the_thumbs_stand_where_the_color_is() {
        let frame = tree_of(ColorPicker::new("c", blue(), palette())).layout(window());
        let (area, hue) = (region(&frame, "c:area"), region(&frame, "c:hue"));
        let thumbs = thumbs(&frame);
        let travel = area.width - THUMB;
        // Half way across, a fifth of the way down; two thirds of the way along the bar.
        assert_eq!(
            thumbs[0],
            (
                Point::new(
                    area.x + 8.0 + 0.5 * travel,
                    area.y + 8.0 + 0.2 * (area.height - THUMB)
                ),
                Color::rgb(102, 102, 204)
            )
        );
        assert!((thumbs[1].0.x - (hue.x + 8.0 + travel * 2.0 / 3.0)).abs() < 0.01);
        assert_eq!(
            (thumbs[1].0.y, thumbs[1].1),
            (hue.y + 8.0, Color::rgb(0, 0, 255))
        );
    }

    #[test]
    fn dragging_the_field_and_the_bar_and_typing_hex_lead_to_the_next_color() {
        let mut tree = tree_of(ColorPicker::new("c", blue(), palette()));
        let frame = tree.layout(window());
        let (area, hue) = (region(&frame, "c:area"), region(&frame, "c:hue"));
        let color_after = |action: &UiAction| ColorPicker::color_after("c", blue(), action);

        // The top right of the field is the full hue.
        let actions = tree.handle_event(UiEvent::PointerDown(Point::new(
            area.x + area.width - 8.0,
            area.y + 8.0,
        )));
        assert_eq!(color_after(&actions[0]), Some(Hsv::new(240.0, 1.0, 1.0)));
        let actions = tree.handle_event(UiEvent::PointerUp(Point::new(
            area.x + 8.0,
            area.y + area.height,
        )));
        assert!(matches!(
            actions[0],
            UiAction::AreaSliderChanged {
                committed: true,
                ..
            }
        ));
        assert_eq!(color_after(&actions[0]), Some(Hsv::new(240.0, 0.0, 0.0)));

        // The middle of the bar is cyan's hue, the colour's saturation and value kept.
        let point = Point::new(hue.x + hue.width / 2.0, hue.y + 8.0);
        let actions = tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(color_after(&actions[0]), Some(Hsv::new(180.0, 0.5, 0.8)));
        tree.handle_event(UiEvent::PointerUp(point));

        let typed = |value: &str| {
            color_after(&UiAction::InputChanged {
                id: "c:hex".into(),
                value: value.into(),
            })
        };
        assert_eq!(typed("#ff0000"), Some(Hsv::new(0.0, 1.0, 1.0)));
        assert_eq!(typed("#ff00"), None, "half typed");
        // A gray keeps the hue, black the hue and the saturation, and the same color all three.
        assert_eq!(
            typed("#808080").map(|hsv| (hsv.hue, hsv.saturation)),
            Some((240.0, 0.0))
        );
        assert_eq!(typed("000"), Some(Hsv::new(240.0, 0.5, 0.0)));
        assert_eq!(typed("#6666cc"), Some(blue()));
        // Not the picker's, or not one of its parts.
        for action in [
            UiAction::InputChanged {
                id: "d:hex".into(),
                value: "#fff".into(),
            },
            UiAction::InputChanged {
                id: "c:hexa".into(),
                value: "#fff".into(),
            },
            UiAction::SliderChanged {
                id: "c".into(),
                fraction: 0.5,
                committed: true,
            },
            UiAction::Clicked { id: "c:hue".into() },
        ] {
            assert_eq!(color_after(&action), None, "{action:?}");
        }
    }

    #[test]
    fn the_keyboard_moves_both_thumbs() {
        let mut tree = tree_of(ColorPicker::new("c", blue(), palette()));
        tree.layout(window());
        let key = |tree: &mut UiTree, key| {
            tree.handle_event(UiEvent::Key {
                key,
                shift: false,
                ctrl: false,
            })
        };
        let next =
            |actions: Vec<UiAction>| ColorPicker::color_after("c", blue(), &actions[0]).unwrap();
        key(&mut tree, Key::Tab);
        assert_eq!(tree.focused_id(), Some("c:area"));
        let up = next(key(&mut tree, Key::ArrowUp));
        assert!((up.value - 0.85).abs() < 1e-5 && up.saturation == 0.5);
        let right = next(key(&mut tree, Key::ArrowRight));
        assert!((right.saturation - 0.55).abs() < 1e-5 && (right.value - 0.8).abs() < 1e-5);
        key(&mut tree, Key::Tab);
        assert_eq!(tree.focused_id(), Some("c:hue"));
        assert!((next(key(&mut tree, Key::ArrowRight)).hue - 258.0).abs() < 0.01);
        assert_eq!(next(key(&mut tree, Key::End)).hue, 360.0);
        key(&mut tree, Key::Tab);
        assert_eq!(tree.focused_id(), Some("c:hex"));
    }

    #[test]
    fn the_hex_field_shows_the_color_and_takes_text() {
        let mut tree = tree_of(ColorPicker::new("c", blue(), palette()));
        let frame = tree.layout(window());
        let field = frame
            .regions
            .iter()
            .find(|region| region.id == "c:hex")
            .unwrap();
        assert_eq!(field.kind, HitKind::TextInput);
        assert_eq!(tree.input_value("c:hex"), Some("#6666cc"));
        tree.focus("c:hex");
        assert_eq!(
            tree.handle_event(UiEvent::Key {
                key: Key::Backspace,
                shift: false,
                ctrl: false
            }),
            vec![UiAction::InputChanged {
                id: "c:hex".into(),
                value: "#6666c".into()
            }]
        );
        // Lined up with the field and the bar, inside the thumb's half.
        assert_eq!((field.rect.x, field.rect.width), (8.0, 204.0));
    }

    #[test]
    fn a_disabled_picker_is_passive_in_the_same_place_with_muted_rings() {
        let mut tree = tree_of(ColorPicker::new("c", blue(), palette()).disabled());
        let frame = tree.layout(window());
        assert!(frame
            .regions
            .iter()
            .all(|region| region.kind != HitKind::Button));
        assert!(!tree.focus("c:area") && !tree.focus("c:hue"));
        let point = Point::new(100.0, 60.0);
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        let enabled =
            tree_of(ColorPicker::new("c", blue(), palette()).enabled(true)).layout(window());
        assert_eq!(thumbs(&frame), thumbs(&enabled));
        let rings: Vec<_> = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::StrokeRect { color, width, .. } if *width == RING => Some(*color),
                _ => None,
            })
            .collect();
        assert_eq!(rings, [palette().muted, palette().muted]);
        for id in ["c:area", "c:hue", "c:hex"] {
            assert!(
                frame
                    .access
                    .iter()
                    .any(|node| node.id == id && node.disabled),
                "{id}"
            );
        }
    }

    #[test]
    fn the_platform_is_told_each_part_its_name_and_its_value() {
        let frame = tree_of(ColorPicker::new("c", blue(), palette()).part_labels(
            "Tono y brillo",
            "Matiz",
            "Hex",
        ))
        .layout(window());
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap();
        let (area, hue, hex) = (node("c:area"), node("c:hue"), node("c:hex"));
        assert_eq!(
            (area.role, area.name.as_str(), area.value_text.as_deref()),
            (Role::Slider, "Tono y brillo", Some("50%, 80%"))
        );
        assert_eq!(
            (hue.role, hue.name.as_str(), hue.value_text.as_deref()),
            (Role::Slider, "Matiz", Some("240°"))
        );
        assert_eq!(
            (hex.name.as_str(), hex.value_text.as_deref()),
            ("Hex", Some("#6666cc"))
        );
    }

    #[test]
    fn a_width_sets_every_part_and_bad_ones_are_ignored_or_raised() {
        for (width, expected) in [
            (300.0, 300.0),
            (40.0, MIN_WIDTH),
            (f32::NAN, DEFAULT_WIDTH),
            (f32::INFINITY, DEFAULT_WIDTH),
        ] {
            let frame =
                tree_of(ColorPicker::new("c", blue(), palette()).width(width)).layout(window());
            let area = region(&frame, "c:area");
            assert_eq!(area.width, expected, "{width}");
            assert_eq!(
                area.height,
                ((expected - THUMB) * FIELD_RATIO).round() + THUMB
            );
            assert_eq!(region(&frame, "c:hue").width, expected);
        }
    }
}
