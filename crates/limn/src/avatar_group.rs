//! Several people as avatars that overlap in a row, with a count for those that do not fit.

use crate::{Access, Avatar, Color, Component, ComponentPalette, Element, FontFamily, Role};

/// Width of the ring that separates an avatar from the one it lies over.
const RING: f32 = 2.0;
/// The share of the diameter one avatar covers of the next, unless set.
const DEFAULT_OVERLAP: f32 = 0.25;
/// The largest share that can be covered: beyond it the initials are hidden.
const MAX_OVERLAP: f32 = 0.6;
/// The largest count written in full in the circle: more does not fit in it.
const MAX_COUNT: usize = 99;

/// The host owns the people and their order. The group is passive: it reports
/// nothing and takes no pointer or focus. The first avatar lies on top and each
/// one after it is partly under the one before, so the row reads from its start.
pub struct AvatarGroup {
    avatars: Vec<Avatar>,
    palette: ComponentPalette,
    limit: Option<usize>,
    size: f32,
    overlap: f32,
    ring: Option<Color>,
    id: Option<String>,
    label: Option<String>,
}

impl AvatarGroup {
    /// A group of `avatars` in the order given, each 32 logical pixels across
    /// whatever its own size, all of them shown.
    pub fn new(avatars: Vec<Avatar>, palette: ComponentPalette) -> Self {
        Self {
            avatars,
            palette,
            limit: None,
            size: 32.0,
            overlap: DEFAULT_OVERLAP,
            ring: None,
            id: None,
            label: None,
        }
    }

