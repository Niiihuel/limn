//! A folded sidebar: a narrow column of pictures for a collection under the
//! button that unfolds it, each picture named by a hint at its side.

use crate::{
    Access, ButtonVariant, Component, ComponentPalette, Element, Icon, IconButton, Role,
    ScrollbarAppearance, Tooltip, TooltipPlacement, TooltipSpec,
};

/// Side of an item's picture, in logical pixels.
const PICTURE: f32 = 48.0;
/// Space between a picture and the edge of its item's hover fill.
const ITEM_PADDING: f32 = 8.0;
/// Space between the items and the rail's edges.
const MARGIN: f32 = 4.0;
/// Side of the button that unfolds the sidebar.
const EXPAND_SIDE: f32 = 40.0;
/// Side of that button's icon.
const EXPAND_ICON: f32 = 24.0;
/// Side of the mark over the corner of what is playing.
const PLAYING_SIDE: f32 = 20.0;

/// One entry of a [`SideRail`]: a picture that stands for it, and the title
/// and second line its hint shows and the platform reads.
pub struct SideRailItem {
    id: String,
    title: String,
    subtitle: String,
    picture: Option<Element>,
    round: bool,
}

impl SideRailItem {
    /// `id` is reported as `UiAction::Clicked`; `title` names it in its hint
    /// and to the platform.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self { id: id.into(), title: title.into(), subtitle: String::new(), picture: None, round: false }
    }

    /// A muted second line under the title in its hint, such as "Playlist ·
    /// 42 songs", which the platform reads as its description. An empty one
    /// is no line at all.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = subtitle.into();
        self
    }

    /// A caller-owned picture, held in a 48 px square with the `control`
    /// radius. Without one the item draws a muted note on the palette's
    /// `selected`, as for a cover that has not loaded.
    pub fn picture(mut self, picture: Element) -> Self {
        self.picture = Some(picture);
        self
    }

    /// A round picture: a person rather than a collection.
    pub fn round(mut self, round: bool) -> Self {
        self.round = round;
        self
    }
}

/// The library of a sidebar folded into a column of pictures, 72 px wide.
/// The host owns the items, which page is shown, what plays and whether
/// the sidebar is folded: the rail reports a click on an item's ID, on
/// `{id}:expand` to unfold it and on `{id}:create`. Its items scroll when
/// they do not fit its height.
pub struct SideRail {
    id: String,
    items: Vec<SideRailItem>,
    expand: Option<String>,
    create: Option<String>,
    current: Option<String>,
    playing: Option<String>,
    library_icon: Icon,
    create_icon: Icon,
    palette: ComponentPalette,
}

impl SideRail {
    /// The rail's width in logical pixels: a picture, the padding around it
    /// and the margins beside its item.
    pub const WIDTH: f32 = PICTURE + 2.0 * (ITEM_PADDING + MARGIN);

