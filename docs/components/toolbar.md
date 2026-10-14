# Toolbar

A bar of commands in groups. It decides the height, the space between
controls, the rule between two groups and the size of every control in it, so
that a host does not assemble rows, separators and paddings by hand. Use it
over a document or a pane, or down the side of a window.

## Import

```rust
use limn::{Component, ControlSize, Toolbar, ToolbarItem};
```

## Usage

Describe each control as a `ToolbarItem` and add them a group at a time. The
bar renders them, and each reports a click on its own ID. The bar stores
nothing: a toggle takes its value from your state each frame.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Toolbar::new("document", palette)
        .group(vec![
            ToolbarItem::button("new", "New").icon(Icon::Add),
            ToolbarItem::icon_button("open", Icon::Folder, "Open"),
        ])
        .group(vec![
            ToolbarItem::toggle("bold", "B", state.bold).accessibility_label("Bold"),
            ToolbarItem::icon_toggle("pin", Icon::Pin, "Pin the panel", state.pinned),
        ])
        .spacer()
        .group(vec![ToolbarItem::icon_button("settings", Icon::Settings, "Settings")])
        .accessibility_label("Document")
        .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "new" => state.new_document(),
            "bold" => state.bold = !state.bold,
            "pin" => state.pinned = !state.pinned,
            _ => {}
        }
    }
}
```

## Items

| Constructor | What the bar renders |
| --- | --- |
| `ToolbarItem::button(id, label)` | A ghost `Button` with this label. |
| `ToolbarItem::icon_button(id, icon, title)` | A ghost button of the icon alone, a square. `title` is its hint and its name. |
| `ToolbarItem::toggle(id, label, pressed)` | A [`Toggle`](toggle.md) with this label. |
| `ToolbarItem::icon_toggle(id, icon, title, pressed)` | A `Toggle` of the icon alone. `title` is its hint and its name. |
| `ToolbarItem::element(element, width)` | An element you built, such as a field or a short text. |

Every item but the last takes `.icon(icon)` before its label,
`.tooltip(title)`, `.accessibility_label(name)`, `.width(px)`,
`.priority(n)`, `.disabled()` and `.enabled(bool)`. A width that is negative
becomes zero and one that is not finite is ignored.

The bar cannot change an element you built. It centers it in the bar and
counts it `width` wide; the element keeps its own size, is not disabled with
the bar and never moves to the overflow menu.

## Groups and the spacer

`.group(items)` adds a group after what the bar holds. Controls stand 2 apart,
and a rule is drawn between two groups that follow each other. An empty group
adds nothing, and no rule.

`.spacer()` pushes the groups added after it to the far end. No rule is drawn
across it. It needs room to take: give the bar a `width`, or put it where its
container stretches it, as a column does.

## Sizes

```rust
Toolbar::new("document", palette).size(ControlSize::Compact)
```

`.size` sets the height of every control the bar renders: `Dense` is 24,
`Compact` 28, `Regular` 32 and `Spacious` 36. `Regular` is the default. The
bar is as high as its controls, and 8 higher when it draws a surface or a
border. The order of the builders does not matter.

## Surface and border

```rust
Toolbar::new("document", palette).surface(true).bordered(true)
```

By default the bar paints nothing of its own and has no padding, for the
header of a panel that already has a ground. `.surface(true)` paints the
palette's `surface` behind it and `.bordered(true)` outlines it in the
palette's `border`. Either one sets the controls 4 in from the edge and rounds
the bar with `tokens().radius.card`.

## Vertical

```rust
Toolbar::new("views", palette).group(views).group(tools).vertical()
```

`.vertical()` lays the controls in a column, with a horizontal rule between
groups. `.spacer()` then pushes down, once the bar has a height: give the
rendered element one with `.size(None, Some(height))`.

## Width and overflow

```rust
Toolbar::new("files", palette)
    .group(items)
    .width(pane_width)
    .overflow(state.more_open, "More")