    /// Show the first `limit` avatars and then a circle of the same size that
    /// reads `+N` for the rest, or `99+` for more than 99. No circle is added
    /// when all fit. A limit of zero is raised to one.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit.max(1));
        self
    }

    /// Set the diameter of every avatar in logical pixels, in place of each
    /// one's own, clamped as `Avatar::size` clamps it (16 to 256). A value
    /// that is not finite is ignored.
    pub fn size(mut self, size: f32) -> Self {
        if size.is_finite() {
            self.size = size;
        }
        self
    }

    /// Set the share of its diameter that one avatar covers of the next,
    /// clamped to 0 (they touch) to 0.6; a quarter unless set. A value that
    /// is not finite is ignored.
    pub fn overlap(mut self, overlap: f32) -> Self {
        if overlap.is_finite() {
            self.overlap = overlap.clamp(0.0, MAX_OVERLAP);
        }
        self
    }

    /// The color of the ring around each avatar, in place of the palette's
    /// surface: the color of the ground the group stands on.
    pub fn ring(mut self, color: Color) -> Self {
        self.ring = Some(color);
        self
    }

    /// The ID of the group's accessibility node. The group is a node only
    /// with this and `accessibility_label` both.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Name the group for assistive technology as one image, such as
    /// "Ada, Grace and 3 others". It needs `id` as well; without both the
    /// platform is told nothing, as of an `Avatar`.
    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Component for AvatarGroup {
    fn render(self) -> Element {
        let palette = self.palette;
        let ring = self.ring.unwrap_or(palette.surface);
        // `Avatar` owns the clamp of a diameter: ask it rather than repeat it.
        let diameter = Avatar::new("", palette).size(self.size).render().style.width.unwrap_or(self.size);
        let total = self.avatars.len();
        let shown = self.limit.map_or(total, |limit| limit.min(total));
        let hidden = total - shown;
        let mut circles: Vec<Element> =
            self.avatars.into_iter().take(shown).map(|avatar| avatar.size(self.size).render()).collect();
        if hidden > 0 {
            let label = if hidden > MAX_COUNT { format!("{MAX_COUNT}+") } else { format!("+{hidden}") };
            // Three characters stand a little smaller so they stay inside the ring.
            let scale = if label.chars().count() > 2 { 0.32 } else { 0.38 };
            circles.push(
                Element::panel(
                    Element::text(label)
                        .font_family(FontFamily::Sans)
                        .font_size(diameter * scale)
                        .foreground(palette.text)
                        .center_content()
                        .centered_y(),
                )
                .size(Some(diameter), Some(diameter))
                .background(palette.selected)
                .radius(diameter / 2.0),
            );
        }
        let step = diameter * (1.0 - self.overlap);
        let width = if circles.is_empty() { 0.0 } else { step * (circles.len() - 1) as f32 + diameter };
        // The layout has no negative gap. Each circle is a layer of a stack as wide as
        // the group, pushed to its place by a spacer; the layers are given last first,
        // so the first circle is painted last and lies on top.
        let layers = circles
            .into_iter()
            .enumerate()
            .rev()
            .map(|(index, circle)| {
                // A disc in the ring's color hides what lies under, also where the palette's
                // fill lets it through. The ring is drawn over the circle's edge: a border of
                // the circle itself would lie under a photo.
                let ringed = Element::stack(vec![
                    Element::panel(Element::spacer()).size(Some(diameter), Some(diameter)).background(ring).radius(diameter / 2.0),
                    circle,
                    Element::panel(Element::spacer()).size(Some(diameter), Some(diameter)).border(ring, RING).radius(diameter / 2.0),
                ])
                .size(Some(diameter), Some(diameter));
                Element::row(vec![Element::spacer().size(Some(step * index as f32), Some(diameter)), ringed])
            })
            .collect();
        let group = Element::stack(layers).size(Some(width), Some(diameter));
        match (self.id, self.label) {
            (Some(id), Some(label)) if total > 0 => group.accessible(Access::new(Role::Image).id(id).name(label)),
            _ => group,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Key, Point, RasterImage, RasterImageFormat, Rect, Size, UiEvent, UiTree};
    use std::sync::Arc;

    const NAMES: [&str; 5] = ["Ada", "Grace", "Linus", "Margaret", "Barbara"];

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn people(count: usize) -> Vec<Avatar> {
        NAMES.iter().cycle().take(count).map(|name| Avatar::new(*name, palette())).collect()
    }

    fn frame_of(group: AvatarGroup) -> Frame {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![group.render(), Element::clickable("after", Element::spacer()).size(Some(10.0), Some(10.0))]),
            Element::spacer().flex(1.0),
        ]));
        tree.layout(Size::new(600.0, 300.0))
    }

    /// The fills of the circles, in the order they are painted.
    fn circles(frame: &Frame) -> Vec<Rect> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color, radius } if *color == palette().selected => {
                    assert_eq!(*radius, rect.width / 2.0);
                    Some(*rect)
                }
                _ => None,
            })
            .collect()
    }

    fn texts(frame: &Frame) -> Vec<String> {
        frame.text_layouts().iter().map(|block| block.source.clone()).collect()
    }

    /// Where the element after the group starts: the group's width.
    fn width(frame: &Frame) -> f32 {
        frame.regions.iter().find(|region| region.id == "after").unwrap().rect.x
    }

    #[test]
    fn avatars_stand_in_a_row_each_covering_a_quarter_of_the_next() {
        let frame = frame_of(AvatarGroup::new(people(4), palette()));
        let mut rects = circles(&frame);
        rects.sort_by(|a, b| a.x.total_cmp(&b.x));
        let places: Vec<_> = rects.iter().map(|rect| (rect.x, rect.y, rect.width, rect.height)).collect();
        assert_eq!(places, [(0.0, 0.0, 32.0, 32.0), (24.0, 0.0, 32.0, 32.0), (48.0, 0.0, 32.0, 32.0), (72.0, 0.0, 32.0, 32.0)]);
        assert_eq!(width(&frame), 104.0);
        assert_eq!(texts(&frame), ["M", "L", "G", "A"]);
    }

    #[test]
    fn the_first_avatar_is_painted_last_so_it_lies_on_top() {
        let frame = frame_of(AvatarGroup::new(people(5), palette()).limit(3));
        let order: Vec<f32> = circles(&frame).iter().map(|rect| rect.x).collect();
        // The count is lowest, then the third, the second and the first.
        assert_eq!(order, [72.0, 48.0, 24.0, 0.0]);
        assert_eq!(texts(&frame), ["+2", "L", "G", "A"]);
    }

    #[test]
    fn a_limit_shows_the_first_and_counts_the_rest_in_a_circle_of_the_same_size() {
        let shown = |count: usize, limit: Option<usize>| {
            let group = AvatarGroup::new(people(count), palette());
            let frame = frame_of(match limit {
                Some(limit) => group.limit(limit),
                None => group,
            });
            let rects = circles(&frame);
            assert!(rects.iter().all(|rect| rect.width == 32.0 && rect.height == 32.0));
            let mut texts = texts(&frame);
            texts.reverse();
            texts
        };
        assert_eq!(shown(5, None), ["A", "G", "L", "M", "B"]);
        assert_eq!(shown(5, Some(3)), ["A", "G", "L", "+2"]);
        // No count when all fit, and a limit of zero is one.
        assert_eq!(shown(3, Some(3)), ["A", "G", "L"]);
        assert_eq!(shown(2, Some(9)), ["A", "G"]);
        assert_eq!(shown(3, Some(0)), ["A", "+2"]);
        assert_eq!(shown(100, Some(1)), ["A", "+99"]);
        assert_eq!(shown(101, Some(1)), ["A", "99+"]);
    }

    #[test]
    fn the_count_is_centered_in_its_circle() {
        for (count, label) in [(4, "+1"), (15, "+12"), (200, "99+")] {
            let frame = frame_of(AvatarGroup::new(people(count), palette()).limit(3).size(40.0));
            let circle = circles(&frame)[0];
            let blocks = frame.text_layouts();
            let text = &blocks.iter().find(|block| block.source == label).expect(label).runs[0];
            assert!((text.rect.x + text.rect.width / 2.0 - circle.x - 20.0).abs() < 0.1, "{label}");
            assert!((text.rect.y + text.rect.height / 2.0 - circle.y - 20.0).abs() < 0.1, "{label}");
            assert!(text.rect.width < 40.0 - 2.0 * RING, "{label}");
        }
    }

    #[test]
    fn the_size_replaces_each_avatars_own_and_bad_sizes_are_clamped_or_ignored() {
        for (size, diameter) in [(48.0, 48.0), (4.0, 16.0), (1000.0, 256.0), (f32::NAN, 32.0), (f32::INFINITY, 32.0)] {
            let avatars = vec![Avatar::new("Ada", palette()).size(96.0), Avatar::new("Grace", palette()), Avatar::new("Linus", palette())];
            let frame = frame_of(AvatarGroup::new(avatars, palette()).limit(2).size(size));
            let rects = circles(&frame);
            assert_eq!(rects.len(), 3);
            assert!(rects.iter().all(|rect| rect.width == diameter && rect.height == diameter), "{size}");
            assert_eq!(width(&frame), diameter * 2.5, "{size}");
        }
    }

    #[test]
    fn the_overlap_is_a_share_of_the_diameter_and_bad_overlaps_are_clamped_or_ignored() {
        for (overlap, step) in [(0.0, 40.0), (0.5, 20.0), (-1.0, 40.0), (0.9, 16.0), (f32::NAN, 30.0), (f32::NEG_INFINITY, 30.0)] {
            let frame = frame_of(AvatarGroup::new(people(3), palette()).size(40.0).overlap(overlap));
            let mut left: Vec<f32> = circles(&frame).iter().map(|rect| rect.x).collect();
            left.reverse();
            for (index, x) in left.iter().enumerate() {
                assert!((x - step * index as f32).abs() < 0.01, "{overlap}: {left:?}");
            }
            assert!((width(&frame) - (step * 2.0 + 40.0)).abs() < 0.01, "{overlap}");
        }
    }

    #[test]
    fn every_circle_has_a_ring_in_the_surface_color_painted_over_it_or_the_color_given() {
        let other = Color::rgb(10, 120, 200);
        for color in [None, Some(other)] {
            let picture = RasterImage::new("a".repeat(64), RasterImageFormat::Png, 4, Arc::from("AAAA"), "A picture").unwrap();
            let avatars = vec![Avatar::new("Ada", palette()).image(picture), Avatar::new("Grace", palette()), Avatar::new("Linus", palette())];
            let group = AvatarGroup::new(avatars, palette()).limit(2);
            let frame = frame_of(match color {
                Some(color) => group.ring(color),
                None => group,
            });
            let ring = color.unwrap_or(palette().surface);
            // What is painted, by its left edge: an opaque disc, the circle or the photo, its ring.
            let painted: Vec<(&str, f32)> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::FillRect { rect, color, .. } if *color == palette().selected => Some(("circle", rect.x)),
                    DrawCommand::FillRect { rect, color, radius } if *color == ring => {
                        assert_eq!((rect.width, *radius), (32.0, 16.0));
                        Some(("disc", rect.x))
                    }
                    DrawCommand::RasterImage { rect, radius, .. } => {
                        assert_eq!((rect.width, *radius), (32.0, 16.0));
                        Some(("photo", rect.x))
                    }
                    DrawCommand::StrokeRect { rect, color, radius, width } => {
                        assert_eq!((*color, *radius, *width, rect.width, rect.height), (ring, 16.0, RING, 32.0, 32.0));
                        Some(("ring", rect.x))
                    }
                    _ => None,
                })
                .collect();
            let layer = |fill: &'static str, x: f32| [("disc", x), (fill, x), ("ring", x)];
            assert_eq!(painted, [layer("circle", 48.0), layer("circle", 24.0), layer("photo", 0.0)].concat());
        }
    }

    #[test]
    fn the_group_is_passive_and_says_nothing_until_it_has_an_id_and_a_name() {
        let group = || AvatarGroup::new(people(5), palette()).limit(3);
        for silent in [group(), group().id("team"), group().accessibility_label("The team")] {
            let frame = frame_of(silent);
            let ids: Vec<_> = frame.access.iter().map(|node| node.id.as_str()).collect();
            assert_eq!(ids, ["after"]);
        }
        let mut tree = UiTree::new(Element::row(vec![
            group().id("team").accessibility_label("Ada, Grace, Linus and 2 others").render(),
            Element::spacer().flex(1.0),
        ]));
        let frame = tree.layout(Size::new(600.0, 300.0));
        let [said] = frame.access.as_slice() else {
            panic!("one node, got {:?}", frame.access);
        };
        assert_eq!((said.id.as_str(), said.role, said.name.as_str()), ("team", Role::Image, "Ada, Grace, Linus and 2 others"));
        assert_eq!((said.actionable, said.disabled), (false, false));
        assert_eq!(said.rect, Rect::new(0.0, 0.0, 104.0, 32.0));
        // No region, no tab stop, nothing reported.
        assert!(frame.regions.is_empty());
        let point = Point::new(30.0, 16.0);
        tree.handle_event(UiEvent::PointerMove(point));
        assert_eq!(tree.hovered_id(), None);
        assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
        assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
        assert!(tree.handle_event(UiEvent::Key { key: Key::Tab, shift: false, ctrl: false }).is_empty());
        assert_eq!(tree.focused_id(), None);
    }

    #[test]
    fn one_avatar_is_itself_with_a_ring_and_an_empty_group_is_nothing() {
        let one = frame_of(AvatarGroup::new(people(1), palette()));
        assert_eq!(circles(&one), [Rect::new(0.0, 0.0, 32.0, 32.0)]);
        assert_eq!(width(&one), 32.0);
        let none = frame_of(AvatarGroup::new(Vec::new(), palette()).limit(3).id("team").accessibility_label("Nobody"));
        assert_eq!(width(&none), 0.0);
        assert!(none.commands.iter().all(|command| !matches!(command, DrawCommand::FillRect { .. } | DrawCommand::StrokeRect { .. })));
        assert!(none.access.iter().all(|node| node.id != "team"));
    }
}
