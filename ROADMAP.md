# Roadmap

What limn does not do yet, in the order we mean to do it. Nothing here has a
date. The [changelog](CHANGELOG.md) is what has shipped.

## Accessibility

A frame now tells the platform what is in it: each control with a role, a
name and its state, published through AccessKit by `limn-egui`. See the
[accessibility guide](docs/accessibility.md). What is still missing:

- A tree, not a list: a menu's rows as children of the menu, a table's cells
  in rows.
- The caret and the selection of a text field, for the platform to follow.
- More than "activate" and "focus": stepping a slider, scrolling a region.
- The platform's preferences for reduced motion and contrast, read by
  default.
- A check of every component with a screen reader speaking, on each
  platform. What is published on Linux has been read from AT-SPI.

## Platforms

Linux is the only platform limn is developed and tested on, under Wayland and
X11. The browser build works through WebAssembly with WebGL 2 or WebGPU.

- macOS and Windows: build, run the gallery, fix what differs, add both to CI.
- Dragging files between applications (`limn-dnd`) exists for Wayland only.
- In a browser, syntax highlighting and image decoding run on the UI thread.

## Text

- Input methods (IME) and right-to-left text have not been tested.

## Stability

- The API changes without notice until 1.0. Each release lists its changes.
- A minimum supported Rust version below the current 1.95, if nothing needs it.
