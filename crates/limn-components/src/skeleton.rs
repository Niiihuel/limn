//! Passive loading placeholders with one stable, paint-only pulse.

use crate::{Component, ComponentPalette, Element};

/// A block for content whose shape is known while its data is loading.
/// Reuse `id` across tree rebuilds to retain the pulse phase. Without a width,
/// it fills the available horizontal space.
pub struct Skeleton {
    id: String,
    palette: ComponentPalette,
    width: Option<f32>,
    height: f32,
    radius: f32,
}

impl Skeleton {
    /// A block 12 tall with a radius of 6 that fills the available width.
    pub fn new(id: impl Into<String>, palette: ComponentPalette) -> Self {
        Self {
            id: id.into(),
            palette,
            width: None,
            height: 12.0,
            radius: 6.0,
        }
    }

    /// A rounded placeholder for one line of text. Pair lines of different
    /// widths to approximate the copy that will replace them.
    pub fn line(id: impl Into<String>, palette: ComponentPalette, width: f32) -> Self {
        Self::new(id, palette).width(width).height(12.0).radius(6.0)
    }

    /// A large content preview, such as a card, image, or result snippet.
    pub fn block(
        id: impl Into<String>,
        palette: ComponentPalette,
        width: f32,
        height: f32,
    ) -> Self {
        Self::new(id, palette)
            .width(width)
            .height(height)
            .radius(12.0)
    }

    /// A circular avatar or icon placeholder beside related text lines.
    pub fn circle(id: impl Into<String>, palette: ComponentPalette, diameter: f32) -> Self {
        let diameter = if diameter.is_finite() {
            diameter.max(1.0)
        } else {
            32.0
        };
        Self::new(id, palette)
            .width(diameter)
            .height(diameter)
            .radius(diameter / 2.0)
    }

    /// Fix the width, in logical pixels. A width that is not finite is ignored and a
    /// negative one is zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Set the height in logical pixels, never below 1 (12 by default). A value that is not
    /// finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(1.0);
        }
        self
    }

    /// Set the corner radius (6 by default). A radius that is not finite is ignored and a
    /// negative one is zero.
    pub fn radius(mut self, radius: f32) -> Self {
        if radius.is_finite() {
            self.radius = radius.max(0.0);
        }
        self
    }
}

impl Component for Skeleton {
    fn render(self) -> Element {
        // A toolbar's hover may equal the raised surface. Loading still needs
        // two distinct colors; derive the pulse from text contrast in that case
        // so it works in both light and dark themes without changing controls.
        let base = self.palette.raised;
        let highlight = if self.palette.hover == base {
            let toward =
                |channel: u8, text: u8| ((u16::from(channel) * 7 + u16::from(text)) / 8) as u8;
            crate::Color::rgba(
                toward(base.r, self.palette.text.r),
                toward(base.g, self.palette.text.g),
                toward(base.b, self.palette.text.b),
                base.a,
            )
        } else {
            self.palette.hover
        };
        Element::skeleton(self.id, base, highlight)
            .size(self.width, Some(self.height))
            .radius(self.radius)
    }
}

/// Placeholder rows for a list whose items are still loading: an optional
/// icon square and a line per row. The lines vary in length so they read as
/// text. Reuse `id` across tree rebuilds to keep the pulse in phase.
pub struct SkeletonGroup {
    id: String,
    palette: ComponentPalette,
    count: usize,
    row_height: f32,
    icon: bool,
    fractions: Vec<f32>,
}

impl SkeletonGroup {
    const LINE_HEIGHT: f32 = 10.0;

    /// `count` rows, each a line filling a share of the width.
    pub fn rows(id: impl Into<String>, palette: ComponentPalette, count: usize) -> Self {
        Self {
            id: id.into(),
            palette,
            count,
            row_height: 28.0,
            icon: false,
            fractions: vec![0.46, 0.62, 0.38, 0.54],
        }
    }

    /// The height of each row; match the rows that will replace these.
    pub fn row_height(mut self, row_height: f32) -> Self {
        if row_height.is_finite() {
            self.row_height = row_height.max(Self::LINE_HEIGHT);
        }
        self
    }

    /// Leads each row with an icon-sized square.
    pub fn icon(mut self, icon: bool) -> Self {
        self.icon = icon;
        self
    }

    /// The share of the row each line fills, from 0 to 1, repeated in order.
    /// Values that are not finite are dropped; an empty list is ignored.
    pub fn fractions(mut self, fractions: &[f32]) -> Self {
        let fractions: Vec<f32> = fractions
            .iter()
            .filter(|fraction| fraction.is_finite())
            .map(|fraction| fraction.clamp(0.0, 1.0))
            .collect();
        if !fractions.is_empty() {
            self.fractions = fractions;
        }
        self
    }

    /// The rows on their own, for a host that places each one itself, such
    /// as in a virtualized list.
    pub fn into_rows(self) -> Vec<Element> {
        (0..self.count)
            .map(|row| {
                let fraction = self.fractions[row % self.fractions.len()];
                let mut cells = Vec::new();
                if self.icon {
                    cells.push(
                        Skeleton::new(format!("{}:{row}:icon", self.id), self.palette)
                            .width(crate::ICON_SIZE)
                            .height(crate::ICON_SIZE)
                            .radius(crate::tokens().radius.control)
                            .render(),
                    );
                }
                cells.push(
                    Skeleton::new(format!("{}:{row}", self.id), self.palette)
                        .height(Self::LINE_HEIGHT)
                        .radius(Self::LINE_HEIGHT / 2.0)
                        .render()
                        .flex(fraction),
                );
                cells.push(Element::spacer().flex(1.0 - fraction));
                Element::row(cells)
                    .gap(10.0)
                    .centered_y()
                    .size(None, Some(self.row_height))
            })
            .collect()
    }
}

