//! A row of a list: an icon, a title, a second line, and what sits at its
//! trailing edge, all under one hover surface.

use crate::{
    page::px, Access, Color, Component, ComponentPalette, Element, FontFamily, Icon, RadiusRole,
    Role, Text, TooltipSpec,
};

// Width over which a faded title goes transparent.
const TITLE_FADE: f32 = 24.0;

/// Density of a [`ListRow`], in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowSizing {
    /// Height of a row with a title alone; `None` follows the content.
    pub height: Option<f32>,
    /// Height of a row with a subtitle; `None` follows the content.
    pub two_line_height: Option<f32>,
    /// Space around the content, inside the hover surface.
    pub padding: f32,
    /// Space between the icon, the copy and the trailing elements.
    pub gap: f32,
    /// Width of the icon column. The glyph is centered in the row's height.
    pub icon_size: f32,
    /// Side of the square a leading element is held in, at the row's top.
    pub leading_width: f32,
    /// Height of the box the icon is centered in, or the leading element
    /// held in, from the top of the row; `None` centers the icon in the row
    /// and holds the leading element in a square.
    pub leading_height: Option<f32>,
    /// Width of the column of the trailing icon.
    pub trailing_icon_size: f32,
    /// Font size of the title.
    pub title_size: f32,
    /// Font size of the subtitle.
    pub subtitle_size: f32,
    /// Space between the title and the subtitle.
    pub copy_gap: f32,
    /// Font size of the meta text beside the title.
    pub meta_size: f32,
    /// Space between the title and its meta.
    pub meta_gap: f32,
    /// Height of the box the title is centered in, from the top of the row;
    /// `None` follows the font, and centers a title alone in the row.
    pub title_height: Option<f32>,
    /// The same for the subtitle.
    pub subtitle_height: Option<f32>,
}

impl RowSizing {
    /// A list in a panel or a popover: 28 high, 40 with a subtitle.
    pub const COMPACT: Self = Self {
        height: Some(28.0),
        two_line_height: Some(40.0),
        padding: 4.0,
        gap: 8.0,
        icon_size: 16.0,
        leading_width: 24.0,
        leading_height: None,
        trailing_icon_size: 16.0,
        title_size: 12.0,
        subtitle_size: 11.0,
        copy_gap: 2.0,
        meta_size: 11.0,
        meta_gap: 8.0,
        title_height: None,
        subtitle_height: None,
    };

    /// A long list of short entries, such as the frames of a debugger.
    pub const DENSE: Self = Self {
        height: Some(24.0),
        two_line_height: Some(36.0),
        padding: 2.0,
        gap: 6.0,
        leading_width: 20.0,
        copy_gap: 0.0,
        ..Self::COMPACT
    };
}

/// One entry of a list. The host owns the selection and the action: the row
/// reports a click on its ID and paints the palette it is given, hovering to
/// `palette.hover` and pressing to `palette.selected`.
pub struct ListRow {
    id: String,
    title: String,
    subtitle: String,
    subtitle_mono: bool,
    subtitle_ink: Option<Color>,
    copy_width: Option<f32>,
    outlined: bool,
    icon: Option<Icon>,
    icon_ink: Option<Color>,
    leading: Option<Element>,
    meta: Option<String>,
    meta_elements: Vec<Element>,
    trailing: Option<Element>,
    trailing_icon: Option<Icon>,
    trailing_actions: Vec<(String, Element)>,
    title_ink: Option<Color>,
    title_fade: bool,
    hover_with: Option<String>,
    enabled: bool,
    filled: bool,
    radius: RadiusRole,
    width: Option<f32>,
    tooltip: Option<TooltipSpec>,
    sizing: RowSizing,
    palette: ComponentPalette,
}