    /// A rail of these items, first to last. `id` prefixes the IDs of its
    /// head buttons and its scroll area, `{id}:items`.
    pub fn new(id: impl Into<String>, items: Vec<SideRailItem>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            items,
            expand: None,
            create: None,
            current: None,
            playing: None,
            library_icon: Icon::Library,
            create_icon: Icon::Add,
            palette,
        }
    }

    /// Head the rail with a 40 px button that unfolds the sidebar, reporting
    /// `{id}:expand`. `tooltip` names it, such as "Open Your Library".
    pub fn expand(mut self, tooltip: impl Into<String>) -> Self {
        self.expand = Some(tooltip.into());
        self
    }

    /// A button with a plus under the head, reporting `{id}:create`.
    /// `tooltip` names it, such as "Create a playlist".
    pub fn create(mut self, tooltip: impl Into<String>) -> Self {
        self.create = Some(tooltip.into());
        self
    }

    /// The glyph of the head button; `Icon::Library` unless set.
    pub fn library_icon(mut self, icon: Icon) -> Self {
        self.library_icon = icon;
        self
    }

    /// The glyph of the create button; `Icon::Add` unless set.
    pub fn create_icon(mut self, icon: Icon) -> Self {
        self.create_icon = icon;
        self
    }

    /// The ID of the item for the page shown now: it keeps the palette's
    /// `selected` fill and is announced as selected.
    pub fn current(mut self, id: impl Into<String>) -> Self {
        self.current = Some(id.into());
        self
    }

    /// The ID of the item that is playing: a loudspeaker over its picture's
    /// corner.
    pub fn playing(mut self, id: impl Into<String>) -> Self {
        self.playing = Some(id.into());
        self
    }

    fn item(&self, item: SideRailItem) -> Element {
        let palette = self.palette;
        let radius = if item.round { PICTURE / 2.0 } else { crate::tokens().radius.control };
        let picture = item
            .picture
            .unwrap_or_else(|| Element::icon(Icon::MusicNote).foreground(palette.muted).background(palette.selected))
            .size(Some(PICTURE), Some(PICTURE))
            .radius(radius);
        let mut layers = vec![picture];
        if self.playing.as_deref() == Some(item.id.as_str()) {
            let mark = Element::icon(Icon::Volume)
                .foreground(palette.text)
                .background(palette.raised)
                .round()
                .size(Some(PLAYING_SIDE), Some(PLAYING_SIDE));
            let corner = Element::row(vec![Element::spacer().flex(1.0), mark]);
            layers.push(Element::column(vec![Element::spacer().flex(1.0), corner]));
        }
        let mut said = Access::new(Role::Button).name(item.title.clone());
        if !item.subtitle.is_empty() {
            said = said.description(item.subtitle.clone());
        }
        let current = self.current.as_deref() == Some(item.id.as_str());
        if self.current.is_some() {
            said = said.selected(current);
        }
        let button = palette.interactive_surface(
            Element::clickable(item.id.clone(), Element::stack(layers).size(Some(PICTURE), Some(PICTURE)))
                .padding(ITEM_PADDING)
                .accessible(said),
        );
        let button = if current { button.background(palette.selected) } else { button };
        // The rail is read by moving along it: each hint opens at once.
        Tooltip::new(button, item.id, item.title, palette)
            .footer(item.subtitle)
            .placement(TooltipPlacement::Right)
            .delay_ms(0)
            .render()
    }
}

impl Component for SideRail {
    fn render(mut self) -> Element {
        let palette = self.palette;
        let mut head = Vec::new();
        if let Some(tooltip) = self.expand.take() {
            let id = format!("{}:expand", self.id);
            let icon = Element::icon(self.library_icon).large_icon(EXPAND_ICON).foreground(palette.muted);
            let button = palette
                .interactive_surface(Element::clickable(id.clone(), icon))
                .hover_foreground(palette.text)
                .size(Some(EXPAND_SIDE), Some(EXPAND_SIDE));
            head.push(Tooltip::new(button, id, tooltip, palette).placement(TooltipPlacement::Right).render());
        }
        if let Some(tooltip) = self.create.take() {
            head.push(
                IconButton::new(format!("{}:create", self.id), self.create_icon, palette)
                    .variant(ButtonVariant::Ghost)
                    .round()
                    .tooltip_with(TooltipSpec::new(tooltip).placement(TooltipPlacement::Right))
                    .render(),
            );
        }
        let items = std::mem::take(&mut self.items).into_iter().map(|item| self.item(item)).collect();
        // A thin bar over the items, which keep their width, without arrows.
        let bar = ScrollbarAppearance { buttons: false, ..ScrollbarAppearance::overlay() };
        let list = Element::scroll(format!("{}:items", self.id), Element::column(items).padding(MARGIN))
            .scrollbar_appearance(bar)
            .flex(1.0);
        let mut column = Vec::with_capacity(2);
        if !head.is_empty() {
            column.push(Element::column(head.into_iter().map(Element::centered_x).collect()).gap(ITEM_PADDING).padding(ITEM_PADDING));
        }
        column.push(list);
        Element::column(column).background(palette.surface).size(Some(Self::WIDTH), None)
    }
}

#[cfg(test)]
#[path = "side_rail_tests.rs"]
mod tests;