impl Component for SkeletonGroup {
    fn render(self) -> Element {
        Element::column(self.into_rows())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DrawCommand, ElementKind, Size, UiTree};

    #[test]
    fn skeleton_is_a_passive_surface_that_fills_available_width() {
        let palette = ComponentPalette::neutral_dark();
        let element = Skeleton::new("loading-title", palette)
            .height(14.0)
            .render();
        assert!(matches!(element.kind, ElementKind::Skeleton { .. }));
        let mut tree = UiTree::new(Element::column(vec![element]));
        let frame = tree.layout(Size::new(320.0, 20.0));
        assert!(frame.regions.is_empty());
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { rect, radius, .. }
                if rect.width == 320.0 && rect.height == 14.0 && *radius == 6.0
        )));
        assert!(tree.is_animating());
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(320.0, 20.0));
        assert!(!tree.is_animating());
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            DrawCommand::FillRect { color, .. } if *color == palette.raised
        )));
    }

    #[test]
    fn removing_skeleton_stops_repaint_after_layout() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(Element::column(vec![Skeleton::new(
            "temporary-load",
            palette,
        )
        .render()]));
        tree.layout(Size::new(320.0, 40.0));
        assert!(tree.is_animating());
        tree.set_root(Element::spacer());
        tree.layout(Size::new(320.0, 40.0));
        assert!(!tree.is_animating());
    }

    #[test]
    fn semantic_shapes_keep_their_geometry_and_remain_passive() {
        let palette = ComponentPalette::neutral_dark();
        let mut tree = UiTree::new(Element::column(vec![
            Skeleton::block("preview", palette, 250.0, 125.0).render(),
            Skeleton::line("title", palette, 250.0).render(),
            Skeleton::line("description", palette, 200.0).render(),
            Skeleton::circle("avatar", palette, 32.0).render(),
        ]));
        let frame = tree.layout(Size::new(260.0, 200.0));
        assert!(frame.regions.is_empty());
        let sizes = frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, radius, .. } => {
                    Some((rect.width, rect.height, *radius))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(sizes.contains(&(250.0, 125.0, 12.0)));
        assert!(sizes.contains(&(250.0, 12.0, 6.0)));
        assert!(sizes.contains(&(200.0, 12.0, 6.0)));
        assert!(sizes.contains(&(32.0, 32.0, 16.0)));
    }

    fn fills(element: Element, width: f32) -> Vec<crate::Rect> {
        let mut tree = UiTree::new(Element::column(vec![element]));
        tree.set_reduced_motion(true);
        let frame = tree.layout(Size::new(width, 400.0));
        assert!(frame.regions.is_empty());
        frame
            .commands
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillRect { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn group_rows_vary_in_width_and_sit_centered_in_their_height() {
        let palette = ComponentPalette::neutral_dark();
        let lines = fills(SkeletonGroup::rows("list", palette, 5).row_height(30.0).render(), 200.0);
        let widths: Vec<f32> = lines.iter().map(|rect| rect.width.round()).collect();
        // 10 px of gap leaves 190 to share; the fifth row repeats the first.
        assert_eq!(widths, [87.0, 118.0, 72.0, 103.0, 87.0]);
        for (row, rect) in lines.iter().enumerate() {
            assert_eq!((rect.x, rect.height), (0.0, 10.0));
            assert_eq!(rect.y, row as f32 * 30.0 + 10.0);
        }
    }

    #[test]
    fn group_icon_leads_each_row_and_ids_derive_from_the_group() {
        let palette = ComponentPalette::neutral_dark();
        let rows = SkeletonGroup::rows("list", palette, 2).icon(true).into_rows();
        assert_eq!(rows.len(), 2);
        let rects = fills(Element::column(rows), 200.0);
        assert_eq!(rects.len(), 4);
        assert_eq!((rects[0].width, rects[0].height), (16.0, 16.0));
        assert_eq!(rects[1].x, 26.0);

        let tree = format!("{:?}", SkeletonGroup::rows("list", palette, 2).icon(true).render());
        for id in ["list:0:icon", "list:0", "list:1:icon", "list:1"] {
            assert!(tree.contains(&format!("id: {id:?}")), "{id}");
        }
    }

    #[test]
    fn group_ignores_bad_numbers() {
        let palette = ComponentPalette::neutral_dark();
        let group = SkeletonGroup::rows("list", palette, 1)
            .row_height(f32::NAN)
            .row_height(-4.0)
            .fractions(&[])
            .fractions(&[f32::INFINITY, f32::NAN]);
        assert_eq!(group.row_height, 10.0);
        assert_eq!(group.fractions, [0.46, 0.62, 0.38, 0.54]);
        let group = group.fractions(&[f32::NAN, 2.0, -1.0, 0.25]);
        assert_eq!(group.fractions, [1.0, 0.0, 0.25]);
        assert!(SkeletonGroup::rows("none", palette, 0).into_rows().is_empty());
    }
}