impl ListRow {
    /// A title alone, in the [`RowSizing::COMPACT`] profile.
    pub fn new(id: impl Into<String>, title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            subtitle: String::new(),
            subtitle_mono: false,
            subtitle_ink: None,
            copy_width: None,
            outlined: false,
            icon: None,
            icon_ink: None,
            leading: None,
            meta: None,
            meta_elements: Vec::new(),
            trailing: None,
            trailing_icon: None,
            trailing_actions: Vec::new(),
            title_ink: None,
            title_fade: false,
            hover_with: None,
            enabled: true,
            filled: false,
            radius: RadiusRole::Control,
            width: None,
            tooltip: None,
            sizing: RowSizing::COMPACT,
            palette,
        }
    }

    /// A muted glyph before the copy, centered in the row's height.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Paint the icon in a color the host owns instead of the muted one.
    pub fn icon_ink(mut self, color: Color) -> Self {
        self.icon_ink = Some(color);
        self
    }

    /// A caller-owned element before the copy, such as a thumbnail, held in
    /// a square of the profile's `leading_width`. Replaces the icon.
    pub fn leading(mut self, leading: Element) -> Self {
        self.leading = Some(leading);
        self
    }

    /// A muted second line under the title. An empty one is no line at all.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    /// Set the subtitle in the monospaced family, for a command or a path.
    pub fn subtitle_mono(mut self, mono: bool) -> Self {
        self.subtitle_mono = mono;
        self
    }

    /// Paint the subtitle in a color that carries meaning the host owns,
    /// such as a warning, instead of the muted one.
    pub fn subtitle_ink(mut self, color: Color) -> Self {
        self.subtitle_ink = Some(color);
        self
    }

    /// Hold the copy at a fixed width instead of the width the row has
    /// left, for a host that budgets the columns of its rows itself. Values
    /// that are not finite are ignored.
    pub fn copy_width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.copy_width = Some(width.max(0.0));
        }
        self
    }

    /// Muted text at the end of the title's line, such as a status or an age.
    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    /// A caller-owned element in the meta position, such as a [`DiffStat`].
    /// Each call adds one more, after the meta text.
    pub fn meta_element(mut self, meta: Element) -> Self {
        self.meta_elements.push(meta);
        self
    }

    /// A caller-owned element after the copy, across the row's height.
    pub fn trailing(mut self, trailing: Element) -> Self {
        self.trailing = Some(trailing);
        self
    }

    /// A muted glyph at the trailing edge, such as the chevron of a row that
    /// opens a menu or another page.
    pub fn trailing_icon(mut self, icon: Icon) -> Self {
        self.trailing_icon = Some(icon);
        self
    }

    /// A control with a hit target of its own after the row's, such as the
    /// button that stages a file, reporting `id`. Each call adds one more.
    /// The row and its actions then share one hover surface: the fill is
    /// painted behind all of them whichever the pointer is over, so the
    /// caller draws the control with no hover fill of its own. The hint
    /// covers the row's own target alone.
    pub fn trailing_action(mut self, id: impl Into<String>, control: Element) -> Self {
        self.trailing_actions.push((id.into(), control));
        self
    }

    /// Paint the title in a color that carries meaning the host owns, such
    /// as unread or failed. It holds while the row is disabled.
    pub fn title_ink(mut self, color: Color) -> Self {
        self.title_ink = Some(color);
        self
    }

    /// Fade the end of a title that does not fit instead of cutting it with
    /// an ellipsis, for a title that changes while it is on screen.
    pub fn title_fade(mut self, fade: bool) -> Self {
        self.title_fade = fade;
        self
    }

    /// Keep the row hovered while the pointer is over the control `id`, for
    /// a trailing element that takes the pointer from the row under it.
    pub fn hover_with(mut self, id: impl Into<String>) -> Self {
        self.hover_with = Some(id.into());
        self
    }

    /// A disabled row is passive: a muted title and no hit region.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Fill the row with the palette's surface at rest, for a row that
    /// stands on its own instead of on the surface of its list.
    pub fn filled(mut self) -> Self {
        self.filled = true;
        self
    }

    /// Draw a hairline in the palette's border color around the row, for a
    /// row that is a card of its own.
    pub fn outlined(mut self) -> Self {
        self.outlined = true;
        self
    }

    /// The corners of the row's fill and of its hover, by role.
    /// [`RadiusRole::Control`] unless set.
    pub fn radius(mut self, role: RadiusRole) -> Self {
        self.radius = role;
        self
    }

    /// A fixed width, which also clips the content to it. Values that are
    /// not finite are ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// The hint shown over the row, enabled or not.
    pub fn tooltip_with(mut self, spec: TooltipSpec) -> Self {
        self.tooltip = Some(spec);
        self
    }

    /// The row's density. [`RowSizing::COMPACT`] unless set.
    pub fn sizing(mut self, sizing: RowSizing) -> Self {
        self.sizing = sizing;
        self
    }
}

