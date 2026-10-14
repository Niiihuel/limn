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
    /// Something that is clicked. What the tree makes of a button, an icon button
    /// and a clickable.
    Button,
    /// Text that goes somewhere when clicked, such as a breadcrumb's step.
    Link,
    /// A box that is ticked or not; `toggled` says which.
    CheckBox,
    /// A switch that is on or off; `toggled` says which.
    Switch,
    /// One option of a set in which a single one is chosen.
    RadioButton,
    /// A value dragged along a track; `value` is where it stands. What the tree
    /// makes of a slider.
    Slider,
    /// A one-line text field. What the tree makes of a text input, with what it
    /// holds as `value_text`.
    TextInput,
    /// A one-line field whose content is hidden. The tree gives it no `value_text`.
    PasswordInput,
    /// A field of several lines. What the tree makes of a text area and a code
    /// editor.
    MultilineTextInput,
    /// A field that searches.
    SearchInput,
    /// A control that opens a list to choose one value from, such as a select:
    /// `value_text` is the current value and `expanded` whether the list is open.
    ComboBox,
    /// One tab of a strip; `selected` says whether it is the chosen one.
    Tab,
    /// One action in a menu.
    MenuItem,
    /// An item of a menu that is a check box; `toggled` says whether it is on.
    MenuItemCheckBox,
    /// One item of a set in a menu of which a single one is chosen; `toggled`
    /// says whether it is the chosen one.
    MenuItemRadio,
    /// One row of a list or a table.
    ListItem,
    /// One row of a tree; `expanded` says whether its children show.
    TreeItem,
    /// A surface that holds the user until it is closed: a modal, a coach mark.
    Dialog,
    /// A dialog that asks for a decision.
    AlertDialog,
    /// A message that matters now: an alert, a toast, an error.
    Alert,
    /// Information that asks for nothing: a notice, a count of matches.
    Status,
    /// A hint about another control.
    Tooltip,
    /// Work in course: a progress bar, whose `value` is how far it is, or a
    /// spinner, which has none.
    ProgressIndicator,
    /// A picture, named by its text alternative.
    Image,
    /// The title of a section.
    Heading,
    /// Text that names something else.
    Label,
    /// A thing with no kind of its own. A node whose role nothing said is one.
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
    /// A name for a control that nothing else names: its hint's title.
    pub(crate) hint: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) disabled: bool,
    pub(crate) toggled: Option<bool>,
    pub(crate) mixed: bool,
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

    /// What a hint says of the control it is on: its name if the control has
    /// no other, and otherwise its description.
    pub(crate) fn hinted(title: impl Into<String>) -> Self {
        Self { hint: Some(title.into()), ..Self::default() }
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

    /// Whether a switch, a check box or a menu's check box or radio item is on.
    pub fn toggled(mut self, on: bool) -> Self {
        self.toggled = Some(on);
        self
    }

    /// Neither on nor off: a check box over a list of which only part is
    /// chosen. It is said in place of `toggled`.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
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
            hint: self.hint.or(under.hint),
            description: self.description.or(under.description),
            disabled: self.disabled || under.disabled,
            toggled: self.toggled.or(under.toggled),
            mixed: self.mixed || under.mixed,
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
    /// What kind of thing it is. `Role::Group` when neither the component nor the
    /// element's kind said.
    pub role: Role,
    /// Empty when nothing names it: an icon alone, without a name given.
    pub name: String,
    /// A longer explanation, said after the name. `None` when there is none or it
    /// would only repeat the name.
    pub description: Option<String>,
    /// Whether it is there and cannot be used. A disabled node is not `actionable`.
    pub disabled: bool,
    /// Whether a switch or a check box is on. `None` for what is not toggled.
    pub toggled: Option<bool>,
    /// True for a check box that is neither on nor off: part of what it stands for is chosen.
    pub mixed: bool,
    /// Whether it is the chosen one of several. `None` for what is not one of several.
    pub selected: Option<bool>,
    /// Whether what it opens is open. `None` for what opens nothing.
    pub expanded: Option<bool>,
    /// From 0 to 1.
    pub value: Option<f32>,
    /// Its value in words, such as what a field holds. `None` when it has none;
    /// the tree gives none for a password field.
    pub value_text: Option<String>,
    /// Whether it answers a click and takes the keyboard focus.
    pub actionable: bool,
    /// Whether it has the keyboard focus now.
    pub focused: bool,
}
