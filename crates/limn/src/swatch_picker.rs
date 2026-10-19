//! One color among a few: a round swatch for each, a ring around the chosen
//! one, and an optional swatch that opens a picker for a color of one's own.

use crate::{Access, Color, Component, ComponentPalette, ControlSize, Element, Icon, Role};

/// The ring around the chosen swatch, and the space between it and the color.
const RING: f32 = 2.0;
const RING_GAP: f32 = 2.0;
/// Room around each swatch, inside its hit region, for the keyboard focus
/// ring and the hover halo: the focus ring would otherwise lie exactly on
/// the chosen swatch's ring. Two cells touch, so swatches are twice this apart.
const ROOM: f32 = RING + RING_GAP;
/// The edge of a swatch that is not chosen, so one of the surface's own color shows.
const HAIRLINE: f32 = 1.0;

/// A swatch's diameter for a control size.
const fn side(size: ControlSize) -> f32 {
    match size {
        ControlSize::Dense => 20.0,
        ControlSize::Compact => 24.0,
        ControlSize::Regular => 28.0,
        ControlSize::Spacious => 32.0,
    }
}

/// One color of a [`SwatchPicker`].
pub struct Swatch {
    value: String,
    color: Color,
    name: String,
}

impl Swatch {
    /// A swatch of `color` that reports `UiAction::Clicked` with `value` as
    /// its ID when it is chosen, as a [`RadioOption`](crate::RadioOption)
    /// does. The value must be unique in the window. A swatch shows no text,
    /// so `name` is what the platform calls it: "Teal", or "#1abc9c".
    pub fn new(value: impl Into<String>, color: Color, name: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            color,
            name: name.into(),
        }
    }
}

/// The host owns the choice: the picker rings the swatch whose value is
/// `selected` and reports `UiAction::Clicked` with the value of the one
/// that is clicked. The arrow keys, Home and End move the choice among the
/// swatches and report the one they reach, as in a
/// [`RadioGroup`](crate::RadioGroup).
///
/// With [`custom`](Self::custom) a swatch before the others stands for a
/// color of the user's own. It reports `{id}:custom`, for the host to open a
/// [`ColorPicker`](crate::ColorPicker), and is ringed while `selected` is
/// that ID.
pub struct SwatchPicker {
    id: String,
    swatches: Vec<Swatch>,
    selected: Option<String>,
    custom: Option<(Color, String)>,
    columns: usize,
    size: ControlSize,
    enabled: bool,
    accessibility_label: Option<String>,
    palette: ComponentPalette,
}

impl SwatchPicker {
    /// One row of these swatches, `ControlSize::Regular`. `selected` is the
    /// chosen swatch's value; `None`, or a value no swatch has, rings none.
    /// `id` names the picker and prefixes the custom swatch's ID.
    pub fn new(
        id: impl Into<String>,
        swatches: Vec<Swatch>,
        selected: Option<&str>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            swatches,
            selected: selected.map(str::to_owned),
            custom: None,
            columns: 0,
            size: ControlSize::Regular,
            enabled: true,
            accessibility_label: None,
            palette,
        }
    }

    /// Add a swatch before the others for a color of the user's own: it
    /// shows `color` under a pencil and reports `UiAction::Clicked` with
    /// `{id}:custom`. It is reached with Tab, not with the arrows, since it
    /// opens something rather than choosing. `name` is what the platform
    /// calls it, such as "Custom color".
    pub fn custom(mut self, color: Color, name: impl Into<String>) -> Self {
        self.custom = Some((color, name.into()));
        self
    }

    /// Break the swatches into rows of this many, the custom swatch counted
    /// first. Zero, the default, keeps them in one row.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns;
        self
    }

    /// A swatch's diameter: 20 pixels for `Dense`, 24 for `Compact`, 28 for
    /// `Regular` and 32 for `Spacious`.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// A disabled picker is passive: it shows every color, the chosen one
    /// ringed in the muted ink, and none has a hit region.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// What the picker is called to the platform: the question its swatches
    /// answer, such as "Server color". With it the picker is a node of its
    /// own, known by its ID; without it only the swatches are told.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }
}

