//! Boxed preference lists and action rows. The host supplies actions and data;
//! these components follow the same palette as the rest of the native UI.

use crate::{
    Component, ComponentPalette, CornerRadii, Element, FontFamily, Icon, Separator, Text, ICON_SIZE,
};

fn group_radius() -> f32 {
    crate::tokens().radius.card
}

/// A size or radius from the host: finite and not negative, or not taken.
fn length(value: f32) -> Option<f32> {
    value.is_finite().then(|| value.max(0.0))
}

/// A card of settings rows separated by hairlines, under a small muted
/// title. Without a title it is only the card.
pub struct PreferencesGroup {
    title: String,
    description: Option<String>,
    rows: Vec<Element>,
    palette: ComponentPalette,
    title_size: f32,
    description_size: f32,
    radius: Option<f32>,
}
impl PreferencesGroup {
    /// A group of these rows, in order, each usually an [`ActionRow`].
    pub fn new(title: impl Into<String>, rows: Vec<Element>, palette: ComponentPalette) -> Self {
        Self {
            title: title.into(),
            description: None,
            rows,
            palette,
            title_size: 11.0,
            description_size: 11.0,
            radius: None,
        }
    }
    /// The title's size in logical pixels, 11 unless set. A size that is not
    /// finite is ignored.
    pub fn title_size(mut self, size: f32) -> Self {
        self.title_size = length(size).unwrap_or(self.title_size);
        self
    }
    /// The description's size in logical pixels, 11 unless set. A size that
    /// is not finite is ignored.
    pub fn description_size(mut self, size: f32) -> Self {
        self.description_size = length(size).unwrap_or(self.description_size);
        self
    }
    /// Round the card, and the outer corners of its first and last rows, by
    /// `radius` in place of the `card` token. A radius that is not finite is
    /// ignored.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = length(radius).or(self.radius);
        self
    }
    /// A line of muted text under the title, saying what the group is for.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}
impl Component for PreferencesGroup {
    fn render(self) -> Element {
        // A quiet label names the group; the card below it carries the weight.
        let caption = |text: String, size: f32| {
            Element::text(text)
                .font_family(FontFamily::Sans)
                .font_size(size)
                .foreground(self.palette.muted)
        };
        let radius = self.radius.unwrap_or_else(group_radius);
        let mut heading = Vec::new();
        if !self.title.is_empty() {
            heading.push(caption(self.title, self.title_size));
        }
        if let Some(description) = self.description.filter(|text| !text.is_empty()) {
            heading.push(caption(description, self.description_size));
        }
        let last_row = self.rows.len().saturating_sub(1);
        let mut rows = Vec::new();
        for (index, row) in self.rows.into_iter().enumerate() {
            if !rows.is_empty() {
                rows.push(Separator::horizontal(self.palette).render());
            }
            // The group's child clip is rectangular. Its boundary rows must
            // paint the same outer corners during hover, press and focus.
            rows.push(row.corner_radii(CornerRadii {
                top_left: if index == 0 { radius } else { 0.0 },
                top_right: if index == 0 { radius } else { 0.0 },
                bottom_right: if index == last_row { radius } else { 0.0 },
                bottom_left: if index == last_row { radius } else { 0.0 },
            }));
        }
        let card = Element::panel(Element::column(rows))
            .background(self.palette.surface)
            .border(self.palette.border, 1.0)
            .radius(radius)
            .clip_children();
        // Without a title or a description the group is just its card.
        if heading.is_empty() {
            return card;
        }
        Element::column(vec![Element::column(heading).gap(2.0).padding(4.0), card]).gap(4.0)
    }
}

/// Density of an [`ActionRow`], in logical pixels. The row's padding is not
/// part of it: every row in a card shares one inset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionRowSizing {
    /// Space between the icon, the copy and the trailing element.
    pub gap: f32,
    /// Space between the title and the subtitle.
    pub copy_gap: f32,
    /// Width of the column the icon sits in.
    pub icon_column: f32,
    /// Least height of a row with a title only, padding included.
    pub min_height: f32,
    /// Least height of a row with a subtitle, padding included.
    pub min_height_with_subtitle: f32,
}