impl Component for ListRow {
    fn render(self) -> Element {
        let sizing = self.sizing;
        let palette = self.palette;
        let fade = self.title_fade;
        let line = |value: String, size: f32, mono: bool, ink: Color, height: Option<f32>| {
            let text = Element::truncated_text(value)
                .font_family(if mono { FontFamily::Monospace } else { FontFamily::Sans })
                .font_size(size)
                .foreground(ink);
            match height {
                Some(height) => text.size(None, Some(px(height))),
                None => text,
            }
        };
        let two_lines = !self.subtitle.is_empty();
        // A row that can be clicked is named by all it shows; a passive one
        // has to be told its lines.
        let mut disabled = Access::named(&self.title);
        if two_lines {
            disabled = disabled.description(&self.subtitle);
        }
        let ink = self
            .title_ink
            .unwrap_or(if self.enabled { palette.text } else { palette.muted });
        let mut title = if fade {
            // A faded title may shrink below its own width, as a cut one does.
            let title = Element::faded_text(self.title, TITLE_FADE)
                .font_family(FontFamily::Sans)
                .font_size(sizing.title_size)
                .foreground(ink)
                .min_size(0.0, 0.0);
            match sizing.title_height {
                Some(height) => title.size(None, Some(px(height))),
                None => title,
            }
        } else {
            line(self.title, sizing.title_size, false, ink, sizing.title_height)
        };
        let meta = self.meta.map(|meta| {
            let meta = Element::text(meta)
                .font_family(FontFamily::Sans)
                .font_size(sizing.meta_size)
                .foreground(palette.muted);
            // In a boxed title line it starts at the top of the box, as the
            // rows this replaced had it; a free line centers it.
            match sizing.title_height {
                Some(_) => meta,
                None => meta.centered_y(),
            }
        });
        let meta: Vec<Element> = meta.into_iter().chain(self.meta_elements).collect();
        if !meta.is_empty() {
            title = Element::row(std::iter::once(title.flex(1.0)).chain(meta).collect())
                .gap(px(sizing.meta_gap))
                .size(None, sizing.title_height.map(px));
        }
        // A title alone with no box of its own takes the row's height, which
        // centers it; any other copy is stacked from the top.
        let copy = if two_lines || sizing.title_height.is_some() {
            let mut lines = vec![title];
            if two_lines {
                lines.push(line(
                    self.subtitle,
                    sizing.subtitle_size,
                    self.subtitle_mono,
                    self.subtitle_ink.unwrap_or(palette.muted),
                    sizing.subtitle_height,
                ));
            }
            Element::column(lines).gap(px(sizing.copy_gap))
        } else {
            title
        };
        let mut content = Vec::new();
        if let Some(leading) = self.leading {
            let side = px(sizing.leading_width);
            let height = sizing.leading_height.map_or(side, px);
            content.push(leading.size(Some(side), Some(height)));
        } else if let Some(icon) = self.icon {
            content.push(
                Element::icon(icon)
                    .foreground(self.icon_ink.unwrap_or(palette.muted))
                    .size(Some(px(sizing.icon_size)), sizing.leading_height.map(px)),
            );
        }
        content.push(match self.copy_width {
            Some(width) => copy.size(Some(width), None),
            None => copy.flex(1.0),
        });
        content.extend(self.trailing);
        if let Some(icon) = self.trailing_icon {
            content.push(
                Element::icon(icon)
                    .foreground(palette.muted)
                    .size(Some(px(sizing.trailing_icon_size)), None),
            );
        }
        let mut content = Element::row(content).gap(px(sizing.gap));
        if self.width.is_some() {
            content = content.clip_children();
        }
        let height = if two_lines { sizing.two_line_height } else { sizing.height };
        let height = height.filter(|height| height.is_finite()).map(px);
        let shared = !self.trailing_actions.is_empty();
        let target = match (self.enabled, shared) {
            // Under a shared surface the target itself paints nothing.
            (true, true) => Element::clickable(&self.id, content)
                .background(Color::TRANSPARENT)
                .transition_ms(0),
            (true, false) => palette.interactive_surface(Element::clickable(&self.id, content)),
            (false, _) => Element::panel(content).accessible(disabled.id(&self.id).disabled(true)),
        }
        .accessible(Access::new(Role::ListItem))
        .padding(px(sizing.padding));
        let mut tooltip = self.tooltip;
        let mut hint = |row: Element| match tooltip.take() {
            Some(spec) => spec.wrap(row, self.id.clone(), palette),
            None => row,
        };
        let mut row = if shared {
            let ids: Vec<String> = self
                .enabled
                .then(|| self.id.clone())
                .into_iter()
                .chain(self.trailing_actions.iter().map(|(id, _)| id.clone()))
                .collect();
            let target = hint(target.size(None, height)).flex(1.0);
            let actions = self.trailing_actions.into_iter().map(|(_, control)| control);
            Element::row(std::iter::once(target).chain(actions).collect())
                .gap(0.0)
                .hover_background(palette.hover)
                .pressed_background(palette.selected)
                .hover_surface(format!("{}:surface", self.id), ids)
                .transition_ms(crate::tokens().motion.hover_ms)
        } else {
            target
        }
        .size(self.width, height)
        .radius(self.radius.px());
        if self.filled {
            row = row.background(palette.surface);
        }
        if self.outlined {
            row = row.border(palette.border, 1.0);
        }
        if let Some(peer) = self.hover_with.filter(|_| self.enabled && !shared) {
            row = row.hover_with(peer);
        }
        if shared {
            row
        } else {
            hint(row)
        }
    }
}

/// The lines a change adds and removes, as `+N` and `−M`.
pub struct DiffStat {
    additions: usize,
    deletions: usize,
    added: Color,
    removed: Color,
    palette: ComponentPalette,
}

