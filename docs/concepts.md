# Concepts

This page explains the model behind lienzo: what an element is, what the tree
remembers, and where your application's data lives. Read it once; the other
pages assume it.

## The frame loop

A lienzo application never mutates widgets. Every frame it describes the whole
window again and hands that description to a `UiTree`:

```text
state ──view──▶ Element tree ──UiTree::layout──▶ Frame ──backend──▶ pixels
  ▲                                                │
  └──────update◀── UiAction ◀──UiTree::handle_event◀── UiEvent (input)
```

| Piece | What it is | Who owns it |
| --- | --- | --- |
| State | Your data. | The host. |
| `Element` | A description of one thing on screen. Rebuilt every frame. | Nobody; it is discarded. |
| `UiTree` | Lays out the elements and remembers interaction state between frames. | The host, one per window. |
| `Frame` | Drawing commands and hit regions for one layout. | Returned by `layout`. |
| Backend | Paints a frame and translates input. `lienzo-egui` is one. | A separate crate. |

## Elements and components

An **element** is a primitive: text, a row, a button, an input, a popover.
`Element::row`, `Element::text` and the rest create them, and builder methods
such as `.padding(8.0)` or `.background(color)` style them.

A **component** is a struct that builds elements for you. `Button`, `Modal`
and `Empty` are components. Every component follows the same pattern:

```rust
let element = Button::new("save", "Save", palette) // required arguments
    .variant(ButtonVariant::Outline)               // optional builders
    .render();                                      // becomes an Element
```

Because a component renders to ordinary elements, you can wrap, nest or style
the result like anything else in the tree.

## IDs

Interactive elements take an ID, a string you choose. The ID does two jobs.

**It is the control's memory.** Elements are rebuilt every frame, so the tree
keys everything it remembers by ID: focus, hover, press, the text and cursor of
an input, the offset of a scroll area, a running transition. Give a control the
same ID next frame and it keeps all of that.

**It is the control's name in actions.** A click produces
`UiAction::Clicked { id }`.

Two rules follow:

- **Every mounted control needs its own ID.** Two controls with the same ID
  share memory: hovering one lights up the other, and they share one
  transition.
- **To send the same action from two places, keep the IDs distinct and alias
  the action:**

  ```rust
  Button::new("toolbar-save", "Save", palette).render().dispatch_as("save");
  Button::new("menu-save", "Save", palette).render().dispatch_as("save");
  ```

## Who owns state

The host owns every value a user can change. A component is *controlled*: it
receives its value each frame and reports that the user wants a change.

| Component | You pass | It reports |
| --- | --- | --- |
| `Switch` | `checked: bool` | `Clicked { id }` |
| `Collapsible` | `.expanded(bool)` | `Clicked { id }` |
| `Popover`, `Modal` | `open: bool` | `Clicked { id }` for its dismiss ID |
| `TextField` | the current text | `InputChanged { id, value }`, `Submitted { id, value }` |
| `Progress` | a fraction from 0 to 1 | nothing; it is passive |

The component never flips itself. If you ignore the action, nothing changes on
screen. This is what makes the window a pure function of your state.

What the tree does keep is *interaction* state, which is not your data: which
control has focus, where the pointer is, how far a list is scrolled.

## Actions

`UiTree::handle_event` returns the actions an event produced. The common ones:

| Action | Meaning |
| --- | --- |
| `Clicked { id }` | A button, link, row or other clickable was activated by pointer or keyboard. |
| `InputChanged { id, value }` | The text of an input changed. |
| `Submitted { id, value }` | Enter in a single-line input; Ctrl+Enter in a text area. |
| `SliderChanged { id, fraction, committed }` | A slider moved; `committed` is true on release. |
| `CopyRequested { text }` | The user copied; the host writes the clipboard. |
| `DragStarted`, `DragMoved`, `Dropped`, `DragCancelled` | A draggable element was moved. |
| `FilesHovered`, `FilesDropped` | Files from another application are over or dropped on the window. |

The code editor adds `EditorChanged`, `EditorSaveRequested` and `EditorFailed`.

## Keyboard and focus

Focus moves through buttons and inputs in tree order with Tab and Shift+Tab.
Enter and Space activate a focused button. Focus survives a rebuild as long as
the focused control keeps its ID. `tree.focus(id)` moves focus from code, after
a layout that contains the control.

An open popover, menu or dialog keeps Tab inside itself and closes on Escape.

## Passive and interactive

An element is interactive only if it is a button, an input, or wrapped in
`Element::clickable(id, child)`. Everything else, including text and icons, is
passive: it paints, and the pointer passes through it. A disabled button is
rendered as passive content, so it cannot be focused or clicked by any path.

## See also

- [Layout](layout.md) for arranging elements.
- [Animation](animation.md) for what the tree animates on its own.
- [Backend contract](backend.md) for what a `Frame` contains.
