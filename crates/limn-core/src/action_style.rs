//! The measures and the surface of an action menu. The editor's context menu
//! defines the visual standard for action menus; selector menus can keep their
//! own density and richer option content.

use crate::{BoxShadow, Color, ComponentPalette, Element, Point};

pub const ROW_HEIGHT: f32 = 26.0;
pub const INSET: f32 = 4.0;
pub const WIDTH: f32 = 352.0;
pub const MAX_HEIGHT: f32 = 480.0;
pub const LABEL_FONT_SIZE: f32 = 13.0;
pub const SHORTCUT_FONT_SIZE: f32 = 12.0;
pub fn row_radius() -> f32 {
    crate::components::control_radius()
}
pub const HORIZONTAL_PADDING: f32 = 8.0;
pub const GAP: f32 = 8.0;
pub const SEPARATOR_HEIGHT: f32 = 9.0;
pub const SCROLLBAR_GUTTER: f32 = 14.0;
pub const HEADER_HEIGHT: f32 = 24.0;

pub fn surface(
    content: Element,
    palette: ComponentPalette,
    width: f32,
    height: Option<f32>,
    inset: f32,
    border: Option<Color>,
    shadow: Option<BoxShadow>,
) -> Element {
    let shadow =
        shadow.unwrap_or_else(|| BoxShadow::new(palette.shadow, Point::new(0.0, 6.0), 18.0, 0.0));
    Element::panel(content)
        .padding(inset)
        .background(palette.surface)
        .border(border.unwrap_or(palette.border), 1.0)
        .radius(crate::components::surface_radius())
        .shadow(shadow.color, shadow.offset, shadow.blur, shadow.spread)
        .size(Some(width), height)
        .clip_children()
}
