//! A list of options under a trigger: a `DropdownMenu` of plain choices.

use crate::*;

/// One option in a dropdown list; its ID is the host action to execute.
pub struct DropdownOption {
    /// What the host receives when the option is chosen.
    pub action_id: String,
    /// The option's text, and its name for the platform.
    pub label: String,
    /// A leading icon. `None` by default.
    pub icon: Option<Icon>,
    /// Whether it is the current choice.
    pub selected: bool,
    /// Whether it can be chosen. A disabled option stays in the list.
    pub state: ControlState,
    /// A hint shown while the pointer rests on the option or it has the
    /// keyboard, such as the whole of a name its label shortens. `None` by
    /// default.
    pub tooltip: Option<String>,
    /// A picture in the icon's place, such as a custom emoji. `None` by
    /// default.
    pub picture: Option<InlineImage>,
    /// Whether a long label is cut with an ellipsis on one line, as in
    /// [`DropdownMenuItem::truncate`], instead of wrapping. `false` by
    /// default.
    pub truncate: bool,
}

impl DropdownOption {
    /// An enabled option, not selected, that reports `action_id`.
    pub fn new(action_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            action_id: action_id.into(),
            label: label.into(),
            icon: None,
            selected: false,
            state: ControlState::Enabled,
            tooltip: None,
            picture: None,
            truncate: false,
        }
    }

    /// Cut a long label with an ellipsis on one line instead of wrapping it,
    /// as a list of device names does; pair it with a `tooltip` that says
    /// the rest.
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    /// A hint for the option, for what its label cannot say: the whole of a
    /// name the label shortens, say. A blank hint is ignored.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.tooltip = (!text.trim().is_empty()).then_some(text);
        self
    }

    /// Give the option an icon.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Show a picture in the icon's place; see [`DropdownMenuItem::image`].
    pub fn image(mut self, image: RasterImage) -> Self {
        self.picture = Some(InlineImage::Raster(image));
        self
    }

    /// Show the host-fed surface `id` in the icon's place, for an
    /// animation; see [`DropdownMenuItem::surface`].
    pub fn surface(mut self, id: impl Into<String>) -> Self {
        self.picture = Some(InlineImage::Surface(id.into()));
        self
    }

    /// The menu item that shows the option.
    pub(crate) fn item(self) -> DropdownMenuItem {
        let mut item = DropdownMenuItem::new(self.action_id, self.label)
            .selected(self.selected)
            .enabled(self.state != ControlState::Disabled);
        if let Some(icon) = self.icon {
            item = item.icon(icon);
        }
        if let Some(hint) = self.tooltip {
            item = item.tooltip_with(TooltipSpec::new(hint));
        }
        if self.truncate {
            item = item.truncate();
        }
        match self.picture {
            Some(InlineImage::Raster(image)) => item.image(image),
            Some(InlineImage::Surface(id)) => item.surface(id),
            None => item,
        }
    }

    /// Mark the option as the current choice.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Disable the option.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled { ControlState::Enabled } else { ControlState::Disabled };
        self
    }
}

/// A dropdown with caller-owned selection/open state and stable action IDs.
/// The anchor is a slot, so callers can supply an icon button, field, or any
/// other visual trigger without changing the list behavior. Arrow keys,
/// Home/End and Tab move through enabled options and reveal offscreen rows;
/// Enter activates the focused option. The host still owns the open state and
/// handles the resulting ID.
pub struct DropdownList {
    /// The element the list opens from: a button, a field or any other trigger.
    pub anchor: Element,
    /// The options, in the order they are listed.
    pub options: Vec<DropdownOption>,
    /// Whether the list shows. The host owns it.
    pub open: bool,
    /// Reported when the list is dismissed, for the host to close it.
    pub dismiss_id: String,
    /// Width of the list in logical pixels, 240 by default and 96 at least when
    /// it is drawn. `sizing` replaces it with the profile's preferred width.
    pub width: f32,
    /// The side of the anchor it prefers. `BottomStart` by default.
    pub placement: PopoverPlacement,
    /// The colors of the list's surface and of its rows.
    pub palette: ComponentPalette,
    /// Durations of the opening and of the item feedback.
    pub motion: MenuMotion,
    /// A selector profile for the row density and the scroll budget. `None`, the
    /// default, keeps the menu's own.
    pub sizing: Option<crate::SelectorSizing>,
}

impl DropdownList {
    /// A list of `options` on `anchor`, shown while `open` is true and
    /// dismissed with `dismiss_id`. It opens under the start of the anchor,
    /// 240 pixels wide.
    pub fn new(
        anchor: Element,
        options: Vec<DropdownOption>,
        open: bool,
        dismiss_id: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            anchor,
            options,
            open,
            dismiss_id: dismiss_id.into(),
            width: 240.0,
            placement: PopoverPlacement::BottomStart,
            palette,
            motion: MenuMotion::default(),
            sizing: None,
        }
    }

    /// Width of the list in logical pixels, at least 96. A width that is not
    /// finite is ignored.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = width.max(96.0);
        }
        self
    }

    /// The side of the anchor it prefers. The default is `BottomStart`.
    pub fn placement(mut self, placement: PopoverPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Share the width, row density and scroll budget of other selectors.
    /// Call before `width` to override the preferred width for the host.
    pub fn sizing(mut self, sizing: crate::SelectorSizing) -> Self {
        self.width = sizing.preferred_width;
        self.sizing = Some(sizing);
        self
    }

    /// Durations of the opening and of the item feedback; see `MenuMotion`.
    pub fn motion(mut self, motion: MenuMotion) -> Self {
        self.motion = motion;
        self
    }
}

impl Component for DropdownList {
    fn render(self) -> Element {
        let entries = self
            .options
            .into_iter()
            .map(|option| DropdownMenuEntry::Item(option.item()))
            .collect();
        let mut menu = DropdownMenu::new(
            self.anchor,
            entries,
            self.open,
            self.dismiss_id,
            self.palette,
        );
        if let Some(sizing) = self.sizing {
            menu = menu.sizing(sizing);
        }
        menu.width(self.width)
            .placement(self.placement)
            .motion(self.motion)
            .render()
    }
}