impl Component for SwatchPicker {
    fn render(self) -> Element {
        let palette = self.palette;
        let enabled = self.enabled;
        let hover = palette.hover;
        let side = side(self.size);
        let selected = self.selected.as_deref();
        // The color fills the circle, or sits inside the ring when it is the chosen one.
        let swatch = |id: String, color: Color, mark: Option<Icon>, access: Access| {
            let chosen = selected == Some(id.as_str());
            let inner = if chosen {
                side - 2.0 * (RING + RING_GAP)
            } else {
                side
            };
            let content = match mark {
                Some(icon) => {
                    let glyph = (inner / 2.0).round();
                    // The pencil in whichever ink of the palette stands out from the color.
                    let ink = if crate::chip::contrast(palette.text, color)
                        >= crate::chip::contrast(palette.surface, color)
                    {
                        palette.text
                    } else {
                        palette.surface
                    };
                    Element::panel(
                        Element::icon(icon)
                            .foreground(ink)
                            .size(Some(glyph), Some(glyph)),
                    )
                    .padding((inner - glyph) / 2.0)
                }
                None => Element::spacer(),
            };
            let mut disk = Element::panel(content)
                .size(Some(inner), Some(inner))
                .background(color)
                .radius(inner / 2.0);
            let body = if chosen {
                Element::panel(disk)
                    .padding(RING + RING_GAP)
                    .border(if enabled { palette.text } else { palette.muted }, RING)
            } else {
                disk = disk.border(palette.border, HAIRLINE);
                Element::panel(disk)
            }
            .radius(side / 2.0)
            .size(Some(side), Some(side));
            let cell = Element::panel(body).padding(ROOM);
            let access = access.id(id.clone()).selected(chosen).disabled(!enabled);
            if enabled {
                Element::clickable(id, cell).hover_background(hover)
            } else {
                cell
            }
            .radius(side / 2.0 + ROOM)
            .accessible(access)
        };
        let mut cells = Vec::with_capacity(self.swatches.len() + 1);
        if let Some((color, name)) = self.custom {
            cells.push(swatch(
                format!("{}:custom", self.id),
                color,
                Some(Icon::Edit),
                Access::new(Role::Button).name(name),
            ));
        }
        // The arrows step among the swatches that choose, not the one that opens a picker.
        let reachable = if enabled {
            self.swatches
                .iter()
                .map(|swatch| swatch.value.clone())
                .collect()
        } else {
            Vec::new()
        };
        cells.extend(self.swatches.into_iter().map(|item| {
            swatch(
                item.value,
                item.color,
                None,
                Access::new(Role::RadioButton).name(item.name),
            )
        }));
        let columns = if self.columns == 0 {
            cells.len().max(1)
        } else {
            self.columns
        };
        let mut rows = Vec::new();
        let mut cells = cells.into_iter().peekable();
        while cells.peek().is_some() {
            rows.push(Element::row(cells.by_ref().take(columns).collect()));
        }
        let picker = Element::tab_strip(vec![Element::column(rows)], reachable).every_arrow();
        match self.accessibility_label {
            Some(name) => picker.accessible(Access::new(Role::Group).id(self.id).name(name)),
            None => picker,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    const RED: Color = Color::rgb(231, 76, 60);
    const GREEN: Color = Color::rgb(46, 204, 113);
    const BLUE: Color = Color::rgb(52, 152, 219);
    const MINE: Color = Color::rgb(250, 240, 200);
    const NIGHT: Color = Color::rgb(30, 30, 80);

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn window() -> Size {
        Size::new(400.0, 300.0)
    }

    /// Red, green and blue; green is the chosen one.
    fn picker() -> SwatchPicker {
        SwatchPicker::new(
            "accent",
            vec![
                Swatch::new("accent=red", RED, "Red"),
                Swatch::new("accent=green", GREEN, "Green"),
                Swatch::new("accent=blue", BLUE, "Blue"),
            ],
            Some("accent=green"),
            palette(),
        )
    }

    fn tree_of(picker: SwatchPicker) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![picker.render(), Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    fn key(tree: &mut UiTree, key: Key) -> Vec<UiAction> {
        tree.handle_event(UiEvent::Key {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        })
    }

    fn clicked(id: &str) -> Vec<UiAction> {
        vec![UiAction::Clicked { id: id.into() }]
    }

    fn fills(frame: &Frame) -> Vec<(Rect, Color, f32)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect {
                    rect,
                    color,
                    radius,
                } => Some((*rect, *color, *radius)),
                _ => None,
            })
            .collect()
    }

