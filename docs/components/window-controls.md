# WindowControls

The buttons at the end of a window's own title bar, for an app that draws its
frame in place of the system's: minimize, maximize and close, in that order,
as Windows places them.

## Import

```rust
use limn::{Component, Element, WindowControls};
```

## Usage

The host owns the window. It says whether the window is maximized, and answers
each button with its backend's window command.

```rust
fn view(state: &State, palette: ComponentPalette) -> Element {
    Element::row(vec![
        title(state, palette),
        Element::spacer().flex(1.0),
        WindowControls::new("window", palette).maximized(state.maximized).render(),
    ])
    .size(None, Some(32.0))
}

fn update(ctx: &egui::Context, action: &UiAction) {
    if let UiAction::Clicked { id } = action {
        let command = if *id == WindowControls::minimize_id("window") {
            egui::ViewportCommand::Minimized(true)
        } else if *id == WindowControls::maximize_id("window") {
            egui::ViewportCommand::Maximized(!ctx.input(|i| i.viewport().maximized.unwrap_or(false)))
        } else if *id == WindowControls::close_id("window") {
            egui::ViewportCommand::Close
        } else {
            return;
        };
        ctx.send_viewport_cmd(command);
    }
}
```

The buttons report `{id}:minimize`, `{id}:maximize` and `{id}:close`, which
`minimize_id`, `maximize_id` and `close_id` build. The middle button reports
`{id}:maximize` in both states: the host knows which way to go.

## Size

Each button is `WindowControls::BUTTON_WIDTH`, 46 logical pixels, wide, and
`WindowControls::WIDTH`, 138, is all three: the room to keep free at the end
of the title bar. `height(px)` makes them as tall as the bar; 32 by default.

## Look

The glyphs are hairlines in a 10 pixel square, in the palette's text. Minimize
and maximize fill with the palette's hover color under the pointer and its
selected color while pressed, with square corners, as a status bar's cells do.
Close fills with Windows' red, 196 43 28, and its glyph turns white. While
`maximized(true)` the middle button shows two overlapping squares: restore.

## Moving and resizing

The component draws the buttons only. Dragging the empty part of the title bar
to move the window, double-clicking it to maximize, and resizing from the
edges of an undecorated window are the host's, through its backend: with
eframe, `ViewportCommand::StartDrag` and `ViewportCommand::BeginResize`.

## Accessibility

Each button is a `Role::Button` named, and hinted, by `Labels::minimize`,
`Labels::maximize` (`Labels::restore` while maximized) and `Labels::close`:
"Minimize", "Maximize", "Restore" and "Close" in English, "Minimizar",
"Maximizar", "Restaurar" and "Cerrar" in Spanish. They take the keyboard focus
in order, and Enter or Space presses them.