impl ActionRowSizing {
    /// The row of a preferences card.
    pub const DEFAULT: Self = Self {
        gap: 10.0,
        copy_gap: 4.0,
        icon_column: 20.0,
        min_height: 36.0,
        min_height_with_subtitle: 54.0,
    };
}

/// A trailing element no wider than this stays beside the copy in a stacked
/// row: a switch reads as part of its line, not as a block under it.
const INLINE_TRAILING_WIDTH: f32 = 64.0;
const STACK_GAP: f32 = 8.0;
const PADDING: f32 = 10.0;

/// One row of a settings card: a title, an optional subtitle and icon, and a
/// trailing element the host owns. With an action the whole row is a button.
pub struct ActionRow {
    title: String,
    subtitle: Option<String>,
    icon: Option<Icon>,
    trailing: Option<Element>,
    action: Option<String>,
    palette: ComponentPalette,
    sizing: ActionRowSizing,
    wrap_title: bool,
    stacked: bool,
    align_top: bool,
    trailing_top: bool,
    icon_muted: bool,
    enabled: bool,
    title_size: f32,
    subtitle_size: f32,
    radius: Option<f32>,
}
impl ActionRow {
    /// A row with a title in the palette's text color.
    pub fn new(title: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            icon: None,
            trailing: None,
            action: None,
            palette,
            sizing: ActionRowSizing::DEFAULT,
            wrap_title: false,
            stacked: false,
            align_top: false,
            trailing_top: false,
            icon_muted: false,
            enabled: true,
            title_size: 12.0,
            subtitle_size: 11.0,
            radius: None,
        }
    }
    /// The title's size in logical pixels, 12 unless set. A size that is not
    /// finite is ignored.
    pub fn title_size(mut self, size: f32) -> Self {
        self.title_size = length(size).unwrap_or(self.title_size);
        self
    }
    /// The subtitle's size in logical pixels, 11 unless set. A size that is
    /// not finite is ignored.
    pub fn subtitle_size(mut self, size: f32) -> Self {
        self.subtitle_size = length(size).unwrap_or(self.subtitle_size);
        self
    }
    /// Round the row's hover and press fill by `radius` in place of the
    /// `control` token. In a [`PreferencesGroup`] the group sets the corners
    /// of its rows instead. A radius that is not finite is ignored.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = length(radius).or(self.radius);
        self
    }
    /// Supporting copy under the title, in the muted color. It wraps.
    pub fn subtitle(mut self, text: impl Into<String>) -> Self {
        self.subtitle = Some(text.into());
        self
    }
    /// An icon before the copy.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
    /// Paint the icon in the muted color, for a note rather than a setting.
    pub fn icon_muted(mut self, muted: bool) -> Self {
        self.icon_muted = muted;
        self
    }
    /// The element at the end of the row, centered on it: a switch, a
    /// button, a chevron. It keeps its own width and flex.
    pub fn trailing(mut self, element: Element) -> Self {
        self.trailing = Some(element);
        self
    }
    /// The former name of [`ActionRow::trailing`].
    pub fn suffix(self, element: Element) -> Self {
        self.trailing(element)
    }
    /// A read-only value at the end of the row, as plain muted text so it is
    /// not mistaken for a control. Replaces any trailing element.
    pub fn value(self, text: impl Into<String>) -> Self {
        let value = Text::new(text, self.palette).muted().render();
        self.trailing(value)
    }
    /// Make the whole row a button reporting `id`.
    pub fn action(mut self, id: impl Into<String>) -> Self {
        self.action = Some(id.into());
        self
    }
    /// A disabled row is passive: a muted title and icon, and no button even
    /// with an action. A trailing control is the host's to disable.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Disable it; the same as `enabled(false)`.
    pub fn disabled(self) -> Self {
        self.enabled(false)
    }
    /// The row's density. [`ActionRowSizing::DEFAULT`] unless set.
    pub fn sizing(mut self, sizing: ActionRowSizing) -> Self {
        self.sizing = sizing;
        self
    }
    /// Wrap a long title over several lines instead of cutting it.
    pub fn wrap_title(mut self, wrap: bool) -> Self {
        self.wrap_title = wrap;
        self
    }
    /// Place the trailing element under the copy, for a narrow pane. A
    /// trailing element with a fixed width of 64 or less stays beside it.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }
    /// Start the icon and the copy at the top of the row instead of
    /// centering them. The trailing element stays centered.
    pub fn align_top(mut self, top: bool) -> Self {
        self.align_top = top;
        self
    }
    /// Keep the trailing element on the row's first line instead of
    /// centering it, for a keycap beside copy that may wrap.
    pub fn trailing_top(mut self, top: bool) -> Self {
        self.trailing_top = top;
        self
    }
}
impl Component for ActionRow {
    fn render(self) -> Element {
        // A profile is plain data from the host; a bad number stops here.
        let px = |value: f32| if value.is_finite() { value.max(0.0) } else { 0.0 };
        let sizing = self.sizing;
        let minimum_height = px(if self.subtitle.is_some() {
            sizing.min_height_with_subtitle
        } else {
            sizing.min_height
        });
        // With an action the row is a button named by its title and
        // described by its subtitle, whatever else sits in it.
        let mut said = crate::Access::new(crate::Role::Button).name(self.title.clone());
        if let Some(subtitle) = &self.subtitle {
            said = said.description(subtitle.clone());
        }
        let title = if self.wrap_title {
            Element::text(self.title)
        } else {
            Element::truncated_text(self.title)
        };
        let mut copy = vec![title
            .font_family(FontFamily::Sans)
            .font_size(self.title_size)
            .foreground(if self.enabled { self.palette.text } else { self.palette.muted })];
        if let Some(subtitle) = self.subtitle {
            copy.push(
                Element::text(subtitle)
                    .font_family(FontFamily::Sans)
                    .font_size(self.subtitle_size)
                    .foreground(self.palette.muted),
            );
        }
        let mut copy = Element::column(copy).gap(px(sizing.copy_gap));
        let mut trailing = self.trailing;
        let stack = self.stacked
            && trailing.as_ref().is_some_and(|element| {
                !element.style.width.is_some_and(|width| width <= INLINE_TRAILING_WIDTH)
            });
        if let Some(element) = trailing.take_if(|_| stack) {
            copy = Element::column(vec![copy, element]).gap(STACK_GAP);
        }
        if !self.align_top {
            copy = copy.centered_y();
        }
        let content = if stack && self.icon.is_none() {
            // Nothing sits beside a stacked row's column, and its two parts
            // set its height.
            copy
        } else {
            let mut children = Vec::new();
            if let Some(icon) = self.icon {
                let column = px(sizing.icon_column);
                let icon = Element::icon(icon).foreground(if self.icon_muted || !self.enabled {
                    self.palette.muted
                } else {
                    self.palette.text
                });
                children.push(if self.align_top {
                    icon.size(Some(column), Some(ICON_SIZE))
                } else {
                    icon.centered_y().size(Some(column), None)
                });
            }
            children.push(copy.flex(1.0));
            if let Some(element) = trailing {
                children.push(if self.trailing_top { element } else { element.centered_y() });
            }
            Element::row(children).gap(px(sizing.gap)).min_size(0.0, minimum_height)
        }
        .padding(PADDING);
        let row = match self.action {
            Some(id) if self.enabled => self
                .palette
                .interactive_surface(Element::clickable(id, content).accessible(said)),
            Some(id) => content.accessible(said.id(id).disabled(true)),
            None => content,
        };
        match self.radius {
            Some(radius) => row.radius(radius),
            None => row,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, Frame, Key, Point, Rect, Size, UiAction, UiEvent, UiTree};

    #[test]
    fn a_row_with_an_action_is_a_button_named_by_its_title_and_described_by_its_subtitle() {
        let palette = ComponentPalette::neutral_dark();
        let row = || ActionRow::new("Idioma", palette).subtitle("Español").value("es");
        for enabled in [true, false] {
            let frame = UiTree::new(row().action("language").enabled(enabled).render()).layout(Size::new(320.0, 80.0));
            let said = frame.access.iter().find(|node| node.id == "language").unwrap();
            assert_eq!((said.role, said.name.as_str()), (crate::Role::Button, "Idioma"));
            assert_eq!(said.description.as_deref(), Some("Español"));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
        }
        // Without an action it is copy, not a control.
        assert!(UiTree::new(row().render()).layout(Size::new(320.0, 80.0)).access.is_empty());
    }

    #[test]
    fn rows_and_groups_take_their_own_text_sizes_and_corners() {
        let palette = ComponentPalette::neutral_dark();
        let sizes = |frame: &Frame| -> Vec<(String, f32)> {
            frame.commands.iter().filter_map(|command| match command {
                DrawCommand::Text { text, font_size, .. } => Some((text.clone(), *font_size)),
                _ => None,
            }).collect()
        };
        let row = || ActionRow::new("Idioma", palette).subtitle("Español");
        let plain = UiTree::new(row().render()).layout(Size::new(320.0, 80.0));
        assert_eq!(sizes(&plain), [("Idioma".to_owned(), 12.0), ("Español".to_owned(), 11.0)]);
        let large = UiTree::new(row().title_size(17.0).subtitle_size(f32::NAN).subtitle_size(15.0).render())
            .layout(Size::new(320.0, 120.0));
        assert_eq!(sizes(&large), [("Idioma".to_owned(), 17.0), ("Español".to_owned(), 15.0)]);

        let group = |group: PreferencesGroup| UiTree::new(group.render()).layout(Size::new(320.0, 200.0));
        let rows = || vec![row().action("language").render()];
        let card = |frame: &Frame| frame.commands.iter().find_map(|command| match command {
            DrawCommand::StrokeRect { color, radius, .. } if *color == palette.border => Some(*radius),
            _ => None,
        });
        let default = group(PreferencesGroup::new("General", rows(), palette).description("Lo básico"));
        assert_eq!(card(&default), Some(group_radius()));
        assert!(sizes(&default).contains(&("General".to_owned(), 11.0)));
        let phone = group(PreferencesGroup::new("General", rows(), palette).description("Lo básico")
            .title_size(15.0).description_size(13.0).radius(f32::INFINITY).radius(14.0));
        assert_eq!(card(&phone), Some(14.0));
        assert!(sizes(&phone).contains(&("General".to_owned(), 15.0)));
        assert!(sizes(&phone).contains(&("Lo básico".to_owned(), 13.0)));
        // A lone row with an action rounds its hover fill as asked.
        assert_eq!(row().action("language").radius(12.0).render().style.radius, 12.0);
    }

    fn same_bounds(rect: Rect, bounds: Rect) -> bool {
        (rect.x - bounds.x).abs() < 0.001
            && (rect.y - bounds.y).abs() < 0.001
            && (rect.width - bounds.width).abs() < 0.001
            && (rect.height - bounds.height).abs() < 0.001
    }

    fn painted_corners(frame: &Frame, bounds: Rect, color: crate::Color) -> Option<CornerRadii> {
        frame.commands.iter().find_map(|command| match command {
            DrawCommand::FillCornerRect {
                rect,
                color: ink,
                radii,
            } if same_bounds(*rect, bounds) && *ink == color => Some(*radii),
            DrawCommand::FillRect {
                rect,
                color: ink,
                radius,
            } if same_bounds(*rect, bounds) && *ink == color => Some(CornerRadii::same(*radius)),
            _ => None,
        })
    }

    const WIDTHS: [f32; 3] = [220.0, 360.0, 900.0];
    const LONG: &str = "Permitir que el agente modifique archivos y ejecute comandos";

    fn frame_of(row: Element, width: f32) -> Frame {
        let mut tree = UiTree::new(Element::column(vec![row, Element::spacer().flex(1.0)]));
        tree.set_reduced_motion(true);
        tree.layout(Size::new(width, 600.0))
    }

    fn runs(frame: &Frame, source: &str) -> Vec<Rect> {
        frame
            .text_layouts()
            .iter()
            .find(|block| block.source == source)
            .unwrap()
            .runs
            .iter()
            .map(|run| run.rect)
            .collect()
    }

    fn region(frame: &Frame, id: &str) -> Rect {
        frame.regions.iter().find(|region| region.id == id).unwrap().rect
    }

    fn switch(palette: ComponentPalette) -> Element {
        crate::Switch::new("toggle", false, palette).render()
    }

    #[test]
    fn default_sizing_is_the_row_without_a_profile_and_a_profile_sets_its_density() {
        let palette = ComponentPalette::neutral_dark();
        let row = || ActionRow::new("Tema", palette).icon(Icon::Grid).trailing(switch(palette));
        assert_eq!(
            format!("{:?}", row().render()),
            format!("{:?}", row().sizing(ActionRowSizing::DEFAULT).render())
        );
        assert_eq!(row().render().style.min_height, 36.0);
        assert_eq!(row().subtitle("Oscuro").render().style.min_height, 54.0);

        let dense = ActionRowSizing {
            gap: 16.0,
            copy_gap: 2.0,
            icon_column: 16.0,
            min_height: 30.0,
            min_height_with_subtitle: 34.0,
        };
        for width in WIDTHS {
            let frame = frame_of(row().subtitle("Oscuro").wrap_title(true).sizing(dense).render(), width);
            let icon = frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Icon { rect, .. } => Some(*rect),
                    _ => None,
                })
                .unwrap();
            let title = runs(&frame, "Tema")[0];
            let subtitle = runs(&frame, "Oscuro")[0];
            assert_eq!((icon.x, icon.width), (10.0, 16.0));
            assert_eq!(title.x, 10.0 + 16.0 + 16.0);
            assert!((subtitle.y - title.y - title.height - 2.0).abs() < 0.01);
            assert_eq!(region(&frame, "toggle").x + 36.0, width - 10.0);
        }
    }

    #[test]
    fn a_bad_sizing_profile_never_reaches_layout() {
        let palette = ComponentPalette::neutral_dark();
        let bad = ActionRowSizing {
            gap: f32::NAN,
            copy_gap: -4.0,
            icon_column: f32::INFINITY,
            min_height: f32::NEG_INFINITY,
            min_height_with_subtitle: f32::NAN,
        };
        let row = ActionRow::new("Tema", palette)
            .subtitle("Oscuro")
            .icon(Icon::Grid)
            .trailing(switch(palette))
            .sizing(bad)
            .render();
        assert_eq!(row.style.min_height, 0.0);
        assert_eq!(row.style.gap, 0.0);
        let frame = frame_of(row, 360.0);
        let toggle = region(&frame, "toggle");
        assert!(toggle.x.is_finite() && toggle.y.is_finite());
        assert_eq!(toggle.x + toggle.width, 350.0);
    }

    #[test]
    fn wrap_title_wraps_inside_the_row_and_the_default_keeps_one_line() {
        let palette = ComponentPalette::neutral_dark();
        for width in WIDTHS {
            // A truncated title is one painted line, cut when it does not fit.
            let cut = frame_of(ActionRow::new(LONG, palette).render(), width);
            let painted: Vec<_> = cut
                .commands
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Text { text, .. } => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(painted.len(), 1);
            let cut = painted[0] != LONG;
            assert_eq!(cut, width != 900.0, "width {width}");
            let wrapped = frame_of(ActionRow::new(LONG, palette).wrap_title(true).render(), width);
            let lines = runs(&wrapped, LONG);
            assert_eq!(lines.len() > 1, cut, "width {width}");
            for line in lines {
                assert!(line.x >= 10.0 && line.x + line.width <= width - 10.0 + 0.01);
            }
        }
    }

    #[test]
    fn stacked_puts_a_wide_trailing_under_the_copy_and_leaves_a_switch_inline() {
        let palette = ComponentPalette::neutral_dark();
        let button = || Element::button("open", "Abrir").size(Some(120.0), Some(28.0));
        let row = |trailing: Element, stacked: bool| {
            ActionRow::new("Tema", palette).subtitle("Tema activo").trailing(trailing).stacked(stacked).render()
        };
        for width in WIDTHS {
            let frame = frame_of(row(button(), true), width);
            let subtitle = runs(&frame, "Tema activo")[0];
            let open = region(&frame, "open");
            assert_eq!(open.x, 10.0);
            assert!((open.y - subtitle.y - subtitle.height - STACK_GAP).abs() < 0.01);

            // Beside the copy the trailing element keeps its width at the right edge.
            let inline = region(&frame_of(row(button(), false), width), "open");
            assert_eq!((inline.x + inline.width, inline.width), (width - 10.0, 120.0));

            let stacked = frame_of(row(switch(palette), true), width);
            assert_eq!(stacked, frame_of(row(switch(palette), false), width));
            let toggle = region(&stacked, "toggle");
            assert_eq!((toggle.x + toggle.width, toggle.width, toggle.height), (width - 10.0, 36.0, 20.0));
        }
        assert!(matches!(row(button(), true).kind, crate::ElementKind::Column(_)));
        assert!(matches!(row(button(), false).kind, crate::ElementKind::Row(_)));
        // A flexible trailing element hands its flex to the wrapper that centers it.
        let crate::ElementKind::Row(children) = row(button().flex(2.0), false).kind else {
            panic!("an inline row");
        };
        assert_eq!((children[1].style.flex, children[1].style.width), (2.0, Some(120.0)));
    }

    #[test]
    fn align_top_starts_copy_and_icon_at_the_padding_and_centers_the_trailing() {
        let palette = ComponentPalette::neutral_dark();
        let tall = || Element::button("open", "Abrir").size(Some(120.0), Some(80.0));
        let row = |top: bool| {
            ActionRow::new("Tema", palette)
                .icon(Icon::Grid)
                .trailing(tall())
                .wrap_title(true)
                .align_top(top)
                .render()
        };
        let icon_of = |frame: &Frame| {
            frame
                .commands
                .iter()
                .find_map(|command| match command {
                    DrawCommand::Icon { rect, .. } => Some(*rect),
                    _ => None,
                })
                .unwrap()
        };
        for width in WIDTHS {
            let top = frame_of(row(true), width);
            let title = runs(&top, "Tema")[0];
            let icon = icon_of(&top);
            // The glyph keeps its size, centered across the 20 px icon column.
            assert_eq!((title.y, icon.y, icon.x, icon.width), (10.0, 10.0, 12.0, ICON_SIZE));
            assert_eq!(region(&top, "open").y, 10.0);

            let centered = frame_of(row(false), width);
            let title = runs(&centered, "Tema")[0];
            let icon = icon_of(&centered);
            assert!((title.y + title.height / 2.0 - 50.0).abs() < 0.01);
            assert!((icon.y + icon.height / 2.0 - 50.0).abs() < 0.01);
            assert_eq!((icon.x, icon.width), (12.0, ICON_SIZE));
        }
    }

    #[test]
    fn trailing_top_keeps_the_trailing_on_the_first_line_of_wrapped_copy() {
        let palette = ComponentPalette::neutral_dark();
        let row = |top: bool| {
            ActionRow::new(LONG, palette)
                .wrap_title(true)
                .trailing(Element::button("key", "K").size(Some(40.0), Some(18.0)))
                .trailing_top(top)
                .render()
        };
        // Narrow enough for the title to wrap, so the row is taller than the key.
        let top = frame_of(row(true), 220.0);
        assert!(runs(&top, LONG).len() > 1);
        assert_eq!(region(&top, "key").y, 10.0);
        let centered = frame_of(row(false), 220.0);
        assert!(region(&centered, "key").y > 10.0);
        // On one line there is nothing to choose between.
        assert_eq!(region(&frame_of(row(true), 900.0), "key").x, 900.0 - 10.0 - 40.0);
    }

    #[test]
    fn icon_muted_value_and_trailing_paint_in_the_palette() {
        let palette = ComponentPalette::neutral_dark();
        let tint = |muted: bool| {
            let row = ActionRow::new("Nota", palette).icon(Icon::InfoCircle).icon_muted(muted).render();
            frame_of(row, 360.0).commands.iter().find_map(|command| match command {
                DrawCommand::Icon { tint, .. } => Some(*tint),
                _ => None,
            })
        };
        assert_eq!(tint(false), Some(palette.text));
        assert_eq!(tint(true), Some(palette.muted));

        for width in WIDTHS {
            let frame = frame_of(ActionRow::new("Contexto", palette).wrap_title(true).value("200 tokens").render(), width);
            assert!(frame.regions.is_empty());
            assert!(frame.commands.iter().any(|command| matches!(
                command,
                DrawCommand::Text { text, color, font_size, font_family: FontFamily::Sans, .. }
                    if text == "200 tokens" && *color == palette.muted && *font_size == 12.0
            )));
            let value = runs(&frame, "200 tokens")[0];
            let title = runs(&frame, "Contexto")[0];
            assert!((value.x + value.width - (width - 10.0)).abs() < 0.01);
            assert!((value.y + value.height / 2.0 - title.y - title.height / 2.0).abs() < 0.01);
        }
        let chevron = || Element::icon(Icon::ChevronRight);
        assert_eq!(
            format!("{:?}", ActionRow::new("Cuenta", palette).trailing(chevron()).render()),
            format!("{:?}", ActionRow::new("Cuenta", palette).suffix(chevron()).render())
        );
    }

    #[test]
    fn grouped_action_rows_keep_outer_corners_and_full_hit_targets_in_every_state() {
        let palette = ComponentPalette::neutral_dark();
        for width in [220.0, 360.0, 900.0] {
            for count in [1, 2, 3] {
                let mut tree = UiTree::new(
                    PreferencesGroup::new(
                        "Cuenta",
                        (0..count)
                            .map(|index| {
                                ActionRow::new(format!("Acción {index}"), palette)
                                    .action(format!("action:{index}"))
                                    .render()
                            })
                            .collect(),
                        palette,
                    )
                    .render(),
                );
                tree.set_reduced_motion(true);
                let viewport = Size::new(width, 500.0);
                let initial = tree.layout(viewport);
                for index in 0..count {
                    let id = format!("action:{index}");
                    let bounds = initial
                        .regions
                        .iter()
                        .find(|region| region.id == id)
                        .unwrap()
                        .rect;
                    let expected = CornerRadii {
                        top_left: if index == 0 { group_radius() } else { 0.0 },
                        top_right: if index == 0 { group_radius() } else { 0.0 },
                        bottom_right: if index == count - 1 {
                            group_radius()
                        } else {
                            0.0
                        },
                        bottom_left: if index == count - 1 {
                            group_radius()
                        } else {
                            0.0
                        },
                    };
                    assert_eq!(bounds.width, width);
                    let point = Point::new(bounds.x + 1.0, bounds.y + 1.0);
                    assert_eq!(initial.hit_test(point), Some(id.as_str()));
                    tree.handle_event(UiEvent::PointerMove(point));
                    let hovered = tree.layout(viewport);
                    assert_eq!(
                        painted_corners(&hovered, bounds, palette.hover),
                        Some(expected)
                    );
                    tree.handle_event(UiEvent::PointerDown(point));
                    let pressed = tree.layout(viewport);
                    assert_eq!(
                        painted_corners(&pressed, bounds, palette.selected),
                        Some(expected)
                    );
                    assert_eq!(
                        tree.handle_event(UiEvent::PointerUp(point)),
                        vec![UiAction::Clicked { id: id.clone() }]
                    );
                    assert!(tree.focus(&id));
                    let focused = tree.layout(viewport);
                    // Hit rectangles are intersected with the group clip;
                    // fractional heading metrics can round their height by
                    // a few ulps relative to the original painted rectangle.
                    assert!(focused.commands.iter().any(|command| match command {
                        DrawCommand::StrokeCornerRect {
                            rect, width, radii, ..
                        } => same_bounds(*rect, bounds) && *width == 2.0 && *radii == expected,
                        DrawCommand::StrokeRect {
                            rect,
                            width,
                            radius,
                            ..
                        } =>
                            same_bounds(*rect, bounds)
                                && *width == 2.0
                                && CornerRadii::same(*radius) == expected,
                        _ => false,
                    }));
                }
            }
        }
    }

    #[test]
    fn account_group_rounds_manage_hover_without_changing_disconnect_suffix_action() {
        let palette = ComponentPalette::neutral_dark();
        for width in [220.0, 360.0, 900.0] {
            let mut tree = UiTree::new(
                PreferencesGroup::new(
                    "Cuenta",
                    vec![
                        ActionRow::new("Administrar cuenta", palette)
                            .subtitle("Abre la página del proveedor en tu navegador.")
                            .action("provider_account:codex")
                            .suffix(Element::icon(Icon::ArrowRight))
                            .render(),
                        ActionRow::new("Desconectar", palette)
                            .subtitle("Quita esta conexión de la aplicación.")
                            .suffix(
                                Element::button("provider_disconnect:codex", "Desconectar")
                                    .size(Some(100.0), Some(32.0)),
                            )
                            .render(),
                    ],
                    palette,
                )
                .render(),
            );
            tree.set_reduced_motion(true);
            let viewport = Size::new(width, 500.0);
            let initial = tree.layout(viewport);
            let account = initial
                .regions
                .iter()
                .find(|region| region.id == "provider_account:codex")
                .unwrap()
                .rect;
            tree.handle_event(UiEvent::PointerMove(Point::new(
                account.x + 1.0,
                account.y + 1.0,
            )));
            let hovered = tree.layout(viewport);
            assert_eq!(
                painted_corners(&hovered, account, palette.hover),
                Some(CornerRadii::top(group_radius()))
            );
            let disconnect = hovered
                .regions
                .iter()
                .find(|region| region.id == "provider_disconnect:codex")
                .unwrap()
                .rect;
            assert!(disconnect.x >= 0.0 && disconnect.x + disconnect.width <= width);
            let point = Point::new(
                disconnect.x + disconnect.width / 2.0,
                disconnect.y + disconnect.height / 2.0,
            );
            tree.handle_event(UiEvent::PointerDown(point));
            assert_eq!(
                tree.handle_event(UiEvent::PointerUp(point)),
                vec![UiAction::Clicked {
                    id: "provider_disconnect:codex".into()
                }]
            );
        }
    }

    #[test]
    fn a_disabled_row_is_muted_takes_no_pointer_or_focus_and_keeps_its_place() {
        let palette = ComponentPalette::neutral_dark();
        let row = |enabled: bool| {
            ActionRow::new("Atajos", palette)
                .subtitle("Abre la lista de atajos.")
                .icon(Icon::Grid)
                .trailing(switch(palette))
                .wrap_title(true)
                .action("shortcuts")
                .enabled(enabled)
                .render()
        };
        // The icon, the copy and the trailing control, wherever they are painted.
        let places = |frame: &Frame| {
            let icon = frame.commands.iter().find_map(|command| match command {
                DrawCommand::Icon { rect, .. } => Some(*rect),
                _ => None,
            });
            (icon, runs(frame, "Atajos"), runs(frame, "Abre la lista de atajos."), region(frame, "toggle"))
        };
        for width in WIDTHS {
            let enabled = frame_of(row(true), width);
            let bounds = region(&enabled, "shortcuts");
            let mut tree = UiTree::new(Element::column(vec![row(false), Element::spacer().flex(1.0)]));
            tree.set_reduced_motion(true);
            let frame = tree.layout(Size::new(width, 600.0));
            assert!(!frame.regions.iter().any(|region| region.id == "shortcuts"));
            assert!(!tree.focus("shortcuts"));
            let point = Point::new(bounds.x + 1.0, bounds.y + 1.0);
            assert!(tree.handle_event(UiEvent::PointerDown(point)).is_empty());
            assert!(tree.handle_event(UiEvent::PointerUp(point)).is_empty());
            assert!(tree.handle_event(UiEvent::Key { key: Key::Enter, shift: false, ctrl: false }).is_empty());
            assert!(frame.commands.iter().all(|command| match command {
                DrawCommand::Text { color, .. } => *color == palette.muted,
                DrawCommand::Icon { tint, .. } => *tint == palette.muted,
                _ => true,
            }));
            assert_eq!(places(&frame), places(&enabled));
            // The trailing control is the host's, and still answers.
            assert!(tree.focus("toggle"));
        }
    }
}