```

Without a width the bar takes what its container gives it. `.width(px)` fixes
it, and the bar then cuts what falls outside. A negative width becomes zero
and one that is not finite is ignored.

With `.overflow(open, title)` as well, the controls that do not fit move into
a menu behind a button at the far end of the bar, which is shown only while
something is left out. The control with the lowest `.priority(n)` leaves
first, and among equals the one given last. A group that leaves whole takes
its rule with it. In the menu a control is a row with its icon and its label,
or its `accessibility_label` or hint if it has no label; a pressed toggle is
checked and a disabled control is a disabled row. A row reports the same ID
the control does.

You own `open`:

```rust
match id.as_str() {
    "files:overflow_toggle" => state.more_open = true,
    "files:overflow_dismiss" => state.more_open = false,
    "refresh" => {
        state.more_open = false;
        state.refresh();
    }
    _ => {}
}
```

### Limits

- The bar measures no text. An icon alone and an item with `.width(px)` are
  counted exactly; a label is counted as 0.6 of its font size for each
  letter. Give a labelled item a `.width` where the count must be exact. A
  label counted too short is cut at the bar's edge, not drawn outside it.
- An element you built stays in the bar at any width.
- A vertical bar has no overflow menu. Give it a `width` and it cuts what
  falls outside.

## ButtonGroup, ToggleGroup or Toolbar

| Use | When |
| --- | --- |
| [`ButtonGroup`](button.md) | The actions that close a form or a dialog: a few full buttons, the primary one last, in a row that can stack into a column. |
| [`ToggleGroup`](toggle.md) | One set of related toggles joined inside one outline, of which any number may be pressed: bold, italic, underline. |
| `Toolbar` | The commands of a document, a pane or a window: several groups of quiet buttons and toggles at one size, with rules, a far end and, if it can get narrow, an overflow menu. |

A `ToggleGroup` can stand in a toolbar as `ToolbarItem::element`, at the size
you give it.

## Disabled

```rust
Toolbar::new("document", palette).group(items).disabled()
Toolbar::new("document", palette).group(items).enabled(state.editable)
ToolbarItem::button("undo", "Undo").enabled(state.can_undo)
```

A disabled bar disables every control it renders, whatever each one says, and
the button of its overflow menu. Each is drawn muted, keeps its place, its
size and its hint, takes no click and no focus, and reports nothing. A
pressed toggle still shows that it is pressed. One disabled item leaves the
others in use.

An element given with `ToolbarItem::element` is already built, so the bar
cannot make it passive: disable it yourself where you build it.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab, Shift+Tab | Moves to the next or the previous control. Each control of the bar is a stop. |
| Enter, Space | Reports the focused control's `Clicked`. |
| Escape | In the open overflow menu, reports `{id}:overflow_dismiss`. |

The arrow keys do not move along the bar. Inside the open overflow menu they
move between its rows, as in any menu.

## Motion

The fill under the pointer and while pressing cross-fades over
`tokens().motion.hover_ms`, as a button's does. The overflow menu opens and
closes as a `DropdownMenu` does. Both stand at once under reduced motion.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with a control's ID | The control is activated by pointer, Enter or Space, in the bar or as a row of the overflow menu. |
| `UiAction::Clicked { id }` with `{id}:overflow_toggle` | The button of the overflow menu is activated while the menu is closed. |
| `UiAction::Clicked { id }` with `{id}:overflow_dismiss` | The open menu is dismissed: Escape, or a click outside it or on its button. |

## Accessibility

With `.accessibility_label("Document")` the bar is told to the platform as a
group of that name, known by the bar's ID, and as disabled when it is.
Without one the platform is told of the controls alone.

Each control is told as a button, with whether it is disabled; a toggle also
says whether it is pressed. A control is named by its label, or by its
`accessibility_label` in place of it, which a label of one letter needs. A
control of an icon alone is named by its `title`. The button of the overflow
menu is named by the title given to `.overflow` and says whether the menu is
open.