impl DiffStat {
    /// Both counts in the palette's text color until [`DiffStat::inks`] says
    /// which colors mean added and removed.
    pub fn new(additions: usize, deletions: usize, palette: ComponentPalette) -> Self {
        Self {
            additions,
            deletions,
            added: palette.text,
            removed: palette.text,
            palette,
        }
    }

    /// The host's colors for added and for removed lines.
    pub fn inks(mut self, added: Color, removed: Color) -> Self {
        self.added = added;
        self.removed = removed;
        self
    }
}

impl Component for DiffStat {
    fn render(self) -> Element {
        // Closer than two labels would sit: the counts read as one figure.
        const GAP: f32 = 5.0;
        Element::row(vec![
            Text::new(format!("+{}", self.additions), self.palette).ink(self.added).render(),
            Text::new(format!("\u{2212}{}", self.deletions), self.palette).ink(self.removed).render(),
        ])
        .gap(GAP)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, ElementKind, Frame, Point, Rect, Size, UiAction, UiEvent, UiTree};

    const TITLE: &str = "Cambios del repositorio";
    const WHY: &str = "crates/limn/src/list_row.rs";

    fn palette() -> ComponentPalette {
        ComponentPalette::neutral_dark()
    }

    fn tree_of(row: Element) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            row,
            Element::button("after", "Después").size(None, Some(30.0)),
        ]));
        tree.set_reduced_motion(true);
        tree
    }

    fn region(frame: &Frame, id: &str) -> Rect {
        frame.regions.iter().find(|region| region.id == id).unwrap().rect
    }

    fn texts(frame: &Frame) -> Vec<(&str, Point, f32, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Text { text, position, font_size, color, .. } if text != "Después" => {
                    Some((text.as_str(), *position, *font_size, *color))
                }
                _ => None,
            })
            .collect()
    }

    fn icons(frame: &Frame) -> Vec<(Rect, Color)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Icon { rect, tint, .. } => Some((*rect, *tint)),
                _ => None,
            })
            .collect()
    }

    fn fills(frame: &Frame, color: Color) -> Vec<(Rect, f32)> {
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, color: fill, radius } if *fill == color => {
                    Some((*rect, *radius))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_row_tells_the_platform_it_is_an_item_of_a_list_enabled_or_not() {
        let frame = tree_of(Element::column(vec![
            ListRow::new("open", TITLE, palette()).subtitle(WHY).meta("2 h").render(),
            ListRow::new("shared", TITLE, palette())
                .trailing_action("more", Element::icon_button("more", Icon::More, ""))
                .render(),
            ListRow::new("gone", TITLE, palette()).subtitle(WHY).disabled().render(),
        ]))
        .layout(Size::new(360.0, 240.0));
        let node = |id: &str| frame.access.iter().find(|node| node.id == id).unwrap_or_else(|| panic!("no node {id}"));

        let open = node("open");
        assert_eq!(open.role, Role::ListItem);
        assert!(open.name.starts_with(TITLE) && open.name.contains(WHY) && open.name.contains("2 h"), "{}", open.name);
        assert!(open.actionable && !open.disabled);
        assert_eq!((node("shared").role, node("more").role), (Role::ListItem, Role::Button));
        let gone = node("gone");
        assert_eq!((gone.role, gone.name.as_str(), gone.description.as_deref()), (Role::ListItem, TITLE, Some(WHY)));
        assert!(gone.disabled && !gone.actionable);
    }

    #[test]
    fn presets_take_their_heights_and_sizes_from_the_profile() {
        for sizing in [RowSizing::COMPACT, RowSizing::DENSE] {
            let one = ListRow::new("row", TITLE, palette()).icon(Icon::Code).sizing(sizing);
            let frame = tree_of(one.render()).layout(Size::new(320.0, 200.0));
            let row = region(&frame, "row");
            assert_eq!(row, Rect::new(0.0, 0.0, 320.0, sizing.height.unwrap()));
            // The glyph and the title share the row's middle.
            let (icon, tint) = icons(&frame)[0];
            assert_eq!(tint, palette().muted);
            assert_eq!(icon.x + icon.width / 2.0, sizing.padding + sizing.icon_size / 2.0);
            assert_eq!(icon.y + icon.height / 2.0, row.height / 2.0);
            let title = texts(&frame)[0];
            assert_eq!(title.1.x, sizing.padding + sizing.icon_size + sizing.gap);
            assert_eq!((title.2, title.3), (sizing.title_size, palette().text));
            assert!(title.1.y > sizing.padding && title.1.y + title.2 < row.height - sizing.padding);

            let two = ListRow::new("row", TITLE, palette()).icon(Icon::Code).subtitle(WHY).sizing(sizing);
            let frame = tree_of(two.render()).layout(Size::new(320.0, 200.0));
            let row = region(&frame, "row");
            assert_eq!(row.height, sizing.two_line_height.unwrap());
            assert_eq!(region(&frame, "after").y, row.height);
            let lines = texts(&frame);
            assert_eq!(lines[0].1.y, sizing.padding);
            assert_eq!((lines[1].2, lines[1].3), (sizing.subtitle_size, palette().muted));
            assert!(lines[1].1.y >= lines[0].1.y + lines[0].2 + sizing.copy_gap);
            assert!(lines[1].1.y + lines[1].2 <= row.height - sizing.padding);
            assert_eq!(lines[1].1.x, lines[0].1.x);
        }
    }

    #[test]
    fn a_click_reports_the_id_and_a_disabled_row_is_passive() {
        let mut tree = tree_of(ListRow::new("row", TITLE, palette()).subtitle(WHY).render());
        let frame = tree.layout(Size::new(240.0, 120.0));
        let point = Point::new(120.0, region(&frame, "row").height / 2.0);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(
            tree.handle_event(UiEvent::PointerUp(point)),
            vec![UiAction::Clicked { id: "row".into() }]
        );

        let disabled = ListRow::new("row", TITLE, palette()).subtitle(WHY).enabled(false);
        let frame = tree_of(disabled.render()).layout(Size::new(240.0, 120.0));
        assert!(frame.regions.iter().all(|region| region.id != "row"));
        assert_eq!(texts(&frame)[0].3, palette().muted);
        // The same height as the enabled row, so the list does not move.
        assert_eq!(region(&frame, "after").y, 40.0);
        let failed = Color::rgb(255, 92, 92);
        let inked = ListRow::new("row", TITLE, palette()).title_ink(failed).enabled(false);
        let frame = tree_of(inked.render()).layout(Size::new(240.0, 120.0));
        assert_eq!(texts(&frame)[0].3, failed);
    }

    #[test]
    fn hover_and_press_paint_the_palette_on_the_whole_row() {
        let palette = ComponentPalette {
            surface: Color::rgb(30, 31, 32),
            hover: Color::rgb(50, 51, 52),
            selected: Color::rgb(70, 71, 72),
            ..palette()
        };
        let row = || ListRow::new("row", TITLE, palette).icon(Icon::Code).width(200.0);
        let mut tree = tree_of(row().render());
        let size = Size::new(300.0, 120.0);
        let rest = tree.layout(size);
        assert!(fills(&rest, palette.surface).is_empty(), "a row sits on its list's surface");
        let whole = (Rect::new(0.0, 0.0, 200.0, 28.0), RadiusRole::Control.px());
        let point = Point::new(100.0, 14.0);
        tree.handle_event(UiEvent::PointerMove(point));
        assert_eq!(fills(&tree.layout(size), palette.hover), vec![whole]);
        tree.handle_event(UiEvent::PointerDown(point));
        assert_eq!(fills(&tree.layout(size), palette.selected), vec![whole]);

        let filled = tree_of(row().filled().render()).layout(size);
        assert_eq!(fills(&filled, palette.surface), vec![whole]);
        // The role rounds the fill and the hover alike.
        let rounder = (whole.0, RadiusRole::Surface.px());
        let mut tree = tree_of(row().filled().radius(RadiusRole::Surface).render());
        assert_eq!(fills(&tree.layout(size), palette.surface), vec![rounder]);
        tree.handle_event(UiEvent::PointerMove(point));
        assert_eq!(fills(&tree.layout(size), palette.hover), vec![rounder]);
        let passive = tree_of(row().filled().enabled(false).render()).layout(size);
        assert_eq!(fills(&passive, palette.surface), vec![whole]);
    }

    #[test]
    fn meta_shares_the_title_line_and_trailing_elements_end_the_row() {
        let sizing = RowSizing::COMPACT;
        let row = ListRow::new("row", TITLE, palette())
            .icon(Icon::Code)
            .subtitle(WHY)
            .meta("hace 2 min")
            .meta_element(DiffStat::new(4, 1, palette()).render())
            .trailing(Element::button("more", "Más").size(Some(40.0), None))
            .trailing_icon(Icon::ChevronRight)
            .width(360.0);
        let frame = tree_of(row.render()).layout(Size::new(400.0, 120.0));
        let lines = texts(&frame);
        let (title, meta, subtitle) = (lines[0], lines[1], lines[4]);
        // The caller's own meta follows the text on the same line.
        assert_eq!((lines[2].0, lines[3].0), ("+4", "\u{2212}1"));
        assert!(lines[2].1.x > meta.1.x && lines[3].1.y < subtitle.1.y);
        assert_eq!((meta.0, meta.2, meta.3), ("hace 2 min", sizing.meta_size, palette().muted));
        assert!(meta.1.x > title.1.x && meta.1.y < subtitle.1.y);
        // Smaller than the title and centered on its line.
        assert!(meta.1.y > title.1.y && meta.1.y + meta.2 < subtitle.1.y);
        let more = region(&frame, "more");
        let chevron = icons(&frame)[1].0;
        // Copy, then the caller's control across the content, then the glyph.
        assert!(lines[3].1.x < more.x);
        assert_eq!((more.y, more.height), (sizing.padding, 40.0 - 2.0 * sizing.padding));
        assert_eq!(more.x + more.width + sizing.gap, 360.0 - sizing.padding - sizing.trailing_icon_size);
        assert_eq!(chevron.x + chevron.width / 2.0, 360.0 - sizing.padding - sizing.trailing_icon_size / 2.0);
        assert_eq!(chevron.y + chevron.height / 2.0, 20.0);
        // Each control keeps its own action.
        assert_eq!(frame.hit_test(Point::new(more.x + 4.0, 20.0)), Some("more"));
        assert_eq!(frame.hit_test(Point::new(60.0, 20.0)), Some("row"));
    }

    #[test]
    fn a_leading_element_is_a_square_at_the_top_and_replaces_the_icon() {
        let mark = Color::rgb(1, 2, 3);
        let row = ListRow::new("row", TITLE, palette())
            .icon(Icon::Code)
            .leading(Element::spacer().background(mark))
            .subtitle(WHY);
        let frame = tree_of(row.render()).layout(Size::new(300.0, 120.0));
        assert_eq!(fills(&frame, mark), vec![(Rect::new(4.0, 4.0, 24.0, 24.0), 0.0)]);
        assert!(icons(&frame).is_empty());
        assert_eq!(texts(&frame)[0].1.x, 4.0 + 24.0 + 8.0);
    }

    #[test]
    fn the_profile_can_box_the_icon_and_the_leading_element_from_the_top() {
        let sizing = RowSizing { height: Some(40.0), leading_height: Some(22.0), ..RowSizing::COMPACT };
        let row = ListRow::new("row", TITLE, palette()).icon(Icon::Code).sizing(sizing);
        let frame = tree_of(row.render()).layout(Size::new(300.0, 120.0));
        let glyph = icons(&frame)[0].0;
        assert_eq!(glyph.y + glyph.height / 2.0, 4.0 + 11.0);
        let mark = Color::rgb(1, 2, 3);
        let row = ListRow::new("row", TITLE, palette())
            .leading(Element::spacer().background(mark))
            .sizing(sizing);
        let frame = tree_of(row.render()).layout(Size::new(300.0, 120.0));
        assert_eq!(fills(&frame, mark), vec![(Rect::new(4.0, 4.0, 24.0, 22.0), 0.0)]);
    }

    #[test]
    fn a_faded_title_is_never_cut_with_an_ellipsis() {
        let row = |fade| ListRow::new("row", TITLE, palette()).title_fade(fade).width(80.0).render();
        let cut = tree_of(row(false)).layout(Size::new(300.0, 120.0));
        assert!(texts(&cut)[0].0.ends_with('…'));
        let faded = tree_of(row(true)).layout(Size::new(300.0, 120.0));
        let bands = texts(&faded);
        // The whole line is painted in bands of falling opacity.
        assert!(bands.len() > 1 && bands.iter().all(|band| band.0 == TITLE));
        assert!(bands.iter().all(|band| band.1 == bands[0].1 && band.2 == RowSizing::COMPACT.title_size));
        // With room to spare it is one plain line where the cut one is.
        let wide = |fade| {
            let row = ListRow::new("row", TITLE, palette()).title_fade(fade).render();
            let frame = tree_of(row).layout(Size::new(600.0, 120.0));
            texts(&frame).iter().map(|line| (line.0.to_owned(), line.1)).collect::<Vec<_>>()
        };
        assert_eq!(wide(true), wide(false));
    }

    #[test]
    fn hover_with_keeps_the_row_hovered_under_its_trailing_control() {
        let palette = ComponentPalette { hover: Color::rgb(50, 51, 52), ..palette() };
        let row = |peer: bool| {
            let row = ListRow::new("row", TITLE, palette)
                .trailing(Element::button("more", "Más").size(Some(40.0), None))
                .width(200.0);
            if peer { row.hover_with("more") } else { row }.render()
        };
        let size = Size::new(300.0, 120.0);
        for peer in [false, true] {
            let mut tree = tree_of(row(peer));
            let more = region(&tree.layout(size), "more");
            tree.handle_event(UiEvent::PointerMove(Point::new(more.x + 4.0, 14.0)));
            let frame = tree.layout(size);
            let whole = fills(&frame, palette.hover)
                .iter()
                .any(|(rect, _)| *rect == Rect::new(0.0, 0.0, 200.0, 28.0));
            assert_eq!(whole, peer);
            assert_eq!(frame.hit_test(Point::new(more.x + 4.0, 14.0)), Some("more"));
        }
        // A passive row has nothing to hover.
        let passive = ListRow::new("row", TITLE, palette).hover_with("more").enabled(false).render();
        assert_eq!(passive.hover_peer, None);
    }

    #[test]
    fn trailing_actions_keep_their_targets_under_one_shared_hover_surface() {
        let palette = ComponentPalette {
            surface: Color::rgb(30, 31, 32),
            hover: Color::rgb(50, 51, 52),
            selected: Color::rgb(70, 71, 72),
            ..palette()
        };
        let stage = || Element::clickable("stage", Element::icon(Icon::Add)).size(Some(24.0), None);
        let row = || {
            ListRow::new("row", TITLE, palette)
                .icon(Icon::Code)
                .trailing_action("stage", stage())
                .tooltip_with(TooltipSpec::new("Abrir cambios"))
                .filled()
                .width(200.0)
        };
        let size = Size::new(300.0, 120.0);
        let whole = (Rect::new(0.0, 0.0, 200.0, 28.0), RadiusRole::Control.px());
        let mut tree = tree_of(row().render());
        let rest = tree.layout(size);
        assert_eq!(fills(&rest, palette.surface), vec![whole]);
        // Two targets that meet without overlapping.
        let (open, action) = (region(&rest, "row"), region(&rest, "stage"));
        assert_eq!((open.x, open.width, action.x, action.width), (0.0, 176.0, 176.0, 24.0));
        for (point, id) in [(Point::new(60.0, 14.0), "row"), (Point::new(188.0, 14.0), "stage")] {
            assert_eq!(rest.hit_test(point), Some(id));
            tree.handle_event(UiEvent::PointerMove(point));
            let hovered = tree.layout(size);
            // One fill, the whole row's, whichever target the pointer is over.
            assert_eq!(fills(&hovered, palette.hover), vec![whole], "{id}");
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(fills(&tree.layout(size), palette.selected), vec![whole], "{id}");
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked { id: id.into() }]
            );
        }
        // The hint belongs to the row's own target, not to its actions.
        let ElementKind::Row(children) = &row().render().kind else {
            panic!("the shared surface is a row of targets");
        };
        assert!(matches!(&children[0].kind, ElementKind::Tooltip { anchor_id, .. } if anchor_id == "row"));
        // Disabled, the row is passive and its action still answers.
        let passive = ListRow::new("row", TITLE, palette).trailing_action("stage", stage()).enabled(false);
        let frame = tree_of(passive.width(200.0).render()).layout(size);
        assert!(frame.regions.iter().all(|region| region.id != "row"));
        assert_eq!(frame.hit_test(Point::new(188.0, 14.0)), Some("stage"));
    }

    #[test]
    fn a_narrow_row_cuts_its_copy_and_paints_nothing_outside() {
        let ink = Color::rgb(1, 2, 3);
        for width in [60.0, 120.0, 400.0] {
            let row = ListRow::new("row", TITLE, palette())
                .icon(Icon::Code)
                .icon_ink(ink)
                .subtitle(WHY)
                .subtitle_mono(true)
                .trailing_icon(Icon::ChevronDown)
                .width(width);
            let frame = tree_of(row.render()).layout(Size::new(600.0, 120.0));
            let lines = texts(&frame);
            assert_eq!(lines.len(), 2);
            // The host's ink is the leading glyph's alone, and only the subtitle is mono.
            let tints: Vec<Color> = icons(&frame).iter().map(|icon| icon.1).collect();
            assert_eq!(tints, [ink, palette().muted]);
            let families: Vec<FontFamily> = frame
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, font_family, .. } if text != "Después" => Some(*font_family),
                    _ => None,
                })
                .collect();
            assert_eq!(families, [FontFamily::Sans, FontFamily::Monospace]);
            if width < 400.0 {
                assert!(lines.iter().all(|line| line.0.ends_with('…') || line.0.is_empty()));
            }
            assert!(icons(&frame).iter().all(|(rect, _)| rect.x + rect.width <= width));
            assert!(frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::PushClip(rect) if rect.x + rect.width <= width
            )));
        }
    }

    #[test]
    fn line_boxes_stack_from_the_top_and_a_free_height_follows_them() {
        let sizing = RowSizing {
            height: None,
            two_line_height: None,
            padding: 9.0,
            title_height: Some(18.0),
            subtitle_height: Some(16.0),
            ..RowSizing::COMPACT
        };
        let two = ListRow::new("row", TITLE, palette()).icon(Icon::Search).subtitle(WHY).sizing(sizing);
        let frame = tree_of(two.render()).layout(Size::new(300.0, 200.0));
        assert_eq!(region(&frame, "row").height, 9.0 + 18.0 + 2.0 + 16.0 + 9.0);
        let lines = texts(&frame);
        assert!(lines[0].1.y > 9.0 && lines[0].1.y + 12.0 < 27.0);
        assert!(lines[1].1.y > 29.0 && lines[1].1.y + 11.0 < 45.0);
        // A boxed title alone stays at the top of a taller row.
        let boxed = RowSizing { height: Some(40.0), padding: 4.0, ..sizing };
        let frame = tree_of(ListRow::new("row", TITLE, palette()).sizing(boxed).render())
            .layout(Size::new(300.0, 200.0));
        assert!(texts(&frame)[0].1.y + 12.0 < 22.0);
    }

    #[test]
    fn bad_numbers_are_ignored() {
        let row = ListRow::new("row", TITLE, palette()).width(f32::NAN).width(f32::INFINITY);
        assert_eq!(row.width, None);
        assert_eq!(ListRow::new("row", TITLE, palette()).width(-8.0).width, Some(0.0));
        let sizing = RowSizing {
            height: Some(f32::NAN),
            two_line_height: Some(-10.0),
            padding: f32::NEG_INFINITY,
            gap: f32::NAN,
            icon_size: -4.0,
            ..RowSizing::COMPACT
        };
        for subtitle in ["", WHY] {
            let row = ListRow::new("row", TITLE, palette()).icon(Icon::Code).subtitle(subtitle).sizing(sizing);
            let frame = tree_of(row.render()).layout(Size::new(200.0, 120.0));
            for command in &frame.commands {
                if let DrawCommand::Text { position, .. } = command {
                    assert!(position.x.is_finite() && position.y.is_finite() && position.x >= 0.0);
                }
            }
        }
    }

    #[test]
    fn the_tooltip_wraps_the_row_enabled_or_not() {
        for enabled in [true, false] {
            let row = ListRow::new("row", TITLE, palette())
                .enabled(enabled)
                .tooltip_with(TooltipSpec::new("Abrir cambios").description(WHY));
            let ElementKind::Tooltip { anchor_id, title, .. } = row.render().kind else {
                panic!("the row is the anchor of its hint");
            };
            assert_eq!((anchor_id.as_str(), title.as_str()), ("row", "Abrir cambios"));
        }
    }

    #[test]
    fn subtitle_ink_replaces_the_muted_color_of_the_second_line() {
        let warning = Color::rgb(230, 180, 80);
        let row = ListRow::new("row", TITLE, palette()).subtitle(WHY).subtitle_ink(warning);
        let frame = tree_of(row.render()).layout(Size::new(360.0, 80.0));
        let lines = texts(&frame);
        assert_eq!((lines[0].3, lines[1].3), (palette().text, warning));
    }

    #[test]
    fn copy_width_holds_the_copy_whatever_the_row_has_left() {
        let row = |width| {
            Element::row(vec![ListRow::new("row", TITLE, palette())
                .trailing_icon(Icon::ChevronRight)
                .copy_width(100.0)
                .render()
                .size(Some(width), None)])
        };
        let chevron = |width| icons(&tree_of(row(width)).layout(Size::new(400.0, 80.0)))[0].0.x;
        // Padding 4, the copy, the gap: the chevron does not follow the edge.
        assert_eq!(chevron(300.0), 112.0);
        assert_eq!(chevron(90.0), 112.0);
        let bad = ListRow::new("row", TITLE, palette()).copy_width(f32::NAN).copy_width(-5.0);
        assert_eq!(bad.copy_width, Some(0.0));
    }

    #[test]
    fn outlined_draws_a_hairline_in_the_border_color() {
        let borders = |row: ListRow| {
            tree_of(row.render())
                .layout(Size::new(360.0, 80.0))
                .commands
                .iter()
                .filter(|command| matches!(command,
                    DrawCommand::StrokeRect { color, width, .. }
                        if *color == palette().border && *width == 1.0))
                .count()
        };
        assert_eq!(borders(ListRow::new("row", TITLE, palette())), 0);
        assert_eq!(borders(ListRow::new("row", TITLE, palette()).outlined()), 1);
    }

    #[test]
    fn diff_stat_writes_both_counts_in_the_host_inks() {
        let (added, removed) = (Color::rgb(139, 207, 163), Color::rgb(240, 140, 140));
        let stat = DiffStat::new(12, 3, palette()).inks(added, removed);
        let frame = tree_of(stat.render()).layout(Size::new(200.0, 80.0));
        let lines = texts(&frame);
        assert_eq!((lines[0].0, lines[0].2, lines[0].3), ("+12", 12.0, added));
        assert_eq!((lines[1].0, lines[1].2, lines[1].3), ("\u{2212}3", 12.0, removed));
        assert_eq!(lines[0].1.y, lines[1].1.y);
        assert!(lines[1].1.x > lines[0].1.x + 5.0);
        // Without host inks it is plain text, never a color of limn's own.
        let plain = tree_of(DiffStat::new(0, 0, palette()).render()).layout(Size::new(200.0, 80.0));
        assert!(texts(&plain).iter().all(|line| line.3 == palette().text));
    }
}
