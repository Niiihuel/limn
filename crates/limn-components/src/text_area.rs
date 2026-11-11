//! A multi-line text input on the surface every field shares.

use crate::text_field::disabled_field;
use crate::{lighter, Component, ComponentPalette, ControlState, Element, FontFamily, RadiusRole, Role};

/// A multi-line editor using the same palette and focus treatment as
/// [`TextField`](crate::TextField). The host owns its text through `InputChanged`.
pub struct TextArea {
    id: String,
    value: String,
    placeholder: String,
    palette: ComponentPalette,
    width: Option<f32>,
    height: f32,
    radius: RadiusRole,
    state: ControlState,
    preserve_line_endings: bool,
    enter_submits: bool,
    persistent_placeholder: bool,
    accessibility_label: Option<String>,
}

impl TextArea {
    /// A multi-line field with its current value and the placeholder shown
    /// while it is empty. The host stores what `InputChanged` reports under this ID.
    pub fn new(
        id: impl Into<String>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
        palette: ComponentPalette,
    ) -> Self {
        Self {
            id: id.into(),
            value: value.into(),
            placeholder: placeholder.into(),
            palette,
            width: None,
            height: 96.0,
            radius: RadiusRole::Control,
            state: ControlState::Enabled,
            preserve_line_endings: false,
            enter_submits: false,
            persistent_placeholder: false,
            accessibility_label: None,
        }
    }

    /// Make Enter report `Submitted` and Shift+Enter break the line, as a chat
    /// composer does; Ctrl+Enter submits either way. Off by default: Enter
    /// breaks the line.
    pub fn enter_submits(mut self, submits: bool) -> Self {
        self.enter_submits = submits;
        self
    }

    /// Keep the placeholder visible while this area is focused and empty.
    pub fn persistent_placeholder(mut self, visible: bool) -> Self {
        self.persistent_placeholder = visible;
        self
    }

    /// What the field is called to the platform, in place of its
    /// placeholder: an example of what to type is not a name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Fix the width, in logical pixels. A width that is not finite is ignored and a negative one is zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// Set the height, in logical pixels, never below 64. A height that is not
    /// finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = height.max(64.0);
        }
        self
    }

    /// Round the box with another radius token. The default is `Control`.
    pub fn radius(mut self, radius: RadiusRole) -> Self {
        self.radius = radius;
        self
    }

    /// Opt in to retaining CRLF/CR bytes through editing and clipboard events.
    pub fn preserve_line_endings(mut self, preserve: bool) -> Self {
        self.preserve_line_endings = preserve;
        self
    }

    /// Disable it: it keeps its place and its value and cannot be used.
    pub fn disabled(mut self) -> Self {
        self.state = ControlState::Disabled;
        self
    }

    /// Enable or disable it; the counterpart of [`disabled`](Self::disabled) for a
    /// state the host computes.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.state = if enabled {
            ControlState::Enabled
        } else {
            ControlState::Disabled
        };
        self
    }
}

impl Component for TextArea {
    fn render(self) -> Element {
        if self.state == ControlState::Disabled {
            let access = disabled_field(
                Role::MultilineTextInput,
                self.id,
                self.accessibility_label,
                &self.placeholder,
                Some(self.value.clone()),
            );
            let text = if self.value.is_empty() {
                self.placeholder
            } else {
                self.value
            };
            return Element::panel(
                Element::text(text)
                    .foreground(self.palette.muted)
                    .font_family(FontFamily::Sans)
                    .font_size(13.0),
            )
            .padding(8.0)
            .background(self.palette.raised)
            .border(self.palette.border, 1.0)
            .radius(self.radius.px())
            .size(self.width, Some(self.height))
            .accessible(access);
        }
        let area = Element::text_area(self.id, self.value, self.placeholder)
            .persistent_placeholder(self.persistent_placeholder)
            .text_area_preserve_line_endings(self.preserve_line_endings)
            .text_area_enter_submits(self.enter_submits)
            .background(self.palette.raised)
            .hover_background(lighter(self.palette.raised, 5))
            .border(self.palette.border, 1.0)
            .focus_border(lighter(self.palette.border, 70), 1.5)
            .foreground(self.palette.text)
            .font_family(FontFamily::Sans)
            .font_size(13.0)
            .padding(8.0)
            .radius(self.radius.px())
            .min_size(80.0, self.height)
            .size(self.width, Some(self.height));
        match self.accessibility_label {
            Some(name) => area.accessibility_label(name),
            None => area,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessNode, Size, UiTree};

    fn access_node(element: Element, id: &str) -> AccessNode {
        let nodes = UiTree::new(Element::column(vec![element, Element::spacer().flex(1.0)]))
            .layout(Size::new(480.0, 320.0))
            .access;
        nodes.iter().find(|node| node.id == id).cloned().unwrap_or_else(|| panic!("no node {id} among {nodes:?}"))
    }

    #[test]
    fn a_text_area_is_a_multiline_field_with_its_name_and_value_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let area = || TextArea::new("notes", "First line", "Notes", palette).enabled(enabled);
            let said = access_node(area().render(), "notes");
            assert_eq!((said.role, said.name.as_str()), (Role::MultilineTextInput, "Notes"));
            assert_eq!(said.value_text.as_deref(), Some("First line"));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
            assert_eq!(access_node(area().accessibility_label("Release notes").render(), "notes").name, "Release notes");
        }
    }


    #[test]
    fn a_text_area_takes_another_radius_token_enabled_and_disabled() {
        let palette = ComponentPalette::neutral_dark();
        let area = || TextArea::new("description", "", "Descripción", palette);
        assert_eq!(area().render().style.radius, RadiusRole::Control.px());
        for area in [area(), area().disabled()] {
            let rounded = area.radius(RadiusRole::Surface).render();
            assert_eq!(rounded.style.radius, RadiusRole::Surface.px());
        }
    }
}