    fn strokes(frame: &Frame) -> Vec<(Rect, Color, f32)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::StrokeRect {
                    rect, color, width, ..
                } => Some((*rect, *color, *width)),
                _ => None,
            })
            .collect()
    }

    fn middle(rect: Rect) -> Point {
        Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0)
    }

    fn region(frame: &Frame, id: &str) -> Rect {
        frame
            .regions
            .iter()
            .find(|region| region.id == id)
            .unwrap()
            .rect
    }

    /// The swatch itself, `ROOM` in from the edges of its cell.
    fn inside(cell: Rect) -> Rect {
        Rect::new(
            cell.x + ROOM,
            cell.y + ROOM,
            cell.width - 2.0 * ROOM,
            cell.height - 2.0 * ROOM,
        )
    }

    #[test]
    fn each_swatch_is_a_circle_of_its_color_and_the_chosen_one_sits_inside_a_ring() {
        let frame = tree_of(picker()).layout(window());
        let fills = fills(&frame);
        assert_eq!(
            fills.iter().map(|fill| fill.1).collect::<Vec<_>>(),
            [RED, GREEN, BLUE]
        );
        let (red, green) = (fills[0], fills[1]);
        assert_eq!((red.0.width, red.0.height, red.2), (28.0, 28.0, 14.0));
        // The chosen color is 8 narrower, centered in its 28 pixel swatch,
        // which stands 4 in from the edges of its 36 pixel cell.
        assert_eq!((green.0.width, green.2), (20.0, 10.0));
        let cell = region(&frame, "accent=green");
        let ring = inside(cell);
        assert_eq!((cell.width, cell.height), (36.0, 36.0));
        assert_eq!((green.0.x - ring.x, green.0.y - ring.y), (4.0, 4.0));
        // Two swatches are 8 apart.
        assert_eq!(ring.x - (red.0.x + red.0.width), 8.0);
        // The ring in the text's ink; the others edged in a hairline of the border.
        let strokes = strokes(&frame);
        assert!(
            strokes.contains(&(ring, palette().text, RING)),
            "{strokes:?}"
        );
        assert_eq!(
            strokes
                .iter()
                .filter(|stroke| stroke.1 == palette().border && stroke.2 == HAIRLINE)
                .count(),
            2
        );
    }

    #[test]
    fn the_focus_ring_circles_the_cell_clear_of_the_choice_ring() {
        let mut tree = tree_of(picker());
        tree.layout(window());
        key(&mut tree, Key::Tab);
        key(&mut tree, Key::Tab);
        assert_eq!(tree.focused_id(), Some("accent=green"));
        let frame = tree.layout(window());
        let cell = region(&frame, "accent=green");
        let strokes = strokes(&frame);
        // The choice ring keeps the swatch's edge and is the only stroke there;
        // the focus ring takes the cell's edge, 2 pixels outside it.
        assert_eq!(
            strokes
                .iter()
                .filter(|stroke| stroke.0 == inside(cell))
                .collect::<Vec<_>>(),
            [&(inside(cell), palette().text, RING)]
        );
        assert_eq!(
            strokes
                .iter()
                .filter(|stroke| stroke.0 == cell)
                .map(|stroke| stroke.2)
                .collect::<Vec<_>>(),
            [2.0]
        );
    }

    #[test]
    fn a_swatch_under_the_pointer_lights_a_halo_around_it() {
        let mut tree = tree_of(picker());
        let frame = tree.layout(window());
        let halo = |frame: &Frame| {
            fills(frame)
                .into_iter()
                .filter(|fill| fill.1 == palette().hover)
                .collect::<Vec<_>>()
        };
        assert!(halo(&frame).is_empty());
        let cell = region(&frame, "accent=blue");
        tree.handle_event(UiEvent::PointerMove(middle(cell)));
        let frame = tree.layout(window());
        assert_eq!(halo(&frame), [(cell, palette().hover, 18.0)]);
        // Under the swatch's own color, so only the room around it lights.
        let order = |color: Color| {
            fills(&frame)
                .iter()
                .position(|fill| fill.1 == color)
                .unwrap()
        };
        assert!(order(palette().hover) < order(BLUE));
    }

    #[test]
    fn nothing_or_an_unknown_value_rings_none() {
        for selected in [None, Some("accent=purple")] {
            let swatches = vec![
                Swatch::new("accent=red", RED, "Red"),
                Swatch::new("accent=blue", BLUE, "Blue"),
            ];
            let frame = tree_of(SwatchPicker::new("accent", swatches, selected, palette()))
                .layout(window());
            assert!(fills(&frame).iter().all(|fill| fill.0.width == 28.0));
            assert!(frame.access.iter().all(|node| node.selected == Some(false)));
        }
    }

    #[test]
    fn a_click_on_a_swatch_reports_its_value() {
        let mut tree = tree_of(picker());
        let frame = tree.layout(window());
        for id in ["accent=red", "accent=green", "accent=blue"] {
            let point = middle(
                frame
                    .regions
                    .iter()
                    .find(|region| region.id == id)
                    .unwrap()
                    .rect,
            );
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(tree.handle_event(UiEvent::PointerUp(point)), clicked(id));
        }
    }

    #[test]
    fn the_arrows_home_and_end_move_the_choice_and_wrap() {
        let mut tree = tree_of(picker());
        tree.layout(window());
        assert!(tree.focus("accent=green"));
        for (pressed, reached) in [
            (Key::ArrowRight, "accent=blue"),
            (Key::ArrowRight, "accent=red"),
            (Key::ArrowLeft, "accent=blue"),
            (Key::ArrowUp, "accent=green"),
            (Key::ArrowDown, "accent=blue"),
            (Key::Home, "accent=red"),
            (Key::End, "accent=blue"),
        ] {
            assert_eq!(key(&mut tree, pressed), clicked(reached), "{pressed:?}");
            assert_eq!(tree.focused_id(), Some(reached));
        }
    }

    #[test]
    fn tab_reaches_each_swatch_and_enter_and_space_choose_it() {
        let mut tree = tree_of(picker());
        tree.layout(window());
        for id in ["accent=red", "accent=green", "accent=blue"] {
            key(&mut tree, Key::Tab);
            assert_eq!(tree.focused_id(), Some(id));
            assert_eq!(key(&mut tree, Key::Enter), clicked(id));
            assert_eq!(key(&mut tree, Key::Space), clicked(id));
        }
    }

    #[test]
    fn the_custom_swatch_comes_first_with_a_pencil_and_reports_opening_the_picker() {
        let mut tree = tree_of(picker().custom(MINE, "Custom color"));
        let frame = tree.layout(window());
        assert_eq!(frame.regions[0].id, "accent:custom");
        assert_eq!(fills(&frame)[0].1, MINE);
        // A light color takes the darker of the palette's two inks.
        let pencil = frame.commands.iter().find_map(|command| match command {
            DrawCommand::Icon {
                icon: Icon::Edit,
                tint,
                rect,
            } => Some((*tint, *rect)),
            _ => None,
        });
        let (tint, glyph) = pencil.expect("a pencil");
        assert_eq!((tint, glyph.width), (palette().surface, 14.0));
        // Tab reaches it first and Enter opens; the arrows pass it by.
        key(&mut tree, Key::Tab);
        assert_eq!(tree.focused_id(), Some("accent:custom"));
        assert_eq!(key(&mut tree, Key::Enter), clicked("accent:custom"));
        let point = middle(frame.regions[0].rect);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            clicked("accent:custom")
        );
        tree.focus("accent=red");
        assert_eq!(key(&mut tree, Key::ArrowLeft), clicked("accent=blue"));
    }

    #[test]
    fn the_custom_swatch_is_ringed_while_it_is_the_choice() {
        let swatches = vec![Swatch::new("accent=red", RED, "Red")];
        let custom = SwatchPicker::new("accent", swatches, Some("accent:custom"), palette())
            .custom(NIGHT, "Custom color");
        let frame = tree_of(custom).layout(window());
        assert_eq!(fills(&frame)[0].0.width, 20.0);
        let node = frame
            .access
            .iter()
            .find(|node| node.id == "accent:custom")
            .unwrap();
        assert_eq!(
            (node.role, node.name.as_str(), node.selected),
            (Role::Button, "Custom color", Some(true))
        );
        // A dark color takes the lighter ink.
        assert!(frame.commands.iter().any(
            |command| matches!(command, DrawCommand::Icon { tint, .. } if *tint == palette().text)
        ));
    }

    #[test]
    fn columns_break_the_swatches_into_rows() {
        let frame = tree_of(picker().custom(MINE, "Custom color").columns(2)).layout(window());
        let places: Vec<_> = frame
            .regions
            .iter()
            .map(|region| (region.rect.x, region.rect.y))
            .collect();
        assert_eq!(places, [(0.0, 0.0), (36.0, 0.0), (0.0, 36.0), (36.0, 36.0)]);
    }

    #[test]
    fn a_disabled_picker_has_no_region_takes_no_focus_and_keeps_its_place_and_its_ring() {
        let mut tree = tree_of(picker().custom(MINE, "Custom color").disabled());
        let frame = tree.layout(window());
        assert!(frame.regions.is_empty());
        assert!(!tree.focus("accent=green"));
        key(&mut tree, Key::Tab);
        assert_eq!(tree.focused_id(), None);
        let enabled = tree_of(picker().custom(MINE, "Custom color").enabled(true)).layout(window());
        for region in &enabled.regions {
            tree.handle_event(UiEvent::PointerDown(middle(region.rect)));
            assert!(tree
                .handle_event(UiEvent::PointerUp(middle(region.rect)))
                .is_empty());
        }
        let rects = |frame: &Frame| fills(frame).iter().map(|fill| fill.0).collect::<Vec<_>>();
        assert_eq!(rects(&frame), rects(&enabled));
        assert!(strokes(&frame)
            .iter()
            .any(|stroke| stroke.1 == palette().muted && stroke.2 == RING));
        assert!(frame
            .access
            .iter()
            .all(|node| node.disabled && !node.actionable));
    }

    #[test]
    fn each_swatch_is_a_radio_button_named_by_its_name_and_a_named_picker_is_a_group() {
        let frame = tree_of(picker().accessibility_label("Accent color")).layout(window());
        let said: Vec<_> = frame
            .access
            .iter()
            .map(|node| {
                (
                    node.id.as_str(),
                    node.role,
                    node.name.as_str(),
                    node.selected,
                )
            })
            .collect();
        assert_eq!(
            said,
            [
                ("accent=red", Role::RadioButton, "Red", Some(false)),
                ("accent=green", Role::RadioButton, "Green", Some(true)),
                ("accent=blue", Role::RadioButton, "Blue", Some(false)),
                ("accent", Role::Group, "Accent color", None),
            ]
        );
    }

    #[test]
    fn a_size_sets_the_diameter() {
        for (size, expected) in [
            (ControlSize::Dense, 20.0),
            (ControlSize::Compact, 24.0),
            (ControlSize::Regular, 28.0),
            (ControlSize::Spacious, 32.0),
        ] {
            let frame = tree_of(picker().size(size)).layout(window());
            let cell = expected + 2.0 * ROOM;
            assert!(frame
                .regions
                .iter()
                .all(|region| region.rect.width == cell && region.rect.height == cell));
            assert_eq!(fills(&frame)[1].0.width, expected - 8.0);
        }
    }
}
