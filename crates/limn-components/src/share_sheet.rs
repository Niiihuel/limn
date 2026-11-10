//! A share sheet as iOS draws one: what is shared at its top beside a way out, the apps
//! it can go to in a row to swipe, and what can be done with it in a list of its own.

use crate::{Access, Color, Component, ComponentPalette, Element, FontFamily, GlassMaterial, Icon, Modal, Role, Text};

/// An app or a place the item can go to: the host's mark for it and its name.
pub struct ShareTarget {
    id: String,
    label: String,
    mark: Element,
}

impl ShareTarget {
    /// A target reporting `id` when pressed. `mark` is drawn in a rounded square, as an
    /// app's icon is; the host draws it, a glyph on the app's colour or its picture.
    pub fn new(id: impl Into<String>, label: impl Into<String>, mark: Element) -> Self {
        Self { id: id.into(), label: label.into(), mark }
    }
}

/// Something done with the item, a row of the sheet's list: its name and, at its end, its glyph.
pub struct ShareAction {
    id: String,
    label: String,
    icon: Icon,
}

impl ShareAction {
    /// An action reporting `id` when pressed.
    pub fn new(id: impl Into<String>, label: impl Into<String>, icon: Icon) -> Self {
        Self { id: id.into(), label: label.into(), icon }
    }
}

/// A controlled share sheet from the foot of the window. The host keeps `open` and handles
/// each target's and action's ID; dismissal, by the cross, the backdrop or Escape, reports
/// `dismiss_id`. Its surface can be glass and pop out of its trigger as any popover can, by
/// [`Element::popover_morph`] on what it renders.
pub struct ShareSheet {
    base: Element,
    open: bool,
    title: String,
    subtitle: String,
    dismiss_id: String,
    palette: ComponentPalette,
    preview: Option<Element>,
    targets: Vec<ShareTarget>,
    actions: Vec<ShareAction>,
    material: Option<GlassMaterial>,
    ios: Option<crate::SheetSpec>,
}

/// iOS's measures: a 52 preview with corners of 10, a 30 cross, app squares 60 with corners
/// of 14 under 11-point names in cells 76 wide, list rows 50 tall in a group with corners of 14.
const PREVIEW: f32 = 52.0;
const CROSS: f32 = 30.0;
const TOUCH: f32 = 44.0;
const MARK: f32 = 60.0;
const CELL: f32 = 76.0;
const ROW: f32 = 50.0;
const GROUP_RADIUS: f32 = 14.0;

impl ShareSheet {
    /// A sheet sharing `title`, `subtitle` under it (iOS shows the link's site there).
    pub fn new(
        base: Element,
        open: bool,
        title: impl Into<String>,
        subtitle: impl Into<String>,
        dismiss_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            base,
            open,
            title: title.into(),
            subtitle: subtitle.into(),
            dismiss_id: dismiss_id.into(),
            palette,
            preview: None,
            targets: Vec::new(),
            actions: Vec::new(),
            material: None,
            ios: None,
        }
    }

    /// A picture of what is shared, beside its name.
    pub fn preview(mut self, picture: Element) -> Self {
        self.preview = Some(picture);
        self
    }

    /// The apps and places it can go to, in their order.
    pub fn targets(mut self, targets: Vec<ShareTarget>) -> Self {
        self.targets = targets;
        self
    }

    /// What can be done with it, in their order.
    pub fn actions(mut self, actions: Vec<ShareAction>) -> Self {
        self.actions = actions;
        self
    }

    /// Move it as iOS 26's sheet; see [`crate::BottomSheet::ios`].
    pub fn ios(mut self, spec: crate::SheetSpec) -> Self {
        self.ios = Some(spec);
        self
    }

    /// Draw its surface in `material` over the page beneath it.
    pub fn glass(mut self, material: GlassMaterial) -> Self {
        self.material = Some(material);
        self
    }
}

