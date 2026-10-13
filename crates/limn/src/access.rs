//! What a control tells the platform about itself, so that a screen reader,
//! a switch device or an automation tool can find it, name it and use it.
//!
//! The tree works most of it out: a button is a button, named by the text
//! in it; a field is a field, named by its placeholder. A component says
//! what the tree cannot know with [`Element::accessible`](crate::Element::accessible):
//! that it is a switch and is on, that it is a tab and is the chosen one,
//! that an icon alone means "Close". [`Frame::access`](crate::Frame) is the
//! result, one node for each thing, for a backend to publish.

use crate::Rect;

/// What kind of thing a control is. A backend maps each to the platform's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Role {
    Button,
    Link,
    CheckBox,
    Switch,
    RadioButton,
    Slider,
    TextInput,
    PasswordInput,
    MultilineTextInput,
    SearchInput,
    ComboBox,
    Tab,
    MenuItem,
    ListItem,
    TreeItem,
    Dialog,
    AlertDialog,
    Alert,
    Status,
    Tooltip,
    ProgressIndicator,
    Image,
    Heading,
    Label,
    Group,
}

/// What a component says about one of its elements. Everything is optional:
/// what is left out the tree works out from the element.
///
/// ```
/// use limn::{Access, Role};
/// let access = Access::new(Role::Switch).name("Wi-Fi").toggled(true);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Access {
    pub(crate) id: Option<String>,
    pub(crate) role: Option<Role>,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) disabled: bool,
    pub(crate) toggled: Option<bool>,
    pub(crate) selected: Option<bool>,
    pub(crate) expanded: Option<bool>,
    pub(crate) value: Option<f32>,
    pub(crate) value_text: Option<String>,
}

impl Access {
    /// A thing of this kind.
    pub fn new(role: Role) -> Self {
        Self { role: Some(role), ..Self::default() }
    }

    /// Only a name, for an element whose kind the tree already knows: an
    /// icon button, a field without a placeholder.
    pub fn named(name: impl Into<String>) -> Self {
        Self { name: Some(name.into()), ..Self::default() }
    }

    /// The ID the node is known by, for an element that has none of its own:
    /// a passive one, such as a progress bar, or a control drawn disabled.
    /// An interactive element is known by the ID it already has.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// What it is called: what a screen reader says first. Without one, the
    /// text inside the element is its name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// A longer explanation, said after the name.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// It is there and cannot be used.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Whether a switch or a check box is on.
    pub fn toggled(mut self, on: bool) -> Self {
        self.toggled = Some(on);
        self
    }

    /// Whether it is the chosen one of several: a tab, a row, an option.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    /// Whether what it opens is open: a menu, a section.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self
    }

    /// Where it stands from 0 to 1: a slider, a progress bar. A value that
    /// is not finite is none.
    pub fn value(mut self, fraction: f32) -> Self {
        self.value = fraction.is_finite().then(|| fraction.clamp(0.0, 1.0));
        self
    }

    /// Its value in words: what a field holds, "35 %".
    pub fn value_text(mut self, text: impl Into<String>) -> Self {
        self.value_text = Some(text.into());
        self
    }

    /// What is said here over what is said there.
    pub(crate) fn over(self, under: Access) -> Access {
        Access {
            id: self.id.or(under.id),
            role: self.role.or(under.role),
            name: self.name.or(under.name),
            description: self.description.or(under.description),
            disabled: self.disabled || under.disabled,
            toggled: self.toggled.or(under.toggled),
            selected: self.selected.or(under.selected),
            expanded: self.expanded.or(under.expanded),
            value: self.value.or(under.value),
            value_text: self.value_text.or(under.value_text),
        }
    }
}

/// One thing in a frame, as the platform should know it: where it is, what
/// it is, what it is called and the state it is in.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct AccessNode {
    /// The control's ID. An action the platform asks for is asked of it.
    pub id: String,
    /// Where it is, clipped to what can be seen.
    pub rect: Rect,
    pub role: Role,
    /// Empty when nothing names it: an icon alone, without a name given.
    pub name: String,
    pub description: Option<String>,
    pub disabled: bool,
    pub toggled: Option<bool>,
    pub selected: Option<bool>,
    pub expanded: Option<bool>,
    /// From 0 to 1.
    pub value: Option<f32>,
    pub value_text: Option<String>,
    /// Whether it answers a click and takes the keyboard focus.
    pub actionable: bool,
    /// Whether it has the keyboard focus now.
    pub focused: bool,
}
