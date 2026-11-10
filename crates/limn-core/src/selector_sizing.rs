//! Shared dimensions for selector popovers, in logical pixels.
//!
//! Compact menus show action labels. Catalogs may add a short identifier on a
//! second line; explanations belong in tooltips. These tokens describe the
//! content, while `Popover` keeps the final surface inside the viewport.

/// The dimensions of a selector popover, in logical pixels: its width range, its row
/// heights, its type sizes and how many rows show before the list scrolls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectorSizing {
    /// Width of the content when the host has room: the upper bound of `fit_width`,
    /// and the width a menu takes from `sizing`.
    pub preferred_width: f32,
    /// The lower bound of `fit_width`: the narrowest the content gets, however
    /// little room is offered.
    pub min_width: f32,
    /// Height of a one-line row.
    pub row_height: f32,
    /// Height of a two-line row: a label over a detail.
    pub detail_row_height: f32,
    /// Height of a heading row and of a menu header's title line.
    pub header_height: f32,
    /// Width of the slot kept for a row's icon, its check or its chevron.
    pub icon_size: f32,
    /// Font size of a row's label.
    pub label_size: f32,
    /// Font size of a two-line row's second line and of its status text.
    pub detail_size: f32,
    /// Space between a one-line row's edge and its content.
    pub padding: f32,
    /// Space between a row's icon, its label and its trailing parts.
    pub gap: f32,
    /// Width the rows give up to the scrollbar while the list scrolls.
    pub scrollbar_gutter: f32,
    /// The widest a row's tooltip should get. No component reads it: a host
    /// passes it to its tooltip's `max_width`.
    pub tooltip_width: f32,
    /// How many entries a `DropdownMenu` shows before its list scrolls.
    pub max_visible_items: usize,
}

impl SelectorSizing {
    /// Mode, permissions and other short action lists.
    pub const COMPACT: Self = Self {
        preferred_width: 256.0,
        min_width: 200.0,
        row_height: 28.0,
        detail_row_height: 40.0,
        header_height: 28.0,
        icon_size: 16.0,
        label_size: 12.0,
        detail_size: 11.0,
        padding: 6.0,
        gap: 6.0,
        scrollbar_gutter: 14.0,
        tooltip_width: 320.0,
        max_visible_items: 8,
    };

    /// Searchable providers, models, projects and branches.
    pub const CATALOG: Self = Self {
        preferred_width: 320.0,
        min_width: 220.0,
        max_visible_items: 6,
        ..Self::COMPACT
    };

    /// Bound the content width to the space offered by its host. The final
    /// popover placement may further reduce it for a small viewport.
    pub fn fit_width(self, available: f32) -> f32 {
        if available.is_finite() {
            available.clamp(self.min_width, self.preferred_width)
        } else {
            self.preferred_width
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_profiles_keep_compact_actions_and_catalog_detail_geometry() {
        for sizing in [SelectorSizing::COMPACT, SelectorSizing::CATALOG] {
            assert_eq!(sizing.row_height, 28.0);
            assert_eq!(sizing.detail_row_height, 40.0);
            assert_eq!(sizing.header_height, 28.0);
            assert_eq!(sizing.icon_size, 16.0);
            assert_eq!(sizing.label_size, 12.0);
            assert_eq!(sizing.detail_size, 11.0);
            assert_eq!(sizing.padding, 6.0);
            assert_eq!(sizing.gap, 6.0);
            assert_eq!(sizing.scrollbar_gutter, 14.0);
            assert_eq!(sizing.tooltip_width, 320.0);
            assert_eq!(sizing.row_height - 2.0 * sizing.padding, sizing.icon_size);
            assert_eq!((sizing.detail_row_height - sizing.row_height) / 2.0, 6.0);
        }
        assert_eq!(SelectorSizing::COMPACT.preferred_width, 256.0);
        assert_eq!(SelectorSizing::CATALOG.preferred_width, 320.0);
        assert_eq!(SelectorSizing::COMPACT.max_visible_items, 8);
        assert_eq!(SelectorSizing::CATALOG.max_visible_items, 6);
    }

    #[test]
    fn fit_width_handles_small_large_and_non_finite_host_widths() {
        for sizing in [SelectorSizing::COMPACT, SelectorSizing::CATALOG] {
            for width in [f32::MIN, -1.0, 0.0, sizing.min_width - 1.0] {
                assert_eq!(sizing.fit_width(width), sizing.min_width);
            }
            let between = (sizing.min_width + sizing.preferred_width) / 2.0;
            assert_eq!(sizing.fit_width(between), between);
            for width in [sizing.preferred_width + 1.0, f32::MAX] {
                assert_eq!(sizing.fit_width(width), sizing.preferred_width);
            }
            for width in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
                assert_eq!(sizing.fit_width(width), sizing.preferred_width);
            }
        }
    }
}
