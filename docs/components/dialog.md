# Modal / AlertDialog

A centered surface over a dimmed window that takes the keyboard until it is
answered. There are two types: `Modal` for a form or other content, and
`AlertDialog` to confirm a consequential action.

## Import

```rust
use limn::{AlertDialog, Component, Modal};
```

## Usage

A dialog wraps the window's root element, not the button that opens it: it
covers the whole window, so it is built last, around everything else. It is
controlled: pass whether it is open, and close it when it reports. While
closed, `render` returns the root alone.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    let root = window_content(state, palette);
    Modal::new(
        root,
        state.creating,
        "New project",
        "The name can be changed later.",
        "create:close",
        palette,
    )
    .body(TextField::new("create:name", &state.name, "My project", palette).render())
    .footer(Button::new("create:confirm", "Create", palette).render())
    .initial_focus("create:name")
    .render()
}

fn update(state: &mut State, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        match id.as_str() {
            "create:open" => state.creating = true,
            "create:close" => state.creating = false,
            "create:confirm" => state.create(),
            _ => {}
        }
    }
}
```

The button that reports `create:open` is an ordinary button anywhere in the
window. To use several dialogs, wrap one in the next: the element one returns
is the base of the other.

## Modal

`Modal::new(base, open, title, description, close_id, palette)` has a title
row with a close button, the description when it is not empty, a `body` in a
scroll area and a `footer` that stays in view.

```rust
Modal::new(root, open, "Sources", "", "sources:close", palette)
    .width(Modal::fit_width(window_width, 420.0))
    .height(360.0)
    .body(form)
    .footer(actions)
    .backdrop_dismiss(true)
    .render()
```

`width` is 520 unless set and never under 240. `Modal::fit_width(viewport_width,
max)` is the width that keeps the window's margin clear, and
`Modal::body_width(modal_width)` the width a body can count on. `height`
bounds the card, never under 160; the body takes what is left and scrolls.
`closable(false)` leaves the close button out of the title row; Escape
still reports the close ID. `scrim(color)` sets what dims the window, black
at about 70 % unless set. `title_gap(px)` is the space under the title row,
which is at least 24 high; 10 unless set. `backdrop_dismiss(true)` makes a click on the dimmed window report the close
ID; without it that click does nothing. `initial_focus(id)` names the control
focused when the modal opens and `return_focus(id)` the one focused when it
closes.

### A body to the card's edges

```rust
Modal::new(root, open, "Settings", "", "settings:close", palette)
    .width(Modal::fit_width(window_width, 1024.0))
    .height(772.0)
    .flush_body(true)
    .body(Element::row(vec![pages, divider, page]))
    .render()
```

`flush_body(true)` lays the body out to the card's edges, under a 44-high
title row and a hairline in the palette's border, for a body that is a
layout of its own: a column of pages beside their content, as a settings
window has. The body takes the card's width and the height the title row
and the footer leave; the modal puts no scroll area round it, so it scrolls
what it wants itself. The title row and the footer keep their padding.

## AlertDialog

`AlertDialog::new(base, open, title, description, confirm_id, cancel_id,
palette)` asks a question with a Cancel and a confirm button. The host goes
ahead only when it receives `confirm_id`.

```rust
AlertDialog::new(
    root,
    state.deleting,
    "Delete the project?",
    "This cannot be undone.",
    "delete:confirm",
    "delete:cancel",
    palette,
)
.confirm_label("Delete")
.cancel_label("Cancel")
.destructive(danger)
.render()
```

The labels are "Continue" and "Cancel" unless set, or the `confirm` and
`cancel` of the library's [labels](../theming.md#language). `destructive(color)`
fills the confirm button with that color and draws its label in white;
`soft_destructive(fill, border, ink)` paints it pale with a one-pixel outline
instead, such as #FBE7E7, #DC3C3D and #A42528. `cancel_variant(variant)`
paints Cancel, `Outline` unless set; `Secondary` fills it with the palette's
`raised`. `description_size(px)` is 12 unless set, and `description_line_height(px)`
sets each of its lines at least that high. `closable(false)` leaves
out the close button; Cancel and Escape still report `cancel_id`.
`button_sizing(sizing)` sets the actions' geometry, `ControlSize::Compact`'s
unless set (28 high, 12 px labels). `scrim` and `title_gap` are the modal's. `confirm_enabled(false)` keeps the
button in view and passive while the action is being prepared.
`alternative(id, label)` adds a second way to go ahead between the two, and
`body(element)` puts content between the description and the actions. `width`
is 440 unless set; under 360 the actions stack. Cancel has the focus when the
dialog opens, and a click on the dimmed window does not dismiss it.

## Keyboard

| Key | Effect |
| --- | --- |
| Tab | Moves among the dialog's controls and does not leave it. |
| Escape | Reports the close ID of a `Modal`, the cancel ID of an `AlertDialog`. |

When the dialog goes, focus returns to the control that had it.

## Motion

The card fades in and settles 4 px into place, the dimmed backdrop fades with
it, and both fade out; see [Animation](../animation.md). `animation_ms` sets
the entrance.

## Actions

| Action | When |
| --- | --- |
| `UiAction::Clicked { id }` with a modal's close ID | Its close button, Escape, or the backdrop with `backdrop_dismiss(true)`. |
| `UiAction::Clicked { id }` with `confirm_id` | The confirm button. |
| `UiAction::Clicked { id }` with `cancel_id` | Cancel, the close button or Escape. |
| `UiAction::Clicked { id }` with the alternative's ID | The alternative button. |

Controls the host puts in the body or the footer report their own actions.

## Accessibility

A `Modal` is a dialog named by its title and described by its description,
known as `{close_id}:dialog`. An `AlertDialog` is an alert dialog named and
described the same way, known as `{cancel_id}:dialog`. The close button is a
button named by `labels().close`, and a disabled `DismissButton` is still
there, as disabled.