impl Component for ShareSheet {
    fn render(self) -> Element {
        let p = self.palette;
        let ink = |share: u8| Color { a: share, ..p.text };
        let mut head = Vec::new();
        if let Some(picture) = self.preview {
            head.push(picture.size(Some(PREVIEW), Some(PREVIEW)).radius(10.0).clip_children().centered_y());
        }
        head.push(Element::column(vec![
            Text::new(&self.title, p).font_size(15.0).truncate().render().font_family(FontFamily::SansBold),
            Text::new(&self.subtitle, p).font_size(13.0).muted().truncate().render(),
        ]).gap(2.0).centered_y().flex(1.0));
        // A 30 circle, as iOS draws it, in a 44 a finger can find.
        let circle = Element::panel(Element::icon(Icon::Close).foreground(p.text).size(Some(14.0), Some(14.0)).centered())
            .background(ink(30)).round().size(Some(CROSS), Some(CROSS)).centered();
        let cross = Element::clickable(&self.dismiss_id, circle)
            .accessible(Access::new(Role::Button).name(crate::labels().close)).size(Some(TOUCH), Some(TOUCH));
        head.push(cross.centered_y());
        let mut body = vec![Element::row(head).gap(12.0)];
        body.push(Element::panel(Element::spacer()).background(ink(28)).size(None, Some(0.5)));
        if !self.targets.is_empty() {
            let cells = self.targets.into_iter().map(|target| {
                let mark = target.mark.size(Some(MARK), Some(MARK)).radius(14.0).clip_children().centered_x();
                let name = Text::new(&target.label, p).font_size(11.0).truncate().render().center_content();
                Element::clickable(&target.id, Element::column(vec![mark, name]).gap(6.0))
                    .accessible(Access::new(Role::Button).name(&target.label)).size(Some(CELL), None)
            }).collect();
            // As iOS's row: swiped, with no bar over the names.
            body.push(Element::horizontal_scroll_2d(format!("{}:targets", self.dismiss_id), Element::row(cells).gap(4.0)).drag_to_pan()
                .scrollbar_visibility(crate::ScrollbarVisibility::AutoHide { hold_ms: 0, fade_ms: 0 }));
        }
        if !self.actions.is_empty() {
            let mut rows = Vec::new();
            for (index, action) in self.actions.into_iter().enumerate() {
                if index > 0 {
                    rows.push(Element::row(vec![Element::spacer().size(Some(16.0), None),
                        Element::panel(Element::spacer()).background(ink(28)).size(None, Some(0.5)).flex(1.0)]));
                }
                let line = Element::row(vec![
                    Element::spacer().size(Some(16.0), None),
                    Text::new(&action.label, p).font_size(16.0).truncate().render().flex(1.0).centered_y(),
                    Element::icon(action.icon).foreground(p.text).size(Some(20.0), Some(20.0)).centered_y(),
                    Element::spacer().size(Some(16.0), None),
                ]);
                rows.push(Element::clickable(&action.id, line).accessible(Access::new(Role::Button).name(&action.label))
                    .size(None, Some(ROW)));
            }
            body.push(Element::panel(Element::column(rows)).background(ink(20)).radius(GROUP_RADIUS).clip_children());
        }
        let mut sheet = Modal::new(self.base, self.open, self.title, "", self.dismiss_id, p)
            .headless(true)
            .body(Element::column(body).gap(16.0).padding(16.0));
        if let Some(material) = self.material {
            sheet = sheet.glass(material);
        }
        if let Some(spec) = self.ios {
            sheet = sheet.ios_sheet(spec);
        }
        sheet.bottom_sheet(600.0, 0.0).render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Size, UiAction, UiEvent, UiTree};

    fn sheet(open: bool) -> Element {
        let palette = ComponentPalette::neutral_dark();
        ShareSheet::new(Element::spacer(), open, "Sparks", "music.youtube.com", "share:close", palette)
            .preview(Element::panel(Element::spacer()).background(Color::rgb(200, 120, 40)))
            .targets(vec![ShareTarget::new("share:mail", "Mail", Element::panel(Element::spacer()).background(Color::rgb(30, 136, 245)))])
            .actions(vec![ShareAction::new("share:copy", "Copy link", Icon::Copy)])
            .render()
    }

    #[test]
    fn it_shows_what_is_shared_the_apps_and_the_actions_and_leaves_by_its_cross() {
        let mut tree = UiTree::new(sheet(true));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(390.0, 844.0));
        for id in ["share:close", "share:mail", "share:copy"] {
            assert!(frame.regions.iter().any(|region| region.id == id), "{id}");
        }
        // The backdrop reports the same dismissal; the cross is the small one.
        let cross = frame.regions.iter().find(|region| region.id == "share:close" && region.rect.width < 100.0).unwrap().rect;
        assert!(cross.y > 400.0, "a sheet from the foot of the window: {cross:?}");
        let at = crate::Point::new(cross.x + cross.width / 2.0, cross.y + cross.height / 2.0);
        tree.handle_event(UiEvent::PointerMove(at));
        tree.handle_event(UiEvent::PointerDown(at));
        assert_eq!(tree.handle_event(UiEvent::PointerUp(at)), vec![UiAction::Clicked { id: "share:close".into() }]);
        let mut closed = UiTree::new(sheet(false));
        assert!(!closed.layout(Size::new(390.0, 844.0)).regions.iter().any(|region| region.id == "share:copy"));
    }
}
