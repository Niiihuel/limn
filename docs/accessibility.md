# Accessibility

limn draws its own controls, so the platform knows only what limn tells it.
Every frame carries that: one node for each control, with a role, a name and
its state. A screen reader, a switch device or a test tool reads the nodes;
it does not read pixels.

## What you get without doing anything

The tree makes a node for everything that takes the keyboard focus:

| Element | Role | Named by |
| --- | --- | --- |
| A button | Button | Its label |
| Anything clickable | Button | The text inside it |
| A text field | Text input, or password input | Its placeholder |
| A text area | Multi-line text input | Its placeholder |
| A slider | Slider | Nothing, unless you name it |

A field gives its value too, except a password, which never does.

Components add what the tree cannot know. A `Switch` is a switch and says
whether it is on; a tab says it is a tab and whether it is the chosen one; a
`Select` says whether its list is open and what it holds; a `Progress` says
how far it is. A control drawn disabled is still there, and says so. Each
component's guide has an Accessibility section with what it tells.

## What you have to do

**Name what has no text.** An icon alone, a switch, a slider: nothing on
screen says what they are. A hint is not a name; a screen reader does not
hover.

```rust
IconButton::new("close", Icon::Close, palette).accessibility_label("Close").render()
Switch::new("wifi", state.wifi, palette).accessibility_label("Wi-Fi").render()
```

A component without that builder takes it on what it renders:

```rust
Slider::new("volume", state.volume, palette).render().accessibility_label("Volume")
```

A `FormField` names the control inside it with its label, so a field in a
form needs nothing more.

**Say what your own elements are.** For an element you build yourself, say
its role and its state:

```rust
Element::clickable("wifi", thumb)
    .accessible(Access::new(Role::Switch).name("Wi-Fi").toggled(state.wifi))
```

What you leave out the tree works out. Said on a wrapper that is not itself
a control, it goes to the control inside it.

A passive element is a node only if you give it an ID to be known by:

```rust
bar.accessible(Access::new(Role::ProgressIndicator).id("upload").name("Uploading").value(0.4))
```

**Carry out what the platform asks.** A screen reader's "activate" and
"focus" arrive as requests. With `limn-egui`, call this each frame beside
`events` and apply what it returns as you apply the tree's own actions:

```rust
for action in limn_egui::apply_access_requests(&ctx, &mut tree) {
    update(&mut state, &action);
}
```

An "activate" takes the path of a click, so a control answers it as it
answers the pointer.

**Name the window.** egui gives the platform a window with no name. With
`limn-egui`, say it each frame:

```rust
limn_egui::name_window(&ctx, "Notes");
```

## Testing it

`Frame::access` is plain data, so a test reads it like any other part of a
frame:

```rust
let frame = UiTree::new(view(&state, window)).layout(window);
let save = frame.access.iter().find(|node| node.id == "save").unwrap();
assert_eq!((save.role, save.name.as_str()), (Role::Button, "Save changes"));
assert!(!save.disabled);
```

A node with an empty name is a control nobody named. A test that walks
`frame.access` and fails on an empty name finds every one.

To see what a screen reader on Linux is really given, `tools/atspi/run.sh`
starts a program on an accessibility bus of its own and prints every node it
publishes there, with its role, name, description and states:

```sh
nix-shell --run "nix-shell -p at-spi2-core dbus gobject-introspection \
  'python3.withPackages (p: [p.pygobject3])' --run \
  'dbus-run-session -- tools/atspi/run.sh target/debug/limn-gallery --page switch'"
```

```text
application | 'limn-gallery' | '' |
  frame | 'limn gallery' | '' | focusable,focused,enabled
    toggle button | 'Notifications' | '' | focusable,enabled actions=click
    toggle button | 'Automatic updates' | '' | pressed
```

The gallery's pages were read this way. It is what the platform is told, not
what a person hears: nobody has yet used limn with a screen reader speaking.

## For a backend

`Frame::access` is a list of `AccessNode`: `id`, `rect`, `role`, `name`,
`description`, `disabled`, `toggled`, `selected`, `expanded`, `value` from 0
to 1, `value_text`, `actionable` and `focused`. Publish each to the
platform's accessibility API with its bounds in window coordinates, and turn
an activation request into a press and a release at the middle of the node.
`limn-egui` does this through AccessKit. `Role` is not exhaustive: map a role
you do not know to a group, so the node keeps its name.

## What is not there yet

- The nodes are a flat list: a menu's rows are not children of the menu, a
  table's cells are not in rows. A screen reader reads them in order.
- Text inside a field is given as its whole value: there is no caret or
  selection for the platform to follow.
- Only "activate" and "focus" are carried out. A slider cannot be stepped
  from a screen reader; it can from the keyboard.
- The preference for reduced motion is read only in a browser, with
  `limn_egui::prefers_reduced_motion()`. On the desktop it is `None` and the
  host sets `UiTree::set_reduced_motion` from its own setting. The preference
  for contrast is not read anywhere.
- A control scrolled out of view is not in the list: only what is laid out
  on screen is. Tab still reaches it and brings it into view.
- A chart tells the platform nothing: its values are only drawn.
- A disabled button reaches AT-SPI without focus and without an action, but
  still marked "enabled": AccessKit withholds that state only from fields
  that can be read-only.
- It is published on desktop. In a browser the canvas says nothing.
