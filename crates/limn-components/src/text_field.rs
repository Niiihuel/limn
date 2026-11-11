//! A single-line text input on the surface every field shares.

use crate::{control_radius, lighter, Access, Color, Component, ComponentPalette, ControlSize, ControlState, Element, FontFamily, Icon, Role, ICON_SIZE};

/// A single-line input with the same density, focus treatment and palette as
/// the shared search field. The host owns its value through `InputChanged`.
pub struct TextField {
    id: String,
    value: String,
    placeholder: String,
    persistent_placeholder: bool,
    palette: ComponentPalette,
    size: ControlSize,
    width: Option<f32>,
    height: Option<f32>,
    state: ControlState,
    password: bool,
    monospace: bool,
    icon: Option<Icon>,
    accessibility_label: Option<String>,
    label: Option<String>,
    outline: Option<(Color, Color)>,
    radius: Option<f32>,
}

impl TextField {
    /// A field with its current value and the placeholder shown while it is
    /// empty. The host stores what `InputChanged` reports under this ID.
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
            persistent_placeholder: false,
            palette,
            size: ControlSize::Regular,
            width: None,
            height: None,
            state: ControlState::Enabled,
            password: false,
            monospace: false,
            icon: None,
            accessibility_label: None,
            label: None,
            outline: None,
            radius: None,
        }
    }

    /// Name it with a label that rests inside it while it is empty and
    /// rises onto its outline once it is used, as an outlined Material field
    /// does, drawn as a pill on the palette's surface. The label names it to
    /// the platform unless [`accessibility_label`](Self::accessibility_label) does.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// What the field is called to the platform, in place of its
    /// placeholder: an example of what to type is not a name.
    pub fn accessibility_label(mut self, name: impl Into<String>) -> Self {
        self.accessibility_label = Some(name.into());
        self
    }

    /// Take the height and text size of a control size.
    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    /// Put an icon before the text, inside the field, as a search field has
    /// its magnifier: a `#` before a channel's name, an at sign before an
    /// address. It is decorative, in the muted ink until the field has the
    /// focus, and the text starts after it.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Set the value in the monospaced family, for a path, an identifier or
    /// a number.
    pub fn monospace(mut self, monospace: bool) -> Self {
        self.monospace = monospace;
        self
    }

    /// Mask what is typed when `hidden`, as a password field does.
    pub fn password(mut self, hidden: bool) -> Self {
        self.password = hidden;
        self
    }

    /// Keep the placeholder visible while this field is focused and empty.
    pub fn persistent_placeholder(mut self, visible: bool) -> Self {
        self.persistent_placeholder = visible;
        self
    }

    /// Fix the width, in logical pixels. A width that is not finite is ignored and a negative one is zero.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() {
            self.width = Some(width.max(0.0));
        }
        self
    }

    /// A height of the host's own, for a form whose inputs are taller than
    /// a [`ControlSize`]. The field is never shorter than its control size;
    /// a value that is not finite is ignored.
    pub fn height(mut self, height: f32) -> Self {
        if height.is_finite() {
            self.height = Some(height);
        }
        self
    }

    /// The field's own outline: `border` at rest and `focus` while it has the
    /// focus, each as wide as the palette's, in place of the palette's colors,
    /// such as a settings page's hairline and its accent. A disabled field keeps `border`.
    pub fn outline(mut self, border: Color, focus: Color) -> Self {
        self.outline = Some((border, focus));
        self
    }

    /// The corner radius in logical pixels, in place of the control's. A value
    /// that is not finite is ignored and a negative one is zero.
    pub fn radius(mut self, radius: f32) -> Self {
        if radius.is_finite() {
            self.radius = Some(radius.max(0.0));
        }
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

impl Component for TextField {
    fn render(mut self) -> Element {
        // A label needs room to rest in and to rise out of.
        let least = self.size.height().max(if self.label.is_some() { LABELED_FIELD } else { 28.0 });
        let height = self.height.map_or(least, |height| height.max(least));
        let family = if self.monospace {
            FontFamily::Monospace
        } else {
            FontFamily::Sans
        };
        if self.state == ControlState::Disabled {
            let access = disabled_field(
                if self.password { Role::PasswordInput } else { Role::TextInput },
                self.id,
                self.accessibility_label,
                &self.placeholder,
                // What is typed into a password field is nobody's to read out.
                (!self.password).then(|| self.value.clone()),
            );
            let text = if self.password && !self.value.is_empty() {
                "•".repeat(self.value.chars().count())
            } else if self.value.is_empty() {
                self.placeholder
            } else {
                self.value
            };
            let text = Element::truncated_text(text)
                .foreground(self.palette.muted)
                .font_family(family)
                .font_size(13.0);
            // Where the enabled field draws them: the icon in the padding's
            // corner and the text 8 after it, both on the middle line.
            let content = match self.icon {
                Some(icon) => Element::row(vec![
                    Element::icon(icon)
                        .foreground(self.palette.muted)
                        .size(Some(ICON_SIZE), Some(ICON_SIZE))
                        .centered_y(),
                    text.flex(1.0),
                ])
                .gap(8.0),
                None => text,
            };
            let border = self.outline.map_or(self.palette.border, |(border, _)| border);
            return Element::panel(content)
                .padding(8.0)
                .background(self.palette.raised)
                .border(border, 1.0)
                .radius(self.radius.unwrap_or_else(|| input_radius(self.size)))
                .size(self.width, Some(height))
                .accessible(access);
        }
        let mut input = Element::text_input(self.id, self.value, self.placeholder)
            .password(self.password)
            .persistent_placeholder(self.persistent_placeholder);
        if let Some(icon) = self.icon {
            input = input
                .leading_icon(icon)
                .leading_icon_tint(self.palette.muted);
        }
        let mut input = input_surface(input, self.palette, self.size, self.width)
            .size(self.width, Some(height))
            .font_family(family);
        let labeled = self.label.is_some();
        if let Some(label) = self.label {
            // Outlined on the surface it sits on, round, its text clear of the curve.
            input = input
                .floating_label(label.clone(), self.palette.surface)
                .background(self.palette.surface)
                .hover_background(self.palette.surface)
                // As Material's outlined field: a faint line at rest, brighter under the
                // pointer, whole while in use, whatever border the palette has.
                .border(Color { a: 77, ..self.palette.text }, 1.0)
                .hover_border(Color { a: 180, ..self.palette.text }, 1.0)
                .focus_border(self.palette.text, 1.5)
                .font_size(14.0)
                .padding(14.0)
                .radius(height / 2.0);
            if self.accessibility_label.is_none() {
                self.accessibility_label = Some(label);
            }
        }
        if let Some((border, focus)) = self.outline {
            input = input.border(border, 1.0).focus_border(focus, 1.5);
        }
        if let Some(radius) = self.radius {
            input = input.radius(radius);
        }
        let input = match self.accessibility_label {
            Some(name) => input.accessibility_label(name),
            None => input,
        };
        // Room over it for the label risen half out of it, inside whatever clips the field.
        if labeled {
            return Element::column(vec![Element::spacer().size(None, Some(LABEL_RISE)), input]).size(self.width, None);
        }
        input
    }
}

/// How far a [`TextField`]'s risen label reaches over its outline.
const LABEL_RISE: f32 = 8.0;

/// The least height of a [`TextField`] with a label.
const LABELED_FIELD: f32 = 48.0;

// What a field drawn disabled still says: the node its input would have been.
pub(crate) fn disabled_field(
    role: Role,
    id: String,
    name: Option<String>,
    placeholder: &str,
    value: Option<String>,
) -> Access {
    let mut access = Access::new(role).id(id).disabled(true);
    if let Some(name) = name.or_else(|| (!placeholder.is_empty()).then(|| placeholder.to_owned())) {
        access = access.name(name);
    }
    match value {
        Some(value) => access.value_text(value),
        None => access,
    }
}

pub(crate) fn input_radius(_size: ControlSize) -> f32 {
    control_radius()
}

// The fill, border, radius, ink and height every text field shares; a
// select drawn as a field takes it too.
pub(crate) fn input_surface(
    input: Element,
    palette: ComponentPalette,
    size: ControlSize,
    width: Option<f32>,
) -> Element {
    let height = size.height().max(28.0);
    input
        .background(palette.raised)
        .hover_background(lighter(palette.raised, 5))
        .border(palette.border, 1.0)
        .focus_border(lighter(palette.border, 70), 1.5)
        .foreground(palette.text)
        .font_family(FontFamily::Sans)
        .font_size(13.0)
        .padding(8.0)
        .radius(input_radius(size))
        .min_size(80.0, height)
        .size(width, Some(height))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AccessNode, DrawCommand, SearchField, Size, TextArea, UiTree};

    fn access_node(element: Element, id: &str) -> AccessNode {
        let nodes = UiTree::new(Element::column(vec![element, Element::spacer().flex(1.0)]))
            .layout(Size::new(480.0, 320.0))
            .access;
        nodes.iter().find(|node| node.id == id).cloned().unwrap_or_else(|| panic!("no node {id} among {nodes:?}"))
    }

    #[test]
    fn a_text_field_is_named_by_its_placeholder_or_the_name_it_is_given_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let field = || TextField::new("name", "limn", "e.g. my-project", palette).enabled(enabled);
            let said = access_node(field().render(), "name");
            assert_eq!((said.role, said.name.as_str()), (Role::TextInput, "e.g. my-project"));
            assert_eq!(said.value_text.as_deref(), Some("limn"));
            assert_eq!((said.disabled, said.actionable), (!enabled, enabled));
            assert_eq!(access_node(field().accessibility_label("Project name").render(), "name").name, "Project name");
            let secret = access_node(field().password(true).render(), "name");
            assert_eq!((secret.role, secret.value_text), (Role::PasswordInput, None));
        }
    }


    // A field at the top left of a window, at its own size.
    fn field_tree(field: Element) -> UiTree {
        let mut tree = UiTree::new(Element::column(vec![
            Element::row(vec![field, Element::spacer().flex(1.0)]),
            Element::spacer().flex(1.0),
        ]));
        tree.set_reduced_motion(true);
        tree
    }


    #[test]
    fn a_text_field_icon_sits_before_its_text_enabled_or_not() {
        let palette = ComponentPalette::neutral_dark();
        let drawn = |field: TextField| {
            let frame = field_tree(field.width(240.0).render()).layout(Size::new(400.0, 120.0));
            let icon = frame.commands.iter().find_map(|command| match command {
                DrawCommand::Icon {
                    rect,
                    icon: Icon::Hash,
                    tint,
                    ..
                } => Some((*rect, *tint)),
                _ => None,
            });
            let text = frame.commands.iter().find_map(|command| match command {
                DrawCommand::Text { position, text, .. } if text == "general" => Some(*position),
                _ => None,
            });
            (icon.expect("the icon"), text.expect("the text"))
        };
        let field = || TextField::new("name", "general", "Name", palette).icon(Icon::Hash);
        let ((icon, tint), text) = drawn(field());
        // In the padding's corner, muted, the text 8 after it and the line
        // in the middle of the field.
        assert_eq!((icon.x, icon.width, tint), (8.0, ICON_SIZE, palette.muted));
        assert_eq!(icon.y + icon.height / 2.0, 16.0);
        assert_eq!(text.x, 8.0 + ICON_SIZE + 8.0);
        // Disabled, the two stay where they were, both muted.
        let ((still, muted), moved) = drawn(field().disabled());
        assert_eq!((still, muted, moved.x), (icon, palette.muted, text.x));
        // Without one the text starts in the padding's corner, as before.
        let frame = field_tree(TextField::new("name", "general", "Name", palette).render())
            .layout(Size::new(400.0, 120.0));
        assert!(!frame
            .commands
            .iter()
            .any(|command| matches!(command, DrawCommand::Icon { .. })));
        // Focused, the icon takes the text's ink, as a search field's magnifier does.
        let mut tree = field_tree(field().render());
        tree.layout(Size::new(400.0, 120.0));
        assert!(tree.focus("name"));
        let focused = tree.layout(Size::new(400.0, 120.0));
        assert!(focused.commands.iter().any(|command| matches!(command,
            DrawCommand::Icon { icon: Icon::Hash, tint, .. } if *tint == palette.text)));
    }


    #[test]
    fn a_text_field_icon_stays_inside_a_narrow_field() {
        let palette = ComponentPalette::neutral_dark();
        for width in [40.0, 80.0, 180.0] {
            for field in [
                TextField::new("name", "a long value", "Name", palette)
                    .icon(Icon::Hash)
                    .width(width),
                TextField::new("name", "a long value", "Name", palette)
                    .icon(Icon::Hash)
                    .width(width)
                    .disabled(),
            ] {
                let frame = field_tree(field.render()).layout(Size::new(400.0, 120.0));
                for command in &frame.commands {
                    if let DrawCommand::Icon { rect, .. } = command {
                        assert!(
                            rect.x >= 0.0 && rect.x + rect.width <= width,
                            "{width}: {rect:?}"
                        );
                    }
                }
            }
        }
    }


    #[test]
    fn shared_text_editors_disable_without_leaving_input_targets() {
        let palette = ComponentPalette::neutral_dark();
        let enabled = UiTree::new(Element::column(vec![
            TextField::new("name", "", "Nombre", palette).render(),
            TextArea::new("description", "", "Descripción", palette)
                .height(72.0)
                .render(),
        ]))
        .layout(Size::new(320.0, 120.0));
        assert!(enabled.regions.iter().any(|region| region.id == "name"));
        assert!(enabled
            .regions
            .iter()
            .any(|region| region.id == "description"));

        let disabled = UiTree::new(Element::column(vec![
            TextField::new("name", "", "Nombre", palette)
                .disabled()
                .render(),
            TextArea::new("description", "", "Descripción", palette)
                .disabled()
                .render(),
        ]))
        .layout(Size::new(320.0, 140.0));
        assert!(disabled.regions.is_empty());
    }


    #[test]
    fn monospace_text_field_changes_only_the_family_enabled_and_disabled() {
        let palette = ComponentPalette::neutral_dark();
        for disabled in [false, true] {
            let field = |mono| {
                let field = TextField::new("path", "src/main.rs", "", palette)
                    .monospace(mono)
                    .enabled(!disabled);
                UiTree::new(field.render()).layout(Size::new(300.0, 32.0))
            };
            let family = |frame: &crate::Frame| {
                frame.commands.iter().find_map(|command| match command {
                    DrawCommand::Text { font_family, .. } => Some(*font_family),
                    _ => None,
                })
            };
            assert_eq!(family(&field(false)), Some(FontFamily::Sans));
            assert_eq!(family(&field(true)), Some(FontFamily::Monospace));
            assert_eq!(field(false).regions, field(true).regions);
        }
    }


    #[test]
    // A 28 px field leaves 12 px inside its padding, less than a 13 px line.
    fn a_compact_field_shows_its_whole_line_in_its_middle() {
        let palette = ComponentPalette::neutral_dark();
        let value = "https://example.com/docs/en/overview";
        // Without the bundled faces limn lays no text out itself: only egui's path is drawn.
        let modes: &[bool] = if cfg!(feature = "bundled-fonts") { &[false, true] } else { &[false] };
        for &laid_out in modes {
            for field in [
                TextField::new("url", value, "", palette).size(ControlSize::Compact).width(300.0).render(),
                SearchField::new("url", value, "", palette).size(ControlSize::Compact).width(300.0).render(),
            ] {
                let mut tree = UiTree::new(Element::column(vec![field]));
                tree.set_text_layout(laid_out);
                let frame = tree.layout(Size::new(400.0, 100.0));
                let field = frame.regions.iter().find(|region| region.id == "url").expect("field").rect;
                assert_eq!(field.height, 28.0);
                let mut clips = vec![];
                let mut drawn = vec![];
                for command in &frame.commands {
                    match command {
                        DrawCommand::PushClip(rect) => clips.push(*rect),
                        DrawCommand::PopClip => {
                            clips.pop();
                        }
                        DrawCommand::Text { text, position, font_size, .. } if text.contains("example.com") => {
                            drawn.push((*clips.last().expect("clipped to the field"), position.y, *font_size));
                        }
                        DrawCommand::Glyphs(run) if run.text.contains("example.com") => {
                            drawn.push((*clips.last().expect("clipped to the field"), run.origin.y, run.size));
                        }
                        _ => {}
                    }
                }
                assert!(!drawn.is_empty(), "{laid_out}: no text drawn");
                for (clip, top, size) in drawn {
                    let line = 18.0 * size / 14.0;
                    assert!(clip.height >= line - 0.5, "{laid_out}: a line of {line} in {clip:?}");
                    assert!(
                        clip.y >= field.y && clip.y + clip.height <= field.y + field.height,
                        "{laid_out}: {clip:?} out of {field:?}"
                    );
                    let middle = top + line / 2.0 - field.y;
                    assert!((middle - field.height / 2.0).abs() < 1.0, "{laid_out}: line at {middle}");
                }
            }
        }
    }


    #[test]
    fn a_text_field_takes_a_height_of_the_host_and_never_less_than_its_size() {
        let palette = ComponentPalette::neutral_dark();
        for enabled in [true, false] {
            let style = |height: f32| {
                TextField::new("path", "", "", palette)
                    .size(ControlSize::Compact)
                    .enabled(enabled)
                    .height(height)
                    .render()
                    .style
            };
            let plain = TextField::new("path", "", "", palette)
                .size(ControlSize::Compact)
                .enabled(enabled)
                .render()
                .style;
            assert_eq!(style(40.0).height, Some(40.0));
            assert_eq!(style(40.0).min_height, plain.min_height);
            for bad in [f32::NAN, f32::INFINITY, -8.0, 4.0] {
                assert_eq!(style(bad).height, plain.height, "{bad}");
            }
        }
    }


    #[test]
    fn a_text_field_takes_the_hosts_outline_and_radius_and_the_palettes_otherwise() {
        let palette = ComponentPalette::neutral_dark();
        let (border, focus) = (Color::rgb(60, 60, 60), Color::rgb(88, 101, 242));
        // Every outline the field draws, as its color, width and radius.
        let strokes = |field: TextField, focused: bool| {
            let mut tree = UiTree::new(Element::column(vec![field.width(200.0).render()]));
            tree.layout(Size::new(300.0, 120.0));
            if focused {
                assert!(tree.focus("field"));
            }
            tree.layout(Size::new(300.0, 120.0)).commands.iter().filter_map(|command| match command {
                DrawCommand::StrokeRect { color, width, radius, .. } => Some((*color, *width, *radius)),
                _ => None,
            }).collect::<Vec<_>>()
        };
        let field = || TextField::new("field", "", "Name", palette);
        let rest = (palette.border, 1.0, control_radius());
        assert_eq!(strokes(field(), false), [rest], "as the palette draws it");
        let set = || field().outline(border, focus).radius(8.0);
        assert_eq!(strokes(set(), false), [(border, 1.0, 8.0)]);
        let focused = strokes(set(), true);
        assert!(focused.contains(&(focus, 1.5, 8.0)) && !focused.contains(&(border, 1.0, 8.0)), "{focused:?}");
        // Disabled, it keeps the host's border and radius, and nothing focuses.
        assert_eq!(strokes(set().disabled(), false), [(border, 1.0, 8.0)]);
        // A radius that is not finite is ignored; a negative one is zero.
        assert_eq!(strokes(field().radius(f32::NAN), false), [rest]);
        assert_eq!(strokes(field().radius(-3.0), false), [(palette.border, 1.0, 0.0)]);
    }
}
