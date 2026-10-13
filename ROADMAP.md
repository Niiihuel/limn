# Roadmap

What limn does not do yet, in the order we mean to do it. Nothing here has a
date. The [changelog](CHANGELOG.md) is what has shipped.

## Accessibility

limn draws its own controls and tells the platform nothing about them, so a
screen reader sees an empty window. This is the largest gap.

- Give every interactive element a role, a name and a state in the tree.
- Publish that tree to the platform through AccessKit from `limn-egui`.
- Check keyboard reach for every component, and document the keys.
- Honor the platform's reduced-motion and contrast preferences by default.

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
